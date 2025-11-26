//! Hardware-specific tests requiring real serial devices.
//!
//! These tests are ignored by default and require actual hardware to run.
//! They should be run manually with the `--ignored` flag and appropriate
//! environment variables set.

pub mod real_port_tests;
