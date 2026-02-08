use bevy::prelude::*;

use crate::{
    app::{CommandQueue, ControllerState},
    protocol,
};

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

    // Maximum tilt angle in radians (1 degree)
    const MAX_TILT_ANGLE: f32 = 1.0_f32.to_radians();

    // Left stick Y-axis: pitch (inverted so up is positive)
    if let Some(value) = gamepad.get(GamepadAxis::LeftStickY) {
        controller_state.pitch = -value * MAX_TILT_ANGLE; // Invert Y axis and scale to max angle
    }

    // Left stick X-axis: yaw
    if let Some(value) = gamepad.get(GamepadAxis::LeftStickX) {
        controller_state.yaw = value;
    }

    // Right stick Y-axis: throttle adjustment (up increases, down decreases)
    if let Some(value) = gamepad.get(GamepadAxis::RightStickY) {
        // Inverted: up is positive, down is negative
        let adjustment = value * time.delta_secs() * 0.15; // 0.5 = throttle change rate
        controller_state.throttle = (controller_state.throttle + adjustment).clamp(0.0, 1.0);
    }

    // Right stick X-axis: roll
    if let Some(value) = gamepad.get(GamepadAxis::RightStickX) {
        controller_state.roll = value * MAX_TILT_ANGLE;
    }

    if gamepad.pressed(GamepadButton::Start)
        && let Err(e) = protocol::send_command_emergency_stop(&command_queue, 2)
    {
        eprintln!("EMERGENCY FAILED RUN: {e}");
    }
}
