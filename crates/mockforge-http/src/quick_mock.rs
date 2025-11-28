//! Quick Mock Mode
//!
//! Provides instant REST API mocking from JSON files with auto-route detection.
//! Perfect for rapid prototyping and testing without configuration.
//!
//! # Features
//! - Zero configuration setup
//! - Auto-detection of routes from JSON keys
//! - Dynamic data generation with tokens ($random, $faker, $ai)
//! - Full CRUD operations on all detected resources
//! - **Pagination** support with `page` and `limit` parameters
//! - **Filtering** support with any field as query parameter
//! - **Sorting** support with `sort=field:direction` syntax
//!
//! # Example
//! ```json
//! {
//!   "users": [
//!     {"id": "$random.uuid", "name": "$faker.name", "email": "$faker.email", "role": "admin"}
//!   ],
//!   "posts": [
//!     {"id": "$random.int", "title": "Sample Post", "content": "$faker.paragraph"}
//!   ]
//! }
//! ```
//!
//! ## Auto-generated Endpoints
//! - GET /users - List all users
//! - GET /users/:id - Get single user
//! - POST /users - Create user
//! - PUT /users/:id - Update user
//! - DELETE /users/:id - Delete user
//! - Same for /posts
//!
//! ## Query Parameters
//! - `?page=2&limit=10` - Pagination (page is 1-indexed, limit 1-1000)
//! - `?role=admin` - Filter by field value
//! - `?sort=name:asc` - Sort by field (asc/desc)
//! - `?role=admin&sort=name:desc&page=1&limit=20` - Combined filtering, sorting, and pagination
//!
//! ## Response Format
//! List endpoints return:
//! ```json
//! {
//!   "data": [...],
//!   "pagination": {
//!     "page": 1,
//!     "limit": 50,
//!     "total": 100,
//!     "totalPages": 2,
//!     "hasNext": true,
//!     "hasPrev": false
//!   }
//! }
//! ```

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use mockforge_data::token_resolver::TokenResolver;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Query parameters for list endpoints
#[derive(Debug, Deserialize, Serialize)]
pub struct ListQueryParams {
    /// Page number (1-indexed)
    #[serde(default)]
    page: Option<usize>,
    /// Number of items per page
    #[serde(default)]
    limit: Option<usize>,
    /// Sort field and direction (e.g., "name:asc", "created:desc")
    #[serde(default)]
    sort: Option<String>,
    /// Filter parameters (dynamic, any field can be filtered)
    #[serde(flatten)]
    filters: HashMap<String, String>,
}

impl Default for ListQueryParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(50),
            sort: None,
            filters: HashMap::new(),
        }
    }
}

/// Quick mock state holding the data store
#[derive(Clone)]
pub struct QuickMockState {
    /// Data store: resource_name -> Vec<Value>
    data: Arc<RwLock<HashMap<String, Vec<Value>>>>,
    /// Token resolver for dynamic data generation
    resolver: Arc<TokenResolver>,
}

impl QuickMockState {
    /// Create a new quick mock state
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            resolver: Arc::new(TokenResolver::new()),
        }
    }

    /// Initialize from JSON file data
    pub async fn from_json(json_data: Value) -> Result<Self, String> {
        let state = Self::new();

        if let Value::Object(obj) = json_data {
            let mut data = state.data.write().await;

            for (key, value) in obj {
                // Only process arrays at root level as resources
                if let Value::Array(arr) = value {
                    // Resolve tokens in the data
                    let mut resolved_items = Vec::new();
                    for item in arr {
                        match state.resolver.resolve(&item).await {
                            Ok(resolved) => resolved_items.push(resolved),
                            Err(e) => {
                                eprintln!("Warning: Failed to resolve tokens in {}: {}", key, e)
                            }
                        }
                    }
                    data.insert(key, resolved_items);
                } else {
                    // Single object resources
                    match state.resolver.resolve(&value).await {
                        Ok(resolved) => {
                            data.insert(key, vec![resolved]);
                        }
                        Err(e) => eprintln!("Warning: Failed to resolve tokens in {}: {}", key, e),
                    }
                }
            }
        }

        Ok(state)
    }

    /// Get all resource names
    pub async fn resource_names(&self) -> Vec<String> {
        let data = self.data.read().await;
        data.keys().cloned().collect()
    }
}

impl Default for QuickMockState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a router with auto-detected routes from quick mock state
pub async fn build_quick_router(state: QuickMockState) -> Router {
    let resource_names = state.resource_names().await;
    let mut router = Router::new();

    for resource in resource_names {
        // Create nested router for this resource
        let resource_router = Router::new()
            .route(
                "/",
                get({
                    let resource = resource.clone();
                    move |State(state): State<QuickMockState>,
                          Query(params): Query<ListQueryParams>| {
                        let resource = resource.clone();
                        async move { list_handler_impl(state, resource, params).await }
                    }
                })
                .post({
                    let resource = resource.clone();
                    move |State(state): State<QuickMockState>, Json(payload): Json<Value>| {
                        let resource = resource.clone();
                        async move { create_handler_impl(state, resource, payload).await }
                    }
                }),
            )
            .route(
                "/{id}",
                get({
                    let resource = resource.clone();
                    move |State(state): State<QuickMockState>, Path(id): Path<String>| {
                        let resource = resource.clone();
                        async move { get_handler_impl(state, resource, id).await }
                    }
                })
                .put({
                    let resource = resource.clone();
                    move |State(state): State<QuickMockState>,
                          Path(id): Path<String>,
                          Json(payload): Json<Value>| {
                        let resource = resource.clone();
                        async move { update_handler_impl(state, resource, id, payload).await }
                    }
                })
                .delete({
                    let resource = resource.clone();
                    move |State(state): State<QuickMockState>, Path(id): Path<String>| {
                        let resource = resource.clone();
                        async move { delete_handler_impl(state, resource, id).await }
                    }
                }),
            );

        router = router.nest(&format!("/{}", resource), resource_router);

        println!("  ✓ Registered routes for /{}", resource);
    }

    // Add info endpoint
    router = router.route("/__quick/info", get(info_handler));

    router.with_state(state)
}

/// Implementation for listing all items in a resource with pagination, filtering, and sorting
async fn list_handler_impl(
    state: QuickMockState,
    resource: String,
    params: ListQueryParams,
) -> Result<Json<Value>, StatusCode> {
    let data = state.data.read().await;

    if let Some(items) = data.get(&resource) {
        let mut filtered_items: Vec<&Value> = items.iter().collect();

        // Apply filters
        for (key, value) in &params.filters {
            // Skip pagination, sorting, and limit params
            if key == "page" || key == "limit" || key == "sort" {
                continue;
            }

            filtered_items.retain(|item| {
                if let Some(field_value) = item.get(key) {
                    // Support both string and other type comparisons
                    match field_value {
                        Value::String(s) => s.contains(value),
                        Value::Number(n) => n.to_string() == *value,
                        Value::Bool(b) => b.to_string() == *value,
                        _ => false,
                    }
                } else {
                    false
                }
            });
        }

        // Apply sorting
        if let Some(sort) = &params.sort {
            let parts: Vec<&str> = sort.split(':').collect();
            let field = parts.first().unwrap_or(&"id");
            let direction = parts.get(1).unwrap_or(&"asc");

            filtered_items.sort_by(|a, b| {
                let a_val = a.get(*field);
                let b_val = b.get(*field);

                let cmp = match (a_val, b_val) {
                    (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
                    (Some(Value::Number(a)), Some(Value::Number(b))) => {
                        if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                            a_f.partial_cmp(&b_f).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    }
                    (Some(Value::Bool(a)), Some(Value::Bool(b))) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                };

                if *direction == "desc" {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        // Get total count before pagination
        let total = filtered_items.len();

        // Apply pagination
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(50).clamp(1, 1000);
        let offset = (page - 1) * limit;

        let paginated_items: Vec<Value> =
            filtered_items.into_iter().skip(offset).take(limit).cloned().collect();

        let total_pages = total.div_ceil(limit);

        Ok(Json(json!({
            "data": paginated_items,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": total,
                "totalPages": total_pages,
                "hasNext": page < total_pages,
                "hasPrev": page > 1
            }
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Implementation for getting a single item by ID
async fn get_handler_impl(
    state: QuickMockState,
    resource: String,
    id: String,
) -> Result<Json<Value>, StatusCode> {
    let data = state.data.read().await;

    if let Some(items) = data.get(&resource) {
        // Try to find by id field
        for item in items {
            if let Some(item_id) = item.get("id") {
                // Support both string and number IDs
                let matches = match item_id {
                    Value::String(s) => s == &id,
                    Value::Number(n) => n.to_string() == id,
                    _ => false,
                };

                if matches {
                    return Ok(Json(item.clone()));
                }
            }
        }

        // Try index-based access if no id field found
        if let Ok(index) = id.parse::<usize>() {
            if let Some(item) = items.get(index) {
                return Ok(Json(item.clone()));
            }
        }

        Err(StatusCode::NOT_FOUND)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Implementation for creating a new item
async fn create_handler_impl(
    state: QuickMockState,
    resource: String,
    mut payload: Value,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // Resolve tokens in the payload
    payload = state
        .resolver
        .resolve(&payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut data = state.data.write().await;

    if let Some(items) = data.get_mut(&resource) {
        // Auto-generate ID if not provided
        if payload.get("id").is_none() {
            let new_id = items.len() + 1;
            if let Value::Object(obj) = &mut payload {
                obj.insert("id".to_string(), json!(new_id));
            }
        }

        items.push(payload.clone());
        Ok((StatusCode::CREATED, Json(payload)))
    } else {
        // Create new resource
        let mut new_payload = payload.clone();
        if new_payload.get("id").is_none() {
            if let Value::Object(obj) = &mut new_payload {
                obj.insert("id".to_string(), json!(1));
            }
        }

        data.insert(resource, vec![new_payload.clone()]);
        Ok((StatusCode::CREATED, Json(new_payload)))
    }
}

/// Implementation for updating an item
async fn update_handler_impl(
    state: QuickMockState,
    resource: String,
    id: String,
    mut payload: Value,
) -> Result<Json<Value>, StatusCode> {
    // Resolve tokens in the payload
    payload = state
        .resolver
        .resolve(&payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut data = state.data.write().await;

    if let Some(items) = data.get_mut(&resource) {
        // Try to find and update by id field
        for item in items.iter_mut() {
            if let Some(item_id) = item.get("id") {
                let matches = match item_id {
                    Value::String(s) => s == &id,
                    Value::Number(n) => n.to_string() == id,
                    _ => false,
                };

                if matches {
                    *item = payload.clone();
                    return Ok(Json(payload));
                }
            }
        }

        // Try index-based update if no id field found
        if let Ok(index) = id.parse::<usize>() {
            if let Some(item) = items.get_mut(index) {
                *item = payload.clone();
                return Ok(Json(payload));
            }
        }

        Err(StatusCode::NOT_FOUND)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Implementation for deleting an item
async fn delete_handler_impl(
    state: QuickMockState,
    resource: String,
    id: String,
) -> Result<StatusCode, StatusCode> {
    let mut data = state.data.write().await;

    if let Some(items) = data.get_mut(&resource) {
        // Try to find and delete by id field
        let original_len = items.len();
        items.retain(|item| {
            if let Some(item_id) = item.get("id") {
                let matches = match item_id {
                    Value::String(s) => s == &id,
                    Value::Number(n) => n.to_string() == id,
                    _ => false,
                };
                !matches
            } else {
                true
            }
        });

        if items.len() < original_len {
            return Ok(StatusCode::NO_CONTENT);
        }

        // Try index-based deletion if no id field found
        if let Ok(index) = id.parse::<usize>() {
            if index < items.len() {
                items.remove(index);
                return Ok(StatusCode::NO_CONTENT);
            }
        }

        Err(StatusCode::NOT_FOUND)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Handler for getting quick mock info
async fn info_handler(State(state): State<QuickMockState>) -> Json<Value> {
    let data = state.data.read().await;
    let mut resources = HashMap::new();

    for (name, items) in data.iter() {
        resources.insert(
            name.clone(),
            json!({
                "count": items.len(),
                "endpoints": {
                    "list": format!("GET /{}", name),
                    "get": format!("GET /{}/:id or GET /{}/{{id}}", name, name),
                    "create": format!("POST /{}", name),
                    "update": format!("PUT /{}/:id or PUT /{}/{{id}}", name, name),
                    "delete": format!("DELETE /{}/:id or DELETE /{}/{{id}}", name, name),
                },
                "queryParams": {
                    "page": "Page number (1-indexed, default: 1)",
                    "limit": "Items per page (1-1000, default: 50)",
                    "sort": "Sort field and direction (e.g., name:asc, id:desc)",
                    "filters": "Any field can be used as filter (e.g., ?role=admin&status=active)"
                },
                "examples": {
                    "pagination": format!("GET /{}?page=2&limit=10", name),
                    "filtering": format!("GET /{}?name=Alice", name),
                    "sorting": format!("GET /{}?sort=name:asc", name),
                    "combined": format!("GET /{}?role=admin&sort=name:desc&page=1&limit=20", name)
                }
            }),
        );
    }

    Json(json!({
        "mode": "quick",
        "version": "1.1.0",
        "features": [
            "CRUD operations",
            "Pagination",
            "Filtering",
            "Sorting",
            "Dynamic token resolution ($random, $faker, $ai)"
        ],
        "resources": resources,
        "info": "/__quick/info (this endpoint)"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_quick_mock_from_json() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "posts": [
                {"id": 1, "title": "First Post"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let resource_names = state.resource_names().await;

        assert_eq!(resource_names.len(), 2);
        assert!(resource_names.contains(&"users".to_string()));
        assert!(resource_names.contains(&"posts".to_string()));
    }

    #[tokio::test]
    async fn test_list_handler() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(Request::builder().uri("/users").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_handler() {
        let json_data = json!({
            "users": []
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"Charlie"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_pagination() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
                {"id": 3, "name": "Charlie"},
                {"id": 4, "name": "David"},
                {"id": 5, "name": "Eve"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        // Test first page with limit
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/users?page=1&limit=2").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test second page
        let response = router
            .oneshot(Request::builder().uri("/users?page=2&limit=2").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_filtering() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice", "role": "admin"},
                {"id": 2, "name": "Bob", "role": "user"},
                {"id": 3, "name": "Charlie", "role": "admin"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(Request::builder().uri("/users?role=admin").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_sorting() {
        let json_data = json!({
            "users": [
                {"id": 3, "name": "Charlie"},
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        // Test ascending sort
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/users?sort=name:asc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test descending sort
        let response = router
            .oneshot(Request::builder().uri("/users?sort=name:desc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(Request::builder().uri("/users/1").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_handler() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/users/1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":1,"name":"Alice Updated"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_handler() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        let response = router
            .oneshot(
                Request::builder().method("DELETE").uri("/users/1").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_combined_query_params() {
        let json_data = json!({
            "users": [
                {"id": 1, "name": "Alice", "role": "admin"},
                {"id": 2, "name": "Bob", "role": "user"},
                {"id": 3, "name": "Charlie", "role": "admin"},
                {"id": 4, "name": "David", "role": "user"}
            ]
        });

        let state = QuickMockState::from_json(json_data).await.unwrap();
        let router = build_quick_router(state).await;

        // Test combined filtering, sorting, and pagination
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/users?role=admin&sort=name:asc&page=1&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
