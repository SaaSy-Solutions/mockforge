//! Entity management and registry
//!
//! This module provides entity definition, management, and registry functionality
//! for tracking all entities in the VBR engine.

use crate::database::VirtualDatabase;
use crate::schema::VbrSchemaDefinition;
use crate::{Error, Result};
use mockforge_core::intelligent_behavior::rules::StateMachine;
use std::collections::HashMap;
use tracing::warn;

/// Entity definition
#[derive(Debug, Clone)]
pub struct Entity {
    /// Entity name
    pub name: String,

    /// Schema definition
    pub schema: VbrSchemaDefinition,

    /// Table name (derived from entity name)
    pub table_name: String,

    /// Optional state machine for this entity
    ///
    /// If set, the entity can participate in state machine transitions.
    /// The state machine defines valid state transitions and lifecycle management.
    pub state_machine: Option<StateMachine>,
}

impl Entity {
    /// Create a new entity
    pub fn new(name: String, schema: VbrSchemaDefinition) -> Self {
        let table_name = name.to_lowercase() + "s"; // Simple pluralization
        Self {
            name,
            schema,
            table_name,
            state_machine: None,
        }
    }

    /// Create a new entity with a state machine
    pub fn with_state_machine(
        name: String,
        schema: VbrSchemaDefinition,
        state_machine: StateMachine,
    ) -> Self {
        let table_name = name.to_lowercase() + "s";
        Self {
            name,
            schema,
            table_name,
            state_machine: Some(state_machine),
        }
    }

    /// Set the state machine for this entity
    pub fn set_state_machine(&mut self, state_machine: StateMachine) {
        self.state_machine = Some(state_machine);
    }

    /// Get the state machine for this entity
    pub fn state_machine(&self) -> Option<&StateMachine> {
        self.state_machine.as_ref()
    }

    /// Check if this entity has a state machine
    pub fn has_state_machine(&self) -> bool {
        self.state_machine.is_some()
    }

    /// Apply a state transition to an entity record
    ///
    /// Updates the entity's state field in the database based on the state machine transition.
    /// The state field name is typically derived from the state machine's resource_type
    /// (e.g., "status" for "Order" resource type).
    ///
    /// This method should be called after validating that the transition is allowed
    /// by the state machine.
    pub async fn apply_state_transition(
        &self,
        database: &dyn VirtualDatabase,
        record_id: &str,
        new_state: &str,
        state_field_name: Option<&str>,
    ) -> Result<()> {
        // Determine state field name
        // Default to "status" if not specified, or use resource_type-based naming
        let field_name = if let Some(name) = state_field_name {
            name
        } else if let Some(ref sm) = self.state_machine {
            // Use resource_type to derive field name (e.g., "Order" -> "order_status")
            // We'll use a static string for now - in production, this would need to be
            // stored or passed differently
            "status"
        } else {
            "status"
        };

        // Check if the field exists in the schema
        let field_exists = self.schema.base.fields.iter().any(|f| f.name == field_name);

        if !field_exists {
            // If field doesn't exist, we'll still try to update it
            // (it might be a dynamic field or added later)
            warn!(
                "State field '{}' not found in entity schema, attempting update anyway",
                field_name
            );
        }

        // Update the state field in the database
        let query = format!("UPDATE {} SET {} = ? WHERE id = ?", self.table_name, field_name);

        database
            .execute(
                &query,
                &[
                    serde_json::Value::String(new_state.to_string()),
                    serde_json::Value::String(record_id.to_string()),
                ],
            )
            .await
            .map_err(|e| Error::generic(format!("Failed to update entity state: {}", e)))?;

        Ok(())
    }

    /// Get the current state of an entity record
    ///
    /// Reads the state field from the database for a specific record.
    pub async fn get_current_state(
        &self,
        database: &dyn VirtualDatabase,
        record_id: &str,
        state_field_name: Option<&str>,
    ) -> Result<Option<String>> {
        let field_name = state_field_name.unwrap_or("status");

        let query = format!("SELECT {} FROM {} WHERE id = ?", field_name, self.table_name);

        let results = database
            .query(&query, &[serde_json::Value::String(record_id.to_string())])
            .await
            .map_err(|e| Error::generic(format!("Failed to query entity state: {}", e)))?;

        if let Some(row) = results.first() {
            if let Some(value) = row.get(field_name) {
                if let Some(state) = value.as_str() {
                    return Ok(Some(state.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Check if a state transition is allowed for an entity record
    ///
    /// Validates that the transition from the current state to the new state
    /// is allowed by the entity's state machine.
    pub async fn can_transition(
        &self,
        database: &dyn VirtualDatabase,
        record_id: &str,
        to_state: &str,
        state_field_name: Option<&str>,
    ) -> Result<bool> {
        let state_machine = self
            .state_machine
            .as_ref()
            .ok_or_else(|| Error::generic("Entity does not have a state machine configured"))?;

        // Get current state
        let current_state = self
            .get_current_state(database, record_id, state_field_name)
            .await?
            .ok_or_else(|| {
                Error::generic(format!("Record '{}' not found or has no state", record_id))
            })?;

        // Check if transition is allowed
        Ok(state_machine.can_transition(&current_state, to_state))
    }

    /// Get the entity name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

/// Entity registry for managing all entities
#[derive(Clone)]
pub struct EntityRegistry {
    /// Registered entities by name
    entities: HashMap<String, Entity>,
}

impl EntityRegistry {
    /// Create a new entity registry
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    /// Register an entity
    pub fn register(&mut self, entity: Entity) -> Result<()> {
        let name = entity.name.clone();
        if self.entities.contains_key(&name) {
            return Err(Error::generic(format!("Entity '{}' already registered", name)));
        }
        self.entities.insert(name, entity);
        Ok(())
    }

    /// Get an entity by name
    pub fn get(&self, name: &str) -> Option<&Entity> {
        self.entities.get(name)
    }

    /// Get all entity names
    pub fn list(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    /// Check if an entity exists
    pub fn exists(&self, name: &str) -> bool {
        self.entities.contains_key(name)
    }

    /// Remove an entity
    pub fn remove(&mut self, name: &str) -> Result<()> {
        self.entities
            .remove(name)
            .ok_or_else(|| Error::generic(format!("Entity '{}' not found", name)))?;
        Ok(())
    }
}

impl Default for EntityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_data::SchemaDefinition;

    #[test]
    fn test_entity_creation() {
        let base_schema = SchemaDefinition::new("User".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        assert_eq!(entity.name(), "User");
        assert_eq!(entity.table_name(), "users");
    }

    #[test]
    fn test_entity_registry() {
        let mut registry = EntityRegistry::new();

        let base_schema = SchemaDefinition::new("User".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        assert!(registry.register(entity).is_ok());
        assert!(registry.exists("User"));
        assert!(registry.get("User").is_some());
    }

    #[test]
    fn test_entity_registry_duplicate() {
        let mut registry = EntityRegistry::new();

        let base_schema1 = SchemaDefinition::new("User".to_string());
        let vbr_schema1 = VbrSchemaDefinition::new(base_schema1);
        let entity1 = Entity::new("User".to_string(), vbr_schema1);

        let base_schema2 = SchemaDefinition::new("User".to_string());
        let vbr_schema2 = VbrSchemaDefinition::new(base_schema2);
        let entity2 = Entity::new("User".to_string(), vbr_schema2);

        assert!(registry.register(entity1).is_ok());
        assert!(registry.register(entity2).is_err());
    }
}
