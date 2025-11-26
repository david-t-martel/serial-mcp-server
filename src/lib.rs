//! Serial MCP Agent Library
//!
//! This library provides core functionality for the serial port MCP server,
//! including state management, error handling, session tracking, and MCP integration.
//!
//! # Modules
//!
//! - `state`: Port configuration and state management
//! - `error`: Unified error handling
//! - `session`: Session tracking and management
//! - `port`: Port abstraction layer for serial communication
//! - `mcp`: MCP server implementation (when `mcp` feature is enabled)
//! - `rest_api`: REST API handlers (when `rest-api` feature is enabled)
//! - `stdio`: Legacy stdio interface

pub mod error;
pub mod port;
pub mod session;
pub mod state;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "rest-api")]
pub mod rest_api;

pub mod stdio;

// Phase 4: Auto-negotiation module
#[cfg(feature = "auto-negotiation")]
pub mod negotiation;

// Phase 5: WebSocket streaming
#[cfg(feature = "websocket")]
pub mod websocket;

// Re-export commonly used types for convenience
pub use error::AppError;
pub use port::{
    DataBits, FlowControl, MockSerialPort, Parity, PortConfiguration, PortError, SerialPortAdapter,
    StopBits, SyncSerialPort,
};
pub use state::{
    AppState, DataBitsCfg, FlowControlCfg, ParityCfg, PortConfig, PortState, StopBitsCfg,
};

#[cfg(feature = "rest-api")]
pub use error::AppResult;

// Re-export auto-negotiation types when enabled
#[cfg(feature = "auto-negotiation")]
pub use negotiation::{
    AutoNegotiator, NegotiatedParams, NegotiationError, NegotiationHints, NegotiationStrategy,
};
