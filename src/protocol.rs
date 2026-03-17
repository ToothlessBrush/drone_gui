use bytemuck::{Pod, Zeroable};

use crate::app::CommandQueue;

// Binary protocol type bytes - matches bluetooth.h BT_CMD_* constants
const BT_CMD_CALIBRATE: u8 = 0x01;
const BT_CMD_SET_PID: u8 = 0x02;
const BT_CMD_SET_MOTOR_BIAS: u8 = 0x03;
const BT_CMD_CONFIG: u8 = 0x04;
const BT_CMD_SAVE: u8 = 0x05;

/// CRC8-DVB-S2 - matches firmware implementation
fn crc8_dvb_s2(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0xD5
            } else {
                crc << 1
            };
        }
    }
    crc
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
pub struct MotorBiasPacket {
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

    // PID terms for velocity x
    pub velocity_x_kp: f32,
    pub velocity_x_ki: f32,
    pub velocity_x_kd: f32,
    pub velocity_x_i_limit: f32,
    pub velocity_x_pid_limit: f32,

    // PID terms for velocity y
    pub velocity_y_kp: f32,
    pub velocity_y_ki: f32,
    pub velocity_y_kd: f32,
    pub velocity_y_i_limit: f32,
    pub velocity_y_pid_limit: f32,
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
pub enum SelectPID {
    Pitch = 0x0,
    #[default]
    Roll = 0x1,
    Yaw = 0x2,
    VelocityX = 0x3,
    VelocityY = 0x4,
}

/// Commands supported over Bluetooth serial - matches BT_CMD_* in bluetooth.h
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    Calibrate,
    TunePID(PIDTunePacket),
    SetMotorBias(MotorBiasPacket),
    Config(ConfigPacket),
    Save,
}

impl CommandType {
    /// Encode command as a binary frame: 0xA5 | TYPE | LEN | PAYLOAD | CRC8
    pub fn to_binary_frame(&self) -> Vec<u8> {
        let (type_byte, payload): (u8, &[u8]) = match self {
            CommandType::Calibrate => (BT_CMD_CALIBRATE, &[]),
            CommandType::TunePID(p) => (BT_CMD_SET_PID, bytemuck::bytes_of(p)),
            CommandType::SetMotorBias(m) => (BT_CMD_SET_MOTOR_BIAS, bytemuck::bytes_of(m)),
            CommandType::Config(c) => (BT_CMD_CONFIG, bytemuck::bytes_of(c)),
            CommandType::Save => (BT_CMD_SAVE, &[]),
        };

        let len = payload.len() as u8;

        // CRC covers type + len + payload
        let mut crc_input = vec![type_byte, len];
        crc_input.extend_from_slice(payload);
        let crc = crc8_dvb_s2(&crc_input);

        let mut frame = vec![0xA5u8, type_byte, len];
        frame.extend_from_slice(payload);
        frame.push(crc);
        frame
    }
}

pub fn send_command_calibrate(queue: &CommandQueue) -> Result<(), String> {
    queue.enqueue(CommandType::Calibrate);
    Ok(())
}

pub fn send_command_tune_pid(
    queue: &CommandQueue,
    axis: SelectPID,
    pid: PIDController,
) -> Result<(), String> {
    queue.enqueue(CommandType::TunePID(PIDTunePacket {
        p: pid.p,
        i: pid.i,
        d: pid.d,
        i_limit: pid.i_limit,
        pid_limit: pid.pid_limit,
        axis: axis as u8,
    }));
    Ok(())
}

pub fn send_command_set_motor_bias(
    queue: &CommandQueue,
    motor_biases: [f32; 4],
) -> Result<(), String> {
    queue.enqueue(CommandType::SetMotorBias(MotorBiasPacket {
        motor1: motor_biases[0],
        motor2: motor_biases[1],
        motor3: motor_biases[2],
        motor4: motor_biases[3],
    }));
    Ok(())
}

pub fn send_command_config(queue: &CommandQueue, config: ConfigPacket) -> Result<(), String> {
    queue.enqueue(CommandType::Config(config));
    Ok(())
}

pub fn send_command_save(queue: &CommandQueue) -> Result<(), String> {
    queue.enqueue(CommandType::Save);
    Ok(())
}
