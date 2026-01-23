// Command protocol - matches FlightController/src/protocol.h
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    Start,
    Stop,
    EmergencyStop,
    SetThrottle(f32),
    Calibrate,
    Reset,
}

impl CommandType {
    pub fn to_ascii(&self) -> String {
        match self {
            CommandType::Start => "FC:START".to_string(),
            CommandType::Stop => "FC:STOP".to_string(),
            CommandType::EmergencyStop => "FC:EMERGENCY".to_string(),
            CommandType::SetThrottle(throttle) => format!("FC:SET_THROTTLE:{}", throttle),
            CommandType::Calibrate => "FC:CALIBRATE".to_string(),
            CommandType::Reset => "FC:RESET".to_string(),
        }
    }
}
