use crate::app::{AppState, CommandQueue};
use crate::persistence::PersistentSettings;
use crate::protocol;
use bevy_egui::egui::{self, DragValue};

/// Renders the flight controller commands section
pub fn render_commands_section(
    ui: &mut egui::Ui,
    state: &AppState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
    width: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.heading("FC Commands");

        if state.uart_sender.is_some() {
            render_command_buttons(ui, command_queue);
            ui.separator();
            render_flight_config_controls(ui, state, command_queue, persistent_settings);
        } else {
            ui.label("Connect to serial port to enable commands");
        }
    });
}

/// Calibrate IMU button
fn render_command_buttons(ui: &mut egui::Ui, command_queue: &CommandQueue) {
    ui.horizontal(|ui| {
        if ui.button("Calibrate IMU").clicked() {
            if let Err(e) = protocol::send_command_calibrate(command_queue) {
                eprintln!("{}", e);
            }
        }
        ui.label("Calibrate gyro/accel bias");
    });
}

fn render_flight_config_controls(
    ui: &mut egui::Ui,
    state: &AppState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.label("Flight Config");

    ui.horizontal(|ui| {
        ui.label("Hover Throttle");
        ui.add(
            DragValue::new(&mut persistent_settings.throttle_hover)
                .range(0.05..=0.95)
                .speed(0.01),
        );
    });

    ui.horizontal(|ui| {
        ui.label("Throttle Expo");
        ui.add(
            DragValue::new(&mut persistent_settings.throttle_expo)
                .range(0.0..=1.0)
                .speed(0.01),
        );
    });

    let max_angle_deg = |rad: f32| rad.to_degrees();
    let deg_to_rad = |deg: f32| deg.to_radians();

    ui.horizontal(|ui| {
        ui.label("Max Roll");
        let mut deg = max_angle_deg(persistent_settings.max_roll_angle);
        if ui
            .add(DragValue::new(&mut deg).range(5.0..=60.0).speed(0.5).suffix("°"))
            .changed()
        {
            persistent_settings.max_roll_angle = deg_to_rad(deg);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Max Pitch");
        let mut deg = max_angle_deg(persistent_settings.max_pitch_angle);
        if ui
            .add(DragValue::new(&mut deg).range(5.0..=60.0).speed(0.5).suffix("°"))
            .changed()
        {
            persistent_settings.max_pitch_angle = deg_to_rad(deg);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Max Yaw Rate");
        let mut deg_s = max_angle_deg(persistent_settings.max_yaw_rate);
        if ui
            .add(DragValue::new(&mut deg_s).range(10.0..=360.0).speed(1.0).suffix("°/s"))
            .changed()
        {
            persistent_settings.max_yaw_rate = deg_to_rad(deg_s);
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Send Config").clicked() {
            let config = persistent_settings.to_config_packet();
            if let Err(e) = protocol::send_command_config(command_queue, config) {
                eprintln!("Failed to send config: {}", e);
            } else if let Ok(mut buffer) = state.data_buffer.lock() {
                buffer.push_log("Flight config sent".to_string());
            }
        }

        if ui.button("Save").clicked() {
            if let Err(e) = protocol::send_command_save(command_queue) {
                eprintln!("Failed to send save: {}", e);
            } else if let Ok(mut buffer) = state.data_buffer.lock() {
                buffer.push_log("Save to flash queued".to_string());
            }
        }
    });
}
