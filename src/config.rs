// LoRa configuration constants
pub const LORA_ADDRESS: u32 = 1;
pub const LORA_NETWORK_ID: u32 = 6;
pub const LORA_BAND: u32 = 915_000_000;
pub const LORA_SPREADING_FACTOR: u32 = 9;
pub const LORA_BANDWIDTH: u32 = 7;
pub const LORA_CODING_RATE: u32 = 1;
pub const LORA_PREAMBLE: u32 = 4;

// Serial port configuration
pub const BAUD_RATE: u32 = 115_200;
pub const SERIAL_TIMEOUT_MS: u64 = 100;
pub const INTER_COMMAND_DELAY_MS: u64 = 100;

// Data buffer limits
pub const MAX_POINTS: usize = 500;
pub const MAX_LOG_MESSAGES: usize = 100;
