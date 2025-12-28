//! Runtime request handlers
//!
//! This module provides runtime request handlers for generated CRUD operations,
//! including request validation, response formatting, and error handling.

use crate::constraints::ConstraintValidator;
use crate::Result;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::Json;
use axum::Extension;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Validates that a field name is a safe SQL identifier.
/// Only allows alphanumeric characters and underscores.
/// This prevents SQL injection through field names.
fn is_safe_sql_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true)
}

/// Validates a field name against the entity schema AND ensures it's a safe SQL identifier.
/// Returns the field name if valid, or an error otherwise.
fn validate_field_name<'a>(
    field_name: &'a str,
    entity: &crate::entities::Entity,
) -> std::result::Result<&'a str, (StatusCode, Json<Value>)> {
    // First check if it's a safe SQL identifier
    if !is_safe_sql_identifier(field_name) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid field name: '{}'. Field names must contain only alphanumeric characters and underscores.", field_name)
            })),
        ));
    }

    // Then check if the field exists in the schema
    if !entity.schema.base.fields.iter().any(|f| f.name == field_name) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Unknown field: '{}'. Field does not exist in entity schema.", field_name)
            })),
        ));
    }

    Ok(field_name)
}

/// Generic handler context
///
/// This context is shared across all handlers via Axum Extension.
/// The database is wrapped in Arc to allow sharing across async tasks.
#[derive(Clone)]
pub struct HandlerContext {
    /// Database instance (shared via Arc)
    pub database: Arc<dyn crate::database::VirtualDatabase + Send + Sync>,
    /// Entity registry
    pub registry: crate::entities::EntityRegistry,
    /// Session data manager (optional, for session-scoped data)
    pub session_manager: Option<std::sync::Arc<crate::session::SessionDataManager>>,
    /// Snapshots directory (optional, for snapshot operations)
    pub snapshots_dir: Option<std::path::PathBuf>,
}

/// Helper function to get entity and table name
fn get_entity_info<'a>(
    registry: &'a crate::entities::EntityRegistry,
    entity_name: &str,
) -> std::result::Result<(&'a crate::entities::Entity, &'a str), (StatusCode, Json<Value>)> {
    let entity = registry.get(entity_name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Entity '{}' not found", entity_name)
            })),
        )
    })?;

    Ok((entity, entity.table_name()))
}

/// Helper function to apply auto-generation rules
async fn apply_auto_generation(
    data: &mut Value,
    schema: &crate::schema::VbrSchemaDefinition,
    entity_name: &str,
    database: &dyn crate::database::VirtualDatabase,
) -> Result<()> {
    if let Value::Object(obj) = data {
        for (field_name, rule) in &schema.auto_generation {
            if !obj.contains_key(field_name) {
                let generated_value = match rule {
                    crate::schema::AutoGenerationRule::Uuid => {
                        Value::String(uuid::Uuid::new_v4().to_string())
                    }
                    crate::schema::AutoGenerationRule::Timestamp => {
                        Value::String(chrono::Utc::now().to_rfc3339())
                    }
                    crate::schema::AutoGenerationRule::Date => {
                        Value::String(chrono::Utc::now().date_naive().to_string())
                    }
                    crate::schema::AutoGenerationRule::AutoIncrement => {
                        // Auto-increment is handled by database
                        continue;
                    }
                    crate::schema::AutoGenerationRule::Pattern(pattern) => {
                        // Get counter for pattern-based IDs that use increment
                        let counter = if pattern.contains("increment") {
                            Some(
                                crate::id_generation::get_and_increment_counter(
                                    database,
                                    entity_name,
                                    field_name,
                                )
                                .await?,
                            )
                        } else {
                            None
                        };
                        let id = crate::id_generation::generate_id(
                            rule,
                            entity_name,
                            field_name,
                            counter,
                        )?;
                        Value::String(id)
                    }
                    crate::schema::AutoGenerationRule::Realistic { .. } => {
                        let id =
                            crate::id_generation::generate_id(rule, entity_name, field_name, None)?;
                        Value::String(id)
                    }
                    crate::schema::AutoGenerationRule::Custom(_) => {
                        // Custom rules would need evaluation engine
                        continue;
                    }
                };
                obj.insert(field_name.clone(), generated_value);
            }
        }
    }
    Ok(())
}

/// Helper function to build WHERE clause from query parameters
/// Returns an error if any field name is invalid (SQL injection prevention)
fn build_where_clause(
    params: &HashMap<String, String>,
    entity: &crate::entities::Entity,
) -> std::result::Result<(String, Vec<Value>), (StatusCode, Json<Value>)> {
    let mut conditions = Vec::new();
    let mut bind_values = Vec::new();

    for (key, value) in params {
        // Skip pagination and sorting parameters
        if matches!(key.as_str(), "limit" | "offset" | "sort" | "order") {
            continue;
        }

        // Validate field name is safe AND exists in schema
        if is_safe_sql_identifier(key) && entity.schema.base.fields.iter().any(|f| f.name == *key) {
            conditions.push(format!("{} = ?", key));
            bind_values.push(Value::String(value.clone()));
        }
        // Silently ignore fields that don't exist or are invalid
        // This maintains backwards compatibility while being safe
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    Ok((where_clause, bind_values))
}

/// Helper function to build ORDER BY clause
/// Validates sort field is safe and exists in schema (SQL injection prevention)
fn build_order_by(params: &HashMap<String, String>, entity: &crate::entities::Entity) -> String {
    if let Some(sort_field) = params.get("sort") {
        // Validate sort field is a safe SQL identifier AND exists in schema
        if is_safe_sql_identifier(sort_field)
            && entity.schema.base.fields.iter().any(|f| f.name == *sort_field)
        {
            let order = params
                .get("order")
                .map(|o| o.to_uppercase())
                .unwrap_or_else(|| "ASC".to_string());
            if order == "ASC" || order == "DESC" {
                return format!("ORDER BY {} {}", sort_field, order);
            }
        }
    }
    String::new()
}

/// Helper function to get pagination parameters
fn get_pagination(params: &HashMap<String, String>) -> (Option<usize>, Option<usize>) {
    let limit = params.get("limit").and_then(|v| v.parse().ok());
    let offset = params.get("offset").and_then(|v| v.parse().ok());
    (limit, offset)
}

/// List all entities (GET /api/{entity})
pub async fn list_handler(
    Path(entity_name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (entity, table_name) = get_entity_info(&context.registry, &entity_name)?;

    // Build query with validated field names
    let (where_clause, bind_values) = build_where_clause(&params, entity)?;
    let order_by = build_order_by(&params, entity);
    let (limit, offset) = get_pagination(&params);

    // Build SELECT query
    let mut query = format!("SELECT * FROM {} {}", table_name, where_clause);
    if !order_by.is_empty() {
        query.push_str(&format!(" {}", order_by));
    }

    // Add LIMIT and OFFSET (directly in query, not as parameters)
    if let Some(limit_val) = limit {
        query.push_str(&format!(" LIMIT {}", limit_val));
    }
    if let Some(offset_val) = offset {
        query.push_str(&format!(" OFFSET {}", offset_val));
    }

    // Execute query
    let results = context.database.query(&query, &bind_values).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database query failed: {}", e)})),
        )
    })?;

    // Get total count for pagination
    let count_query = format!("SELECT COUNT(*) as total FROM {} {}", table_name, where_clause);
    let count_results = context.database.query(&count_query, &bind_values).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Count query failed: {}", e)})),
        )
    })?;

    let total = count_results
        .first()
        .and_then(|r| r.get("total"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(Json(json!({
        "data": results,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

/// Get entity by ID (GET /api/{entity}/{id})
pub async fn get_handler(
    Path((entity_name, id)): Path<(String, String)>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (_entity, table_name) = get_entity_info(&context.registry, &entity_name)?;

    // Get primary key field (default to "id")
    let primary_key = "id";

    // Build SELECT query
    let query = format!("SELECT * FROM {} WHERE {} = ?", table_name, primary_key);
    let params = vec![Value::String(id.clone())];

    // Execute query
    let results = context.database.query(&query, &params).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database query failed: {}", e)})),
        )
    })?;

    // Return first result or 404
    if let Some(result) = results.into_iter().next() {
        Ok(Json(Value::Object(
            result.into_iter().collect::<serde_json::Map<String, Value>>(),
        )))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("{} with id '{}' not found", entity_name, id)
            })),
        ))
    }
}

/// Create entity (POST /api/{entity})
pub async fn create_handler(
    Path(entity_name): Path<String>,
    Extension(context): Extension<HandlerContext>,
    Json(mut body): Json<Value>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (entity, table_name) = get_entity_info(&context.registry, &entity_name)?;

    // Apply auto-generation rules
    apply_auto_generation(&mut body, &entity.schema, &entity_name, context.database.as_ref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Auto-generation failed: {}", e)})),
            )
        })?;

    // Validate foreign key constraints
    let validator = ConstraintValidator;
    if let Value::Object(obj) = &body {
        for fk in &entity.schema.foreign_keys {
            if let Some(fk_value) = obj.get(&fk.field) {
                validator
                    .validate_foreign_key(
                        context.database.as_ref(),
                        table_name,
                        &fk.field,
                        fk_value,
                        &(fk.target_entity.to_lowercase() + "s"),
                        &fk.target_field,
                    )
                    .await
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    })?;
            }
        }
    }

    // Build INSERT query
    if let Value::Object(obj) = &body {
        // Validate all field names are safe SQL identifiers (SQL injection prevention)
        let mut validated_fields: Vec<String> = Vec::new();
        let mut values: Vec<Value> = Vec::new();

        for (field_name, value) in obj.iter() {
            // Validate field name is safe
            if !is_safe_sql_identifier(field_name) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Invalid field name: '{}'. Field names must contain only alphanumeric characters and underscores.", field_name)
                    })),
                ));
            }
            validated_fields.push(field_name.clone());
            values.push(value.clone());
        }

        let placeholders: Vec<String> =
            (0..validated_fields.len()).map(|_| "?".to_string()).collect();

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            validated_fields.join(", "),
            placeholders.join(", ")
        );

        // Execute insert
        let inserted_id = context.database.execute_with_id(&query, &values).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Insert failed: {}", e)})),
            )
        })?;

        // Fetch the created record
        let primary_key = "id";
        let select_query = format!("SELECT * FROM {} WHERE {} = ?", table_name, primary_key);
        let select_results = context
            .database
            .query(&select_query, &[Value::String(inserted_id)])
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to fetch created record: {}", e)})),
                )
            })?;

        if let Some(result) = select_results.into_iter().next() {
            Ok(Json(Value::Object(
                result.into_iter().collect::<serde_json::Map<String, Value>>(),
            )))
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to retrieve created record"})),
            ))
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Request body must be a JSON object"})),
        ))
    }
}

/// Update entity (PUT /api/{entity}/{id})
pub async fn update_handler(
    Path((entity_name, id)): Path<(String, String)>,
    Extension(context): Extension<HandlerContext>,
    Json(body): Json<Value>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (entity, table_name) = get_entity_info(&context.registry, &entity_name)?;
    let primary_key = "id";

    // Check if record exists
    let check_query =
        format!("SELECT COUNT(*) as count FROM {} WHERE {} = ?", table_name, primary_key);
    let check_results = context
        .database
        .query(&check_query, &[Value::String(id.clone())])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database query failed: {}", e)})),
            )
        })?;

    let exists = check_results
        .first()
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_u64())
        .map(|v| v > 0)
        .unwrap_or(false);

    if !exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("{} with id '{}' not found", entity_name, id)
            })),
        ));
    }

    // Validate foreign key constraints
    let validator = ConstraintValidator;
    if let Value::Object(obj) = &body {
        for fk in &entity.schema.foreign_keys {
            if let Some(fk_value) = obj.get(&fk.field) {
                validator
                    .validate_foreign_key(
                        context.database.as_ref(),
                        table_name,
                        &fk.field,
                        fk_value,
                        &(fk.target_entity.to_lowercase() + "s"),
                        &fk.target_field,
                    )
                    .await
                    .map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    })?;
            }
        }
    }

    // Build UPDATE query
    if let Value::Object(obj) = &body {
        let mut set_clauses = Vec::new();
        let mut values = Vec::new();

        for (field, value) in obj.iter() {
            if field != primary_key {
                // Validate field name is safe (SQL injection prevention)
                if !is_safe_sql_identifier(field) {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": format!("Invalid field name: '{}'. Field names must contain only alphanumeric characters and underscores.", field)
                        })),
                    ));
                }
                set_clauses.push(format!("{} = ?", field));
                values.push(value.clone());
            }
        }

        if set_clauses.is_empty() {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No fields to update"}))));
        }

        values.push(Value::String(id.clone()));

        let query = format!(
            "UPDATE {} SET {} WHERE {} = ?",
            table_name,
            set_clauses.join(", "),
            primary_key
        );

        // Execute update
        context.database.execute(&query, &values).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Update failed: {}", e)})),
            )
        })?;

        // Fetch updated record
        let select_query = format!("SELECT * FROM {} WHERE {} = ?", table_name, primary_key);
        let select_results =
            context.database.query(&select_query, &[Value::String(id)]).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Failed to fetch updated record: {}", e)})),
                )
            })?;

        if let Some(result) = select_results.into_iter().next() {
            Ok(Json(Value::Object(
                result.into_iter().collect::<serde_json::Map<String, Value>>(),
            )))
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to retrieve updated record"})),
            ))
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Request body must be a JSON object"})),
        ))
    }
}

/// Partial update entity (PATCH /api/{entity}/{id})
pub async fn patch_handler(
    Path((entity_name, id)): Path<(String, String)>,
    Extension(context): Extension<HandlerContext>,
    Json(body): Json<Value>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    // PATCH is similar to PUT but only updates provided fields
    // For now, we'll use the same logic as PUT
    // In a full implementation, we'd fetch the existing record first and merge
    update_handler(Path((entity_name.clone(), id.clone())), Extension(context), Json(body)).await
}

/// Delete entity (DELETE /api/{entity}/{id})
pub async fn delete_handler(
    Path((entity_name, id)): Path<(String, String)>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (_entity, table_name) = get_entity_info(&context.registry, &entity_name)?;
    let primary_key = "id";

    // Check if record exists
    let check_query =
        format!("SELECT COUNT(*) as count FROM {} WHERE {} = ?", table_name, primary_key);
    let check_results = context
        .database
        .query(&check_query, &[Value::String(id.clone())])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database query failed: {}", e)})),
            )
        })?;

    let exists = check_results
        .first()
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_u64())
        .map(|v| v > 0)
        .unwrap_or(false);

    if !exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("{} with id '{}' not found", entity_name, id)
            })),
        ));
    }

    // Build DELETE query
    let query = format!("DELETE FROM {} WHERE {} = ?", table_name, primary_key);
    let params = vec![Value::String(id.clone())];

    // Execute delete
    let rows_affected = context.database.execute(&query, &params).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Delete failed: {}", e)})),
        )
    })?;

    if rows_affected > 0 {
        Ok(Json(json!({
            "message": format!("{} with id '{}' deleted successfully", entity_name, id),
            "id": id
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("{} with id '{}' not found", entity_name, id)
            })),
        ))
    }
}

/// Get related entities (GET /api/{entity}/{id}/{relationship})
///
/// This handler supports relationship traversal:
/// - Forward relationships: Get child entities (one-to-many)
///   Example: GET /api/users/123/orders -> Get all orders where user_id = 123
/// - Reverse relationships: Get parent entity (many-to-one)
///   Example: GET /api/orders/456/user -> Get the user for order 456
pub async fn get_relationship_handler(
    Path((entity_name, id, relationship_name)): Path<(String, String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (entity, table_name) = get_entity_info(&context.registry, &entity_name)?;
    let primary_key = "id";

    // First, verify the parent entity exists
    let check_query =
        format!("SELECT COUNT(*) as count FROM {} WHERE {} = ?", table_name, primary_key);
    let check_results = context
        .database
        .query(&check_query, &[Value::String(id.clone())])
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Database query failed: {}", e)})),
            )
        })?;

    let exists = check_results
        .first()
        .and_then(|r| r.get("count"))
        .and_then(|v| v.as_u64())
        .map(|v| v > 0)
        .unwrap_or(false);

    if !exists {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("{} with id '{}' not found", entity_name, id)
            })),
        ));
    }

    // Find the relationship definition
    // Strategy 1: Forward relationship (one-to-many)
    // Example: GET /api/users/123/orders
    // - Look for an entity named "orders" that has a FK pointing to "users"
    if let Some(target_entity) = context.registry.get(&relationship_name) {
        // Check if this target entity has a FK pointing to the current entity
        if let Some(fk) = target_entity
            .schema
            .foreign_keys
            .iter()
            .find(|fk| fk.target_entity == entity_name)
        {
            // Forward relationship: Get child entities
            // Example: GET /api/users/123/orders -> Get orders where user_id = 123
            let target_table = target_entity.table_name();

            // Build query to get related entities
            let (where_clause, mut bind_values) = build_where_clause(&params, target_entity)?;
            let order_by = build_order_by(&params, target_entity);
            let (limit, offset) = get_pagination(&params);

            // Add the foreign key condition
            let fk_condition = if where_clause.is_empty() {
                format!("WHERE {} = ?", fk.field)
            } else {
                format!("{} AND {} = ?", where_clause, fk.field)
            };
            bind_values.push(Value::String(id.clone()));

            let mut query = format!("SELECT * FROM {} {}", target_table, fk_condition);
            if !order_by.is_empty() {
                query.push_str(&format!(" {}", order_by));
            }
            if let Some(limit_val) = limit {
                query.push_str(&format!(" LIMIT {}", limit_val));
            }
            if let Some(offset_val) = offset {
                query.push_str(&format!(" OFFSET {}", offset_val));
            }

            let results = context.database.query(&query, &bind_values).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Database query failed: {}", e)})),
                )
            })?;

            // Get total count
            let count_query =
                format!("SELECT COUNT(*) as total FROM {} {}", target_table, fk_condition);
            let count_results =
                context.database.query(&count_query, &bind_values).await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Count query failed: {}", e)})),
                    )
                })?;

            let total = count_results
                .first()
                .and_then(|r| r.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            return Ok(Json(json!({
                "data": results,
                "total": total,
                "relationship": relationship_name,
                "parent_entity": entity_name,
                "parent_id": id
            })));
        }
    }

    // Strategy 2: Reverse relationship (many-to-one)
    // Example: GET /api/orders/456/user
    // - The current entity (orders) has a FK field pointing to "user" entity
    // Try reverse relationship: Find if current entity has a FK pointing to the relationship
    // Example: GET /api/orders/456/user -> orders table has user_id field pointing to users
    if let Some(fk) = entity.schema.foreign_keys.iter().find(|fk| {
        // Relationship name might match the target entity or the FK field
        fk.target_entity.to_lowercase() == relationship_name.to_lowercase()
            || fk.field == relationship_name
            || fk.field == format!("{}_id", relationship_name)
    }) {
        // Reverse relationship: Get the parent entity
        // Get the current entity record to find the FK value
        let current_query = format!("SELECT * FROM {} WHERE {} = ?", table_name, primary_key);
        let current_results = context
            .database
            .query(&current_query, &[Value::String(id.clone())])
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Database query failed: {}", e)})),
                )
            })?;

        if let Some(current_record) = current_results.into_iter().next() {
            // Find the FK value
            let fk_value = current_record.get(&fk.field).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Foreign key field '{}' not found in record", fk.field)
                    })),
                )
            })?;

            // Get the target entity
            let target_entity = context.registry.get(&fk.target_entity).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": format!("Target entity '{}' not found", fk.target_entity)
                    })),
                )
            })?;

            let target_table = target_entity.table_name();
            let target_primary_key = "id";

            // Query the target entity
            let target_query =
                format!("SELECT * FROM {} WHERE {} = ?", target_table, target_primary_key);
            let target_results =
                context.database.query(&target_query, &[fk_value.clone()]).await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Database query failed: {}", e)})),
                    )
                })?;

            if let Some(target_record) = target_results.into_iter().next() {
                return Ok(Json(Value::Object(
                    target_record.into_iter().collect::<serde_json::Map<String, Value>>(),
                )));
            } else {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": format!("Related {} not found", relationship_name)
                    })),
                ));
            }
        } else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("{} with id '{}' not found", entity_name, id)
                })),
            ));
        }
    }

    // Strategy 3: Many-to-many relationship
    // Example: GET /api/users/123/roles -> Get all roles for user 123 via user_roles junction table
    // Check if there's a many-to-many relationship between current entity and relationship name
    for m2m in &entity.schema.many_to_many {
        // Check if relationship_name matches entity_a or entity_b
        let is_entity_a = m2m.entity_a.to_lowercase() == relationship_name.to_lowercase();
        let is_entity_b = m2m.entity_b.to_lowercase() == relationship_name.to_lowercase();

        if is_entity_a || is_entity_b {
            // Get the target entity
            let target_entity_name = if is_entity_a {
                &m2m.entity_a
            } else {
                &m2m.entity_b
            };

            let target_entity = context.registry.get(target_entity_name).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "error": format!("Target entity '{}' not found", target_entity_name)
                    })),
                )
            })?;

            let junction_table = m2m.junction_table.as_ref().ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Junction table name not specified for many-to-many relationship"
                    })),
                )
            })?;

            // Determine which field to use based on which entity we're querying from
            let fk_field = if entity_name.to_lowercase() == m2m.entity_a.to_lowercase() {
                &m2m.entity_a_field
            } else {
                &m2m.entity_b_field
            };

            let target_fk_field = if is_entity_a {
                &m2m.entity_b_field
            } else {
                &m2m.entity_a_field
            };

            let target_table = target_entity.table_name();

            // Build query with JOIN through junction table
            let (where_clause, mut bind_values) = build_where_clause(&params, target_entity)?;
            let order_by = build_order_by(&params, target_entity);
            let (limit, offset) = get_pagination(&params);

            // Build JOIN query
            let mut query = format!(
                "SELECT t.* FROM {} t INNER JOIN {} j ON t.id = j.{} WHERE j.{} = ?",
                target_table, junction_table, target_fk_field, fk_field
            );

            bind_values.push(Value::String(id.clone()));

            if !where_clause.is_empty() {
                query.push_str(&format!(" AND {}", where_clause));
            }

            if !order_by.is_empty() {
                query.push_str(&format!(" {}", order_by));
            }

            if let Some(limit_val) = limit {
                query.push_str(&format!(" LIMIT {}", limit_val));
            }

            if let Some(offset_val) = offset {
                query.push_str(&format!(" OFFSET {}", offset_val));
            }

            let results = context.database.query(&query, &bind_values).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": format!("Database query failed: {}", e)})),
                )
            })?;

            // Get total count
            let count_query = format!(
                "SELECT COUNT(*) as total FROM {} t INNER JOIN {} j ON t.id = j.{} WHERE j.{} = ?",
                target_table, junction_table, target_fk_field, fk_field
            );
            let count_results =
                context.database.query(&count_query, &bind_values).await.map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("Count query failed: {}", e)})),
                    )
                })?;

            let total = count_results
                .first()
                .and_then(|r| r.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            return Ok(Json(json!({
                "data": results,
                "total": total,
                "relationship": relationship_name,
                "parent_entity": entity_name,
                "parent_id": id,
                "relationship_type": "many_to_many"
            })));
        }
    }

    // No relationship found - return error
    Err((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": format!("Relationship '{}' not found for entity '{}'", relationship_name, entity_name)
        })),
    ))
}

/// Create snapshot (POST /vbr-api/snapshots)
#[derive(serde::Deserialize)]
pub struct CreateSnapshotRequest {
    pub name: String,
    pub description: Option<String>,
}

pub async fn create_snapshot_handler(
    Extension(context): Extension<HandlerContext>,
    Json(body): Json<CreateSnapshotRequest>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let snapshots_dir = context.snapshots_dir.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Snapshots directory not configured"})),
        )
    })?;

    let manager = crate::snapshots::SnapshotManager::new(snapshots_dir);
    let metadata = manager
        .create_snapshot(&body.name, body.description, context.database.as_ref(), &context.registry)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to create snapshot: {}", e)})),
            )
        })?;

    Ok(Json(serde_json::to_value(&metadata).unwrap()))
}

/// List snapshots (GET /vbr-api/snapshots)
pub async fn list_snapshots_handler(
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let snapshots_dir = context.snapshots_dir.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Snapshots directory not configured"})),
        )
    })?;

    let manager = crate::snapshots::SnapshotManager::new(snapshots_dir);
    let snapshots = manager.list_snapshots().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to list snapshots: {}", e)})),
        )
    })?;

    Ok(Json(serde_json::to_value(&snapshots).unwrap()))
}

/// Restore snapshot (POST /vbr-api/snapshots/{name}/restore)
pub async fn restore_snapshot_handler(
    Path(name): Path<String>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let snapshots_dir = context.snapshots_dir.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Snapshots directory not configured"})),
        )
    })?;

    let manager = crate::snapshots::SnapshotManager::new(snapshots_dir);
    manager
        .restore_snapshot(&name, context.database.as_ref(), &context.registry)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to restore snapshot: {}", e)})),
            )
        })?;

    Ok(Json(json!({"message": format!("Snapshot '{}' restored successfully", name)})))
}

/// Delete snapshot (DELETE /vbr-api/snapshots/{name})
pub async fn delete_snapshot_handler(
    Path(name): Path<String>,
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    let snapshots_dir = context.snapshots_dir.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Snapshots directory not configured"})),
        )
    })?;

    let manager = crate::snapshots::SnapshotManager::new(snapshots_dir);
    manager.delete_snapshot(&name).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to delete snapshot: {}", e)})),
        )
    })?;

    Ok(Json(json!({"message": format!("Snapshot '{}' deleted successfully", name)})))
}

/// Reset database (POST /vbr-api/reset)
pub async fn reset_handler(
    Extension(context): Extension<HandlerContext>,
) -> std::result::Result<Json<Value>, (StatusCode, Json<Value>)> {
    crate::snapshots::reset_database(context.database.as_ref(), &context.registry)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to reset database: {}", e)})),
            )
        })?;

    Ok(Json(json!({"message": "Database reset successfully"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{InMemoryDatabase, VirtualDatabase};
    use crate::entities::{Entity, EntityRegistry};
    use crate::migration::MigrationManager;
    use crate::schema::VbrSchemaDefinition;
    use mockforge_data::{FieldDefinition, SchemaDefinition};
    use std::sync::Arc;

    async fn setup_test_database() -> (Arc<dyn VirtualDatabase + Send + Sync>, EntityRegistry) {
        let mut db = InMemoryDatabase::new().await.unwrap();
        db.initialize().await.unwrap();
        let registry = EntityRegistry::new();
        (Arc::new(db), registry)
    }

    async fn create_test_entity(
        database: &dyn VirtualDatabase,
        registry: &mut EntityRegistry,
        entity_name: &str,
    ) {
        let base_schema = SchemaDefinition::new(entity_name.to_string())
            .with_field(FieldDefinition::new("id".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));

        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new(entity_name.to_string(), vbr_schema);

        let manager = MigrationManager::new();
        let create_sql = manager.generate_create_table(&entity).unwrap();
        database.create_table(&create_sql).await.unwrap();

        registry.register(entity).unwrap();
    }

    fn create_test_context(
        database: Arc<dyn VirtualDatabase + Send + Sync>,
        registry: EntityRegistry,
    ) -> HandlerContext {
        HandlerContext {
            database,
            registry,
            session_manager: None,
            snapshots_dir: None,
        }
    }

    // SQL identifier validation tests
    #[test]
    fn test_is_safe_sql_identifier_valid() {
        assert!(is_safe_sql_identifier("name"));
        assert!(is_safe_sql_identifier("user_id"));
        assert!(is_safe_sql_identifier("firstName"));
        assert!(is_safe_sql_identifier("field123"));
        assert!(is_safe_sql_identifier("a"));
    }

    #[test]
    fn test_is_safe_sql_identifier_invalid() {
        assert!(!is_safe_sql_identifier("")); // Empty
        assert!(!is_safe_sql_identifier("123abc")); // Starts with digit
        assert!(!is_safe_sql_identifier("field-name")); // Contains hyphen
        assert!(!is_safe_sql_identifier("field.name")); // Contains dot
        assert!(!is_safe_sql_identifier("field; DROP TABLE users")); // SQL injection
        assert!(!is_safe_sql_identifier("field'name")); // Contains quote
        assert!(!is_safe_sql_identifier("field\"name")); // Contains double quote
        assert!(!is_safe_sql_identifier("field name")); // Contains space
        assert!(!is_safe_sql_identifier(&"a".repeat(65))); // Too long
    }

    // Helper function tests
    #[test]
    fn test_build_where_clause_empty() {
        let params = HashMap::new();
        let base_schema = SchemaDefinition::new("Test".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("Test".to_string(), vbr_schema);

        let result = build_where_clause(&params, &entity);
        assert!(result.is_ok());
        let (where_clause, bind_values) = result.unwrap();
        assert_eq!(where_clause, "");
        assert_eq!(bind_values.len(), 0);
    }

    #[test]
    fn test_build_where_clause_with_params() {
        let mut params = HashMap::new();
        params.insert("name".to_string(), "John".to_string());

        let base_schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        let result = build_where_clause(&params, &entity);
        assert!(result.is_ok());
        let (where_clause, bind_values) = result.unwrap();
        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("name = ?"));
        assert_eq!(bind_values.len(), 1);
    }

    #[test]
    fn test_build_where_clause_ignores_pagination() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "10".to_string());
        params.insert("offset".to_string(), "5".to_string());
        params.insert("sort".to_string(), "name".to_string());

        let base_schema = SchemaDefinition::new("Test".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("Test".to_string(), vbr_schema);

        let result = build_where_clause(&params, &entity);
        assert!(result.is_ok());
        let (where_clause, bind_values) = result.unwrap();
        assert_eq!(where_clause, "");
        assert_eq!(bind_values.len(), 0);
    }

    #[test]
    fn test_build_where_clause_ignores_invalid_field_names() {
        let mut params = HashMap::new();
        params.insert("valid_field".to_string(), "value".to_string());
        params.insert("invalid-field".to_string(), "value".to_string()); // Invalid: contains hyphen
        params.insert("name; DROP TABLE".to_string(), "value".to_string()); // SQL injection attempt

        let base_schema = SchemaDefinition::new("Test".to_string())
            .with_field(FieldDefinition::new("valid_field".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("Test".to_string(), vbr_schema);

        let result = build_where_clause(&params, &entity);
        assert!(result.is_ok());
        let (where_clause, bind_values) = result.unwrap();
        // Only valid_field should be included
        assert!(where_clause.contains("valid_field = ?"));
        assert!(!where_clause.contains("invalid-field"));
        assert!(!where_clause.contains("DROP"));
        assert_eq!(bind_values.len(), 1);
    }

    #[test]
    fn test_build_order_by_no_sort() {
        let params = HashMap::new();
        let base_schema = SchemaDefinition::new("Test".to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("Test".to_string(), vbr_schema);

        let order_by = build_order_by(&params, &entity);
        assert_eq!(order_by, "");
    }

    #[test]
    fn test_build_order_by_with_sort() {
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "name".to_string());

        let base_schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        let order_by = build_order_by(&params, &entity);
        assert_eq!(order_by, "ORDER BY name ASC");
    }

    #[test]
    fn test_build_order_by_with_sort_desc() {
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "name".to_string());
        params.insert("order".to_string(), "DESC".to_string());

        let base_schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        let order_by = build_order_by(&params, &entity);
        assert_eq!(order_by, "ORDER BY name DESC");
    }

    #[test]
    fn test_build_order_by_invalid_field() {
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "invalid_field".to_string());

        let base_schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        let order_by = build_order_by(&params, &entity);
        assert_eq!(order_by, "");
    }

    #[test]
    fn test_build_order_by_invalid_order() {
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "name".to_string());
        params.insert("order".to_string(), "INVALID".to_string());

        let base_schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        let entity = Entity::new("User".to_string(), vbr_schema);

        let order_by = build_order_by(&params, &entity);
        assert_eq!(order_by, "");
    }

    #[test]
    fn test_get_pagination_no_params() {
        let params = HashMap::new();
        let (limit, offset) = get_pagination(&params);
        assert_eq!(limit, None);
        assert_eq!(offset, None);
    }

    #[test]
    fn test_get_pagination_with_limit() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "10".to_string());

        let (limit, offset) = get_pagination(&params);
        assert_eq!(limit, Some(10));
        assert_eq!(offset, None);
    }

    #[test]
    fn test_get_pagination_with_limit_and_offset() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "20".to_string());
        params.insert("offset".to_string(), "5".to_string());

        let (limit, offset) = get_pagination(&params);
        assert_eq!(limit, Some(20));
        assert_eq!(offset, Some(5));
    }

    #[test]
    fn test_get_pagination_invalid_values() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "abc".to_string());
        params.insert("offset".to_string(), "xyz".to_string());

        let (limit, offset) = get_pagination(&params);
        assert_eq!(limit, None);
        assert_eq!(offset, None);
    }

    // HandlerContext tests
    #[test]
    fn test_handler_context_clone() {
        let (database, registry) = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { setup_test_database().await });

        let context = create_test_context(database, registry);
        let cloned = context.clone();

        assert!(Arc::ptr_eq(&context.database, &cloned.database));
    }

    // get_entity_info tests
    #[tokio::test]
    async fn test_get_entity_info_success() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let result = get_entity_info(&registry, "User");
        assert!(result.is_ok());
        let (entity, table_name) = result.unwrap();
        assert_eq!(entity.name(), "User");
        assert_eq!(table_name, "users");
    }

    #[tokio::test]
    async fn test_get_entity_info_not_found() {
        let (_database, registry) = setup_test_database().await;

        let result = get_entity_info(&registry, "NonExistent");
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // apply_auto_generation tests
    #[tokio::test]
    async fn test_apply_auto_generation_uuid() {
        let (database, _registry) = setup_test_database().await;
        let mut data = json!({});
        let base_schema = SchemaDefinition::new("Test".to_string());
        let mut vbr_schema = VbrSchemaDefinition::new(base_schema);
        vbr_schema
            .auto_generation
            .insert("id".to_string(), crate::schema::AutoGenerationRule::Uuid);

        apply_auto_generation(&mut data, &vbr_schema, "Test", database.as_ref())
            .await
            .unwrap();

        assert!(data.as_object().unwrap().contains_key("id"));
        assert!(data["id"].is_string());
    }

    #[tokio::test]
    async fn test_apply_auto_generation_timestamp() {
        let (database, _registry) = setup_test_database().await;
        let mut data = json!({});
        let base_schema = SchemaDefinition::new("Test".to_string());
        let mut vbr_schema = VbrSchemaDefinition::new(base_schema);
        vbr_schema
            .auto_generation
            .insert("created_at".to_string(), crate::schema::AutoGenerationRule::Timestamp);

        apply_auto_generation(&mut data, &vbr_schema, "Test", database.as_ref())
            .await
            .unwrap();

        assert!(data.as_object().unwrap().contains_key("created_at"));
        assert!(data["created_at"].is_string());
    }

    #[tokio::test]
    async fn test_apply_auto_generation_date() {
        let (database, _registry) = setup_test_database().await;
        let mut data = json!({});
        let base_schema = SchemaDefinition::new("Test".to_string());
        let mut vbr_schema = VbrSchemaDefinition::new(base_schema);
        vbr_schema
            .auto_generation
            .insert("date_field".to_string(), crate::schema::AutoGenerationRule::Date);

        apply_auto_generation(&mut data, &vbr_schema, "Test", database.as_ref())
            .await
            .unwrap();

        assert!(data.as_object().unwrap().contains_key("date_field"));
        assert!(data["date_field"].is_string());
    }

    #[tokio::test]
    async fn test_apply_auto_generation_skips_existing() {
        let (database, _registry) = setup_test_database().await;
        let mut data = json!({"id": "existing-id"});
        let base_schema = SchemaDefinition::new("Test".to_string());
        let mut vbr_schema = VbrSchemaDefinition::new(base_schema);
        vbr_schema
            .auto_generation
            .insert("id".to_string(), crate::schema::AutoGenerationRule::Uuid);

        apply_auto_generation(&mut data, &vbr_schema, "Test", database.as_ref())
            .await
            .unwrap();

        assert_eq!(data["id"], "existing-id");
    }

    // CreateSnapshotRequest tests
    #[test]
    fn test_create_snapshot_request_deserialize() {
        let json = r#"{"name": "test-snapshot", "description": "Test description"}"#;
        let request: CreateSnapshotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test-snapshot");
        assert_eq!(request.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_create_snapshot_request_deserialize_no_description() {
        let json = r#"{"name": "test-snapshot"}"#;
        let request: CreateSnapshotRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test-snapshot");
        assert_eq!(request.description, None);
    }

    // Integration tests for handlers
    #[tokio::test]
    async fn test_list_handler_empty() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);
        let params = HashMap::new();

        let result =
            list_handler(Path("User".to_string()), Query(params), Extension(context)).await;

        assert!(result.is_ok());
        let json_value = result.unwrap().0;
        assert!(json_value["data"].is_array());
        assert_eq!(json_value["total"], 0);
    }

    #[tokio::test]
    async fn test_list_handler_entity_not_found() {
        let (database, registry) = setup_test_database().await;
        let context = create_test_context(database, registry);
        let params = HashMap::new();

        let result =
            list_handler(Path("NonExistent".to_string()), Query(params), Extension(context)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_handler_not_found() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);

        let result = get_handler(
            Path(("User".to_string(), "nonexistent-id".to_string())),
            Extension(context),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_handler_invalid_body() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);
        let body = json!("not an object");

        let result = create_handler(Path("User".to_string()), Extension(context), Json(body)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_handler_not_found() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);
        let body = json!({"name": "Updated"});

        let result = update_handler(
            Path(("User".to_string(), "nonexistent-id".to_string())),
            Extension(context),
            Json(body),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_handler_invalid_body() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);
        let body = json!("not an object");

        let result = update_handler(
            Path(("User".to_string(), "some-id".to_string())),
            Extension(context),
            Json(body),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_handler_not_found() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);

        let result = delete_handler(
            Path(("User".to_string(), "nonexistent-id".to_string())),
            Extension(context),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_relationship_handler_entity_not_found() {
        let (database, registry) = setup_test_database().await;
        let context = create_test_context(database, registry);
        let params = HashMap::new();

        let result = get_relationship_handler(
            Path(("User".to_string(), "123".to_string(), "orders".to_string())),
            Query(params),
            Extension(context),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_snapshot_handlers_no_directory() {
        let (database, registry) = setup_test_database().await;
        let context = create_test_context(database, registry);

        // Test create_snapshot_handler without snapshots_dir
        let body = CreateSnapshotRequest {
            name: "test".to_string(),
            description: None,
        };
        let result = create_snapshot_handler(Extension(context.clone()), Json(body)).await;
        assert!(result.is_err());

        // Test list_snapshots_handler without snapshots_dir
        let result = list_snapshots_handler(Extension(context.clone())).await;
        assert!(result.is_err());

        // Test restore_snapshot_handler without snapshots_dir
        let result =
            restore_snapshot_handler(Path("test".to_string()), Extension(context.clone())).await;
        assert!(result.is_err());

        // Test delete_snapshot_handler without snapshots_dir
        let result = delete_snapshot_handler(Path("test".to_string()), Extension(context)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reset_handler_success() {
        let (database, mut registry) = setup_test_database().await;
        create_test_entity(database.as_ref(), &mut registry, "User").await;

        let context = create_test_context(database, registry);

        let result = reset_handler(Extension(context)).await;
        assert!(result.is_ok());
        let json_value = result.unwrap().0;
        assert!(json_value["message"].as_str().unwrap().contains("reset successfully"));
    }
}
