use bevy_egui::egui::{self, Slider, DragValue};
use crate::app::{AppState, CommandQueue, ControllerState};
use crate::persistence::PersistentSettings;
use crate::protocol;

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
        ui.heading("Flight Controller Commands");

        if state.uart_sender.is_some() {
            if let Ok(address) = state.send_address.parse::<u16>() {
                render_command_buttons(ui, address, command_queue, persistent_settings, control);
                ui.separator();
                render_throttle_controls(ui, control, address, command_queue, persistent_settings);
            } else {
                ui.label("Enter valid address to enable commands");
            }
        } else {
            ui.label("Connect to serial port to enable commands");
        }
    });
}

/// Renders the flight command buttons
fn render_command_buttons(
    ui: &mut egui::Ui,
    address: u16,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
    control: &mut ControllerState,
) {
    ui.horizontal(|ui| {
        if ui.button("Start").clicked() {
            persistent_settings.is_manual_mode = false;
            if let Err(e) = protocol::send_command_start(command_queue, address) {
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
            if let Err(e) = protocol::send_command_start_manual(command_queue, address) {
                eprintln!("{}", e);
            }
        }
        ui.label("Start manual control mode");
    });
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        if ui.button("Stop").clicked()
            && let Err(e) = protocol::send_command_stop(command_queue, address)
        {
            eprintln!("{}", e);
        }
        ui.label("Disarm and stop motors normally");
    });
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        if ui.button("Emergency Stop").clicked()
            && let Err(e) = protocol::send_command_emergency_stop(command_queue, address)
        {
            eprintln!("{}", e);
        }
        ui.label("Immediate emergency shutdown");
    });
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        if ui.button("Calibrate").clicked()
            && let Err(e) = protocol::send_command_calibrate(command_queue, address)
        {
            eprintln!("{}", e);
        }
        ui.label("Calibrate IMU sensors");
    });
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        if ui.button("Reset").clicked()
            && let Err(e) = protocol::send_command_reset(command_queue, address)
        {
            eprintln!("{}", e);
        }
        ui.label("Reset flight controller state");
    });
}

/// Renders the throttle controls (base throttle and motor throttles)
fn render_throttle_controls(
    ui: &mut egui::Ui,
    control: &mut ControllerState,
    address: u16,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.label("Base Throttle");
    let mut throttle_clone = control.throttle;
    ui.add(Slider::new(&mut throttle_clone, 0.0..=1.0));
    control.throttle = throttle_clone;

    ui.separator();
    ui.label("Bias");

    // Master throttle
    ui.horizontal(|ui| {
        ui.label("Master");
        let mut master_clone = control.master_motor_throttle;
        let master_changed = ui
            .add(DragValue::new(&mut master_clone).range(0.0..=1.0).speed(0.01))
            .changed();
        if master_changed {
            control.master_motor_throttle = master_clone;
            control.motor_throttles = [master_clone; 4];
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_throttle(command_queue, address, control.motor_throttles);
        }
    });

    // Motors 1 & 3
    ui.horizontal(|ui| {
        ui.label("Motors 1 & 3");
        let mut motor_13_clone = control.motor_13_throttle;
        if ui
            .add(DragValue::new(&mut motor_13_clone).range(0.0..=1.0).speed(0.01))
            .changed()
        {
            control.motor_13_throttle = motor_13_clone;
            control.motor_throttles[0] = motor_13_clone;
            control.motor_throttles[2] = motor_13_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_throttle(command_queue, address, control.motor_throttles);
        }
    });

    // Motors 2 & 4
    ui.horizontal(|ui| {
        ui.label("Motors 2 & 4");
        let mut motor_24_clone = control.motor_24_throttle;
        if ui
            .add(DragValue::new(&mut motor_24_clone).range(0.0..=1.0).speed(0.01))
            .changed()
        {
            control.motor_24_throttle = motor_24_clone;
            control.motor_throttles[1] = motor_24_clone;
            control.motor_throttles[3] = motor_24_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_throttle(command_queue, address, control.motor_throttles);
        }
    });

    // Individual motor controls
    render_individual_motor_control(ui, control, 0, "Motor 1", address, command_queue, persistent_settings);
    render_individual_motor_control(ui, control, 1, "Motor 2", address, command_queue, persistent_settings);
    render_individual_motor_control(ui, control, 2, "Motor 3", address, command_queue, persistent_settings);
    render_individual_motor_control(ui, control, 3, "Motor 4", address, command_queue, persistent_settings);

    ui.label("Set Point");
}

/// Renders a single motor control value box
fn render_individual_motor_control(
    ui: &mut egui::Ui,
    control: &mut ControllerState,
    motor_index: usize,
    label: &str,
    address: u16,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut motor_clone = control.motor_throttles[motor_index];
        if ui
            .add(DragValue::new(&mut motor_clone).range(0.0..=1.0).speed(0.01))
            .changed()
        {
            control.motor_throttles[motor_index] = motor_clone;
            persistent_settings.motor_throttles = control.motor_throttles;
            send_motor_throttle(command_queue, address, control.motor_throttles);
        }
    });
}

/// Helper function to send motor throttle command
fn send_motor_throttle(command_queue: &CommandQueue, address: u16, throttles: [f32; 4]) {
    if let Err(e) = protocol::send_command_set_motor_throttle(command_queue, address, throttles) {
        eprintln!("Failed to send motor throttle: {}", e);
    }
}
