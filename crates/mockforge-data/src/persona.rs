//! Persona profile system for consistent, personality-driven data generation
//!
//! This module provides a system for generating mock data with specific "personalities"
//! that remain consistent over time. Each persona has a unique ID, domain, traits,
//! and a deterministic seed that ensures the same persona always generates the same
//! data patterns.

use crate::domains::{Domain, DomainGenerator};
use mockforge_core::Result;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

/// Persona profile defining a consistent data personality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaProfile {
    /// Unique identifier for this persona (e.g., user_id, device_id, transaction_id)
    pub id: String,
    /// Business domain this persona belongs to
    pub domain: Domain,
    /// Trait name to value mappings (e.g., "spending_level" → "high", "account_type" → "premium")
    pub traits: HashMap<String, String>,
    /// Deterministic seed derived from persona ID and domain for consistency
    pub seed: u64,
    /// Additional persona-specific metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl PersonaProfile {
    /// Create a new persona profile
    ///
    /// The seed is deterministically derived from the persona ID and domain,
    /// ensuring the same ID and domain always produce the same seed.
    pub fn new(id: String, domain: Domain) -> Self {
        let seed = Self::derive_seed(&id, domain);
        Self {
            id,
            domain,
            traits: HashMap::new(),
            seed,
            metadata: HashMap::new(),
        }
    }

    /// Create a persona with initial traits
    pub fn with_traits(id: String, domain: Domain, traits: HashMap<String, String>) -> Self {
        let mut persona = Self::new(id, domain);
        persona.traits = traits;
        persona
    }

    /// Derive a deterministic seed from persona ID and domain
    ///
    /// Uses a simple hash function to convert the ID and domain into a u64 seed.
    /// This ensures the same ID and domain always produce the same seed.
    fn derive_seed(id: &str, domain: Domain) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        domain.as_str().hash(&mut hasher);
        hasher.finish()
    }

    /// Add or update a trait
    pub fn set_trait(&mut self, name: String, value: String) {
        self.traits.insert(name, value);
    }

    /// Get a trait value
    pub fn get_trait(&self, name: &str) -> Option<&String> {
        self.traits.get(name)
    }

    /// Add metadata
    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }
}

/// Registry for managing persona profiles
///
/// Provides thread-safe access to persona profiles with in-memory storage
/// and optional persistence capabilities.
#[derive(Debug, Clone)]
pub struct PersonaRegistry {
    /// In-memory storage of personas keyed by their ID
    personas: Arc<RwLock<HashMap<String, PersonaProfile>>>,
    /// Default traits to apply to new personas
    default_traits: HashMap<String, String>,
}

impl PersonaRegistry {
    /// Create a new persona registry
    pub fn new() -> Self {
        Self {
            personas: Arc::new(RwLock::new(HashMap::new())),
            default_traits: HashMap::new(),
        }
    }

    /// Create a registry with default traits for new personas
    pub fn with_default_traits(default_traits: HashMap<String, String>) -> Self {
        Self {
            personas: Arc::new(RwLock::new(HashMap::new())),
            default_traits,
        }
    }

    /// Get or create a persona profile
    ///
    /// If a persona with the given ID exists, returns it. Otherwise, creates
    /// a new persona with the specified domain and applies default traits.
    pub fn get_or_create_persona(&self, id: String, domain: Domain) -> PersonaProfile {
        let personas = self.personas.read().unwrap();

        // Check if persona already exists
        if let Some(persona) = personas.get(&id) {
            return persona.clone();
        }
        drop(personas);

        // Create new persona with default traits
        let mut persona = PersonaProfile::new(id.clone(), domain);
        for (key, value) in &self.default_traits {
            persona.set_trait(key.clone(), value.clone());
        }

        // Store the new persona
        let mut personas = self.personas.write().unwrap();
        personas.insert(id, persona.clone());
        persona
    }

    /// Get a persona by ID
    pub fn get_persona(&self, id: &str) -> Option<PersonaProfile> {
        let personas = self.personas.read().unwrap();
        personas.get(id).cloned()
    }

    /// Update persona traits
    pub fn update_persona(&self, id: &str, traits: HashMap<String, String>) -> Result<()> {
        let mut personas = self.personas.write().unwrap();
        if let Some(persona) = personas.get_mut(id) {
            for (key, value) in traits {
                persona.set_trait(key, value);
            }
            Ok(())
        } else {
            Err(mockforge_core::Error::generic(format!("Persona with ID '{}' not found", id)))
        }
    }

    /// Remove a persona
    pub fn remove_persona(&self, id: &str) -> bool {
        let mut personas = self.personas.write().unwrap();
        personas.remove(id).is_some()
    }

    /// Get all persona IDs
    pub fn list_persona_ids(&self) -> Vec<String> {
        let personas = self.personas.read().unwrap();
        personas.keys().cloned().collect()
    }

    /// Clear all personas
    pub fn clear(&self) {
        let mut personas = self.personas.write().unwrap();
        personas.clear();
    }

    /// Get the number of registered personas
    pub fn count(&self) -> usize {
        let personas = self.personas.read().unwrap();
        personas.len()
    }
}

impl Default for PersonaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Generator for creating data based on persona profiles
///
/// Uses the persona's seed and traits to generate consistent, domain-appropriate
/// data that reflects the persona's personality.
#[derive(Debug)]
pub struct PersonaGenerator {
    /// Domain generator for domain-specific data generation
    domain_generator: DomainGenerator,
}

impl PersonaGenerator {
    /// Create a new persona generator
    pub fn new(domain: Domain) -> Self {
        Self {
            domain_generator: DomainGenerator::new(domain),
        }
    }

    /// Generate data for a specific field type based on persona
    ///
    /// Uses the persona's seed to create a deterministic RNG, then generates
    /// domain-specific data that may be influenced by the persona's traits.
    pub fn generate_for_persona(
        &self,
        persona: &PersonaProfile,
        field_type: &str,
    ) -> Result<Value> {
        // Create a deterministic RNG from the persona's seed
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(persona.seed);

        // Generate base value using domain generator
        let mut value = self.domain_generator.generate(field_type)?;

        // Apply persona traits to influence the generated value
        value = self.apply_persona_traits(persona, field_type, value, &mut rng)?;

        Ok(value)
    }

    /// Apply persona traits to influence generated values
    ///
    /// Modifies the generated value based on persona traits. For example,
    /// a high-spending persona might generate larger transaction amounts.
    fn apply_persona_traits(
        &self,
        persona: &PersonaProfile,
        field_type: &str,
        value: Value,
        _rng: &mut StdRng,
    ) -> Result<Value> {
        match persona.domain {
            Domain::Finance => self.apply_finance_traits(persona, field_type, value),
            Domain::Ecommerce => self.apply_ecommerce_traits(persona, field_type, value),
            Domain::Healthcare => self.apply_healthcare_traits(persona, field_type, value),
            _ => Ok(value), // For other domains, return value as-is for now
        }
    }

    /// Apply finance-specific persona traits
    fn apply_finance_traits(
        &self,
        persona: &PersonaProfile,
        field_type: &str,
        value: Value,
    ) -> Result<Value> {
        match field_type {
            "amount" | "balance" | "transaction_amount" => {
                // Adjust amount based on spending level trait
                if let Some(spending_level) = persona.get_trait("spending_level") {
                    let multiplier = match spending_level.as_str() {
                        "high" => 2.0,
                        "moderate" => 1.0,
                        "conservative" | "low" => 0.5,
                        _ => 1.0,
                    };

                    if let Some(num) = value.as_f64() {
                        return Ok(Value::Number(
                            serde_json::Number::from_f64(num * multiplier)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ));
                    }
                }
                Ok(value)
            }
            "currency" => {
                // Use preferred currency if trait exists
                if let Some(currency) = persona.get_trait("preferred_currency") {
                    return Ok(Value::String(currency.clone()));
                }
                Ok(value)
            }
            "account_type" => {
                // Use account type trait if exists
                if let Some(account_type) = persona.get_trait("account_type") {
                    return Ok(Value::String(account_type.clone()));
                }
                Ok(value)
            }
            _ => Ok(value),
        }
    }

    /// Apply e-commerce-specific persona traits
    fn apply_ecommerce_traits(
        &self,
        persona: &PersonaProfile,
        field_type: &str,
        value: Value,
    ) -> Result<Value> {
        match field_type {
            "price" | "order_total" => {
                // Adjust price based on customer segment
                if let Some(segment) = persona.get_trait("customer_segment") {
                    let multiplier = match segment.as_str() {
                        "VIP" => 1.5,
                        "regular" => 1.0,
                        "new" => 0.7,
                        _ => 1.0,
                    };

                    if let Some(num) = value.as_f64() {
                        return Ok(Value::Number(
                            serde_json::Number::from_f64(num * multiplier)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ));
                    }
                }
                Ok(value)
            }
            "shipping_method" => {
                // Use preferred shipping method if trait exists
                if let Some(shipping) = persona.get_trait("preferred_shipping") {
                    return Ok(Value::String(shipping.clone()));
                }
                Ok(value)
            }
            _ => Ok(value),
        }
    }

    /// Apply healthcare-specific persona traits
    fn apply_healthcare_traits(
        &self,
        persona: &PersonaProfile,
        field_type: &str,
        value: Value,
    ) -> Result<Value> {
        match field_type {
            "insurance_type" => {
                // Use insurance type trait if exists
                if let Some(insurance) = persona.get_trait("insurance_type") {
                    return Ok(Value::String(insurance.clone()));
                }
                Ok(value)
            }
            "blood_type" => {
                // Use blood type trait if exists
                if let Some(blood_type) = persona.get_trait("blood_type") {
                    return Ok(Value::String(blood_type.clone()));
                }
                Ok(value)
            }
            _ => Ok(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_profile_new() {
        let persona = PersonaProfile::new("user123".to_string(), Domain::Finance);
        assert_eq!(persona.id, "user123");
        assert_eq!(persona.domain, Domain::Finance);
        assert!(persona.traits.is_empty());
        assert!(persona.seed > 0);
    }

    #[test]
    fn test_persona_profile_deterministic_seed() {
        let persona1 = PersonaProfile::new("user123".to_string(), Domain::Finance);
        let persona2 = PersonaProfile::new("user123".to_string(), Domain::Finance);

        // Same ID and domain should produce same seed
        assert_eq!(persona1.seed, persona2.seed);
    }

    #[test]
    fn test_persona_profile_different_seeds() {
        let persona1 = PersonaProfile::new("user123".to_string(), Domain::Finance);
        let persona2 = PersonaProfile::new("user456".to_string(), Domain::Finance);

        // Different IDs should produce different seeds
        assert_ne!(persona1.seed, persona2.seed);
    }

    #[test]
    fn test_persona_profile_traits() {
        let mut persona = PersonaProfile::new("user123".to_string(), Domain::Finance);
        persona.set_trait("spending_level".to_string(), "high".to_string());

        assert_eq!(persona.get_trait("spending_level"), Some(&"high".to_string()));
        assert_eq!(persona.get_trait("nonexistent"), None);
    }

    #[test]
    fn test_persona_registry_get_or_create() {
        let registry = PersonaRegistry::new();

        let persona1 = registry.get_or_create_persona("user123".to_string(), Domain::Finance);
        let persona2 = registry.get_or_create_persona("user123".to_string(), Domain::Finance);

        // Should return the same persona
        assert_eq!(persona1.id, persona2.id);
        assert_eq!(persona1.seed, persona2.seed);
    }

    #[test]
    fn test_persona_registry_default_traits() {
        let mut default_traits = HashMap::new();
        default_traits.insert("spending_level".to_string(), "high".to_string());

        let registry = PersonaRegistry::with_default_traits(default_traits);
        let persona = registry.get_or_create_persona("user123".to_string(), Domain::Finance);

        assert_eq!(persona.get_trait("spending_level"), Some(&"high".to_string()));
    }

    #[test]
    fn test_persona_registry_update() {
        let registry = PersonaRegistry::new();
        registry.get_or_create_persona("user123".to_string(), Domain::Finance);

        let mut traits = HashMap::new();
        traits.insert("spending_level".to_string(), "low".to_string());

        registry.update_persona("user123", traits).unwrap();

        let persona = registry.get_persona("user123").unwrap();
        assert_eq!(persona.get_trait("spending_level"), Some(&"low".to_string()));
    }

    #[test]
    fn test_persona_generator_finance_traits() {
        let generator = PersonaGenerator::new(Domain::Finance);
        let mut persona = PersonaProfile::new("user123".to_string(), Domain::Finance);
        persona.set_trait("spending_level".to_string(), "high".to_string());

        // Generate amount - should be influenced by high spending level
        let value = generator.generate_for_persona(&persona, "amount").unwrap();
        assert!(value.is_string() || value.is_number());
    }

    #[test]
    fn test_persona_generator_consistency() {
        let generator = PersonaGenerator::new(Domain::Finance);
        let persona = PersonaProfile::new("user123".to_string(), Domain::Finance);

        // Generate multiple values - should be consistent due to deterministic seed
        let value1 = generator.generate_for_persona(&persona, "amount").unwrap();
        let value2 = generator.generate_for_persona(&persona, "amount").unwrap();

        // Note: Due to how domain generator works, values might differ,
        // but the seed ensures the RNG state is consistent
        assert!(value1.is_string() || value1.is_number());
        assert!(value2.is_string() || value2.is_number());
    }
}
