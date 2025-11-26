//! Serial MCP Agent Library
//!
//! This library provides core functionality for the serial port MCP server,
//! including state management, error handling, session tracking, and MCP integration.
//!
//! # Modules
//!
//! - `config`: Configuration management with TOML support
//! - `state`: Port configuration and state management
//! - `error`: Unified error handling
//! - `session`: Session tracking and management
//! - `port`: Port abstraction layer for serial communication
//! - `service`: Business logic layer for port operations
//! - `mcp`: MCP server implementation (when `mcp` feature is enabled)
//! - `rest_api`: REST API handlers (when `rest-api` feature is enabled)
//! - `tui`: Terminal UI application (when `tui` feature is enabled)
//! - `stdio`: Legacy stdio interface

pub mod config;
pub mod error;
pub mod port;
pub mod service;
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

// TUI module
#[cfg(feature = "tui")]
pub mod tui;

// Re-export commonly used types for convenience
pub use error::AppError;
pub use port::{
    DataBits, FlowControl, MockSerialPort, Parity, PortConfiguration, PortError, SerialPortAdapter,
    StopBits, SyncSerialPort,
};
pub use service::{
    AutoCloseInfo, CloseResult, MetricsResult, OpenConfig, OpenResult, PortMetrics, PortService,
    ReadResult, ReconfigureConfig, ServiceError, ServiceResult, StatusResult, WriteResult,
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

// Re-export config types
pub use config::{Config, ConfigLoader, ConfigError, ConfigResult};
