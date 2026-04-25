//! `POST .../requests/{id}/execute` + `GET .../requests/{id}/history`.
//!
//! "Execute" in the cloud sense is: expand `{{var}}` tokens in the stored request/response
//! template against (supplied variables merged over the active environment's variables),
//! return the rendered payload, and record one row in `workspace_request_history`. No
//! outbound HTTP happens — the response body is whatever the user configured.

use std::collections::HashMap;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        workspace_environment::{WorkspaceEnvVariable, WorkspaceEnvironment},
        workspace_request::{HistoryEntryResponse, WorkspaceRequest, WorkspaceRequestHistory},
        CloudWorkspace,
    },
    AppState,
};

async fn require_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
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
    Ok(workspace)
}

/// Replace every `{{name}}` token in `input` with the value in `vars` if present.
/// Unknown tokens are left in place so users see exactly which var is missing. We use a
/// simple byte-level scan (not regex) because the substitution is narrow and we'd rather
/// avoid pulling in `handlebars` for 30 lines of logic.
fn expand(input: &str, vars: &HashMap<String, String>) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Find the closing }}.
            if let Some(rel) = input[i + 2..].find("}}") {
                let name = input[i + 2..i + 2 + rel].trim();
                if let Some(value) = vars.get(name) {
                    out.push_str(value);
                    i += 2 + rel + 2;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn expand_headers(headers: &Value, vars: &HashMap<String, String>) -> Map<String, Value> {
    let mut out = Map::new();
    if let Some(obj) = headers.as_object() {
        for (k, v) in obj {
            let new_key = expand(k, vars);
            let new_val = match v {
                Value::String(s) => Value::String(expand(s, vars)),
                other => other.clone(),
            };
            out.insert(new_key, new_val);
        }
    }
    out
}

async fn gather_variables(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    overrides: &HashMap<String, Value>,
) -> ApiResult<HashMap<String, String>> {
    let mut vars: HashMap<String, String> = HashMap::new();

    // Base layer: active environment's variables, if any.
    let envs = WorkspaceEnvironment::list_by_workspace(pool, workspace_id).await?;
    if let Some(active) = envs.iter().find(|e| e.is_active) {
        let active_vars = WorkspaceEnvVariable::list_by_environment(pool, active.id).await?;
        for v in active_vars {
            vars.insert(v.name, v.value);
        }
    }

    // Overlay: caller-supplied variables win (strings stringified, numbers formatted).
    for (k, v) in overrides {
        let s = match v {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        vars.insert(k.clone(), s);
    }
    Ok(vars)
}

#[derive(Debug, Deserialize, Default)]
pub struct ExecuteRequestBody {
    #[serde(default)]
    pub variables: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteRequestResponse {
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub request_method: String,
    pub request_path: String,
    pub request_headers: Map<String, Value>,
    pub request_body: Option<String>,
    pub response_status_code: i32,
    pub response_headers: Map<String, Value>,
    pub response_body: Option<String>,
    pub response_time_ms: i32,
    pub response_size_bytes: i32,
    pub error_message: Option<String>,
}

/// POST /api/v1/workspaces/{workspace_id}/requests/{request_id}/execute
pub async fn execute_request(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, request_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ExecuteRequestBody>,
) -> ApiResult<Json<ExecuteRequestResponse>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let req = WorkspaceRequest::find_by_id(state.db.pool(), request_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Request not found".to_string()))?;
    if req.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Request does not belong to this workspace".to_string(),
        ));
    }

    let vars = gather_variables(state.db.pool(), workspace_id, &body.variables).await?;

    let start = Instant::now();
    let rendered_path = expand(&req.path, &vars);
    let rendered_body = expand(&req.response_body, &vars);
    let rendered_req_headers = expand_headers(&req.request_headers, &vars);
    let rendered_resp_headers = expand_headers(&req.response_headers, &vars);
    let elapsed_ms = start.elapsed().as_millis() as i32;

    let response_size = rendered_body.len() as i32;

    // Persist the execution so the History modal can list it later.
    let _ = WorkspaceRequestHistory::insert(
        state.db.pool(),
        request_id,
        workspace_id,
        Some(user_id),
        &req.method,
        &rendered_path,
        &Value::Object(rendered_req_headers.clone()),
        None,
        req.status_code,
        &Value::Object(rendered_resp_headers.clone()),
        Some(&rendered_body),
        elapsed_ms,
        response_size,
        None,
    )
    .await?;

    Ok(Json(ExecuteRequestResponse {
        executed_at: chrono::Utc::now(),
        request_method: req.method.clone(),
        request_path: rendered_path,
        request_headers: rendered_req_headers,
        request_body: None,
        response_status_code: req.status_code,
        response_headers: rendered_resp_headers,
        response_body: Some(rendered_body),
        response_time_ms: elapsed_ms,
        response_size_bytes: response_size,
        error_message: None,
    }))
}

#[derive(Debug, Serialize)]
pub struct HistoryListResponse {
    pub history: Vec<HistoryEntryResponse>,
    pub total: i64,
}

/// GET /api/v1/workspaces/{workspace_id}/requests/{request_id}/history
pub async fn list_request_history(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, request_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<HistoryListResponse>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    let req = WorkspaceRequest::find_by_id(state.db.pool(), request_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Request not found".to_string()))?;
    if req.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Request does not belong to this workspace".to_string(),
        ));
    }

    let entries = WorkspaceRequestHistory::list_for_request(state.db.pool(), request_id, 100)
        .await?
        .into_iter()
        .map(|h| h.to_response())
        .collect::<Vec<_>>();
    let total = WorkspaceRequestHistory::count_for_request(state.db.pool(), request_id).await?;

    Ok(Json(HistoryListResponse {
        history: entries,
        total,
    }))
}

// ---------- Tiny unit tests for the template expansion ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_substitutes_known_tokens() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Ray".to_string());
        vars.insert("host".to_string(), "example.test".to_string());
        assert_eq!(expand("hello {{name}} at {{host}}", &vars), "hello Ray at example.test");
    }

    #[test]
    fn expand_leaves_unknown_tokens_in_place() {
        let vars = HashMap::new();
        assert_eq!(expand("{{missing}}", &vars), "{{missing}}");
    }

    #[test]
    fn expand_handles_adjacent_and_nested() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "1".to_string());
        vars.insert("b".to_string(), "2".to_string());
        assert_eq!(expand("{{a}}{{b}}", &vars), "12");
        // Partial / stray braces are preserved verbatim.
        assert_eq!(expand("{{ a }}{not-a-var}", &vars), "1{not-a-var}");
    }

    #[test]
    fn expand_ignores_unterminated_template() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "1".to_string());
        assert_eq!(expand("{{a", &vars), "{{a");
    }
}
