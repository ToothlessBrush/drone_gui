pub mod panels;
pub mod windows;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::app::{AppState, CommandQueue, ControllerState};
use crate::drone_scene::{Drone, DroneOrientation, ViewportImage};
use crate::persistence::PersistentSettings;

/// Main UI system that renders all the egui panels
pub fn ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<AppState>,
    mut control: ResMut<ControllerState>,
    mut drone_query: Query<&mut DroneOrientation, With<Drone>>,
    viewport_image: Res<ViewportImage>,
    command_queue: Res<CommandQueue>,
    mut persistent_settings: ResMut<PersistentSettings>,
) {
    // Register the viewport image with egui context if not already done
    if state.viewport_texture_id.is_none() {
        let egui_texture_id = contexts.add_image(viewport_image.handle.clone());
        state.viewport_texture_id = Some(egui_texture_id);
        println!("Registered viewport texture with egui: {:?}", egui_texture_id);
    }

    // Update video texture if new frame is available
    update_video_texture(&mut contexts, &mut state);

    // Update drone orientation from telemetry
    update_drone_orientation(&state, &mut drone_query);

    let ctx = contexts.ctx_mut();
    ctx.request_repaint();

    // Top Panel - Connection controls
    render_top_panel(ctx, &mut state, &command_queue, &persistent_settings);

    // Central Panel - Main content
    render_central_panel(ctx, &mut state, &mut control, &command_queue, &mut persistent_settings);

    // PID Tuning Window
    windows::render_pid_tuning_window(ctx, &mut state, &command_queue, &mut persistent_settings);
}

/// Updates the video texture if a new frame is available
fn update_video_texture(contexts: &mut EguiContexts, state: &mut AppState) {
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
}

/// Updates the drone orientation in the 3D scene from telemetry data
fn update_drone_orientation(
    state: &AppState,
    drone_query: &mut Query<&mut DroneOrientation, With<Drone>>,
) {
    if let Ok(buffer) = state.data_buffer.lock()
        && let Some(latest) = buffer.data.back()
    {
        for mut orientation in drone_query.iter_mut() {
            orientation.roll = latest.roll;
            orientation.pitch = latest.pitch;
            orientation.yaw = latest.yaw;
        }
    }
}

/// Renders the top connection panel
fn render_top_panel(
    ctx: &egui::Context,
    state: &mut AppState,
    command_queue: &CommandQueue,
    persistent_settings: &PersistentSettings,
) {
    egui::TopBottomPanel::top("top_panel")
        .frame(egui::Frame {
            inner_margin: egui::Margin::same(8.0),
            fill: ctx.style().visuals.window_fill(),
            ..Default::default()
        })
        .show(ctx, |ui| {
            panels::render_connection_panel(ui, state, command_queue, persistent_settings);
        });
}

/// Renders the central panel with main content
fn render_central_panel(
    ctx: &egui::Context,
    state: &mut AppState,
    control: &mut ControllerState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame {
            inner_margin: egui::Margin::same(8.0),
            fill: ctx.style().visuals.window_fill(),
            ..Default::default()
        })
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Horizontal layout: View | Commands | Log
                    render_main_sections(ui, state, control, command_queue, persistent_settings);

                    // Clear plots button
                    if ui.button("clear plots").clicked() {
                        state.data_buffer.lock().unwrap().clear_data();
                    }

                    // Attitude and PID plots
                    panels::render_attitude_plot(ui, state);
                    panels::render_pid_plot(ui, state);
                });
        });
}

/// Renders the three main sections (viewport, commands, logs)
fn render_main_sections(
    ui: &mut egui::Ui,
    state: &mut AppState,
    control: &mut ControllerState,
    command_queue: &CommandQueue,
    persistent_settings: &mut PersistentSettings,
) {
    ui.horizontal_top(|ui| {
        let available_width = ui.available_width();
        let left_width = available_width * 0.25; // 25% for 3D view
        let middle_width = available_width * 0.20; // 20% for commands
        let right_width = available_width * 0.55; // 55% for logs

        // 3D Viewport Section
        ui.group(|ui| {
            panels::render_viewport_section(ui, state, left_width);
        });

        // Flight Controller Commands Section
        ui.group(|ui| {
            panels::render_commands_section(
                ui,
                state,
                control,
                command_queue,
                persistent_settings,
                middle_width,
            );
        });

        // System Logs Section
        ui.group(|ui| {
            panels::render_logs_section(ui, state, right_width);
        });
    });
}
