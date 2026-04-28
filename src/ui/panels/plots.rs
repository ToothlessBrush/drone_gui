use crate::app::AppState;
use crate::telemetry::PidAxis;
use bevy_egui::egui;
use egui::Color32;
use egui_plot::{Legend, Line, Plot, PlotPoint, Text};

/// Find local peaks (maxima and minima) in plot data.
/// Returns (x, y) pairs for points that are local extrema,
/// with a minimum prominence filter to avoid labeling noise.
fn find_peaks(data: &[[f64; 2]], min_prominence: f64) -> Vec<[f64; 2]> {
    if data.len() < 3 {
        return vec![];
    }
    let mut peaks = Vec::new();
    for i in 1..data.len() - 1 {
        let prev = data[i - 1][1];
        let curr = data[i][1];
        let next = data[i + 1][1];
        // Local maximum
        if curr > prev && curr > next && (curr - prev).abs() >= min_prominence && (curr - next).abs() >= min_prominence {
            peaks.push(data[i]);
        }
        // Local minimum
        if curr < prev && curr < next && (prev - curr).abs() >= min_prominence && (next - curr).abs() >= min_prominence {
            peaks.push(data[i]);
        }
    }
    peaks
}

/// Add peak labels to a plot for a given data series.
fn plot_peaks(plot_ui: &mut egui_plot::PlotUi, data: &[[f64; 2]], color: Color32, min_prominence: f64) {
    for peak in find_peaks(data, min_prominence) {
        plot_ui.text(
            Text::new(
                PlotPoint::new(peak[0], peak[1]),
                format!("{:.1}", peak[1]),
            )
            .color(color),
        );
    }
}

/// True when the buffer has at least two distinct timestamps — egui_plot 0.29
/// panics with "Bad final plot bounds" if x_min == x_max.
fn has_plottable_range(data: &std::collections::VecDeque<crate::telemetry::TelemetryData>) -> bool {
    if data.len() < 2 {
        return false;
    }
    let first = data.front().unwrap().timestamp;
    data.iter().any(|d| d.timestamp != first)
}

/// Renders the attitude plot (Roll, Pitch, Yaw)
pub fn render_attitude_plot(ui: &mut egui::Ui, state: &AppState) {
    let max_width = ui.ctx().screen_rect().width() - 32.0;
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_max_width(max_width - 16.0);
        ui.label("Attitude (Roll, Pitch, Yaw)");
        let buffer = state.data_buffer.lock().unwrap();
        if !has_plottable_range(&buffer.data) {
            ui.label("Waiting for telemetry…");
            return;
        }
        let plot_height = (ui.ctx().screen_rect().height() * 0.25).min(300.0);
        let plot_width = ui.available_width();

        let roll_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.roll as f64]).collect();
        let pitch_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.pitch as f64]).collect();
        let yaw_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.yaw as f64]).collect();
        let roll_sp: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.input_roll as f64]).collect();
        let pitch_sp: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.input_pitch as f64]).collect();
        let yaw_sp: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.input_yaw as f64]).collect();

        Plot::new("attitude_plot")
            .legend(Legend::default())
            .height(plot_height)
            .width(plot_width)
            .show(ui, |plot_ui| {
                let r_color = Color32::from_rgb(255, 0, 0);
                let p_color = Color32::from_rgb(0, 255, 0);
                let y_color = Color32::from_rgb(0, 0, 255);
                plot_ui.line(Line::new(roll_data.clone()).name("Roll").color(r_color));
                plot_ui.line(Line::new(pitch_data.clone()).name("Pitch").color(p_color));
                plot_ui.line(Line::new(yaw_data.clone()).name("Yaw").color(y_color));
                plot_ui.line(Line::new(roll_sp).name("Roll SP").color(r_color.gamma_multiply(0.5)).style(egui_plot::LineStyle::dashed_dense()));
                plot_ui.line(Line::new(pitch_sp).name("Pitch SP").color(p_color.gamma_multiply(0.5)).style(egui_plot::LineStyle::dashed_dense()));
                plot_ui.line(Line::new(yaw_sp).name("Yaw SP").color(y_color.gamma_multiply(0.5)).style(egui_plot::LineStyle::dashed_dense()));
                plot_peaks(plot_ui, &roll_data, r_color, 1.0);
                plot_peaks(plot_ui, &pitch_data, p_color, 1.0);
                plot_peaks(plot_ui, &yaw_data, y_color, 1.0);
            });
    });
}

/// Renders the gyro rate plot (X, Y, Z angular velocity)
pub fn render_gyro_plot(ui: &mut egui::Ui, state: &AppState) {
    let max_width = ui.ctx().screen_rect().width() - 32.0;
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_max_width(max_width - 16.0);
        ui.label("Gyro Rates (deg/s)");
        let buffer = state.data_buffer.lock().unwrap();
        if !has_plottable_range(&buffer.data) {
            ui.label("Waiting for telemetry…");
            return;
        }
        let plot_height = (ui.ctx().screen_rect().height() * 0.20).min(200.0);
        let plot_width = ui.available_width();

        let rad2deg = 180.0 / std::f64::consts::PI;
        let gx_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.gyro_x as f64 * rad2deg]).collect();
        let gy_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.gyro_y as f64 * rad2deg]).collect();
        let gz_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.gyro_z as f64 * rad2deg]).collect();

        Plot::new("gyro_plot")
            .legend(Legend::default())
            .height(plot_height)
            .width(plot_width)
            .show(ui, |plot_ui| {
                let x_color = Color32::from_rgb(255, 0, 0);
                let y_color = Color32::from_rgb(0, 255, 0);
                let z_color = Color32::from_rgb(0, 0, 255);
                plot_ui.line(Line::new(gx_data.clone()).name("Gyro X").color(x_color));
                plot_ui.line(Line::new(gy_data.clone()).name("Gyro Y").color(y_color));
                plot_ui.line(Line::new(gz_data.clone()).name("Gyro Z").color(z_color));
                plot_peaks(plot_ui, &gx_data, x_color, 5.0);
                plot_peaks(plot_ui, &gy_data, y_color, 5.0);
                plot_peaks(plot_ui, &gz_data, z_color, 5.0);
            });
    });
}

/// Renders the velocity + height plot
pub fn render_velocity_plot(ui: &mut egui::Ui, state: &AppState) {
    let max_width = ui.ctx().screen_rect().width() - 32.0;
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_max_width(max_width - 16.0);
        ui.label("Velocity (m/s) & Height (m)");
        let buffer = state.data_buffer.lock().unwrap();
        if !has_plottable_range(&buffer.data) {
            ui.label("Waiting for telemetry…");
            return;
        }
        let plot_height = (ui.ctx().screen_rect().height() * 0.20).min(200.0);
        let plot_width = ui.available_width();

        let vx_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.vel_x as f64]).collect();
        let vy_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.vel_y as f64]).collect();
        let h_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.height as f64]).collect();

        Plot::new("velocity_plot")
            .legend(Legend::default())
            .height(plot_height)
            .width(plot_width)
            .show(ui, |plot_ui| {
                let vx_color = Color32::from_rgb(255, 100, 100);
                let vy_color = Color32::from_rgb(100, 255, 100);
                let h_color = Color32::from_rgb(255, 255, 100);
                plot_ui.line(Line::new(vx_data.clone()).name("Vel X").color(vx_color));
                plot_ui.line(Line::new(vy_data.clone()).name("Vel Y").color(vy_color));
                plot_ui.line(Line::new(h_data.clone()).name("Height").color(h_color));
                plot_peaks(plot_ui, &vx_data, vx_color, 0.1);
                plot_peaks(plot_ui, &vy_data, vy_color, 0.1);
                plot_peaks(plot_ui, &h_data, h_color, 0.05);
            });
    });
}

/// Renders the motor throttle output plot (M1, M2, M3, M4)
pub fn render_motor_plot(ui: &mut egui::Ui, state: &AppState) {
    let max_width = ui.ctx().screen_rect().width() - 32.0;
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_max_width(max_width - 16.0);
        ui.label("Motor Outputs (0-1)");
        let buffer = state.data_buffer.lock().unwrap();
        if !has_plottable_range(&buffer.data) {
            ui.label("Waiting for telemetry…");
            return;
        }
        let plot_height = (ui.ctx().screen_rect().height() * 0.20).min(200.0);
        let plot_width = ui.available_width();

        let m1_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.motor1 as f64]).collect();
        let m2_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.motor2 as f64]).collect();
        let m3_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.motor3 as f64]).collect();
        let m4_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.motor4 as f64]).collect();
        let thr_input: Vec<[f64; 2]> = buffer.data.iter().map(|d| [d.timestamp as f64 / 1000.0, d.input_throttle as f64]).collect();

        Plot::new("motor_plot")
            .legend(Legend::default())
            .height(plot_height)
            .width(plot_width)
            .show(ui, |plot_ui| {
                let m1_color = Color32::from_rgb(255, 80, 80);
                let m2_color = Color32::from_rgb(80, 255, 80);
                let m3_color = Color32::from_rgb(80, 80, 255);
                let m4_color = Color32::from_rgb(255, 255, 80);
                let thr_color = Color32::from_rgb(200, 200, 200);
                plot_ui.line(Line::new(m1_data.clone()).name("M1").color(m1_color));
                plot_ui.line(Line::new(m2_data.clone()).name("M2").color(m2_color));
                plot_ui.line(Line::new(m3_data.clone()).name("M3").color(m3_color));
                plot_ui.line(Line::new(m4_data.clone()).name("M4").color(m4_color));
                plot_ui.line(Line::new(thr_input).name("Throttle Input").color(thr_color).style(egui_plot::LineStyle::dashed_dense()));
                plot_peaks(plot_ui, &m1_data, m1_color, 0.05);
                plot_peaks(plot_ui, &m2_data, m2_color, 0.05);
                plot_peaks(plot_ui, &m3_data, m3_color, 0.05);
                plot_peaks(plot_ui, &m4_data, m4_color, 0.05);
            });
    });
}

/// Renders the PID plot for the selected axis
pub fn render_pid_plot(ui: &mut egui::Ui, state: &mut AppState) {
    let max_width = ui.ctx().screen_rect().width() - 32.0;
    ui.set_max_width(max_width);
    ui.group(|ui| {
        ui.set_max_width(max_width - 16.0);
        ui.horizontal(|ui| {
            ui.label("PID Axis:");
            ui.selectable_value(&mut state.selected_pid_axis, PidAxis::Roll, "Roll");
            ui.selectable_value(&mut state.selected_pid_axis, PidAxis::Pitch, "Pitch");
            ui.selectable_value(&mut state.selected_pid_axis, PidAxis::Yaw, "Yaw");
        });

        let selected_axis = state.selected_pid_axis;
        let axis_name = match selected_axis {
            PidAxis::Roll => "Roll",
            PidAxis::Pitch => "Pitch",
            PidAxis::Yaw => "Yaw",
        };

        ui.label(format!("{axis_name} PID Values (P, I, D)"));

        let buffer = state.data_buffer.lock().unwrap();
        if !has_plottable_range(&buffer.data) {
            ui.label("Waiting for telemetry…");
            return;
        }
        let plot_height = (ui.ctx().screen_rect().height() * 0.20).min(200.0);
        let plot_width = ui.available_width();

        let p_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| {
            let val = match selected_axis { PidAxis::Roll => d.roll_p, PidAxis::Pitch => d.pitch_p, PidAxis::Yaw => d.yaw_p };
            [d.timestamp as f64 / 1000.0, val as f64]
        }).collect();
        let i_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| {
            let val = match selected_axis { PidAxis::Roll => d.roll_i, PidAxis::Pitch => d.pitch_i, PidAxis::Yaw => d.yaw_i };
            [d.timestamp as f64 / 1000.0, val as f64]
        }).collect();
        let d_data: Vec<[f64; 2]> = buffer.data.iter().map(|d| {
            let val = match selected_axis { PidAxis::Roll => d.roll_d, PidAxis::Pitch => d.pitch_d, PidAxis::Yaw => d.yaw_d };
            [d.timestamp as f64 / 1000.0, val as f64]
        }).collect();

        Plot::new("pid_plot")
            .legend(Legend::default())
            .height(plot_height)
            .width(plot_width)
            .show(ui, |plot_ui| {
                let p_color = Color32::from_rgb(255, 100, 100);
                let i_color = Color32::from_rgb(100, 255, 100);
                let d_color = Color32::from_rgb(100, 100, 255);
                plot_ui.line(Line::new(p_data.clone()).name("P").color(p_color));
                plot_ui.line(Line::new(i_data.clone()).name("I").color(i_color));
                plot_ui.line(Line::new(d_data.clone()).name("D").color(d_color));
                plot_peaks(plot_ui, &p_data, p_color, 0.05);
                plot_peaks(plot_ui, &i_data, i_color, 0.05);
                plot_peaks(plot_ui, &d_data, d_color, 0.05);
            });
    });
}
