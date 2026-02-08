use bytemuck::{Pod, Zeroable};

use crate::app::CommandQueue;
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

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct HeartBeatPacket {
    pub base_throttle: f32,
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct PIDTunePacket {
    pub p: f32,
    pub i: f32,
    pub d: f32,
    pub i_limit: f32,
    pub pid_limit: f32,
    pub axis: u8,
}

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct MotorThrottlePacket {
    pub motor1: f32,
    pub motor2: f32,
    pub motor3: f32,
    pub motor4: f32,
}

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq)]
pub struct ConfigPacket {
    // motor bias
    pub motor1: f32,
    pub motor2: f32,
    pub motor3: f32,
    pub motor4: f32,

    // PID terms for roll
    pub roll_kp: f32,
    pub roll_ki: f32,
    pub roll_kd: f32,
    pub roll_i_limit: f32,
    pub roll_pid_limit: f32,

    // PID terms for pitch
    pub pitch_kp: f32,
    pub pitch_ki: f32,
    pub pitch_kd: f32,
    pub pitch_i_limit: f32,
    pub pitch_pid_limit: f32,

    // PID terms for yaw
    pub yaw_kp: f32,
    pub yaw_ki: f32,
    pub yaw_kd: f32,
    pub yaw_i_limit: f32,
    pub yaw_pid_limit: f32,
}

pub struct PIDController {
    pub p: f32,
    pub i: f32,
    pub d: f32,
    pub i_limit: f32,
    pub pid_limit: f32,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Axis {
    Pitch = 0x0,
    #[default]
    Roll = 0x1,
    Yaw = 0x2,
}

#[allow(unused)]
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
    StartManual,
    #[allow(unused)]
    SetThrottle(ThrottlePacket),
    #[allow(unused)]
    SetPoint(SetpointPacket),
    TunePID(PIDTunePacket),
    HeartBeat(HeartBeatPacket),
    SetMotorThrottle(MotorThrottlePacket),
    Config(ConfigPacket),
    Calibrate,
    Reset,
}

impl CommandType {
    pub fn get_ascii(&self) -> String {
        match self {
            // base commands
            CommandType::Start => "FC:START".to_string(),
            CommandType::Stop => "FC:STOP".to_string(),
            CommandType::EmergencyStop => "ES".to_string(),
            CommandType::StartManual => "FC:MANUAL".to_string(),
            CommandType::Calibrate => "FC:CALIBRATE".to_string(),
            CommandType::Reset => "FC:RESET".to_string(),

            // encoded commands
            CommandType::SetThrottle(throttle) => format!("ST:{}", throttle.to_hex()),
            CommandType::SetPoint(point) => format!("SP:{}", point.to_hex()),
            CommandType::HeartBeat(beat) => format!("HB:{}", beat.to_hex()),
            CommandType::TunePID(tune) => format!("TP:{}", tune.to_hex()),
            CommandType::SetMotorThrottle(mt) => format!("MB:{}", mt.to_hex()),
            CommandType::Config(config) => format!("CF:{}", config.to_hex()),
        }
    }
}

pub fn send_command_start(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::Start);
    Ok(())
}

pub fn send_command_stop(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::Stop);
    Ok(())
}

pub fn send_command_emergency_stop(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::EmergencyStop);
    Ok(())
}

pub fn send_command_start_manual(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::StartManual);
    Ok(())
}

pub fn send_command_calibrate(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::Calibrate);
    Ok(())
}

pub fn send_command_reset(queue: &CommandQueue, address: u16) -> Result<(), String> {
    queue.enqueue(address, CommandType::Reset);
    Ok(())
}

#[allow(unused)]
pub fn send_command_set_throttle(
    queue: &CommandQueue,
    address: u16,
    throttle_value: f32,
) -> Result<(), String> {
    queue.enqueue(
        address,
        CommandType::SetThrottle(ThrottlePacket(throttle_value)),
    );
    Ok(())
}

#[allow(unused)]
pub fn send_command_set_point(
    queue: &CommandQueue,
    address: u16,
    attitude: Attitude,
) -> Result<(), String> {
    queue.enqueue(
        address,
        CommandType::SetPoint(SetpointPacket {
            roll: attitude.roll,
            pitch: attitude.pitch,
            yaw: attitude.yaw,
        }),
    );
    Ok(())
}

pub fn send_command_tune_pid(
    queue: &CommandQueue,
    address: u16,
    axis: Axis,
    pid: PIDController,
) -> Result<(), String> {
    queue.enqueue(
        address,
        CommandType::TunePID(PIDTunePacket {
            p: pid.p,
            i: pid.i,
            d: pid.d,
            i_limit: pid.i_limit,
            pid_limit: pid.pid_limit,
            axis: axis as u8,
        }),
    );
    Ok(())
}

pub fn send_command_set_motor_throttle(
    queue: &CommandQueue,
    address: u16,
    motor_throttles: [f32; 4],
) -> Result<(), String> {
    queue.enqueue(
        address,
        CommandType::SetMotorThrottle(MotorThrottlePacket {
            motor1: motor_throttles[0],
            motor2: motor_throttles[1],
            motor3: motor_throttles[2],
            motor4: motor_throttles[3],
        }),
    );
    Ok(())
}

pub fn send_command_config(
    queue: &CommandQueue,
    address: u16,
    config: ConfigPacket,
) -> Result<(), String> {
    queue.enqueue(address, CommandType::Config(config));
    Ok(())
}
