use clap::Parser;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::signal;

// All modules are now in the library - import what we need
#[cfg(feature = "rest-api")]
use serial_mcp_agent::AppResult;
use serial_mcp_agent::{session, AppState, PortState};

#[cfg(feature = "mcp")]
use serial_mcp_agent::mcp;

#[cfg(feature = "rest-api")]
use serial_mcp_agent::rest_api;


// Command-line arguments
#[derive(Parser, Debug)]
#[command(
    author = "Gemini",
    version = "3.1.0",
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
    // Initialize tracing subscriber once. Keep stdout clean for MCP framed protocol; send logs to stderr.
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init();
    // Initialize the shared application state
    let app_state: AppState = Arc::new(Mutex::new(PortState::default()));
    // Initialize session store. Default to on-disk file (sessions.db). Allow override via env SESSION_DB_URL.
    // If the on-disk database cannot be opened (common in CI / read-only or sandboxed environments),
    // fall back to an in-memory shared SQLite instance so the server can still start and tests pass.
    let db_url =
        std::env::var("SESSION_DB_URL").unwrap_or_else(|_| "sqlite://sessions.db".to_string());
    let session_store = match session::SessionStore::new(&db_url).await {
        Ok(store) => store,
        Err(e) => {
            tracing::warn!(error = %e, db_url, "Failed to open session database; falling back to in-memory");
            session::SessionStore::new("sqlite::memory:?cache=shared").await?
        }
    };

    // If the --server flag is provided (and REST feature enabled), launch HTTP server; otherwise always fall back to
    // stdio MCP (preferred) or legacy stdio if MCP feature is disabled. This keeps a consistent developer UX and
    // preserves the ability to run headless via stdio even when "rest-api" feature remains enabled.
    #[cfg(feature = "rest-api")]
    {
        if args.server {
            // --- HTTP Server Mode ---
            let rest_ctx = rest_api::RestContext {
                state: app_state.clone(),
                sessions: std::sync::Arc::new(session_store.clone()),
            };
            let app = rest_api::build_router(rest_ctx);

            let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
            tracing::info!(version = env!("CARGO_PKG_VERSION"), %addr, "Starting HTTP server (REST + MCP stdio)");

            let listener = TcpListener::bind(addr).await?;
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        } else {
            // --- STDIO Mode (MCP preferred) ---
            #[cfg(feature = "mcp")]
            {
                tracing::info!("Serial MCP Server starting (stdio MCP mode)");
                if let Err(e) = mcp::start_mcp_server_stdio(app_state.clone(), session_store).await
                {
                    tracing::error!(error = %e, "MCP server exited with error");
                }
            }
            #[cfg(not(feature = "mcp"))]
            {
                tracing::warn!("MCP feature disabled, falling back to legacy stdio JSON mode");
                stdio::run_stdio_interface(app_state.clone()).await;
            }
        }
    }
    #[cfg(not(feature = "rest-api"))]
    {
        #[cfg(feature = "mcp")]
        {
            tracing::info!("Serial MCP Server starting (stdio MCP mode)");
            if let Err(e) = mcp::start_mcp_server_stdio(app_state.clone(), session_store).await {
                tracing::error!(error = %e, "MCP server exited with error");
            }
        }
        #[cfg(not(feature = "mcp"))]
        {
            tracing::warn!("MCP feature disabled, falling back to legacy stdio JSON mode");
            stdio::run_stdio_interface(app_state.clone()).await;
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
async fn http_list_ports() -> AppResult<axum::Json<serde_json::Value>> {
    Err(serial_mcp_agent::AppError::InvalidPayload(
        "REST API deprecated".into(),
    ))
}

// --- Graceful Shutdown Handler ---
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            tracing::warn!(error = %e, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        } else {
            tracing::warn!("failed to install terminate signal handler");
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown...");
}
