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
}
