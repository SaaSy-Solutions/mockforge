//! Configuration for the Intelligent Mock Behavior system
//!
//! Re-exported from `mockforge_foundation::intelligent_behavior::config` (Phase 6 / A8).

pub use mockforge_foundation::intelligent_behavior::{
    config::{
        BehaviorModelConfig, IntelligentBehaviorConfig, PerformanceConfig, PersonasConfig,
        VectorStoreConfig,
    },
    Persona,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_defaults() {
        let config = IntelligentBehaviorConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_persona_numeric_trait_range() {
        let persona = Persona {
            name: "test".to_string(),
            traits: {
                let mut m = HashMap::new();
                m.insert("count".to_string(), "10-20".to_string());
                m
            },
        };
        assert_eq!(persona.get_numeric_trait("count"), Some(15));
    }

    #[test]
    fn test_persona_numeric_trait_single() {
        let persona = Persona {
            name: "test".to_string(),
            traits: {
                let mut m = HashMap::new();
                m.insert("count".to_string(), "42".to_string());
                m
            },
        };
        assert_eq!(persona.get_numeric_trait("count"), Some(42));
    }

    #[test]
    fn test_personas_config_active() {
        let personas = PersonasConfig {
            personas: vec![Persona {
                name: "default".to_string(),
                traits: HashMap::new(),
            }],
            active_persona: Some("default".to_string()),
        };
        assert!(personas.get_active_persona().is_some());
    }
}
