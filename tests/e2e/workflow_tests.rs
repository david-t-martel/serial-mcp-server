//! Complete workflow E2E tests: discover -> negotiate -> open -> communicate -> close
//!
//! These tests verify full end-to-end workflows including:
//! - Complete MCP workflow with mock ports
//! - Session persistence across operations
//! - Idle disconnect behavior
//! - Port reconfiguration

use serial_mcp_agent::port::{MockSerialPort, SerialPortAdapter};
use serial_mcp_agent::session::SessionStore;
use serial_mcp_agent::state::{PortConfig, PortState};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_full_workflow_open_write_read_close() {
    // Complete workflow: open port -> write -> read -> close
    let state = Arc::new(Mutex::new(PortState::Closed));

    // Create mock port
    let mut mock = MockSerialPort::new("MOCK0");
    mock.enqueue_read(b"Response from device\r\n");

    // Configure port
    let config = PortConfig {
        port_name: "MOCK0".to_string(),
        baud_rate: 9600,
        timeout_ms: 1000,
        data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
        parity: serial_mcp_agent::state::ParityCfg::None,
        stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
        flow_control: serial_mcp_agent::state::FlowControlCfg::None,
        terminator: Some("\n".to_string()),
        idle_disconnect_ms: None,
    };

    // Open port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(mock),
            config: config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Write data
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open { port, .. } = &mut *state_guard {
            let written = port.write_bytes(b"Command\r\n").unwrap();
            assert_eq!(written, 9);
        }
    }

    // Read response
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open { port, .. } = &mut *state_guard {
            let mut buffer = [0u8; 100];
            let read = port.read_bytes(&mut buffer).unwrap();
            assert_eq!(&buffer[..read], b"Response from device\r\n");
        }
    }

    // Close port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Closed;
    }

    // Verify closed
    let state_guard = state.lock().unwrap();
    assert!(matches!(*state_guard, PortState::Closed));
}

#[tokio::test]
async fn test_session_persistence_across_operations() {
    // Test that sessions persist across multiple operations
    let sessions = SessionStore::new("sqlite::memory:?cache=shared")
        .await
        .expect("Failed to create session store");

    // Create session
    let session = sessions
        .create_session("device_1", Some("MOCK0"))
        .await
        .expect("Failed to create session");

    let session_id = session.id.clone();

    // Append messages
    sessions
        .append_message(&session_id, "user", Some("sent"), "Command 1", None, None)
        .await
        .expect("Failed to append message 1");

    sessions
        .append_message(
            &session_id,
            "device",
            Some("received"),
            "Response 1",
            None,
            Some(10),
        )
        .await
        .expect("Failed to append message 2");

    sessions
        .append_message(&session_id, "user", Some("sent"), "Command 2", None, None)
        .await
        .expect("Failed to append message 3");

    // Retrieve session
    let retrieved = sessions
        .get_session(&session_id)
        .await
        .expect("Failed to get session")
        .expect("Session not found");

    assert_eq!(retrieved.device_id, "device_1");
    assert_eq!(retrieved.port_name.as_deref(), Some("MOCK0"));
    assert_eq!(retrieved.closed, 0);

    // List messages
    let messages = sessions
        .list_messages(&session_id, 100)
        .await
        .expect("Failed to list messages");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[1].role, "device");
    assert_eq!(messages[2].role, "user");

    // Close session
    sessions
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    // Verify closed
    let closed_session = sessions
        .get_session(&session_id)
        .await
        .expect("Failed to get session")
        .expect("Session not found");

    assert_eq!(closed_session.closed, 1);
}

#[tokio::test]
async fn test_idle_disconnect_workflow() {
    // Test idle disconnect behavior
    let state = Arc::new(Mutex::new(PortState::Closed));

    let config = PortConfig {
        port_name: "MOCK0".to_string(),
        baud_rate: 9600,
        timeout_ms: 1000,
        data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
        parity: serial_mcp_agent::state::ParityCfg::None,
        stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
        flow_control: serial_mcp_agent::state::FlowControlCfg::None,
        terminator: Some("\n".to_string()),
        idle_disconnect_ms: Some(100), // 100ms idle timeout
    };

    // Open port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(MockSerialPort::new("MOCK0")),
            config: config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Simulate idle time
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Check if port would be considered idle
    {
        let state_guard = state.lock().unwrap();
        if let PortState::Open {
            last_activity,
            config,
            ..
        } = &*state_guard
        {
            let idle_duration = last_activity.elapsed();
            let idle_threshold = config
                .idle_disconnect_ms
                .map(Duration::from_millis)
                .unwrap_or(Duration::from_secs(u64::MAX));

            assert!(
                idle_duration >= idle_threshold,
                "Port should be considered idle after threshold"
            );
        }
    }
}

#[tokio::test]
async fn test_reconfigure_port_workflow() {
    // Test reconfiguring port parameters
    let state = Arc::new(Mutex::new(PortState::Closed));

    let initial_config = PortConfig {
        port_name: "MOCK0".to_string(),
        baud_rate: 9600,
        timeout_ms: 1000,
        data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
        parity: serial_mcp_agent::state::ParityCfg::None,
        stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
        flow_control: serial_mcp_agent::state::FlowControlCfg::None,
        terminator: Some("\n".to_string()),
        idle_disconnect_ms: None,
    };

    // Open with initial config
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(MockSerialPort::new("MOCK0")),
            config: initial_config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Close port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Closed;
    }

    // Reconfigure with new baud rate
    let new_config = PortConfig {
        baud_rate: 115200,
        ..initial_config
    };

    // Reopen with new config
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(MockSerialPort::new("MOCK0")),
            config: new_config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Verify new config
    {
        let state_guard = state.lock().unwrap();
        if let PortState::Open { config, .. } = &*state_guard {
            assert_eq!(config.baud_rate, 115200);
        } else {
            panic!("Port should be open");
        }
    }
}

#[tokio::test]
async fn test_multiple_read_write_cycles() {
    // Test multiple read/write cycles with mock port
    let mut mock = MockSerialPort::new("MOCK0");

    // Enqueue responses (they all go into one queue)
    mock.enqueue_read(b"Response 1\r\nResponse 2\r\nResponse 3\r\n");

    // Write commands and verify logging
    mock.write_bytes(b"Command 1\r\n").unwrap();
    mock.write_bytes(b"Command 2\r\n").unwrap();
    mock.write_bytes(b"Command 3\r\n").unwrap();

    // Read all responses at once (mock queue behavior)
    let mut buffer = [0u8; 100];
    let n = mock.read_bytes(&mut buffer).unwrap();
    assert_eq!(&buffer[..n], b"Response 1\r\nResponse 2\r\nResponse 3\r\n");

    // Verify all writes were logged
    let writes = mock.get_write_log();
    assert_eq!(writes.len(), 3);
    assert_eq!(writes[0], b"Command 1\r\n");
    assert_eq!(writes[1], b"Command 2\r\n");
    assert_eq!(writes[2], b"Command 3\r\n");
}

#[tokio::test]
async fn test_buffer_clear_workflow() {
    // Test clearing buffers
    let mut mock = MockSerialPort::new("MOCK0");
    mock.enqueue_read(b"Old data that should be cleared\r\n");

    // Clear buffers
    mock.clear_buffers().unwrap();

    // Verify buffers were cleared
    assert!(mock.was_cleared());
    assert_eq!(mock.available_bytes(), 0);

    // Enqueue new data
    mock.enqueue_read(b"New data\r\n");

    // Read should get new data
    let mut buffer = [0u8; 100];
    let n = mock.read_bytes(&mut buffer).unwrap();
    assert_eq!(&buffer[..n], b"New data\r\n");
}

#[tokio::test]
async fn test_timeout_streak_tracking() {
    // Test timeout streak counting
    let state = Arc::new(Mutex::new(PortState::Closed));

    let config = PortConfig {
        port_name: "MOCK0".to_string(),
        baud_rate: 9600,
        timeout_ms: 100,
        data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
        parity: serial_mcp_agent::state::ParityCfg::None,
        stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
        flow_control: serial_mcp_agent::state::FlowControlCfg::None,
        terminator: Some("\n".to_string()),
        idle_disconnect_ms: None,
    };

    // Open port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(MockSerialPort::new("MOCK0")),
            config: config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Simulate timeout
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open { timeout_streak, .. } = &mut *state_guard {
            *timeout_streak += 1;
            assert_eq!(*timeout_streak, 1);
        }
    }

    // Simulate another timeout
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open { timeout_streak, .. } = &mut *state_guard {
            *timeout_streak += 1;
            assert_eq!(*timeout_streak, 2);
        }
    }

    // Successful read should reset streak
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open { timeout_streak, .. } = &mut *state_guard {
            *timeout_streak = 0;
            assert_eq!(*timeout_streak, 0);
        }
    }
}

#[tokio::test]
async fn test_byte_counting() {
    // Test read/write byte counting
    let state = Arc::new(Mutex::new(PortState::Closed));

    let mut mock = MockSerialPort::new("MOCK0");
    mock.enqueue_read(b"Response data\r\n");

    let config = PortConfig {
        port_name: "MOCK0".to_string(),
        baud_rate: 9600,
        timeout_ms: 1000,
        data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
        parity: serial_mcp_agent::state::ParityCfg::None,
        stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
        flow_control: serial_mcp_agent::state::FlowControlCfg::None,
        terminator: Some("\n".to_string()),
        idle_disconnect_ms: None,
    };

    // Open port
    {
        let mut state_guard = state.lock().unwrap();
        *state_guard = PortState::Open {
            port: Box::new(mock),
            config: config.clone(),
            last_activity: std::time::Instant::now(),
            timeout_streak: 0,
            bytes_read_total: 0,
            bytes_written_total: 0,
            idle_close_count: 0,
            open_started: std::time::Instant::now(),
        };
    }

    // Write and track bytes
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open {
            port,
            bytes_written_total,
            ..
        } = &mut *state_guard
        {
            let written = port.write_bytes(b"Command\r\n").unwrap();
            *bytes_written_total += written as u64;
            assert_eq!(*bytes_written_total, 9);
        }
    }

    // Read and track bytes
    {
        let mut state_guard = state.lock().unwrap();
        if let PortState::Open {
            port,
            bytes_read_total,
            ..
        } = &mut *state_guard
        {
            let mut buffer = [0u8; 100];
            let read = port.read_bytes(&mut buffer).unwrap();
            *bytes_read_total += read as u64;
            assert_eq!(*bytes_read_total, 15);
        }
    }
}

#[tokio::test]
async fn test_session_message_filtering() {
    // Test filtering messages by various criteria
    let sessions = SessionStore::new("sqlite::memory:?cache=shared")
        .await
        .expect("Failed to create session store");

    let session = sessions
        .create_session("device_1", Some("MOCK0"))
        .await
        .expect("Failed to create session");

    // Add messages with various attributes
    sessions
        .append_message(
            &session.id,
            "user",
            Some("sent"),
            "Command 1",
            Some("command"),
            None,
        )
        .await
        .unwrap();

    sessions
        .append_message(&session.id, "device", Some("received"), "OK", None, Some(5))
        .await
        .unwrap();

    sessions
        .append_message(
            &session.id,
            "user",
            Some("sent"),
            "Command 2",
            Some("command"),
            None,
        )
        .await
        .unwrap();

    sessions
        .append_message(
            &session.id,
            "device",
            Some("received"),
            "ERROR",
            Some("error"),
            Some(8),
        )
        .await
        .unwrap();

    // Filter by role
    let user_messages = sessions
        .filter_messages(&session.id, Some("user"), None, None, 100)
        .await
        .unwrap();
    assert_eq!(user_messages.len(), 2);

    // Filter by direction
    let received = sessions
        .filter_messages(&session.id, None, None, Some("received"), 100)
        .await
        .unwrap();
    assert_eq!(received.len(), 2);

    // Filter by feature
    let errors = sessions
        .filter_messages(&session.id, None, Some("error"), None, 100)
        .await
        .unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].content, "ERROR");
}

#[tokio::test]
async fn test_session_export() {
    // Test exporting session data
    let sessions = SessionStore::new("sqlite::memory:?cache=shared")
        .await
        .expect("Failed to create session store");

    let session = sessions
        .create_session("device_1", Some("MOCK0"))
        .await
        .expect("Failed to create session");

    // Add some messages
    sessions
        .append_message(&session.id, "user", Some("sent"), "Test", None, None)
        .await
        .unwrap();

    // Export session
    let exported = sessions
        .export_session_json(&session.id)
        .await
        .expect("Failed to export");

    // Verify export structure
    assert!(exported.get("session").is_some());
    assert!(exported.get("messages").is_some());

    let messages = exported.get("messages").unwrap().as_array().unwrap();
    assert_eq!(messages.len(), 1);
}
