//! Hardware-specific tests requiring real serial devices.
//!
//! These tests are ignored by default and require actual hardware to run.
//! They should be run manually with the `--ignored` flag and appropriate
//! environment variables set.

pub mod enhanced_real_port_tests;
pub mod port_discovery_tests;
pub mod real_port_tests;
pub mod utils;

#[cfg(feature = "auto-negotiation")]
pub mod auto_negotiation_hardware;
