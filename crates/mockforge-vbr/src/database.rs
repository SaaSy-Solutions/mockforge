//! Virtual database abstraction
//!
//! This module provides a virtual database abstraction trait that supports multiple
//! storage backends: SQLite (persistent, production-like), JSON files (human-readable),
//! and in-memory (fast, no persistence).

use crate::{Error, Result};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Virtual database abstraction trait
///
/// This trait allows the VBR engine to work with different storage backends
/// transparently, supporting SQLite, JSON files, and in-memory storage.
#[async_trait]
pub trait VirtualDatabase: Send + Sync {
    /// Initialize the database and create necessary tables/schemas
    async fn initialize(&mut self) -> Result<()>;

    /// Execute a query that returns rows (SELECT)
    async fn query(&self, query: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>>;

    /// Execute a query that modifies data (INSERT, UPDATE, DELETE)
    async fn execute(&self, query: &str, params: &[Value]) -> Result<u64>;

    /// Execute a query and return the last inserted row ID
    async fn execute_with_id(&self, query: &str, params: &[Value]) -> Result<String>;

    /// Check if a table exists
    async fn table_exists(&self, table_name: &str) -> Result<bool>;

    /// Create a table from a CREATE TABLE statement
    async fn create_table(&self, create_statement: &str) -> Result<()>;

    /// Get database connection information (for debugging)
    fn connection_info(&self) -> String;

    /// Close the database connection (cleanup)
    async fn close(&mut self) -> Result<()>;
}

/// Create a virtual database instance based on the storage backend configuration
pub async fn create_database(
    backend: &crate::config::StorageBackend,
) -> Result<std::sync::Arc<dyn VirtualDatabase + Send + Sync>> {
    use std::sync::Arc;
    match backend {
        crate::config::StorageBackend::Sqlite { path } => {
            let mut db = SqliteDatabase::new(path.clone()).await?;
            db.initialize().await?;
            Ok(Arc::new(db))
        }
        crate::config::StorageBackend::Json { path } => {
            let mut db = JsonDatabase::new(path.clone()).await?;
            db.initialize().await?;
            Ok(Arc::new(db))
        }
        crate::config::StorageBackend::Memory => {
            let mut db = InMemoryDatabase::new().await?;
            db.initialize().await?;
            Ok(Arc::new(db))
        }
    }
}

/// SQLite database backend implementation
pub struct SqliteDatabase {
    pool: sqlx::SqlitePool,
    path: std::path::PathBuf,
}

impl SqliteDatabase {
    /// Create a new SQLite database connection
    pub async fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::generic(format!("Failed to create database directory: {}", e))
            })?;
        }

        let db_url = format!("sqlite://{}", path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await
            .map_err(|e| Error::generic(format!("Failed to connect to SQLite database: {}", e)))?;

        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| Error::generic(format!("Failed to enable WAL mode: {}", e)))?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| Error::generic(format!("Failed to enable foreign keys: {}", e)))?;

        Ok(Self { pool, path })
    }
}

#[async_trait]
impl VirtualDatabase for SqliteDatabase {
    async fn initialize(&mut self) -> Result<()> {
        // SQLite databases are initialized on connection
        // Additional initialization can be done here if needed
        Ok(())
    }

    async fn query(&self, query: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>> {
        use sqlx::Row;

        // For now, use a simple approach - bind parameters one by one
        // This is a simplified implementation; full implementation would handle
        // parameterized queries more robustly
        let mut query_builder = sqlx::query(query);

        // Bind parameters based on their type
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query_builder.bind(f)
                    } else {
                        query_builder.bind(n.to_string())
                    }
                }
                Value::Bool(b) => query_builder.bind(*b),
                Value::Null => query_builder.bind::<Option<String>>(None),
                Value::Array(_) | Value::Object(_) => {
                    let json_str = serde_json::to_string(param).unwrap_or_default();
                    query_builder.bind(json_str)
                }
            };
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::generic(format!("Query execution failed: {}", e)))?;

        // Convert rows to HashMap
        let mut results = Vec::new();
        for row in rows {
            let mut map = HashMap::new();
            let columns = row.columns();
            for (idx, column) in columns.iter().enumerate() {
                let value = row_value_to_json(&row, idx)?;
                map.insert(column.name().to_string(), value);
            }
            results.push(map);
        }

        Ok(results)
    }

    async fn execute(&self, query: &str, params: &[Value]) -> Result<u64> {
        // Build query with parameters
        let mut query_builder = sqlx::query(query);

        // Bind parameters based on their type
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query_builder.bind(f)
                    } else {
                        query_builder.bind(n.to_string())
                    }
                }
                Value::Bool(b) => query_builder.bind(*b),
                Value::Null => query_builder.bind::<Option<String>>(None),
                Value::Array(_) | Value::Object(_) => {
                    let json_str = serde_json::to_string(param).unwrap_or_default();
                    query_builder.bind(json_str)
                }
            };
        }

        let result = query_builder
            .execute(&self.pool)
            .await
            .map_err(|e| Error::generic(format!("Execute failed: {}", e)))?;

        Ok(result.rows_affected())
    }

    async fn execute_with_id(&self, query: &str, params: &[Value]) -> Result<String> {
        // Build query with parameters
        let mut query_builder = sqlx::query(query);

        // Bind parameters based on their type
        for param in params {
            query_builder = match param {
                Value::String(s) => query_builder.bind(s),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        query_builder.bind(f)
                    } else {
                        query_builder.bind(n.to_string())
                    }
                }
                Value::Bool(b) => query_builder.bind(*b),
                Value::Null => query_builder.bind::<Option<String>>(None),
                Value::Array(_) | Value::Object(_) => {
                    let json_str = serde_json::to_string(param).unwrap_or_default();
                    query_builder.bind(json_str)
                }
            };
        }

        let result = query_builder
            .execute(&self.pool)
            .await
            .map_err(|e| Error::generic(format!("Execute failed: {}", e)))?;

        // Get last inserted row ID
        let last_id = result.last_insert_rowid();
        Ok(last_id.to_string())
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
        let result = sqlx::query_scalar::<_, String>(query)
            .bind(table_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::generic(format!("Failed to check table existence: {}", e)))?;

        Ok(result.is_some())
    }

    async fn create_table(&self, create_statement: &str) -> Result<()> {
        sqlx::query(create_statement)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::generic(format!("Failed to create table: {}", e)))?;

        Ok(())
    }

    fn connection_info(&self) -> String {
        format!("SQLite: {}", self.path.display())
    }

    async fn close(&mut self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }
}

/// Helper function to extract a row value as JSON
fn row_value_to_json(row: &sqlx::sqlite::SqliteRow, idx: usize) -> Result<Value> {
    use sqlx::Row;

    // Try to get the value as different types
    if let Ok(value) = row.try_get::<String, _>(idx) {
        return Ok(Value::String(value));
    }
    if let Ok(value) = row.try_get::<i64, _>(idx) {
        return Ok(Value::Number(value.into()));
    }
    if let Ok(value) = row.try_get::<f64, _>(idx) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            return Ok(Value::Number(n));
        }
    }
    if let Ok(value) = row.try_get::<bool, _>(idx) {
        return Ok(Value::Bool(value));
    }
    if let Ok(value) = row.try_get::<Option<String>, _>(idx) {
        return Ok(value.map(Value::String).unwrap_or(Value::Null));
    }

    // Default: try to get as string
    Ok(Value::String(row.get::<String, _>(idx)))
}

/// JSON file database backend implementation
pub struct JsonDatabase {
    path: std::path::PathBuf,
    data: Arc<RwLock<HashMap<String, Vec<HashMap<String, Value>>>>>,
}

impl JsonDatabase {
    /// Create a new JSON database
    pub async fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Load existing data if file exists
        let data = if path.exists() {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| Error::generic(format!("Failed to read JSON database: {}", e)))?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            data: Arc::new(RwLock::new(data)),
        })
    }

    /// Save data to JSON file
    async fn save(&self) -> Result<()> {
        let data = self.data.read().await;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::generic(format!("Failed to create database directory: {}", e))
            })?;
        }

        // Serialize the data (not the RwLock wrapper)
        let content = serde_json::to_string_pretty(&*data)
            .map_err(|e| Error::generic(format!("Failed to serialize JSON database: {}", e)))?;

        tokio::fs::write(&self.path, content)
            .await
            .map_err(|e| Error::generic(format!("Failed to write JSON database: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl VirtualDatabase for JsonDatabase {
    async fn initialize(&mut self) -> Result<()> {
        // JSON databases don't need schema initialization
        Ok(())
    }

    async fn query(&self, query: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>> {
        // Simple SQL-like query parser for JSON backend
        // This is a basic implementation - for full SQL support, consider using sqlparser crate
        let data = self.data.read().await;
        let query_upper = query.trim().to_uppercase();

        // Handle SELECT COUNT(*) queries
        if query_upper.contains("COUNT(*)") || query_upper.contains("COUNT( * )") {
            let table_name = extract_table_name_from_select(query)?;
            if let Some(records) = data.get(table_name) {
                let count = if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?.len()
                } else {
                    records.len()
                };
                let mut result = HashMap::new();
                // Always use "count" as the field name for COUNT(*) queries
                result.insert("count".to_string(), Value::Number(count.into()));
                return Ok(vec![result]);
            }
        } else if query_upper.starts_with("SELECT") {
            // Extract table name from query
            let table_name = extract_table_name_from_select(query)?;

            if let Some(records) = data.get(table_name) {
                // Apply simple WHERE filtering
                let filtered = if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?
                } else {
                    records.clone()
                };

                // Apply LIMIT and OFFSET
                let result = apply_json_pagination(&filtered, query)?;
                return Ok(result);
            }
        } else if query_upper.starts_with("COUNT") {
            // Handle COUNT queries
            let table_name = extract_table_name_from_count(query)?;
            if let Some(records) = data.get(table_name) {
                let count = if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?.len()
                } else {
                    records.len()
                };
                let mut result = HashMap::new();
                result.insert("total".to_string(), Value::Number(count.into()));
                return Ok(vec![result]);
            }
        }

        Ok(vec![])
    }

    async fn execute(&self, query: &str, params: &[Value]) -> Result<u64> {
        let mut data = self.data.write().await;

        // Parse INSERT, UPDATE, DELETE queries
        let query_upper = query.trim().to_uppercase();

        if query_upper.starts_with("INSERT") {
            let (table_name, record) = parse_insert_query(query, params)?;
            let records = data.entry(table_name).or_insert_with(Vec::new);
            records.push(record);
            self.save().await?;
            Ok(1)
        } else if query_upper.starts_with("UPDATE") {
            let (table_name, updates, where_clause, where_params) =
                parse_update_query(query, params)?;
            if let Some(records) = data.get_mut(&table_name) {
                let mut updated = 0;
                for record in records.iter_mut() {
                    if matches_json_where(record, &where_clause, &where_params)? {
                        record.extend(updates.clone());
                        updated += 1;
                    }
                }
                self.save().await?;
                Ok(updated)
            } else {
                Ok(0)
            }
        } else if query_upper.starts_with("DELETE") {
            let (table_name, where_clause, where_params) = parse_delete_query(query, params)?;
            if let Some(records) = data.get_mut(&table_name) {
                let initial_len = records.len();
                records.retain(|record| {
                    !matches_json_where(record, &where_clause, &where_params).unwrap_or(false)
                });
                let deleted = initial_len - records.len();
                self.save().await?;
                Ok(deleted as u64)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    async fn execute_with_id(&self, query: &str, params: &[Value]) -> Result<String> {
        // For INSERT, extract the ID from the inserted record
        let mut data = self.data.write().await;

        if query.trim().to_uppercase().starts_with("INSERT") {
            let (table_name, mut record) = parse_insert_query(query, params)?;

            // Generate ID if not present
            if !record.contains_key("id") {
                use uuid::Uuid;
                record.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
            }

            let id = record.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let records = data.entry(table_name).or_insert_with(Vec::new);
            records.push(record);
            self.save().await?;
            Ok(id)
        } else {
            self.execute(query, params).await?;
            Ok(String::new())
        }
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(table_name))
    }

    async fn create_table(&self, _create_statement: &str) -> Result<()> {
        // JSON backend doesn't need explicit table creation
        Ok(())
    }

    fn connection_info(&self) -> String {
        format!("JSON: {}", self.path.display())
    }

    async fn close(&mut self) -> Result<()> {
        self.save().await
    }
}

/// In-memory database backend implementation
pub struct InMemoryDatabase {
    data: Arc<RwLock<HashMap<String, Vec<HashMap<String, Value>>>>>,
}

impl InMemoryDatabase {
    /// Create a new in-memory database
    pub async fn new() -> Result<Self> {
        Ok(Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl VirtualDatabase for InMemoryDatabase {
    async fn initialize(&mut self) -> Result<()> {
        // In-memory databases don't need initialization
        Ok(())
    }

    async fn query(&self, query: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>> {
        // Reuse JSON backend query logic (same structure)
        let data = self.data.read().await;
        let query_upper = query.trim().to_uppercase();

        // Handle SELECT COUNT(*) queries
        if query_upper.contains("COUNT(*)") || query_upper.contains("COUNT( * )") {
            let table_name = extract_table_name_from_select(query)?;
            let count = if let Some(records) = data.get(table_name) {
                if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?.len()
                } else {
                    records.len()
                }
            } else {
                // Table doesn't exist yet, return 0
                0
            };
            let mut result = HashMap::new();
            result.insert("count".to_string(), Value::Number(count.into()));
            return Ok(vec![result]);
        } else if query_upper.starts_with("SELECT") {
            let table_name = extract_table_name_from_select(query)?;

            if let Some(records) = data.get(table_name) {
                let filtered = if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?
                } else {
                    records.clone()
                };

                let result = apply_json_pagination(&filtered, query)?;
                return Ok(result);
            }
        } else if query_upper.starts_with("COUNT") {
            let table_name = extract_table_name_from_count(query)?;
            if let Some(records) = data.get(table_name) {
                let count = if query.contains("WHERE") {
                    apply_json_where_clause(records, query, params)?.len()
                } else {
                    records.len()
                };
                let mut result = HashMap::new();
                result.insert("total".to_string(), Value::Number(count.into()));
                return Ok(vec![result]);
            }
        }

        Ok(vec![])
    }

    async fn execute(&self, query: &str, params: &[Value]) -> Result<u64> {
        let mut data = self.data.write().await;

        let query_upper = query.trim().to_uppercase();

        if query_upper.starts_with("INSERT") {
            let (table_name, record) = parse_insert_query(query, params)?;
            let records = data.entry(table_name).or_insert_with(Vec::new);
            records.push(record);
            Ok(1)
        } else if query_upper.starts_with("UPDATE") {
            let (table_name, updates, where_clause, where_params) =
                parse_update_query(query, params)?;
            if let Some(records) = data.get_mut(&table_name) {
                let mut updated = 0;
                for record in records.iter_mut() {
                    if matches_json_where(record, &where_clause, &where_params)? {
                        record.extend(updates.clone());
                        updated += 1;
                    }
                }
                Ok(updated)
            } else {
                Ok(0)
            }
        } else if query_upper.starts_with("DELETE") {
            let (table_name, where_clause, where_params) = parse_delete_query(query, params)?;
            // Ensure table exists (for DELETE FROM table_name without WHERE, we need the table)
            let records = data.entry(table_name.clone()).or_insert_with(Vec::new);
            let initial_len = records.len();
            records.retain(|record| {
                !matches_json_where(record, &where_clause, &where_params).unwrap_or(false)
            });
            let deleted = initial_len - records.len();
            Ok(deleted as u64)
        } else {
            Ok(0)
        }
    }

    async fn execute_with_id(&self, query: &str, params: &[Value]) -> Result<String> {
        let mut data = self.data.write().await;

        if query.trim().to_uppercase().starts_with("INSERT") {
            let (table_name, mut record) = parse_insert_query(query, params)?;

            if !record.contains_key("id") {
                use uuid::Uuid;
                record.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
            }

            let id = record.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let records = data.entry(table_name).or_insert_with(Vec::new);
            records.push(record);
            Ok(id)
        } else {
            self.execute(query, params).await?;
            Ok(String::new())
        }
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(table_name))
    }

    async fn create_table(&self, create_statement: &str) -> Result<()> {
        // In-memory backend doesn't need explicit table creation, but we should
        // extract table name and ensure it exists in the data HashMap
        // Extract table name from CREATE TABLE statement
        // Format: "CREATE TABLE IF NOT EXISTS table_name (" or "CREATE TABLE table_name ("
        let query_upper = create_statement.to_uppercase();
        if query_upper.contains("CREATE TABLE") {
            let mut rest = create_statement;

            // Skip "CREATE TABLE"
            if let Some(idx) = query_upper.find("CREATE TABLE") {
                rest = &create_statement[idx + 12..];
            }

            // Skip "IF NOT EXISTS" if present
            let rest_upper = rest.to_uppercase();
            if rest_upper.trim_start().starts_with("IF NOT EXISTS") {
                if let Some(idx) = rest_upper.find("IF NOT EXISTS") {
                    rest = &rest[idx + 13..];
                }
            }

            // Find the table name (ends at '(' or whitespace)
            let table_name = rest
                .trim_start()
                .split(|c: char| c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if !table_name.is_empty() {
                let mut data = self.data.write().await;
                data.entry(table_name).or_insert_with(Vec::new);
            }
        }
        Ok(())
    }

    fn connection_info(&self) -> String {
        "In-Memory".to_string()
    }

    async fn close(&mut self) -> Result<()> {
        // In-memory databases don't need cleanup
        Ok(())
    }
}

// Helper functions for JSON/InMemory query parsing

/// Extract table name from SELECT query
fn extract_table_name_from_select(query: &str) -> Result<&str> {
    // Simple parser: "SELECT * FROM table_name"
    let parts: Vec<&str> = query.split_whitespace().collect();
    if let Some(from_idx) = parts.iter().position(|&p| p.to_uppercase() == "FROM") {
        if from_idx + 1 < parts.len() {
            let table_name = parts[from_idx + 1].trim_end_matches(';');
            return Ok(table_name);
        }
    }
    Err(Error::generic("Invalid SELECT query: missing FROM clause".to_string()))
}

/// Extract table name from COUNT query
fn extract_table_name_from_count(query: &str) -> Result<&str> {
    // "SELECT COUNT(*) FROM table_name" or "SELECT COUNT(*) as total FROM table_name"
    extract_table_name_from_select(query)
}

/// Apply WHERE clause filtering to JSON records
fn apply_json_where_clause(
    records: &[HashMap<String, Value>],
    query: &str,
    params: &[Value],
) -> Result<Vec<HashMap<String, Value>>> {
    // Simple WHERE clause parser - supports basic "field = ?" patterns
    let mut result = Vec::new();

    for record in records {
        if matches_json_where(record, query, params)? {
            result.push(record.clone());
        }
    }

    Ok(result)
}

/// Check if a record matches WHERE clause
fn matches_json_where(
    record: &HashMap<String, Value>,
    query: &str,
    params: &[Value],
) -> Result<bool> {
    // Extract WHERE clause from query
    if let Some(where_idx) = query.to_uppercase().find("WHERE") {
        let where_clause = &query[where_idx + 5..];

        // Parse simple conditions like "field = ?"
        let parts: Vec<&str> = where_clause.split_whitespace().collect();
        if parts.len() >= 3 && parts[1] == "=" {
            let field = parts[0];
            let param_idx = parts.iter().position(|&p| p == "?").unwrap_or(0);

            if param_idx < params.len() {
                let expected_value = &params[0]; // Use first param for simple cases
                let actual_value = record.get(field);

                return Ok(matches_value(actual_value, expected_value));
            }
        }
    }

    Ok(true) // No WHERE clause or couldn't parse
}

/// Check if two values match
fn matches_value(actual: Option<&Value>, expected: &Value) -> bool {
    match (actual, expected) {
        (Some(a), e) => a == e,
        (None, Value::Null) => true,
        _ => false,
    }
}

/// Apply pagination (LIMIT and OFFSET) to results
fn apply_json_pagination(
    records: &[HashMap<String, Value>],
    query: &str,
) -> Result<Vec<HashMap<String, Value>>> {
    let mut result = records.to_vec();

    // Extract LIMIT
    if let Some(limit_idx) = query.to_uppercase().find("LIMIT") {
        let limit_str = query[limit_idx + 5..]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(';');

        if let Ok(limit) = limit_str.parse::<usize>() {
            // Extract OFFSET
            let offset = if let Some(offset_idx) = query.to_uppercase().find("OFFSET") {
                query[offset_idx + 6..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .trim_end_matches(';')
                    .parse::<usize>()
                    .unwrap_or(0)
            } else {
                0
            };

            let start = offset.min(result.len());
            let end = (start + limit).min(result.len());
            result = result[start..end].to_vec();
        }
    }

    Ok(result)
}

/// Parse INSERT query and return (table_name, record)
fn parse_insert_query(query: &str, params: &[Value]) -> Result<(String, HashMap<String, Value>)> {
    // Simple parser: "INSERT INTO table_name (field1, field2) VALUES (?, ?)"
    let parts: Vec<&str> = query.split_whitespace().collect();

    if let Some(into_idx) = parts.iter().position(|&p| p.to_uppercase() == "INTO") {
        if into_idx + 1 < parts.len() {
            let table_name = parts[into_idx + 1].to_string();

            // Extract field names
            if let Some(fields_start) = query.find('(') {
                if let Some(fields_end) = query[fields_start + 1..].find(')') {
                    let fields_str = &query[fields_start + 1..fields_start + 1 + fields_end];
                    let fields: Vec<&str> = fields_str.split(',').map(|s| s.trim()).collect();

                    // Build record from params
                    let mut record = HashMap::new();
                    for (idx, field) in fields.iter().enumerate() {
                        if idx < params.len() {
                            record.insert(field.to_string(), params[idx].clone());
                        }
                    }

                    return Ok((table_name, record));
                }
            }
        }
    }

    Err(Error::generic("Invalid INSERT query format".to_string()))
}

/// Parse UPDATE query
fn parse_update_query(
    query: &str,
    params: &[Value],
) -> Result<(String, HashMap<String, Value>, String, Vec<Value>)> {
    // "UPDATE table_name SET field1 = ?, field2 = ? WHERE field3 = ?"
    let parts: Vec<&str> = query.split_whitespace().collect();

    if parts.len() < 4 || parts[0].to_uppercase() != "UPDATE" {
        return Err(Error::generic("Invalid UPDATE query".to_string()));
    }

    let table_name = parts[1].to_string();

    // Extract SET clause
    if let Some(set_idx) = parts.iter().position(|&p| p.to_uppercase() == "SET") {
        let set_clause = &query[query.to_uppercase().find("SET").unwrap() + 3..];
        let where_clause = if let Some(where_idx) = set_clause.to_uppercase().find("WHERE") {
            &set_clause[..where_idx]
        } else {
            set_clause
        };

        // Parse SET fields
        let mut updates = HashMap::new();
        let set_parts: Vec<&str> = where_clause.split(',').collect();
        let mut param_idx = 0;

        for part in set_parts {
            let field_eq: Vec<&str> = part.split('=').map(|s| s.trim()).collect();
            if field_eq.len() == 2 && field_eq[1] == "?" {
                if param_idx < params.len() {
                    updates.insert(field_eq[0].to_string(), params[param_idx].clone());
                    param_idx += 1;
                }
            }
        }

        // Extract WHERE clause
        let (where_clause_str, where_params) =
            if let Some(where_idx) = set_clause.to_uppercase().find("WHERE") {
                let where_part = &set_clause[where_idx + 5..];
                (where_part.to_string(), params[param_idx..].to_vec())
            } else {
                (String::new(), Vec::new())
            };

        return Ok((table_name, updates, where_clause_str, where_params));
    }

    Err(Error::generic("Invalid UPDATE query: missing SET clause".to_string()))
}

/// Parse DELETE query
fn parse_delete_query(query: &str, params: &[Value]) -> Result<(String, String, Vec<Value>)> {
    // "DELETE FROM table_name WHERE field = ?"
    let parts: Vec<&str> = query.split_whitespace().collect();

    if let Some(from_idx) = parts.iter().position(|&p| p.to_uppercase() == "FROM") {
        if from_idx + 1 < parts.len() {
            let table_name = parts[from_idx + 1].to_string();

            // Extract WHERE clause
            if let Some(where_idx) = query.to_uppercase().find("WHERE") {
                let where_clause = query[where_idx + 5..].to_string();
                return Ok((table_name, where_clause, params.to_vec()));
            } else {
                return Ok((table_name, String::new(), Vec::new()));
            }
        }
    }

    Err(Error::generic("Invalid DELETE query".to_string()))
}
