use serialport::SerialPort;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::*;
use crate::parser::{parse_log, parse_rcv, parse_telemetry};
use crate::telemetry::DataBuffer;

pub enum UartCommand {
    Send { address: u16, data: String },
    Disconnect,
}

pub fn start_uart_thread(
    port_path: String,
    data_buffer: Arc<Mutex<DataBuffer>>,
) -> Result<mpsc::Sender<UartCommand>, String> {
    let mut port = serialport::new(&port_path, BAUD_RATE)
        .timeout(Duration::from_millis(SERIAL_TIMEOUT_MS))
        .open()
        .map_err(|e| format!("failed to open port '{}': {}", port_path, e))?;

    if !init_lora_receiver(&mut port) {
        return Err("Failed to initialize LoRa".to_string());
    }

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        uart_loop(port, data_buffer, rx);
    });

    Ok(tx)
}

fn uart_loop(
    mut port: Box<dyn SerialPort>,
    data_buffer: Arc<Mutex<DataBuffer>>,
    rx: mpsc::Receiver<UartCommand>,
) {
    let mut buffer = String::new();
    let mut serial_buf = vec![0u8; 256];

    loop {
        // Check for outgoing commands (non-blocking)
        if let Ok(cmd) = rx.try_recv() {
            if matches!(cmd, UartCommand::Disconnect) {
                println!("Disconnecting from serial port");
                drop(port);
                break;
            }
            handle_uart_command(&mut port, cmd);
        }

        // Handle incoming serial data
        handle_serial_read(&mut port, &mut buffer, &mut serial_buf, &data_buffer);
    }
    println!("UART thread exited");
}

fn handle_uart_command(port: &mut Box<dyn SerialPort>, cmd: UartCommand) {
    match cmd {
        UartCommand::Send { address, data } => {
            send_lora_data(port, address, &data);
        }
        UartCommand::Disconnect => {
            // Already handled in uart_loop
        }
    }
}

fn send_lora_data(port: &mut Box<dyn SerialPort>, address: u16, data: &str) {
    let payload_length = data.len();

    // Check maximum payload length
    if payload_length > 240 {
        eprintln!(
            "Payload length {} exceeds maximum of 240 bytes",
            payload_length
        );
        return;
    }

    let cmd = format!("AT+SEND={},{},{}", address, payload_length, data);
    println!("Sending: {}", cmd);

    if let Err(e) = port.write_all(format!("{}\r\n", cmd).as_bytes()) {
        eprintln!("Failed to send data: {}", e);
        return;
    }

    // Wait for +OK response
    if wait_for_response(port, "+OK") {
        println!(
            "✓ Received +OK - Data sent successfully to address {}: '{}'",
            address, data
        );
    } else {
        eprintln!("Failed to send data to address {}", address);
    }
}

fn handle_serial_read(
    port: &mut Box<dyn SerialPort>,
    buffer: &mut String,
    serial_buf: &mut [u8],
    data_buffer: &Arc<Mutex<DataBuffer>>,
) {
    match port.read(serial_buf) {
        Ok(n) => process_bytes(buffer, &serial_buf[..n], data_buffer),
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
        Err(_) => thread::sleep(Duration::from_millis(100)),
    }
}

fn process_bytes(buffer: &mut String, bytes: &[u8], data_buffer: &Arc<Mutex<DataBuffer>>) {
    let Ok(s) = std::str::from_utf8(bytes) else {
        return;
    };
    buffer.push_str(s);

    // If we see +RCV in the buffer, it means a new message is starting
    // Clear any incomplete previous message by keeping only from the last +RCV
    if let Some(last_rcv_pos) = buffer.rfind("+RCV") {
        // If +RCV is not at the start, we have garbage before it - discard it
        if last_rcv_pos > 0 {
            let new_buffer = buffer[last_rcv_pos..].to_string();
            *buffer = new_buffer;
        }
    }

    while let Some(pos) = buffer.find('\n') {
        let line = buffer.drain(..=pos).collect::<String>();
        process_line(line.trim(), data_buffer);
    }
}

fn process_line(line: &str, data_buffer: &Arc<Mutex<DataBuffer>>) {
    let Some(rcv) = parse_rcv(line) else {
        eprintln!("Failed to parse RCV: {line}");
        return;
    };

    let Ok(mut buf) = data_buffer.lock() else {
        return;
    };

    // Parse string-based telemetry and log messages
    if let Some(telem) = parse_telemetry(&rcv.message) {
        buf.push(telem);
    } else if let Some(log_msg) = parse_log(&rcv.message) {
        buf.push_log(log_msg);
    }
}

fn init_lora_receiver(port: &mut Box<dyn SerialPort>) -> bool {
    let commands = vec![
        "AT".to_string(),
        format!("AT+ADDRESS={}", LORA_ADDRESS),
        format!("AT+NETWORKID={}", LORA_NETWORK_ID),
        format!("AT+BAND={}", LORA_BAND),
        format!(
            "AT+PARAMETER={},{},{},{}",
            LORA_SPREADING_FACTOR, LORA_BANDWIDTH, LORA_CODING_RATE, LORA_PREAMBLE
        ),
    ];

    for cmd in commands {
        println!("Sending: {cmd}");

        if let Err(e) = port.write_all(format!("{cmd}\r\n").as_bytes()) {
            eprintln!("Failed to send command '{cmd}': {e}");
            return false;
        }

        // Wait for +OK response
        if !wait_for_response(port, "+OK") {
            eprintln!("Failed to get +OK response for '{cmd}'");
            return false;
        }

        thread::sleep(Duration::from_millis(INTER_COMMAND_DELAY_MS));
    }

    println!("LoRa receiver configuration complete");
    true
}

fn wait_for_response(port: &mut Box<dyn SerialPort>, expected: &str) -> bool {
    let mut buffer = String::new();
    let mut serial_buf = vec![0u8; 256];
    let timeout = Instant::now();
    let max_wait = Duration::from_secs(2);

    loop {
        if timeout.elapsed() > max_wait {
            eprintln!("Timeout waiting for response");
            return false;
        }

        match port.read(&mut serial_buf) {
            Ok(n) if n > 0 => {
                if let Ok(s) = std::str::from_utf8(&serial_buf[..n]) {
                    buffer.push_str(s);
                    println!("Received: {}", s.trim());
                    // Check if we have a complete line (ends with \r\n)
                    if buffer.ends_with("\r\n") {
                        let line = buffer.trim();

                        // Check for error first
                        if let Some(code) = line.strip_prefix("+ERR=") {
                            // Extract code after "+ERR="
                            eprintln!("LoRa module error: {code}");
                            return false;
                        }

                        // Check for expected response
                        if line.contains(expected) {
                            println!("Got expected response: {line}");
                            return true;
                        }

                        // Clear buffer and continue waiting for response
                        buffer.clear();
                    }
                }
            }
            Ok(_) => {
                // No bytes read, small sleep to avoid busy waiting
                thread::sleep(Duration::from_millis(10));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                eprintln!("Error reading response: {e}");
                return false;
            }
        }
    }
}
