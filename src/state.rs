use rust_mcp_sdk::macros::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::port::SerialPortAdapter;

/// A type alias for the shared, thread-safe application state.
pub type AppState = Arc<Mutex<PortState>>;

/// Type alias for the port adapter used in PortState.
/// This abstraction enables:
/// - Dependency injection for testing (MockSerialPort)
/// - Future async serial implementations (TokioSerialPort)
/// - Consistent interface across different backends
pub type PortAdapter = Box<dyn SerialPortAdapter>;

/// Configuration for the serial port.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PortConfig {
    pub port_name: String,
    #[serde(default = "default_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_data_bits")]
    pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")]
    pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")]
    pub flow_control: FlowControlCfg,
    #[serde(default = "default_terminator")]
    pub terminator: Option<String>,
    #[serde(default)]
    pub idle_disconnect_ms: Option<u64>,
}

// Default configuration constants
pub const DEFAULT_BAUD_RATE: u32 = 9600;
pub const DEFAULT_TIMEOUT_MS: u64 = 1000;
pub const DEFAULT_RECONFIG_BAUD_RATE: u32 = 9600;

/// Default baud rate for serial port configuration (9600 bps).
pub fn default_baud() -> u32 {
    DEFAULT_BAUD_RATE
}

/// Default timeout for serial port operations (1000 ms).
pub fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// Default data bits configuration (8 bits).
pub fn default_data_bits() -> DataBitsCfg {
    DataBitsCfg::Eight
}

/// Default parity configuration (none).
pub fn default_parity() -> ParityCfg {
    ParityCfg::None
}

/// Default stop bits configuration (1 bit).
pub fn default_stop_bits() -> StopBitsCfg {
    StopBitsCfg::One
}

/// Default flow control configuration (none).
pub fn default_flow_control() -> FlowControlCfg {
    FlowControlCfg::None
}

/// Default terminator for serial communication (newline).
pub fn default_terminator() -> Option<String> {
    Some("\n".into())
}

/// Default baud rate for port reconfiguration (9600 bps).
pub fn default_reconfig_baud() -> u32 {
    DEFAULT_RECONFIG_BAUD_RATE
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataBitsCfg {
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParityCfg {
    None,
    Odd,
    Even,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StopBitsCfg {
    One,
    Two,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlowControlCfg {
    None,
    Hardware,
    Software,
}

/// Represents the current state of the serial port.
#[derive(Serialize, Debug)]
#[serde(tag = "status", content = "details")]
#[derive(Default)]
pub enum PortState {
    #[default]
    Closed,
    Open {
        /// The port adapter implementing SerialPortAdapter trait.
        /// Uses trait object for dependency injection and testing.
        #[serde(skip_serializing)]
        port: PortAdapter,
        /// The configuration is included in the status response.
        config: PortConfig,
        #[serde(skip_serializing)]
        last_activity: Instant,
        #[serde(skip_serializing)]
        timeout_streak: u32,
        #[serde(skip_serializing)]
        bytes_read_total: u64,
        #[serde(skip_serializing)]
        bytes_written_total: u64,
        #[serde(skip_serializing)]
        idle_close_count: u64,
        #[serde(skip_serializing)]
        open_started: Instant,
    },
}
