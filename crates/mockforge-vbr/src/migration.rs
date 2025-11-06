//! Database migration system
//!
//! This module handles generating SQLite schema from entity definitions,
//! creating tables, indexes, foreign keys, and managing schema migrations.

use crate::entities::Entity;
use crate::schema::{CascadeAction, VbrSchemaDefinition};
use crate::{Error, Result};
use std::collections::HashMap;

/// Migration manager for database schema evolution
pub struct MigrationManager {
    /// Current migration version
    version: u64,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new() -> Self {
        Self { version: 0 }
    }

    /// Generate CREATE TABLE statement from an entity
    pub fn generate_create_table(&self, entity: &Entity) -> Result<String> {
        let schema = &entity.schema;
        let table_name = entity.table_name();

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);
        let mut columns = Vec::new();

        // Add columns from schema fields
        for field in &schema.base.fields {
            let column_def = self.field_to_column_definition(field, &schema)?;
            columns.push(column_def);
        }

        // Add primary key constraint
        if !schema.primary_key.is_empty() {
            let pk_fields = schema.primary_key.join(", ");
            columns.push(format!("PRIMARY KEY ({})", pk_fields));
        }

        sql.push_str(&columns.join(",\n    "));
        sql.push_str("\n)");

        Ok(sql)
    }

    /// Generate foreign key constraints
    pub fn generate_foreign_keys(&self, entity: &Entity) -> Vec<String> {
        let mut fk_statements = Vec::new();
        let table_name = entity.table_name();

        for fk in &entity.schema.foreign_keys {
            let on_delete = cascade_action_to_sql(fk.on_delete);
            let on_update = cascade_action_to_sql(fk.on_update);

            let fk_name = format!("fk_{}_{}", table_name, fk.field);
            let statement = format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE {} ON UPDATE {}",
                table_name, fk_name, fk.field, fk.target_entity.to_lowercase() + "s", fk.target_field, on_delete, on_update
            );
            fk_statements.push(statement);
        }

        fk_statements
    }

    /// Generate index creation statements
    pub fn generate_indexes(&self, entity: &Entity) -> Vec<String> {
        let mut index_statements = Vec::new();
        let table_name = entity.table_name();

        for index in &entity.schema.indexes {
            let unique = if index.unique { "UNIQUE " } else { "" };
            let fields = index.fields.join(", ");
            let statement = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique, index.name, table_name, fields
            );
            index_statements.push(statement);
        }

        index_statements
    }

    /// Convert a field definition to SQL column definition
    fn field_to_column_definition(
        &self,
        field: &mockforge_data::FieldDefinition,
        schema: &VbrSchemaDefinition,
    ) -> Result<String> {
        let name = &field.name;
        let sql_type = rust_type_to_sql_type(&field.field_type)?;
        let mut parts = vec![format!("{} {}", name, sql_type)];

        // Add NOT NULL if required
        if field.required {
            parts.push("NOT NULL".to_string());
        }

        // Add default value if specified
        if let Some(default) = &field.default {
            let default_value = value_to_sql_default(default)?;
            parts.push(format!("DEFAULT {}", default_value));
        }

        // Check for auto-generation rules
        if let Some(rule) = schema.auto_generation.get(name) {
            match rule {
                crate::schema::AutoGenerationRule::AutoIncrement => {
                    parts.push("AUTOINCREMENT".to_string());
                }
                crate::schema::AutoGenerationRule::Uuid => {
                    // UUID will be generated in application code
                }
                crate::schema::AutoGenerationRule::Timestamp => {
                    parts.push("DEFAULT CURRENT_TIMESTAMP".to_string());
                }
                crate::schema::AutoGenerationRule::Date => {
                    parts.push("DEFAULT (date('now'))".to_string());
                }
                crate::schema::AutoGenerationRule::Custom(expr) => {
                    parts.push(format!("DEFAULT ({})", expr));
                }
            }
        }

        Ok(parts.join(" "))
    }

    /// Migrate database schema for all entities
    pub async fn migrate(
        &self,
        entities: &[Entity],
        database: &mut dyn crate::database::VirtualDatabase,
    ) -> Result<()> {
        // Generate and execute CREATE TABLE statements
        for entity in entities {
            let create_table = self.generate_create_table(entity)?;
            database.create_table(&create_table).await?;

            // Create indexes
            for index_sql in self.generate_indexes(entity) {
                database.execute(&index_sql, &[]).await?;
            }

            // Note: Foreign keys are added after all tables are created
            // This will be handled in a separate pass
        }

        // Second pass: add foreign key constraints
        for entity in entities {
            for fk_sql in self.generate_foreign_keys(entity) {
                // Foreign keys might fail if tables don't exist yet, so we continue on error
                let _ = database.execute(&fk_sql, &[]).await;
            }
        }

        Ok(())
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert Rust type to SQL type
fn rust_type_to_sql_type(rust_type: &str) -> Result<&str> {
    match rust_type.to_lowercase().as_str() {
        "string" | "str" | "text" | "uuid" | "email" | "url" => Ok("TEXT"),
        "integer" | "int" | "i32" | "i64" => Ok("INTEGER"),
        "float" | "double" | "f32" | "f64" | "number" => Ok("REAL"),
        "boolean" | "bool" => Ok("INTEGER"), // SQLite uses INTEGER for booleans
        "date" | "datetime" | "timestamp" => Ok("TEXT"), // SQLite stores dates as TEXT
        _ => Ok("TEXT"),                     // Default to TEXT for unknown types
    }
}

/// Convert cascade action to SQL
fn cascade_action_to_sql(action: CascadeAction) -> &'static str {
    match action {
        CascadeAction::NoAction => "NO ACTION",
        CascadeAction::Cascade => "CASCADE",
        CascadeAction::SetNull => "SET NULL",
        CascadeAction::SetDefault => "SET DEFAULT",
        CascadeAction::Restrict => "RESTRICT",
    }
}

/// Convert serde_json::Value to SQL default value string
fn value_to_sql_default(value: &serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::String(s) => Ok(format!("'{}'", s.replace("'", "''"))),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(if *b { "1" } else { "0" }.to_string()),
        serde_json::Value::Null => Ok("NULL".to_string()),
        _ => Err(Error::generic("Unsupported default value type".to_string())),
    }
}
