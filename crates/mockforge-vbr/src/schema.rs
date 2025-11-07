//! VBR schema extensions
//!
//! This module extends the SchemaDefinition from mockforge-data with VBR-specific
//! metadata including primary keys, foreign keys, indexes, unique constraints,
//! and auto-generation rules.

use mockforge_data::SchemaDefinition;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// VBR-specific schema metadata that extends SchemaDefinition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VbrSchemaDefinition {
    /// Base schema definition from mockforge-data
    #[serde(flatten)]
    pub base: SchemaDefinition,

    /// Primary key field name(s)
    pub primary_key: Vec<String>,

    /// Foreign key relationships
    pub foreign_keys: Vec<ForeignKeyDefinition>,

    /// Index definitions
    pub indexes: Vec<IndexDefinition>,

    /// Unique constraints
    pub unique_constraints: Vec<UniqueConstraint>,

    /// Auto-generation rules for fields
    pub auto_generation: HashMap<String, AutoGenerationRule>,

    /// Many-to-many relationships
    pub many_to_many: Vec<ManyToManyDefinition>,
}

/// Foreign key relationship definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyDefinition {
    /// Field name in this entity
    pub field: String,

    /// Target entity name
    pub target_entity: String,

    /// Target field name (usually "id")
    pub target_field: String,

    /// Cascade action on delete
    #[serde(default)]
    pub on_delete: CascadeAction,

    /// Cascade action on update
    #[serde(default)]
    pub on_update: CascadeAction,
}

/// Many-to-many relationship definition
///
/// Represents a many-to-many relationship between two entities using a junction table.
/// For example, Users and Roles with a user_roles junction table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManyToManyDefinition {
    /// First entity name
    pub entity_a: String,

    /// Second entity name
    pub entity_b: String,

    /// Junction table name (auto-generated if not provided)
    /// Format: "{entity_a}_{entity_b}" or "{entity_b}_{entity_a}" (alphabetically sorted)
    pub junction_table: Option<String>,

    /// Foreign key field name in junction table pointing to entity_a
    pub entity_a_field: String,

    /// Foreign key field name in junction table pointing to entity_b
    pub entity_b_field: String,

    /// Cascade action on delete for entity_a
    #[serde(default)]
    pub on_delete_a: CascadeAction,

    /// Cascade action on delete for entity_b
    #[serde(default)]
    pub on_delete_b: CascadeAction,
}

impl ManyToManyDefinition {
    /// Create a new many-to-many relationship definition
    pub fn new(entity_a: String, entity_b: String) -> Self {
        // Auto-generate junction table name (alphabetically sorted)
        let (field_a, field_b) = if entity_a.to_lowercase() < entity_b.to_lowercase() {
            (
                format!("{}_id", entity_a.to_lowercase()),
                format!("{}_id", entity_b.to_lowercase()),
            )
        } else {
            (
                format!("{}_id", entity_b.to_lowercase()),
                format!("{}_id", entity_a.to_lowercase()),
            )
        };

        let junction_table = if entity_a.to_lowercase() < entity_b.to_lowercase() {
            Some(format!("{}_{}", entity_a.to_lowercase(), entity_b.to_lowercase()))
        } else {
            Some(format!("{}_{}", entity_b.to_lowercase(), entity_a.to_lowercase()))
        };

        Self {
            entity_a: entity_a.clone(),
            entity_b: entity_b.clone(),
            junction_table,
            entity_a_field: format!("{}_id", entity_a.to_lowercase()),
            entity_b_field: format!("{}_id", entity_b.to_lowercase()),
            on_delete_a: CascadeAction::Cascade,
            on_delete_b: CascadeAction::Cascade,
        }
    }

    /// Set the junction table name
    pub fn with_junction_table(mut self, table_name: String) -> Self {
        self.junction_table = Some(table_name);
        self
    }

    /// Set the foreign key field names
    pub fn with_fields(mut self, entity_a_field: String, entity_b_field: String) -> Self {
        self.entity_a_field = entity_a_field;
        self.entity_b_field = entity_b_field;
        self
    }

    /// Set cascade actions
    pub fn with_cascade_actions(mut self, on_delete_a: CascadeAction, on_delete_b: CascadeAction) -> Self {
        self.on_delete_a = on_delete_a;
        self.on_delete_b = on_delete_b;
        self
    }
}

/// Cascade action for foreign keys
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum CascadeAction {
    /// No action
    #[default]
    NoAction,
    /// Cascade (delete/update related records)
    Cascade,
    /// Set null
    SetNull,
    /// Set default
    SetDefault,
    /// Restrict (prevent if related records exist)
    Restrict,
}

/// Index definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// Index name
    pub name: String,

    /// Fields included in the index
    pub fields: Vec<String>,

    /// Whether the index is unique
    #[serde(default)]
    pub unique: bool,
}

/// Unique constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
    /// Constraint name
    pub name: String,

    /// Fields that must be unique together
    pub fields: Vec<String>,
}

/// Auto-generation rule for fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutoGenerationRule {
    /// Auto-incrementing integer
    AutoIncrement,
    /// UUID generation
    Uuid,
    /// Current timestamp
    Timestamp,
    /// Current date
    Date,
    /// Custom function/expression
    Custom(String),
    /// Pattern-based ID generation
    ///
    /// Supports template variables:
    /// - `{increment}` or `{increment:06}` - Auto-incrementing number with padding
    /// - `{timestamp}` - Unix timestamp
    /// - `{random}` - Random alphanumeric string
    /// - `{uuid}` - UUID v4
    ///
    /// Examples:
    /// - "USR-{increment:06}" -> "USR-000001"
    /// - "ORD-{timestamp}" -> "ORD-1704067200"
    Pattern(String),
    /// Realistic-looking ID generation (Stripe-style)
    ///
    /// Generates IDs in the format: `{prefix}_{random_alphanumeric}`
    ///
    /// # Arguments
    /// * `prefix` - Prefix for the ID (e.g., "cus", "ord")
    /// * `length` - Total length of the random part (excluding prefix and underscore)
    Realistic {
        /// Prefix for the ID
        prefix: String,
        /// Length of the random alphanumeric part
        length: usize,
    },
}

impl VbrSchemaDefinition {
    /// Create a new VBR schema definition from a base schema
    pub fn new(base: SchemaDefinition) -> Self {
        Self {
            base,
            primary_key: vec!["id".to_string()], // Default primary key
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            unique_constraints: Vec::new(),
            auto_generation: HashMap::new(),
            many_to_many: Vec::new(),
        }
    }

    /// Set the primary key field(s)
    pub fn with_primary_key(mut self, fields: Vec<String>) -> Self {
        self.primary_key = fields;
        self
    }

    /// Add a foreign key relationship
    pub fn with_foreign_key(mut self, fk: ForeignKeyDefinition) -> Self {
        self.foreign_keys.push(fk);
        self
    }

    /// Add an index
    pub fn with_index(mut self, index: IndexDefinition) -> Self {
        self.indexes.push(index);
        self
    }

    /// Add a unique constraint
    pub fn with_unique_constraint(mut self, constraint: UniqueConstraint) -> Self {
        self.unique_constraints.push(constraint);
        self
    }

    /// Set auto-generation rule for a field
    pub fn with_auto_generation(mut self, field: String, rule: AutoGenerationRule) -> Self {
        self.auto_generation.insert(field, rule);
        self
    }

    /// Add a many-to-many relationship
    pub fn with_many_to_many(mut self, m2m: ManyToManyDefinition) -> Self {
        self.many_to_many.push(m2m);
        self
    }
}
