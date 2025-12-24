//! Invalid data generation for error testing
//!
//! This module provides functionality to generate invalid request data
//! for testing error handling. Supports mixing valid and invalid requests
//! based on a configurable error rate.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Types of invalid data that can be generated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvalidDataType {
    /// Omit a required field
    MissingField,
    /// Provide wrong data type (string where number expected, etc.)
    WrongType,
    /// Provide empty string where value required
    Empty,
    /// Provide null where not nullable
    Null,
    /// Provide value outside min/max constraints
    OutOfRange,
    /// Provide malformed data (invalid email, URL, etc.)
    Malformed,
}

impl std::fmt::Display for InvalidDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField => write!(f, "missing-field"),
            Self::WrongType => write!(f, "wrong-type"),
            Self::Empty => write!(f, "empty"),
            Self::Null => write!(f, "null"),
            Self::OutOfRange => write!(f, "out-of-range"),
            Self::Malformed => write!(f, "malformed"),
        }
    }
}

impl std::str::FromStr for InvalidDataType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "missing-field" | "missingfield" => Ok(Self::MissingField),
            "wrong-type" | "wrongtype" => Ok(Self::WrongType),
            "empty" => Ok(Self::Empty),
            "null" => Ok(Self::Null),
            "out-of-range" | "outofrange" => Ok(Self::OutOfRange),
            "malformed" => Ok(Self::Malformed),
            _ => Err(format!("Invalid error type: '{}'", s)),
        }
    }
}

/// Configuration for invalid data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidDataConfig {
    /// Percentage of requests that should use invalid data (0.0 to 1.0)
    pub error_rate: f64,
    /// Types of invalid data to generate
    pub error_types: HashSet<InvalidDataType>,
    /// Specific fields to target for invalidation (if empty, any field)
    pub target_fields: Vec<String>,
}

impl Default for InvalidDataConfig {
    fn default() -> Self {
        let mut error_types = HashSet::new();
        error_types.insert(InvalidDataType::MissingField);
        error_types.insert(InvalidDataType::WrongType);
        error_types.insert(InvalidDataType::Empty);

        Self {
            error_rate: 0.2, // 20% invalid by default
            error_types,
            target_fields: Vec::new(),
        }
    }
}

impl InvalidDataConfig {
    /// Create a new config with a specific error rate
    pub fn new(error_rate: f64) -> Self {
        Self {
            error_rate: error_rate.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Set the error types to generate
    pub fn with_error_types(mut self, types: HashSet<InvalidDataType>) -> Self {
        self.error_types = types;
        self
    }

    /// Add specific target fields
    pub fn with_target_fields(mut self, fields: Vec<String>) -> Self {
        self.target_fields = fields;
        self
    }

    /// Parse error types from a comma-separated string
    pub fn parse_error_types(s: &str) -> Result<HashSet<InvalidDataType>, String> {
        if s.is_empty() {
            return Ok(HashSet::new());
        }

        s.split(',')
            .map(|t| t.trim().parse::<InvalidDataType>())
            .collect()
    }
}

/// Generates k6 JavaScript code for invalid data testing
pub struct InvalidDataGenerator;

impl InvalidDataGenerator {
    /// Generate k6 code for determining if current request should be invalid
    pub fn generate_should_invalidate(error_rate: f64) -> String {
        format!(
            "// Determine if this request should use invalid data\n\
             const shouldInvalidate = Math.random() < {};\n",
            error_rate
        )
    }

    /// Generate k6 code for selecting a random invalid data type
    pub fn generate_type_selection(types: &HashSet<InvalidDataType>) -> String {
        let type_array: Vec<String> = types.iter().map(|t| format!("'{}'", t)).collect();

        format!(
            "// Select random invalid data type\n\
             const invalidTypes = [{}];\n\
             const invalidType = invalidTypes[Math.floor(Math.random() * invalidTypes.length)];\n",
            type_array.join(", ")
        )
    }

    /// Generate k6 code for creating invalid data based on type
    pub fn generate_invalidation_logic() -> String {
        r#"// Apply invalidation based on selected type
function invalidateField(value, fieldName, invalidType) {
  switch (invalidType) {
    case 'missing-field':
      return undefined; // Will be filtered out
    case 'wrong-type':
      if (typeof value === 'number') return 'not_a_number';
      if (typeof value === 'string') return 12345;
      if (typeof value === 'boolean') return 'not_a_boolean';
      if (Array.isArray(value)) return 'not_an_array';
      return null;
    case 'empty':
      if (typeof value === 'string') return '';
      if (Array.isArray(value)) return [];
      if (typeof value === 'object') return {};
      return null;
    case 'null':
      return null;
    case 'out-of-range':
      if (typeof value === 'number') return value > 0 ? -9999999 : 9999999;
      if (typeof value === 'string') return 'x'.repeat(10000);
      return value;
    case 'malformed':
      if (typeof value === 'string') {
        // Check common formats and malform them
        if (value.includes('@')) return 'not-an-email';
        if (value.startsWith('http')) return 'not://a.valid.url';
        return value + '%%%invalid%%%';
      }
      return value;
    default:
      return value;
  }
}

function invalidatePayload(payload, targetFields, invalidType) {
  const result = { ...payload };

  // Determine which fields to invalidate
  let fieldsToInvalidate;
  if (targetFields && targetFields.length > 0) {
    fieldsToInvalidate = targetFields;
  } else {
    // Pick a random field
    const allFields = Object.keys(result);
    fieldsToInvalidate = [allFields[Math.floor(Math.random() * allFields.length)]];
  }

  for (const field of fieldsToInvalidate) {
    if (result.hasOwnProperty(field)) {
      const newValue = invalidateField(result[field], field, invalidType);
      if (newValue === undefined) {
        delete result[field];
      } else {
        result[field] = newValue;
      }
    }
  }

  return result;
}
"#
        .to_string()
    }

    /// Generate k6 code for a complete invalid data test scenario
    pub fn generate_complete_invalidation(config: &InvalidDataConfig, target_fields_js: &str) -> String {
        let mut code = String::new();

        code.push_str(&Self::generate_should_invalidate(config.error_rate));
        code.push('\n');
        code.push_str(&Self::generate_type_selection(&config.error_types));
        code.push('\n');
        code.push_str(&format!(
            "const targetFields = {};\n\n",
            target_fields_js
        ));
        code.push_str("// Apply invalidation if needed\n");
        code.push_str("const finalPayload = shouldInvalidate\n");
        code.push_str("  ? invalidatePayload(payload, targetFields, invalidType)\n");
        code.push_str("  : payload;\n");

        code
    }

    /// Generate k6 code for checking expected error responses
    pub fn generate_error_checks() -> String {
        r#"// Check response based on whether we sent invalid data
if (shouldInvalidate) {
  check(res, {
    'invalid request: expects error response': (r) => r.status >= 400,
    'invalid request: has error message': (r) => {
      try {
        const body = r.json();
        return body.error || body.message || body.errors;
      } catch (e) {
        return r.body && r.body.length > 0;
      }
    },
  });
} else {
  check(res, {
    'valid request: status is OK': (r) => r.status >= 200 && r.status < 300,
  });
}
"#
        .to_string()
    }

    /// Generate complete test helper functions
    pub fn generate_helper_functions() -> String {
        Self::generate_invalidation_logic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_invalid_data_type_display() {
        assert_eq!(InvalidDataType::MissingField.to_string(), "missing-field");
        assert_eq!(InvalidDataType::WrongType.to_string(), "wrong-type");
        assert_eq!(InvalidDataType::Empty.to_string(), "empty");
        assert_eq!(InvalidDataType::Null.to_string(), "null");
        assert_eq!(InvalidDataType::OutOfRange.to_string(), "out-of-range");
        assert_eq!(InvalidDataType::Malformed.to_string(), "malformed");
    }

    #[test]
    fn test_invalid_data_type_from_str() {
        assert_eq!(
            InvalidDataType::from_str("missing-field").unwrap(),
            InvalidDataType::MissingField
        );
        assert_eq!(
            InvalidDataType::from_str("wrong-type").unwrap(),
            InvalidDataType::WrongType
        );
        assert_eq!(
            InvalidDataType::from_str("empty").unwrap(),
            InvalidDataType::Empty
        );
        assert_eq!(
            InvalidDataType::from_str("null").unwrap(),
            InvalidDataType::Null
        );
        assert_eq!(
            InvalidDataType::from_str("out-of-range").unwrap(),
            InvalidDataType::OutOfRange
        );
    }

    #[test]
    fn test_invalid_data_type_from_str_variants() {
        // With underscores
        assert_eq!(
            InvalidDataType::from_str("missing_field").unwrap(),
            InvalidDataType::MissingField
        );

        // Without separator
        assert_eq!(
            InvalidDataType::from_str("wrongtype").unwrap(),
            InvalidDataType::WrongType
        );
    }

    #[test]
    fn test_invalid_data_type_from_str_invalid() {
        assert!(InvalidDataType::from_str("invalid").is_err());
    }

    #[test]
    fn test_invalid_data_config_default() {
        let config = InvalidDataConfig::default();
        assert!((config.error_rate - 0.2).abs() < f64::EPSILON);
        assert!(config.error_types.contains(&InvalidDataType::MissingField));
        assert!(config.error_types.contains(&InvalidDataType::WrongType));
        assert!(config.error_types.contains(&InvalidDataType::Empty));
        assert!(config.target_fields.is_empty());
    }

    #[test]
    fn test_invalid_data_config_new() {
        let config = InvalidDataConfig::new(0.5);
        assert!((config.error_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invalid_data_config_clamp() {
        let config1 = InvalidDataConfig::new(1.5);
        assert!((config1.error_rate - 1.0).abs() < f64::EPSILON);

        let config2 = InvalidDataConfig::new(-0.5);
        assert!((config2.error_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invalid_data_config_builders() {
        let mut types = HashSet::new();
        types.insert(InvalidDataType::Null);

        let config = InvalidDataConfig::new(0.3)
            .with_error_types(types)
            .with_target_fields(vec!["email".to_string()]);

        assert!((config.error_rate - 0.3).abs() < f64::EPSILON);
        assert!(config.error_types.contains(&InvalidDataType::Null));
        assert_eq!(config.error_types.len(), 1);
        assert_eq!(config.target_fields, vec!["email"]);
    }

    #[test]
    fn test_parse_error_types() {
        let types = InvalidDataConfig::parse_error_types("missing-field,wrong-type,null").unwrap();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&InvalidDataType::MissingField));
        assert!(types.contains(&InvalidDataType::WrongType));
        assert!(types.contains(&InvalidDataType::Null));
    }

    #[test]
    fn test_parse_error_types_empty() {
        let types = InvalidDataConfig::parse_error_types("").unwrap();
        assert!(types.is_empty());
    }

    #[test]
    fn test_generate_should_invalidate() {
        let code = InvalidDataGenerator::generate_should_invalidate(0.2);
        assert!(code.contains("Math.random() < 0.2"));
        assert!(code.contains("shouldInvalidate"));
    }

    #[test]
    fn test_generate_type_selection() {
        let mut types = HashSet::new();
        types.insert(InvalidDataType::MissingField);
        types.insert(InvalidDataType::Null);

        let code = InvalidDataGenerator::generate_type_selection(&types);
        assert!(code.contains("invalidTypes"));
        assert!(code.contains("Math.random()"));
    }

    #[test]
    fn test_generate_invalidation_logic() {
        let code = InvalidDataGenerator::generate_invalidation_logic();
        assert!(code.contains("function invalidateField"));
        assert!(code.contains("function invalidatePayload"));
        assert!(code.contains("missing-field"));
        assert!(code.contains("wrong-type"));
        assert!(code.contains("out-of-range"));
    }

    #[test]
    fn test_generate_complete_invalidation() {
        let config = InvalidDataConfig::default();
        let code = InvalidDataGenerator::generate_complete_invalidation(&config, "[]");

        assert!(code.contains("shouldInvalidate"));
        assert!(code.contains("invalidType"));
        assert!(code.contains("targetFields"));
        assert!(code.contains("finalPayload"));
    }

    #[test]
    fn test_generate_error_checks() {
        let code = InvalidDataGenerator::generate_error_checks();
        assert!(code.contains("shouldInvalidate"));
        assert!(code.contains("expects error response"));
        assert!(code.contains("status is OK"));
    }
}
