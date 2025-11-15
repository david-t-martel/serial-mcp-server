// Simple installation utility to initialize the session database schema.
// Usage: cargo run --bin init_db -- <optional-path-or-url>
// If no path supplied, defaults to sqlite://sessions.db

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = env::args().nth(1).unwrap_or_else(|| "sqlite://sessions.db".to_string());
    serial_mcp_agent::session::SessionStore::new(&arg).await?; // creating store runs migrations
    println!("Database initialized at {arg}");
    Ok(())
}
