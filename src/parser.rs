use chrono::Local;

use crate::protocol::TelemetryPacket;
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
/// Format: "TELEM:roll:pitch:yaw:roll_p:roll_i:roll_d:pitch_p:pitch_i:pitch_d:yaw_p:yaw_i:yaw_d:alt:voltage"
/// Each field is a float formatted as [sign]whole.decimal (e.g., "0.123", "-1.456")
pub fn parse_telemetry(line: &str) -> Option<TelemetryData> {
    let parts: Vec<&str> = line.split(':').collect();

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

/// Parse binary telemetry packet from raw bytes
/// This is the more efficient format that matches the C TelemetryPacket struct
pub fn parse_binary_telemetry(data: &[u8]) -> Option<TelemetryData> {
    let packet = TelemetryPacket::from_bytes(data)?;

    Some(TelemetryData {
        timestamp: (packet.timestamp_ms as f64) / 1000.0,
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
        altitude: packet.altitude,
        battery_voltage: packet.battery_volt,
    })
}

/// Detect if data is binary telemetry (vs text)
/// Binary telemetry packets are exactly 84 bytes and don't start with printable ASCII
pub fn is_binary_telemetry(data: &[u8]) -> bool {
    if data.len() != TelemetryPacket::SIZE {
        return false;
    }

    // Binary packets have 4-byte little-endian timestamp at the start
    // Text packets (TELEM:...) start with 'T' (0x54)
    // Check if first byte looks like a reasonable timestamp lower byte
    // (not a printable ASCII character)
    if data[0] >= 0x20 && data[0] <= 0x7E {
        return false; // Likely ASCII text
    }

    true
}
