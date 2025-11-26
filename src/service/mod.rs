//! Service layer for serial port operations.
//!
//! This module provides a clean abstraction layer that decouples business logic
//! from API handlers (REST, MCP, WebSocket). All port operations flow through
//! the PortService, which manages state and provides consistent error handling.
//!
//! # Architecture
//!
//! ```text
//! REST API ─┐
//! MCP API  ─┼──> PortService ──> AppState (Arc<Mutex<PortState>>)
//! WebSocket─┘
//! ```
//!
//! # Benefits
//!
//! - **Single Responsibility**: Service handles port logic, handlers handle protocol
//! - **DRY**: Eliminates duplication between REST and MCP handlers
//! - **Testability**: Service can be tested independently of HTTP/MCP layers
//! - **Type Safety**: Strong typing with dedicated result types

use crate::{
    port::{DataBits, FlowControl, Parity, PortConfiguration, StopBits, SyncSerialPort},
    state::{AppState, DataBitsCfg, FlowControlCfg, ParityCfg, PortConfig, PortState, StopBitsCfg},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ========== Error Types ==========

/// Service-specific errors for port operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// Port is already open when attempting to open
    PortAlreadyOpen,
    /// Port is closed when operation requires it to be open
    PortNotOpen,
    /// State lock is poisoned (critical concurrency failure)
    StateLockPoisoned,
    /// Invalid configuration parameter
    InvalidConfig(String),
    /// Port operation failed
    PortError(String),
    /// No port name provided when required
    NoPortSpecified,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PortAlreadyOpen => write!(f, "Port is already open"),
            Self::PortNotOpen => write!(f, "Port is not open"),
            Self::StateLockPoisoned => write!(f, "State lock is poisoned"),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::PortError(msg) => write!(f, "Port operation failed: {}", msg),
            Self::NoPortSpecified => write!(f, "No port name specified"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Convenient Result type for service operations
pub type ServiceResult<T> = Result<T, ServiceError>;

// ========== Request/Response DTOs ==========

/// Configuration for opening a port
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub data_bits: DataBitsCfg,
    pub parity: ParityCfg,
    pub stop_bits: StopBitsCfg,
    pub flow_control: FlowControlCfg,
    pub terminator: Option<String>,
    pub idle_disconnect_ms: Option<u64>,
}

/// Configuration for reconfiguring a port
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReconfigureConfig {
    pub port_name: Option<String>,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub data_bits: DataBitsCfg,
    pub parity: ParityCfg,
    pub stop_bits: StopBitsCfg,
    pub flow_control: FlowControlCfg,
    pub terminator: Option<String>,
    pub idle_disconnect_ms: Option<u64>,
}

/// Result from opening a port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenResult {
    pub port_name: String,
    pub baud_rate: u32,
    pub message: String,
}

/// Result from closing a port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseResult {
    pub message: String,
}

/// Result from writing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub bytes_written: usize,
    pub bytes_written_total: u64,
}

/// Result from reading data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    pub data: String,
    pub bytes_read: usize,
    pub bytes_read_total: u64,
    /// If Some, indicates the port was auto-closed due to idle timeout
    pub auto_closed: Option<AutoCloseInfo>,
}

/// Information about an auto-close event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCloseInfo {
    pub reason: String,
    pub idle_close_count: u64,
}

/// Port status information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "PascalCase")]
pub enum StatusResult {
    Closed,
    Open {
        config: PortConfig,
        #[serde(skip_serializing_if = "Option::is_none")]
        metrics: Option<PortMetrics>,
    },
}

/// Port metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMetrics {
    pub bytes_read_total: u64,
    pub bytes_written_total: u64,
    pub idle_close_count: u64,
    pub open_duration_ms: u64,
    pub last_activity_ms: u64,
    pub timeout_streak: u32,
}

/// Detailed port metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResult {
    pub state: String,
    pub bytes_read_total: Option<u64>,
    pub bytes_written_total: Option<u64>,
    pub idle_close_count: Option<u64>,
    pub open_duration_ms: Option<u64>,
    pub last_activity_ms: Option<u64>,
    pub timeout_streak: Option<u32>,
}

// ========== Service Implementation ==========

/// Port service providing business logic for serial port operations.
///
/// This service encapsulates all port management logic, allowing API handlers
/// to focus on protocol-specific concerns (HTTP, MCP, WebSocket).
#[derive(Clone)]
pub struct PortService {
    state: AppState,
}

impl PortService {
    /// Create a new port service with the given shared state.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Open a serial port with the specified configuration.
    ///
    /// # Errors
    ///
    /// - `ServiceError::PortAlreadyOpen` if a port is already open
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    /// - `ServiceError::PortError` if the port cannot be opened
    pub fn open(&self, config: OpenConfig) -> ServiceResult<OpenResult> {
        let mut st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        // Check if port is already open
        if matches!(&*st, PortState::Open { .. }) {
            return Err(ServiceError::PortAlreadyOpen);
        }

        // Convert config enums to port module types
        let port_config = PortConfiguration {
            baud_rate: config.baud_rate,
            data_bits: Self::convert_data_bits(config.data_bits),
            parity: Self::convert_parity(config.parity),
            stop_bits: Self::convert_stop_bits(config.stop_bits),
            flow_control: Self::convert_flow_control(config.flow_control),
            timeout: Duration::from_millis(config.timeout_ms),
        };

        // Open the port
        let port = SyncSerialPort::open(&config.port_name, port_config)
            .map_err(|e| ServiceError::PortError(e.to_string()))?;

        // Update state
        *st = PortState::Open {
            port: Box::new(port),
            config: PortConfig {
                port_name: config.port_name.clone(),
                baud_rate: config.baud_rate,
                timeout_ms: config.timeout_ms,
                data_bits: config.data_bits,
                parity: config.parity,
                stop_bits: config.stop_bits,
                flow_control: config.flow_control,
                terminator: config.terminator,
                idle_disconnect_ms: config.idle_disconnect_ms,
            },
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };

        Ok(OpenResult {
            port_name: config.port_name,
            baud_rate: config.baud_rate,
            message: "opened".to_string(),
        })
    }

    /// Close the currently open port.
    ///
    /// This operation is idempotent - closing an already-closed port succeeds.
    ///
    /// # Errors
    ///
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    pub fn close(&self) -> ServiceResult<CloseResult> {
        let mut st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        let message = match &*st {
            PortState::Closed => "already closed".to_string(),
            PortState::Open { .. } => {
                *st = PortState::Closed;
                "closed".to_string()
            }
        };

        Ok(CloseResult { message })
    }

    /// Write data to the open port.
    ///
    /// If a terminator is configured and the data doesn't end with it,
    /// the terminator will be automatically appended.
    ///
    /// # Errors
    ///
    /// - `ServiceError::PortNotOpen` if no port is open
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    /// - `ServiceError::PortError` if the write operation fails
    pub fn write(&self, data: &str) -> ServiceResult<WriteResult> {
        let mut st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        match &mut *st {
            PortState::Open {
                port,
                config,
                last_activity,
                bytes_written_total,
                ..
            } => {
                // Prepare data with terminator if configured
                let mut write_data = data.to_string();
                if let Some(term) = &config.terminator {
                    if !write_data.ends_with(term) {
                        write_data.push_str(term);
                    }
                }

                // Write to port
                let bytes = port
                    .write_bytes(write_data.as_bytes())
                    .map_err(|e| ServiceError::PortError(e.to_string()))?;

                // Update metrics
                *bytes_written_total += bytes as u64;
                *last_activity = std::time::Instant::now();

                Ok(WriteResult {
                    bytes_written: bytes,
                    bytes_written_total: *bytes_written_total,
                })
            }
            PortState::Closed => Err(ServiceError::PortNotOpen),
        }
    }

    /// Read data from the open port.
    ///
    /// Reads up to 1024 bytes. If a terminator is configured, it will be
    /// stripped from the returned data. Timeouts are handled gracefully
    /// and return zero-length data.
    ///
    /// If idle disconnect is configured and the timeout is reached, the port
    /// will be automatically closed and the result will indicate this.
    ///
    /// # Errors
    ///
    /// - `ServiceError::PortNotOpen` if no port is open
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    /// - `ServiceError::PortError` if a non-timeout read error occurs
    pub fn read(&self) -> ServiceResult<ReadResult> {
        let mut st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        // Extract read result while holding lock
        let result = match &mut *st {
            PortState::Open {
                port,
                config,
                last_activity,
                timeout_streak,
                bytes_read_total,
                idle_close_count,
                ..
            } => {
                let mut buffer = vec![0u8; 1024];

                // Attempt read
                let bytes_read = match port.read_bytes(buffer.as_mut_slice()) {
                    Ok(n) => n,
                    Err(e) => {
                        // Check if it's a timeout error
                        if let crate::port::PortError::Io(ref io_err) = e {
                            if io_err.kind() == std::io::ErrorKind::TimedOut {
                                0 // Treat timeout as zero bytes read
                            } else {
                                return Err(ServiceError::PortError(e.to_string()));
                            }
                        } else {
                            return Err(ServiceError::PortError(e.to_string()));
                        }
                    }
                };

                let raw = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

                // Update metrics
                if bytes_read > 0 {
                    *last_activity = std::time::Instant::now();
                    *timeout_streak = 0;
                    *bytes_read_total += bytes_read as u64;
                } else {
                    *timeout_streak += 1;
                }

                // Check for idle timeout
                let idle_expired = bytes_read == 0
                    && config
                        .idle_disconnect_ms
                        .map(|ms| last_activity.elapsed() >= Duration::from_millis(ms))
                        .unwrap_or(false);

                if idle_expired {
                    *idle_close_count += 1;
                    let count = *idle_close_count;
                    // Return early to indicate port should be closed
                    Err((count, *bytes_read_total))
                } else {
                    // Strip terminator if configured
                    let data = if let Some(term) = &config.terminator {
                        raw.trim_end_matches(term).to_string()
                    } else {
                        raw
                    };

                    Ok((data, bytes_read, *bytes_read_total))
                }
            }
            PortState::Closed => return Err(ServiceError::PortNotOpen),
        };

        // Handle result outside borrow scope
        match result {
            Ok((data, bytes_read, total)) => Ok(ReadResult {
                data,
                bytes_read,
                bytes_read_total: total,
                auto_closed: None,
            }),
            Err((idle_count, total)) => {
                // Close the port due to idle timeout
                *st = PortState::Closed;
                Ok(ReadResult {
                    data: String::new(),
                    bytes_read: 0,
                    bytes_read_total: total,
                    auto_closed: Some(AutoCloseInfo {
                        reason: "idle_timeout".to_string(),
                        idle_close_count: idle_count,
                    }),
                })
            }
        }
    }

    /// Reconfigure the port (close and reopen with new settings).
    ///
    /// If no port_name is provided in the config, uses the currently open port's name.
    /// This operation resets all metrics (bytes read/written, idle close count).
    ///
    /// # Errors
    ///
    /// - `ServiceError::NoPortSpecified` if no port name provided and no port is open
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    /// - `ServiceError::PortError` if the port cannot be opened with new settings
    pub fn reconfigure(&self, config: ReconfigureConfig) -> ServiceResult<OpenResult> {
        let mut st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        // Determine target port name
        let target = match (&config.port_name, &*st) {
            (Some(p), _) => p.clone(),
            (None, PortState::Open { config, .. }) => config.port_name.clone(),
            (None, PortState::Closed) => return Err(ServiceError::NoPortSpecified),
        };

        // Build port configuration
        let port_config = PortConfiguration {
            baud_rate: config.baud_rate,
            data_bits: Self::convert_data_bits(config.data_bits),
            parity: Self::convert_parity(config.parity),
            stop_bits: Self::convert_stop_bits(config.stop_bits),
            flow_control: Self::convert_flow_control(config.flow_control),
            timeout: Duration::from_millis(config.timeout_ms),
        };

        // Open port with new configuration
        let port = SyncSerialPort::open(&target, port_config)
            .map_err(|e| ServiceError::PortError(e.to_string()))?;

        // Replace state
        *st = PortState::Open {
            port: Box::new(port),
            config: PortConfig {
                port_name: target.clone(),
                baud_rate: config.baud_rate,
                timeout_ms: config.timeout_ms,
                data_bits: config.data_bits,
                parity: config.parity,
                stop_bits: config.stop_bits,
                flow_control: config.flow_control,
                terminator: config.terminator,
                idle_disconnect_ms: config.idle_disconnect_ms,
            },
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };

        Ok(OpenResult {
            port_name: target,
            baud_rate: config.baud_rate,
            message: "reconfigured".to_string(),
        })
    }

    /// Get current port status.
    ///
    /// # Errors
    ///
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    pub fn status(&self) -> ServiceResult<StatusResult> {
        let st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        let result = match &*st {
            PortState::Closed => StatusResult::Closed,
            PortState::Open {
                config,
                bytes_read_total,
                bytes_written_total,
                idle_close_count,
                open_started,
                last_activity,
                timeout_streak,
                ..
            } => StatusResult::Open {
                config: config.clone(),
                metrics: Some(PortMetrics {
                    bytes_read_total: *bytes_read_total,
                    bytes_written_total: *bytes_written_total,
                    idle_close_count: *idle_close_count,
                    open_duration_ms: open_started.elapsed().as_millis() as u64,
                    last_activity_ms: last_activity.elapsed().as_millis() as u64,
                    timeout_streak: *timeout_streak,
                }),
            },
        };

        Ok(result)
    }

    /// Get port metrics.
    ///
    /// # Errors
    ///
    /// - `ServiceError::StateLockPoisoned` if the state lock is poisoned
    pub fn metrics(&self) -> ServiceResult<MetricsResult> {
        let st = self
            .state
            .lock()
            .map_err(|_| ServiceError::StateLockPoisoned)?;

        let result = match &*st {
            PortState::Closed => MetricsResult {
                state: "Closed".to_string(),
                bytes_read_total: None,
                bytes_written_total: None,
                idle_close_count: None,
                open_duration_ms: None,
                last_activity_ms: None,
                timeout_streak: None,
            },
            PortState::Open {
                bytes_read_total,
                bytes_written_total,
                idle_close_count,
                open_started,
                last_activity,
                timeout_streak,
                ..
            } => MetricsResult {
                state: "Open".to_string(),
                bytes_read_total: Some(*bytes_read_total),
                bytes_written_total: Some(*bytes_written_total),
                idle_close_count: Some(*idle_close_count),
                open_duration_ms: Some(open_started.elapsed().as_millis() as u64),
                last_activity_ms: Some(last_activity.elapsed().as_millis() as u64),
                timeout_streak: Some(*timeout_streak),
            },
        };

        Ok(result)
    }

    /// Check if a port is currently open.
    ///
    /// Returns false if the state lock is poisoned.
    pub fn is_open(&self) -> bool {
        self.state
            .lock()
            .map(|st| matches!(&*st, PortState::Open { .. }))
            .unwrap_or(false)
    }

    // ========== Helper Methods ==========

    fn convert_data_bits(bits: DataBitsCfg) -> DataBits {
        match bits {
            DataBitsCfg::Five => DataBits::Five,
            DataBitsCfg::Six => DataBits::Six,
            DataBitsCfg::Seven => DataBits::Seven,
            DataBitsCfg::Eight => DataBits::Eight,
        }
    }

    fn convert_parity(parity: ParityCfg) -> Parity {
        match parity {
            ParityCfg::None => Parity::None,
            ParityCfg::Odd => Parity::Odd,
            ParityCfg::Even => Parity::Even,
        }
    }

    fn convert_stop_bits(bits: StopBitsCfg) -> StopBits {
        match bits {
            StopBitsCfg::One => StopBits::One,
            StopBitsCfg::Two => StopBits::Two,
        }
    }

    fn convert_flow_control(flow: FlowControlCfg) -> FlowControl {
        match flow {
            FlowControlCfg::None => FlowControl::None,
            FlowControlCfg::Hardware => FlowControl::Hardware,
            FlowControlCfg::Software => FlowControl::Software,
        }
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PortState;
    use std::sync::{Arc, Mutex};

    fn create_test_service() -> PortService {
        let state = Arc::new(Mutex::new(PortState::Closed));
        PortService::new(state)
    }

    #[allow(dead_code)]
    fn create_open_config(port_name: &str) -> OpenConfig {
        OpenConfig {
            port_name: port_name.to_string(),
            baud_rate: 9600,
            timeout_ms: 1000,
            data_bits: DataBitsCfg::Eight,
            parity: ParityCfg::None,
            stop_bits: StopBitsCfg::One,
            flow_control: FlowControlCfg::None,
            terminator: Some("\n".to_string()),
            idle_disconnect_ms: None,
        }
    }

    #[test]
    fn test_service_creation() {
        let service = create_test_service();
        assert!(!service.is_open());
    }

    #[test]
    fn test_close_when_already_closed() {
        let service = create_test_service();
        let result = service.close();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message, "already closed");
    }

    #[test]
    fn test_write_when_not_open() {
        let service = create_test_service();
        let result = service.write("test");
        assert!(matches!(result, Err(ServiceError::PortNotOpen)));
    }

    #[test]
    fn test_read_when_not_open() {
        let service = create_test_service();
        let result = service.read();
        assert!(matches!(result, Err(ServiceError::PortNotOpen)));
    }

    #[test]
    fn test_status_when_closed() {
        let service = create_test_service();
        let result = service.status();
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), StatusResult::Closed));
    }

    #[test]
    fn test_metrics_when_closed() {
        let service = create_test_service();
        let result = service.metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.state, "Closed");
        assert!(metrics.bytes_read_total.is_none());
        assert!(metrics.bytes_written_total.is_none());
    }

    #[test]
    fn test_reconfigure_without_port_name_when_closed() {
        let service = create_test_service();
        let config = ReconfigureConfig {
            port_name: None,
            baud_rate: 9600,
            timeout_ms: 1000,
            data_bits: DataBitsCfg::Eight,
            parity: ParityCfg::None,
            stop_bits: StopBitsCfg::One,
            flow_control: FlowControlCfg::None,
            terminator: None,
            idle_disconnect_ms: None,
        };
        let result = service.reconfigure(config);
        assert!(matches!(result, Err(ServiceError::NoPortSpecified)));
    }

    #[test]
    fn test_service_error_display() {
        assert_eq!(
            ServiceError::PortAlreadyOpen.to_string(),
            "Port is already open"
        );
        assert_eq!(ServiceError::PortNotOpen.to_string(), "Port is not open");
        assert_eq!(
            ServiceError::StateLockPoisoned.to_string(),
            "State lock is poisoned"
        );
        assert_eq!(
            ServiceError::InvalidConfig("test".to_string()).to_string(),
            "Invalid configuration: test"
        );
        assert_eq!(
            ServiceError::PortError("test".to_string()).to_string(),
            "Port operation failed: test"
        );
        assert_eq!(
            ServiceError::NoPortSpecified.to_string(),
            "No port name specified"
        );
    }

    #[test]
    fn test_convert_data_bits() {
        assert_eq!(
            PortService::convert_data_bits(DataBitsCfg::Five),
            DataBits::Five
        );
        assert_eq!(
            PortService::convert_data_bits(DataBitsCfg::Six),
            DataBits::Six
        );
        assert_eq!(
            PortService::convert_data_bits(DataBitsCfg::Seven),
            DataBits::Seven
        );
        assert_eq!(
            PortService::convert_data_bits(DataBitsCfg::Eight),
            DataBits::Eight
        );
    }

    #[test]
    fn test_convert_parity() {
        assert_eq!(PortService::convert_parity(ParityCfg::None), Parity::None);
        assert_eq!(PortService::convert_parity(ParityCfg::Odd), Parity::Odd);
        assert_eq!(PortService::convert_parity(ParityCfg::Even), Parity::Even);
    }

    #[test]
    fn test_convert_stop_bits() {
        assert_eq!(
            PortService::convert_stop_bits(StopBitsCfg::One),
            StopBits::One
        );
        assert_eq!(
            PortService::convert_stop_bits(StopBitsCfg::Two),
            StopBits::Two
        );
    }

    #[test]
    fn test_convert_flow_control() {
        assert_eq!(
            PortService::convert_flow_control(FlowControlCfg::None),
            FlowControl::None
        );
        assert_eq!(
            PortService::convert_flow_control(FlowControlCfg::Hardware),
            FlowControl::Hardware
        );
        assert_eq!(
            PortService::convert_flow_control(FlowControlCfg::Software),
            FlowControl::Software
        );
    }

    #[test]
    fn test_service_error_eq() {
        assert_eq!(ServiceError::PortAlreadyOpen, ServiceError::PortAlreadyOpen);
        assert_eq!(ServiceError::PortNotOpen, ServiceError::PortNotOpen);
        assert_ne!(ServiceError::PortAlreadyOpen, ServiceError::PortNotOpen);
    }
}
