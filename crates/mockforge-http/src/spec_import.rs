//! Specification Import API
//!
//! Provides REST endpoints for importing OpenAPI and AsyncAPI specifications
//! and automatically generating mock endpoints.

use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use mockforge_core::import::asyncapi_import::{import_asyncapi_spec, AsyncApiImportResult};
use mockforge_core::import::openapi_import::{import_openapi_spec, OpenApiImportResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::*;

/// Specification metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecMetadata {
    /// Unique identifier for the specification
    pub id: String,
    /// Human-readable name of the specification
    pub name: String,
    /// Type of specification (OpenAPI or AsyncAPI)
    pub spec_type: SpecType,
    /// Specification version
    pub version: String,
    /// Optional description from the spec
    pub description: Option<String>,
    /// List of server URLs from the spec
    pub servers: Vec<String>,
    /// ISO 8601 timestamp when the spec was uploaded
    pub uploaded_at: String,
    /// Number of routes/channels generated from this spec
    pub route_count: usize,
}

/// Specification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpecType {
    /// OpenAPI/Swagger specification
    OpenApi,
    /// AsyncAPI specification
    AsyncApi,
}

/// Import request body for uploading a specification
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportSpecRequest {
    /// Specification content (YAML or JSON)
    pub spec_content: String,
    /// Optional specification type (auto-detected if not provided)
    pub spec_type: Option<SpecType>,
    /// Optional custom name for the specification
    pub name: Option<String>,
    /// Optional base URL override
    pub base_url: Option<String>,
    /// Whether to automatically generate mock endpoints (default: true)
    pub auto_generate_mocks: Option<bool>,
}

/// Response from specification import
#[derive(Debug, Serialize)]
pub struct ImportSpecResponse {
    /// ID of the imported specification
    pub spec_id: String,
    /// Type of specification that was imported
    pub spec_type: SpecType,
    /// Number of routes/channels generated
    pub routes_generated: usize,
    /// Warnings encountered during import
    pub warnings: Vec<String>,
    /// Coverage statistics for the imported spec
    pub coverage: CoverageStats,
}

/// Coverage statistics for imported specification
#[derive(Debug, Serialize)]
pub struct CoverageStats {
    /// Total number of endpoints in the specification
    pub total_endpoints: usize,
    /// Number of endpoints that were successfully mocked
    pub mocked_endpoints: usize,
    /// Coverage percentage (0-100)
    pub coverage_percentage: u32,
    /// Breakdown by HTTP method (for OpenAPI) or operation type (for AsyncAPI)
    pub by_method: HashMap<String, usize>,
}

/// Query parameters for listing specifications
#[derive(Debug, Deserialize)]
pub struct ListSpecsQuery {
    /// Optional filter by specification type
    pub spec_type: Option<SpecType>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Shared state for spec import API
#[derive(Clone)]
pub struct SpecImportState {
    /// Map of spec ID to stored specification
    pub specs: Arc<RwLock<HashMap<String, StoredSpec>>>,
}

/// Stored specification with metadata and routes
#[derive(Debug, Clone)]
pub struct StoredSpec {
    /// Specification metadata
    pub metadata: SpecMetadata,
    /// Original specification content
    pub content: String,
    /// Serialized routes/channels as JSON
    pub routes_json: String,
}

impl SpecImportState {
    /// Create a new specification import state
    pub fn new() -> Self {
        Self {
            specs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for SpecImportState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create spec import router
pub fn spec_import_router(state: SpecImportState) -> Router {
    Router::new()
        .route("/specs", post(import_spec))
        .route("/specs", get(list_specs))
        .route("/specs/{id}", get(get_spec))
        .route("/specs/{id}", delete(delete_spec))
        .route("/specs/{id}/coverage", get(get_spec_coverage))
        .route("/specs/{id}/routes", get(get_spec_routes))
        .route("/specs/upload", post(upload_spec_file))
        .with_state(state)
}

/// Import a specification from JSON body
#[instrument(skip(state, payload))]
async fn import_spec(
    State(state): State<SpecImportState>,
    Json(payload): Json<ImportSpecRequest>,
) -> Result<Json<ImportSpecResponse>, (StatusCode, String)> {
    info!("Importing specification");

    // Auto-detect spec type if not provided
    let spec_type = if let Some(st) = payload.spec_type {
        st
    } else {
        detect_spec_type(&payload.spec_content)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to detect spec type: {}", e)))?
    };

    // Convert YAML to JSON if needed
    let json_content = if is_yaml(&payload.spec_content) {
        yaml_to_json(&payload.spec_content)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to parse YAML: {}", e)))?
    } else {
        payload.spec_content.clone()
    };

    // Import based on type
    let (metadata, openapi_result, asyncapi_result) = match spec_type {
        SpecType::OpenApi => {
            let result =
                import_openapi_spec(&json_content, payload.base_url.as_deref()).map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to import OpenAPI spec: {}", e))
                })?;

            let metadata = SpecMetadata {
                id: generate_spec_id(),
                name: payload.name.unwrap_or_else(|| result.spec_info.title.clone()),
                spec_type: SpecType::OpenApi,
                version: result.spec_info.version.clone(),
                description: result.spec_info.description.clone(),
                servers: result.spec_info.servers.clone(),
                uploaded_at: chrono::Utc::now().to_rfc3339(),
                route_count: result.routes.len(),
            };

            (metadata, Some(result), None)
        }
        SpecType::AsyncApi => {
            let result = import_asyncapi_spec(&payload.spec_content, payload.base_url.as_deref())
                .map_err(|e| {
                (StatusCode::BAD_REQUEST, format!("Failed to import AsyncAPI spec: {}", e))
            })?;

            let metadata = SpecMetadata {
                id: generate_spec_id(),
                name: payload.name.unwrap_or_else(|| result.spec_info.title.clone()),
                spec_type: SpecType::AsyncApi,
                version: result.spec_info.version.clone(),
                description: result.spec_info.description.clone(),
                servers: result.spec_info.servers.clone(),
                uploaded_at: chrono::Utc::now().to_rfc3339(),
                route_count: result.channels.len(),
            };

            (metadata, None, Some(result))
        }
    };

    let spec_id = metadata.id.clone();

    // Build response and serialize routes
    let (routes_generated, warnings, coverage, routes_json) =
        if let Some(ref result) = openapi_result {
            let coverage = calculate_openapi_coverage(result);
            let routes_json = serde_json::to_string(&result.routes).map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize routes: {}", e))
            })?;
            (result.routes.len(), result.warnings.clone(), coverage, routes_json)
        } else if let Some(ref result) = asyncapi_result {
            let coverage = calculate_asyncapi_coverage(result);
            let routes_json = serde_json::to_string(&result.channels).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to serialize channels: {}", e),
                )
            })?;
            (result.channels.len(), result.warnings.clone(), coverage, routes_json)
        } else {
            (
                0,
                vec![],
                CoverageStats {
                    total_endpoints: 0,
                    mocked_endpoints: 0,
                    coverage_percentage: 0,
                    by_method: HashMap::new(),
                },
                "[]".to_string(),
            )
        };

    // Store the spec
    let stored_spec = StoredSpec {
        metadata: metadata.clone(),
        content: payload.spec_content,
        routes_json,
    };

    state.specs.write().await.insert(spec_id.clone(), stored_spec);

    info!("Specification imported successfully: {}", spec_id);

    Ok(Json(ImportSpecResponse {
        spec_id,
        spec_type,
        routes_generated,
        warnings,
        coverage,
    }))
}

/// Upload a specification file (multipart form data)
#[instrument(skip(state, multipart))]
async fn upload_spec_file(
    State(state): State<SpecImportState>,
    mut multipart: Multipart,
) -> Result<Json<ImportSpecResponse>, (StatusCode, String)> {
    info!("Uploading specification file");

    let mut spec_content = None;
    let mut name = None;
    let mut base_url = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                let data = field.bytes().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e))
                })?;
                spec_content = Some(
                    String::from_utf8(data.to_vec())
                        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid UTF-8: {}", e)))?,
                );
            }
            "name" => {
                name = Some(field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read name: {}", e))
                })?);
            }
            "base_url" => {
                base_url = Some(field.text().await.map_err(|e| {
                    (StatusCode::BAD_REQUEST, format!("Failed to read base_url: {}", e))
                })?);
            }
            _ => {}
        }
    }

    let spec_content =
        spec_content.ok_or((StatusCode::BAD_REQUEST, "Missing 'file' field".to_string()))?;

    // Call import_spec with the extracted data
    let request = ImportSpecRequest {
        spec_content,
        spec_type: None,
        name,
        base_url,
        auto_generate_mocks: Some(true),
    };

    import_spec(State(state), Json(request)).await
}

/// List all imported specifications
#[instrument(skip(state))]
async fn list_specs(
    State(state): State<SpecImportState>,
    Query(params): Query<ListSpecsQuery>,
) -> Json<Vec<SpecMetadata>> {
    let specs = state.specs.read().await;

    let mut metadata_list: Vec<SpecMetadata> = specs
        .values()
        .filter(|spec| {
            if let Some(ref spec_type) = params.spec_type {
                &spec.metadata.spec_type == spec_type
            } else {
                true
            }
        })
        .map(|spec| spec.metadata.clone())
        .collect();

    // Sort by uploaded_at descending
    metadata_list.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

    // Apply pagination
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100);

    let paginated: Vec<SpecMetadata> = metadata_list.into_iter().skip(offset).take(limit).collect();

    Json(paginated)
}

/// Get specification details
#[instrument(skip(state))]
async fn get_spec(
    State(state): State<SpecImportState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SpecMetadata>, StatusCode> {
    let specs = state.specs.read().await;

    specs
        .get(&id)
        .map(|spec| Json(spec.metadata.clone()))
        .ok_or(StatusCode::NOT_FOUND)
}

/// Delete a specification
#[instrument(skip(state))]
async fn delete_spec(
    State(state): State<SpecImportState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut specs = state.specs.write().await;

    if specs.remove(&id).is_some() {
        info!("Deleted specification: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Get coverage statistics for a spec
#[instrument(skip(state))]
async fn get_spec_coverage(
    State(state): State<SpecImportState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<CoverageStats>, StatusCode> {
    let specs = state.specs.read().await;

    let spec = specs.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    // Parse routes to recalculate coverage
    let coverage = match spec.metadata.spec_type {
        SpecType::OpenApi => {
            // For now, return basic stats based on metadata
            CoverageStats {
                total_endpoints: spec.metadata.route_count,
                mocked_endpoints: spec.metadata.route_count,
                coverage_percentage: 100,
                by_method: HashMap::new(),
            }
        }
        SpecType::AsyncApi => CoverageStats {
            total_endpoints: spec.metadata.route_count,
            mocked_endpoints: spec.metadata.route_count,
            coverage_percentage: 100,
            by_method: HashMap::new(),
        },
    };

    Ok(Json(coverage))
}

/// Get routes generated from a spec
#[instrument(skip(state))]
async fn get_spec_routes(
    State(state): State<SpecImportState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let specs = state.specs.read().await;

    let spec = specs.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    let routes: serde_json::Value =
        serde_json::from_str(&spec.routes_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(routes))
}

// Helper functions

fn generate_spec_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("spec-{}", timestamp)
}

fn detect_spec_type(content: &str) -> Result<SpecType, String> {
    // Try parsing as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if json.get("openapi").is_some() {
            return Ok(SpecType::OpenApi);
        } else if json.get("asyncapi").is_some() {
            return Ok(SpecType::AsyncApi);
        }
    }

    // Try parsing as YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(content) {
        if yaml.get("openapi").is_some() {
            return Ok(SpecType::OpenApi);
        } else if yaml.get("asyncapi").is_some() {
            return Ok(SpecType::AsyncApi);
        }
    }

    Err("Unable to detect specification type".to_string())
}

fn is_yaml(content: &str) -> bool {
    // Simple heuristic: if it doesn't start with '{' or '[', assume YAML
    let trimmed = content.trim_start();
    !trimmed.starts_with('{') && !trimmed.starts_with('[')
}

fn yaml_to_json(yaml_content: &str) -> Result<String, String> {
    let yaml_value: serde_json::Value =
        serde_yaml::from_str(yaml_content).map_err(|e| format!("Failed to parse YAML: {}", e))?;
    serde_json::to_string(&yaml_value).map_err(|e| format!("Failed to convert to JSON: {}", e))
}

fn calculate_openapi_coverage(result: &OpenApiImportResult) -> CoverageStats {
    let total_endpoints = result.routes.len();
    let mocked_endpoints = result.routes.len(); // All routes have mocks

    let mut by_method = HashMap::new();
    for route in &result.routes {
        *by_method.entry(route.method.clone()).or_insert(0) += 1;
    }

    CoverageStats {
        total_endpoints,
        mocked_endpoints,
        coverage_percentage: 100,
        by_method,
    }
}

fn calculate_asyncapi_coverage(result: &AsyncApiImportResult) -> CoverageStats {
    let total_endpoints = result.channels.len();
    let mocked_endpoints = result.channels.len();

    let mut by_method = HashMap::new();
    for channel in &result.channels {
        let protocol = format!("{:?}", channel.protocol);
        *by_method.entry(protocol).or_insert(0) += 1;
    }

    CoverageStats {
        total_endpoints,
        mocked_endpoints,
        coverage_percentage: 100,
        by_method,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_openapi_json() {
        let content = r#"{"openapi": "3.0.0", "info": {"title": "Test", "version": "1.0.0"}}"#;
        assert_eq!(detect_spec_type(content).unwrap(), SpecType::OpenApi);
    }

    #[test]
    fn test_detect_asyncapi_json() {
        let content = r#"{"asyncapi": "2.0.0", "info": {"title": "Test", "version": "1.0.0"}}"#;
        assert_eq!(detect_spec_type(content).unwrap(), SpecType::AsyncApi);
    }

    #[test]
    fn test_is_yaml() {
        assert!(is_yaml("openapi: 3.0.0"));
        assert!(!is_yaml("{\"openapi\": \"3.0.0\"}"));
        assert!(!is_yaml("[1, 2, 3]"));
    }
}
