//! Terminal User Interface (TUI) module for rust-comm.
//!
//! This module provides a ratatui-based terminal interface for interactive
//! serial port communication with features like auto-complete, hex view,
//! and theming support.
//!
//! # Features
//!
//! - Interactive terminal with command history
//! - Auto-complete for commands and port names
//! - Hex/ASCII data view toggle
//! - Multiple theme support (dark, light, solarized, dracula, nord)
//! - Config editor for live configuration changes
//! - Scripting support via rhai (with `scripting` feature)
//!
//! # Example
//!
//! ```rust,ignore
//! use serial_mcp_agent::tui::App;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = App::new()?;
//!     app.run().await
//! }
//! ```

mod app;
mod event;
mod theme;
mod ui;

pub mod widgets;

// TODO: Implement scripting module for rhai integration
// #[cfg(feature = "scripting")]
// mod scripting;

pub use app::{App, AppState, FocusArea, Mode};
pub use event::{Event, EventHandler};
pub use theme::{Theme, THEMES};
pub use ui::render;
