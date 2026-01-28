mod app;
mod config;
mod drone_scene;
mod parser;
mod protocol;
mod telemetry;
mod uart;
mod video;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Drone Telemetry Monitor".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::audio::AudioPlugin>()
                .disable::<bevy::gltf::GltfPlugin>()
                .disable::<bevy::animation::AnimationPlugin>(),
            EguiPlugin,
        ))
        .add_systems(Startup, drone_scene::setup_drone_scene)
        .add_systems(Update, drone_scene::update_drone_orientation)
        .add_systems(
            Update,
            app::ui_system.after(drone_scene::update_drone_orientation),
        )
        .add_systems(Update, app::heartbeat_system)
        .insert_resource(app::AppState::default())
        .insert_resource(app::HeartbeatTimer::default())
        .insert_non_send_resource(app::GamepadState::default())
        .run();
}
