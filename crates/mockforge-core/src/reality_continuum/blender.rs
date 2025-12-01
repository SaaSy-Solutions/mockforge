//! Response blending logic for Reality Continuum
//!
//! Implements intelligent merging of mock and real responses based on blend ratio.
//! Supports deep merging of JSON objects, combining arrays, and weighted selection
//! for primitive values.

use super::config::MergeStrategy;
use serde_json::Value;
use std::collections::HashMap;

/// Response blender for combining mock and real responses
#[derive(Debug, Clone)]
pub struct ResponseBlender {
    /// Merge strategy to use
    strategy: MergeStrategy,
}

impl ResponseBlender {
    /// Create a new response blender with the specified strategy
    pub fn new(strategy: MergeStrategy) -> Self {
        Self { strategy }
    }

    /// Create a new response blender with default field-level strategy
    pub fn default() -> Self {
        Self {
            strategy: MergeStrategy::FieldLevel,
        }
    }

    /// Blend two JSON responses based on the blend ratio
    ///
    /// # Arguments
    /// * `mock` - Mock response value
    /// * `real` - Real response value
    /// * `ratio` - Blend ratio (0.0 = 100% mock, 1.0 = 100% real)
    ///
    /// # Returns
    /// Blended response value
    pub fn blend_responses(&self, mock: &Value, real: &Value, ratio: f64) -> Value {
        self.blend_responses_with_config(mock, real, ratio, None)
    }

    /// Blend two JSON responses with field-level configuration
    ///
    /// # Arguments
    /// * `mock` - Mock response value
    /// * `real` - Real response value
    /// * `global_ratio` - Global blend ratio (0.0 = 100% mock, 1.0 = 100% real)
    /// * `field_config` - Optional field-level reality configuration
    ///
    /// # Returns
    /// Blended response value
    pub fn blend_responses_with_config(
        &self,
        mock: &Value,
        real: &Value,
        global_ratio: f64,
        field_config: Option<&super::field_mixer::FieldRealityConfig>,
    ) -> Value {
        let global_ratio = global_ratio.clamp(0.0, 1.0);

        // If no field config, use global ratio
        if field_config.is_none() {
            // If ratio is 0.0, return mock entirely
            if global_ratio == 0.0 {
                return mock.clone();
            }

            // If ratio is 1.0, return real entirely
            if global_ratio == 1.0 {
                return real.clone();
            }

            // Apply the selected merge strategy
            match self.strategy {
                MergeStrategy::FieldLevel => self.blend_field_level(mock, real, global_ratio),
                MergeStrategy::Weighted => self.blend_weighted(mock, real, global_ratio),
                MergeStrategy::BodyBlend => self.blend_body(mock, real, global_ratio),
            }
        } else {
            // Use field-level blending
            self.blend_with_field_config(mock, real, global_ratio, field_config.unwrap())
        }
    }

    /// Blend responses with field-level configuration
    fn blend_with_field_config(
        &self,
        mock: &Value,
        real: &Value,
        global_ratio: f64,
        field_config: &super::field_mixer::FieldRealityConfig,
    ) -> Value {
        match (mock, real) {
            (Value::Object(mock_obj), Value::Object(real_obj)) => {
                let mut result = serde_json::Map::new();

                // Collect all keys from both objects
                let mut all_keys = std::collections::HashSet::new();
                for key in mock_obj.keys() {
                    all_keys.insert(key.clone());
                }
                for key in real_obj.keys() {
                    all_keys.insert(key.clone());
                }

                // Blend each key with field-specific ratio
                for key in all_keys {
                    let json_path = key.clone();
                    let mock_val = mock_obj.get(&key);
                    let real_val = real_obj.get(&key);

                    // Get field-specific blend ratio
                    let field_ratio =
                        field_config.get_blend_ratio_for_path(&json_path).unwrap_or(global_ratio);

                    match (mock_val, real_val) {
                        (Some(m), Some(r)) => {
                            // Both exist - recursively blend with field ratio
                            result.insert(
                                key,
                                self.blend_with_field_config(m, r, field_ratio, field_config),
                            );
                        }
                        (Some(m), None) => {
                            // Only in mock - include if field ratio < 0.5
                            if field_ratio < 0.5 {
                                result.insert(key, m.clone());
                            }
                        }
                        (None, Some(r)) => {
                            // Only in real - include if field ratio >= 0.5
                            if field_ratio >= 0.5 {
                                result.insert(key, r.clone());
                            }
                        }
                        (None, None) => {
                            // Neither (shouldn't happen)
                        }
                    }
                }

                Value::Object(result)
            }
            (Value::Array(mock_arr), Value::Array(real_arr)) => {
                // For arrays, use global ratio (field-level doesn't apply well to arrays)
                match self.strategy {
                    MergeStrategy::FieldLevel => self.blend_field_level(mock, real, global_ratio),
                    MergeStrategy::Weighted => self.blend_weighted(mock, real, global_ratio),
                    MergeStrategy::BodyBlend => self.blend_body(mock, real, global_ratio),
                }
            }
            _ => {
                // For primitives, use global ratio
                if global_ratio < 0.5 {
                    mock.clone()
                } else {
                    real.clone()
                }
            }
        }
    }

    /// Field-level intelligent merge
    ///
    /// Deep merges objects, combines arrays, and uses weighted selection for primitives.
    fn blend_field_level(&self, mock: &Value, real: &Value, ratio: f64) -> Value {
        match (mock, real) {
            // Both are objects - deep merge
            (Value::Object(mock_obj), Value::Object(real_obj)) => {
                let mut result = serde_json::Map::new();

                // Collect all keys from both objects
                let mut all_keys = std::collections::HashSet::new();
                for key in mock_obj.keys() {
                    all_keys.insert(key.clone());
                }
                for key in real_obj.keys() {
                    all_keys.insert(key.clone());
                }

                // Merge each key
                for key in all_keys {
                    let mock_val = mock_obj.get(&key);
                    let real_val = real_obj.get(&key);

                    match (mock_val, real_val) {
                        (Some(m), Some(r)) => {
                            // Both exist - recursively blend
                            result.insert(key, self.blend_field_level(m, r, ratio));
                        }
                        (Some(m), None) => {
                            // Only in mock - include with reduced weight
                            if ratio < 0.5 {
                                result.insert(key, m.clone());
                            }
                        }
                        (None, Some(r)) => {
                            // Only in real - include with increased weight
                            if ratio >= 0.5 {
                                result.insert(key, r.clone());
                            }
                        }
                        (None, None) => {
                            // Neither (shouldn't happen)
                        }
                    }
                }

                Value::Object(result)
            }
            // Both are arrays - combine based on ratio
            (Value::Array(mock_arr), Value::Array(real_arr)) => {
                let mut result = Vec::new();

                // Calculate how many items from each array
                let total_len = mock_arr.len().max(real_arr.len());
                let mock_count = ((1.0 - ratio) * total_len as f64).round() as usize;
                let real_count = (ratio * total_len as f64).round() as usize;

                // Add items from mock array
                for (i, item) in mock_arr.iter().enumerate() {
                    if i < mock_count {
                        result.push(item.clone());
                    }
                }

                // Add items from real array
                for (i, item) in real_arr.iter().enumerate() {
                    if i < real_count {
                        result.push(item.clone());
                    }
                }

                // If arrays have different lengths, blend remaining items
                if mock_arr.len() != real_arr.len() {
                    let min_len = mock_arr.len().min(real_arr.len());
                    for i in min_len..total_len {
                        if i < mock_arr.len() && i < real_arr.len() {
                            // Blend corresponding items
                            result.push(self.blend_field_level(&mock_arr[i], &real_arr[i], ratio));
                        } else if i < mock_arr.len() {
                            result.push(mock_arr[i].clone());
                        } else if i < real_arr.len() {
                            result.push(real_arr[i].clone());
                        }
                    }
                }

                Value::Array(result)
            }
            // Both are numbers - weighted average
            (Value::Number(mock_num), Value::Number(real_num)) => {
                if let (Some(mock_f64), Some(real_f64)) = (mock_num.as_f64(), real_num.as_f64()) {
                    let blended = mock_f64 * (1.0 - ratio) + real_f64 * ratio;
                    Value::Number(serde_json::Number::from_f64(blended).unwrap_or(mock_num.clone()))
                } else {
                    // Fallback to weighted selection
                    if ratio < 0.5 {
                        Value::Number(mock_num.clone())
                    } else {
                        Value::Number(real_num.clone())
                    }
                }
            }
            // Both are strings - weighted selection
            (Value::String(mock_str), Value::String(real_str)) => {
                if ratio < 0.5 {
                    Value::String(mock_str.clone())
                } else {
                    Value::String(real_str.clone())
                }
            }
            // Both are booleans - weighted selection
            (Value::Bool(mock_bool), Value::Bool(real_bool)) => {
                if ratio < 0.5 {
                    Value::Bool(*mock_bool)
                } else {
                    Value::Bool(*real_bool)
                }
            }
            // Type mismatch - prefer real if ratio > 0.5, otherwise mock
            _ => {
                if ratio >= 0.5 {
                    real.clone()
                } else {
                    mock.clone()
                }
            }
        }
    }

    /// Weighted selection strategy
    ///
    /// Randomly selects between mock and real based on ratio (for testing/demo purposes).
    /// In practice, this would use the ratio as a probability threshold.
    fn blend_weighted(&self, mock: &Value, real: &Value, ratio: f64) -> Value {
        // For deterministic behavior, use threshold-based selection
        // In a real implementation, you might want to use actual randomness
        if ratio >= 0.5 {
            real.clone()
        } else {
            mock.clone()
        }
    }

    /// Body blending strategy
    ///
    /// Merges arrays, averages numeric fields, and deep merges objects.
    fn blend_body(&self, mock: &Value, real: &Value, ratio: f64) -> Value {
        // Similar to field-level but with different array handling
        match (mock, real) {
            (Value::Object(mock_obj), Value::Object(real_obj)) => {
                let mut result = serde_json::Map::new();

                // Collect all keys
                let mut all_keys = std::collections::HashSet::new();
                for key in mock_obj.keys() {
                    all_keys.insert(key.clone());
                }
                for key in real_obj.keys() {
                    all_keys.insert(key.clone());
                }

                // Merge each key
                for key in all_keys {
                    let mock_val = mock_obj.get(&key);
                    let real_val = real_obj.get(&key);

                    match (mock_val, real_val) {
                        (Some(m), Some(r)) => {
                            result.insert(key, self.blend_body(m, r, ratio));
                        }
                        (Some(m), None) => {
                            result.insert(key, m.clone());
                        }
                        (None, Some(r)) => {
                            result.insert(key, r.clone());
                        }
                        (None, None) => {}
                    }
                }

                Value::Object(result)
            }
            (Value::Array(mock_arr), Value::Array(real_arr)) => {
                // Combine arrays, interleaving based on ratio
                let mut result = Vec::new();
                let max_len = mock_arr.len().max(real_arr.len());

                for i in 0..max_len {
                    if i < mock_arr.len() && i < real_arr.len() {
                        // Blend corresponding items
                        result.push(self.blend_body(&mock_arr[i], &real_arr[i], ratio));
                    } else if i < mock_arr.len() {
                        result.push(mock_arr[i].clone());
                    } else if i < real_arr.len() {
                        result.push(real_arr[i].clone());
                    }
                }

                Value::Array(result)
            }
            (Value::Number(mock_num), Value::Number(real_num)) => {
                if let (Some(mock_f64), Some(real_f64)) = (mock_num.as_f64(), real_num.as_f64()) {
                    let blended = mock_f64 * (1.0 - ratio) + real_f64 * ratio;
                    Value::Number(serde_json::Number::from_f64(blended).unwrap_or(mock_num.clone()))
                } else if ratio < 0.5 {
                    Value::Number(mock_num.clone())
                } else {
                    Value::Number(real_num.clone())
                }
            }
            _ => {
                if ratio >= 0.5 {
                    real.clone()
                } else {
                    mock.clone()
                }
            }
        }
    }

    /// Blend HTTP status codes
    ///
    /// Returns the status code to use based on blend ratio.
    /// Prefers real status code if ratio > 0.5, otherwise uses mock.
    pub fn blend_status_code(&self, mock_status: u16, real_status: u16, ratio: f64) -> u16 {
        if ratio >= 0.5 {
            real_status
        } else {
            mock_status
        }
    }

    /// Blend HTTP headers
    ///
    /// Merges headers from both responses, preferring real headers when ratio > 0.5.
    pub fn blend_headers(
        &self,
        mock_headers: &HashMap<String, String>,
        real_headers: &HashMap<String, String>,
        ratio: f64,
    ) -> HashMap<String, String> {
        let mut result = HashMap::new();

        // Collect all header keys
        let mut all_keys = std::collections::HashSet::new();
        for key in mock_headers.keys() {
            all_keys.insert(key.clone());
        }
        for key in real_headers.keys() {
            all_keys.insert(key.clone());
        }

        // Merge headers
        for key in all_keys {
            let mock_val = mock_headers.get(&key);
            let real_val = real_headers.get(&key);

            match (mock_val, real_val) {
                (Some(m), Some(r)) => {
                    // Both exist - prefer real if ratio > 0.5
                    if ratio >= 0.5 {
                        result.insert(key, r.clone());
                    } else {
                        result.insert(key, m.clone());
                    }
                }
                (Some(m), None) => {
                    // Only in mock - include if ratio < 0.5
                    if ratio < 0.5 {
                        result.insert(key, m.clone());
                    }
                }
                (None, Some(r)) => {
                    // Only in real - include if ratio >= 0.5
                    if ratio >= 0.5 {
                        result.insert(key, r.clone());
                    }
                }
                (None, None) => {}
            }
        }

        result
    }
}

impl Default for ResponseBlender {
    fn default() -> Self {
        Self::new(MergeStrategy::FieldLevel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_blend_objects() {
        let blender = ResponseBlender::default();
        let mock = json!({
            "id": 1,
            "name": "Mock User",
            "email": "mock@example.com"
        });
        let real = json!({
            "id": 2,
            "name": "Real User",
            "status": "active"
        });

        let blended = blender.blend_responses(&mock, &real, 0.5);
        assert!(blended.is_object());
    }

    #[test]
    fn test_blend_arrays() {
        let blender = ResponseBlender::default();
        let mock = json!([1, 2, 3]);
        let real = json!([4, 5, 6]);

        let blended = blender.blend_responses(&mock, &real, 0.5);
        assert!(blended.is_array());
    }

    #[test]
    fn test_blend_numbers() {
        let blender = ResponseBlender::default();
        let mock = json!(10.0);
        let real = json!(20.0);

        let blended = blender.blend_responses(&mock, &real, 0.5);
        if let Value::Number(n) = blended {
            if let Some(f) = n.as_f64() {
                assert!((f - 15.0).abs() < 0.1); // Should be approximately 15.0
            }
        }
    }

    #[test]
    fn test_blend_status_code() {
        let blender = ResponseBlender::default();
        assert_eq!(blender.blend_status_code(200, 404, 0.3), 200); // Prefer mock
        assert_eq!(blender.blend_status_code(200, 404, 0.7), 404); // Prefer real
    }

    #[test]
    fn test_blend_headers() {
        let blender = ResponseBlender::default();
        let mut mock_headers = HashMap::new();
        mock_headers.insert("X-Mock".to_string(), "true".to_string());
        mock_headers.insert("Content-Type".to_string(), "application/json".to_string());

        let mut real_headers = HashMap::new();
        real_headers.insert("X-Real".to_string(), "true".to_string());
        real_headers.insert("Content-Type".to_string(), "application/xml".to_string());

        let blended = blender.blend_headers(&mock_headers, &real_headers, 0.7);
        assert_eq!(blended.get("Content-Type"), Some(&"application/xml".to_string()));
        assert_eq!(blended.get("X-Real"), Some(&"true".to_string()));
    }

    #[test]
    fn test_blend_nested_objects() {
        let blender = ResponseBlender::default();
        let mock = json!({
            "user": {
                "id": 1,
                "name": "Mock User",
                "email": "mock@example.com"
            },
            "metadata": {
                "source": "mock"
            }
        });
        let real = json!({
            "user": {
                "id": 2,
                "name": "Real User",
                "status": "active"
            },
            "metadata": {
                "source": "real",
                "timestamp": "2025-01-01T00:00:00Z"
            }
        });

        let blended = blender.blend_responses(&mock, &real, 0.5);
        assert!(blended.is_object());
        assert!(blended.get("user").is_some());
        assert!(blended.get("metadata").is_some());
    }

    #[test]
    fn test_blend_ratio_boundaries() {
        let blender = ResponseBlender::default();
        let mock = json!({"value": "mock"});
        let real = json!({"value": "real"});

        // At 0.0, should return mock
        let result_0 = blender.blend_responses(&mock, &real, 0.0);
        assert_eq!(result_0, mock);

        // At 1.0, should return real
        let result_1 = blender.blend_responses(&mock, &real, 1.0);
        assert_eq!(result_1, real);
    }

    #[test]
    fn test_blend_mixed_types() {
        let blender = ResponseBlender::default();
        let mock = json!({
            "string": "mock",
            "number": 10,
            "boolean": true,
            "array": [1, 2, 3]
        });
        let real = json!({
            "string": "real",
            "number": 20,
            "boolean": false,
            "array": [4, 5, 6]
        });

        let blended = blender.blend_responses(&mock, &real, 0.5);
        assert!(blended.is_object());
        assert!(blended.get("string").is_some());
        assert!(blended.get("number").is_some());
        assert!(blended.get("boolean").is_some());
        assert!(blended.get("array").is_some());
    }
}
