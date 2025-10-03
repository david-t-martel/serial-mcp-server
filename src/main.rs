use axum::{
    extract::{Json, State},
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::signal;

// Declare modules for better code organization
mod error;
mod mcp;
mod state;
mod stdio;

use crate::error::AppResult;
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

    if args.server {
        // --- HTTP Server Mode ---
        let app = Router::new()
            .route("/ports/list", get(http_list_ports))
            .route("/port/status", get(http_get_status))
            .route("/port/open", post(http_open_port))
            .route("/port/write", post(http_write_to_port))
            .route("/port/read", get(http_read_from_port))
            .route("/port/close", post(http_close_port))
            .route("/mcp/help", get(mcp::get_mcp_help))
            .route("/mcp/examples", get(mcp::get_mcp_examples))
            .with_state(app_state);

        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
        println!("Robust Serial MCP Server v3.0");
        println!("Starting server on http://{}", addr);
        println!("Use GET /mcp/help for API documentation.");

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else {
        // --- Stdio Mode ---
        println!("Robust Serial MCP Server v3.0");
        stdio::run_stdio_interface(app_state).await;
    }

    Ok(())
}

// --- HTTP Handler Wrappers ---
// These handlers are thin wrappers that call the core MCP logic.
// The `AppResult` return type automatically converts success and error
// cases into the appropriate HTTP responses via `IntoResponse`.

async fn http_list_ports() -> AppResult<Json<serde_json::Value>> {
    mcp::list_available_ports().await
}

async fn http_get_status(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    mcp::get_port_status(state).await
}

async fn http_open_port(
    State(state): State<AppState>,
    Json(config): Json<state::PortConfig>,
) -> AppResult<Json<serde_json::Value>> {
    mcp::open_port(state, config).await
}

async fn http_write_to_port(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    mcp::write_to_port(state, payload).await
}

async fn http_read_from_port(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    mcp::read_from_port(state).await
}

async fn http_close_port(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    mcp::close_port(state).await
}

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

