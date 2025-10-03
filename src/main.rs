#[cfg(feature = "rest-api")] use axum::{extract::{Json, State}, routing::{get, post}, Router};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::signal;

// Declare modules for better code organization
mod error;
mod mcp; // new MCP sdk implementation (stdio MCP by default)
mod state;
mod stdio;

#[cfg(feature = "rest-api")] use crate::error::AppResult;
use crate::state::{AppState, PortState};

// Command-line arguments
#[derive(Parser, Debug)]
#[command(
    author = "Gemini",
    version = "3.0.0",
    about = "A robust, production-grade serial port server with a rich MCP interface for LLM agents.",
    long_about = "Provides a highly reliable, cross-platform HTTP and stdio interface for controlling serial ports. Features structured error handling and self-documentation for seamless integration with automated agents."
)]
struct Args {
    /// Start the HTTP server. If not set, runs in stdio mode.
    #[arg(short, long)]
    server: bool,

    /// Set the port for the HTTP server.
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

// --- Main Application Entry Point ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // Initialize the shared application state
    let app_state: AppState = Arc::new(Mutex::new(PortState::default()));

    #[cfg(feature = "rest-api")]
    if args.server {
        // --- HTTP Server Mode ---
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            // Placeholder endpoints (legacy REST removed). Provide minimal surface until redesigned.
            .with_state(app_state);

        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
        println!("Robust Serial MCP Server v3.0");
        println!("Starting server on http://{}", addr);
        println!("Use GET /mcp/help for API documentation.");

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } 
    #[cfg(not(feature = "rest-api"))]
    {
        // Prefer MCP stdio when mcp feature enabled
        #[cfg(feature = "mcp")] {
            println!("Serial MCP Server (official SDK) starting in stdio MCP mode");
            if let Err(e) = mcp::start_mcp_server_stdio(app_state.clone()).await {
                eprintln!("MCP server exited with error: {e}");
            }
        }
        #[cfg(not(feature = "mcp"))] {
            println!("MCP feature disabled, falling back to legacy stdio JSON mode");
            stdio::run_stdio_interface(app_state).await;
        }
    }

    Ok(())
}

// --- HTTP Handler Wrappers ---
// These handlers are thin wrappers that call the core MCP logic.
// The `AppResult` return type automatically converts success and error
// cases into the appropriate HTTP responses via `IntoResponse`.

#[cfg(feature = "rest-api")]
// Placeholder handlers kept for compatibility; currently non-functional beyond health.
// Future: Reintroduce REST by bridging to MCP tool calls or remove feature entirely.
async fn http_list_ports() -> AppResult<axum::Json<serde_json::Value>> { Err(crate::error::AppError::InvalidPayload("REST API deprecated".into())) }

// --- Graceful Shutdown Handler ---
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\nSignal received, starting graceful shutdown...");
}

