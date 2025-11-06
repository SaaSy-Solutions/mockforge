//! Constraint enforcement
//!
//! This module handles constraint validation and enforcement for foreign keys,
//! unique constraints, and check constraints.

use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Constraint validator
pub struct ConstraintValidator;

impl ConstraintValidator {
    /// Validate foreign key constraint
    pub async fn validate_foreign_key(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        table_name: &str,
        field: &str,
        value: &Value,
        target_table: &str,
        target_field: &str,
    ) -> Result<()> {
        // Check if the referenced record exists
        let query = format!("SELECT COUNT(*) FROM {} WHERE {} = ?", target_table, target_field);

        let params = vec![value.clone()];
        let results = database.query(&query, &params).await?;

        if results.is_empty() || results[0].get("COUNT(*)") == Some(&Value::Number(0.into())) {
            return Err(Error::generic(format!(
                "Foreign key constraint violation: {} = {:?} does not exist in {}.{}",
                field, value, target_table, target_field
            )));
        }

        Ok(())
    }

    /// Validate unique constraint
    pub async fn validate_unique(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        table_name: &str,
        fields: &[String],
        values: &HashMap<String, Value>,
        exclude_id: Option<&Value>,
    ) -> Result<()> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for field in fields {
            if let Some(value) = values.get(field) {
                conditions.push(format!("{} = ?", field));
                params.push(value.clone());
            }
        }

        if conditions.is_empty() {
            return Ok(()); // No fields to check
        }

        let mut query =
            format!("SELECT COUNT(*) FROM {} WHERE {}", table_name, conditions.join(" AND "));

        if let Some(id) = exclude_id {
            query.push_str(&format!(" AND id != ?"));
            params.push(id.clone());
        }

        let results = database.query(&query, &params).await?;

        if !results.is_empty() {
            if let Some(count) = results[0].get("COUNT(*)") {
                if count.as_u64().unwrap_or(0) > 0 {
                    return Err(Error::generic(format!(
                        "Unique constraint violation: combination of {:?} already exists",
                        fields
                    )));
                }
            }
        }

        Ok(())
    }
}
