use chrono::{DateTime, Local};
use egui_plot::PlotPoints;
use std::collections::VecDeque;

use crate::config::{MAX_LOG_MESSAGES, MAX_POINTS};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PidAxis {
    Roll,
    Pitch,
    Yaw,
}

#[derive(Clone, Debug)]
pub struct TelemetryData {
    pub timestamp: f64,
    pub clock_time: DateTime<Local>,
    // Attitude
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    // Roll PID
    pub roll_p: f32,
    pub roll_i: f32,
    pub roll_d: f32,
    // Pitch PID
    pub pitch_p: f32,
    pub pitch_i: f32,
    pub pitch_d: f32,
    // Yaw PID
    pub yaw_p: f32,
    pub yaw_i: f32,
    pub yaw_d: f32,
    // Other telemetry
    pub altitude: f32,
    pub battery_voltage: f32,
}

#[derive(Clone, Debug)]
pub struct LogMessage {
    pub _timestamp: f64,
    pub clock_time: DateTime<Local>,
    pub message: String,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct ReceivedMessage {
    pub from: u32,
    pub length: u32,
    pub message: String,
    pub rssi: i32,
    pub snr: i32,
    pub time: DateTime<Local>,
}

pub struct DataBuffer {
    pub data: VecDeque<TelemetryData>,
    pub logs: VecDeque<LogMessage>,
    start_time: std::time::Instant,
}

impl DataBuffer {
    pub fn new() -> Self {
        Self {
            data: VecDeque::with_capacity(MAX_POINTS),
            logs: VecDeque::with_capacity(MAX_LOG_MESSAGES),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn push(&mut self, mut telem: TelemetryData) {
        telem.timestamp = self.start_time.elapsed().as_secs_f64();
        telem.clock_time = Local::now();

        if self.data.len() >= MAX_POINTS {
            self.data.pop_front();
        }
        self.data.push_back(telem);
    }

    pub fn push_log(&mut self, message: String) {
        let log_msg = LogMessage {
            _timestamp: self.start_time.elapsed().as_secs_f64(),
            clock_time: Local::now(),
            message,
        };

        if self.logs.len() >= MAX_LOG_MESSAGES {
            self.logs.pop_front();
        }
        self.logs.push_back(log_msg);
    }

    pub fn get_roll_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.roll as f64])
            .collect()
    }

    pub fn get_pitch_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.pitch as f64])
            .collect()
    }

    pub fn get_yaw_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.yaw as f64])
            .collect()
    }

    pub fn get_pid_p_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_p,
                    PidAxis::Pitch => d.pitch_p,
                    PidAxis::Yaw => d.yaw_p,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }

    pub fn get_pid_i_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_i,
                    PidAxis::Pitch => d.pitch_i,
                    PidAxis::Yaw => d.yaw_i,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }

    pub fn get_pid_d_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_d,
                    PidAxis::Pitch => d.pitch_d,
                    PidAxis::Yaw => d.yaw_d,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }
}
