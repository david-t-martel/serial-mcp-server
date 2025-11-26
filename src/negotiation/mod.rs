//! Port auto-negotiation module.
//!
//! This module provides automatic port detection and baud rate negotiation
//! capabilities for serial devices. It implements multiple strategies for
//! detecting the correct communication parameters.

pub mod detector;
pub mod strategies;

// Re-export main types
pub use detector::AutoNegotiator;
pub use strategies::{NegotiatedParams, NegotiationError, NegotiationHints, NegotiationStrategy};
