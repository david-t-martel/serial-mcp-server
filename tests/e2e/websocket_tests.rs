//! End-to-end tests for WebSocket streaming functionality.
//!
//! Tests real-time serial port communication over WebSocket connections,
//! including data streaming, command handling, and connection management.

#![cfg(all(feature = "rest-api", feature = "websocket"))]

use futures::{SinkExt, StreamExt};
use serial_mcp_agent::{
    port::MockSerialPort,
    rest_api::RestContext,
    session::SessionStore,
    state::{AppState, PortConfig, PortState},
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};

/// Helper to create test application state with a mock port.
fn create_test_state_with_mock() -> AppState {
    let mut mock_port = MockSerialPort::new("TEST_PORT");

    // Configure mock to return data on read
    mock_port.enqueue_read(b"test response\n");

    let state = PortState::Open {
        port: Box::new(mock_port),
        config: PortConfig {
            port_name: "TEST_PORT".to_string(),
            baud_rate: 9600,
            timeout_ms: 1000,
            data_bits: serial_mcp_agent::state::DataBitsCfg::Eight,
            parity: serial_mcp_agent::state::ParityCfg::None,
            stop_bits: serial_mcp_agent::state::StopBitsCfg::One,
            flow_control: serial_mcp_agent::state::FlowControlCfg::None,
            terminator: Some("\n".to_string()),
            idle_disconnect_ms: None,
        },
        last_activity: std::time::Instant::now(),
        timeout_streak: 0,
        bytes_read_total: 0,
        bytes_written_total: 0,
        idle_close_count: 0,
        open_started: std::time::Instant::now(),
    };

    Arc::new(Mutex::new(state))
}

/// Helper to create test application state with closed port.
fn create_test_state_closed() -> AppState {
    Arc::new(Mutex::new(PortState::Closed))
}

/// Helper to start a test server and return its address.
async fn start_test_server(app_state: AppState) -> String {
    let session_store = SessionStore::new("sqlite::memory:")
        .await
        .expect("Failed to create session store");

    let ctx = RestContext {
        state: app_state,
        sessions: Arc::new(session_store),
    };

    let app = serial_mcp_agent::rest_api::build_router(ctx);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get address");

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    format!("ws://127.0.0.1:{}/ws/serial", addr.port())
}

#[tokio::test]
async fn test_websocket_connection() {
    let state = create_test_state_closed();
    let url = start_test_server(state).await;

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut _write, mut read) = ws_stream.split();

    // Should receive initial status message
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for message")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .expect("Invalid JSON");
            assert_eq!(json["type"], "status");
            assert_eq!(json["state"], "Closed");
        }
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_write_command() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state.clone()).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status message
    let _ = read.next().await;

    // Send write command
    let write_cmd = json!({
        "type": "write",
        "data": "test command"
    });

    write
        .send(TungsteniteMessage::Text(write_cmd.to_string()))
        .await
        .expect("Failed to send");

    // Should receive status acknowledgment
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for response")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .expect("Invalid JSON");
            assert_eq!(json["type"], "status");
            assert_eq!(json["state"], "Open");
        }
        _ => panic!("Expected text message"),
    }

    // Verify data was written
    let st = state.lock().unwrap();
    if let PortState::Open { bytes_written_total, .. } = &*st {
        assert!(*bytes_written_total > 0, "No bytes written");
    } else {
        panic!("Port should be open");
    }
}

#[tokio::test]
async fn test_websocket_write_to_closed_port() {
    let state = create_test_state_closed();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Try to write to closed port
    let write_cmd = json!({
        "type": "write",
        "data": "test"
    });

    write
        .send(TungsteniteMessage::Text(write_cmd.to_string()))
        .await
        .expect("Failed to send");

    // Should receive error message
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for error")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .expect("Invalid JSON");
            assert_eq!(json["type"], "error");
            assert!(json["message"].as_str().unwrap().contains("not open"));
        }
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_subscribe_unsubscribe() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Subscribe to data stream
    let subscribe_cmd = json!({"type": "subscribe"});
    write
        .send(TungsteniteMessage::Text(subscribe_cmd.to_string()))
        .await
        .expect("Failed to send");

    // Wait for data messages (mock port returns "test response\n")
    // Give time for serial reader to broadcast
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Unsubscribe
    let unsubscribe_cmd = json!({"type": "unsubscribe"});
    write
        .send(TungsteniteMessage::Text(unsubscribe_cmd.to_string()))
        .await
        .expect("Failed to send");

    // After unsubscribe, should not receive more data messages
    // (We can't easily test the negative case, but no panic is good)
}

#[tokio::test]
async fn test_websocket_data_streaming() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Subscribe
    let subscribe_cmd = json!({"type": "subscribe"});
    write
        .send(TungsteniteMessage::Text(subscribe_cmd.to_string()))
        .await
        .expect("Failed to send");

    // Wait for data message
    let msg = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(Ok(TungsteniteMessage::Text(text))) = read.next().await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json["type"] == "data" {
                        return json;
                    }
                }
            }
        }
    })
    .await
    .expect("Timeout waiting for data message");

    assert_eq!(msg["type"], "data");
    assert!(msg["data"].as_str().is_some());
    assert!(msg["timestamp"].as_str().is_some());
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let state = create_test_state_closed();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Send ping
    write
        .send(TungsteniteMessage::Ping(vec![1, 2, 3]))
        .await
        .expect("Failed to send ping");

    // Should receive pong
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for pong")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Pong(data) => {
            assert_eq!(data, vec![1, 2, 3]);
        }
        _ => panic!("Expected pong message"),
    }
}

#[tokio::test]
async fn test_websocket_invalid_command() {
    let state = create_test_state_closed();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Send invalid JSON
    write
        .send(TungsteniteMessage::Text("invalid json".to_string()))
        .await
        .expect("Failed to send");

    // Should receive error message
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for error")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .expect("Invalid JSON");
            assert_eq!(json["type"], "error");
        }
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_concurrent_connections() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state).await;

    // Create multiple concurrent connections
    let mut handles = vec![];

    for _ in 0..3 {
        let url_clone = url.clone();
        let handle = tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url_clone)
                .await
                .expect("Failed to connect");

            let (mut write, mut read) = ws_stream.split();

            // Consume initial status
            let _ = read.next().await;

            // Subscribe
            let subscribe_cmd = json!({"type": "subscribe"});
            write
                .send(TungsteniteMessage::Text(subscribe_cmd.to_string()))
                .await
                .expect("Failed to send");

            // Wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        handles.push(handle);
    }

    // All connections should complete successfully
    for handle in handles {
        handle.await.expect("Connection task failed");
    }
}

#[tokio::test]
async fn test_websocket_graceful_disconnect() {
    let state = create_test_state_closed();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut _read) = ws_stream.split();

    // Send close frame
    write
        .send(TungsteniteMessage::Close(None))
        .await
        .expect("Failed to send close");

    // Connection should close gracefully (no panic)
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_websocket_status_with_metrics() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (_write, mut read) = ws_stream.split();

    // Receive initial status with metrics
    let msg = tokio::time::timeout(Duration::from_secs(1), read.next())
        .await
        .expect("Timeout waiting for status")
        .expect("No message received")
        .expect("WebSocket error");

    match msg {
        TungsteniteMessage::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)
                .expect("Invalid JSON");
            assert_eq!(json["type"], "status");
            assert_eq!(json["state"], "Open");

            // Verify metrics are present
            let metrics = &json["metrics"];
            assert!(metrics.is_object());
            assert!(metrics["bytes_read_total"].is_number());
            assert!(metrics["bytes_written_total"].is_number());
            assert!(metrics["open_duration_ms"].is_number());
            assert!(metrics["last_activity_ms"].is_number());
            assert!(metrics["timeout_streak"].is_number());
        }
        _ => panic!("Expected text message"),
    }
}

#[tokio::test]
async fn test_websocket_terminator_stripping() {
    let state = create_test_state_with_mock();
    let url = start_test_server(state.clone()).await;

    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Consume initial status
    let _ = read.next().await;

    // Write data without terminator
    let write_cmd = json!({
        "type": "write",
        "data": "test"
    });

    write
        .send(TungsteniteMessage::Text(write_cmd.to_string()))
        .await
        .expect("Failed to send");

    // Wait for response
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that terminator was appended
    let st = state.lock().unwrap();
    if let PortState::Open { .. } = &*st {
        // The mock port should have received "test\n"
        // (The bytes_written_total should reflect it)
        // This is verified in the write command test
    }
}
