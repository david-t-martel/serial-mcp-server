//! WebSocket streaming for real-time serial port communication.
//!
//! Provides bidirectional WebSocket endpoints for streaming serial data to clients
//! and accepting write commands in real-time. Supports multiple concurrent connections
//! with proper backpressure handling and clean disconnection.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State as AxumState, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tracing::{debug, error, info, warn};

use crate::{
    rest_api::RestContext,
    state::PortState,
};

/// Maximum number of messages buffered per WebSocket connection.
/// Prevents slow clients from consuming unlimited memory.
const WS_BUFFER_SIZE: usize = 100;

/// Interval for reading serial data when port is open (milliseconds).
const SERIAL_READ_INTERVAL_MS: u64 = 50;

/// WebSocket message types for client communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMessage {
    /// Data received from serial port
    Data {
        data: String,
        timestamp: String,
    },
    /// Port status update
    Status {
        state: PortStatusState,
        #[serde(skip_serializing_if = "Option::is_none")]
        metrics: Option<PortMetrics>,
    },
    /// Error notification
    Error {
        message: String,
    },
}

/// Incoming WebSocket commands from clients.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsCommand {
    /// Write data to serial port
    Write { data: String },
    /// Subscribe to serial data stream
    Subscribe,
    /// Unsubscribe from serial data stream
    Unsubscribe,
}

/// Port connection state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum PortStatusState {
    Open,
    Closed,
}

/// Port metrics for status updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortMetrics {
    bytes_read_total: u64,
    bytes_written_total: u64,
    open_duration_ms: u64,
    last_activity_ms: u64,
    timeout_streak: u32,
}

/// Shared state for broadcasting serial data to all connected WebSocket clients.
#[derive(Clone)]
struct BroadcastState {
    tx: broadcast::Sender<WsMessage>,
}

impl BroadcastState {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(WS_BUFFER_SIZE);
        Self { tx }
    }

    fn broadcast(&self, msg: WsMessage) {
        // Ignore send errors - they just mean no active receivers
        let _ = self.tx.send(msg);
    }

    fn subscribe(&self) -> BroadcastStream<WsMessage> {
        BroadcastStream::new(self.tx.subscribe())
    }
}

/// WebSocket upgrade handler.
///
/// This is the main entry point for WebSocket connections at `/ws/serial`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(ctx): AxumState<RestContext>,
) -> impl IntoResponse {
    // Create a broadcast channel for this connection
    let broadcast_state = BroadcastState::new();
    let broadcast_clone = broadcast_state.clone();

    // Spawn a background task to read from serial port and broadcast data
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        serial_reader_task(ctx_clone, broadcast_clone).await;
    });

    // Upgrade the HTTP connection to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, ctx, broadcast_state))
}

/// Main WebSocket connection handler.
///
/// Manages bidirectional communication:
/// - Receives commands from client (write, subscribe, unsubscribe)
/// - Sends serial data, status updates, and errors to client
async fn handle_socket(socket: WebSocket, ctx: RestContext, broadcast: BroadcastState) {
    let (mut sender, mut receiver) = socket.split();
    let client_id = uuid::Uuid::new_v4();

    info!("WebSocket client connected: {}", client_id);

    // Track subscription state
    let mut subscribed = false;
    let mut broadcast_stream = broadcast.subscribe();

    // Send initial status
    if let Err(e) = send_status(&mut sender, &ctx).await {
        error!("Failed to send initial status to {}: {}", client_id, e);
        return;
    }

    loop {
        tokio::select! {
            // Handle incoming messages from WebSocket client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let result = handle_client_message(&text, &ctx, &mut sender, &mut subscribed).await;
                        if let Err(e) = result {
                            let error_msg = format!("Command error: {}", e);
                            drop(e); // Explicitly drop the error before await
                            error!("Error handling client message from {}: {}", client_id, error_msg);
                            let _ = send_error(&mut sender, &error_msg).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client {} disconnected", client_id);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            error!("Failed to send pong to {}: {}", client_id, e);
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types (Binary, Pong)
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for {}: {}", client_id, e);
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended for {}", client_id);
                        break;
                    }
                }
            }

            // Handle broadcast messages (serial data, status updates)
            msg = broadcast_stream.next(), if subscribed => {
                match msg {
                    Some(Ok(ws_msg)) => {
                        if let Err(e) = send_message(&mut sender, &ws_msg).await {
                            error!("Failed to send broadcast to {}: {}", client_id, e);
                            break;
                        }
                    }
                    Some(Err(BroadcastStreamRecvError::Lagged(skipped))) => {
                        warn!("Client {} lagged, skipped {} messages", client_id, skipped);
                        let _ = send_error(&mut sender, &format!("Lagged: {} messages skipped", skipped)).await;
                    }
                    None => {
                        debug!("Broadcast stream ended for {}", client_id);
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket handler finished for {}", client_id);
}

/// Handle incoming client command messages.
async fn handle_client_message(
    text: &str,
    ctx: &RestContext,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    subscribed: &mut bool,
) -> Result<(), String> {
    let command: WsCommand = serde_json::from_str(text).map_err(|e| e.to_string())?;

    match command {
        WsCommand::Write { data } => {
            handle_write_command(ctx, data, sender).await?;
        }
        WsCommand::Subscribe => {
            *subscribed = true;
            debug!("Client subscribed to serial data stream");
        }
        WsCommand::Unsubscribe => {
            *subscribed = false;
            debug!("Client unsubscribed from serial data stream");
        }
    }

    Ok(())
}

/// Handle write command - write data to serial port.
async fn handle_write_command(
    ctx: &RestContext,
    data: String,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<(), String> {
    // Perform the write and collect the response message
    let response = {
        let mut st = ctx.state.lock().map_err(|e| format!("State lock error: {}", e))?;

        match &mut *st {
            PortState::Open {
                port,
                config,
                last_activity,
                bytes_written_total,
                ..
            } => {
                let mut write_data = data.clone();

                // Append terminator if configured
                if let Some(term) = &config.terminator {
                    if !write_data.ends_with(term) {
                        write_data.push_str(term);
                    }
                }

                match port.write_bytes(write_data.as_bytes()) {
                    Ok(bytes) => {
                        *bytes_written_total += bytes as u64;
                        *last_activity = std::time::Instant::now();

                        debug!("Wrote {} bytes to serial port", bytes);

                        // Build acknowledgment
                        Ok(WsMessage::Status {
                            state: PortStatusState::Open,
                            metrics: Some(PortMetrics {
                                bytes_read_total: 0, // Not tracked here
                                bytes_written_total: *bytes_written_total,
                                open_duration_ms: 0,
                                last_activity_ms: 0,
                                timeout_streak: 0,
                            }),
                        })
                    }
                    Err(e) => {
                        error!("Write error: {}", e);
                        Err(format!("Write failed: {}", e))
                    }
                }
            }
            PortState::Closed => Err("Port not open".to_string()),
        }
    }; // st is dropped here

    // Send the response (after mutex is released)
    match response {
        Ok(msg) => send_message(sender, &msg).await?,
        Err(error_msg) => send_error(sender, &error_msg).await?,
    }

    Ok(())
}

/// Background task that continuously reads from serial port and broadcasts data.
async fn serial_reader_task(ctx: RestContext, broadcast: BroadcastState) {
    let mut interval = tokio::time::interval(Duration::from_millis(SERIAL_READ_INTERVAL_MS));

    loop {
        interval.tick().await;

        // Check if lock failed
        let lock_ok = ctx.state.lock().is_ok();
        if !lock_ok {
            error!("Failed to acquire state lock in reader task");
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // Check port state and read data
        let read_result = {
            let mut st = ctx.state.lock().unwrap();

            match &mut *st {
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

                    match port.read_bytes(buffer.as_mut_slice()) {
                        Ok(bytes_read) if bytes_read > 0 => {
                            let raw = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();

                            // Update metrics
                            *last_activity = std::time::Instant::now();
                            *timeout_streak = 0;
                            *bytes_read_total += bytes_read as u64;

                            // Strip terminator if configured
                            let data = if let Some(term) = &config.terminator {
                                raw.trim_end_matches(term).to_string()
                            } else {
                                raw
                            };

                            Some(Ok(data))
                        }
                        Ok(_) => {
                            // No data (timeout)
                            *timeout_streak += 1;

                            // Check for idle disconnect
                            let idle_expired = config
                                .idle_disconnect_ms
                                .map(|ms| last_activity.elapsed() >= Duration::from_millis(ms))
                                .unwrap_or(false);

                            if idle_expired {
                                *idle_close_count += 1;
                                Some(Err("idle_timeout".to_string()))
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            // Check if it's a timeout (not an error)
                            if let crate::port::PortError::Io(ref io_err) = e {
                                if io_err.kind() == std::io::ErrorKind::TimedOut {
                                    *timeout_streak += 1;
                                    None
                                } else {
                                    Some(Err(e.to_string()))
                                }
                            } else {
                                Some(Err(e.to_string()))
                            }
                        }
                    }
                }
                PortState::Closed => None,
            }
        };

        // Process read result and broadcast
        match read_result {
            Some(Ok(data)) => {
                let msg = WsMessage::Data {
                    data,
                    timestamp: Utc::now().to_rfc3339(),
                };
                broadcast.broadcast(msg);
            }
            Some(Err(error_msg)) => {
                if error_msg == "idle_timeout" {
                    // Port was closed due to idle timeout
                    let msg = WsMessage::Status {
                        state: PortStatusState::Closed,
                        metrics: None,
                    };
                    broadcast.broadcast(msg);

                    // Close the port
                    let mut st = ctx.state.lock().unwrap();
                    *st = PortState::Closed;
                } else {
                    // Other error
                    let msg = WsMessage::Error {
                        message: error_msg,
                    };
                    broadcast.broadcast(msg);
                }
            }
            None => {
                // No data to broadcast (normal timeout or port closed)
            }
        }
    }
}

/// Send a WebSocket message to the client.
async fn send_message(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: &WsMessage,
) -> Result<(), String> {
    let json = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    sender.send(Message::Text(json.into())).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Send an error message to the client.
async fn send_error(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    error_msg: &str,
) -> Result<(), String> {
    let msg = WsMessage::Error {
        message: error_msg.to_string(),
    };
    send_message(sender, &msg).await
}

/// Send current port status to the client.
async fn send_status(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    ctx: &RestContext,
) -> Result<(), String> {
    let msg = {
        let st = ctx.state.lock().map_err(|e| format!("State lock error: {}", e))?;

        match &*st {
            PortState::Closed => WsMessage::Status {
                state: PortStatusState::Closed,
                metrics: None,
            },
            PortState::Open {
                bytes_read_total,
                bytes_written_total,
                open_started,
                last_activity,
                timeout_streak,
                ..
            } => WsMessage::Status {
                state: PortStatusState::Open,
                metrics: Some(PortMetrics {
                    bytes_read_total: *bytes_read_total,
                    bytes_written_total: *bytes_written_total,
                    open_duration_ms: open_started.elapsed().as_millis() as u64,
                    last_activity_ms: last_activity.elapsed().as_millis() as u64,
                    timeout_streak: *timeout_streak,
                }),
            },
        }
    }; // st is dropped here

    send_message(sender, &msg).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Data {
            data: "test data".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "data");
        assert_eq!(json["data"], "test data");
        assert_eq!(json["timestamp"], "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_ws_command_deserialization() {
        let json = json!({
            "type": "write",
            "data": "test"
        });

        let cmd: WsCommand = serde_json::from_value(json).unwrap();
        match cmd {
            WsCommand::Write { data } => assert_eq!(data, "test"),
            _ => panic!("Expected Write command"),
        }
    }

    #[test]
    fn test_subscribe_command() {
        let json = json!({"type": "subscribe"});
        let cmd: WsCommand = serde_json::from_value(json).unwrap();
        matches!(cmd, WsCommand::Subscribe);
    }

    #[test]
    fn test_unsubscribe_command() {
        let json = json!({"type": "unsubscribe"});
        let cmd: WsCommand = serde_json::from_value(json).unwrap();
        matches!(cmd, WsCommand::Unsubscribe);
    }

    #[test]
    fn test_error_message_serialization() {
        let msg = WsMessage::Error {
            message: "test error".to_string(),
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["message"], "test error");
    }

    #[test]
    fn test_status_message_open() {
        let msg = WsMessage::Status {
            state: PortStatusState::Open,
            metrics: Some(PortMetrics {
                bytes_read_total: 100,
                bytes_written_total: 50,
                open_duration_ms: 1000,
                last_activity_ms: 100,
                timeout_streak: 0,
            }),
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "status");
        assert_eq!(json["state"], "Open");
        assert_eq!(json["metrics"]["bytes_read_total"], 100);
        assert_eq!(json["metrics"]["bytes_written_total"], 50);
    }

    #[test]
    fn test_status_message_closed() {
        let msg = WsMessage::Status {
            state: PortStatusState::Closed,
            metrics: None,
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "status");
        assert_eq!(json["state"], "Closed");
        assert!(json.get("metrics").is_none() || json["metrics"].is_null());
    }
}
