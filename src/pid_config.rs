use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// PID configuration for a single axis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisPidConfig {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub ki_limit: f32,
    pub limit: f32,
}

impl Default for AxisPidConfig {
    fn default() -> Self {
        Self {
            kp: 0.1,
            ki: 0.0,
            kd: 0.0,
            ki_limit: 10.25,
            limit: 0.2,
        }
    }
}

/// Complete PID configuration matching C struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidConfig {
    pub roll: AxisPidConfig,
    pub pitch: AxisPidConfig,
    pub yaw: AxisPidConfig,
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            roll: AxisPidConfig {
                kp: 0.1,
                ki: 0.1,
                kd: 0.1,
                ki_limit: 10.25,
                limit: 0.2, // 20% throttle
            },
            pitch: AxisPidConfig {
                kp: 0.1,
                ki: 0.0,
                kd: 0.0,
                ki_limit: 10.25,
                limit: 0.2, // 20% throttle
            },
            yaw: AxisPidConfig {
                kp: 0.1,
                ki: 0.0,
                kd: 0.0,
                ki_limit: 10.25,
                limit: 0.1, // 10% throttle
            },
        }
    }
}

impl PidConfig {
    /// Load configuration from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let config: PidConfig = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }

    /// Get the axis config by index (0=roll, 1=pitch, 2=yaw)
    pub fn get_axis(&self, axis: u8) -> &AxisPidConfig {
        match axis {
            0 => &self.roll,
            1 => &self.pitch,
            2 => &self.yaw,
            _ => &self.roll,
        }
    }

    /// Get mutable axis config by index (0=roll, 1=pitch, 2=yaw)
    pub fn get_axis_mut(&mut self, axis: u8) -> &mut AxisPidConfig {
        match axis {
            0 => &mut self.roll,
            1 => &mut self.pitch,
            2 => &mut self.yaw,
            _ => &mut self.roll,
        }
    }
}

/// History entry for tracking PID configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidConfigHistoryEntry {
    pub timestamp: String,
    pub config: PidConfig,
    pub note: String,
}

impl PidConfigHistoryEntry {
    pub fn new(config: PidConfig, note: String) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            timestamp,
            config,
            note,
        }
    }
}

/// PID configuration history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidConfigHistory {
    pub entries: Vec<PidConfigHistoryEntry>,
}

impl Default for PidConfigHistory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl PidConfigHistory {
    /// Load history from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read history file: {}", e))?;

        let history: PidConfigHistory = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse history file: {}", e))?;

        Ok(history)
    }

    /// Save history to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize history: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Failed to write history file: {}", e))?;

        Ok(())
    }

    /// Add a new entry to history
    pub fn add_entry(&mut self, config: PidConfig, note: String) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.entries.push(PidConfigHistoryEntry {
            timestamp,
            config,
            note,
        });

        // Keep only last 50 entries
        if self.entries.len() > 50 {
            self.entries.drain(0..self.entries.len() - 50);
        }
    }
}
