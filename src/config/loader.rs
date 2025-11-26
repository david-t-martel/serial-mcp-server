//! Configuration loader with file resolution and environment override support.

use super::error::{ConfigError, ConfigResult};
use super::schema::Config;
use std::path::{Path, PathBuf};

/// Environment variable prefix for overrides
const ENV_PREFIX: &str = "RUST_COMM";

/// Config file name
const CONFIG_FILE_NAME: &str = "config.toml";

/// Environment variable for explicit config path
const CONFIG_PATH_ENV: &str = "RUST_COMM_CONFIG";

/// Configuration loader with resolution and override logic.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Resolved config file path (if any)
    pub config_path: Option<PathBuf>,
    /// The loaded configuration
    pub config: Config,
}

impl ConfigLoader {
    /// Load configuration using standard resolution order.
    ///
    /// Resolution priority (highest to lowest):
    /// 1. `RUST_COMM_CONFIG` environment variable (explicit path)
    /// 2. `./config.toml` (current directory)
    /// 3. `~/.config/rust-comm/config.toml` (XDG on Linux/macOS)
    /// 4. `%APPDATA%\rust-comm\config.toml` (Windows)
    /// 5. Built-in defaults (no file required)
    ///
    /// Environment variables can override any config file values.
    pub fn load() -> ConfigResult<Self> {
        let config_path = resolve_config_path();

        let mut config = if let Some(ref path) = config_path {
            load_from_file(path)?
        } else {
            Config::default()
        };

        apply_env_overrides(&mut config)?;

        Ok(Self { config_path, config })
    }

    /// Load configuration from a specific file path.
    pub fn load_from(path: impl AsRef<Path>) -> ConfigResult<Self> {
        let path = path.as_ref().to_path_buf();
        let mut config = load_from_file(&path)?;
        apply_env_overrides(&mut config)?;

        Ok(Self {
            config_path: Some(path),
            config,
        })
    }

    /// Create a loader with default configuration (no file).
    pub fn with_defaults() -> Self {
        let mut config = Config::default();
        // Still apply env overrides even with defaults
        let _ = apply_env_overrides(&mut config);

        Self {
            config_path: None,
            config,
        }
    }

    /// Get the loaded configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Consume the loader and return the configuration.
    pub fn into_config(self) -> Config {
        self.config
    }

    /// Save the current configuration to file.
    pub fn save(&self) -> ConfigResult<()> {
        let path = self
            .config_path
            .as_ref()
            .ok_or_else(|| ConfigError::MissingRequired("No config file path set".to_string()))?;

        save_to_file(&self.config, path)
    }

    /// Save the current configuration to a specific file.
    pub fn save_to(&self, path: impl AsRef<Path>) -> ConfigResult<()> {
        save_to_file(&self.config, path.as_ref())
    }

    /// Reload configuration from file (if path is set).
    pub fn reload(&mut self) -> ConfigResult<()> {
        if let Some(ref path) = self.config_path {
            self.config = load_from_file(path)?;
            apply_env_overrides(&mut self.config)?;
        }
        Ok(())
    }
}

/// Resolve the configuration file path using standard locations.
pub fn resolve_config_path() -> Option<PathBuf> {
    // 1. Explicit environment variable
    if let Ok(path) = std::env::var(CONFIG_PATH_ENV) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. Current directory
    let cwd_config = PathBuf::from(CONFIG_FILE_NAME);
    if cwd_config.exists() {
        return Some(cwd_config);
    }

    // 3. XDG config directory (Linux/macOS) or APPDATA (Windows)
    if let Some(config_dir) = get_config_dir() {
        let app_config = config_dir.join("rust-comm").join(CONFIG_FILE_NAME);
        if app_config.exists() {
            return Some(app_config);
        }
    }

    // 4. No config file found - will use defaults
    None
}

/// Get the platform-specific config directory.
fn get_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    }
}

/// Load configuration from a file.
fn load_from_file(path: &Path) -> ConfigResult<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
        path: path.to_path_buf(),
        source: e,
    })?;

    toml::from_str(&content).map_err(ConfigError::ParseError)
}

/// Save configuration to a file.
fn save_to_file(config: &Config, path: &Path) -> ConfigResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigError::WriteError {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content).map_err(|e| ConfigError::WriteError {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Apply environment variable overrides to the configuration.
///
/// Environment variables follow the pattern: `RUST_COMM_<SECTION>_<KEY>`
/// For example:
/// - `RUST_COMM_SERVER_PORT=8080`
/// - `RUST_COMM_SERIAL_DEFAULT_BAUD=9600`
/// - `RUST_COMM_TESTING_PORT=COM15`
fn apply_env_overrides(config: &mut Config) -> ConfigResult<()> {
    // Server overrides
    if let Ok(val) = std::env::var(format!("{}_SERVER_HOST", ENV_PREFIX)) {
        config.server.host = val;
    }
    if let Ok(val) = std::env::var(format!("{}_SERVER_PORT", ENV_PREFIX)) {
        config.server.port = val.parse().map_err(|_| {
            ConfigError::env_parse(format!("{}_SERVER_PORT", ENV_PREFIX), "Invalid port number")
        })?;
    }
    if let Ok(val) = std::env::var(format!("{}_SERVER_LOG_LEVEL", ENV_PREFIX)) {
        config.server.log_level = val;
    }

    // Serial overrides
    if let Ok(val) = std::env::var(format!("{}_SERIAL_DEFAULT_BAUD", ENV_PREFIX)) {
        config.serial.default_baud = val.parse().map_err(|_| {
            ConfigError::env_parse(
                format!("{}_SERIAL_DEFAULT_BAUD", ENV_PREFIX),
                "Invalid baud rate",
            )
        })?;
    }
    if let Ok(val) = std::env::var(format!("{}_SERIAL_DEFAULT_TIMEOUT_MS", ENV_PREFIX)) {
        config.serial.default_timeout_ms = val.parse().map_err(|_| {
            ConfigError::env_parse(
                format!("{}_SERIAL_DEFAULT_TIMEOUT_MS", ENV_PREFIX),
                "Invalid timeout",
            )
        })?;
    }

    // Testing overrides (also support legacy TEST_PORT etc.)
    if let Ok(val) = std::env::var(format!("{}_TESTING_PORT", ENV_PREFIX))
        .or_else(|_| std::env::var("TEST_PORT"))
    {
        config.testing.port = Some(val);
    }
    if let Ok(val) = std::env::var(format!("{}_TESTING_BAUD", ENV_PREFIX))
        .or_else(|_| std::env::var("TEST_BAUD"))
    {
        config.testing.baud = val.parse().map_err(|_| {
            ConfigError::env_parse(
                format!("{}_TESTING_BAUD or TEST_BAUD", ENV_PREFIX),
                "Invalid baud rate",
            )
        })?;
    }
    if let Ok(val) = std::env::var(format!("{}_TESTING_TIMEOUT_MS", ENV_PREFIX))
        .or_else(|_| std::env::var("TEST_TIMEOUT"))
    {
        config.testing.timeout_ms = val.parse().map_err(|_| {
            ConfigError::env_parse(
                format!("{}_TESTING_TIMEOUT_MS or TEST_TIMEOUT", ENV_PREFIX),
                "Invalid timeout",
            )
        })?;
    }
    if let Ok(val) = std::env::var("LOOPBACK_ENABLED") {
        config.testing.loopback_enabled = val.to_lowercase() == "true" || val == "1";
    }

    // MCP overrides
    if let Ok(val) = std::env::var(format!("{}_MCP_SESSION_DB", ENV_PREFIX))
        .or_else(|_| std::env::var("SESSION_DB_URL"))
    {
        config.mcp.session_db = val;
    }

    // TUI overrides
    if let Ok(val) = std::env::var(format!("{}_TUI_THEME", ENV_PREFIX)) {
        config.tui.theme = val;
    }

    Ok(())
}

/// Get the default config directory for creating new config files.
pub fn get_default_config_dir() -> Option<PathBuf> {
    get_config_dir().map(|d| d.join("rust-comm"))
}

/// Get the default config file path for creating new config files.
pub fn get_default_config_path() -> Option<PathBuf> {
    get_default_config_dir().map(|d| d.join(CONFIG_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_loader() {
        let loader = ConfigLoader::with_defaults();
        assert_eq!(loader.config().server.port, 3000);
    }

    #[test]
    fn test_env_override() {
        // Set environment variable
        env::set_var("RUST_COMM_SERVER_PORT", "9999");

        let loader = ConfigLoader::with_defaults();
        assert_eq!(loader.config().server.port, 9999);

        // Clean up
        env::remove_var("RUST_COMM_SERVER_PORT");
    }

    #[test]
    fn test_legacy_test_port_env() {
        env::set_var("TEST_PORT", "COM99");
        env::set_var("TEST_BAUD", "57600");

        let loader = ConfigLoader::with_defaults();
        assert_eq!(loader.config().testing.port, Some("COM99".to_string()));
        assert_eq!(loader.config().testing.baud, 57600);

        // Clean up
        env::remove_var("TEST_PORT");
        env::remove_var("TEST_BAUD");
    }
}
