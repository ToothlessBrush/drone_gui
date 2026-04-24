use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::protocol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidParameters {
    pub p: f32,
    pub i: f32,
    pub d: f32,
    pub i_limit: f32,
    pub pid_limit: f32,
}

impl Default for PidParameters {
    fn default() -> Self {
        Self {
            p: 1.0,
            i: 0.0,
            d: 0.0,
            i_limit: 10.0,
            pid_limit: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct PersistentSettings {
    // PID parameters for each axis
    #[serde(default)]
    pub pid_roll: PidParameters,
    #[serde(default)]
    pub pid_pitch: PidParameters,
    #[serde(default)]
    pub pid_yaw: PidParameters,
    #[serde(default)]
    pub pid_velocity_x: PidParameters,
    #[serde(default)]
    pub pid_velocity_y: PidParameters,

    // Flight config: throttle curve + angle sensitivity
    #[serde(default = "default_throttle_hover")]
    pub throttle_hover: f32,
    #[serde(default = "default_throttle_expo")]
    pub throttle_expo: f32,
    #[serde(default = "default_max_roll_angle")]
    pub max_roll_angle: f32,
    #[serde(default = "default_max_pitch_angle")]
    pub max_pitch_angle: f32,
    #[serde(default = "default_max_yaw_rate")]
    pub max_yaw_rate: f32,

    // Currently selected axis for tuning (not persisted, just for UI state)
    #[serde(skip)]
    pub selected_tune_axis: protocol::SelectPID,
}

fn default_throttle_hover() -> f32 { 0.45 }
fn default_throttle_expo() -> f32 { 0.6 }
fn default_max_roll_angle() -> f32 { 0.5 }
fn default_max_pitch_angle() -> f32 { 0.5 }
fn default_max_yaw_rate() -> f32 { 1.571 }

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            pid_roll: PidParameters::default(),
            pid_pitch: PidParameters::default(),
            pid_yaw: PidParameters::default(),
            pid_velocity_x: PidParameters::default(),
            pid_velocity_y: PidParameters::default(),
            throttle_hover: default_throttle_hover(),
            throttle_expo: default_throttle_expo(),
            max_roll_angle: default_max_roll_angle(),
            max_pitch_angle: default_max_pitch_angle(),
            max_yaw_rate: default_max_yaw_rate(),
            selected_tune_axis: protocol::SelectPID::Roll,
        }
    }
}

impl PersistentSettings {
    fn settings_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_config_dir = config_dir.join("drone_gui");
        let _ = fs::create_dir_all(&app_config_dir);
        app_config_dir.join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::settings_path();

        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(settings) => {
                    println!("Loaded settings from {:?}", path);
                    settings
                }
                Err(e) => {
                    eprintln!("Failed to parse settings file: {}", e);
                    Self::default()
                }
            },
            Err(_) => {
                println!("No settings file found, using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path();

        match serde_json::to_string_pretty(self) {
            Ok(json) => match fs::write(&path, json) {
                Ok(()) => Ok(()),
                Err(e) => Err(format!("Failed to write settings file: {}", e)),
            },
            Err(e) => Err(format!("Failed to serialize settings: {}", e)),
        }
    }

    pub fn get_pid(&self, axis: protocol::SelectPID) -> &PidParameters {
        match axis {
            protocol::SelectPID::Roll => &self.pid_roll,
            protocol::SelectPID::Pitch => &self.pid_pitch,
            protocol::SelectPID::Yaw => &self.pid_yaw,
            protocol::SelectPID::VelocityX => &self.pid_velocity_x,
            protocol::SelectPID::VelocityY => &self.pid_velocity_y,
        }
    }

    pub fn get_pid_mut(&mut self, axis: protocol::SelectPID) -> &mut PidParameters {
        match axis {
            protocol::SelectPID::Roll => &mut self.pid_roll,
            protocol::SelectPID::Pitch => &mut self.pid_pitch,
            protocol::SelectPID::Yaw => &mut self.pid_yaw,
            protocol::SelectPID::VelocityX => &mut self.pid_velocity_x,
            protocol::SelectPID::VelocityY => &mut self.pid_velocity_y,
        }
    }

    pub fn to_config_packet(&self) -> protocol::ConfigPacket {
        protocol::ConfigPacket {
            throttle_hover: self.throttle_hover,
            throttle_expo: self.throttle_expo,
            max_roll_angle: self.max_roll_angle,
            max_pitch_angle: self.max_pitch_angle,
            max_yaw_rate: self.max_yaw_rate,
        }
    }
}

pub fn auto_save_system(settings: Res<PersistentSettings>) {
    if settings.is_changed()
        && !settings.is_added()
        && let Err(e) = settings.save()
    {
        eprintln!("Failed to auto-save settings: {}", e);
    }
}
