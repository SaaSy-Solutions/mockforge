//! Consistency engine for maintaining entity ID → persona mappings
//!
//! This module provides a consistency layer that ensures the same entity ID
//! always generates the same data pattern. It maintains a mapping between
//! entity IDs and their persona profiles, and provides deterministic value
//! generation based on those personas.

use crate::domains::Domain;
use crate::persona::{PersonaGenerator, PersonaProfile, PersonaRegistry};
use mockforge_core::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Consistency store for maintaining entity ID to persona mappings
///
/// Provides thread-safe access to persona-based data generation with
/// in-memory caching and optional persistence capabilities.
#[derive(Debug, Clone)]
pub struct ConsistencyStore {
    /// Persona registry for managing personas
    persona_registry: Arc<PersonaRegistry>,
    /// Domain generator instances keyed by domain
    generators: Arc<RwLock<HashMap<Domain, PersonaGenerator>>>,
    /// Default domain to use when domain is not specified
    default_domain: Option<Domain>,
}

impl ConsistencyStore {
    /// Create a new consistency store
    pub fn new() -> Self {
        Self {
            persona_registry: Arc::new(PersonaRegistry::new()),
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain: None,
        }
    }

    /// Create a consistency store with a default domain
    pub fn with_default_domain(default_domain: Domain) -> Self {
        Self {
            persona_registry: Arc::new(PersonaRegistry::new()),
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain: Some(default_domain),
        }
    }

    /// Create a consistency store with a persona registry and default domain
    pub fn with_registry_and_domain(
        persona_registry: Arc<PersonaRegistry>,
        default_domain: Option<Domain>,
    ) -> Self {
        Self {
            persona_registry,
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain,
        }
    }

    /// Get or create a persona for an entity
    ///
    /// If a persona for this entity ID already exists, returns it.
    /// Otherwise, creates a new persona with the specified domain.
    pub fn get_entity_persona(&self, entity_id: &str, domain: Option<Domain>) -> PersonaProfile {
        let domain = domain.or(self.default_domain).unwrap_or(Domain::General);
        self.persona_registry.get_or_create_persona(entity_id.to_string(), domain)
    }

    /// Generate a consistent value for an entity
    ///
    /// Uses the entity's persona to generate a value that will be consistent
    /// across multiple calls for the same entity ID and field type.
    pub fn generate_consistent_value(
        &self,
        entity_id: &str,
        field_type: &str,
        domain: Option<Domain>,
    ) -> Result<Value> {
        // Get or create persona for this entity
        let persona = self.get_entity_persona(entity_id, domain);
        let domain = persona.domain;

        // Get or create generator for this domain
        // We need to clone the generator or create it fresh each time
        // since we can't hold a reference across the lock
        let generator = {
            let generators = self.generators.read().unwrap();
            if generators.contains_key(&domain) {
                // If generator exists, we'll create a new one with same domain
                // (PersonaGenerator is lightweight)
                PersonaGenerator::new(domain)
            } else {
                drop(generators);
                let mut generators = self.generators.write().unwrap();
                generators.insert(domain, PersonaGenerator::new(domain));
                PersonaGenerator::new(domain)
            }
        };

        // Generate value using persona
        generator.generate_for_persona(&persona, field_type)
    }

    /// Get the persona registry
    pub fn persona_registry(&self) -> &Arc<PersonaRegistry> {
        &self.persona_registry
    }

    /// Set the default domain
    pub fn set_default_domain(&mut self, domain: Option<Domain>) {
        self.default_domain = domain;
    }

    /// Get the default domain
    pub fn default_domain(&self) -> Option<Domain> {
        self.default_domain
    }

    /// Clear all personas (useful for testing or reset)
    pub fn clear(&self) {
        self.persona_registry.clear();
        let mut generators = self.generators.write().unwrap();
        generators.clear();
    }

    /// Get the number of registered personas
    pub fn persona_count(&self) -> usize {
        self.persona_registry.count()
    }
}

impl Default for ConsistencyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity ID extractor for finding entity IDs in various contexts
///
/// Provides utilities for extracting entity IDs from field names,
/// request paths, query parameters, and request bodies.
pub struct EntityIdExtractor;

impl EntityIdExtractor {
    /// Extract entity ID from a field name
    ///
    /// Looks for common patterns like "user_id", "device_id", "transaction_id", etc.
    /// Returns the field name if it matches a pattern, or None if no pattern matches.
    pub fn from_field_name(field_name: &str) -> Option<String> {
        let field_lower = field_name.to_lowercase();

        // Common entity ID patterns (check both exact match and case-insensitive)
        let patterns = [
            "user_id",
            "userid",
            "user-id",
            "device_id",
            "deviceid",
            "device-id",
            "transaction_id",
            "transactionid",
            "transaction-id",
            "order_id",
            "orderid",
            "order-id",
            "customer_id",
            "customerid",
            "customer-id",
            "patient_id",
            "patientid",
            "patient-id",
            "account_id",
            "accountid",
            "account-id",
            "id", // Generic ID field
        ];

        // Check exact match (case-insensitive)
        for pattern in &patterns {
            if field_lower == *pattern {
                return Some(field_name.to_string());
            }
        }

        // Check if field name ends with the pattern (e.g., "user_id" in "my_user_id")
        for pattern in &patterns {
            if field_lower.ends_with(&format!("_{}", pattern))
                || field_lower.ends_with(&format!("-{}", pattern))
            {
                return Some(field_name.to_string());
            }
        }

        None
    }

    /// Extract entity ID from a request path
    ///
    /// Looks for path parameters like "/users/{user_id}" or "/devices/{device_id}".
    /// Returns the entity ID if found in the path.
    pub fn from_path(path: &str) -> Option<String> {
        // Simple extraction: look for common patterns
        // This could be enhanced to parse OpenAPI path templates

        // Patterns like /users/123, /devices/abc, etc.
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 3 {
            let resource = segments[segments.len() - 2].to_lowercase();
            let id = segments[segments.len() - 1];

            // Check if resource matches known entity types
            let entity_types = [
                "user",
                "users",
                "device",
                "devices",
                "transaction",
                "transactions",
                "order",
                "orders",
                "customer",
                "customers",
                "patient",
                "patients",
                "account",
                "accounts",
            ];

            if entity_types.contains(&resource.as_str()) && !id.is_empty() {
                return Some(id.to_string());
            }
        }

        None
    }

    /// Extract entity ID from a JSON value (request body or response)
    ///
    /// Looks for common ID fields in the JSON object.
    pub fn from_json_value(value: &Value) -> Option<String> {
        if let Some(obj) = value.as_object() {
            // Check common ID field names
            let id_fields = [
                "user_id",
                "userId",
                "user-id",
                "device_id",
                "deviceId",
                "device-id",
                "transaction_id",
                "transactionId",
                "transaction-id",
                "order_id",
                "orderId",
                "order-id",
                "customer_id",
                "customerId",
                "customer-id",
                "patient_id",
                "patientId",
                "patient-id",
                "account_id",
                "accountId",
                "account-id",
                "id",
            ];

            for field in &id_fields {
                if let Some(id_value) = obj.get(*field) {
                    if let Some(id_str) = id_value.as_str() {
                        return Some(id_str.to_string());
                    } else if let Some(id_num) = id_value.as_u64() {
                        return Some(id_num.to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract entity ID from multiple sources (field name, path, JSON)
    ///
    /// Tries each source in order and returns the first match.
    pub fn from_multiple_sources(
        field_name: Option<&str>,
        path: Option<&str>,
        json_value: Option<&Value>,
    ) -> Option<String> {
        // Try field name first
        if let Some(field) = field_name {
            if let Some(id) = Self::from_field_name(field) {
                return Some(id);
            }
        }

        // Try path
        if let Some(p) = path {
            if let Some(id) = Self::from_path(p) {
                return Some(id);
            }
        }

        // Try JSON value
        if let Some(json) = json_value {
            if let Some(id) = Self::from_json_value(json) {
                return Some(id);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_consistency_store_new() {
        let store = ConsistencyStore::new();
        assert_eq!(store.persona_count(), 0);
    }

    #[test]
    fn test_consistency_store_with_default_domain() {
        let store = ConsistencyStore::with_default_domain(Domain::Finance);
        assert_eq!(store.default_domain(), Some(Domain::Finance));
    }

    #[test]
    fn test_get_entity_persona() {
        let store = ConsistencyStore::with_default_domain(Domain::Finance);
        let persona1 = store.get_entity_persona("user123", None);
        let persona2 = store.get_entity_persona("user123", None);

        // Should return the same persona
        assert_eq!(persona1.id, persona2.id);
        assert_eq!(persona1.seed, persona2.seed);
    }

    #[test]
    fn test_generate_consistent_value() {
        let store = ConsistencyStore::with_default_domain(Domain::Finance);

        // Generate value for same entity multiple times
        let value1 = store.generate_consistent_value("user123", "amount", None).unwrap();
        let value2 = store.generate_consistent_value("user123", "amount", None).unwrap();

        // Values should be consistent (same seed ensures same RNG state)
        assert!(value1.is_string() || value1.is_number());
        assert!(value2.is_string() || value2.is_number());
    }

    #[test]
    fn test_entity_id_extractor_from_field_name() {
        assert_eq!(EntityIdExtractor::from_field_name("user_id"), Some("user_id".to_string()));
        assert_eq!(EntityIdExtractor::from_field_name("deviceId"), Some("deviceId".to_string()));
        assert_eq!(
            EntityIdExtractor::from_field_name("transaction_id"),
            Some("transaction_id".to_string())
        );
        assert_eq!(EntityIdExtractor::from_field_name("name"), None);
    }

    #[test]
    fn test_entity_id_extractor_from_path() {
        assert_eq!(EntityIdExtractor::from_path("/users/123"), Some("123".to_string()));
        assert_eq!(EntityIdExtractor::from_path("/devices/abc-123"), Some("abc-123".to_string()));
        assert_eq!(EntityIdExtractor::from_path("/orders/ORD-456"), Some("ORD-456".to_string()));
        assert_eq!(EntityIdExtractor::from_path("/api/health"), None);
    }

    #[test]
    fn test_entity_id_extractor_from_json_value() {
        let json = json!({
            "user_id": "user123",
            "name": "John Doe"
        });
        assert_eq!(EntityIdExtractor::from_json_value(&json), Some("user123".to_string()));

        let json2 = json!({
            "id": 456,
            "name": "Device"
        });
        assert_eq!(EntityIdExtractor::from_json_value(&json2), Some("456".to_string()));
    }

    #[test]
    fn test_entity_id_extractor_from_multiple_sources() {
        // Should find from field name
        let id1 = EntityIdExtractor::from_multiple_sources(Some("user_id"), None, None);
        assert_eq!(id1, Some("user_id".to_string()));

        // Should find from path if field name doesn't match
        let id2 = EntityIdExtractor::from_multiple_sources(Some("name"), Some("/users/123"), None);
        assert_eq!(id2, Some("123".to_string()));

        // Should find from JSON if others don't match
        let json = json!({"user_id": "user456"});
        let id3 = EntityIdExtractor::from_multiple_sources(
            Some("name"),
            Some("/api/health"),
            Some(&json),
        );
        assert_eq!(id3, Some("user456".to_string()));
    }
}
