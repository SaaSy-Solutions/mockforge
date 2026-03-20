//! Shared Persona types used across modules (openapi, intelligent_behavior, etc.)
//!
//! This module breaks the coupling between `openapi` and `intelligent_behavior`
//! by providing a common location for the `Persona` type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A persona defines consistent data patterns across endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Persona {
    /// Persona name (e.g., "commercial_midwest", "hobbyist_urban")
    pub name: String,

    /// Persona traits (key-value pairs, e.g., "apiary_count": "20-40", "hive_count": "800-1500")
    #[serde(default)]
    pub traits: HashMap<String, String>,
}

impl Persona {
    /// Get a numeric trait value, parsing ranges like "20-40" or single values
    /// Returns the midpoint for ranges, or the value for single numbers
    pub fn get_numeric_trait(&self, key: &str) -> Option<u64> {
        self.traits.get(key).and_then(|value| {
            // Try to parse as range (e.g., "20-40")
            if let Some((min_str, max_str)) = value.split_once('-') {
                if let (Ok(min), Ok(max)) =
                    (min_str.trim().parse::<u64>(), max_str.trim().parse::<u64>())
                {
                    // Return midpoint for ranges
                    return Some((min + max) / 2);
                }
            }
            // Try to parse as single number
            value.parse::<u64>().ok()
        })
    }

    /// Get a trait value as string
    pub fn get_trait(&self, key: &str) -> Option<&String> {
        self.traits.get(key)
    }
}

/// Personas configuration for consistent data generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PersonasConfig {
    /// List of configured personas
    #[serde(default)]
    pub personas: Vec<Persona>,

    /// Active persona name (if None, uses first persona or defaults)
    pub active_persona: Option<String>,
}

impl PersonasConfig {
    /// Get the active persona, or the first persona if no active persona is set
    pub fn get_active_persona(&self) -> Option<&Persona> {
        if let Some(active_name) = &self.active_persona {
            // Find persona by name
            self.personas.iter().find(|p| p.name == *active_name)
        } else if !self.personas.is_empty() {
            // Use first persona as default
            Some(&self.personas[0])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_get_numeric_trait() {
        let mut persona = Persona {
            name: "test".to_string(),
            traits: HashMap::new(),
        };

        // Test range parsing
        persona.traits.insert("hive_count".to_string(), "20-40".to_string());
        assert_eq!(persona.get_numeric_trait("hive_count"), Some(30)); // midpoint

        // Test single value
        persona.traits.insert("apiary_count".to_string(), "50".to_string());
        assert_eq!(persona.get_numeric_trait("apiary_count"), Some(50));

        // Test non-existent trait
        assert_eq!(persona.get_numeric_trait("nonexistent"), None);

        // Test invalid format
        persona.traits.insert("invalid".to_string(), "not-a-number".to_string());
        assert_eq!(persona.get_numeric_trait("invalid"), None);
    }

    #[test]
    fn test_personas_config_get_active_persona() {
        let mut config = PersonasConfig::default();

        // Test with no personas
        assert!(config.get_active_persona().is_none());

        // Test with personas but no active specified (should return first)
        config.personas.push(Persona {
            name: "first".to_string(),
            traits: HashMap::new(),
        });
        config.personas.push(Persona {
            name: "second".to_string(),
            traits: HashMap::new(),
        });
        let active = config.get_active_persona();
        assert!(active.is_some());
        assert_eq!(active.unwrap().name, "first");

        // Test with active persona specified
        config.active_persona = Some("second".to_string());
        let active = config.get_active_persona();
        assert!(active.is_some());
        assert_eq!(active.unwrap().name, "second");

        // Test with invalid active persona name
        config.active_persona = Some("nonexistent".to_string());
        assert!(config.get_active_persona().is_none());
    }
}
