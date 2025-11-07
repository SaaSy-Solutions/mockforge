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
use uuid::Uuid;

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
fn build_where_clause(
    params: &HashMap<String, String>,
    entity: &crate::entities::Entity,
) -> (String, Vec<Value>) {
    let mut conditions = Vec::new();
    let mut bind_values = Vec::new();

    for (key, value) in params {
        // Skip pagination and sorting parameters
        if matches!(key.as_str(), "limit" | "offset" | "sort" | "order") {
            continue;
        }

        // Check if field exists in schema
        if entity.schema.base.fields.iter().any(|f| f.name == *key) {
            conditions.push(format!("{} = ?", key));
            bind_values.push(Value::String(value.clone()));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    (where_clause, bind_values)
}

/// Helper function to build ORDER BY clause
fn build_order_by(params: &HashMap<String, String>, entity: &crate::entities::Entity) -> String {
    if let Some(sort_field) = params.get("sort") {
        // Validate sort field exists
        if entity.schema.base.fields.iter().any(|f| f.name == *sort_field) {
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

    // Build query
    let (where_clause, mut bind_values) = build_where_clause(&params, entity);
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
            result
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect::<serde_json::Map<String, Value>>(),
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
        let fields: Vec<String> = obj.keys().cloned().collect();
        let placeholders: Vec<String> = (0..fields.len()).map(|_| "?".to_string()).collect();
        let values: Vec<Value> =
            fields.iter().map(|f| obj.get(f).cloned().unwrap_or(Value::Null)).collect();

        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            fields.join(", "),
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
                result
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect::<serde_json::Map<String, Value>>(),
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
                // Don't update primary key
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
                result
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect::<serde_json::Map<String, Value>>(),
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
            let (where_clause, mut bind_values) = build_where_clause(&params, target_entity);
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
                    target_record
                        .into_iter()
                        .map(|(k, v)| (k, v))
                        .collect::<serde_json::Map<String, Value>>(),
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
            let (where_clause, mut bind_values) = build_where_clause(&params, target_entity);
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
