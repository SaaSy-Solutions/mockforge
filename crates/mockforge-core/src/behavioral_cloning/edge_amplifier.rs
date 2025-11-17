//! Rare edge case amplification
//!
//! This module provides functionality to identify and amplify rare
//! error patterns for testing purposes.

use crate::behavioral_cloning::types::{
    EdgeAmplificationConfig, EndpointProbabilityModel, ErrorPattern,
};
use crate::Result;

/// Edge amplifier for increasing rare error frequency
pub struct EdgeAmplifier;

impl EdgeAmplifier {
    /// Create a new edge amplifier
    pub fn new() -> Self {
        Self
    }

    /// Identify rare edge patterns in a probability model
    ///
    /// Finds patterns with probability below the threshold
    /// (default 1%).
    pub fn identify_rare_edges(
        model: &EndpointProbabilityModel,
        threshold: f64,
    ) -> Vec<&ErrorPattern> {
        model
            .error_patterns
            .iter()
            .filter(|pattern| pattern.probability < threshold)
            .collect()
    }

    /// Apply amplification to a probability model
    ///
    /// Increases the probability of rare patterns and normalizes
    /// the remaining probabilities to sum to 1.0.
    ///
    /// Stores original probabilities before amplification for later restoration.
    pub fn apply_amplification(
        model: &mut EndpointProbabilityModel,
        config: &EdgeAmplificationConfig,
    ) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        // Store original probabilities if not already stored
        if model.original_error_probabilities.is_none() {
            let mut original = std::collections::HashMap::new();
            for pattern in &model.error_patterns {
                original.insert(pattern.error_type.clone(), pattern.probability);
            }
            model.original_error_probabilities = Some(original);
        }

        // Identify rare patterns
        let rare_patterns: Vec<usize> = model
            .error_patterns
            .iter()
            .enumerate()
            .filter(|(_, pattern)| pattern.probability < config.rare_threshold)
            .map(|(idx, _)| idx)
            .collect();

        if rare_patterns.is_empty() {
            return Ok(());
        }

        // Calculate total probability of rare patterns
        let rare_total: f64 = rare_patterns
            .iter()
            .map(|&idx| model.error_patterns[idx].probability)
            .sum();

        // Calculate total probability of non-rare patterns
        let non_rare_total: f64 = model
            .error_patterns
            .iter()
            .enumerate()
            .filter(|(idx, _)| !rare_patterns.contains(idx))
            .map(|(_, pattern)| pattern.probability)
            .sum();

        // Set amplified probability for rare patterns
        let amplified_total = config.amplification_factor;

        // Normalize rare patterns to sum to amplified_total
        if rare_total > 0.0 {
            let scale_factor = amplified_total / rare_total;
            for &idx in &rare_patterns {
                model.error_patterns[idx].probability *= scale_factor;
            }
        } else {
            // If no rare patterns existed, distribute amplified_total evenly
            let per_pattern = amplified_total / rare_patterns.len() as f64;
            for &idx in &rare_patterns {
                model.error_patterns[idx].probability = per_pattern;
            }
        }

        // Normalize non-rare patterns to sum to (1.0 - amplified_total)
        if non_rare_total > 0.0 {
            let scale_factor = (1.0 - amplified_total) / non_rare_total;
            for (idx, pattern) in model.error_patterns.iter_mut().enumerate() {
                if !rare_patterns.contains(&idx) {
                    pattern.probability *= scale_factor;
                }
            }
        }

        // Ensure probabilities sum to 1.0 (with small tolerance for floating point)
        let total: f64 = model.error_patterns.iter().map(|p| p.probability).sum();
        if total > 0.0 && (total - 1.0).abs() > 0.001 {
            let scale = 1.0 / total;
            for pattern in &mut model.error_patterns {
                pattern.probability *= scale;
            }
        }

        Ok(())
    }

    /// Restore original probabilities (before amplification)
    ///
    /// Restores the error pattern probabilities to their values before
    /// amplification was applied. Requires that original probabilities
    /// were stored during amplification.
    pub fn restore_original(model: &mut EndpointProbabilityModel) -> Result<()> {
        let original_probs = match &model.original_error_probabilities {
            Some(probs) => probs,
            None => {
                // No original probabilities stored - nothing to restore
                return Ok(());
            }
        };

        // Restore each pattern's probability from the stored original
        for pattern in &mut model.error_patterns {
            if let Some(&original_prob) = original_probs.get(&pattern.error_type) {
                pattern.probability = original_prob;
            }
        }

        // Clear the stored original probabilities after restoration
        model.original_error_probabilities = None;

        Ok(())
    }
}

impl Default for EdgeAmplifier {
    fn default() -> Self {
        Self::new()
    }
}
