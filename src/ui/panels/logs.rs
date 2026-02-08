use bevy_egui::egui;
use crate::app::AppState;

/// Renders the system logs section
pub fn render_logs_section(
    ui: &mut egui::Ui,
    state: &AppState,
    width: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);
        let mut buffer = state.data_buffer.lock().unwrap();
        ui.label(format!("System Logs ({} messages)", buffer.logs.len()));

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .id_salt("system_logs")
            .auto_shrink([false; 2])
            .stick_to_bottom(state.auto_scroll_logs)
            .show(ui, |ui| {
                if ui.button("clear logs").clicked() {
                    buffer.clear_logs();
                }

                for log in buffer.logs.iter() {
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", log.clock_time.format("%H:%M:%S%.3f")));
                        ui.label(&log.message);
                    });
                }
            });
    });
}
