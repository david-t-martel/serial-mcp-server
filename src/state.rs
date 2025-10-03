use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// A type alias for the shared, thread-safe application state.
pub type AppState = Arc<Mutex<PortState>>;

/// Configuration for the serial port.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    1000
}

/// Represents the current state of the serial port.
#[derive(Serialize, Debug)]
#[serde(tag = "status", content = "details")]
pub enum PortState {
    Closed,
    Open {
        // The actual port object is not serializable and is kept private.
        #[serde(skip_serializing)]
        port: Box<dyn serialport::SerialPort>,
        // The configuration is included in the status response.
        config: PortConfig,
    },
}

impl Default for PortState {
    fn default() -> Self {
        PortState::Closed
    }
}
