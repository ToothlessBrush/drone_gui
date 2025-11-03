use chrono::{DateTime, Local};
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const LORA_ADDRESS: u32 = 1;
const LORA_NETWORK_ID: u32 = 6;
const LORA_BAND: u32 = 915_000_000;
const LORA_SPREADING_FACTOR: u32 = 9;
const LORA_BANDWIDTH: u32 = 7;
const LORA_CODING_RATE: u32 = 1;
const LORA_PREAMBLE: u32 = 4;

const BAUD_RATE: u32 = 115_200;
const SERIAL_TIMEOUT_MS: u64 = 100;
const COMMAND_DELAY_MS: u64 = 200;
const INTER_COMMAND_DELAY_MS: u64 = 100;

const MAX_POINTS: usize = 500;
const MAX_LOG_MESSAGES: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq)]
enum PidAxis {
    Roll,
    Pitch,
    Yaw,
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Drone Telemetry",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    )
    .expect("failed to run eframe");
}

#[derive(Clone, Debug)]
struct TelemetryData {
    timestamp: f64,
    clock_time: DateTime<Local>,
    // Attitude
    roll: f32,
    pitch: f32,
    yaw: f32,
    // Roll PID
    roll_p: f32,
    roll_i: f32,
    roll_d: f32,
    // Pitch PID
    pitch_p: f32,
    pitch_i: f32,
    pitch_d: f32,
    // Yaw PID
    yaw_p: f32,
    yaw_i: f32,
    yaw_d: f32,
    // Other telemetry
    altitude: f32,
    battery_voltage: f32,
}

#[derive(Clone, Debug)]
struct LogMessage {
    timestamp: f64,
    clock_time: DateTime<Local>,
    message: String,
}

#[derive(Debug, Clone)]
#[allow(unused)]
struct ReceivedMessage {
    pub from: u32,
    pub length: u32,
    pub message: String,
    pub rssi: i32,
    pub snr: i32,
    pub time: DateTime<Local>,
}

struct DataBuffer {
    data: VecDeque<TelemetryData>,
    logs: VecDeque<LogMessage>,
    start_time: Instant,
}

impl DataBuffer {
    fn new() -> Self {
        Self {
            data: VecDeque::with_capacity(MAX_POINTS),
            logs: VecDeque::with_capacity(MAX_LOG_MESSAGES),
            start_time: Instant::now(),
        }
    }

    fn push(&mut self, mut telem: TelemetryData) {
        telem.timestamp = self.start_time.elapsed().as_secs_f64();
        telem.clock_time = Local::now();

        if self.data.len() >= MAX_POINTS {
            self.data.pop_front();
        }
        self.data.push_back(telem);
    }

    fn push_log(&mut self, message: String) {
        let log_msg = LogMessage {
            timestamp: self.start_time.elapsed().as_secs_f64(),
            clock_time: Local::now(),
            message,
        };

        if self.logs.len() >= MAX_LOG_MESSAGES {
            self.logs.pop_front();
        }
        self.logs.push_back(log_msg);
    }

    fn get_roll_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.roll as f64])
            .collect()
    }

    fn get_pitch_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.pitch as f64])
            .collect()
    }

    fn get_yaw_data<'a>(&'a self) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| [d.timestamp, d.yaw as f64])
            .collect()
    }

    fn get_pid_p_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_p,
                    PidAxis::Pitch => d.pitch_p,
                    PidAxis::Yaw => d.yaw_p,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }

    fn get_pid_i_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_i,
                    PidAxis::Pitch => d.pitch_i,
                    PidAxis::Yaw => d.yaw_i,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }

    fn get_pid_d_data<'a>(&'a self, axis: PidAxis) -> PlotPoints<'a> {
        self.data
            .iter()
            .map(|d| {
                let val = match axis {
                    PidAxis::Roll => d.roll_d,
                    PidAxis::Pitch => d.pitch_d,
                    PidAxis::Yaw => d.yaw_d,
                };
                [d.timestamp, val as f64]
            })
            .collect()
    }
}

struct MyEguiApp {
    data_buffer: Arc<Mutex<DataBuffer>>,
    serial_connected: bool,
    port_path: String,
    selected_pid_axis: PidAxis,
    auto_scroll_logs: bool,
}

impl Default for MyEguiApp {
    fn default() -> Self {
        Self {
            data_buffer: Arc::new(Mutex::new(DataBuffer::new())),
            serial_connected: false,
            port_path: "/dev/ttyAMA1".to_string(),
            selected_pid_axis: PidAxis::Roll,
            auto_scroll_logs: true,
        }
    }
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn start_uart_thread(&mut self) {
        if self.serial_connected {
            return;
        }
        let port_path = self.port_path.clone();
        let data_buffer = Arc::clone(&self.data_buffer);
        thread::spawn(move || {
            Self::uart_loop(port_path, data_buffer);
        });
        self.serial_connected = true;
    }

    fn uart_loop(port_path: String, data_buffer: Arc<Mutex<DataBuffer>>) {
        let mut port = match serialport::new(&port_path, 115_200)
            .timeout(Duration::from_millis(100))
            .open()
        {
            Ok(p) => p,
            Err(_) => return,
        };

        println!("Initializing LoRa receiver module...");
        if !Self::init_lora_receiver(&mut port) {
            eprintln!("Failed to initialize LoRa receiver module!");
            return;
        }
        println!("LoRa receiver initialized successfully");

        let mut buffer = String::new();
        let mut serial_buf = vec![0u8; 256];

        loop {
            Self::handle_serial_read(&mut port, &mut buffer, &mut serial_buf, &data_buffer);
        }
    }

    fn handle_serial_read(
        port: &mut Box<dyn SerialPort>,
        buffer: &mut String,
        serial_buf: &mut [u8],
        data_buffer: &Arc<Mutex<DataBuffer>>,
    ) {
        match port.read(serial_buf) {
            Ok(n) => Self::process_bytes(buffer, &serial_buf[..n], data_buffer),
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
            Self::process_line(line.trim(), data_buffer);
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
            if !Self::wait_for_response(port, "+OK") {
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
}

fn parse_rcv(line: &str) -> Option<ReceivedMessage> {
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

// Parse telemetry from serial data
// Format: "TELEM:roll,pitch,yaw,roll_p,roll_i,roll_d,pitch_p,pitch_i,pitch_d,yaw_p,yaw_i,yaw_d,alt,voltage"
fn parse_telemetry(line: &str) -> Option<TelemetryData> {
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

// Parse log message from serial data
// Format: "LOG:message text here"
fn parse_log(line: &str) -> Option<String> {
    line.strip_prefix("LOG:").map(str::to_string)
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Serial Port:");
                ui.text_edit_singleline(&mut self.port_path);

                if ui
                    .button(if self.serial_connected {
                        "Connected"
                    } else {
                        "Connect"
                    })
                    .clicked()
                    && !self.serial_connected
                {
                    self.start_uart_thread();
                }

                ui.separator();

                if ui.button("Clear Data").clicked()
                    && let Ok(mut buffer) = self.data_buffer.lock()
                {
                    buffer.data.clear();
                }

                if ui.button("Clear Logs").clicked()
                    && let Ok(mut buffer) = self.data_buffer.lock()
                {
                    buffer.logs.clear();
                }

                ui.separator();

                ui.checkbox(&mut self.auto_scroll_logs, "Auto-scroll logs");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Drone Telemetry Monitor");

            let buffer = self.data_buffer.lock().unwrap();

            // Attitude Plot
            ui.group(|ui| {
                ui.label("Attitude (Roll, Pitch, Yaw)");
                Plot::new("attitude_plot")
                    .legend(Legend::default())
                    .height(200.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("Roll", buffer.get_roll_data()).color(egui::Color32::RED),
                        );
                        plot_ui.line(
                            Line::new("Pitch", buffer.get_pitch_data()).color(egui::Color32::GREEN),
                        );
                        plot_ui.line(
                            Line::new("Yaw", buffer.get_yaw_data()).color(egui::Color32::BLUE),
                        );
                    });
            });

            ui.add_space(10.0);

            // PID Selection and Plot
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("PID Axis:");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Roll, "Roll");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Pitch, "Pitch");
                    ui.selectable_value(&mut self.selected_pid_axis, PidAxis::Yaw, "Yaw");
                });

                let axis_name = match self.selected_pid_axis {
                    PidAxis::Roll => "Roll",
                    PidAxis::Pitch => "Pitch",
                    PidAxis::Yaw => "Yaw",
                };

                ui.label(format!("{axis_name} PID Values (P, I, D)"));

                Plot::new("pid_plot")
                    .legend(Legend::default())
                    .height(200.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new("P", buffer.get_pid_p_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        );
                        plot_ui.line(
                            Line::new("I", buffer.get_pid_i_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(100, 255, 100)),
                        );
                        plot_ui.line(
                            Line::new("D", buffer.get_pid_d_data(self.selected_pid_axis))
                                .color(egui::Color32::from_rgb(100, 100, 255)),
                        );
                    });
            });

            ui.add_space(10.0);

            // Display current values
            if let Some(latest) = buffer.data.back() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Current Values");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "Last Update: {}",
                                latest.clock_time.format("%H:%M:%S%.3f")
                            ));
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.label(format!("Roll: {:.2}°", latest.roll));
                        ui.label(format!("Pitch: {:.2}°", latest.pitch));
                        ui.label(format!("Yaw: {:.2}°", latest.yaw));
                    });

                    ui.separator();

                    ui.label("Roll PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.roll_p));
                        ui.label(format!("I: {:.3}", latest.roll_i));
                        ui.label(format!("D: {:.3}", latest.roll_d));
                    });

                    ui.label("Pitch PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.pitch_p));
                        ui.label(format!("I: {:.3}", latest.pitch_i));
                        ui.label(format!("D: {:.3}", latest.pitch_d));
                    });

                    ui.label("Yaw PID:");
                    ui.horizontal(|ui| {
                        ui.label(format!("P: {:.3}", latest.yaw_p));
                        ui.label(format!("I: {:.3}", latest.yaw_i));
                        ui.label(format!("D: {:.3}", latest.yaw_d));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(format!("Altitude: {:.2}m", latest.altitude));
                        ui.label(format!("Battery: {:.2}V", latest.battery_voltage));
                    });
                });
            }

            ui.add_space(10.0);

            // Log Section
            ui.group(|ui| {
                ui.label(format!("System Logs ({} messages)", buffer.logs.len()));

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .auto_shrink([false; 2])
                    .stick_to_bottom(self.auto_scroll_logs)
                    .show(ui, |ui| {
                        for log in buffer.logs.iter() {
                            ui.horizontal(|ui| {
                                ui.label(format!("[{}]", log.clock_time.format("%H:%M:%S%.3f")));
                                ui.label(&log.message);
                            });
                        }
                    });
            });
        });
    }
}
