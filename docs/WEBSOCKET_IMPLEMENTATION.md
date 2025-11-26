# WebSocket Streaming Implementation (Phase 5)

## Overview

Phase 5 WebSocket streaming provides real-time bidirectional communication for serial port data. This implementation enables clients to receive live serial data streams and send write commands via WebSocket connections.

## Architecture

### Components

1. **WebSocket Module** (`src/websocket.rs`)
   - Main WebSocket endpoint handler at `/ws/serial`
   - Bidirectional message routing (client ↔ serial port)
   - Broadcast-based message distribution to multiple clients
   - Thread-safe state management with proper mutex scoping

2. **Integration** (`src/rest_api.rs`)
   - Feature-gated WebSocket route integration
   - Shared application state with REST endpoints
   - Seamless integration with existing port management

3. **Test Suite** (`tests/e2e/websocket_tests.rs`)
   - 11 comprehensive end-to-end tests
   - Coverage includes: connections, streaming, commands, errors
   - All tests passing with sequentially execution

## Message Protocol

### Incoming Messages (Client → Server)

#### Write Command
```json
{
  "type": "write",
  "data": "command data"
}
```
Writes data to the serial port. Automatically appends configured terminator if not present.

#### Subscribe
```json
{
  "type": "subscribe"
}
```
Subscribes the client to receive serial data stream.

#### Unsubscribe
```json
{
  "type": "unsubscribe"
}
```
Unsubscribes the client from serial data stream.

### Outgoing Messages (Server → Client)

#### Data Message
```json
{
  "type": "data",
  "data": "received serial data",
  "timestamp": "2024-01-01T00:00:00Z"
}
```
Serial data received from the port. Terminators are stripped if configured.

#### Status Message
```json
{
  "type": "status",
  "state": "Open",
  "metrics": {
    "bytes_read_total": 1024,
    "bytes_written_total": 512,
    "open_duration_ms": 30000,
    "last_activity_ms": 100,
    "timeout_streak": 0
  }
}
```
Port status and metrics. Sent on connection and after write operations.

#### Error Message
```json
{
  "type": "error",
  "message": "Error description"
}
```
Error notifications for failed operations or protocol violations.

## Key Features

### Real-Time Data Streaming
- Continuous serial data broadcast to subscribed clients
- 50ms polling interval for optimal latency/CPU balance
- Automatic terminator stripping based on port configuration

### Multiple Concurrent Connections
- Broadcast architecture supports unlimited simultaneous clients
- Each client maintains independent subscription state
- Buffer size limits (100 messages) prevent slow client memory exhaustion

### Backpressure Handling
- Slow clients skip messages when buffer fills (lagging)
- Lag notifications sent to affected clients
- Fast clients never blocked by slow clients

### Thread-Safe State Access
- Proper mutex guard scoping prevents deadlocks
- No holding locks across await points
- Send-safe error types throughout

### Clean Disconnection
- Graceful WebSocket close handling
- Automatic resource cleanup
- Background task termination

## Implementation Details

### Broadcast Pattern
- Single serial reader task broadcasts to all clients
- `tokio::sync::broadcast` channel for efficient multi-consumer
- `BroadcastStream` wrapper for async stream integration

### Error Handling
- `String` error types for Send compatibility
- No `Box<dyn Error>` held across await points
- Explicit error conversion at boundaries

### Connection Lifecycle
```
1. Client connects → WebSocket upgrade
2. Send initial status → Port state + metrics
3. Client sends subscribe → Enable data streaming
4. Serial reader broadcasts data → Client receives stream
5. Client sends write → Data written to port
6. Client closes → Resources cleaned up
```

## Testing

### Test Coverage
- ✅ Basic connection and status
- ✅ Write commands (success and error cases)
- ✅ Subscribe/unsubscribe lifecycle
- ✅ Data streaming with mock serial port
- ✅ Ping/pong frames
- ✅ Invalid command handling
- ✅ Concurrent connections (3 simultaneous)
- ✅ Graceful disconnection
- ✅ Terminator handling
- ✅ Closed port error handling
- ✅ Port metrics with open port

### Running Tests
```bash
# All WebSocket tests
cargo test --release --features websocket --test integration_e2e -- websocket

# Sequential execution (recommended for timing-sensitive tests)
cargo test --release --features websocket --test integration_e2e -- websocket --test-threads=1

# Specific test
cargo test --release --features websocket test_websocket_data_streaming
```

## Configuration

### Feature Flag
The WebSocket functionality is gated behind the `websocket` feature:

```toml
[features]
websocket = ["rest-api", "tokio-stream", "futures"]
```

### Dependencies
- `tokio-stream` (with `sync` feature) for broadcast streams
- `futures` for WebSocket stream operations
- `axum` (with `ws` feature) for WebSocket support

### Build
```bash
# Build with WebSocket support
cargo build --release --features websocket

# Default build (includes websocket via default features)
cargo build --release
```

## Usage Example

### JavaScript Client
```javascript
const ws = new WebSocket('ws://localhost:3000/ws/serial');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  switch (msg.type) {
    case 'status':
      console.log('Port status:', msg.state);
      if (msg.metrics) {
        console.log('Metrics:', msg.metrics);
      }
      break;

    case 'data':
      console.log('Serial data:', msg.data, 'at', msg.timestamp);
      break;

    case 'error':
      console.error('Error:', msg.message);
      break;
  }
};

// Subscribe to data stream
ws.send(JSON.stringify({ type: 'subscribe' }));

// Write to serial port
ws.send(JSON.stringify({
  type: 'write',
  data: 'AT+COMMAND\r\n'
}));

// Unsubscribe
ws.send(JSON.stringify({ type: 'unsubscribe' }));
```

### Rust Client
```rust
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures::{SinkExt, StreamExt};

let (ws_stream, _) = connect_async("ws://localhost:3000/ws/serial").await?;
let (mut write, mut read) = ws_stream.split();

// Subscribe
write.send(Message::Text(r#"{"type":"subscribe"}"#.into())).await?;

// Read messages
while let Some(msg) = read.next().await {
    match msg? {
        Message::Text(text) => {
            let json: serde_json::Value = serde_json::from_str(&text)?;
            println!("Received: {:?}", json);
        }
        _ => {}
    }
}
```

## Performance Characteristics

- **Latency**: ~50ms average (polling interval)
- **Throughput**: Limited by serial port baud rate
- **Memory**: ~100 messages × message size per client
- **CPU**: Minimal (single background task)
- **Concurrent Clients**: Unlimited (broadcast architecture)

## Limitations and Future Enhancements

### Current Limitations
- Fixed 50ms polling interval (not configurable)
- No authentication/authorization
- No rate limiting per client
- Single port per server instance

### Planned Enhancements
- Configurable polling interval
- Token-based authentication
- Rate limiting and quotas
- Multi-port support (port selection in connection URL)
- Binary data support
- Compression for high-throughput scenarios

## File Locations

- **Implementation**: `C:\codedev\rust-comm\src\websocket.rs` (478 lines)
- **Integration**: `C:\codedev\rust-comm\src\rest_api.rs` (lines 117-121)
- **Library Module**: `C:\codedev\rust-comm\src\lib.rs` (lines 33-35)
- **Tests**: `C:\codedev\rust-comm\tests\e2e\websocket_tests.rs` (493 lines)
- **E2E Module**: `C:\codedev\rust-comm\tests\e2e\mod.rs` (lines 11-13)

## Dependencies Added

### Cargo.toml
- `tokio-stream = { version = "0.1", features = ["sync"], optional = true }`
- `futures = { version = "0.3", optional = true }` (feature-gated)

### Dev Dependencies
- `tokio-tungstenite = "0.24"` (for WebSocket client testing)

## Verification

```bash
# Compile check
cargo build --release --features websocket

# Run all tests
cargo test --release --features websocket

# Run only WebSocket tests
cargo test --release --features websocket --test integration_e2e -- websocket --test-threads=1

# Generate documentation
cargo doc --no-deps --features websocket --open
```

## Integration with Existing Features

### REST API
- WebSocket endpoint coexists with REST endpoints
- Shares application state and session store
- Same port configuration and management

### MCP Server
- Independent of MCP stdio interface
- Can run simultaneously in server mode
- Complementary real-time alternative to polling REST endpoints

### Session Management
- WebSocket connections don't create sessions (read-only to serial)
- Can be enhanced to log WebSocket interactions to sessions

## Security Considerations

### Current Implementation
- No authentication (intended for local/trusted networks)
- No TLS/WSS support (HTTP only)
- No input validation beyond JSON parsing

### Recommended Production Setup
- Deploy behind reverse proxy (nginx, Caddy)
- Enable TLS at proxy level (WSS)
- Implement authentication middleware
- Add rate limiting
- Validate all input data thoroughly

## Troubleshooting

### Tests Failing
- Ensure running with `--test-threads=1` for timing-sensitive tests
- Check that port 0 (random) binding is available
- Verify `websocket` feature is enabled

### Connection Refused
- Server must be started with `--server` flag
- Check firewall allows connection on server port
- Verify WebSocket feature is compiled in

### No Data Received
- Send `{"type":"subscribe"}` message after connecting
- Ensure serial port is open with data available
- Check mock port has data enqueued in tests

## Summary

Phase 5 WebSocket streaming successfully implements real-time bidirectional serial communication with:
- ✅ Full message protocol (write, subscribe, unsubscribe)
- ✅ Real-time data streaming with 50ms latency
- ✅ Multiple concurrent client support
- ✅ Backpressure handling for slow clients
- ✅ Thread-safe state management
- ✅ Comprehensive test coverage (11 tests, all passing)
- ✅ Clean integration with existing REST API
- ✅ Production-ready error handling
- ✅ Complete documentation

The implementation is production-ready for local/trusted network deployments and provides a solid foundation for future enhancements.
