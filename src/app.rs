use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_egui::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};

use crate::protocol;
use crate::telemetry::{DataBuffer, PidAxis};
use crate::uart::{self, UartCommand};

#[derive(Resource)]
pub struct CommandTimer {
    pub timer: Timer,
}

impl Default for CommandTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}

#[derive(Resource, Default)]
pub struct CommandQueue {
    pub queue: Arc<Mutex<VecDeque<protocol::CommandType>>>,
}

impl CommandQueue {
    pub fn enqueue(&self, command: protocol::CommandType) {
        if let Ok(mut queue) = self.queue.lock() {
            // Remove any existing command of the same type
            let cmd_discriminant = std::mem::discriminant(&command);
            queue.retain(|existing_cmd| {
                std::mem::discriminant(existing_cmd) != cmd_discriminant
            });
            queue.push_back(command);
        }
    }

    pub fn dequeue(&self) -> Option<Vec<u8>> {
        if let Ok(mut queue) = self.queue.lock() {
            queue.pop_front().map(|cmd| cmd.to_binary_frame())
        } else {
            None
        }
    }
}

#[derive(Resource, Clone)]
pub struct ControllerState {
    pub master_motor_throttle: f32,
    pub motor_13_throttle: f32,
    pub motor_24_throttle: f32,
    pub motor_throttles: [f32; 4],
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            master_motor_throttle: 0.0,
            motor_13_throttle: 0.0,
            motor_24_throttle: 0.0,
            motor_throttles: [0.0; 4],
        }
    }
}

impl ControllerState {
    pub fn from_persistent(settings: &crate::persistence::PersistentSettings) -> Self {
        let throttles = settings.motor_throttles;
        Self {
            master_motor_throttle: throttles[0],
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
                "/dev/ttyUSB0".to_string()
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
            viewport_texture_id: None,
            show_pid_tuning: false,
        }
    }
}

impl AppState {
    pub fn start_uart_thread(&mut self) -> Result<(), String> {
        if self.serial_connected {
            return Ok(());
        }

        let port_path = self.port_path.clone();
        let data_buffer = Arc::clone(&self.data_buffer);

        match uart::start_uart_thread(port_path, data_buffer) {
            Ok(sender) => {
                self.uart_sender = Some(sender);
                self.serial_connected = true;
                Ok(())
            }
            Err(e) => {
                self.serial_connected = false;
                self.uart_sender = None;
                Err(e)
            }
        }
    }

    pub fn refresh_ports(&mut self) {
        self.available_ports = serialport::available_ports()
            .map(|ports| ports.iter().map(|p| p.port_name.clone()).collect())
            .unwrap_or_else(|_| vec![]);
    }

    pub fn disconnect_uart(&mut self) {
        if let Some(sender) = &self.uart_sender {
            let _ = sender.send(UartCommand::Disconnect);
        }
        self.uart_sender = None;
        self.serial_connected = false;
    }

}

/// Sends Disconnect to the UART thread on app exit so the serial port is released cleanly.
pub fn uart_shutdown_system(
    mut state: ResMut<AppState>,
    mut exit_events: EventReader<AppExit>,
) {
    if exit_events.read().next().is_some() {
        state.disconnect_uart();
    }
}

/// Dispatches queued commands to the UART thread and responds to config requests
pub fn command_dispatch_system(
    time: Res<Time>,
    mut timer: ResMut<CommandTimer>,
    state: Res<AppState>,
    command_queue: Res<CommandQueue>,
) {
    if !state.serial_connected {
        return;
    }

    timer.timer.tick(time.delta());

    if timer.timer.just_finished()
        && let Some(sender) = &state.uart_sender
    {
        if let Some(frame) = command_queue.dequeue() {
            if let Err(e) = sender.send(UartCommand::Send { data: frame }) {
                eprintln!("Failed to send command: {}", e);
            }
        }
    }
}
