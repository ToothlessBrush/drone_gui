use bevy_egui::egui;
use crate::app::{AppState, CommandQueue};
use crate::persistence::PersistentSettings;

/// Renders the top connection panel with serial, video, and send controls
pub fn render_connection_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    command_queue: &CommandQueue,
    persistent_settings: &PersistentSettings,
) {
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
        } else if ui.button("Connect").clicked() {
            match state.start_uart_thread(command_queue, persistent_settings) {
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
}
