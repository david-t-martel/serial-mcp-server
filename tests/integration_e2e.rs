//! E2E integration test suite.
//!
//! This test suite covers end-to-end functionality without requiring hardware.
//! All tests use mock serial ports to simulate device behavior.

#[path = "common/mod.rs"]
mod common;

#[path = "e2e/mod.rs"]
mod e2e;

// Re-export test modules to make them discoverable
pub use e2e::*;
