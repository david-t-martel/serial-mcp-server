//! Shared test utilities for Serial MCP Server tests.
//!
//! This module provides common test infrastructure including:
//! - Mock port creation with pre-programmed responses
//! - Test harness for state management
//! - JSON assertion helpers
//! - Common test data builders

#![allow(dead_code)]

use serde_json::Value;
use serial_mcp_agent::port::{MockSerialPort, SerialPortAdapter};
use serial_mcp_agent::session::SessionStore;
use serial_mcp_agent::state::{AppState, PortState};
use std::sync::{Arc, Mutex};

/// Create a mock serial port with pre-programmed responses.
///
/// # Arguments
/// * `port_name` - The name for the mock port (e.g., "MOCK0")
/// * `responses` - List of byte arrays to return on subsequent reads
///
/// # Example
/// ```ignore
/// let mock = create_mock_port_with_responses("MOCK0", vec![b"OK\r\n", b"READY\r\n"]);
/// ```
pub fn create_mock_port_with_responses(port_name: &str, responses: Vec<&[u8]>) -> MockSerialPort {
    let mut mock = MockSerialPort::new(port_name);
    for response in responses {
        mock.enqueue_read(response);
    }
    mock
}

/// Create a mock port that simulates manufacturer identification.
///
/// # Arguments
/// * `manufacturer_id` - The manufacturer/product ID response
pub fn create_manufacturer_mock_port(port_name: &str, manufacturer_id: &str) -> MockSerialPort {
    let mut mock = MockSerialPort::new(port_name);
    // Simulate typical manufacturer identification response
    mock.enqueue_read(format!("{}\r\n", manufacturer_id).as_bytes());
    mock
}

/// Create a mock port that echoes data back (for loopback testing).
pub struct EchoMockPort {
    inner: MockSerialPort,
}

impl EchoMockPort {
    pub fn new(port_name: &str) -> Self {
        Self {
            inner: MockSerialPort::new(port_name),
        }
    }

    /// Get a mutable reference to the underlying mock port.
    pub fn inner_mut(&mut self) -> &mut MockSerialPort {
        &mut self.inner
    }

    /// Consume and return the inner mock port.
    pub fn into_inner(self) -> MockSerialPort {
        self.inner
    }
}

/// Assert that a JSON value contains specific fields with expected values.
///
/// # Example
/// ```ignore
/// let actual = json!({"status": "ok", "port": "COM1"});
/// let expected = json!({"status": "ok"});
/// assert_json_contains(&actual, &expected); // Passes - actual contains all of expected
/// ```
pub fn assert_json_contains(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Object(actual_map), Value::Object(expected_map)) => {
            for (key, expected_value) in expected_map {
                let actual_value = actual_map
                    .get(key)
                    .unwrap_or_else(|| panic!("Expected key '{}' not found in actual JSON", key));
                assert_json_contains(actual_value, expected_value);
            }
        }
        (Value::Array(actual_arr), Value::Array(expected_arr)) => {
            assert_eq!(actual_arr.len(), expected_arr.len(), "Array lengths differ");
            for (actual_item, expected_item) in actual_arr.iter().zip(expected_arr.iter()) {
                assert_json_contains(actual_item, expected_item);
            }
        }
        _ => {
            assert_eq!(
                actual, expected,
                "JSON values differ: expected {:?}, got {:?}",
                expected, actual
            );
        }
    }
}

/// Test harness that provides a complete test environment with state and sessions.
pub struct TestHarness {
    pub state: AppState,
    pub sessions: Arc<SessionStore>,
}

impl TestHarness {
    /// Create a new test harness with in-memory session storage.
    pub async fn new() -> Self {
        let state = Arc::new(Mutex::new(PortState::Closed));
        let sessions = Arc::new(
            SessionStore::new("sqlite::memory:?cache=shared")
                .await
                .expect("Failed to create in-memory session store"),
        );

        Self { state, sessions }
    }

    /// Create a test harness with a pre-configured mock port.
    pub async fn with_mock_port(mock: MockSerialPort) -> Self {
        let harness = Self::new().await;

        // Open the port with the mock
        let config = serial_mcp_agent::state::PortConfig {
            port_name: mock.name().to_string(),
            baud_rate: 9600,
            timeout_ms: 1000,
            data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
            parity: serial_mcp_agent::state::ParityCfg::None,
            stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
            flow_control: serial_mcp_agent::state::FlowControlCfg::None,
            terminator: Some("\n".to_string()),
            idle_disconnect_ms: None,
        };

        let mut state_guard = harness.state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(mock),
            config,
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
        drop(state_guard);

        harness
    }

    /// Get the current port state.
    pub fn get_state(&self) -> PortState {
        // This is a simplified version - in real tests you'd clone or extract what you need
        match &*self.state.lock().unwrap() {
            PortState::Closed => PortState::Closed,
            PortState::Open { config, .. } => {
                // Return a simplified open state for testing
                PortState::Open {
                    port: Box::new(MockSerialPort::new("TEST")),
                    config: config.clone(),
                    last_activity: std::time::Instant::now(),
                    timeout_streak: 0,
                    bytes_read_total: 0,
                    bytes_written_total: 0,
                    idle_close_count: 0,
                    open_started: std::time::Instant::now(),
                }
            }
        }
    }

    /// Check if port is currently open.
    pub fn is_port_open(&self) -> bool {
        matches!(&*self.state.lock().unwrap(), PortState::Open { .. })
    }
}

/// Builder for creating test port configurations.
pub struct PortConfigBuilder {
    port_name: String,
    baud_rate: u32,
    timeout_ms: u64,
}

impl PortConfigBuilder {
    pub fn new(port_name: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            baud_rate: 9600,
            timeout_ms: 1000,
        }
    }

    pub fn baud_rate(mut self, baud: u32) -> Self {
        self.baud_rate = baud;
        self
    }

    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }

    pub fn build(self) -> serial_mcp_agent::state::PortConfig {
        serial_mcp_agent::state::PortConfig {
            port_name: self.port_name,
            baud_rate: self.baud_rate,
            timeout_ms: self.timeout_ms,
            data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
            parity: serial_mcp_agent::state::ParityCfg::None,
            stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
            flow_control: serial_mcp_agent::state::FlowControlCfg::None,
            terminator: Some("\n".to_string()),
            idle_disconnect_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mock_port_with_responses() {
        let mut mock = create_mock_port_with_responses("MOCK0", vec![b"Hello", b"World"]);

        let mut buf = [0u8; 5]; // Smaller buffer to read one word at a time
        let n = mock.read_bytes(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"Hello");

        let n = mock.read_bytes(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"World");
    }

    #[test]
    fn test_assert_json_contains_object() {
        let actual = serde_json::json!({
            "status": "ok",
            "port": "COM1",
            "extra": "data"
        });
        let expected = serde_json::json!({
            "status": "ok",
            "port": "COM1"
        });

        assert_json_contains(&actual, &expected);
    }

    #[test]
    #[should_panic(expected = "Expected key 'missing' not found")]
    fn test_assert_json_contains_missing_key() {
        let actual = serde_json::json!({"status": "ok"});
        let expected = serde_json::json!({"missing": "key"});

        assert_json_contains(&actual, &expected);
    }

    #[test]
    fn test_port_config_builder() {
        let config = PortConfigBuilder::new("COM1")
            .baud_rate(115200)
            .timeout_ms(500)
            .build();

        assert_eq!(config.port_name, "COM1");
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.timeout_ms, 500);
    }

    #[tokio::test]
    async fn test_harness_creation() {
        let harness = TestHarness::new().await;
        assert!(!harness.is_port_open());
    }

    #[tokio::test]
    async fn test_harness_with_mock_port() {
        let mock = MockSerialPort::new("MOCK0");
        let harness = TestHarness::with_mock_port(mock).await;
        assert!(harness.is_port_open());
    }
}
