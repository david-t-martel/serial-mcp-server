# Async Serial Port Quick Start Guide

## Enable the Feature

Add to `Cargo.toml`:

```toml
[dependencies]
serial_mcp_agent = { version = "3.1.0", features = ["async-serial"] }
```

## Basic Usage

```rust
use serial_mcp_agent::port::{TokioSerialPort, PortConfiguration, AsyncSerialPortAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the port
    let mut config = PortConfiguration::default();
    config.baud_rate = 115200;

    // Open the port
    let mut port = TokioSerialPort::open("/dev/ttyUSB0", &config)?;

    // Write data
    port.write_bytes(b"AT\r\n").await?;

    // Read response
    let mut buffer = [0u8; 128];
    let bytes_read = port.read_bytes(&mut buffer).await?;

    println!("Received: {}", String::from_utf8_lossy(&buffer[..bytes_read]));

    Ok(())
}
```

## Choosing the Right Implementation

### Use `TokioSerialPort` when:
- ✓ Writing new async code
- ✓ Performance is critical (50-100μs latency)
- ✓ You have control over the async runtime

### Use `BlockingSerialPortWrapper` when:
- ✓ Integrating existing sync code
- ✓ Need to wrap `SyncSerialPort` instances
- ✓ API compatibility is more important than performance

## Common Patterns

### Request-Response

```rust
async fn send_command(port: &mut impl AsyncSerialPortAdapter, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Send command
    let cmd_bytes = format!("{}\r\n", cmd);
    port.write_bytes(cmd_bytes.as_bytes()).await?;

    // Read response
    let mut buffer = [0u8; 256];
    let bytes_read = port.read_bytes(&mut buffer).await?;

    Ok(String::from_utf8_lossy(&buffer[..bytes_read]).to_string())
}
```

### With Timeout

```rust
use tokio::time::{timeout, Duration};

async fn read_with_timeout(port: &mut impl AsyncSerialPortAdapter) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 128];

    let bytes_read = timeout(
        Duration::from_secs(5),
        port.read_bytes(&mut buffer)
    ).await??;

    Ok(buffer[..bytes_read].to_vec())
}
```

### Line-Based Protocol

```rust
use tokio::io::{AsyncBufReadExt, BufReader};

async fn read_line(port: &mut TokioSerialPort) -> Result<String, Box<dyn std::error::Error>> {
    let stream = port.as_raw_mut();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(line.trim().to_string())
}
```

## Error Handling

```rust
use serial_mcp_agent::port::PortError;

match port.write_bytes(data).await {
    Ok(n) => println!("Wrote {} bytes", n),
    Err(PortError::NotFound(name)) => eprintln!("Port not found: {}", name),
    Err(PortError::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
        eprintln!("Operation timed out");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Platform-Specific Port Names

```rust
let port_name = if cfg!(windows) {
    "COM3"
} else if cfg!(target_os = "macos") {
    "/dev/tty.usbserial"
} else {
    "/dev/ttyUSB0"
};
```

## Configuration Examples

### High-Speed Serial

```rust
let mut config = PortConfiguration::default();
config.baud_rate = 921600;
config.flow_control = FlowControl::Hardware;
```

### 7-Bit Data with Even Parity

```rust
use serial_mcp_agent::port::{DataBits, Parity};

let mut config = PortConfiguration::default();
config.data_bits = DataBits::Seven;
config.parity = Parity::Even;
```

### Custom Timeout

```rust
use std::time::Duration;

let mut config = PortConfiguration::default();
config.timeout = Duration::from_millis(100);
```

## Testing

Use `MockSerialPort` for testing (will be async-compatible in future):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_communication() {
        // For now, test with real ports or mock in blocking mode
        // Future: async mock support
    }
}
```

## Performance Tips

1. **Batch writes** when possible to reduce syscall overhead
2. **Pre-allocate buffers** to avoid allocations in hot paths
3. **Use `TokioSerialPort`** for best performance
4. **Avoid `BlockingSerialPortWrapper`** in high-throughput scenarios
5. **Monitor bytes_available()** to optimize read buffer sizes

## Troubleshooting

### Port not found

```
Error: Port not found: /dev/ttyUSB0
```

**Solution**: Check port exists with `ls /dev/tty*` (Linux/Mac) or Device Manager (Windows)

### Permission denied

```
Error: I/O error: Permission denied (os error 13)
```

**Solution**: Add user to `dialout` group (Linux): `sudo usermod -a -G dialout $USER`

### Timeout errors

```
Error: Operation timed out after 1s
```

**Solution**: Increase timeout in configuration or use `tokio::time::timeout()`

## Full Example

See `examples/async_port_usage.rs` for a complete working example:

```bash
cargo run --example async_port_usage --features async-serial
```

## API Documentation

Full API docs:
```bash
cargo doc --features async-serial --open
```

## See Also

- [ASYNC_PORT_README.md](./ASYNC_PORT_README.md) - Comprehensive guide
- [PHASE_3_IMPLEMENTATION.md](../../PHASE_3_IMPLEMENTATION.md) - Implementation details
- [tokio-serial docs](https://docs.rs/tokio-serial/) - Underlying library
