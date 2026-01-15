use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use egui_plot::{Legend, Line, Plot};
use gilrs::Gilrs;
use std::sync::{Arc, Mutex, mpsc};

// Use egui's Color32 from bevy_egui to avoid version conflicts
use egui::Color32;

use crate::drone_scene::{Drone, DroneOrientation, ViewportImage};
use crate::telemetry::{DataBuffer, PidAxis};
use crate::uart::{self, UartCommand};
use crate::video::{self, SharedVideoFrame};

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
}

// Gilrs is not Sync, so we keep it as a NonSend resource
// NonSend resources can only be accessed from the main thread
pub struct GamepadState {
    pub gilrs: gilrs::Gilrs,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            data_buffer: Arc::new(Mutex::new(DataBuffer::new())),
            serial_connected: false,
            port_path: "/dev/ttyAMA1".to_string(),
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
        }
    }
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            gilrs: Gilrs::new().unwrap(),
        }
    }
}

impl AppState {
    fn start_uart_thread(&mut self) {
        if self.serial_connected {
            return;
        }
        let port_path = self.port_path.clone();
        let data_buffer = Arc::clone(&self.data_buffer);
        let sender = uart::start_uart_thread(port_path, data_buffer);
        self.uart_sender = Some(sender);
        self.serial_connected = true;
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
    gamepad: Option<NonSendMut<GamepadState>>,
    mut drone_query: Query<&mut DroneOrientation, With<Drone>>,
    viewport_image: Res<ViewportImage>,
) {
    // Handle gamepad events
    if let Some(mut gamepad) = gamepad {
        while let Some(gilrs::Event {
            id, event, time, ..
        }) = gamepad.gilrs.next_event()
        {
            println!("{:?} New event from {}: {:?}", time, id, event);
        }
    }

    // Register the viewport image with egui context if not already done
    if state.viewport_texture_id.is_none() {
        // Use bevy_egui's add_image to register the Bevy image handle
        let egui_texture_id = contexts.add_image(viewport_image.handle.clone());
        state.viewport_texture_id = Some(egui_texture_id);
        println!("Registered viewport texture with egui: {:?}", egui_texture_id);
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
    if let Ok(buffer) = state.data_buffer.lock() {
        if let Some(latest) = buffer.data.back() {
            for mut orientation in drone_query.iter_mut() {
                orientation.roll = latest.roll;
                orientation.pitch = latest.pitch;
                orientation.yaw = latest.yaw;
            }
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
                ui.text_edit_singleline(&mut state.port_path);
                if ui
                    .button(if state.serial_connected {
                        "Connected ✓"
                    } else {
                        "Connect"
                    })
                    .clicked()
                {
                    if !state.serial_connected {
                        state.start_uart_thread();
                    }
                }

                ui.separator();

                // Video connection
                ui.label("Video Device:");
                ui.text_edit_singleline(&mut state.video_device_path);
                if ui
                    .button(if state.video_connected {
                        "Connected ✓"
                    } else {
                        "Connect"
                    })
                    .clicked()
                {
                    if !state.video_connected {
                        state.start_video_thread();
                    }
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
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Video feed and logs
                    ui.horizontal(|ui| {
                        let available_width = ui.available_width();

                        // 3D Viewport
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label("3D Drone View");
                                let viewport_width = (available_width * 0.4).min(320.0);
                                let viewport_height = viewport_width * 0.75; // Match render target aspect
                                if let Some(texture_id) = state.viewport_texture_id {
                                    ui.image(egui::load::SizedTexture::new(
                                        texture_id,
                                        egui::vec2(viewport_width, viewport_height),
                                    ));
                                } else {
                                    ui.allocate_space(egui::vec2(viewport_width, viewport_height));
                                    ui.label("Loading 3D view...");
                                }
                            });
                        });

                        ui.separator();

                        // System logs
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                let buffer = state.data_buffer.lock().unwrap();
                                ui.label(format!("System Logs ({} messages)", buffer.logs.len()));

                                let log_width = available_width * 0.5;
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .id_salt("system_logs")
                                    .auto_shrink([false; 2])
                                    .stick_to_bottom(auto_scroll)
                                    .show(ui, |ui| {
                                        ui.set_min_width(log_width);
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

                    ui.add_space(10.0);

                    // Attitude Plot - Graph only (3D view is in the separate Bevy 3D scene)
                    ui.group(|ui| {
                        ui.label("Attitude (Roll, Pitch, Yaw)");
                        let buffer = state.data_buffer.lock().unwrap();
                        let available_width = ui.available_width();
                        Plot::new("attitude_plot")
                            .legend(Legend::default())
                            .height(300.0)
                            .width(available_width - 20.0)
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

                    ui.add_space(10.0);

                    // PID Selection and Plot
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

                        Plot::new("pid_plot")
                            .legend(Legend::default())
                            .height(200.0)
                            .width(available_width - 20.0)
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

                    ui.add_space(10.0);

                    // Current values display
                    ui.group(|ui| {
                        ui.label("Current Values");
                        let buffer = state.data_buffer.lock().unwrap();
                        if let Some(latest) = buffer.data.back() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Roll: {:.2}°", latest.roll));
                                ui.separator();
                                ui.label(format!("Pitch: {:.2}°", latest.pitch));
                                ui.separator();
                                ui.label(format!("Yaw: {:.2}°", latest.yaw));
                                ui.separator();
                                ui.label(format!("Alt: {:.2}m", latest.altitude));
                                ui.separator();
                                ui.label(format!("Battery: {:.2}V", latest.battery_voltage));
                            });
                        } else {
                            ui.label("No data received yet");
                        }
                    });
                }); // End of scroll area
        });
}
