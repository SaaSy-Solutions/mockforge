//! Sequence learning from recorded traffic
//!
//! This module provides functionality to discover and model multi-step
//! flows from real traffic using trace correlation.

use crate::behavioral_cloning::types::BehavioralSequence;
use crate::scenarios::ScenarioDefinition;
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for querying trace data for sequence learning
#[async_trait]
pub trait TraceQueryProvider: Send + Sync {
    /// Get requests grouped by trace_id, ordered by timestamp
    ///
    /// Returns a vector of (trace_id, requests) tuples where requests
    /// are ordered by timestamp within each trace.
    async fn get_requests_by_trace(
        &self,
        min_requests_per_trace: Option<usize>,
    ) -> Result<Vec<(String, Vec<TraceRequest>)>>;
}

/// A request in a trace for sequence learning
#[derive(Debug, Clone)]
pub struct TraceRequest {
    /// Request ID
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Duration in milliseconds (time until next request in sequence)
    pub duration_ms: Option<u64>,
}

/// Sequence learner for discovering behavioral patterns
pub struct SequenceLearner;

impl SequenceLearner {
    /// Discover sequences from trace correlation
    ///
    /// Uses trace_id/span_id to find correlated request sequences
    /// and identifies common patterns.
    ///
    /// This implementation:
    /// 1. Queries database for requests grouped by trace_id
    /// 2. Orders by timestamp within each trace
    /// 3. Identifies common subsequences (e.g., login → list → detail)
    /// 4. Calculates transition probabilities between steps
    /// 5. Returns reusable behavioral sequences
    pub async fn discover_sequences_from_traces(
        provider: &dyn TraceQueryProvider,
        min_frequency: f64,
        min_requests_per_trace: Option<usize>,
    ) -> Result<Vec<BehavioralSequence>> {
        // Query database for requests grouped by trace_id
        let trace_groups = provider
            .get_requests_by_trace(min_requests_per_trace)
            .await?;

        if trace_groups.is_empty() {
            return Ok(Vec::new());
        }

        // Convert trace groups to sequences of (endpoint, method, delay) tuples
        let mut sequences: Vec<Vec<(String, String, Option<u64>)>> = Vec::new();
        let mut trace_ids: Vec<String> = Vec::new();

        for (trace_id, requests) in trace_groups {
            trace_ids.push(trace_id.clone());
            let mut sequence: Vec<(String, String, Option<u64>)> = Vec::new();

            // Sort requests by timestamp (should already be sorted, but ensure)
            let mut sorted_requests = requests;
            sorted_requests.sort_by_key(|r| r.timestamp);

            // Build sequence with delays between requests
            for (idx, request) in sorted_requests.iter().enumerate() {
                let delay = if idx > 0 {
                    let prev_timestamp = sorted_requests[idx - 1].timestamp;
                    let duration = request
                        .timestamp
                        .signed_duration_since(prev_timestamp)
                        .num_milliseconds();
                    if duration > 0 {
                        Some(duration as u64)
                    } else {
                        None
                    }
                } else {
                    None
                };

                sequence.push((
                    request.path.clone(),
                    request.method.clone(),
                    delay,
                ));
            }

            if !sequence.is_empty() {
                sequences.push(sequence);
            }
        }

        // Use existing learn_sequence_pattern to identify common patterns
        // This will match sequences and group them, but we need to track which trace_ids
        // contributed to each learned sequence
        let learned_sequences = Self::learn_sequence_pattern(&sequences, min_frequency)?;

        // Map learned sequences back to trace IDs by matching patterns
        let mut result = Vec::new();
        for mut learned_seq in learned_sequences {
            // Find all trace IDs that match this learned sequence pattern
            let mut contributing_traces = Vec::new();
            for (trace_idx, sequence) in sequences.iter().enumerate() {
                if trace_idx < trace_ids.len() {
                    // Check if this sequence matches the learned pattern
                    if sequence.len() == learned_seq.steps.len() {
                        let mut matches = true;
                        for (step_idx, step) in learned_seq.steps.iter().enumerate() {
                            if step_idx < sequence.len() {
                                let (path, method, _) = &sequence[step_idx];
                                if step.endpoint != *path || step.method != *method {
                                    matches = false;
                                    break;
                                }
                            } else {
                                matches = false;
                                break;
                            }
                        }
                        if matches {
                            contributing_traces.push(trace_ids[trace_idx].clone());
                        }
                    }
                }
            }
            learned_seq.learned_from = contributing_traces;
            result.push(learned_seq);
        }

        Ok(result)
    }

    /// Learn sequence pattern from a set of request sequences
    ///
    /// Analyzes multiple sequences to extract common patterns
    /// and calculate confidence scores.
    ///
    /// Each sequence is represented as (endpoint, method, delay_ms) tuples.
    pub fn learn_sequence_pattern(
        sequences: &[Vec<(String, String, Option<u64>)>], // (endpoint, method, delay)
        min_frequency: f64,
    ) -> Result<Vec<BehavioralSequence>> {
        if sequences.is_empty() {
            return Ok(Vec::new());
        }

        // Find common subsequences using longest common subsequence (LCS) approach
        // For simplicity, we'll find exact matches first, then look for patterns

        // Step 1: Normalize sequences to (method, endpoint) pairs for comparison
        let normalized: Vec<Vec<(String, String)>> = sequences
            .iter()
            .map(|seq| {
                seq.iter()
                    .map(|(endpoint, method, _)| (method.clone(), endpoint.clone()))
                    .collect()
            })
            .collect();

        // Step 2: Find exact sequence matches
        let mut sequence_counts: HashMap<Vec<(String, String)>, (usize, Vec<usize>)> =
            HashMap::new();
        for (idx, seq) in normalized.iter().enumerate() {
            let entry = sequence_counts.entry(seq.clone()).or_insert_with(|| (0, Vec::new()));
            entry.0 += 1;
            entry.1.push(idx);
        }

        // Step 3: Build BehavioralSequences from frequent patterns
        let total_sequences = sequences.len() as f64;
        let mut learned_sequences = Vec::new();

        for (pattern, (count, indices)) in sequence_counts {
            let frequency = count as f64 / total_sequences;
            if frequency < min_frequency {
                continue;
            }

            // Calculate confidence based on consistency
            // Higher confidence if the pattern appears consistently with similar delays
            let mut delays_by_position: Vec<Vec<u64>> = vec![Vec::new(); pattern.len()];
            for &idx in &indices {
                let original_seq = &sequences[idx];
                for (pos, (_, _, delay)) in original_seq.iter().enumerate() {
                    if let Some(d) = delay {
                        if pos < delays_by_position.len() {
                            delays_by_position[pos].push(*d);
                        }
                    }
                }
            }

            // Calculate average delays and variance for confidence
            let mut steps = Vec::new();
            let mut total_variance = 0.0;
            for (pos, (method, endpoint)) in pattern.iter().enumerate() {
                let avg_delay = if pos < delays_by_position.len()
                    && !delays_by_position[pos].is_empty()
                {
                    let delays = &delays_by_position[pos];
                    let avg = delays.iter().sum::<u64>() as f64 / delays.len() as f64;
                    let variance = delays
                        .iter()
                        .map(|&d| {
                            let diff = d as f64 - avg;
                            diff * diff
                        })
                        .sum::<f64>()
                        / delays.len() as f64;
                    total_variance += variance;
                    Some(avg as u64)
                } else {
                    None
                };

                let mut step = crate::behavioral_cloning::types::SequenceStep::new(
                    endpoint.clone(),
                    method.clone(),
                );
                if let Some(delay) = avg_delay {
                    step.expected_delay_ms = Some(delay);
                }
                // Calculate transition probability (how often this step follows the previous)
                let transition_prob = if pos == 0 {
                    1.0 // First step always happens
                } else {
                    // Count how many sequences have this step after the previous
                    let prev_pattern = &pattern[..pos];
                    let matching_sequences = normalized
                        .iter()
                        .filter(|seq| {
                            seq.len() > pos && seq[..pos] == *prev_pattern && seq[pos] == (method.clone(), endpoint.clone())
                        })
                        .count();
                    matching_sequences as f64 / count as f64
                };
                step.probability = transition_prob;
                steps.push(step);
            }

            // Confidence is based on frequency and consistency (inverse of variance)
            let avg_variance = total_variance / pattern.len() as f64;
            let consistency_score = 1.0 / (1.0 + avg_variance / 1000.0); // Normalize variance
            let confidence = frequency * 0.7 + consistency_score * 0.3;

            // Generate sequence ID and name
            let sequence_id = format!(
                "seq_{}_{}",
                pattern[0].0.to_lowercase(),
                pattern[0].1.replace('/', "_").replace('{', "").replace('}', "")
            );
            let sequence_name = format!(
                "{} {} → {} steps",
                pattern[0].0,
                pattern[0].1,
                pattern.len()
            );

            let learned_from: Vec<String> = indices
                .iter()
                .map(|&idx| format!("trace_{}", idx))
                .collect();

            let sequence = BehavioralSequence::new(sequence_id, sequence_name)
                .with_frequency(frequency)
                .with_confidence(confidence)
                .with_learned_from(learned_from);

            // Add steps
            let mut seq_with_steps = sequence;
            for step in steps {
                seq_with_steps = seq_with_steps.add_step(step);
            }

            learned_sequences.push(seq_with_steps);
        }

        // Sort by frequency and confidence
        learned_sequences.sort_by(|a, b| {
            b.frequency
                .partial_cmp(&a.frequency)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        Ok(learned_sequences)
    }

    /// Generate a scenario definition from a learned sequence
    ///
    /// Converts a BehavioralSequence into a ScenarioDefinition
    /// that can be executed by the ScenarioExecutor.
    pub fn generate_sequence_scenario(sequence: &BehavioralSequence) -> ScenarioDefinition {
        use crate::scenarios::ScenarioStep;

        let mut scenario = ScenarioDefinition::new(&sequence.id, &sequence.name)
            .with_tags(sequence.tags.clone());

        if let Some(description) = &sequence.description {
            scenario.description = Some(description.clone());
        }

        // Convert sequence steps to scenario steps
        for (idx, step) in sequence.steps.iter().enumerate() {
            let mut scenario_step = ScenarioStep::new(
                format!("step_{}", idx),
                step.name.as_deref().unwrap_or(&step.endpoint),
                &step.method,
                &step.endpoint,
            )
            .expect_status(200); // Default, could be learned

            if let Some(delay) = step.expected_delay_ms {
                scenario_step.delay_ms = Some(delay);
            }

            // Add conditions as query params if needed
            for (key, value) in &step.conditions {
                scenario_step.query_params.insert(key.clone(), value.clone());
            }

            scenario = scenario.add_step(scenario_step);
        }

        scenario
    }

    /// Check if an incoming request matches a learned sequence
    ///
    /// Returns the matching sequence and current step index if found.
    pub fn match_sequence<'a>(
        sequences: &'a [BehavioralSequence],
        endpoint: &str,
        method: &str,
        conditions: Option<&HashMap<String, String>>,
    ) -> Option<(&'a BehavioralSequence, usize)> {
        // Find sequences that start with this endpoint/method
        for sequence in sequences {
            if let Some(first_step) = sequence.steps.first() {
                // Check if endpoint and method match
                if first_step.endpoint == endpoint && first_step.method == method {
                    // Check conditions if provided
                    if let Some(conditions_map) = conditions {
                        let mut matches = true;
                        for (key, value) in &first_step.conditions {
                            if let Some(actual_value) = conditions_map.get(key) {
                                if actual_value != value {
                                    matches = false;
                                    break;
                                }
                            } else {
                                matches = false;
                                break;
                            }
                        }
                        if !matches {
                            continue;
                        }
                    } else if !first_step.conditions.is_empty() {
                        // Sequence requires conditions but none provided
                        continue;
                    }

                    // Found a match - return sequence with step index 0
                    return Some((sequence, 0));
                }
            }
        }

        None
    }

    /// Find the next step in a sequence given the current step
    ///
    /// Returns the next step index if found, or None if sequence is complete.
    pub fn find_next_step(
        sequence: &BehavioralSequence,
        current_step_idx: usize,
        endpoint: &str,
        method: &str,
    ) -> Option<usize> {
        if current_step_idx + 1 >= sequence.steps.len() {
            return None; // Sequence complete
        }

        let next_step = &sequence.steps[current_step_idx + 1];
        if next_step.endpoint == endpoint && next_step.method == method {
            Some(current_step_idx + 1)
        } else {
            None
        }
    }
}
