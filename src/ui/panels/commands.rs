use crate::app::{AppState, CommandQueue, ControllerState};
use crate::persistence::PersistentSettings;
use crate::protocol;
use bevy_egui::egui::{self, DragValue};

/// Renders the flight controller commands section
pub fn render_commands_section(
    ui: &mut egui::Ui,
    state: &AppState,
    control: &mut ControllerState,
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
            render_motor_bias_controls(ui, control, command_queue, persistent_settings);
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

/// Renders motor bias controls
fn render_motor_bias_controls(
    ui: &mut egui::Ui,
    control: &mut ControllerState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.label("Motor Bias");

    // Master bias
    ui.horizontal(|ui| {
        ui.label("Master");
        let mut master_clone = control.master_motor_throttle;
        if ui
            .add(
                DragValue::new(&mut master_clone)
                    .range(0.0..=1.0)
                    .speed(0.01),
            )
            .changed()
        {
            control.master_motor_throttle = master_clone;
            control.motor_throttles = [master_clone; 4];
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_bias(command_queue, control.motor_throttles);
        }
    });

    // Motors 1 & 3
    ui.horizontal(|ui| {
        ui.label("Motors 1 & 3");
        let mut motor_13_clone = control.motor_13_throttle;
        if ui
            .add(
                DragValue::new(&mut motor_13_clone)
                    .range(0.0..=1.0)
                    .speed(0.01),
            )
            .changed()
        {
            control.motor_13_throttle = motor_13_clone;
            control.motor_throttles[0] = motor_13_clone;
            control.motor_throttles[2] = motor_13_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_bias(command_queue, control.motor_throttles);
        }
    });

    // Motors 2 & 4
    ui.horizontal(|ui| {
        ui.label("Motors 2 & 4");
        let mut motor_24_clone = control.motor_24_throttle;
        if ui
            .add(
                DragValue::new(&mut motor_24_clone)
                    .range(0.0..=1.0)
                    .speed(0.01),
            )
            .changed()
        {
            control.motor_24_throttle = motor_24_clone;
            control.motor_throttles[1] = motor_24_clone;
            control.motor_throttles[3] = motor_24_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_bias(command_queue, control.motor_throttles);
        }
    });

    // Individual motor controls
    for (i, label) in ["Motor 1", "Motor 2", "Motor 3", "Motor 4"].iter().enumerate() {
        render_individual_motor_control(ui, control, i, label, command_queue, persistent_settings);
    }
}

fn render_individual_motor_control(
    ui: &mut egui::Ui,
    control: &mut ControllerState,
    motor_index: usize,
    label: &str,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut motor_clone = control.motor_throttles[motor_index];
        if ui
            .add(
                DragValue::new(&mut motor_clone)
                    .range(0.0..=1.0)
                    .speed(0.01),
            )
            .changed()
        {
            control.motor_throttles[motor_index] = motor_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_bias(command_queue, control.motor_throttles);
        }
    });
}

fn send_motor_bias(command_queue: &CommandQueue, biases: [f32; 4]) {
    if let Err(e) = protocol::send_command_set_motor_bias(command_queue, biases) {
        eprintln!("Failed to send motor bias: {}", e);
    }
}
