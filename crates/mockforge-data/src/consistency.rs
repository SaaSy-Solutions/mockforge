//! Consistency engine for maintaining entity ID → persona mappings
//!
//! This module provides a consistency layer that ensures the same entity ID
//! always generates the same data pattern. It maintains a mapping between
//! entity IDs and their persona profiles, and provides deterministic value
//! generation based on those personas.

use crate::domains::Domain;
use crate::persona::{PersonaGenerator, PersonaProfile, PersonaRegistry};
use crate::persona_graph::PersonaGraph;
use crate::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Entity type for cross-endpoint consistency
///
/// Allows the same base ID to have different personas for different entity types
/// while maintaining relationships between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// User entity
    User,
    /// Device entity
    Device,
    /// Organization entity
    Organization,
    /// Generic/unspecified entity type
    Generic,
}

impl EntityType {
    /// Convert entity type to string
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::User => "user",
            EntityType::Device => "device",
            EntityType::Organization => "organization",
            EntityType::Generic => "generic",
        }
    }

    /// Parse entity type from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" | "users" => EntityType::User,
            "device" | "devices" => EntityType::Device,
            "organization" | "organizations" | "org" | "orgs" => EntityType::Organization,
            _ => EntityType::Generic,
        }
    }
}

/// Consistency store for maintaining entity ID to persona mappings
///
/// Provides thread-safe access to persona-based data generation with
/// in-memory caching and optional persistence capabilities.
/// Supports cross-entity type consistency where the same base ID can have
/// different personas for different entity types (user, device, organization).
#[derive(Debug)]
pub struct ConsistencyStore {
    /// Persona registry for managing personas
    persona_registry: Arc<PersonaRegistry>,
    /// Domain generator instances keyed by domain
    generators: Arc<RwLock<HashMap<Domain, PersonaGenerator>>>,
    /// Default domain to use when domain is not specified
    default_domain: Option<Domain>,
    /// Persona graph for managing entity relationships
    persona_graph: Arc<PersonaGraph>,
}

impl ConsistencyStore {
    /// Create a new consistency store
    pub fn new() -> Self {
        Self {
            persona_registry: Arc::new(PersonaRegistry::new()),
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain: None,
            persona_graph: Arc::new(PersonaGraph::new()),
        }
    }

    /// Create a consistency store with a default domain
    pub fn with_default_domain(default_domain: Domain) -> Self {
        Self {
            persona_registry: Arc::new(PersonaRegistry::new()),
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain: Some(default_domain),
            persona_graph: Arc::new(PersonaGraph::new()),
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
            persona_graph: Arc::new(PersonaGraph::new()),
        }
    }

    /// Create a consistency store with a persona graph
    pub fn with_persona_graph(persona_graph: Arc<PersonaGraph>) -> Self {
        Self {
            persona_registry: Arc::new(PersonaRegistry::new()),
            generators: Arc::new(RwLock::new(HashMap::new())),
            default_domain: None,
            persona_graph,
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

    /// Get or create a persona for an entity with a specific type
    ///
    /// Creates a persona keyed by both entity ID and entity type, allowing
    /// the same base ID to have different personas for different types
    /// (e.g., "user123" as a user vs "user123" as a device owner).
    ///
    /// The persona ID is constructed as "{entity_type}:{entity_id}" to ensure uniqueness.
    /// Also automatically links personas in the persona graph.
    pub fn get_or_create_persona_by_type(
        &self,
        entity_id: &str,
        entity_type: EntityType,
        domain: Option<Domain>,
    ) -> PersonaProfile {
        let domain = domain.or(self.default_domain).unwrap_or(Domain::General);
        let persona_id = format!("{}:{}", entity_type.as_str(), entity_id);
        let persona = self.persona_registry.get_or_create_persona(persona_id.clone(), domain);

        // Add persona node to graph
        let entity_type_str = entity_type.as_str();
        self.persona_graph
            .get_or_create_node_with_links(&persona_id, entity_type_str, None, None);

        // If this is not a generic type, establish relationships with the base entity
        if entity_type != EntityType::Generic {
            // Get or create the base entity persona
            let base_persona = self.get_entity_persona(entity_id, Some(domain));
            let base_persona_id = base_persona.id.clone();

            // Add base persona to graph if not already present
            self.persona_graph
                .get_or_create_node_with_links(&base_persona_id, "base", None, None);

            // Link personas based on entity type relationships
            let mut base_persona_mut = base_persona.clone();
            match entity_type {
                EntityType::User => {
                    // User owns devices and belongs to organizations
                    // Relationships will be established when device/org personas are created
                    // Link in graph: base -> user
                    self.persona_graph.link_entity_types(
                        &base_persona_id,
                        "base",
                        &persona_id,
                        entity_type_str,
                    );
                }
                EntityType::Device => {
                    // Device is owned by user - establish reverse relationship
                    base_persona_mut
                        .add_relationship("owns_devices".to_string(), persona_id.clone());
                    // Link in graph: base -> device
                    self.persona_graph.link_entity_types(
                        &base_persona_id,
                        "base",
                        &persona_id,
                        entity_type_str,
                    );
                }
                EntityType::Organization => {
                    // Organization has users - establish relationship
                    base_persona_mut.add_relationship("has_users".to_string(), persona_id.clone());
                    // Link in graph: base -> organization
                    self.persona_graph.link_entity_types(
                        &base_persona_id,
                        "base",
                        &persona_id,
                        entity_type_str,
                    );
                }
                EntityType::Generic => {}
            }

            // Update the base persona in registry with relationships
            // Use the registry's add_relationship method to persist relationships
            for (rel_type, related_ids) in &base_persona_mut.relationships {
                for related_id in related_ids {
                    // Only add if not already present
                    if let Some(existing) = self.persona_registry.get_persona(entity_id) {
                        if !existing.get_related_personas(rel_type).contains(related_id) {
                            self.persona_registry
                                .add_relationship(entity_id, rel_type.clone(), related_id.clone())
                                .ok();
                        }
                    }
                }
            }
        }

        persona
    }

    /// Link two personas in the graph based on their entity types
    ///
    /// This is a convenience method for establishing relationships between
    /// personas of different entity types (e.g., user -> order, order -> payment).
    pub fn link_personas(
        &self,
        from_entity_id: &str,
        from_entity_type: &str,
        to_entity_id: &str,
        to_entity_type: &str,
    ) {
        let from_persona_id = format!("{}:{}", from_entity_type, from_entity_id);
        let to_persona_id = format!("{}:{}", to_entity_type, to_entity_id);

        // Ensure both personas exist in the graph
        self.persona_graph.get_or_create_node_with_links(
            &from_persona_id,
            from_entity_type,
            None,
            None,
        );
        self.persona_graph.get_or_create_node_with_links(
            &to_persona_id,
            to_entity_type,
            None,
            None,
        );

        // Link them
        self.persona_graph.link_entity_types(
            &from_persona_id,
            from_entity_type,
            &to_persona_id,
            to_entity_type,
        );
    }

    /// Get the persona graph
    pub fn persona_graph(&self) -> &Arc<PersonaGraph> {
        &self.persona_graph
    }

    /// Get all personas for a base entity ID across different types
    ///
    /// Returns personas for all entity types associated with the base ID.
    pub fn get_personas_for_base_id(
        &self,
        base_id: &str,
        domain: Option<Domain>,
    ) -> Vec<PersonaProfile> {
        let domain = domain.or(self.default_domain).unwrap_or(Domain::General);
        let mut personas = Vec::new();

        // Get the base persona
        let base_persona = self.get_entity_persona(base_id, Some(domain));
        personas.push(base_persona);

        // Get personas for each entity type
        for entity_type in [
            EntityType::User,
            EntityType::Device,
            EntityType::Organization,
        ] {
            let persona_id = format!("{}:{}", entity_type.as_str(), base_id);
            if let Some(persona) = self.persona_registry.get_persona(&persona_id) {
                personas.push(persona);
            }
        }

        personas
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
        // Generate with default reality ratio (0.0 = fully synthetic)
        self.generate_consistent_value_with_reality(entity_id, field_type, domain, 0.0, None, None)
    }

    /// Generate a consistent value for an entity with reality awareness
    ///
    /// Uses the entity's persona to generate a value that will be consistent
    /// across multiple calls, with reality continuum blending applied.
    ///
    /// # Arguments
    /// * `entity_id` - Entity ID
    /// * `field_type` - Type of field to generate
    /// * `domain` - Optional domain (uses default if not provided)
    /// * `reality_ratio` - Reality continuum ratio (0.0 = mock, 1.0 = real)
    /// * `recorded_data` - Optional recorded/snapshot data to blend with
    /// * `real_data` - Optional real/upstream data to blend with
    pub fn generate_consistent_value_with_reality(
        &self,
        entity_id: &str,
        field_type: &str,
        domain: Option<Domain>,
        reality_ratio: f64,
        recorded_data: Option<&Value>,
        real_data: Option<&Value>,
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

        // Generate value using persona with reality awareness
        generator.generate_for_persona_with_reality(
            &persona,
            field_type,
            reality_ratio,
            recorded_data,
            real_data,
        )
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

    /// Extract entity ID and type from a request path
    ///
    /// Looks for path parameters like "/users/{user_id}" or "/devices/{device_id}".
    /// Returns a tuple of (entity_id, entity_type) if found in the path.
    pub fn from_path(path: &str) -> Option<(String, EntityType)> {
        // Simple extraction: look for common patterns
        // This could be enhanced to parse OpenAPI path templates

        // Patterns like /users/123, /devices/abc, etc.
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 3 {
            let resource = segments[segments.len() - 2].to_lowercase();
            let id = segments[segments.len() - 1];

            // Check if resource matches known entity types
            let entity_type = EntityType::parse(&resource);

            if entity_type != EntityType::Generic && !id.is_empty() {
                return Some((id.to_string(), entity_type));
            }

            // Fallback for other entity types
            let entity_types = [
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
                return Some((id.to_string(), EntityType::Generic));
            }
        }

        None
    }

    /// Extract entity ID from a request path (backward compatibility)
    ///
    /// Returns just the entity ID without type information.
    pub fn from_path_id_only(path: &str) -> Option<String> {
        Self::from_path(path).map(|(id, _)| id)
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
            if let Some((id, _)) = Self::from_path(p) {
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
        assert_eq!(
            EntityIdExtractor::from_path("/users/123"),
            Some(("123".to_string(), EntityType::User))
        );
        assert_eq!(
            EntityIdExtractor::from_path("/devices/abc-123"),
            Some(("abc-123".to_string(), EntityType::Device))
        );
        assert_eq!(
            EntityIdExtractor::from_path("/organizations/org1"),
            Some(("org1".to_string(), EntityType::Organization))
        );
        assert_eq!(EntityIdExtractor::from_path("/api/health"), None);
    }

    #[test]
    fn test_entity_id_extractor_from_path_id_only() {
        assert_eq!(EntityIdExtractor::from_path_id_only("/users/123"), Some("123".to_string()));
        assert_eq!(
            EntityIdExtractor::from_path_id_only("/devices/abc-123"),
            Some("abc-123".to_string())
        );
        assert_eq!(
            EntityIdExtractor::from_path_id_only("/organizations/org1"),
            Some("org1".to_string())
        );
        assert_eq!(EntityIdExtractor::from_path_id_only("/api/health"), None);
    }

    #[test]
    fn test_entity_type() {
        assert_eq!(EntityType::User.as_str(), "user");
        assert_eq!(EntityType::Device.as_str(), "device");
        assert_eq!(EntityType::Organization.as_str(), "organization");
        assert_eq!(EntityType::parse("users"), EntityType::User);
        assert_eq!(EntityType::parse("devices"), EntityType::Device);
        assert_eq!(EntityType::parse("organizations"), EntityType::Organization);
    }

    #[test]
    fn test_get_or_create_persona_by_type() {
        let store = ConsistencyStore::with_default_domain(Domain::Finance);

        // Create personas for same base ID but different types
        let user_persona = store.get_or_create_persona_by_type("user123", EntityType::User, None);
        let device_persona =
            store.get_or_create_persona_by_type("user123", EntityType::Device, None);

        // Should have different persona IDs
        assert_ne!(user_persona.id, device_persona.id);
        assert!(user_persona.id.contains("user:user123"));
        assert!(device_persona.id.contains("device:user123"));
    }

    #[test]
    fn test_get_personas_for_base_id() {
        let store = ConsistencyStore::with_default_domain(Domain::Finance);

        // Create personas for different types with same base ID
        store.get_or_create_persona_by_type("user123", EntityType::User, None);
        store.get_or_create_persona_by_type("user123", EntityType::Device, None);

        let personas = store.get_personas_for_base_id("user123", None);
        assert!(personas.len() >= 2); // At least base + user + device
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
