/// Parse log message from a raw serial line
/// Format: "LOG:message text here"
pub fn parse_log(line: &str) -> Option<String> {
    line.strip_prefix("LOG:").map(str::to_string)
}

/// Check if the line is an ACK from the flight controller
/// Returns the ACK type string (e.g. "PID", "BIAS", "CONFIG", "SAVE", "CALIBRATE")
pub fn parse_ack(line: &str) -> Option<&str> {
    line.strip_prefix("ACK:")
}

/// Check if the line is an error from the flight controller
/// Returns the error string
pub fn parse_err(line: &str) -> Option<&str> {
    line.strip_prefix("ERR:")
}
