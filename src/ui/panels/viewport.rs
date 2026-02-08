use bevy_egui::egui;
use egui::Color32;
use crate::app::AppState;

/// Renders the 3D viewport section with orientation display
pub fn render_viewport_section(
    ui: &mut egui::Ui,
    state: &AppState,
    width: f32,
) {
    ui.vertical(|ui| {
        ui.label("3D Drone View");
        ui.set_width(width);
        let viewport_height = width * 0.75; // Match render target aspect

        if let Some(texture_id) = state.viewport_texture_id {
            ui.image(egui::load::SizedTexture::new(
                texture_id,
                egui::vec2(width, viewport_height),
            ));
        } else {
            ui.allocate_space(egui::vec2(width, viewport_height));
            ui.label("Loading 3D view...");
        }

        // Current values in a styled box
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                let buffer = state.data_buffer.lock().unwrap();
                if let Some(latest) = buffer.data.back() {
                    ui.vertical(|ui| {
                        // Roll with red background
                        ui.scope(|ui| {
                            egui::Frame::none()
                                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                .fill(Color32::from_rgb(80, 20, 20))
                                .rounding(egui::Rounding::same(4.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Roll: {:.2}°",
                                            latest.roll.to_degrees()
                                        ))
                                        .color(Color32::from_rgb(255, 100, 100))
                                        .monospace(),
                                    );
                                });
                        });

                        // Pitch with green background
                        ui.scope(|ui| {
                            egui::Frame::none()
                                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                .fill(Color32::from_rgb(20, 80, 20))
                                .rounding(egui::Rounding::same(4.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Pitch: {:.2}°",
                                            latest.pitch.to_degrees()
                                        ))
                                        .color(Color32::from_rgb(100, 255, 100))
                                        .monospace(),
                                    );
                                });
                        });

                        // Yaw with blue background
                        ui.scope(|ui| {
                            egui::Frame::none()
                                .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                .fill(Color32::from_rgb(20, 20, 80))
                                .rounding(egui::Rounding::same(4.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Yaw: {:.2}°",
                                            latest.yaw.to_degrees()
                                        ))
                                        .color(Color32::from_rgb(100, 100, 255))
                                        .monospace(),
                                    );
                                });
                        });
                    });
                } else {
                    ui.label("No data received yet");
                }
            });
    });
}
