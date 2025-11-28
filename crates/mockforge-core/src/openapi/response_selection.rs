//! Response selection modes for multiple responses/examples
//!
//! This module provides functionality for selecting responses when multiple
//! options are available (scenarios, examples, or status codes).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Mode for selecting responses when multiple options are available
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseSelectionMode {
    /// Use first available (default behavior)
    First,
    /// Select by scenario name (requires X-Mockforge-Scenario header)
    Scenario,
    /// Round-robin sequential selection
    Sequential,
    /// Random selection
    Random,
    /// Weighted random selection (weights defined per option)
    WeightedRandom,
}

impl Default for ResponseSelectionMode {
    fn default() -> Self {
        Self::First
    }
}

impl ResponseSelectionMode {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "first" => Some(Self::First),
            "scenario" => Some(Self::Scenario),
            "sequential" | "round-robin" | "round_robin" => Some(Self::Sequential),
            "random" => Some(Self::Random),
            "weighted_random" | "weighted-random" | "weighted" => Some(Self::WeightedRandom),
            _ => None,
        }
    }
}

/// Response selector with state for sequential mode
#[derive(Debug)]
pub struct ResponseSelector {
    /// Selection mode
    mode: ResponseSelectionMode,
    /// Counter for sequential mode (per route)
    sequential_counter: Arc<AtomicUsize>,
    /// Weights for weighted random mode (optional)
    weights: Option<HashMap<String, f64>>,
}

impl ResponseSelector {
    /// Create a new response selector
    pub fn new(mode: ResponseSelectionMode) -> Self {
        Self {
            mode,
            sequential_counter: Arc::new(AtomicUsize::new(0)),
            weights: None,
        }
    }

    /// Create a new response selector with weights for weighted random
    pub fn with_weights(mut self, weights: HashMap<String, f64>) -> Self {
        self.weights = Some(weights);
        self
    }

    /// Select an option from a list of available options
    ///
    /// # Arguments
    /// * `options` - List of option identifiers (e.g., scenario names, example names)
    ///
    /// # Returns
    /// Index into the options list for the selected option
    pub fn select(&self, options: &[String]) -> usize {
        if options.is_empty() {
            return 0;
        }

        match self.mode {
            ResponseSelectionMode::First => 0,
            ResponseSelectionMode::Scenario => {
                // Scenario mode requires explicit scenario selection
                // Default to first if no scenario specified
                0
            }
            ResponseSelectionMode::Sequential => {
                // Round-robin: increment counter and wrap around
                let current = self.sequential_counter.fetch_add(1, Ordering::Relaxed);
                current % options.len()
            }
            ResponseSelectionMode::Random => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(0..options.len())
            }
            ResponseSelectionMode::WeightedRandom => self.select_weighted_random(options),
        }
    }

    /// Select using weighted random distribution
    fn select_weighted_random(&self, options: &[String]) -> usize {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // If weights are provided, use them
        if let Some(ref weights) = self.weights {
            let total_weight: f64 =
                options.iter().map(|opt| weights.get(opt).copied().unwrap_or(1.0)).sum();

            if total_weight > 0.0 {
                let random = rng.gen::<f64>() * total_weight;
                let mut cumulative = 0.0;

                for (idx, opt) in options.iter().enumerate() {
                    cumulative += weights.get(opt).copied().unwrap_or(1.0);
                    if random <= cumulative {
                        return idx;
                    }
                }
            }
        }

        // Fall back to uniform random if no weights or invalid weights
        rng.gen_range(0..options.len())
    }

    /// Reset the sequential counter (useful for testing)
    pub fn reset_sequential(&self) {
        self.sequential_counter.store(0, Ordering::Relaxed);
    }

    /// Get the current sequential counter value
    pub fn get_sequential_index(&self) -> usize {
        self.sequential_counter.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_mode() {
        let selector = ResponseSelector::new(ResponseSelectionMode::First);
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        assert_eq!(selector.select(&options), 0);
        assert_eq!(selector.select(&options), 0); // Always returns first
    }

    #[test]
    fn test_sequential_mode() {
        let selector = ResponseSelector::new(ResponseSelectionMode::Sequential);
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        assert_eq!(selector.select(&options), 0);
        assert_eq!(selector.select(&options), 1);
        assert_eq!(selector.select(&options), 2);
        assert_eq!(selector.select(&options), 0); // Wraps around
        assert_eq!(selector.select(&options), 1);
    }

    #[test]
    fn test_random_mode() {
        let selector = ResponseSelector::new(ResponseSelectionMode::Random);
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        // Random selection should return valid indices
        for _ in 0..100 {
            let idx = selector.select(&options);
            assert!(idx < options.len());
        }
    }

    #[test]
    fn test_weighted_random_mode() {
        let mut weights = HashMap::new();
        weights.insert("a".to_string(), 0.5);
        weights.insert("b".to_string(), 0.3);
        weights.insert("c".to_string(), 0.2);

        let selector =
            ResponseSelector::new(ResponseSelectionMode::WeightedRandom).with_weights(weights);
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        // Weighted random should return valid indices
        for _ in 0..100 {
            let idx = selector.select(&options);
            assert!(idx < options.len());
        }
    }

    #[test]
    fn test_mode_from_str() {
        assert_eq!(ResponseSelectionMode::from_str("first"), Some(ResponseSelectionMode::First));
        assert_eq!(
            ResponseSelectionMode::from_str("sequential"),
            Some(ResponseSelectionMode::Sequential)
        );
        assert_eq!(
            ResponseSelectionMode::from_str("round-robin"),
            Some(ResponseSelectionMode::Sequential)
        );
        assert_eq!(ResponseSelectionMode::from_str("random"), Some(ResponseSelectionMode::Random));
        assert_eq!(ResponseSelectionMode::from_str("invalid"), None);
    }

    #[test]
    fn test_reset_sequential() {
        let selector = ResponseSelector::new(ResponseSelectionMode::Sequential);
        let options = vec!["a".to_string(), "b".to_string()];

        assert_eq!(selector.select(&options), 0);
        assert_eq!(selector.select(&options), 1);

        selector.reset_sequential();
        assert_eq!(selector.select(&options), 0);
    }
}
