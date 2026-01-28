use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};

use crate::uart::UartCommand;
pub trait ToHex: Pod {
    fn to_hex(&self) -> String {
        bytemuck::bytes_of(self)
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect()
    }
}

impl<T: Pod> ToHex for T {}

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct ThrottlePacket(pub f32);

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct SetpointPacket {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

pub struct Attitude {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

// Command protocol - matches FlightController/src/protocol.h
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    Start,
    Stop,
    EmergencyStop,
    SetThrottle(ThrottlePacket),
    SetPoint(SetpointPacket),
    Calibrate,
    Reset,
}

impl CommandType {
    pub fn to_ascii(&self) -> String {
        match self {
            // base commands
            CommandType::Start => "FC:START".to_string(),
            CommandType::Stop => "FC:STOP".to_string(),
            CommandType::EmergencyStop => "FC:EMERGENCY".to_string(),
            CommandType::Calibrate => "FC:CALIBRATE".to_string(),
            CommandType::Reset => "FC:RESET".to_string(),

            // encoded commands
            CommandType::SetThrottle(throttle) => format!("ST:{}", throttle.to_hex()),
            CommandType::SetPoint(point) => format!("SP:{}", point.to_hex()),
        }
    }
}

pub fn send_command_start(tx: &mpsc::Sender<UartCommand>, address: u16) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::Start.to_ascii().to_string(),
    })
    .map_err(|e| format!("Failed to send START command: {}", e))
}

pub fn send_command_stop(tx: &mpsc::Sender<UartCommand>, address: u16) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::Stop.to_ascii().to_string(),
    })
    .map_err(|e| format!("Failed to send STOP command: {}", e))
}

pub fn send_command_emergency_stop(
    tx: &mpsc::Sender<UartCommand>,
    address: u16,
) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::EmergencyStop.to_ascii().to_string(),
    })
    .map_err(|e| format!("Failed to send EMERGENCY_STOP command: {}", e))
}

pub fn send_command_calibrate(tx: &mpsc::Sender<UartCommand>, address: u16) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::Calibrate.to_ascii().to_string(),
    })
    .map_err(|e| format!("Failed to send CALIBRATE command: {}", e))
}

pub fn send_command_reset(tx: &mpsc::Sender<UartCommand>, address: u16) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::Reset.to_ascii().to_string(),
    })
    .map_err(|e| format!("Failed to send RESET command: {}", e))
}

pub fn send_command_set_throttle(
    tx: &mpsc::Sender<UartCommand>,
    address: u16,
    throttle_value: f32,
) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::SetThrottle(ThrottlePacket(throttle_value))
            .to_ascii()
            .to_string(),
    })
    .map_err(|e| format!("Failed to send SET_THROTTLE command: {}", e))
}

pub fn send_command_set_point(
    tx: &mpsc::Sender<UartCommand>,
    address: u16,
    attitude: Attitude,
) -> Result<(), String> {
    tx.send(UartCommand::Send {
        address,
        data: CommandType::SetPoint(SetpointPacket {
            roll: attitude.roll,
            pitch: attitude.pitch,
            yaw: attitude.yaw,
        })
        .to_ascii()
        .to_string(),
    })
    .map_err(|e| format!("Failed to send SET_POINT command: {}", e))
}
