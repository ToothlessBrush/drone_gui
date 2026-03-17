use bytemuck;
use serialport::SerialPort;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::config::{BAUD_RATE, SERIAL_TIMEOUT_MS};
use crate::parser::{parse_ack, parse_err, parse_log};
use crate::telemetry::{DataBuffer, TelemetryPacket};

pub enum UartCommand {
    Send { data: Vec<u8> },
    Disconnect,
}

const BT_SYNC: u8 = 0xA5;
const BT_TELEM: u8 = 0x10;

pub fn start_uart_thread(
    port_path: String,
    data_buffer: Arc<Mutex<DataBuffer>>,
) -> Result<mpsc::Sender<UartCommand>, String> {
    let port = serialport::new(&port_path, BAUD_RATE)
        .timeout(Duration::from_millis(SERIAL_TIMEOUT_MS))
        .open()
        .map_err(|e| format!("failed to open port '{}': {}", port_path, e))?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        uart_loop(port, data_buffer, rx);
    });

    println!("Serial port {} opened at {} baud", port_path, BAUD_RATE);
    Ok(tx)
}

fn uart_loop(
    mut port: Box<dyn SerialPort>,
    data_buffer: Arc<Mutex<DataBuffer>>,
    rx: mpsc::Receiver<UartCommand>,
) {
    let mut serial_buf = vec![0u8; 256];
    let mut parser = RxParser::new();

    loop {
        if let Ok(cmd) = rx.try_recv() {
            match cmd {
                UartCommand::Disconnect => {
                    println!("Disconnecting from serial port");
                    drop(port);
                    break;
                }
                UartCommand::Send { data } => {
                    if let Err(e) = port.write_all(&data) {
                        eprintln!("Failed to send binary frame: {}", e);
                    }
                }
            }
        }

        match port.read(&mut serial_buf) {
            Ok(n) if n > 0 => {
                parser.feed(&serial_buf[..n], &data_buffer);
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
    println!("UART thread exited");
}

/// Parses a mixed binary-frame + text-line byte stream.
///
/// Binary frames start with 0xA5 (which can't appear in ASCII text).
/// Text lines are UTF-8 terminated by '\n'.
struct RxParser {
    state: ParseState,
    line_buf: String,
}

enum ParseState {
    Text,
    FrameType,
    FrameLen(u8),
    FramePayload { pkt_type: u8, expected: usize, buf: Vec<u8> },
    FrameCrc { pkt_type: u8, payload: Vec<u8> },
}

impl RxParser {
    fn new() -> Self {
        Self {
            state: ParseState::Text,
            line_buf: String::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8], data_buffer: &Arc<Mutex<DataBuffer>>) {
        for &byte in bytes {
            match &mut self.state {
                ParseState::Text => {
                    if byte == BT_SYNC {
                        self.state = ParseState::FrameType;
                    } else {
                        self.line_buf.push(byte as char);
                        if byte == b'\n' {
                            let line = std::mem::take(&mut self.line_buf);
                            let trimmed = line.trim().to_string();
                            if !trimmed.is_empty() {
                                process_line(&trimmed, data_buffer);
                            }
                        }
                    }
                }

                ParseState::FrameType => {
                    self.state = ParseState::FrameLen(byte);
                }

                ParseState::FrameLen(pkt_type) => {
                    let pkt_type = *pkt_type;
                    let len = byte as usize;
                    if len == 0 {
                        self.state = ParseState::FrameCrc { pkt_type, payload: vec![] };
                    } else if len > 240 {
                        self.state = ParseState::Text; // oversized, discard
                    } else {
                        self.state = ParseState::FramePayload {
                            pkt_type,
                            expected: len,
                            buf: Vec::with_capacity(len),
                        };
                    }
                }

                ParseState::FramePayload { expected, buf, pkt_type } => {
                    buf.push(byte);
                    if buf.len() == *expected {
                        let pkt_type = *pkt_type;
                        let payload = std::mem::take(buf);
                        self.state = ParseState::FrameCrc { pkt_type, payload };
                    }
                }

                ParseState::FrameCrc { pkt_type, payload } => {
                    let pkt_type = *pkt_type;
                    let payload = std::mem::take(payload);
                    self.state = ParseState::Text;

                    let mut crc: u8 = 0;
                    crc = crc8_dvb_s2(crc, pkt_type);
                    crc = crc8_dvb_s2(crc, payload.len() as u8);
                    for &b in &payload {
                        crc = crc8_dvb_s2(crc, b);
                    }
                    if crc == byte {
                        process_frame(pkt_type, &payload, data_buffer);
                    }
                }
            }
        }
    }
}

fn process_frame(pkt_type: u8, payload: &[u8], data_buffer: &Arc<Mutex<DataBuffer>>) {
    if pkt_type == BT_TELEM {
        if let Ok(packet) = bytemuck::try_from_bytes::<TelemetryPacket>(payload) {
            if let Ok(mut buf) = data_buffer.lock() {
                buf.push(packet.into());
            }
        }
    }
}

fn process_line(line: &str, data_buffer: &Arc<Mutex<DataBuffer>>) {
    let Ok(mut buf) = data_buffer.lock() else {
        return;
    };

    if let Some(ack) = parse_ack(line) {
        buf.push_log(format!("ACK: {}", ack));
    } else if let Some(log_msg) = parse_log(line) {
        buf.push_log(log_msg);
    } else if let Some(err) = parse_err(line) {
        buf.push_log(format!("ERR: {}", err));
    }
}

fn crc8_dvb_s2(mut crc: u8, byte: u8) -> u8 {
    crc ^= byte;
    for _ in 0..8 {
        crc = if crc & 0x80 != 0 { (crc << 1) ^ 0xD5 } else { crc << 1 };
    }
    crc
}
