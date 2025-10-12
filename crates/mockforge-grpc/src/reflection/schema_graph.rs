//! Schema relationship graph extraction
//!
//! This module extracts relationship graphs from proto and OpenAPI schemas,
//! identifying foreign keys, references, and data dependencies for coherent
//! synthetic data generation.

use prost_reflect::{DescriptorPool, FieldDescriptor, Kind, MessageDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// A graph representing relationships between schema entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaGraph {
    /// All entities (messages/schemas) in the graph
    pub entities: HashMap<String, EntityNode>,
    /// Direct relationships between entities
    pub relationships: Vec<Relationship>,
    /// Detected foreign key patterns
    pub foreign_keys: HashMap<String, Vec<ForeignKeyMapping>>,
}

/// An entity node in the schema graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNode {
    /// Entity name (e.g., "User", "Order")
    pub name: String,
    /// Full qualified name (e.g., "com.example.User")
    pub full_name: String,
    /// Fields in this entity
    pub fields: Vec<FieldInfo>,
    /// Whether this is a root entity (not referenced by others)
    pub is_root: bool,
    /// Entities that reference this one
    pub referenced_by: Vec<String>,
    /// Entities that this one references
    pub references: Vec<String>,
}

/// Information about a field in an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// Field name
    pub name: String,
    /// Field type (string, int32, message, etc.)
    pub field_type: String,
    /// Whether this field is a potential foreign key
    pub is_foreign_key: bool,
    /// Target entity if this is a foreign key
    pub foreign_key_target: Option<String>,
    /// Whether this field is required
    pub is_required: bool,
    /// Constraints on this field
    pub constraints: HashMap<String, String>,
}

/// A relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Source entity name
    pub from_entity: String,
    /// Target entity name
    pub to_entity: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Field name that creates the relationship
    pub field_name: String,
    /// Whether this relationship is required
    pub is_required: bool,
    /// Cardinality constraints
    pub cardinality: Cardinality,
}

/// Type of relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Direct foreign key reference (user_id -> User)
    ForeignKey,
    /// Embedded object (address within user)
    Embedded,
    /// Array/repeated field relationship
    OneToMany,
    /// Bidirectional relationship
    ManyToMany,
    /// Composition relationship
    Composition,
}

/// Cardinality constraints for relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cardinality {
    /// Minimum number of related entities
    pub min: u32,
    /// Maximum number of related entities (None = unlimited)
    pub max: Option<u32>,
}

/// Foreign key mapping detected via naming conventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyMapping {
    /// Field name (e.g., "user_id")
    pub field_name: String,
    /// Target entity name (e.g., "User")
    pub target_entity: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Detection method used
    pub detection_method: ForeignKeyDetectionMethod,
}

/// Methods used to detect foreign key relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForeignKeyDetectionMethod {
    /// Detected via naming convention (user_id -> User)
    NamingConvention,
    /// Detected via schema reference ($ref in OpenAPI)
    SchemaReference,
    /// Detected via field type (message type in proto)
    MessageType,
    /// Detected via constraint annotation
    Constraint,
}

/// Schema graph extractor for protobuf schemas
pub struct ProtoSchemaGraphExtractor {
    /// Common foreign key patterns
    foreign_key_patterns: Vec<ForeignKeyPattern>,
}

/// Pattern for detecting foreign keys via naming
#[derive(Debug, Clone)]
struct ForeignKeyPattern {
    /// Regex pattern for field names
    pattern: regex::Regex,
    /// How to extract entity name from field name
    entity_extraction: EntityExtractionMethod,
    /// Confidence score for this pattern
    #[allow(dead_code)] // Used in future relationship analysis
    confidence: f64,
}

/// Methods for extracting entity names from field names
#[derive(Debug, Clone)]
enum EntityExtractionMethod {
    /// Remove suffix (user_id -> user)
    RemoveSuffix(String),
    /// Direct mapping
    #[allow(dead_code)] // Used in future entity extraction
    Direct,
    /// Custom transform function
    #[allow(dead_code)] // Used in future entity extraction
    Custom(fn(&str) -> Option<String>),
}

impl ProtoSchemaGraphExtractor {
    /// Create a new proto schema graph extractor
    pub fn new() -> Self {
        let patterns = vec![
            ForeignKeyPattern {
                pattern: regex::Regex::new(r"^(.+)_id$").unwrap(),
                entity_extraction: EntityExtractionMethod::RemoveSuffix("_id".to_string()),
                confidence: 0.9,
            },
            ForeignKeyPattern {
                pattern: regex::Regex::new(r"^(.+)Id$").unwrap(),
                entity_extraction: EntityExtractionMethod::RemoveSuffix("Id".to_string()),
                confidence: 0.85,
            },
            ForeignKeyPattern {
                pattern: regex::Regex::new(r"^(.+)_ref$").unwrap(),
                entity_extraction: EntityExtractionMethod::RemoveSuffix("_ref".to_string()),
                confidence: 0.8,
            },
        ];

        Self {
            foreign_key_patterns: patterns,
        }
    }

    /// Extract schema graph from protobuf descriptor pool
    pub fn extract_from_proto(
        &self,
        pool: &DescriptorPool,
    ) -> Result<SchemaGraph, Box<dyn std::error::Error + Send + Sync>> {
        let mut entities = HashMap::new();
        let mut relationships = Vec::new();
        let mut foreign_keys = HashMap::new();

        info!("Extracting schema graph from protobuf descriptors");

        // First pass: Extract all entities and their fields
        for message_descriptor in pool.all_messages() {
            let entity = self.extract_entity_from_message(&message_descriptor)?;
            entities.insert(entity.name.clone(), entity);
        }

        // Second pass: Analyze relationships and foreign keys
        for (entity_name, entity) in &entities {
            let fk_mappings = self.detect_foreign_keys(entity, &entities)?;
            if !fk_mappings.is_empty() {
                foreign_keys.insert(entity_name.clone(), fk_mappings);
            }

            let entity_relationships = self.extract_relationships(entity, &entities)?;
            relationships.extend(entity_relationships);
        }

        // Third pass: Update cross-references
        let mut updated_entities = entities;
        self.update_cross_references(&mut updated_entities, &relationships);

        let graph = SchemaGraph {
            entities: updated_entities,
            relationships,
            foreign_keys,
        };

        info!(
            "Extracted schema graph with {} entities and {} relationships",
            graph.entities.len(),
            graph.relationships.len()
        );

        Ok(graph)
    }

    /// Extract an entity from a proto message descriptor
    fn extract_entity_from_message(
        &self,
        descriptor: &MessageDescriptor,
    ) -> Result<EntityNode, Box<dyn std::error::Error + Send + Sync>> {
        let name = Self::extract_entity_name(descriptor.name());
        let full_name = descriptor.full_name().to_string();

        let mut fields = Vec::new();
        for field_descriptor in descriptor.fields() {
            let field_info = self.extract_field_info(&field_descriptor)?;
            fields.push(field_info);
        }

        Ok(EntityNode {
            name,
            full_name,
            fields,
            is_root: true, // Will be updated later
            referenced_by: Vec::new(),
            references: Vec::new(),
        })
    }

    /// Extract field information from a proto field descriptor
    fn extract_field_info(
        &self,
        field: &FieldDescriptor,
    ) -> Result<FieldInfo, Box<dyn std::error::Error + Send + Sync>> {
        let name = field.name().to_string();
        let field_type = Self::kind_to_string(&field.kind());
        let is_required = true; // Proto fields are required by default unless marked optional

        // Check if this looks like a foreign key
        let (is_foreign_key, foreign_key_target) =
            self.analyze_potential_foreign_key(&name, &field.kind());

        let mut constraints = HashMap::new();
        if field.is_list() {
            constraints.insert("repeated".to_string(), "true".to_string());
        }

        Ok(FieldInfo {
            name,
            field_type,
            is_foreign_key,
            foreign_key_target,
            is_required,
            constraints,
        })
    }

    /// Analyze if a field might be a foreign key
    fn analyze_potential_foreign_key(
        &self,
        field_name: &str,
        kind: &Kind,
    ) -> (bool, Option<String>) {
        // Check naming patterns
        for pattern in &self.foreign_key_patterns {
            if pattern.pattern.is_match(field_name) {
                if let Some(entity_name) = self.extract_entity_name_from_field(field_name, pattern)
                {
                    return (true, Some(entity_name));
                }
            }
        }

        // Check if it's a message type (embedded relationship)
        if let Kind::Message(message_descriptor) = kind {
            let entity_name = Self::extract_entity_name(message_descriptor.name());
            return (false, Some(entity_name)); // Not FK, but related entity
        }

        (false, None)
    }

    /// Extract entity name from field name using pattern
    fn extract_entity_name_from_field(
        &self,
        field_name: &str,
        pattern: &ForeignKeyPattern,
    ) -> Option<String> {
        match &pattern.entity_extraction {
            EntityExtractionMethod::RemoveSuffix(suffix) => {
                if field_name.ends_with(suffix) {
                    let base_name = &field_name[..field_name.len() - suffix.len()];
                    Some(Self::normalize_entity_name(base_name))
                } else {
                    None
                }
            }
            EntityExtractionMethod::Direct => Some(Self::normalize_entity_name(field_name)),
            EntityExtractionMethod::Custom(func) => func(field_name),
        }
    }

    /// Detect foreign keys in an entity
    fn detect_foreign_keys(
        &self,
        entity: &EntityNode,
        all_entities: &HashMap<String, EntityNode>,
    ) -> Result<Vec<ForeignKeyMapping>, Box<dyn std::error::Error + Send + Sync>> {
        let mut mappings = Vec::new();

        for field in &entity.fields {
            if field.is_foreign_key {
                if let Some(target) = &field.foreign_key_target {
                    // Check if target entity exists
                    if all_entities.contains_key(target) {
                        mappings.push(ForeignKeyMapping {
                            field_name: field.name.clone(),
                            target_entity: target.clone(),
                            confidence: 0.9, // High confidence for detected patterns
                            detection_method: ForeignKeyDetectionMethod::NamingConvention,
                        });
                    }
                }
            }
        }

        Ok(mappings)
    }

    /// Extract relationships from an entity
    fn extract_relationships(
        &self,
        entity: &EntityNode,
        all_entities: &HashMap<String, EntityNode>,
    ) -> Result<Vec<Relationship>, Box<dyn std::error::Error + Send + Sync>> {
        let mut relationships = Vec::new();

        for field in &entity.fields {
            if let Some(target_entity) = &field.foreign_key_target {
                if all_entities.contains_key(target_entity) {
                    let relationship_type = if field.is_foreign_key {
                        RelationshipType::ForeignKey
                    } else if field.field_type.contains("message") {
                        RelationshipType::Embedded
                    } else {
                        RelationshipType::Composition
                    };

                    let cardinality = if field.constraints.contains_key("repeated") {
                        Cardinality { min: 0, max: None }
                    } else {
                        Cardinality {
                            min: if field.is_required { 1 } else { 0 },
                            max: Some(1),
                        }
                    };

                    relationships.push(Relationship {
                        from_entity: entity.name.clone(),
                        to_entity: target_entity.clone(),
                        relationship_type,
                        field_name: field.name.clone(),
                        is_required: field.is_required,
                        cardinality,
                    });
                }
            }
        }

        Ok(relationships)
    }

    /// Update cross-references between entities
    fn update_cross_references(
        &self,
        entities: &mut HashMap<String, EntityNode>,
        relationships: &[Relationship],
    ) {
        // Build reference maps
        let mut referenced_by_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut references_map: HashMap<String, Vec<String>> = HashMap::new();

        for rel in relationships {
            // Track what references what
            references_map
                .entry(rel.from_entity.clone())
                .or_default()
                .push(rel.to_entity.clone());

            // Track what is referenced by what
            referenced_by_map
                .entry(rel.to_entity.clone())
                .or_default()
                .push(rel.from_entity.clone());
        }

        // Update entities
        for (entity_name, entity) in entities.iter_mut() {
            if let Some(refs) = references_map.get(entity_name) {
                entity.references = refs.clone();
            }

            if let Some(referenced_by) = referenced_by_map.get(entity_name) {
                entity.referenced_by = referenced_by.clone();
                entity.is_root = false; // Referenced entities are not root
            }
        }
    }

    /// Convert protobuf Kind to string representation
    fn kind_to_string(kind: &Kind) -> String {
        match kind {
            Kind::String => "string".to_string(),
            Kind::Int32 => "int32".to_string(),
            Kind::Int64 => "int64".to_string(),
            Kind::Uint32 => "uint32".to_string(),
            Kind::Uint64 => "uint64".to_string(),
            Kind::Bool => "bool".to_string(),
            Kind::Float => "float".to_string(),
            Kind::Double => "double".to_string(),
            Kind::Bytes => "bytes".to_string(),
            Kind::Message(msg) => format!("message:{}", msg.full_name()),
            Kind::Enum(enum_desc) => format!("enum:{}", enum_desc.full_name()),
            _ => "unknown".to_string(),
        }
    }

    /// Extract entity name from message name (remove package, normalize)
    fn extract_entity_name(message_name: &str) -> String {
        Self::normalize_entity_name(message_name)
    }

    /// Normalize entity name (PascalCase, singular)
    fn normalize_entity_name(name: &str) -> String {
        // Convert snake_case to PascalCase
        name.split('_')
            .map(|part| {
                let mut chars: Vec<char> = part.chars().collect();
                if let Some(first_char) = chars.first_mut() {
                    *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
                }
                chars.into_iter().collect::<String>()
            })
            .collect::<String>()
    }
}

impl Default for ProtoSchemaGraphExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foreign_key_pattern_matching() {
        let extractor = ProtoSchemaGraphExtractor::new();

        // Test standard patterns
        let (is_fk, target) = extractor.analyze_potential_foreign_key("user_id", &Kind::Int32);
        assert!(is_fk);
        assert_eq!(target, Some("User".to_string()));

        let (is_fk, target) = extractor.analyze_potential_foreign_key("orderId", &Kind::Int64);
        assert!(is_fk);
        assert_eq!(target, Some("Order".to_string()));
    }

    #[test]
    fn test_entity_name_normalization() {
        assert_eq!(ProtoSchemaGraphExtractor::normalize_entity_name("user"), "User");
        assert_eq!(ProtoSchemaGraphExtractor::normalize_entity_name("order_item"), "OrderItem");
        assert_eq!(
            ProtoSchemaGraphExtractor::normalize_entity_name("ProductCategory"),
            "ProductCategory"
        );
    }
}
