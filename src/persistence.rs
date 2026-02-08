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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorBias {
    pub motor1: f32,
    pub motor2: f32,
    pub motor3: f32,
    pub motor4: f32,
}

impl Default for MotorBias {
    fn default() -> Self {
        Self {
            motor1: 0.0,
            motor2: 0.0,
            motor3: 0.0,
            motor4: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct PersistentSettings {
    // Motor bias values
    #[serde(default)]
    pub motor_bias: MotorBias,

    // PID parameters for each axis
    #[serde(default)]
    pub pid_roll: PidParameters,
    #[serde(default)]
    pub pid_pitch: PidParameters,
    #[serde(default)]
    pub pid_yaw: PidParameters,

    // Individual motor throttle values
    #[serde(default)]
    pub motor_throttles: [f32; 4],

    // Currently selected axis for tuning (not persisted, just for UI state)
    #[serde(skip)]
    pub selected_tune_axis: protocol::Axis,

    // Track if we're in manual mode (not serialized)
    #[serde(skip)]
    pub is_manual_mode: bool,
}

impl Default for PersistentSettings {
    fn default() -> Self {
        Self {
            motor_bias: MotorBias::default(),
            pid_roll: PidParameters::default(),
            pid_pitch: PidParameters::default(),
            pid_yaw: PidParameters::default(),
            motor_throttles: [0.0; 4],
            selected_tune_axis: protocol::Axis::Roll,
            is_manual_mode: false,
        }
    }
}

impl PersistentSettings {
    /// Get the path to the settings file
    fn settings_path() -> PathBuf {
        // Save settings in the user's config directory
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));

        let app_config_dir = config_dir.join("drone_gui");

        // Create the directory if it doesn't exist
        let _ = fs::create_dir_all(&app_config_dir);

        app_config_dir.join("settings.json")
    }

    /// Load settings from disk, or use defaults if file doesn't exist
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

    /// Save settings to disk
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

    /// Get PID parameters for a specific axis
    pub fn get_pid(&self, axis: protocol::Axis) -> &PidParameters {
        match axis {
            protocol::Axis::Roll => &self.pid_roll,
            protocol::Axis::Pitch => &self.pid_pitch,
            protocol::Axis::Yaw => &self.pid_yaw,
        }
    }

    /// Get mutable PID parameters for a specific axis
    pub fn get_pid_mut(&mut self, axis: protocol::Axis) -> &mut PidParameters {
        match axis {
            protocol::Axis::Roll => &mut self.pid_roll,
            protocol::Axis::Pitch => &mut self.pid_pitch,
            protocol::Axis::Yaw => &mut self.pid_yaw,
        }
    }

    /// Convert settings to ConfigPacket for sending to flight controller
    pub fn to_config_packet(&self) -> protocol::ConfigPacket {
        protocol::ConfigPacket {
            motor1: self.motor_throttles[0],
            motor2: self.motor_throttles[1],
            motor3: self.motor_throttles[2],
            motor4: self.motor_throttles[3],
            roll_kp: self.pid_roll.p,
            roll_ki: self.pid_roll.i,
            roll_kd: self.pid_roll.d,
            roll_i_limit: self.pid_roll.i_limit,
            roll_pid_limit: self.pid_roll.pid_limit,
            pitch_kp: self.pid_pitch.p,
            pitch_ki: self.pid_pitch.i,
            pitch_kd: self.pid_pitch.d,
            pitch_i_limit: self.pid_pitch.i_limit,
            pitch_pid_limit: self.pid_pitch.pid_limit,
            yaw_kp: self.pid_yaw.p,
            yaw_ki: self.pid_yaw.i,
            yaw_kd: self.pid_yaw.d,
            yaw_i_limit: self.pid_yaw.i_limit,
            yaw_pid_limit: self.pid_yaw.pid_limit,
        }
    }
}

/// System that automatically saves settings when they change
pub fn auto_save_system(settings: Res<PersistentSettings>) {
    if settings.is_changed() && !settings.is_added()
        && let Err(e) = settings.save() {
            eprintln!("Failed to auto-save settings: {}", e);
        }
}
