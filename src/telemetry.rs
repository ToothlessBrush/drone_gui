use bytemuck::{Pod, Zeroable};
use chrono::{DateTime, Local};
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
    pub timestamp: u32,
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
    // Gyroscope angular velocity (rad/s)
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,
    // Optical flow velocity
    pub vel_x: f32,
    pub vel_y: f32,
    pub vel_z: f32,
    // Height above ground (m)
    pub height: f32,
    // Motor throttle outputs (0.0-1.0)
    pub motor1: f32,
    pub motor2: f32,
    pub motor3: f32,
    pub motor4: f32,
    // Commanded setpoints from pilot sticks
    pub input_throttle: f32,
    pub input_roll: f32,
    pub input_pitch: f32,
    pub input_yaw: f32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TelemetryPacket {
    timestamp_ms: u32,

    roll: f32,
    pitch: f32,
    yaw: f32,

    roll_p_term: f32,
    roll_i_term: f32,
    roll_d_term: f32,

    pitch_p_term: f32,
    pitch_i_term: f32,
    pitch_d_term: f32,

    yaw_p_term: f32,
    yaw_i_term: f32,
    yaw_d_term: f32,

    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,

    vel_x: f32,
    vel_y: f32,
    vel_z: f32,

    height: f32,

    motor1: f32,
    motor2: f32,
    motor3: f32,
    motor4: f32,

    input_throttle: f32,
    input_roll: f32,
    input_pitch: f32,
    input_yaw: f32,
}

impl From<&TelemetryPacket> for TelemetryData {
    fn from(packet: &TelemetryPacket) -> Self {
        Self {
            timestamp: packet.timestamp_ms,
            clock_time: Local::now(),
            roll: packet.roll,
            pitch: packet.pitch,
            yaw: packet.yaw,
            roll_p: packet.roll_p_term,
            roll_i: packet.roll_i_term,
            roll_d: packet.roll_d_term,
            pitch_p: packet.pitch_p_term,
            pitch_i: packet.pitch_i_term,
            pitch_d: packet.pitch_d_term,
            yaw_p: packet.yaw_p_term,
            yaw_i: packet.yaw_i_term,
            yaw_d: packet.yaw_d_term,
            gyro_x: packet.gyro_x,
            gyro_y: packet.gyro_y,
            gyro_z: packet.gyro_z,
            vel_x: packet.vel_x,
            vel_y: packet.vel_y,
            vel_z: packet.vel_z,
            height: packet.height,
            motor1: packet.motor1,
            motor2: packet.motor2,
            motor3: packet.motor3,
            motor4: packet.motor4,
            input_throttle: packet.input_throttle,
            input_roll: packet.input_roll,
            input_pitch: packet.input_pitch,
            input_yaw: packet.input_yaw,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogMessage {
    pub _timestamp: f64,
    pub clock_time: DateTime<Local>,
    pub message: String,
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

    pub fn clear_data(&mut self) {
        self.data.clear();
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    pub fn push(&mut self, mut telem: TelemetryData) {
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

}
