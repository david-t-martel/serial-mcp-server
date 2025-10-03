use crate::error::AppError;
use axum::Json; // for consistent Json<Value> used in closure
use crate::mcp;
use crate::state::{AppState, PortConfig};
use serde_json::{json, Value};
use std::io::{self, Write};

/// Runs the application in stdio mode, processing JSON commands from stdin.
pub async fn run_stdio_interface(state: AppState) {
    println!("Stdio mode enabled. Send JSON commands. (e.g., {{\"command\": \"help\"}})");
    let mut buffer = String::new();
    let stdin = io::stdin();
    loop {
        buffer.clear();
        print!("> ");
        // Ensure the prompt is displayed immediately.
        io::stdout().flush().unwrap_or(());

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
        "list_ports" => mcp::list_available_ports().await,
        "status" => mcp::get_port_status(state).await,
        "open" => {
            let config: Result<PortConfig, _> = serde_json::from_value(params);
            match config {
                Ok(c) => mcp::open_port(state, c).await,
                Err(e) => Err(e.into()),
            }
        }
        "write" => mcp::write_to_port(state, params).await,
        "read" => mcp::read_from_port(state).await,
        "close" => mcp::close_port(state).await,
        "help" => mcp::get_mcp_help().await,
        "examples" => mcp::get_mcp_examples().await,
        "exit" => {
            println!("Exiting stdio mode.");
            std::process::exit(0);
        }
        _ => Err(AppError::InvalidPayload(format!("Unknown command: '{}'", command))),
    };

    to_json_response(result)
}
