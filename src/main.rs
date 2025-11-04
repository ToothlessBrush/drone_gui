mod app;
mod config;
mod parser;
mod telemetry;
mod uart;

use app::MyEguiApp;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Drone Telemetry",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .expect("failed to run eframe");
}
