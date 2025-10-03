use crate::error::{AppError, AppResult};
use crate::state::{AppState, PortConfig, PortState};
use axum::Json;
use serde_json::{json, Value};
use std::io::Write;
use std::time::Duration;

/// Lists all available serial ports on the system.
pub async fn list_available_ports() -> AppResult<Json<Value>> {
    let ports = serialport::available_ports()?;
    let port_info: Vec<_> = ports
        .into_iter()
        .map(|p| {
            json!({
                "port_name": p.port_name,
                // Add more details if needed in the future
            })
        })
        .collect();
    Ok(Json(json!({"status": "success", "ports": port_info})))
}

/// Gets the current status of the managed serial port.
pub async fn get_port_status(state: AppState) -> AppResult<Json<Value>> {
    let port_state = state.lock().unwrap();
    let response_val = serde_json::to_value(&*port_state)?;
    Ok(Json(response_val))
}

/// Opens and configures the serial port.
pub async fn open_port(state: AppState, config: PortConfig) -> AppResult<Json<Value>> {
    let mut port_state = state.lock().unwrap();
    if let PortState::Open { .. } = *port_state {
        return Err(AppError::PortAlreadyOpen);
    }

    let port = serialport::new(&config.port_name, config.baud_rate)
        .timeout(Duration::from_millis(config.timeout_ms))
        .open()?;

    *port_state = PortState::Open {
        port,
        config,
    };

    let response_val = serde_json::to_value(&*port_state)?;
    Ok(Json(json!({
        "status": "success",
        "message": "Port opened successfully.",
        "current_state": response_val
    })))
}

/// Writes data to the serial port.
pub async fn write_to_port(state: AppState, payload: Value) -> AppResult<Json<Value>> {
    let mut port_state = state.lock().unwrap();
    if let PortState::Open { port, .. } = &mut *port_state {
        // Validate payload structure
        let data = payload["data"].as_str().ok_or_else(|| {
            AppError::InvalidPayload("Missing 'data' field or it's not a string.".to_string())
        })?;

        let bytes_written = port.write(data.as_bytes())?;
        Ok(Json(json!({"status": "success", "bytes_written": bytes_written})))
    } else {
        Err(AppError::PortNotOpen)
    }
}

/// Reads data from the serial port.
pub async fn read_from_port(state: AppState) -> AppResult<Json<Value>> {
    let mut port_state = state.lock().unwrap();
    if let PortState::Open { port, .. } = &mut *port_state {
        let mut buffer: Vec<u8> = vec![0; 1024];
        let bytes_read = match port.read(buffer.as_mut_slice()) {
            Ok(bytes) => bytes,
            // A timeout is not a fatal error, it just means no data was read.
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
            Err(e) => return Err(AppError::IoError(e)),
        };

        let data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
        Ok(Json(json!({"status": "success", "data": data, "bytes_read": bytes_read})))
    } else {
        Err(AppError::PortNotOpen)
    }
}

/// Closes the serial port.
pub async fn close_port(state: AppState) -> AppResult<Json<Value>> {
    let mut port_state = state.lock().unwrap();
    if let PortState::Closed = *port_state {
        // This is idempotent, not a hard error.
        return Ok(Json(json!({
            "status": "success",
            "message": "Port was already closed.",
            "current_state": serde_json::to_value(&*port_state)?
        })));
    }

    *port_state = PortState::Closed;
    Ok(Json(json!({
        "status": "success",
        "message": "Port closed successfully.",
        "current_state": serde_json::to_value(&*port_state)?
    })))
}

/// Provides API documentation.
pub async fn get_mcp_help() -> AppResult<Json<Value>> {
    Ok(Json(json!({
        "description": "MCP Interface for Rust Serial Port Server (v3.0 - Robust).",
        "endpoints": {
            "GET /ports/list": "Lists available serial ports.",
            "GET /port/status": "Gets the current status and config of the managed port.",
            "POST /port/open": "Opens the port. Body: {\"port_name\": \"...\", \"baud_rate\": ...}",
            "POST /port/write": "Writes to the port. Body: {\"data\": \"...\"}",
            "GET /port/read": "Reads from the port.",
            "POST /port/close": "Closes the port.",
            "GET /mcp/help": "Shows this help message.",
            "GET /mcp/examples": "Provides example API usage.",
        }
    })))
}

/// Provides example API usage.
pub async fn get_mcp_examples() -> AppResult<Json<Value>> {
    Ok(Json(json!({
        "note": "Serial port names are platform-specific. Use '/dev/tty...' on Linux/macOS and 'COMx' on Windows.",
        "examples": [
            { "action": "List ports", "curl": "curl http://localhost:3000/ports/list" },
            { "action": "Open a port (Linux/macOS)", "curl": "curl -X POST -H \"Content-Type: application/json\" -d '{\"port_name\": \"/dev/ttyUSB0\", \"baud_rate\": 9600}' http://localhost:3000/port/open" },
            { "action": "Open a port (Windows)", "curl": "curl -X POST -H \"Content-Type: application/json\" -d '{\"port_name\": \"COM3\", \"baud_rate\": 9600}' http://localhost:3000/port/open" },
            { "action": "Check status", "curl": "curl http://localhost:3000/port/status" },
            { "action": "Write to port", "curl": "curl -X POST -H \"Content-Type: application/json\" -d '{\"data\": \"ATZ\\r\\n\"}' http://localhost:3000/port/write" },
            { "action": "Read from port", "curl": "curl http://localhost:3000/port/read" },
            { "action": "Close port", "curl": "curl -X POST http://localhost:3000/port/close" }
        ]
    })))
}
