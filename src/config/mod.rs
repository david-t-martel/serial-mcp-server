//! Configuration module for rust-comm.
//!
//! This module provides TOML-based configuration with environment variable overrides.
//!
//! # Configuration Resolution
//!
//! Configuration is loaded from the following locations (in order of priority):
//!
//! 1. `RUST_COMM_CONFIG` environment variable (explicit path)
//! 2. `./config.toml` (current directory)
//! 3. `~/.config/rust-comm/config.toml` (XDG on Linux/macOS)
//! 4. `%APPDATA%\rust-comm\config.toml` (Windows)
//! 5. Built-in defaults (no file required)
//!
//! # Environment Overrides
//!
//! Any configuration value can be overridden via environment variables.
//! The pattern is: `RUST_COMM_<SECTION>_<KEY>`
//!
//! Examples:
//! - `RUST_COMM_SERVER_PORT=8080`
//! - `RUST_COMM_SERIAL_DEFAULT_BAUD=9600`
//! - `RUST_COMM_TESTING_PORT=COM15`
//!
//! Legacy environment variables are also supported:
//! - `TEST_PORT`, `TEST_BAUD`, `TEST_TIMEOUT`, `LOOPBACK_ENABLED`
//! - `SESSION_DB_URL`
//!
//! # Example
//!
//! ```rust,ignore
//! use serial_mcp_agent::config::ConfigLoader;
//!
//! // Load configuration with automatic resolution
//! let loader = ConfigLoader::load()?;
//! let config = loader.config();
//!
//! println!("Server port: {}", config.server.port);
//! println!("Default baud: {}", config.serial.default_baud);
//!
//! // Or load with defaults only
//! let loader = ConfigLoader::with_defaults();
//! ```

mod error;
mod loader;
mod schema;

pub use error::{ConfigError, ConfigResult};
pub use loader::{
    get_default_config_dir, get_default_config_path, resolve_config_path, ConfigLoader,
};
pub use schema::{
    Config, KeybindingsConfig, LogFormat, LoggingConfig, McpConfig, SerialConfig, ServerConfig,
    ServerMode, TestDiscoveryConfig, TestingConfig, TuiConfig,
};

// Future: ConfigWatcher for hot-reload feature
// #[cfg(feature = "hot-reload")]
// mod watcher;
//
// #[cfg(feature = "hot-reload")]
// pub use watcher::ConfigWatcher;
//
// Note: Configuration persistence (save/load) is fully implemented in loader.rs
