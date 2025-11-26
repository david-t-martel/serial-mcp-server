//! Legacy stdio interface (deprecated).
//!
//! This module provides a deprecated JSON-based stdio interface that predates the MCP implementation.
//! It is only compiled when the `legacy-stdio` feature is enabled AND the `mcp` feature is disabled.
//!
//! **DEPRECATED**: Use MCP interface instead. This module will be removed in a future release.

#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
use crate::error::AppError;
#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
use crate::state::{AppState, PortConfig};
#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
use serde_json::{json, Value};
#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
use std::io::{self, Write};

/// Runs the application in stdio mode, processing JSON commands from stdin.
#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
pub async fn run_stdio_interface(state: AppState) {
    println!("Legacy stdio mode enabled (MCP feature disabled). Send JSON commands, e.g. {\"command\": \"help\"}.");
    let mut buffer = String::new();
    let stdin = io::stdin();
    loop {
        buffer.clear();
        print!("> ");
        let _ = io::stdout().flush(); // ignore transient flush errors

        if stdin.read_line(&mut buffer).is_err() || buffer.trim().is_empty() {
            // Exit on EOF (Ctrl+D) or read error.
            break;
        }

        let response = match serde_json::from_str::<Value>(&buffer) {
            Ok(json_input) => process_stdio_command(json_input, state.clone()).await,
            Err(e) => {
                // Directly create a JSON response for a parsing error.
                let app_err: AppError = e.into();
                json!({
                    "status": "error",
                    "error": {
                        "type": "DeserializationError",
                        "message": app_err.to_string()
                    }
                })
            }
        };

        // Pretty-print the JSON response for better readability.
        if let Ok(pretty_response) = serde_json::to_string_pretty(&response) {
            println!("{}", pretty_response);
        } else {
            // Fallback for serialization errors.
            println!("{}", response);
        }
    }
}

/// Processes a single command received via the stdio interface.
#[cfg(all(feature = "legacy-stdio", not(feature = "mcp")))]
async fn process_stdio_command(json_input: Value, state: AppState) -> Value {
    let command = json_input["command"].as_str().unwrap_or("").to_lowercase();
    let params = json_input.get("params").cloned().unwrap_or(Value::Null);

    // This closure simplifies converting a Result into a final JSON Value.
    let to_json_response = |result: Result<Json<Value>, AppError>| -> Value {
        match result {
            Ok(Json(val)) => val,
            Err(e) => {
                // Serialize the AppError into our standard error JSON format.
                let (status_code, error_type, message) = match e {
                    AppError::PortNotOpen => (409, "PortNotOpen", e.to_string()),
                    AppError::PortAlreadyOpen => (409, "PortAlreadyOpen", e.to_string()),
                    AppError::InvalidPayload(_) => (400, "InvalidPayload", e.to_string()),
                    _ => (500, "InternalError", e.to_string()),
                };
                json!({
                    "status": "error",
                    "error": { "type": error_type, "message": message, "code": status_code }
                })
            }
        }
    };

    let result = match command.as_str() {
        "help" => {
            let help = json!({
                "status": "ok",
                "commands": ["help", "exit"],
                "note": "All operational functionality has moved to the MCP tools interface; rebuild with feature 'mcp' (default) and use tools/call."
            });
            Ok(Json(help))
        }
        "list_ports" | "status" | "open" | "write" | "read" | "close" | "examples" => {
            Err(AppError::InvalidPayload("Operational commands removed; enable 'mcp' feature and use MCP tools (method tools/call).".into()))
        }
        "exit" => {
            println!("Exiting stdio mode.");
            std::process::exit(0);
        }
        _ => Err(AppError::InvalidPayload(format!("Unknown command: '{}'", command))),
    };

    to_json_response(result)
}
