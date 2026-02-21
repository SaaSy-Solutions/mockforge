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
        _table_name: &str,
        field: &str,
        value: &Value,
        target_table: &str,
        target_field: &str,
    ) -> Result<()> {
        // Check if the referenced record exists
        let query = format!("SELECT COUNT(*) FROM {} WHERE {} = ?", target_table, target_field);

        let params = vec![value.clone()];
        let results = database.query(&query, &params).await?;

        // Check count result - column name varies by backend ("COUNT(*)" for SQLite, "count" for in-memory/JSON)
        let count = results
            .first()
            .and_then(|row| row.get("COUNT(*)").or_else(|| row.get("count")))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if results.is_empty() || count == 0 {
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
            query.push_str(" AND id != ?");
            params.push(id.clone());
        }

        let results = database.query(&query, &params).await?;

        if !results.is_empty() {
            // Column name varies by backend ("COUNT(*)" for SQLite, "count" for in-memory/JSON)
            let count = results[0]
                .get("COUNT(*)")
                .or_else(|| results[0].get("count"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if count > 0 {
                return Err(Error::generic(format!(
                    "Unique constraint violation: combination of {:?} already exists",
                    fields
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{InMemoryDatabase, VirtualDatabase};

    async fn setup_test_db() -> InMemoryDatabase {
        let mut db = InMemoryDatabase::new().await.unwrap();
        db.initialize().await.unwrap();

        // Create a test users table
        db.create_table("CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        // Create a test orders table with foreign key reference
        db.create_table(
            "CREATE TABLE IF NOT EXISTS orders (id TEXT PRIMARY KEY, user_id TEXT, total REAL)",
        )
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn test_validate_foreign_key_success() {
        let db = setup_test_db().await;

        // Insert a user
        db.execute(
            "INSERT INTO users (id, name) VALUES (?, ?)",
            &[
                Value::String("user-1".to_string()),
                Value::String("John".to_string()),
            ],
        )
        .await
        .unwrap();

        let validator = ConstraintValidator;
        let result = validator
            .validate_foreign_key(
                &db,
                "orders",
                "user_id",
                &Value::String("user-1".to_string()),
                "users",
                "id",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_foreign_key_failure() {
        let db = setup_test_db().await;

        let validator = ConstraintValidator;
        let result = validator
            .validate_foreign_key(
                &db,
                "orders",
                "user_id",
                &Value::String("nonexistent-user".to_string()),
                "users",
                "id",
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Foreign key constraint violation"));
    }

    #[tokio::test]
    async fn test_validate_unique_no_duplicates() {
        let db = setup_test_db().await;

        // Insert a user
        db.execute(
            "INSERT INTO users (id, name) VALUES (?, ?)",
            &[
                Value::String("user-1".to_string()),
                Value::String("John".to_string()),
            ],
        )
        .await
        .unwrap();

        let validator = ConstraintValidator;
        let mut values = HashMap::new();
        values.insert("name".to_string(), Value::String("Jane".to_string()));

        let result = validator
            .validate_unique(&db, "users", &["name".to_string()], &values, None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unique_with_duplicate() {
        let db = setup_test_db().await;

        // Insert a user
        db.execute(
            "INSERT INTO users (id, name) VALUES (?, ?)",
            &[
                Value::String("user-1".to_string()),
                Value::String("John".to_string()),
            ],
        )
        .await
        .unwrap();

        let validator = ConstraintValidator;
        let mut values = HashMap::new();
        values.insert("name".to_string(), Value::String("John".to_string()));

        let result = validator
            .validate_unique(&db, "users", &["name".to_string()], &values, None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unique constraint violation"));
    }

    #[tokio::test]
    async fn test_validate_unique_with_exclude_id() {
        let db = setup_test_db().await;

        // Insert a user
        db.execute(
            "INSERT INTO users (id, name) VALUES (?, ?)",
            &[
                Value::String("user-1".to_string()),
                Value::String("John".to_string()),
            ],
        )
        .await
        .unwrap();

        let validator = ConstraintValidator;
        let mut values = HashMap::new();
        values.insert("name".to_string(), Value::String("John".to_string()));

        // Should pass when excluding the same record
        let result = validator
            .validate_unique(
                &db,
                "users",
                &["name".to_string()],
                &values,
                Some(&Value::String("user-1".to_string())),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unique_empty_fields() {
        let db = setup_test_db().await;

        let validator = ConstraintValidator;
        let values = HashMap::new();

        let result = validator.validate_unique(&db, "users", &[], &values, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unique_missing_value() {
        let db = setup_test_db().await;

        let validator = ConstraintValidator;
        let values = HashMap::new(); // No values provided

        let result = validator
            .validate_unique(&db, "users", &["name".to_string()], &values, None)
            .await;

        // Should pass because no values to check
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_unique_multiple_fields() {
        let db = setup_test_db().await;

        // Create a table with composite unique constraint
        db.create_table(
            "CREATE TABLE IF NOT EXISTS products (id TEXT PRIMARY KEY, category TEXT, sku TEXT)",
        )
        .await
        .unwrap();

        // Insert a product
        db.execute(
            "INSERT INTO products (id, category, sku) VALUES (?, ?, ?)",
            &[
                Value::String("prod-1".to_string()),
                Value::String("electronics".to_string()),
                Value::String("SKU-001".to_string()),
            ],
        )
        .await
        .unwrap();

        let validator = ConstraintValidator;
        let mut values = HashMap::new();
        values.insert("category".to_string(), Value::String("electronics".to_string()));
        values.insert("sku".to_string(), Value::String("SKU-002".to_string())); // Different SKU

        let result = validator
            .validate_unique(
                &db,
                "products",
                &["category".to_string(), "sku".to_string()],
                &values,
                None,
            )
            .await;

        assert!(result.is_ok());
    }
}
