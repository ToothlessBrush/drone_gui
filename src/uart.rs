use serialport::SerialPort;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::*;
use crate::parser::{parse_log, parse_rcv, parse_telemetry};
use crate::telemetry::DataBuffer;

pub fn start_uart_thread(port_path: String, data_buffer: Arc<Mutex<DataBuffer>>) {
    thread::spawn(move || {
        uart_loop(port_path, data_buffer);
    });
}

fn uart_loop(port_path: String, data_buffer: Arc<Mutex<DataBuffer>>) {
    let mut port = match serialport::new(&port_path, BAUD_RATE)
        .timeout(Duration::from_millis(SERIAL_TIMEOUT_MS))
        .open()
    {
        Ok(p) => p,
        Err(_) => return,
    };

    println!("Initializing LoRa receiver module...");
    if !init_lora_receiver(&mut port) {
        eprintln!("Failed to initialize LoRa receiver module!");
        return;
    }
    println!("LoRa receiver initialized successfully");

    let mut buffer = String::new();
    let mut serial_buf = vec![0u8; 256];

    loop {
        handle_serial_read(&mut port, &mut buffer, &mut serial_buf, &data_buffer);
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
