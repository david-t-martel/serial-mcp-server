//! Hardware integration test suite.
//!
//! These tests require actual serial hardware and are ignored by default.
//! Run with: cargo test --all-features -- --ignored

#[path = "common/mod.rs"]
mod common;

#[path = "hardware/mod.rs"]
mod hardware;

// Re-export test modules to make them discoverable
pub use hardware::*;
