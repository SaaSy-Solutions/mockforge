//! Workspace import preview/execute + autocomplete for environment variables.
//!
//! The mockforge-core importers are marked deprecated (they're slated to move into their
//! own crate). Silence those warnings at the module level — we're intentionally using them.
#![allow(deprecated)]

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        workspace_environment::{WorkspaceEnvVariable, WorkspaceEnvironment},
        workspace_folder::WorkspaceFolder,
        workspace_request::WorkspaceRequest,
        CloudWorkspace,
    },
    AppState,
};

async fn require_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;
    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }
    Ok(())
}

// ---------- Parsed-route IR ----------
//
// All four importers produce subtly different shapes. We normalize to a single flat
// representation so preview + execute share code.

#[derive(Debug, Clone, Serialize)]
pub struct ParsedRoute {
    pub method: String,
    pub path: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub status_code: u16,
    pub response: ParsedResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub routes: Vec<ParsedRoute>,
    pub variables: HashMap<String, String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequestBody {
    pub format: String,
    pub data: String,
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    #[serde(default)]
    pub create_folders: bool,
    #[serde(default)]
    pub selected_routes: Option<Vec<usize>>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
}

fn body_to_string(body: &serde_json::Value) -> String {
    match body {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn parse(
    format: &str,
    data: &str,
    base_url: Option<&str>,
    environment: Option<&str>,
) -> ApiResult<PreviewResponse> {
    use mockforge_core::import::{
        import_curl_commands, import_insomnia_export, import_openapi_spec,
        import_postman_collection,
    };

    match format.to_lowercase().as_str() {
        "postman" => {
            #[allow(deprecated)]
            let result = import_postman_collection(data, base_url)
                .map_err(|e| ApiError::InvalidRequest(format!("Postman import failed: {e}")))?;
            let routes = result
                .routes
                .into_iter()
                .map(|r| ParsedRoute {
                    method: r.method,
                    path: r.path,
                    name: None,
                    description: None,
                    headers: r.headers,
                    body: r.body,
                    status_code: r.response.status,
                    response: ParsedResponse {
                        status: r.response.status,
                        headers: r.response.headers,
                        body: body_to_string(&r.response.body),
                    },
                })
                .collect();
            Ok(PreviewResponse {
                success: true,
                routes,
                variables: result.variables,
                warnings: result.warnings,
            })
        }
        "insomnia" => {
            #[allow(deprecated)]
            let result = import_insomnia_export(data, environment)
                .map_err(|e| ApiError::InvalidRequest(format!("Insomnia import failed: {e}")))?;
            let routes = result
                .routes
                .into_iter()
                .map(|r| ParsedRoute {
                    method: r.method,
                    path: r.path,
                    name: None,
                    description: None,
                    headers: r.headers,
                    body: r.body,
                    status_code: r.response.status,
                    response: ParsedResponse {
                        status: r.response.status,
                        headers: r.response.headers,
                        body: body_to_string(&r.response.body),
                    },
                })
                .collect();
            Ok(PreviewResponse {
                success: true,
                routes,
                variables: result.variables,
                warnings: result.warnings,
            })
        }
        "curl" => {
            #[allow(deprecated)]
            let result = import_curl_commands(data, base_url)
                .map_err(|e| ApiError::InvalidRequest(format!("cURL import failed: {e}")))?;
            let routes = result
                .routes
                .into_iter()
                .map(|r| ParsedRoute {
                    method: r.method,
                    path: r.path,
                    name: None,
                    description: None,
                    headers: r.headers,
                    body: r.body,
                    status_code: r.response.status,
                    response: ParsedResponse {
                        status: r.response.status,
                        headers: r.response.headers,
                        body: body_to_string(&r.response.body),
                    },
                })
                .collect();
            Ok(PreviewResponse {
                success: true,
                routes,
                variables: HashMap::new(),
                warnings: result.warnings,
            })
        }
        "openapi" | "swagger" => {
            let result = import_openapi_spec(data, base_url)
                .map_err(|e| ApiError::InvalidRequest(format!("OpenAPI import failed: {e}")))?;
            let routes = result
                .routes
                .into_iter()
                .map(|r| ParsedRoute {
                    method: r.method,
                    path: r.path,
                    name: None,
                    description: None,
                    headers: r.headers,
                    body: r.body,
                    status_code: r.response.status,
                    response: ParsedResponse {
                        status: r.response.status,
                        headers: r.response.headers,
                        body: body_to_string(&r.response.body),
                    },
                })
                .collect();
            Ok(PreviewResponse {
                success: true,
                routes,
                variables: HashMap::new(),
                warnings: result.warnings,
            })
        }
        other => Err(ApiError::InvalidRequest(format!(
            "Unsupported import format '{other}'. Expected one of: postman, insomnia, curl, openapi"
        ))),
    }
}

/// POST /api/v1/import/preview
pub async fn preview_import(
    State(_state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    _headers: HeaderMap,
    Json(request): Json<ImportRequestBody>,
) -> ApiResult<Json<PreviewResponse>> {
    let preview = parse(
        &request.format,
        &request.data,
        request.base_url.as_deref(),
        request.environment.as_deref(),
    )?;
    Ok(Json(preview))
}

/// POST /api/v1/workspaces/{workspace_id}/import
pub async fn import_to_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ImportRequestBody>,
) -> ApiResult<Json<serde_json::Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    // Parent folder check (if caller specified one)
    if let Some(folder_id) = request.folder_id {
        let folder = WorkspaceFolder::find_by_id(state.db.pool(), folder_id)
            .await?
            .ok_or_else(|| ApiError::InvalidRequest("Folder not found".to_string()))?;
        if folder.workspace_id != workspace_id {
            return Err(ApiError::InvalidRequest(
                "Folder does not belong to this workspace".to_string(),
            ));
        }
    }

    let create_folders = request.create_folders;
    let selected = request.selected_routes.clone();
    let parsed = parse(
        &request.format,
        &request.data,
        request.base_url.as_deref(),
        request.environment.as_deref(),
    )?;

    // Optionally bucket routes into one folder per HTTP method.
    let method_folder_cache: HashMap<String, Uuid> = if create_folders {
        let mut cache = HashMap::new();
        let mut unique_methods: Vec<String> =
            parsed.routes.iter().map(|r| r.method.to_uppercase()).collect();
        unique_methods.sort();
        unique_methods.dedup();
        for method in unique_methods {
            let folder = WorkspaceFolder::create(
                state.db.pool(),
                workspace_id,
                request.folder_id,
                &method,
                &format!("Imported {method} routes"),
            )
            .await?;
            cache.insert(method, folder.id);
        }
        cache
    } else {
        HashMap::new()
    };

    let mut imported = 0usize;
    for (idx, route) in parsed.routes.iter().enumerate() {
        if let Some(ref sel) = selected {
            if !sel.contains(&idx) {
                continue;
            }
        }

        let method_upper = route.method.to_uppercase();
        let target_folder = if create_folders {
            method_folder_cache.get(&method_upper).copied()
        } else {
            request.folder_id
        };

        let name = route.name.clone().unwrap_or_else(|| format!("{} {}", method_upper, route.path));
        let req_headers = serde_json::to_value(&route.headers).unwrap_or(json!({}));
        let resp_headers = serde_json::to_value(&route.response.headers).unwrap_or(json!({}));

        WorkspaceRequest::create(
            state.db.pool(),
            workspace_id,
            target_folder,
            &name,
            route.description.as_deref().unwrap_or(""),
            &method_upper,
            &route.path,
            route.response.status as i32,
            &route.response.body,
            &req_headers,
            &resp_headers,
        )
        .await?;
        imported += 1;
    }

    Ok(Json(json!({
        "success": true,
        "imported": imported,
        "warnings": parsed.warnings,
    })))
}

// ---------- Autocomplete ----------

#[derive(Debug, Deserialize)]
pub struct AutocompleteRequest {
    pub input: String,
    pub cursor_position: usize,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteSuggestion {
    pub text: String,
    pub display_text: String,
    pub kind: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteResponse {
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub start_position: usize,
    pub end_position: usize,
}

/// Find the `{{` before cursor and the `}}` after it (if any).
fn detect_template_span(input: &str, cursor: usize) -> Option<(usize, usize, String)> {
    let chars: Vec<char> = input.chars().collect();
    let cursor = cursor.min(chars.len());

    // Walk left looking for `{{` that isn't already closed before the cursor.
    let mut i = cursor;
    while i >= 2 {
        if chars[i - 1] == '{' && chars[i - 2] == '{' {
            let start = i - 2;
            // Find matching close after cursor, if any.
            let mut end = cursor;
            let mut j = cursor;
            while j + 1 < chars.len() {
                if chars[j] == '}' && chars[j + 1] == '}' {
                    end = j + 2;
                    break;
                }
                j += 1;
            }
            let fragment: String = chars[start + 2..cursor].iter().collect();
            return Some((start, end.max(cursor), fragment));
        }
        if chars[i - 1] == '}' && i >= 2 && chars[i - 2] == '}' {
            // Just passed the end of a template.
            return None;
        }
        i -= 1;
    }
    None
}

/// POST /api/v1/workspaces/{workspace_id}/autocomplete
pub async fn autocomplete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<AutocompleteRequest>,
) -> ApiResult<Json<AutocompleteResponse>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let span = detect_template_span(&request.input, request.cursor_position);
    let (start, end, prefix) = match span {
        Some(s) => s,
        None => {
            return Ok(Json(AutocompleteResponse {
                suggestions: Vec::new(),
                start_position: request.cursor_position,
                end_position: request.cursor_position,
            }));
        }
    };

    // Collect variable names from the active environment (fallback: every env in the workspace).
    let envs = WorkspaceEnvironment::list_by_workspace(state.db.pool(), workspace_id).await?;
    let active = envs.iter().find(|e| e.is_active);

    let mut seen = std::collections::HashSet::new();
    let mut suggestions: Vec<AutocompleteSuggestion> = Vec::new();

    let source_envs: Vec<&WorkspaceEnvironment> = match active {
        Some(e) => vec![e],
        None => envs.iter().collect(),
    };

    for env in source_envs {
        let vars = WorkspaceEnvVariable::list_by_environment(state.db.pool(), env.id).await?;
        for var in vars {
            if !var.name.starts_with(&prefix) {
                continue;
            }
            if !seen.insert(var.name.clone()) {
                continue;
            }
            suggestions.push(AutocompleteSuggestion {
                text: var.name.clone(),
                display_text: var.name.clone(),
                kind: "variable".to_string(),
                documentation: Some(format!("From environment '{}'", env.name)),
            });
        }
    }

    Ok(Json(AutocompleteResponse {
        suggestions,
        start_position: start + 2,
        end_position: end.saturating_sub(2).max(start + 2),
    }))
}
