//! Configuration error types for the config module.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Config file not found at expected path
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    /// Failed to read config file
    #[error("Failed to read configuration file '{path}': {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse TOML
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize config
    #[error("Failed to serialize configuration: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// Failed to write config file
    #[error("Failed to write configuration file '{path}': {source}")]
    WriteError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid configuration value
    #[error("Invalid configuration value for '{key}': {message}")]
    ValidationError { key: String, message: String },

    /// Environment variable parse error
    #[error("Failed to parse environment variable '{var}': {message}")]
    EnvParseError { var: String, message: String },

    /// Missing required configuration
    #[error("Missing required configuration: {0}")]
    MissingRequired(String),

    /// Config watcher error
    #[cfg(feature = "hot-reload")]
    #[error("Config watcher error: {0}")]
    WatcherError(String),
}

impl ConfigError {
    /// Create a validation error
    pub fn validation<K: Into<String>, M: Into<String>>(key: K, message: M) -> Self {
        Self::ValidationError {
            key: key.into(),
            message: message.into(),
        }
    }

    /// Create an env parse error
    pub fn env_parse<V: Into<String>, M: Into<String>>(var: V, message: M) -> Self {
        Self::EnvParseError {
            var: var.into(),
            message: message.into(),
        }
    }
}

/// Result type for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;
