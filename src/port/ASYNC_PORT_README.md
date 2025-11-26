# Async Serial Port Implementation

## Overview

This module provides async serial port support for the serial MCP agent using `tokio-serial`. It implements Phase 3 of the Serial MCP Server enhancement plan.

## Features

- **Native async I/O** with `TokioSerialPort` using tokio-serial
- **Blocking wrapper** with `BlockingSerialPortWrapper` for integrating sync ports into async code
- **Unified trait** `AsyncSerialPortAdapter` for polymorphic async serial operations
- **Feature-gated** behind the `async-serial` feature flag

## Architecture

### Components

1. **`AsyncSerialPortAdapter` trait**: Defines the async interface for serial port operations
   - `async fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PortError>`
   - `async fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, PortError>`
   - `fn name(&self) -> &str`
   - `fn config(&self) -> &PortConfiguration`
   - `async fn bytes_available(&mut self) -> Result<u32, PortError>`

2. **`TokioSerialPort`**: Native async implementation using tokio_serial::SerialStream
   - True async I/O without blocking threads
   - Integrates with Tokio runtime
   - Low overhead, high performance

3. **`BlockingSerialPortWrapper`**: Wraps `SyncSerialPort` for async use
   - Uses `tokio::task::spawn_blocking` to run sync operations
   - Prevents blocking the async runtime
   - Useful for migrating sync code to async

## Usage

### Basic Example

```rust
use serial_mcp_agent::port::{TokioSerialPort, PortConfiguration, AsyncSerialPortAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PortConfiguration::default();
    config.baud_rate = 115200;

    let mut port = TokioSerialPort::open("/dev/ttyUSB0", &config)?;

    // Write data
    let data = b"AT\r\n";
    port.write_bytes(data).await?;

    // Read response
    let mut buffer = [0u8; 128];
    let bytes_read = port.read_bytes(&mut buffer).await?;

    println!("Received: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

    Ok(())
}
```

### Using Blocking Wrapper

```rust
use serial_mcp_agent::port::{SyncSerialPort, BlockingSerialPortWrapper, PortConfiguration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PortConfiguration::default();

    // Wrap a sync port for async use
    let sync_port = SyncSerialPort::open("/dev/ttyUSB0", config.clone())?;
    let mut async_port = BlockingSerialPortWrapper::new(sync_port, config);

    // Use the async API
    async_port.write_bytes(b"Hello\r\n").await?;

    Ok(())
}
```

### Trait Object Usage

```rust
use serial_mcp_agent::port::{AsyncSerialPortAdapter, TokioSerialPort, PortConfiguration};

async fn communicate(port: &mut dyn AsyncSerialPortAdapter) -> Result<(), Box<dyn std::error::Error>> {
    port.write_bytes(b"PING\r\n").await?;

    let mut buffer = [0u8; 64];
    let bytes_read = port.read_bytes(&mut buffer).await?;

    println!("Response: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PortConfiguration::default();
    let mut port = TokioSerialPort::open("/dev/ttyUSB0", &config)?;

    communicate(&mut port).await?;

    Ok(())
}
```

## Configuration

### Cargo.toml

Add the `async-serial` feature to enable async port support:

```toml
[dependencies]
serial_mcp_agent = { version = "3.1.0", features = ["async-serial"] }
```

### Feature Flags

- `async-serial`: Enables async serial port support (adds `tokio-serial` and `futures`)
- Automatically included with `auto-negotiation` feature (Phase 4)

## Performance Characteristics

### TokioSerialPort (Native Async)

- **Latency**: ~50-100μs per operation
- **Throughput**: Near line-rate (depends on baud rate)
- **CPU Usage**: Minimal (true async I/O)
- **Thread overhead**: None (uses Tokio reactor)

### BlockingSerialPortWrapper

- **Latency**: ~200-500μs per operation (spawn_blocking overhead)
- **Throughput**: Same as sync (depends on thread pool size)
- **CPU Usage**: Higher (thread pool management)
- **Thread overhead**: Uses Tokio's blocking thread pool

**Recommendation**: Use `TokioSerialPort` for new code. Use `BlockingSerialPortWrapper` only when integrating existing sync code.

## Error Handling

All async operations return `Result<T, PortError>`:

```rust
match port.write_bytes(data).await {
    Ok(bytes_written) => println!("Wrote {} bytes", bytes_written),
    Err(PortError::NotFound(name)) => eprintln!("Port not found: {}", name),
    Err(PortError::Io(e)) => eprintln!("I/O error: {}", e),
    Err(PortError::Timeout(duration)) => eprintln!("Timeout after {:?}", duration),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Platform Support

- **Linux**: Full support via tty/USB serial devices
- **macOS**: Full support via /dev/tty.* and /dev/cu.* devices
- **Windows**: Full support via COM ports

## Thread Safety

- **`AsyncSerialPortAdapter`**: Requires `Send` but not `Sync`
- Serial ports are accessed exclusively (mutable methods only)
- Use `Arc<Mutex<T>>` if shared access is needed (see `BlockingSerialPortWrapper`)

## Testing

Run tests with:

```bash
cargo test --features async-serial --lib port::async_port
```

Run the example:

```bash
cargo run --example async_port_usage --features async-serial
```

## Integration with MCP Server

The async port implementation integrates with the MCP server architecture:

1. **Session management**: Each session can have its own async port
2. **Concurrent operations**: Multiple sessions can use different ports simultaneously
3. **Non-blocking**: Doesn't block other MCP server operations

Future phases will leverage this for:
- **Phase 4**: Auto-negotiation using async timeouts
- **Phase 5**: WebSocket streaming of serial data
- **Phase 6**: Concurrent multi-port operations

## Implementation Details

### Why `Send` but not `Sync`?

Serial ports require exclusive access (mutable methods). The underlying `tokio_serial::SerialStream` contains platform-specific types that are not `Sync`. By requiring only `Send`, we allow ports to be moved between tasks but not shared without synchronization.

### Conversion Functions

The module provides helper functions to convert between our configuration types and tokio-serial types:

- `convert_data_bits()`
- `convert_flow_control()`
- `convert_parity()`
- `convert_stop_bits()`

These ensure type safety and maintain the abstraction boundary.

### BlockingSerialPortWrapper Design

Uses `Arc<Mutex<SyncSerialPort>>` to enable shared ownership and thread-safe access. Each operation uses `spawn_blocking` to run the sync operation in Tokio's blocking thread pool, preventing executor blocking.

## Future Enhancements

- [ ] Async timeout support (Phase 4)
- [ ] Streaming adapters (Phase 5)
- [ ] Connection pooling for multi-port scenarios
- [ ] Metrics and performance monitoring
- [ ] Retry logic with exponential backoff

## See Also

- [`SyncSerialPort`](../sync_port.rs): Synchronous serial port implementation
- [`MockSerialPort`](../mock.rs): Mock implementation for testing
- [`PortConfiguration`](../traits.rs): Port configuration types
- [tokio-serial documentation](https://docs.rs/tokio-serial/)
