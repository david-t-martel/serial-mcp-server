//! Configuration schema definitions.
//!
//! This module defines the structure of the configuration file using serde.
//! All configuration sections are defined here with appropriate defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Root configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Serial port configuration
    pub serial: SerialConfig,
    /// Hardware testing configuration
    pub testing: TestingConfig,
    /// TUI configuration
    pub tui: TuiConfig,
    /// MCP server configuration
    pub mcp: McpConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            serial: SerialConfig::default(),
            testing: TestingConfig::default(),
            tui: TuiConfig::default(),
            mcp: McpConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Server configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port number for HTTP server
    pub port: u16,
    /// Server mode: "mcp", "rest", or "stdio"
    pub mode: ServerMode,
    /// Log level: "trace", "debug", "info", "warn", "error"
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            mode: ServerMode::Mcp,
            log_level: "info".to_string(),
        }
    }
}

/// Server operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerMode {
    /// MCP stdio mode (default)
    Mcp,
    /// REST HTTP mode
    Rest,
    /// Legacy stdio JSON mode
    Stdio,
}

impl Default for ServerMode {
    fn default() -> Self {
        Self::Mcp
    }
}

/// Serial port configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SerialConfig {
    /// Default baud rate for new connections
    pub default_baud: u32,
    /// Default timeout in milliseconds
    pub default_timeout_ms: u64,
    /// Enable auto-discovery
    pub auto_discover: bool,
    /// Discovery interval in milliseconds
    pub discovery_interval_ms: u64,
    /// Port aliases for convenience
    #[serde(default)]
    pub port_aliases: HashMap<String, String>,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            default_baud: 115200,
            default_timeout_ms: 1000,
            auto_discover: true,
            discovery_interval_ms: 5000,
            port_aliases: HashMap::new(),
        }
    }
}

impl SerialConfig {
    /// Get the default timeout as Duration
    pub fn default_timeout(&self) -> Duration {
        Duration::from_millis(self.default_timeout_ms)
    }

    /// Resolve a port name through aliases
    pub fn resolve_port(&self, name: &str) -> String {
        self.port_aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
}

/// Hardware testing configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TestingConfig {
    /// Test port name (or use auto-discovery)
    pub port: Option<String>,
    /// Test baud rate
    pub baud: u32,
    /// Whether loopback is enabled on test port
    pub loopback_enabled: bool,
    /// Test timeout in milliseconds
    pub timeout_ms: u64,
    /// Skip performance tests
    pub skip_performance: bool,
    /// Discovery settings for test port
    pub discovery: TestDiscoveryConfig,
}

impl Default for TestingConfig {
    fn default() -> Self {
        Self {
            port: None,
            baud: 115200,
            loopback_enabled: false,
            timeout_ms: 2000,
            skip_performance: false,
            discovery: TestDiscoveryConfig::default(),
        }
    }
}

impl TestingConfig {
    /// Get the test timeout as Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Test port discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TestDiscoveryConfig {
    /// Enable auto-discovery if port not set
    pub enabled: bool,
    /// Preferred manufacturers for test port selection
    #[serde(default)]
    pub prefer_manufacturers: Vec<String>,
    /// Ports to exclude from discovery
    #[serde(default)]
    pub exclude_ports: Vec<String>,
}

impl Default for TestDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_manufacturers: vec![
                "FTDI".to_string(),
                "ESP32".to_string(),
                "Arduino".to_string(),
            ],
            exclude_ports: vec!["COM1".to_string(), "COM2".to_string()],
        }
    }
}

/// TUI configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Theme name: "dark", "light", "solarized", "dracula", "nord"
    pub theme: String,
    /// Refresh rate in Hz
    pub refresh_rate_hz: u32,
    /// Number of hex columns in hex view
    pub hex_columns: u8,
    /// Show timestamps in terminal output
    pub show_timestamps: bool,
    /// Command history size
    pub history_size: usize,
    /// Custom keybindings
    pub keybindings: KeybindingsConfig,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            refresh_rate_hz: 30,
            hex_columns: 16,
            show_timestamps: true,
            history_size: 1000,
            keybindings: KeybindingsConfig::default(),
        }
    }
}

impl TuiConfig {
    /// Get refresh interval as Duration
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_millis(1000 / self.refresh_rate_hz as u64)
    }
}

/// Keybindings configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    /// Quit key
    pub quit: String,
    /// Clear screen key
    pub clear: String,
    /// Send data key
    pub send: String,
    /// Toggle hex mode key
    pub hex_mode: String,
    /// Show help key
    pub help: String,
    /// Open config editor key
    pub config: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            clear: "ctrl+l".to_string(),
            send: "enter".to_string(),
            hex_mode: "ctrl+h".to_string(),
            help: "f1".to_string(),
            config: ":config".to_string(),
        }
    }
}

/// MCP server configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    /// Session database URL
    pub session_db: String,
    /// Maximum number of sessions
    pub max_sessions: usize,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            session_db: "sqlite://sessions.db".to_string(),
            max_sessions: 100,
            session_timeout_secs: 3600,
        }
    }
}

impl McpConfig {
    /// Get session timeout as Duration
    pub fn session_timeout(&self) -> Duration {
        Duration::from_secs(self.session_timeout_secs)
    }
}

/// Logging configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log file path (optional)
    pub file: Option<PathBuf>,
    /// Max log file size in MB
    pub max_size_mb: u32,
    /// Log rotation: "daily", "hourly", "size"
    pub rotation: String,
    /// Log format: "json", "pretty", "compact"
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            file: None,
            max_size_mb: 10,
            rotation: "daily".to_string(),
            format: LogFormat::Pretty,
        }
    }
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON format
    Json,
    /// Pretty format with colors
    Pretty,
    /// Compact format
    Compact,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Pretty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.serial.default_baud, 115200);
        assert_eq!(config.tui.theme, "dark");
    }

    #[test]
    fn test_port_alias_resolution() {
        let mut config = SerialConfig::default();
        config
            .port_aliases
            .insert("arduino".to_string(), "COM3".to_string());

        assert_eq!(config.resolve_port("arduino"), "COM3");
        assert_eq!(config.resolve_port("COM5"), "COM5");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("[serial]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [server]
            port = 8080

            [serial]
            default_baud = 9600
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.serial.default_baud, 9600);
        // Defaults should still work
        assert_eq!(config.tui.theme, "dark");
    }
}
