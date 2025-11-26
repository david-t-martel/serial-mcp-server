//! End-to-end tests for the Serial MCP Server.
//!
//! These tests run against the actual system without requiring real hardware.
//! They use mock serial ports to simulate device behavior and test the complete
//! workflow from discovery through communication.

pub mod discovery_tests;
pub mod negotiation_tests;
pub mod workflow_tests;

// WebSocket streaming tests (Phase 5)
#[cfg(all(feature = "rest-api", feature = "websocket"))]
pub mod websocket_tests;
