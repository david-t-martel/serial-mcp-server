use serde::{Deserialize, Serialize};
use rust_mcp_sdk::macros::JsonSchema;
use std::sync::{Arc, Mutex};

/// A type alias for the shared, thread-safe application state.
pub type AppState = Arc<Mutex<PortState>>;

/// Configuration for the serial port.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct PortConfig {
    pub port_name: String,
    #[serde(default = "default_baud")] pub baud_rate: u32,
    #[serde(default = "default_timeout")] pub timeout_ms: u64,
    #[serde(default = "default_data_bits")] pub data_bits: DataBitsCfg,
    #[serde(default = "default_parity")] pub parity: ParityCfg,
    #[serde(default = "default_stop_bits")] pub stop_bits: StopBitsCfg,
    #[serde(default = "default_flow_control")] pub flow_control: FlowControlCfg,
}

fn default_baud() -> u32 { 9600 }
fn default_timeout() -> u64 { 1000 }
fn default_data_bits() -> DataBitsCfg { DataBitsCfg::Eight }
fn default_parity() -> ParityCfg { ParityCfg::None }
fn default_stop_bits() -> StopBitsCfg { StopBitsCfg::One }
fn default_flow_control() -> FlowControlCfg { FlowControlCfg::None }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataBitsCfg { Five, Six, Seven, Eight }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParityCfg { None, Odd, Even }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StopBitsCfg { One, Two }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlowControlCfg { None, Hardware, Software }

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
