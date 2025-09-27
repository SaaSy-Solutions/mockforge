//! Workspace import utilities
//!
//! This module provides functionality to automatically organize imported API definitions
//! into workspace and folder structures based on the source format and content.

use crate::import::curl_import::MockForgeRoute as CurlRoute;
use crate::import::har_import::MockForgeRoute as HarRoute;
use crate::import::insomnia_import::MockForgeRoute as InsomniaRoute;
use crate::import::openapi_import::MockForgeRoute as OpenApiRoute;
use crate::import::postman_import::{
    ImportResult as PostmanImportResult, MockForgeRoute as PostmanRoute,
};
use crate::routing::HttpMethod;
use crate::workspace::{MockRequest, MockResponse, Workspace, WorkspaceRegistry};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Common import route structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRoute {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub response: ImportResponse,
}

/// Common import response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// Import configuration for workspace organization
#[derive(Debug, Clone)]
pub struct WorkspaceImportConfig {
    /// Whether to create folders based on collection structure
    pub create_folders: bool,
    /// Base folder name for imported requests
    pub base_folder_name: Option<String>,
    /// Whether to preserve source hierarchy
    pub preserve_hierarchy: bool,
    /// Maximum folder depth
    pub max_depth: usize,
}

impl Default for WorkspaceImportConfig {
    fn default() -> Self {
        Self {
            create_folders: true,
            base_folder_name: None,
            preserve_hierarchy: true,
            max_depth: 5,
        }
    }
}

/// Import result into workspace structure
#[derive(Debug)]
pub struct WorkspaceImportResult {
    /// Created workspace
    pub workspace: Workspace,
    /// Number of requests imported
    pub request_count: usize,
    /// Number of folders created
    pub folder_count: usize,
    /// Import warnings
    pub warnings: Vec<String>,
}

/// Convert PostmanRoute to common ImportRoute
fn postman_route_to_import_route(route: PostmanRoute) -> ImportRoute {
    ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }
}

/// Convert InsomniaRoute to common ImportRoute
fn insomnia_route_to_import_route(route: InsomniaRoute) -> ImportRoute {
    ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }
}

/// Convert CurlRoute to common ImportRoute
fn curl_route_to_import_route(route: CurlRoute) -> ImportRoute {
    ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }
}

/// Convert OpenApiRoute to common ImportRoute
fn openapi_route_to_import_route(route: OpenApiRoute) -> ImportRoute {
    ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }
}

/// Convert HarRoute to common ImportRoute
fn har_route_to_import_route(route: HarRoute) -> ImportRoute {
    ImportRoute {
        method: route.method,
        path: route.path,
        headers: route.headers,
        body: route.body,
        response: ImportResponse {
            status: route.response.status,
            headers: route.response.headers,
            body: route.response.body,
        },
    }
}

/// Import routes into a new workspace
pub fn import_postman_to_workspace(
    routes: Vec<ImportRoute>,
    workspace_name: String,
    config: WorkspaceImportConfig,
) -> Result<WorkspaceImportResult> {
    let mut workspace = Workspace::new(workspace_name);
    let warnings = Vec::new();

    if config.create_folders {
        // Group routes by common prefixes to create folder structure
        let folder_groups = group_routes_by_folders(&routes, &config, |r| &r.path);

        for (folder_path, folder_routes) in folder_groups {
            let folder_id = create_folder_hierarchy(&mut workspace, &folder_path, &config)?;
            add_routes_to_folder(&mut workspace, folder_id, folder_routes)?;
        }
    } else {
        // Add all routes directly to workspace
        for route in &routes {
            let request = convert_postman_route_to_request(route);
            workspace.add_request(request)?;
        }
    }

    let folder_count = if config.create_folders {
        // Re-calculate folder groups to get count
        let folder_groups = group_routes_by_folders(&routes, &config, |r| &r.path);
        count_folders(&folder_groups)
    } else {
        0
    };

    let result = WorkspaceImportResult {
        workspace,
        request_count: routes.len(),
        folder_count,
        warnings,
    };

    Ok(result)
}

/// Import Postman routes into an existing workspace
pub fn import_postman_to_existing_workspace(
    registry: &mut WorkspaceRegistry,
    workspace_id: &str,
    routes: Vec<ImportRoute>,
    config: WorkspaceImportConfig,
) -> Result<WorkspaceImportResult> {
    let workspace = registry
        .get_workspace_mut(workspace_id)
        .ok_or_else(|| Error::generic(format!("Workspace '{}' not found", workspace_id)))?;

    let warnings = Vec::new();

    if config.create_folders {
        // Group routes by common prefixes
        let folder_groups = group_routes_by_folders(&routes, &config, |r| &r.path);

        for (folder_path, folder_routes) in folder_groups {
            let folder_id = create_folder_hierarchy(workspace, &folder_path, &config)?;
            add_routes_to_folder(workspace, folder_id, folder_routes)?;
        }
    } else {
        // Add all routes directly to workspace
        for route in &routes {
            let request = convert_postman_route_to_request(route);
            workspace.add_request(request)?;
        }
    }

    let folder_count = if config.create_folders {
        // Re-calculate folder groups to get count
        let folder_groups = group_routes_by_folders(&routes, &config, |r| &r.path);
        count_folders(&folder_groups)
    } else {
        0
    };

    let result = WorkspaceImportResult {
        workspace: workspace.clone(),
        request_count: routes.len(),
        folder_count,
        warnings,
    };

    Ok(result)
}

/// Group routes by folder structure based on path patterns
fn group_routes_by_folders<'a, T>(
    routes: &'a [T],
    config: &'a WorkspaceImportConfig,
    get_path: fn(&T) -> &str,
) -> HashMap<String, Vec<&'a T>> {
    let mut groups = HashMap::new();

    for route in routes {
        let path = get_path(route);
        let folder_path = determine_folder_path(path, config);
        groups.entry(folder_path).or_insert_with(Vec::new).push(route);
    }

    groups
}

/// Determine the folder path for a route based on its path
fn determine_folder_path(route_path: &str, config: &WorkspaceImportConfig) -> String {
    if !config.preserve_hierarchy {
        return config.base_folder_name.clone().unwrap_or_else(|| "Imported".to_string());
    }

    // Split path into segments
    let segments: Vec<&str> = route_path.trim_start_matches('/').split('/').collect();

    // Create folder path based on first few segments
    let depth = std::cmp::min(config.max_depth, segments.len().saturating_sub(1));
    let folder_segments = &segments[..depth];

    if folder_segments.is_empty() {
        config.base_folder_name.clone().unwrap_or_else(|| "Root".to_string())
    } else {
        folder_segments.join("/")
    }
}

/// Create folder hierarchy in workspace
fn create_folder_hierarchy(
    workspace: &mut Workspace,
    folder_path: &str,
    _config: &WorkspaceImportConfig,
) -> Result<String> {
    if folder_path == "Root" || folder_path.is_empty() {
        // Return root workspace ID (we'll use a placeholder)
        return Ok("root".to_string());
    }

    let segments: Vec<&str> = folder_path.split('/').collect();
    let mut current_parent: Option<String> = None;

    for segment in segments {
        let folder_name = segment.to_string();

        // Check if folder already exists
        let existing_folder = if let Some(parent_id) = &current_parent {
            workspace
                .find_folder(parent_id)
                .and_then(|parent| parent.folders.iter().find(|f| f.name == folder_name))
        } else {
            workspace.folders.iter().find(|f| f.name == folder_name)
        };

        let folder_id = if let Some(existing) = existing_folder {
            existing.id.clone()
        } else {
            // Create new folder
            if let Some(parent_id) = &current_parent {
                let parent = workspace.find_folder_mut(parent_id).ok_or_else(|| {
                    Error::generic(format!("Parent folder '{}' not found", parent_id))
                })?;
                parent.add_folder(folder_name)?
            } else {
                workspace.add_folder(folder_name)?
            }
        };

        current_parent = Some(folder_id);
    }

    current_parent.ok_or_else(|| Error::generic("Failed to create folder hierarchy".to_string()))
}

/// Add routes to a specific folder
fn add_routes_to_folder(
    workspace: &mut Workspace,
    folder_id: String,
    routes: Vec<&ImportRoute>,
) -> Result<()> {
    if folder_id == "root" {
        // Add to workspace root
        for route in &routes {
            let request = convert_postman_route_to_request(route);
            workspace.add_request(request)?;
        }
    } else {
        // Add to specific folder
        let folder = workspace
            .find_folder_mut(&folder_id)
            .ok_or_else(|| Error::generic(format!("Folder '{}' not found", folder_id)))?;

        for route in &routes {
            let request = convert_postman_route_to_request(route);
            folder.add_request(request)?;
        }
    }

    Ok(())
}

/// Convert a Postman MockForgeRoute to a MockRequest
fn convert_postman_route_to_request(route: &ImportRoute) -> MockRequest {
    // Parse HTTP method from string
    let method = match route.method.to_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET, // Default to GET
    };

    let mut request =
        MockRequest::new(method, route.path.clone(), format!("Imported: {}", route.path));

    // Set response
    let mut response = MockResponse::default();
    response.status_code = route.response.status;

    // Convert headers
    for (key, value) in &route.response.headers {
        response.headers.insert(key.clone(), value.clone());
    }

    // Convert body
    if let Some(body_value) = route.response.body.as_str() {
        response.body = Some(body_value.to_string());
    } else {
        response.body = Some("{}".to_string());
    }

    request.response = response;

    // Add tags
    request.tags.push("imported".to_string());

    request
}

/// Count total folders in grouped structure
fn count_folders<T>(groups: &HashMap<String, Vec<T>>) -> usize {
    groups.keys().filter(|k| *k != "Root" && !k.is_empty()).count()
}

/// Create workspace from Postman collection import
pub fn create_workspace_from_postman(
    import_result: PostmanImportResult,
    workspace_name: Option<String>,
) -> Result<WorkspaceImportResult> {
    let name = workspace_name.unwrap_or_else(|| "Postman Import".to_string());
    let config = WorkspaceImportConfig::default();

    let routes: Vec<ImportRoute> =
        import_result.routes.into_iter().map(postman_route_to_import_route).collect();
    import_postman_to_workspace(routes, name, config)
}

/// Create workspace from Insomnia export import
pub fn create_workspace_from_insomnia(
    import_result: crate::import::InsomniaImportResult,
    workspace_name: Option<String>,
) -> Result<WorkspaceImportResult> {
    let name = workspace_name.unwrap_or_else(|| "Insomnia Import".to_string());
    let config = WorkspaceImportConfig::default();

    let routes: Vec<ImportRoute> =
        import_result.routes.into_iter().map(insomnia_route_to_import_route).collect();
    import_postman_to_workspace(routes, name, config)
}

/// Create workspace from curl commands import
pub fn create_workspace_from_curl(
    import_result: crate::import::CurlImportResult,
    workspace_name: Option<String>,
) -> Result<WorkspaceImportResult> {
    let name = workspace_name.unwrap_or_else(|| "Curl Import".to_string());
    let config = WorkspaceImportConfig {
        create_folders: false, // Curl imports typically don't have folder structure
        ..Default::default()
    };

    let routes: Vec<ImportRoute> =
        import_result.routes.into_iter().map(curl_route_to_import_route).collect();
    import_postman_to_workspace(routes, name, config)
}

/// Create workspace from OpenAPI specification import
pub fn create_workspace_from_openapi(
    import_result: crate::import::OpenApiImportResult,
    workspace_name: Option<String>,
) -> Result<WorkspaceImportResult> {
    let name = workspace_name.unwrap_or_else(|| "OpenAPI Import".to_string());
    let config = WorkspaceImportConfig::default();

    let routes: Vec<ImportRoute> =
        import_result.routes.into_iter().map(openapi_route_to_import_route).collect();
    import_postman_to_workspace(routes, name, config)
}

/// Create workspace from HAR archive import
pub fn create_workspace_from_har(
    import_result: crate::import::HarImportResult,
    workspace_name: Option<String>,
) -> Result<WorkspaceImportResult> {
    let name = workspace_name.unwrap_or_else(|| "HAR Import".to_string());
    let config = WorkspaceImportConfig::default();

    let routes: Vec<ImportRoute> =
        import_result.routes.into_iter().map(har_route_to_import_route).collect();
    import_postman_to_workspace(routes, name, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_path_determination() {
        let config = WorkspaceImportConfig::default();

        // Test basic path
        assert_eq!(determine_folder_path("/api/users", &config), "api");

        // Test nested path
        assert_eq!(determine_folder_path("/api/v1/users/profile", &config), "api/v1");

        // Test root path
        assert_eq!(determine_folder_path("/", &config), "Root");

        // Test max depth
        let mut limited_config = config.clone();
        limited_config.max_depth = 1;
        assert_eq!(determine_folder_path("/api/v1/users/profile", &limited_config), "api");
    }

    #[test]
    fn test_workspace_creation() {
        let routes = vec![
            ImportRoute {
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                headers: HashMap::new(),
                body: None,
                response: ImportResponse {
                    status: 200,
                    headers: HashMap::new(),
                    body: serde_json::json!({"users": []}),
                },
            },
            ImportRoute {
                method: "POST".to_string(),
                path: "/api/users".to_string(),
                headers: HashMap::new(),
                body: None,
                response: ImportResponse {
                    status: 201,
                    headers: HashMap::new(),
                    body: serde_json::json!({"id": 1}),
                },
            },
        ];

        let config = WorkspaceImportConfig::default();
        let result =
            import_postman_to_workspace(routes, "Test Workspace".to_string(), config).unwrap();

        assert_eq!(result.workspace.name, "Test Workspace");
        assert_eq!(result.request_count, 2);
        assert!(result.folder_count > 0);
    }
}
