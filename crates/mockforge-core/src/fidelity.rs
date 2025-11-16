//! Fidelity Score Calculator
//!
//! Computes a fidelity score that quantifies how close a workspace is to its real upstream
//! based on schema and sample comparisons.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Fidelity score for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FidelityScore {
    /// Overall fidelity score (0.0 to 1.0, where 1.0 = perfect match)
    pub overall: f64,
    /// Schema similarity score (0.0 to 1.0)
    pub schema_similarity: f64,
    /// Sample similarity score (0.0 to 1.0)
    pub sample_similarity: f64,
    /// Response time similarity score (0.0 to 1.0)
    pub response_time_similarity: f64,
    /// Error pattern similarity score (0.0 to 1.0)
    pub error_pattern_similarity: f64,
    /// When the score was computed
    pub computed_at: DateTime<Utc>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

/// Schema comparator for comparing mock and real schemas
pub struct SchemaComparator;

impl SchemaComparator {
    /// Compare two schemas and compute similarity score
    ///
    /// # Arguments
    /// * `mock_schema` - Mock/expected schema
    /// * `real_schema` - Real/actual schema
    ///
    /// # Returns
    /// Similarity score (0.0 to 1.0)
    pub fn compare(&self, mock_schema: &Value, real_schema: &Value) -> f64 {
        // Use existing schema diff functionality
        let errors = crate::schema_diff::diff(mock_schema, real_schema);

        if errors.is_empty() {
            return 1.0;
        }

        // Calculate field coverage
        let mock_fields = Self::count_fields(mock_schema);
        let real_fields = Self::count_fields(real_schema);
        let total_fields = mock_fields.max(real_fields);

        if total_fields == 0 {
            return 1.0;
        }

        // Score based on error count and field coverage
        let error_penalty = errors.len() as f64 / total_fields as f64;
        let coverage_score = if mock_fields > 0 && real_fields > 0 {
            let common_fields = total_fields - errors.len();
            common_fields as f64 / total_fields as f64
        } else {
            0.0
        };

        // Combine scores
        (coverage_score * 0.7 + (1.0 - error_penalty.min(1.0)) * 0.3).max(0.0).min(1.0)
    }

    /// Count the number of fields in a schema
    fn count_fields(schema: &Value) -> usize {
        match schema {
            Value::Object(map) => {
                map.len() + map.values().map(|v| Self::count_fields(v)).sum::<usize>()
            }
            Value::Array(arr) => arr.iter().map(|v| Self::count_fields(v)).sum(),
            _ => 1,
        }
    }
}

/// Sample comparator for comparing mock and real sample responses
pub struct SampleComparator;

impl SampleComparator {
    /// Compare sample responses and compute similarity score
    ///
    /// # Arguments
    /// * `mock_samples` - Vector of mock sample responses
    /// * `real_samples` - Vector of real sample responses
    ///
    /// # Returns
    /// Similarity score (0.0 to 1.0)
    pub fn compare(&self, mock_samples: &[Value], real_samples: &[Value]) -> f64 {
        if mock_samples.is_empty() || real_samples.is_empty() {
            return 0.0;
        }

        // Compare structure similarity
        let structure_score = self.compare_structures(mock_samples, real_samples);

        // Compare value distributions (simplified)
        let distribution_score = self.compare_distributions(mock_samples, real_samples);

        // Combine scores
        (structure_score * 0.6 + distribution_score * 0.4).max(0.0).min(1.0)
    }

    /// Compare response structures
    fn compare_structures(&self, mock_samples: &[Value], real_samples: &[Value]) -> f64 {
        // Get structure from first sample of each
        let mock_structure = Self::extract_structure(mock_samples.first().unwrap());
        let real_structure = Self::extract_structure(real_samples.first().unwrap());

        // Compare structures
        let mock_fields: std::collections::HashSet<String> =
            mock_structure.keys().cloned().collect();
        let real_fields: std::collections::HashSet<String> =
            real_structure.keys().cloned().collect();

        let intersection = mock_fields.intersection(&real_fields).count();
        let union = mock_fields.union(&real_fields).count();

        if union == 0 {
            return 1.0;
        }

        intersection as f64 / union as f64
    }

    /// Extract structure from a JSON value
    fn extract_structure(value: &Value) -> HashMap<String, String> {
        let mut structure = HashMap::new();
        Self::extract_structure_recursive(value, "", &mut structure);
        structure
    }

    /// Recursive helper for structure extraction
    fn extract_structure_recursive(
        value: &Value,
        prefix: &str,
        structure: &mut HashMap<String, String>,
    ) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    structure.insert(path.clone(), Self::type_of(val));
                    Self::extract_structure_recursive(val, &path, structure);
                }
            }
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    Self::extract_structure_recursive(first, prefix, structure);
                }
            }
            _ => {
                if !prefix.is_empty() {
                    structure.insert(prefix.to_string(), Self::type_of(value));
                }
            }
        }
    }

    /// Get type string for a value
    fn type_of(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "bool".to_string(),
            Value::Number(n) => {
                if n.is_i64() {
                    "integer".to_string()
                } else {
                    "number".to_string()
                }
            }
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }

    /// Compare value distributions (simplified)
    fn compare_distributions(&self, mock_samples: &[Value], real_samples: &[Value]) -> f64 {
        // Simplified distribution comparison
        // In a real implementation, this would compare statistical distributions
        // For now, just check if value types match
        let mock_types: std::collections::HashSet<String> = mock_samples
            .iter()
            .map(|v| Self::type_of(v))
            .collect();
        let real_types: std::collections::HashSet<String> = real_samples
            .iter()
            .map(|v| Self::type_of(v))
            .collect();

        let intersection = mock_types.intersection(&real_types).count();
        let union = mock_types.union(&real_types).count();

        if union == 0 {
            return 1.0;
        }

        intersection as f64 / union as f64
    }
}

/// Fidelity calculator
pub struct FidelityCalculator {
    schema_comparator: SchemaComparator,
    sample_comparator: SampleComparator,
}

impl FidelityCalculator {
    /// Create a new fidelity calculator
    pub fn new() -> Self {
        Self {
            schema_comparator: SchemaComparator,
            sample_comparator: SampleComparator,
        }
    }

    /// Calculate fidelity score for a workspace
    ///
    /// # Arguments
    /// * `mock_schema` - Mock/expected schema
    /// * `real_schema` - Real/actual schema
    /// * `mock_samples` - Mock sample responses
    /// * `real_samples` - Real sample responses
    /// * `mock_response_times` - Mock response times (optional)
    /// * `real_response_times` - Real response times (optional)
    /// * `mock_error_patterns` - Mock error patterns (optional)
    /// * `real_error_patterns` - Real error patterns (optional)
    ///
    /// # Returns
    /// Fidelity score
    pub fn calculate(
        &self,
        mock_schema: &Value,
        real_schema: &Value,
        mock_samples: &[Value],
        real_samples: &[Value],
        mock_response_times: Option<&[u64]>,
        real_response_times: Option<&[u64]>,
        mock_error_patterns: Option<&HashMap<String, usize>>,
        real_error_patterns: Option<&HashMap<String, usize>>,
    ) -> FidelityScore {
        // Calculate schema similarity (40% weight)
        let schema_similarity = self.schema_comparator.compare(mock_schema, real_schema);

        // Calculate sample similarity (40% weight)
        let sample_similarity = self.sample_comparator.compare(mock_samples, real_samples);

        // Calculate response time similarity (10% weight)
        let response_time_similarity = self.compare_response_times(
            mock_response_times.unwrap_or(&[]),
            real_response_times.unwrap_or(&[]),
        );

        // Calculate error pattern similarity (10% weight)
        let error_pattern_similarity = self.compare_error_patterns(
            mock_error_patterns,
            real_error_patterns,
        );

        // Calculate overall score with weights
        let overall = (schema_similarity * 0.4
            + sample_similarity * 0.4
            + response_time_similarity * 0.1
            + error_pattern_similarity * 0.1)
            .max(0.0)
            .min(1.0);

        FidelityScore {
            overall,
            schema_similarity,
            sample_similarity,
            response_time_similarity,
            error_pattern_similarity,
            computed_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Compare response times
    fn compare_response_times(&self, mock_times: &[u64], real_times: &[u64]) -> f64 {
        if mock_times.is_empty() || real_times.is_empty() {
            return 0.5; // Neutral score if no data
        }

        let mock_avg = mock_times.iter().sum::<u64>() as f64 / mock_times.len() as f64;
        let real_avg = real_times.iter().sum::<u64>() as f64 / real_times.len() as f64;

        if real_avg == 0.0 {
            return if mock_avg == 0.0 { 1.0 } else { 0.0 };
        }

        // Calculate similarity based on ratio
        let ratio = mock_avg / real_avg;
        // Score is highest when ratio is close to 1.0
        (1.0 - (ratio - 1.0).abs()).max(0.0).min(1.0)
    }

    /// Compare error patterns
    fn compare_error_patterns(
        &self,
        mock_patterns: Option<&HashMap<String, usize>>,
        real_patterns: Option<&HashMap<String, usize>>,
    ) -> f64 {
        match (mock_patterns, real_patterns) {
            (Some(mock), Some(real)) => {
                if mock.is_empty() && real.is_empty() {
                    return 1.0;
                }

                let mock_keys: std::collections::HashSet<&String> = mock.keys().collect();
                let real_keys: std::collections::HashSet<&String> = real.keys().collect();

                let intersection = mock_keys.intersection(&real_keys).count();
                let union = mock_keys.union(&real_keys).count();

                if union == 0 {
                    return 1.0;
                }

                intersection as f64 / union as f64
            }
            _ => 0.5, // Neutral score if no data
        }
    }
}

impl Default for FidelityCalculator {
    fn default() -> Self {
        Self::new()
    }
}
