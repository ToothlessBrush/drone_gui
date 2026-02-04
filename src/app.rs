use bevy::prelude::*;
use bevy_egui::egui::Slider;
use bevy_egui::{EguiContexts, egui};
use egui_plot::{Legend, Line, Plot};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};

// Use egui's Color32 from bevy_egui to avoid version conflicts
use egui::Color32;

use crate::drone_scene::{Drone, DroneOrientation, ViewportImage};
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
}

impl CommandQueue {
    pub fn enqueue(&self, address: u16, command: protocol::CommandType) {
        if let Ok(mut queue) = self.queue.lock() {
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
            queue.pop_front().map(|(addr, cmd)| (addr, cmd.to_ascii()))
        } else {
            None
        }
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
    fn start_uart_thread(
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

    fn disconnect_uart(&mut self) {
        if let Some(sender) = &self.uart_sender {
            let _ = sender.send(UartCommand::Disconnect);
        }
        self.uart_sender = None;
        self.serial_connected = false;
    }

    fn send_data(&self) {
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

    fn start_video_thread(&mut self) {
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

/// Main UI system that renders all the egui panels
pub fn ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<AppState>,
    mut control: ResMut<ControllerState>,
    mut drone_query: Query<&mut DroneOrientation, With<Drone>>,
    viewport_image: Res<ViewportImage>,
    command_queue: Res<CommandQueue>,
    mut persistent_settings: ResMut<crate::persistence::PersistentSettings>,
) {
    // Register the viewport image with egui context if not already done
    if state.viewport_texture_id.is_none() {
        // Use bevy_egui's add_image to register the Bevy image handle
        let egui_texture_id = contexts.add_image(viewport_image.handle.clone());
        state.viewport_texture_id = Some(egui_texture_id);
        println!(
            "Registered viewport texture with egui: {:?}",
            egui_texture_id
        );
    }

    // Update video texture if new frame is available
    let frame_data_opt = state
        .video_frame
        .lock()
        .ok()
        .and_then(|guard| guard.clone());
    if let Some(frame_data) = frame_data_opt {
        let ctx = contexts.ctx_mut();
        let texture = state.video_texture.get_or_insert_with(|| {
            ctx.load_texture(
                "video_frame",
                egui::ColorImage::from_rgb([frame_data.width, frame_data.height], &frame_data.data),
                egui::TextureOptions::default(),
            )
        });
        texture.set(
            egui::ColorImage::from_rgb([frame_data.width, frame_data.height], &frame_data.data),
            egui::TextureOptions::default(),
        );
    }

    // Update drone orientation from telemetry
    if let Ok(buffer) = state.data_buffer.lock()
        && let Some(latest) = buffer.data.back()
    {
        for mut orientation in drone_query.iter_mut() {
            orientation.roll = latest.roll;
            orientation.pitch = latest.pitch;
            orientation.yaw = latest.yaw;
        }
    }

    let ctx = contexts.ctx_mut();
    ctx.request_repaint();

    // Top Panel - Connection controls
    egui::TopBottomPanel::top("top_panel")
        .frame(egui::Frame {
            inner_margin: egui::Margin::same(8.0),
            fill: ctx.style().visuals.window_fill(),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Drone Telemetry Monitor");
                ui.separator();

                // Serial connection
                ui.label("Serial Port:");
                egui::ComboBox::from_id_salt("serial_port_select")
                    .selected_text(&state.port_path)
                    .show_ui(ui, |ui| {
                        let available = state.available_ports.clone();
                        for port in &available {
                            ui.selectable_value(&mut state.port_path, port.clone(), port);
                        }
                        // Allow manual entry if not in list
                        ui.separator();
                        ui.label("Or enter manually:");
                        ui.text_edit_singleline(&mut state.port_path);
                    });

                if state.serial_connected {
                    if ui.button("Disconnect").clicked() {
                        state.disconnect_uart();
                    }
                } else {
                    if ui.button("Connect").clicked() {
                        match state.start_uart_thread(&command_queue, &persistent_settings) {
                            Ok(()) => {
                                // Success notification already in uart module
                            }
                            Err(e) => {
                                eprintln!("Serial connection failed: {}", e);
                                // Add error to data buffer so user sees it in logs
                                if let Ok(mut buffer) = state.data_buffer.lock() {
                                    buffer.push_log(format!("Serial Error: {}", e));
                                }
                            }
                        }
                    }
                }

                ui.separator();

                // Video connection
                ui.label("Video Device:");
                ui.text_edit_singleline(&mut state.video_device_path);
                if ui
                    .button(if state.video_connected {
                        "Connected"
                    } else {
                        "Connect"
                    })
                    .clicked()
                    && !state.video_connected
                {
                    state.start_video_thread();
                }

                ui.separator();

                // Send data
                ui.label("Address:");
                ui.add(egui::TextEdit::singleline(&mut state.send_address).desired_width(40.0));
                ui.label("Data:");
                ui.text_edit_singleline(&mut state.send_data);
                if ui.button("Send").clicked() {
                    state.send_data();
                }

                ui.separator();
                ui.checkbox(&mut state.auto_scroll_logs, "Auto-scroll logs");

                ui.separator();
                if ui.button("PID Tuning").clicked() {
                    state.show_pid_tuning = !state.show_pid_tuning;
                }
            });
        });

    // Central Panel - Main content
    egui::CentralPanel::default()
        .frame(egui::Frame {
            inner_margin: egui::Margin::same(8.0),
            fill: ctx.style().visuals.window_fill(),
            ..Default::default()
        })
        .show(ctx, |ui| {
            // Extract values we need from state before locking buffer
            let auto_scroll = state.auto_scroll_logs;

            // Wrap everything in a scroll area
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Horizontal layout: View | Commands | Log
                    ui.horizontal_top(|ui| {
                        let available_width = ui.available_width();
                        let left_width = available_width * 0.25; // 25% for 3D view                                                                                                                                                                                                               
                        let middle_width = available_width * 0.20; // 40% for commands                                                                                                                                                                                                              
                        let right_width = available_width * 0.55; // 30% for logs 

                        // 3D Viewport Section
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label("3D Drone View");
                                ui.set_width(left_width);
                                let viewport_height = left_width * 0.75; // Match render target aspect
                                if let Some(texture_id) = state.viewport_texture_id {
                                    ui.image(egui::load::SizedTexture::new(
                                        texture_id,
                                        egui::vec2(left_width, viewport_height),
                                    ));
                                } else {
                                    ui.allocate_space(egui::vec2(left_width, viewport_height));
                                    ui.label("Loading 3D view...");
                                }

                                // Current values in a styled box
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(8.0))
                                    .show(ui, |ui| {
                                        let buffer = state.data_buffer.lock().unwrap();
                                        if let Some(latest) = buffer.data.back() {
                                            ui.vertical(|ui| {
                                                // Roll with red background
                                                ui.scope(|ui| {
                                                    egui::Frame::none()
                                                        .inner_margin(egui::Margin::symmetric(
                                                            6.0, 4.0,
                                                        ))
                                                        .fill(Color32::from_rgb(80, 20, 20))
                                                        .rounding(egui::Rounding::same(4.0))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "Roll: {:.2}°",
                                                                    latest.roll.to_degrees()
                                                                ))
                                                                .color(Color32::from_rgb(
                                                                    255, 100, 100,
                                                                ))
                                                                .monospace(),
                                                            );
                                                        });
                                                });

                                                // Pitch with green background
                                                ui.scope(|ui| {
                                                    egui::Frame::none()
                                                        .inner_margin(egui::Margin::symmetric(
                                                            6.0, 4.0,
                                                        ))
                                                        .fill(Color32::from_rgb(20, 80, 20))
                                                        .rounding(egui::Rounding::same(4.0))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "Pitch: {:.2}°",
                                                                    latest.pitch.to_degrees()
                                                                ))
                                                                .color(Color32::from_rgb(
                                                                    100, 255, 100,
                                                                ))
                                                                .monospace(),
                                                            );
                                                        });
                                                });

                                                // Yaw with blue background
                                                ui.scope(|ui| {
                                                    egui::Frame::none()
                                                        .inner_margin(egui::Margin::symmetric(
                                                            6.0, 4.0,
                                                        ))
                                                        .fill(Color32::from_rgb(20, 20, 80))
                                                        .rounding(egui::Rounding::same(4.0))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "Yaw: {:.2}°",
                                                                    latest.yaw.to_degrees()
                                                                ))
                                                                .color(Color32::from_rgb(
                                                                    100, 100, 255,
                                                                ))
                                                                .monospace(),
                                                            );
                                                        });
                                                });
                                            });
                                        } else {
                                            ui.label("No data received yet");
                                        }
                                    });
                            });
                        });

                        // Flight Controller Commands Section
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(middle_width);
                                ui.heading("Flight Controller Commands");
                                if state.uart_sender.is_some() {
                                    if let Ok(address) = state.send_address.parse::<u16>() {
                                        ui.horizontal(|ui| {
                                            if ui.button("Start").clicked() {
                                                persistent_settings.is_manual_mode = false;
                                                if let Err(e) = protocol::send_command_start(
                                                    &command_queue,
                                                    address,
                                                ) {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Arm and start motors");
                                        });
                                        ui.add_space(3.0);
                                        ui.horizontal(|ui| {
                                            if ui.button("Start Manual").clicked() {
                                                persistent_settings.is_manual_mode = true;
                                                // Reset all throttles when entering manual mode
                                                control.throttle = 0.0;
                                                control.master_motor_throttle = 0.0;
                                                control.motor_13_throttle = 0.0;
                                                control.motor_24_throttle = 0.0;
                                                control.motor_throttles = [0.0; 4];
                                                if let Err(e) = protocol::send_command_start_manual(
                                                    &command_queue,
                                                    address,
                                                ) {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Start manual control mode");
                                        });
                                        ui.add_space(3.0);
                                        ui.horizontal(|ui| {
                                            if ui.button("Stop").clicked() {
                                                if let Err(e) = protocol::send_command_stop(
                                                    &command_queue,
                                                    address,
                                                ) {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Disarm and stop motors normally");
                                        });
                                        ui.add_space(3.0);

                                        ui.horizontal(|ui| {
                                            if ui.button("Emergency Stop").clicked() {
                                                if let Err(e) =
                                                    protocol::send_command_emergency_stop(
                                                        &command_queue,
                                                        address,
                                                    )
                                                {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Immediate emergency shutdown");
                                        });
                                        ui.add_space(3.0);

                                        ui.horizontal(|ui| {
                                            if ui.button("Calibrate").clicked() {
                                                if let Err(e) = protocol::send_command_calibrate(
                                                    &command_queue,
                                                    address,
                                                ) {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Calibrate IMU sensors");
                                        });
                                        ui.add_space(3.0);

                                        ui.horizontal(|ui| {
                                            if ui.button("Reset").clicked() {
                                                if let Err(e) = protocol::send_command_reset(
                                                    &command_queue,
                                                    address,
                                                ) {
                                                    eprintln!("{}", e);
                                                }
                                            }
                                            ui.label("Reset flight controller state");
                                        });

                                        ui.separator();
                                        ui.label("Base Throttle");
                                        let mut throttle_clone = control.throttle;
                                        ui.add(Slider::new(&mut throttle_clone, 0.0..=1.0));
                                        control.throttle = throttle_clone;

                                        ui.separator();
                                        ui.label("Motor Throttles");
                                        let mut master_clone = control.master_motor_throttle;
                                        let master_changed = ui
                                            .add(
                                                Slider::new(&mut master_clone, 0.0..=1.0)
                                                    .text("Master"),
                                            )
                                            .changed();
                                        if master_changed {
                                            control.master_motor_throttle = master_clone;
                                            control.motor_throttles = [master_clone; 4];
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        let mut motor_13_clone = control.motor_13_throttle;
                                        if ui
                                            .add(
                                                Slider::new(&mut motor_13_clone, 0.0..=1.0)
                                                    .text("Motors 1 & 3"),
                                            )
                                            .changed()
                                        {
                                            control.motor_13_throttle = motor_13_clone;
                                            control.motor_throttles[0] = motor_13_clone;
                                            control.motor_throttles[2] = motor_13_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        let mut motor_24_clone = control.motor_24_throttle;
                                        if ui
                                            .add(
                                                Slider::new(&mut motor_24_clone, 0.0..=1.0)
                                                    .text("Motors 2 & 4"),
                                            )
                                            .changed()
                                        {
                                            control.motor_24_throttle = motor_24_clone;
                                            control.motor_throttles[1] = motor_24_clone;
                                            control.motor_throttles[3] = motor_24_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        let mut motor1_clone = control.motor_throttles[0];
                                        if ui
                                            .add(
                                                Slider::new(&mut motor1_clone, 0.0..=1.0)
                                                    .text("Motor 1"),
                                            )
                                            .changed()
                                        {
                                            control.motor_throttles[0] = motor1_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        let mut motor2_clone = control.motor_throttles[1];
                                        if ui
                                            .add(
                                                Slider::new(&mut motor2_clone, 0.0..=1.0)
                                                    .text("Motor 2"),
                                            )
                                            .changed()
                                        {
                                            control.motor_throttles[1] = motor2_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        let mut motor3_clone = control.motor_throttles[2];
                                        if ui
                                            .add(
                                                Slider::new(&mut motor3_clone, 0.0..=1.0)
                                                    .text("Motor 3"),
                                            )
                                            .changed()
                                        {
                                            control.motor_throttles[2] = motor3_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }

                                        // Motor 4 follows master
                                        let mut motor4_clone = control.motor_throttles[3];
                                        if ui
                                            .add(
                                                Slider::new(&mut motor4_clone, 0.0..=1.0)
                                                    .text("Motor 4"),
                                            )
                                            .changed()
                                        {
                                            control.motor_throttles[3] = motor4_clone;
                                            persistent_settings.motor_throttles =
                                                control.motor_throttles;
                                            if let Err(e) =
                                                protocol::send_command_set_motor_throttle(
                                                    &command_queue,
                                                    address,
                                                    control.motor_throttles,
                                                )
                                            {
                                                eprintln!("Failed to send motor throttle: {}", e);
                                            }
                                        }
                                        ui.label("Set Point");
                                    } else {
                                        ui.label("Enter valid address to enable commands");
                                    }
                                } else {
                                    ui.label("Connect to serial port to enable commands");
                                }
                            });
                        });

                        // System Logs Section
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(right_width);
                                let mut buffer = state.data_buffer.lock().unwrap();
                                ui.label(format!("System Logs ({} messages)", buffer.logs.len()));

                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .id_salt("system_logs")
                                    .auto_shrink([false; 2])
                                    .stick_to_bottom(auto_scroll)
                                    .show(ui, |ui| {
                                        if ui.button("clear logs").clicked() {
                                            buffer.clear_logs();
                                        }

                                        for log in buffer.logs.iter() {
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "[{}]",
                                                    log.clock_time.format("%H:%M:%S%.3f")
                                                ));
                                                ui.label(&log.message);
                                            });
                                        }
                                    });
                            });
                        });
                    });

                    if ui.button("clear plots").clicked() {
                        state.data_buffer.lock().unwrap().clear_data();
                    }

                    // Attitude Plot - Graph only (3D view is in the separate Bevy 3D scene)
                    ui.group(|ui| {
                        ui.label("Attitude (Roll, Pitch, Yaw)");
                        let buffer = state.data_buffer.lock().unwrap();
                        let available_width = ui.available_width();
                        let plot_height = (ui.ctx().screen_rect().height() * 0.25).min(300.0);
                        Plot::new("attitude_plot")
                            .legend(Legend::default())
                            .height(plot_height)
                            .width(available_width)
                            .show(ui, |plot_ui| {
                                plot_ui.line(
                                    Line::new(buffer.get_roll_data())
                                        .name("Roll")
                                        .color(Color32::from_rgb(255, 0, 0)),
                                );
                                plot_ui.line(
                                    Line::new(buffer.get_pitch_data())
                                        .name("Pitch")
                                        .color(Color32::from_rgb(0, 255, 0)),
                                );
                                plot_ui.line(
                                    Line::new(buffer.get_yaw_data())
                                        .name("Yaw")
                                        .color(Color32::from_rgb(0, 0, 255)),
                                );
                            });
                    });

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("PID Axis:");
                            ui.selectable_value(
                                &mut state.selected_pid_axis,
                                PidAxis::Roll,
                                "Roll",
                            );
                            ui.selectable_value(
                                &mut state.selected_pid_axis,
                                PidAxis::Pitch,
                                "Pitch",
                            );
                            ui.selectable_value(&mut state.selected_pid_axis, PidAxis::Yaw, "Yaw");
                        });

                        let selected_axis = state.selected_pid_axis;
                        let axis_name = match selected_axis {
                            PidAxis::Roll => "Roll",
                            PidAxis::Pitch => "Pitch",
                            PidAxis::Yaw => "Yaw",
                        };

                        ui.label(format!("{axis_name} PID Values (P, I, D)"));

                        let buffer = state.data_buffer.lock().unwrap();
                        let available_width = ui.available_width();
                        let plot_height = (ui.ctx().screen_rect().height() * 0.20).min(200.0);

                        Plot::new("pid_plot")
                            .legend(Legend::default())
                            .height(plot_height)
                            .width(available_width)
                            .show(ui, |plot_ui| {
                                plot_ui.line(
                                    Line::new(buffer.get_pid_p_data(selected_axis))
                                        .name("P")
                                        .color(Color32::from_rgb(255, 100, 100)),
                                );
                                plot_ui.line(
                                    Line::new(buffer.get_pid_i_data(selected_axis))
                                        .name("I")
                                        .color(Color32::from_rgb(100, 255, 100)),
                                );
                                plot_ui.line(
                                    Line::new(buffer.get_pid_d_data(selected_axis))
                                        .name("D")
                                        .color(Color32::from_rgb(100, 100, 255)),
                                );
                            });
                    });
                }); // End of scroll area
        });

    // PID Tuning Window
    let mut show_pid_tuning = state.show_pid_tuning;
    if show_pid_tuning {
        egui::Window::new("PID Tuning")
            .open(&mut show_pid_tuning)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Configure PID Parameters");
                ui.separator();

                // Axis selection
                ui.horizontal(|ui| {
                    ui.label("Axis:");
                    ui.selectable_value(
                        &mut persistent_settings.selected_tune_axis,
                        protocol::Axis::Roll,
                        "Roll",
                    );
                    ui.selectable_value(
                        &mut persistent_settings.selected_tune_axis,
                        protocol::Axis::Pitch,
                        "Pitch",
                    );
                    ui.selectable_value(
                        &mut persistent_settings.selected_tune_axis,
                        protocol::Axis::Yaw,
                        "Yaw",
                    );
                });

                ui.separator();

                // Get mutable reference to the selected axis PID parameters
                let selected_axis = persistent_settings.selected_tune_axis;
                let pid_params = persistent_settings.get_pid_mut(selected_axis);

                // PID parameters
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("P (Proportional):");
                    ui.add(
                        egui::DragValue::new(&mut pid_params.p)
                            .speed(0.01)
                            .range(0.0..=20.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("I (Integral):");
                    ui.add(
                        egui::DragValue::new(&mut pid_params.i)
                            .speed(0.001)
                            .range(0.0..=2.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("D (Derivative):");
                    ui.add(
                        egui::DragValue::new(&mut pid_params.d)
                            .speed(0.001)
                            .range(0.0..=2.0),
                    );
                });

                ui.add_space(10.0);
                ui.separator();

                // Limits
                ui.horizontal(|ui| {
                    ui.label("I Limit:");
                    ui.add(
                        egui::DragValue::new(&mut pid_params.i_limit)
                            .speed(0.1)
                            .range(0.0..=50.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("PID Limit:");
                    ui.add(
                        egui::DragValue::new(&mut pid_params.pid_limit)
                            .speed(0.1)
                            .range(0.0..=100.0),
                    );
                });

                ui.add_space(10.0);
                ui.separator();

                // Send button
                ui.horizontal(|ui| {
                    if ui.button("Send Tune").clicked() {
                        if let Ok(address) = state.send_address.parse::<u16>() {
                            let selected_axis = persistent_settings.selected_tune_axis;
                            let pid_params = persistent_settings.get_pid(selected_axis);
                            let pid = protocol::PIDController {
                                p: pid_params.p,
                                i: pid_params.i,
                                d: pid_params.d,
                                i_limit: pid_params.i_limit,
                                pid_limit: pid_params.pid_limit,
                            };

                            if let Err(e) = protocol::send_command_tune_pid(
                                &command_queue,
                                address,
                                selected_axis,
                                pid,
                            ) {
                                eprintln!("Failed to send PID tune command: {}", e);
                            } else {
                                // Log success
                                if let Ok(mut buffer) = state.data_buffer.lock() {
                                    let axis_name = match selected_axis {
                                        protocol::Axis::Roll => "Roll",
                                        protocol::Axis::Pitch => "Pitch",
                                        protocol::Axis::Yaw => "Yaw",
                                    };
                                    buffer.push_log(format!(
                                        "PID tune sent for {}: P={:.2}, I={:.2}, D={:.2}",
                                        axis_name, pid_params.p, pid_params.i, pid_params.d
                                    ));
                                }
                            }
                        } else {
                            eprintln!("Invalid address for PID tuning");
                        }
                    }

                    if ui.button("Close").clicked() {
                        state.show_pid_tuning = false;
                    }
                });

                ui.add_space(5.0);
                ui.label("Note: PID tune will be sent in next heartbeat cycle");
            });
        state.show_pid_tuning = show_pid_tuning;
    }
}

/// Controller input system that reads gamepad axes and updates controller state
/// Left stick: pitch (Y) and yaw (X)
/// Right stick: throttle adjustment (Y) and roll (X)
pub fn controller_input_system(
    time: Res<Time>,
    gamepads: Query<&Gamepad>,
    mut controller_state: ResMut<ControllerState>,
    command_queue: Res<CommandQueue>,
) {
    // Get the first connected gamepad
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };

    // Left stick Y-axis: pitch (inverted so up is positive)
    // if let Some(value) = gamepad.get(GamepadAxis::LeftStickY) {
    //     controller_state.pitch = -value; // Invert Y axis
    // }

    // Left stick X-axis: yaw
    // if let Some(value) = gamepad.get(GamepadAxis::LeftStickX) {
    //     controller_state.yaw = value;
    // }

    // Right stick Y-axis: throttle adjustment (up increases, down decreases)
    if let Some(value) = gamepad.get(GamepadAxis::RightStickY) {
        // Inverted: up is positive, down is negative
        let adjustment = value * time.delta_secs() * 0.25; // 0.5 = throttle change rate
        controller_state.throttle = (controller_state.throttle + adjustment).clamp(0.0, 1.0);
    }

    // Right stick X-axis: roll
    // if let Some(value) = gamepad.get(GamepadAxis::RightStickX) {
    //     controller_state.roll = value;
    // }

    if gamepad.pressed(GamepadButton::Start)
        && let Err(e) = protocol::send_command_emergency_stop(&command_queue, 2)
    {
        eprintln!("EMERGENCY FAILED RUN: {e}");
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
    if let Ok(mut buffer) = state.data_buffer.lock() {
        if buffer.config_requested {
            buffer.config_requested = false;
            if let Ok(address) = state.send_address.parse::<u16>() {
                let config_packet = persistent_settings.to_config_packet();
                if let Err(e) =
                    protocol::send_command_config(&command_queue, address, config_packet)
                {
                    eprintln!("Failed to queue config command: {}", e);
                } else {
                    buffer.push_log("Queued config response".to_string());
                }
            }
        }
    }

    heartbeat_timer.timer.tick(time.delta());

    if heartbeat_timer.timer.just_finished()
        && let Some(sender) = &state.uart_sender
        && let Ok(address) = state.send_address.parse::<u16>()
    {
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
            .to_ascii();

            if let Err(e) = sender.send(UartCommand::Send {
                address,
                data: heartbeat_data,
            }) {
                eprintln!("Failed to send heartbeat: {}", e);
            }
        }
    }
}
