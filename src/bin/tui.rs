//! TUI binary entry point for rust-comm.
//!
//! This binary provides an interactive terminal interface for serial port
//! communication with features like auto-complete, hex view, and theming.
//!
//! # Usage
//!
//! ```bash
//! # Run the TUI
//! cargo run --bin serial-tui --features tui
//!
//! # Or if installed
//! serial-tui
//! ```

use serial_mcp_agent::config::ConfigLoader;
use serial_mcp_agent::tui::App;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Load configuration
    let config = match ConfigLoader::load() {
        Ok(loader) => loader.into_config(),
        Err(e) => {
            eprintln!("Warning: Failed to load config, using defaults: {}", e);
            ConfigLoader::with_defaults().into_config()
        }
    };

    // Initialize logging to file if configured (don't log to stderr in TUI mode)
    if let Some(ref log_file) = config.logging.file {
        // TODO: Set up file logging
        let _ = log_file;
    }

    // Create and run the application
    let mut app = App::new()?;
    app.config = config;

    app.run().await
}
