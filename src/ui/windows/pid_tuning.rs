use bevy_egui::egui;
use crate::app::{AppState, CommandQueue};
use crate::persistence::PersistentSettings;
use crate::protocol;

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

                // Axis selection
                render_axis_selection(ui, persistent_settings);
                ui.separator();

                // PID parameters
                render_pid_parameters(ui, persistent_settings);
                ui.add_space(10.0);
                ui.separator();

                // Limits
                render_pid_limits(ui, persistent_settings);
                ui.add_space(10.0);
                ui.separator();

                // Send button
                render_send_controls(ui, state, command_queue, persistent_settings);

                ui.add_space(5.0);
                ui.label("Note: PID tune will be sent in next heartbeat cycle");
            });

        state.show_pid_tuning = show_pid_tuning;
    }
}

/// Renders the axis selection radio buttons
fn render_axis_selection(ui: &mut egui::Ui, persistent_settings: &mut PersistentSettings) {
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
}

/// Renders the PID parameter controls
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

/// Renders the PID limit controls
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

/// Renders the send and close buttons
fn render_send_controls(
    ui: &mut egui::Ui,
    state: &mut AppState,
    command_queue: &CommandQueue,
    persistent_settings: &PersistentSettings,
) {
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

                if let Err(e) =
                    protocol::send_command_tune_pid(command_queue, address, selected_axis, pid)
                {
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
}
