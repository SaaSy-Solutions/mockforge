//! Probabilistic outcome modeling
//!
//! This module provides functionality to build and use probability models
//! for endpoint behavior, including status codes, latency, and error patterns.

use crate::behavioral_cloning::types::{
    EndpointProbabilityModel, ErrorPattern, LatencyDistribution,
};
use std::collections::HashMap;

/// Probabilistic model builder and sampler
pub struct ProbabilisticModel;

impl ProbabilisticModel {
    /// Build a probability model from a list of status codes and latencies
    ///
    /// This is a pure function that takes observed data and builds a probability model.
    /// The caller is responsible for querying the database and providing the data.
    pub fn build_probability_model_from_data(
        endpoint: &str,
        method: &str,
        status_codes: &[u16],
        latencies_ms: &[u64],
        error_responses: &[(u16, serde_json::Value)],
        request_payloads: &[serde_json::Value],
        response_payloads: &[serde_json::Value],
    ) -> EndpointProbabilityModel {
        let sample_count = status_codes.len().max(latencies_ms.len()) as u64;

        // Calculate status code distribution
        let mut status_code_counts: HashMap<u16, usize> = HashMap::new();
        for &code in status_codes {
            *status_code_counts.entry(code).or_insert(0) += 1;
        }

        let total_status_codes = status_codes.len() as f64;
        let status_code_distribution: HashMap<u16, f64> = status_code_counts
            .into_iter()
            .map(|(code, count)| (code, count as f64 / total_status_codes))
            .collect();

        // Calculate latency distribution
        let latency_distribution = if latencies_ms.is_empty() {
            LatencyDistribution::new(0, 0, 0, 0.0, 0.0, 0, 0)
        } else {
            let mut sorted_latencies = latencies_ms.to_vec();
            sorted_latencies.sort_unstable();

            let len = sorted_latencies.len();
            let p50_idx = (len as f64 * 0.5) as usize;
            let p95_idx = (len as f64 * 0.95) as usize;
            let p99_idx = (len as f64 * 0.99).min((len - 1) as f64) as usize;

            let p50 = sorted_latencies[p50_idx.min(len - 1)];
            let p95 = sorted_latencies[p95_idx.min(len - 1)];
            let p99 = sorted_latencies[p99_idx.min(len - 1)];

            let mean = sorted_latencies.iter().sum::<u64>() as f64 / len as f64;
            let variance = sorted_latencies
                .iter()
                .map(|&x| {
                    let diff = x as f64 - mean;
                    diff * diff
                })
                .sum::<f64>()
                / len as f64;
            let std_dev = variance.sqrt();

            let min = *sorted_latencies.first().unwrap_or(&0);
            let max = *sorted_latencies.last().unwrap_or(&0);

            LatencyDistribution::new(p50, p95, p99, mean, std_dev, min, max)
        };

        // Identify error patterns
        let mut error_patterns: Vec<ErrorPattern> = Vec::new();
        let mut error_counts: HashMap<u16, (usize, Vec<serde_json::Value>)> = HashMap::new();

        for (status_code, response_body) in error_responses {
            if *status_code >= 400 {
                let entry = error_counts.entry(*status_code).or_insert_with(|| (0, Vec::new()));
                entry.0 += 1;
                entry.1.push(response_body.clone());
            }
        }

        let total_errors = error_responses.len() as f64;
        if total_errors > 0.0 {
            for (status_code, (count, samples)) in error_counts {
                let probability = count as f64 / total_errors;
                let mut pattern = ErrorPattern::new(format!("http_{}", status_code), probability);
                pattern.status_code = Some(status_code);
                if let Some(sample) = samples.first() {
                    pattern.sample_responses.push(sample.clone());
                }
                error_patterns.push(pattern);
            }
        }

        // Detect payload variations
        let payload_variations =
            Self::detect_payload_variations(request_payloads, response_payloads, status_codes);

        EndpointProbabilityModel {
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            status_code_distribution,
            latency_distribution,
            error_patterns,
            payload_variations,
            sample_count,
            updated_at: chrono::Utc::now(),
            original_error_probabilities: None,
        }
    }

    /// Detect payload variations from observed request/response bodies
    ///
    /// Groups similar payloads and calculates their probabilities.
    /// Uses structural similarity (JSON structure) rather than exact matching.
    fn detect_payload_variations(
        request_payloads: &[serde_json::Value],
        response_payloads: &[serde_json::Value],
        status_codes: &[u16],
    ) -> Vec<crate::behavioral_cloning::types::PayloadVariation> {
        use crate::behavioral_cloning::types::PayloadVariation;
        use std::collections::HashMap;

        if response_payloads.is_empty() && request_payloads.is_empty() {
            return Vec::new();
        }

        // Group response payloads by status code and structure
        let mut variation_groups: HashMap<String, (usize, serde_json::Value, Option<u16>)> =
            HashMap::new();

        // Process response payloads (grouped by status code)
        for (idx, payload) in response_payloads.iter().enumerate() {
            let status_code = if idx < status_codes.len() {
                Some(status_codes[idx])
            } else {
                None
            };

            // Create a structural signature (normalized JSON structure)
            let signature = Self::payload_signature(payload);
            let key = if let Some(code) = status_code {
                format!("{}:{}", code, signature)
            } else {
                signature.clone()
            };

            let entry =
                variation_groups.entry(key).or_insert_with(|| (0, payload.clone(), status_code));
            entry.0 += 1;
        }

        // Process request payloads (if provided)
        for payload in request_payloads {
            let signature = Self::payload_signature(payload);
            let key = format!("request:{}", signature);

            let entry = variation_groups.entry(key).or_insert_with(|| (0, payload.clone(), None));
            entry.0 += 1;
        }

        // Convert groups to PayloadVariation structs
        let total_samples =
            variation_groups.values().map(|(count, _, _)| *count).sum::<usize>() as f64;
        if total_samples == 0.0 {
            return Vec::new();
        }

        let mut variations = Vec::new();
        for (idx, (key, (count, sample, status_code))) in variation_groups.into_iter().enumerate() {
            let probability = count as f64 / total_samples;
            let variation_id = format!("var_{}", idx);

            let mut variation = PayloadVariation {
                id: variation_id,
                probability,
                sample_payload: sample,
                conditions: None,
            };

            // Add status code as a condition if present
            if let Some(code) = status_code {
                let mut conditions = HashMap::new();
                conditions.insert("status_code".to_string(), code.to_string());
                variation.conditions = Some(conditions);
            }

            variations.push(variation);
        }

        // Sort by probability (descending)
        variations.sort_by(|a, b| {
            b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal)
        });

        variations
    }

    /// Create a structural signature for a JSON payload
    ///
    /// Normalizes the payload to show only structure (keys, types) without values.
    /// This allows grouping similar payloads together.
    fn payload_signature(payload: &serde_json::Value) -> String {
        match payload {
            serde_json::Value::Object(map) => {
                let mut keys: Vec<String> = map.keys().cloned().collect();
                keys.sort();
                let mut sig_parts = Vec::new();
                for key in keys {
                    if let Some(value) = map.get(&key) {
                        let value_type = match value {
                            serde_json::Value::Null => "null",
                            serde_json::Value::Bool(_) => "bool",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::Object(_) => "object",
                        };
                        sig_parts.push(format!("{}:{}", key, value_type));
                    }
                }
                format!("{{{}}}", sig_parts.join(","))
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    // Use first element's structure as representative
                    format!("[{}]", Self::payload_signature(&arr[0]))
                }
            }
            _ => {
                // Primitive value - use type
                match payload {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Bool(_) => "bool",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::String(_) => "string",
                    _ => "unknown",
                }
                .to_string()
            }
        }
    }

    /// Sample a status code based on learned distribution
    pub fn sample_status_code(model: &EndpointProbabilityModel) -> u16 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random: f64 = rng.gen_range(0.0..1.0);

        let mut cumulative = 0.0;
        for (status_code, probability) in &model.status_code_distribution {
            cumulative += probability;
            if random <= cumulative {
                return *status_code;
            }
        }

        // Fallback to most common status code
        model
            .status_code_distribution
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(code, _)| *code)
            .unwrap_or(200)
    }

    /// Sample latency based on learned distribution
    pub fn sample_latency(model: &EndpointProbabilityModel) -> u64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Use normal distribution approximation based on mean and std_dev
        let mean = model.latency_distribution.mean;
        let std_dev = model.latency_distribution.std_dev;

        // Generate normal distribution sample using Box-Muller transform
        let u1: f64 = rng.gen_range(0.0..1.0);
        let u2: f64 = rng.gen_range(0.0..1.0);
        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let sample = mean + std_dev * z0;

        // Clamp to min/max bounds
        sample
            .max(model.latency_distribution.min as f64)
            .min(model.latency_distribution.max as f64) as u64
    }

    /// Sample an error pattern based on conditions
    pub fn sample_error_pattern<'a>(
        model: &'a EndpointProbabilityModel,
        _conditions: Option<&HashMap<String, String>>,
    ) -> Option<&'a ErrorPattern> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random: f64 = rng.gen_range(0.0..1.0);

        let mut cumulative = 0.0;
        for pattern in &model.error_patterns {
            cumulative += pattern.probability;
            if random <= cumulative {
                return Some(pattern);
            }
        }

        None
    }

    /// Update model incrementally with new observations
    pub fn update_model(
        model: &mut EndpointProbabilityModel,
        status_code: u16,
        latency_ms: u64,
        _error_pattern: Option<&ErrorPattern>,
    ) {
        // Update status code distribution
        let total = model.sample_count as f64;
        let new_total = total + 1.0;

        // Update frequency for observed status code
        for (code, prob) in model.status_code_distribution.iter_mut() {
            *prob = (*prob * total) / new_total;
        }

        let status_prob = model.status_code_distribution.entry(status_code).or_insert(0.0);
        *status_prob = (*status_prob * total + 1.0) / new_total;

        // Update latency distribution (simplified - would need proper percentile tracking)
        let old_mean = model.latency_distribution.mean;
        model.latency_distribution.mean = (old_mean * total + latency_ms as f64) / new_total;

        // Update min/max
        if latency_ms < model.latency_distribution.min {
            model.latency_distribution.min = latency_ms;
        }
        if latency_ms > model.latency_distribution.max {
            model.latency_distribution.max = latency_ms;
        }

        model.sample_count += 1;
        model.updated_at = chrono::Utc::now();
    }
}
