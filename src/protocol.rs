use std::mem;

// Command protocol - matches FlightController/src/protocol.h
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    None,
    Start,
    Stop,
    EmergencyStop,
    UpdatePid,
    SetThrottle(f32),
    SetAttitude,
    Calibrate,
    Reset,
}

impl CommandType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(CommandType::None),
            0x01 => Some(CommandType::Start),
            0x02 => Some(CommandType::Stop),
            0x03 => Some(CommandType::EmergencyStop),
            0x04 => Some(CommandType::UpdatePid),
            0x05 => Some(CommandType::SetThrottle(0.0)),
            0x06 => Some(CommandType::SetAttitude),
            0x07 => Some(CommandType::Calibrate),
            0x08 => Some(CommandType::Reset),
            _ => None,
        }
    }

    pub fn to_ascii(&self) -> String {
        match self {
            CommandType::None => "".to_string(),
            CommandType::Start => "FC:START".to_string(),
            CommandType::Stop => "FC:STOP".to_string(),
            CommandType::EmergencyStop => "FC:EMERGENCY".to_string(),
            CommandType::UpdatePid => "FC:UPDATE_PID".to_string(),
            CommandType::SetThrottle(throttle) => format!("FC:SET_THROTTLE:{}", throttle),
            CommandType::SetAttitude => "FC:SET_ATTITUDE".to_string(),
            CommandType::Calibrate => "FC:CALIBRATE".to_string(),
            CommandType::Reset => "FC:RESET".to_string(),
        }
    }
}

// Basic command structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub command: u8,
    pub reserved: u8,
}

// PID update command payload
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CommandUpdatePid {
    pub command: u8,
    pub axis: u8, // 0=roll, 1=pitch, 2=yaw
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
}

// Throttle command payload
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CommandSetThrottle {
    pub command: u8,
    pub reserved: u8,
    pub throttle: f32, // 0.0 to 1.0
}

// Attitude setpoint command payload
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CommandSetAttitude {
    pub command: u8,
    pub reserved: u8,
    pub roll: f32,  // radians
    pub pitch: f32, // radians
    pub yaw: f32,   // radians
}

// Telemetry packet structure - matches C struct exactly
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TelemetryPacket {
    pub timestamp_ms: u32,
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,
    pub pid_roll_output: f32,
    pub pid_pitch_output: f32,
    pub pid_yaw_output: f32,
    pub roll_p_term: f32,
    pub roll_i_term: f32,
    pub roll_d_term: f32,
    pub pitch_p_term: f32,
    pub pitch_i_term: f32,
    pub pitch_d_term: f32,
    pub yaw_p_term: f32,
    pub yaw_i_term: f32,
    pub yaw_d_term: f32,
    pub altitude: f32,
    pub battery_volt: f32,
    pub state: u8,
    pub flags: u8,
}

impl TelemetryPacket {
    pub const SIZE: usize = mem::size_of::<TelemetryPacket>();

    /// Parse telemetry packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        // Safety: We're reading from a byte slice into a packed struct
        // The struct is marked as packed and repr(C) to match the C layout
        unsafe {
            let packet_ptr = data.as_ptr() as *const TelemetryPacket;
            Some(*packet_ptr)
        }
    }

    /// Convert to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        unsafe {
            let ptr = self as *const TelemetryPacket as *const u8;
            let slice = std::slice::from_raw_parts(ptr, Self::SIZE);
            slice.to_vec()
        }
    }
}
