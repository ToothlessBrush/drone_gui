use eframe::egui;
use egui_plot::{Legend, Line, Plot};
use std::sync::{Arc, Mutex};

use crate::telemetry::{DataBuffer, PidAxis};
use crate::uart;

pub struct MyEguiApp {
    pub data_buffer: Arc<Mutex<DataBuffer>>,
    serial_connected: bool,
    port_path: String,
    selected_pid_axis: PidAxis,
    auto_scroll_logs: bool,
}

impl Default for MyEguiApp {
    fn default() -> Self {
        Self {
            data_buffer: Arc::new(Mutex::new(DataBuffer::new())),
            serial_connected: false,
            port_path: "/dev/ttyAMA1".to_string(),
            selected_pid_axis: PidAxis::Roll,
            auto_scroll_logs: true,
        }
    }
}

impl MyEguiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn start_uart_thread(&mut self) {
        if self.serial_connected {
            return;
        }
        let port_path = self.port_path.clone();
        let data_buffer = Arc::clone(&self.data_buffer);
        uart::start_uart_thread(port_path, data_buffer);
        self.serial_connected = true;
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Serial Port:");
                ui.text_edit_singleline(&mut self.port_path);

                if ui
                    .button(if self.serial_connected {
                        "Connected"
                    } else {
                        "Connect"
                    })
                    .clicked()
                    && !self.serial_connected
                {
                    self.start_uart_thread();
                }

                ui.separator();

                if ui.button("Clear Data").clicked()
                    && let Ok(mut buffer) = self.data_buffer.lock()
                {
                    buffer.data.clear();
                }

                if ui.button("Clear Logs").clicked()
                    && let Ok(mut buffer) = self.data_buffer.lock()
                {
                    buffer.logs.clear();
                }

                ui.separator();

                ui.checkbox(&mut self.auto_scroll_logs, "Auto-scroll logs");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Drone Telemetry Monitor");

            let buffer = self.data_buffer.lock().unwrap();

            // Attitude Plot
            ui.group(|ui| {
                ui.label("Attitude (Roll, Pitch, Yaw)");
                Plot::new("attitude_plot")
                    .legend(Legend::default())
                    .height(200.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("Roll", buffer.get_roll_data()).color(egui::Color32::RED),
                        );
                        plot_ui.line(
                            Line::new("Pitch", buffer.get_pitch_data()).color(egui::Color32::GREEN),
                        );
                        plot_ui.line(
                            Line::new("Yaw", buffer.get_yaw_data()).color(egui::Color32::BLUE),
                        );
                    });
            });

            ui.add_space(10.0);

            // PID Selection and Plot
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("PID Axis:");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Roll, "Roll");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Pitch, "Pitch");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Yaw, "Yaw");
                });

                let axis_name = match self.selected_pid_axis {
                    PidAxis::Roll => "Roll",
                    PidAxis::Pitch => "Pitch",
                    PidAxis::Yaw => "Yaw",
                };

                ui.label(format!("{axis_name} PID Values (P, I, D)"));

                Plot::new("pid_plot")
                    .legend(Legend::default())
                    .height(200.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("P", buffer.get_pid_p_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        );
                        plot_ui.line(
                            Line::new("I", buffer.get_pid_i_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(100, 255, 100)),
                        );
                        plot_ui.line(
                            Line::new("D", buffer.get_pid_d_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(100, 100, 255)),
                        );
                    });
            });

            ui.add_space(10.0);

            // Display current values
            if let Some(latest) = buffer.data.back() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Current Values");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "Last Update: {}",
                                latest.clock_time.format("%H:%M:%S%.3f")
                            ));
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.label(format!("Roll: {:.2}°", latest.roll));
                        ui.label(format!("Pitch: {:.2}°", latest.pitch));
                        ui.label(format!("Yaw: {:.2}°", latest.yaw));
                    });

                    ui.separator();

                    ui.label("Roll PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.roll_p));
                        ui.label(format!("I: {:.3}", latest.roll_i));
                        ui.label(format!("D: {:.3}", latest.roll_d));
                    });

                    ui.label("Pitch PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.pitch_p));
                        ui.label(format!("I: {:.3}", latest.pitch_i));
                        ui.label(format!("D: {:.3}", latest.pitch_d));
                    });

                    ui.label("Yaw PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.yaw_p));
                        ui.label(format!("I: {:.3}", latest.yaw_i));
                        ui.label(format!("D: {:.3}", latest.yaw_d));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(format!("Altitude: {:.2}m", latest.altitude));
                        ui.label(format!("Battery: {:.2}V", latest.battery_voltage));
                    });
                });
            }

            ui.add_space(10.0);

            // Log Section
            ui.group(|ui| {
                ui.label(format!("System Logs ({} messages)", buffer.logs.len()));

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .auto_shrink([false; 2])
                    .stick_to_bottom(self.auto_scroll_logs)
                    .show(ui, |ui| {
                        for log in buffer.logs.iter() {
                            ui.horizontal(|ui| {
                                ui.label(format!("[{}]", log.clock_time.format("%H:%M:%S%.3f")));
                                ui.label(&log.message);
                            });
                        }
                    });
            });
        });
    }
}
