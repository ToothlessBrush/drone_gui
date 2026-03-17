use crate::app::{AppState, CommandQueue};
use crate::persistence::PersistentSettings;
use crate::protocol;
use bevy_egui::egui;

/// Renders the PID tuning window
pub fn render_pid_tuning_window(
    ctx: &egui::Context,
    state: &mut AppState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    let mut show_pid_tuning = state.show_pid_tuning;

    if show_pid_tuning {
        egui::Window::new("PID Tuning")
            .open(&mut show_pid_tuning)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Configure PID Parameters");
                ui.separator();

                render_axis_selection(ui, persistent_settings);
                ui.separator();

                render_pid_parameters(ui, persistent_settings);
                ui.add_space(10.0);
                ui.separator();

                render_pid_limits(ui, persistent_settings);
                ui.add_space(10.0);
                ui.separator();

                render_send_controls(ui, state, command_queue, persistent_settings);
            });

        state.show_pid_tuning = show_pid_tuning;
    }
}

fn render_axis_selection(ui: &mut egui::Ui, persistent_settings: &mut PersistentSettings) {
    ui.horizontal(|ui| {
        ui.label("Axis:");
        ui.selectable_value(
            &mut persistent_settings.selected_tune_axis,
            protocol::SelectPID::Roll,
            "Roll",
        );
        ui.selectable_value(
            &mut persistent_settings.selected_tune_axis,
            protocol::SelectPID::Pitch,
            "Pitch",
        );
        ui.selectable_value(
            &mut persistent_settings.selected_tune_axis,
            protocol::SelectPID::Yaw,
            "Yaw",
        );
        ui.selectable_value(
            &mut persistent_settings.selected_tune_axis,
            protocol::SelectPID::VelocityX,
            "Velocity X",
        );
        ui.selectable_value(
            &mut persistent_settings.selected_tune_axis,
            protocol::SelectPID::VelocityY,
            "Velocity Y",
        );
    });
}

fn render_pid_parameters(ui: &mut egui::Ui, persistent_settings: &mut PersistentSettings) {
    let selected_axis = persistent_settings.selected_tune_axis;
    let pid_params = persistent_settings.get_pid_mut(selected_axis);

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
}

fn render_pid_limits(ui: &mut egui::Ui, persistent_settings: &mut PersistentSettings) {
    let selected_axis = persistent_settings.selected_tune_axis;
    let pid_params = persistent_settings.get_pid_mut(selected_axis);

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
}

fn render_send_controls(
    ui: &mut egui::Ui,
    state: &mut AppState,
    command_queue: &CommandQueue,
    persistent_settings: &PersistentSettings,
) {
    ui.horizontal(|ui| {
        let connected = state.uart_sender.is_some();
        ui.add_enabled_ui(connected, |ui| {
            if ui.button("Send Tune").clicked() {
                let config = persistent_settings.to_config_packet();
                if let Err(e) = protocol::send_command_config(command_queue, config) {
                    eprintln!("Failed to send config: {}", e);
                } else if let Ok(mut buffer) = state.data_buffer.lock() {
                    buffer.push_log("Config sent to drone".to_string());
                }
            }

            if ui.button("Save").clicked() {
                if let Err(e) = protocol::send_command_save(command_queue) {
                    eprintln!("Failed to send save command: {}", e);
                } else if let Ok(mut buffer) = state.data_buffer.lock() {
                    buffer.push_log("Save to flash queued".to_string());
                }
            }
        });

        if ui.button("Close").clicked() {
            state.show_pid_tuning = false;
        }
    });
}
