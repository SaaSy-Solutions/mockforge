//! VBR schema extensions
//!
//! This module extends the SchemaDefinition from mockforge-data with VBR-specific
//! metadata including primary keys, foreign keys, indexes, unique constraints,
//! and auto-generation rules.

use mockforge_data::SchemaDefinition;
use serde::{Deserialize, Serialize};
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
    pub fn with_cascade_actions(
        mut self,
        on_delete_a: CascadeAction,
        on_delete_b: CascadeAction,
    ) -> Self {
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
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::SchemaDefinition;

    fn create_test_schema() -> VbrSchemaDefinition {
        let base = SchemaDefinition::new("TestEntity".to_string());
        VbrSchemaDefinition::new(base)
    }

    // CascadeAction tests
    #[test]
    fn test_cascade_action_default() {
        let action = CascadeAction::default();
        assert!(matches!(action, CascadeAction::NoAction));
    }

    #[test]
    fn test_cascade_action_serialize() {
        assert_eq!(serde_json::to_string(&CascadeAction::NoAction).unwrap(), "\"NOACTION\"");
        assert_eq!(serde_json::to_string(&CascadeAction::Cascade).unwrap(), "\"CASCADE\"");
        assert_eq!(serde_json::to_string(&CascadeAction::SetNull).unwrap(), "\"SETNULL\"");
        assert_eq!(serde_json::to_string(&CascadeAction::SetDefault).unwrap(), "\"SETDEFAULT\"");
        assert_eq!(serde_json::to_string(&CascadeAction::Restrict).unwrap(), "\"RESTRICT\"");
    }

    #[test]
    fn test_cascade_action_deserialize() {
        let action: CascadeAction = serde_json::from_str("\"CASCADE\"").unwrap();
        assert!(matches!(action, CascadeAction::Cascade));
    }

    #[test]
    fn test_cascade_action_clone() {
        let action = CascadeAction::Cascade;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_cascade_action_debug() {
        let action = CascadeAction::Restrict;
        let debug = format!("{:?}", action);
        assert!(debug.contains("Restrict"));
    }

    // ForeignKeyDefinition tests
    #[test]
    fn test_foreign_key_definition_clone() {
        let fk = ForeignKeyDefinition {
            field: "user_id".to_string(),
            target_entity: "User".to_string(),
            target_field: "id".to_string(),
            on_delete: CascadeAction::Cascade,
            on_update: CascadeAction::NoAction,
        };

        let cloned = fk.clone();
        assert_eq!(fk.field, cloned.field);
        assert_eq!(fk.target_entity, cloned.target_entity);
    }

    #[test]
    fn test_foreign_key_definition_debug() {
        let fk = ForeignKeyDefinition {
            field: "post_id".to_string(),
            target_entity: "Post".to_string(),
            target_field: "id".to_string(),
            on_delete: CascadeAction::default(),
            on_update: CascadeAction::default(),
        };

        let debug = format!("{:?}", fk);
        assert!(debug.contains("ForeignKeyDefinition"));
        assert!(debug.contains("post_id"));
    }

    #[test]
    fn test_foreign_key_definition_serialize() {
        let fk = ForeignKeyDefinition {
            field: "author_id".to_string(),
            target_entity: "Author".to_string(),
            target_field: "id".to_string(),
            on_delete: CascadeAction::Cascade,
            on_update: CascadeAction::NoAction,
        };

        let json = serde_json::to_string(&fk).unwrap();
        assert!(json.contains("author_id"));
        assert!(json.contains("Author"));
    }

    // ManyToManyDefinition tests
    #[test]
    fn test_many_to_many_definition_new() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string());
        assert_eq!(m2m.entity_a, "User");
        assert_eq!(m2m.entity_b, "Role");
        assert_eq!(m2m.junction_table, Some("role_user".to_string())); // Alphabetical order
        assert_eq!(m2m.entity_a_field, "user_id");
        assert_eq!(m2m.entity_b_field, "role_id");
    }

    #[test]
    fn test_many_to_many_definition_alphabetical_order() {
        // When entity_a comes before entity_b alphabetically
        let m2m1 = ManyToManyDefinition::new("Apple".to_string(), "Banana".to_string());
        assert_eq!(m2m1.junction_table, Some("apple_banana".to_string()));

        // When entity_b comes before entity_a alphabetically
        let m2m2 = ManyToManyDefinition::new("Zebra".to_string(), "Apple".to_string());
        assert_eq!(m2m2.junction_table, Some("apple_zebra".to_string()));
    }

    #[test]
    fn test_many_to_many_definition_with_junction_table() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string())
            .with_junction_table("custom_user_roles".to_string());
        assert_eq!(m2m.junction_table, Some("custom_user_roles".to_string()));
    }

    #[test]
    fn test_many_to_many_definition_with_fields() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string())
            .with_fields("usr_id".to_string(), "role_identifier".to_string());
        assert_eq!(m2m.entity_a_field, "usr_id");
        assert_eq!(m2m.entity_b_field, "role_identifier");
    }

    #[test]
    fn test_many_to_many_definition_with_cascade_actions() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string())
            .with_cascade_actions(CascadeAction::Restrict, CascadeAction::SetNull);
        assert!(matches!(m2m.on_delete_a, CascadeAction::Restrict));
        assert!(matches!(m2m.on_delete_b, CascadeAction::SetNull));
    }

    #[test]
    fn test_many_to_many_definition_clone() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Group".to_string());
        let cloned = m2m.clone();
        assert_eq!(m2m.entity_a, cloned.entity_a);
        assert_eq!(m2m.junction_table, cloned.junction_table);
    }

    #[test]
    fn test_many_to_many_definition_debug() {
        let m2m = ManyToManyDefinition::new("Tag".to_string(), "Post".to_string());
        let debug = format!("{:?}", m2m);
        assert!(debug.contains("ManyToManyDefinition"));
        assert!(debug.contains("Tag"));
    }

    // IndexDefinition tests
    #[test]
    fn test_index_definition_clone() {
        let idx = IndexDefinition {
            name: "idx_email".to_string(),
            fields: vec!["email".to_string()],
            unique: true,
        };

        let cloned = idx.clone();
        assert_eq!(idx.name, cloned.name);
        assert_eq!(idx.unique, cloned.unique);
    }

    #[test]
    fn test_index_definition_debug() {
        let idx = IndexDefinition {
            name: "idx_composite".to_string(),
            fields: vec!["first_name".to_string(), "last_name".to_string()],
            unique: false,
        };

        let debug = format!("{:?}", idx);
        assert!(debug.contains("IndexDefinition"));
        assert!(debug.contains("idx_composite"));
    }

    #[test]
    fn test_index_definition_serialize() {
        let idx = IndexDefinition {
            name: "idx_test".to_string(),
            fields: vec!["field1".to_string()],
            unique: true,
        };

        let json = serde_json::to_string(&idx).unwrap();
        assert!(json.contains("idx_test"));
        assert!(json.contains("\"unique\":true"));
    }

    // UniqueConstraint tests
    #[test]
    fn test_unique_constraint_clone() {
        let constraint = UniqueConstraint {
            name: "uq_email".to_string(),
            fields: vec!["email".to_string()],
        };

        let cloned = constraint.clone();
        assert_eq!(constraint.name, cloned.name);
        assert_eq!(constraint.fields, cloned.fields);
    }

    #[test]
    fn test_unique_constraint_debug() {
        let constraint = UniqueConstraint {
            name: "uq_composite".to_string(),
            fields: vec!["a".to_string(), "b".to_string()],
        };

        let debug = format!("{:?}", constraint);
        assert!(debug.contains("UniqueConstraint"));
    }

    // AutoGenerationRule tests
    #[test]
    fn test_auto_generation_rule_clone() {
        let rule = AutoGenerationRule::Uuid;
        let cloned = rule.clone();
        assert!(matches!(cloned, AutoGenerationRule::Uuid));
    }

    #[test]
    fn test_auto_generation_rule_debug() {
        let rule = AutoGenerationRule::Timestamp;
        let debug = format!("{:?}", rule);
        assert!(debug.contains("Timestamp"));
    }

    #[test]
    fn test_auto_generation_rule_serialize_uuid() {
        let rule = AutoGenerationRule::Uuid;
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("uuid"));
    }

    #[test]
    fn test_auto_generation_rule_serialize_pattern() {
        let rule = AutoGenerationRule::Pattern("USR-{increment:06}".to_string());
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("pattern"));
        assert!(json.contains("USR-{increment:06}"));
    }

    #[test]
    fn test_auto_generation_rule_serialize_realistic() {
        let rule = AutoGenerationRule::Realistic {
            prefix: "cus".to_string(),
            length: 14,
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("realistic"));
        // With adjacently tagged repr, struct fields are nested under "value"
        assert!(json.contains("\"value\":{"));
        assert!(json.contains("\"prefix\":\"cus\""));
        assert!(json.contains("\"length\":14"));
    }

    #[test]
    fn test_auto_generation_rule_all_variants() {
        let rules = vec![
            AutoGenerationRule::AutoIncrement,
            AutoGenerationRule::Uuid,
            AutoGenerationRule::Timestamp,
            AutoGenerationRule::Date,
            AutoGenerationRule::Custom("NOW()".to_string()),
            AutoGenerationRule::Pattern("{uuid}".to_string()),
            AutoGenerationRule::Realistic {
                prefix: "test".to_string(),
                length: 10,
            },
        ];

        for rule in rules {
            let json = serde_json::to_string(&rule).unwrap();
            assert!(!json.is_empty());
        }
    }

    // VbrSchemaDefinition tests
    #[test]
    fn test_vbr_schema_definition_new() {
        let base = SchemaDefinition::new("User".to_string());
        let schema = VbrSchemaDefinition::new(base);

        assert_eq!(schema.primary_key, vec!["id"]);
        assert!(schema.foreign_keys.is_empty());
        assert!(schema.indexes.is_empty());
        assert!(schema.unique_constraints.is_empty());
        assert!(schema.auto_generation.is_empty());
        assert!(schema.many_to_many.is_empty());
    }

    #[test]
    fn test_vbr_schema_definition_with_primary_key() {
        let schema = create_test_schema()
            .with_primary_key(vec!["user_id".to_string(), "role_id".to_string()]);
        assert_eq!(schema.primary_key, vec!["user_id", "role_id"]);
    }

    #[test]
    fn test_vbr_schema_definition_with_foreign_key() {
        let fk = ForeignKeyDefinition {
            field: "user_id".to_string(),
            target_entity: "User".to_string(),
            target_field: "id".to_string(),
            on_delete: CascadeAction::Cascade,
            on_update: CascadeAction::NoAction,
        };

        let schema = create_test_schema().with_foreign_key(fk);
        assert_eq!(schema.foreign_keys.len(), 1);
        assert_eq!(schema.foreign_keys[0].field, "user_id");
    }

    #[test]
    fn test_vbr_schema_definition_with_index() {
        let idx = IndexDefinition {
            name: "idx_email".to_string(),
            fields: vec!["email".to_string()],
            unique: true,
        };

        let schema = create_test_schema().with_index(idx);
        assert_eq!(schema.indexes.len(), 1);
        assert!(schema.indexes[0].unique);
    }

    #[test]
    fn test_vbr_schema_definition_with_unique_constraint() {
        let constraint = UniqueConstraint {
            name: "uq_email".to_string(),
            fields: vec!["email".to_string()],
        };

        let schema = create_test_schema().with_unique_constraint(constraint);
        assert_eq!(schema.unique_constraints.len(), 1);
    }

    #[test]
    fn test_vbr_schema_definition_with_auto_generation() {
        let schema =
            create_test_schema().with_auto_generation("id".to_string(), AutoGenerationRule::Uuid);
        assert!(schema.auto_generation.contains_key("id"));
    }

    #[test]
    fn test_vbr_schema_definition_with_many_to_many() {
        let m2m = ManyToManyDefinition::new("User".to_string(), "Role".to_string());
        let schema = create_test_schema().with_many_to_many(m2m);
        assert_eq!(schema.many_to_many.len(), 1);
    }

    #[test]
    fn test_vbr_schema_definition_builder_chain() {
        let schema = create_test_schema()
            .with_primary_key(vec!["id".to_string()])
            .with_auto_generation("id".to_string(), AutoGenerationRule::Uuid)
            .with_auto_generation("created_at".to_string(), AutoGenerationRule::Timestamp)
            .with_index(IndexDefinition {
                name: "idx_email".to_string(),
                fields: vec!["email".to_string()],
                unique: true,
            })
            .with_unique_constraint(UniqueConstraint {
                name: "uq_username".to_string(),
                fields: vec!["username".to_string()],
            })
            .with_foreign_key(ForeignKeyDefinition {
                field: "org_id".to_string(),
                target_entity: "Organization".to_string(),
                target_field: "id".to_string(),
                on_delete: CascadeAction::SetNull,
                on_update: CascadeAction::NoAction,
            });

        assert_eq!(schema.auto_generation.len(), 2);
        assert_eq!(schema.indexes.len(), 1);
        assert_eq!(schema.unique_constraints.len(), 1);
        assert_eq!(schema.foreign_keys.len(), 1);
    }

    #[test]
    fn test_vbr_schema_definition_clone() {
        let schema = create_test_schema().with_primary_key(vec!["custom_id".to_string()]);

        let cloned = schema.clone();
        assert_eq!(schema.primary_key, cloned.primary_key);
    }

    #[test]
    fn test_vbr_schema_definition_debug() {
        let schema = create_test_schema();
        let debug = format!("{:?}", schema);
        assert!(debug.contains("VbrSchemaDefinition"));
    }
}
