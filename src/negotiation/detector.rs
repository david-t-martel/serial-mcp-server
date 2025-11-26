//! Auto-negotiation orchestrator.
//!
//! The `AutoNegotiator` coordinates multiple negotiation strategies to
//! automatically detect the correct serial port parameters.

use super::strategies::{
    EchoProbeStrategy, ManufacturerStrategy, NegotiatedParams, NegotiationError, NegotiationHints,
    NegotiationStrategy, StandardBaudsStrategy,
};
use tracing::{debug, info, warn};

/// Main auto-negotiation orchestrator.
///
/// This type manages a collection of negotiation strategies and executes
/// them in priority order to find the correct port parameters.
pub struct AutoNegotiator {
    strategies: Vec<Box<dyn NegotiationStrategy>>,
}

impl AutoNegotiator {
    /// Create a new auto-negotiator with default strategies.
    ///
    /// Default strategies (in priority order):
    /// 1. ManufacturerStrategy (priority 80) - uses VID/PID database
    /// 2. EchoProbeStrategy (priority 60) - sends AT commands
    /// 3. StandardBaudsStrategy (priority 30) - brute force testing
    pub fn new() -> Self {
        let mut strategies: Vec<Box<dyn NegotiationStrategy>> = vec![
            Box::new(ManufacturerStrategy::new()),
            Box::new(EchoProbeStrategy::new()),
            Box::new(StandardBaudsStrategy::new()),
        ];

        // Sort by priority (highest first)
        strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));

        Self { strategies }
    }

    /// Create a negotiator with custom strategies.
    pub fn with_strategies(strategies: Vec<Box<dyn NegotiationStrategy>>) -> Self {
        let mut strategies = strategies;
        strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        Self { strategies }
    }

    /// Add a strategy to the negotiator.
    pub fn add_strategy(mut self, strategy: Box<dyn NegotiationStrategy>) -> Self {
        self.strategies.push(strategy);
        self.strategies
            .sort_by_key(|s| std::cmp::Reverse(s.priority()));
        self
    }

    /// Get all registered strategies.
    pub fn strategies(&self) -> &[Box<dyn NegotiationStrategy>] {
        &self.strategies
    }

    /// Detect port parameters using available strategies.
    ///
    /// This method tries each strategy in priority order until one succeeds.
    /// Strategies with higher priority are tried first.
    ///
    /// # Arguments
    /// * `port_name` - The system path to the serial port
    /// * `hints` - Optional hints to guide negotiation
    ///
    /// # Returns
    /// Successfully negotiated parameters, or an error if all strategies fail.
    pub async fn detect(
        &self,
        port_name: &str,
        hints: Option<NegotiationHints>,
    ) -> Result<NegotiatedParams, NegotiationError> {
        let hints = hints.unwrap_or_default();

        info!(
            "Starting auto-negotiation for port {} with {} strategies",
            port_name,
            self.strategies.len()
        );

        // Try each strategy in priority order
        for strategy in &self.strategies {
            debug!(
                "Trying strategy '{}' (priority {})",
                strategy.name(),
                strategy.priority()
            );

            match strategy.negotiate(port_name, &hints).await {
                Ok(params) => {
                    info!(
                        "Strategy '{}' succeeded: {} baud (confidence: {})",
                        params.strategy_used, params.baud_rate, params.confidence
                    );
                    return Ok(params);
                }
                Err(e) => {
                    debug!("Strategy '{}' failed: {}", strategy.name(), e);
                    continue;
                }
            }
        }

        warn!(
            "All {} strategies failed for port {}",
            self.strategies.len(),
            port_name
        );
        Err(NegotiationError::AllStrategiesFailed)
    }

    /// Detect parameters with specific strategy preference.
    ///
    /// Tries the preferred strategy first, then falls back to others.
    pub async fn detect_with_preference(
        &self,
        port_name: &str,
        hints: Option<NegotiationHints>,
        preferred_strategy: &str,
    ) -> Result<NegotiatedParams, NegotiationError> {
        let hints = hints.unwrap_or_default();

        info!(
            "Auto-negotiation for {} with preference for '{}'",
            port_name, preferred_strategy
        );

        // Try preferred strategy first
        if let Some(strategy) = self
            .strategies
            .iter()
            .find(|s| s.name() == preferred_strategy)
        {
            debug!("Trying preferred strategy '{}'", preferred_strategy);
            if let Ok(params) = strategy.negotiate(port_name, &hints).await {
                info!(
                    "Preferred strategy '{}' succeeded: {} baud",
                    preferred_strategy, params.baud_rate
                );
                return Ok(params);
            }
            debug!(
                "Preferred strategy '{}' failed, trying others",
                preferred_strategy
            );
        } else {
            warn!("Preferred strategy '{}' not found", preferred_strategy);
        }

        // Fall back to normal priority order
        self.detect(port_name, Some(hints)).await
    }

    /// Get a manufacturer profile by VID.
    ///
    /// This is a convenience method for accessing the manufacturer database.
    pub fn get_manufacturer_profile(
        vid: u16,
    ) -> Option<&'static crate::negotiation::strategies::manufacturer::ManufacturerProfile> {
        ManufacturerStrategy::get_profile(vid)
    }

    /// Get all known manufacturer profiles.
    pub fn all_manufacturer_profiles(
    ) -> &'static [crate::negotiation::strategies::manufacturer::ManufacturerProfile] {
        ManufacturerStrategy::all_profiles()
    }

    /// Detect parameters for multiple ports in parallel.
    ///
    /// This is useful when you need to detect parameters for several ports
    /// and want to do it concurrently for speed.
    #[cfg(feature = "async-serial")]
    pub async fn detect_multiple(
        &self,
        ports: Vec<(String, Option<NegotiationHints>)>,
    ) -> Vec<(String, Result<NegotiatedParams, NegotiationError>)> {
        use futures::future::join_all;

        let futures = ports.into_iter().map(|(port_name, hints)| {
            let port_name_clone = port_name.clone();
            async move {
                let result = self.detect(&port_name, hints).await;
                (port_name_clone, result)
            }
        });

        join_all(futures).await
    }
}

impl Default for AutoNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_negotiator() {
        let negotiator = AutoNegotiator::new();
        assert_eq!(negotiator.strategies().len(), 3);
    }

    #[test]
    fn test_strategies_sorted_by_priority() {
        let negotiator = AutoNegotiator::new();
        let strategies = negotiator.strategies();

        // Should be sorted by priority (highest first)
        for i in 1..strategies.len() {
            assert!(strategies[i - 1].priority() >= strategies[i].priority());
        }

        // Manufacturer should be first (priority 80)
        assert_eq!(strategies[0].name(), "manufacturer");

        // Echo probe should be second (priority 60)
        assert_eq!(strategies[1].name(), "echo_probe");

        // Standard bauds should be last (priority 30)
        assert_eq!(strategies[2].name(), "standard_bauds");
    }

    #[test]
    fn test_add_strategy() {
        let negotiator = AutoNegotiator::new().add_strategy(Box::new(EchoProbeStrategy::new()));
        assert_eq!(negotiator.strategies().len(), 4);
    }

    #[test]
    fn test_get_manufacturer_profile() {
        let profile = AutoNegotiator::get_manufacturer_profile(0x0403);
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().name, "FTDI");
    }

    #[test]
    fn test_all_manufacturer_profiles() {
        let profiles = AutoNegotiator::all_manufacturer_profiles();
        assert!(!profiles.is_empty());
        assert!(profiles.iter().any(|p| p.name == "FTDI"));
        assert!(profiles.iter().any(|p| p.name == "Arduino"));
    }

    #[test]
    fn test_with_strategies() {
        let strategies: Vec<Box<dyn NegotiationStrategy>> =
            vec![Box::new(StandardBaudsStrategy::new())];
        let negotiator = AutoNegotiator::with_strategies(strategies);
        assert_eq!(negotiator.strategies().len(), 1);
        assert_eq!(negotiator.strategies()[0].name(), "standard_bauds");
    }
}
