use bevy::prelude::*;
use bevy_egui::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};

use crate::protocol;
use crate::telemetry::{DataBuffer, PidAxis};
use crate::uart::{self, UartCommand};
use crate::video::{self, SharedVideoFrame};

#[derive(Resource)]
pub struct HeartbeatTimer {
    pub timer: Timer,
}

impl Default for HeartbeatTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Default)]
pub struct CommandQueue {
    pub queue: Arc<Mutex<VecDeque<(u16, protocol::CommandType)>>>,
    pub estop_active: Arc<Mutex<bool>>,
}

impl CommandQueue {
    pub fn enqueue(&self, address: u16, command: protocol::CommandType) {
        if let Ok(mut queue) = self.queue.lock() {
            // Check if this is an emergency stop command
            if matches!(command, protocol::CommandType::EmergencyStop) {
                // Clear the entire queue on estop
                queue.clear();
                // Set estop active flag
                if let Ok(mut estop_flag) = self.estop_active.lock() {
                    *estop_flag = true;
                }
                // Add estop command
                queue.push_back((address, command));
                return;
            }

            // Check if this is a reset command
            if matches!(command, protocol::CommandType::Reset) {
                // Clear estop active flag
                if let Ok(mut estop_flag) = self.estop_active.lock() {
                    *estop_flag = false;
                }
            }

            // Remove any existing command of the same type
            let cmd_discriminant = std::mem::discriminant(&command);
            queue.retain(|(_, existing_cmd)| {
                std::mem::discriminant(existing_cmd) != cmd_discriminant
            });

            // Add the new command
            queue.push_back((address, command));
        }
    }

    pub fn dequeue(&self) -> Option<(u16, String)> {
        if let Ok(mut queue) = self.queue.lock() {
            queue.pop_front().map(|(addr, cmd)| (addr, cmd.get_ascii()))
        } else {
            None
        }
    }

    pub fn is_estop_active(&self) -> bool {
        self.estop_active.lock().map(|flag| *flag).unwrap_or(false)
    }
}

#[derive(Resource, Clone)]
pub struct ControllerState {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub throttle: f32,
    pub master_motor_throttle: f32,
    pub motor_13_throttle: f32,
    pub motor_24_throttle: f32,
    pub motor_throttles: [f32; 4],
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            throttle: 0.0,
            master_motor_throttle: 0.0,
            motor_13_throttle: 0.0,
            motor_24_throttle: 0.0,
            motor_throttles: [0.0; 4],
        }
    }
}

impl ControllerState {
    /// Create ControllerState from saved persistent settings
    pub fn from_persistent(settings: &crate::persistence::PersistentSettings) -> Self {
        let throttles = settings.motor_throttles;
        Self {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            throttle: 0.0,
            master_motor_throttle: throttles[0], // Use motor 1 as master default
            motor_13_throttle: throttles[0],
            motor_24_throttle: throttles[1],
            motor_throttles: throttles,
        }
    }
}

#[derive(Resource, Clone)]
pub struct AppState {
    pub data_buffer: Arc<Mutex<DataBuffer>>,
    pub serial_connected: bool,
    pub port_path: String,
    pub selected_pid_axis: PidAxis,
    pub auto_scroll_logs: bool,
    pub uart_sender: Option<mpsc::Sender<UartCommand>>,
    pub send_address: String,
    pub send_data: String,
    pub video_frame: SharedVideoFrame,
    pub video_texture: Option<egui::TextureHandle>,
    pub video_connected: bool,
    pub video_device_path: String,
    pub viewport_texture_id: Option<egui::TextureId>,
    pub available_ports: Vec<String>,
    pub show_pid_tuning: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let available_ports = serialport::available_ports()
            .map(|ports| ports.iter().map(|p| p.port_name.clone()).collect())
            .unwrap_or_else(|_| vec![]);

        let default_port = available_ports.first().cloned().unwrap_or_else(|| {
            if cfg!(windows) {
                "COM3".to_string()
            } else {
                "/dev/ttyAMA1".to_string()
            }
        });

        Self {
            data_buffer: Arc::new(Mutex::new(DataBuffer::new())),
            serial_connected: false,
            port_path: default_port,
            available_ports,
            selected_pid_axis: PidAxis::Roll,
            auto_scroll_logs: true,
            uart_sender: None,
            send_address: "0".to_string(),
            send_data: String::new(),
            video_frame: Arc::new(Mutex::new(None)),
            video_texture: None,
            video_connected: false,
            video_device_path: "/dev/video2".to_string(),
            viewport_texture_id: None,
            show_pid_tuning: false,
        }
    }
}

impl AppState {
    pub fn start_uart_thread(
        &mut self,
        command_queue: &crate::app::CommandQueue,
        persistent_settings: &crate::persistence::PersistentSettings,
    ) -> Result<(), String> {
        if self.serial_connected {
            return Ok(());
        }

        let port_path = self.port_path.clone();
        let data_buffer = Arc::clone(&self.data_buffer);

        match uart::start_uart_thread(port_path, data_buffer) {
            Ok(sender) => {
                self.uart_sender = Some(sender);
                self.serial_connected = true;

                // Queue config command on startup
                if let Ok(address) = self.send_address.parse::<u16>() {
                    let config_packet = persistent_settings.to_config_packet();
                    if let Err(e) =
                        protocol::send_command_config(command_queue, address, config_packet)
                    {
                        eprintln!("Failed to queue config command on startup: {}", e);
                    }
                }

                Ok(())
            }
            Err(e) => {
                self.serial_connected = false;
                self.uart_sender = None;
                Err(e)
            }
        }
    }

    pub fn disconnect_uart(&mut self) {
        if let Some(sender) = &self.uart_sender {
            let _ = sender.send(UartCommand::Disconnect);
        }
        self.uart_sender = None;
        self.serial_connected = false;
    }

    pub fn send_data(&self) {
        if let Some(sender) = &self.uart_sender {
            if let Ok(address) = self.send_address.parse::<u16>() {
                let cmd = UartCommand::Send {
                    address,
                    data: self.send_data.clone(),
                };
                if let Err(e) = sender.send(cmd) {
                    eprintln!("Failed to send command: {}", e);
                }
            } else {
                eprintln!("Invalid address: {}", self.send_address);
            }
        }
    }

    pub fn start_video_thread(&mut self) {
        if self.video_connected {
            return;
        }
        let device_path = self.video_device_path.clone();
        match video::start_video_thread(&device_path) {
            Ok(frame_buffer) => {
                self.video_frame = frame_buffer;
                self.video_connected = true;
                println!("Video capture started from {}", device_path);
            }
            Err(e) => {
                eprintln!("Failed to start video capture: {}", e);
            }
        }
    }
}

/// Heartbeat system that sends queued commands or heartbeat every 300ms when UART is connected
pub fn heartbeat_system(
    time: Res<Time>,
    mut heartbeat_timer: ResMut<HeartbeatTimer>,
    state: Res<AppState>,
    controller_state: Res<ControllerState>,
    command_queue: Res<CommandQueue>,
    persistent_settings: Res<crate::persistence::PersistentSettings>,
) {
    // Only send if UART is connected
    if !state.serial_connected {
        return;
    }

    // Check if config was requested
    if let Ok(mut buffer) = state.data_buffer.lock()
        && buffer.config_requested
    {
        buffer.config_requested = false;
        if let Ok(address) = state.send_address.parse::<u16>() {
            let config_packet = persistent_settings.to_config_packet();
            if let Err(e) = protocol::send_command_config(&command_queue, address, config_packet) {
                eprintln!("Failed to queue config command: {}", e);
            } else {
                buffer.push_log("Queued config response".to_string());
            }
        }
    }

    heartbeat_timer.timer.tick(time.delta());

    if heartbeat_timer.timer.just_finished()
        && let Some(sender) = &state.uart_sender
        && let Ok(address) = state.send_address.parse::<u16>()
    {
        // If estop is active, always send estop command instead of heartbeat
        if command_queue.is_estop_active() {
            let estop_data = protocol::CommandType::EmergencyStop.get_ascii();
            if let Err(e) = sender.send(UartCommand::Send {
                address,
                data: estop_data,
            }) {
                eprintln!("Failed to send estop: {}", e);
            }
        } else {
            // Check if there's a command in the queue
            if let Some((cmd_address, cmd_data)) = command_queue.dequeue() {
                // Send the queued command
                if let Err(e) = sender.send(UartCommand::Send {
                    address: cmd_address,
                    data: cmd_data,
                }) {
                    eprintln!("Failed to send queued command: {}", e);
                }
            } else {
                // No queued command, send heartbeat
                let heartbeat_data = protocol::CommandType::HeartBeat(protocol::HeartBeatPacket {
                    base_throttle: controller_state.throttle,
                    roll: controller_state.roll,
                    pitch: controller_state.pitch,
                    yaw: controller_state.yaw,
                })
                .get_ascii();

                if let Err(e) = sender.send(UartCommand::Send {
                    address,
                    data: heartbeat_data,
                }) {
                    eprintln!("Failed to send heartbeat: {}", e);
                }
            }
        }
    }
}
