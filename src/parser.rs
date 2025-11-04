use chrono::Local;

use crate::telemetry::{ReceivedMessage, TelemetryData};

pub fn parse_rcv(line: &str) -> Option<ReceivedMessage> {
    let parts: Vec<&str> = line.strip_prefix("+RCV=")?.split(",").collect();

    let address: u32 = parts[0].parse().ok()?;
    let length: u32 = parts[1].parse().ok()?;
    let message = parts[2].to_string();
    let rssi: i32 = parts[3].parse().ok()?;
    let snr: i32 = parts[4].parse().ok()?;

    Some(ReceivedMessage {
        from: address,
        length,
        message,
        rssi,
        snr,
        time: Local::now(),
    })
}

/// Parse telemetry from serial data
/// Format: "TELEM:roll,pitch,yaw,roll_p,roll_i,roll_d,pitch_p,pitch_i,pitch_d,yaw_p,yaw_i,yaw_d,alt,voltage"
pub fn parse_telemetry(line: &str) -> Option<TelemetryData> {
    let parts: Vec<&str> = line.split([',', ':']).collect();

    if parts.len() >= 15 && parts[0] == "TELEM" {
        Some(TelemetryData {
            timestamp: 0.0,
            clock_time: Local::now(),
            roll: parts[1].parse().ok()?,
            pitch: parts[2].parse().ok()?,
            yaw: parts[3].parse().ok()?,
            roll_p: parts[4].parse().ok()?,
            roll_i: parts[5].parse().ok()?,
            roll_d: parts[6].parse().ok()?,
            pitch_p: parts[7].parse().ok()?,
            pitch_i: parts[8].parse().ok()?,
            pitch_d: parts[9].parse().ok()?,
            yaw_p: parts[10].parse().ok()?,
            yaw_i: parts[11].parse().ok()?,
            yaw_d: parts[12].parse().ok()?,
            altitude: parts[13].parse().ok()?,
            battery_voltage: parts[14].parse().ok()?,
        })
    } else {
        None
    }
}

/// Parse log message from serial data
/// Format: "LOG:message text here"
pub fn parse_log(line: &str) -> Option<String> {
    line.strip_prefix("LOG:").map(str::to_string)
}
