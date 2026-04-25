//! Workspace application-layer encryption: status, config, enable/disable, security-check.
//!
//! The self-hosted surface also exposes `export`/`import` to local filesystem paths; those
//! don't translate to multi-tenant cloud (filesystem concept) and are intentionally omitted.
//! Key material itself lives in the BYOK infrastructure in `handlers::settings` — this module
//! just controls the *policy* (should we encrypt? which patterns count as sensitive?).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{workspace_environment::WorkspaceEnvVariable, CloudWorkspace},
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

/// Matches the `EncryptionStatus` TS interface.
#[derive(Debug, Serialize)]
pub struct EncryptionStatusResponse {
    pub enabled: bool,
    pub algorithm: String,
    pub key_id: Option<String>,
    pub last_rotated: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "masterKeySet")]
    pub master_key_set: bool,
    #[serde(rename = "workspaceKeySet")]
    pub workspace_key_set: bool,
}

/// GET /api/v1/workspaces/{workspace_id}/encryption/status
pub async fn get_status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<EncryptionStatusResponse>> {
    let ws = require_workspace(&state, user_id, &headers, workspace_id).await?;
    let master_key_set =
        std::env::var("BYOK_ENCRYPTION_KEY").map(|v| !v.is_empty()).unwrap_or(false);
    Ok(Json(EncryptionStatusResponse {
        enabled: ws.encryption_enabled,
        algorithm: ws.encryption_algorithm.clone(),
        key_id: ws.encryption_key_rotated_at.map(|ts| format!("k-{}", ts.timestamp())),
        last_rotated: ws.encryption_key_rotated_at,
        master_key_set,
        workspace_key_set: ws.encryption_enabled && master_key_set,
    }))
}

/// GET /api/v1/workspaces/{workspace_id}/encryption/config
pub async fn get_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let ws = require_workspace(&state, user_id, &headers, workspace_id).await?;
    // Always echo `enabled` alongside the stored blob so the UI can read both in one call.
    let mut cfg = ws.encryption_config.clone();
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert("enabled".to_string(), json!(ws.encryption_enabled));
        obj.entry("algorithm".to_string())
            .or_insert_with(|| json!(ws.encryption_algorithm));
    }
    Ok(Json(cfg))
}

/// PUT /api/v1/workspaces/{workspace_id}/encryption/config
pub async fn put_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(mut config): Json<Value>,
) -> ApiResult<Json<Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;

    // Callers sometimes send `enabled`/`algorithm` inside the config blob; keep the flag
    // column authoritative and strip those from the JSONB so we don't store them twice.
    let obj = config
        .as_object_mut()
        .ok_or_else(|| ApiError::InvalidRequest("Config must be a JSON object".to_string()))?;
    obj.remove("enabled");
    obj.remove("algorithm");

    let updated = CloudWorkspace::set_encryption_config(state.db.pool(), workspace_id, &config)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

    Ok(Json(json!({
        "message": "Encryption config updated",
        "config": updated.encryption_config,
    })))
}

/// POST /api/v1/workspaces/{workspace_id}/encryption/enable
pub async fn enable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    CloudWorkspace::set_encryption_enabled(state.db.pool(), workspace_id, true)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;
    Ok(Json(json!({ "message": "Encryption enabled" })))
}

/// POST /api/v1/workspaces/{workspace_id}/encryption/disable
pub async fn disable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    require_workspace(&state, user_id, &headers, workspace_id).await?;
    CloudWorkspace::set_encryption_enabled(state.db.pool(), workspace_id, false)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;
    Ok(Json(json!({ "message": "Encryption disabled" })))
}

// ---------- Security check ----------

/// Low-noise pattern list. Matched case-insensitively against variable names.
const DEFAULT_SENSITIVE_NAME_PATTERNS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "bearer",
    "auth",
    "private",
    "credential",
    "oauth",
    "jwt",
    "ssh_key",
    "access_key",
];

/// Low-noise pattern list. Matched case-insensitively against variable *values* to detect
/// likely-secrets stored in plaintext without `is_secret = true`.
const SUSPICIOUS_VALUE_PREFIXES: &[&str] = &[
    "sk_", "sk-", "AKIA", "ghp_", "glpat-", "xoxb-", "xoxp-", "Bearer ", "eyJ",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityCheck {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SecurityCheckResult {
    pub passed: bool,
    pub checks: Vec<SecurityCheck>,
    #[serde(rename = "isSecure")]
    pub is_secure: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub recommendations: Vec<String>,
}

/// POST /api/v1/workspaces/{workspace_id}/encryption/security-check
///
/// Scans every environment variable in the workspace for patterns that look sensitive
/// but are stored with `is_secret = false`. Reports a per-check result + human-readable
/// warnings/recommendations the UI can surface.
pub async fn security_check(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<SecurityCheckResult>> {
    use crate::models::workspace_environment::WorkspaceEnvironment;

    let ws = require_workspace(&state, user_id, &headers, workspace_id).await?;
    let pool = state.db.pool();

    // Every variable in every environment in this workspace.
    let envs = WorkspaceEnvironment::list_by_workspace(pool, workspace_id).await?;
    let mut all_vars: Vec<(String, WorkspaceEnvVariable)> = Vec::new();
    for env in &envs {
        let vars = WorkspaceEnvVariable::list_by_environment(pool, env.id).await?;
        for v in vars {
            all_vars.push((env.name.clone(), v));
        }
    }

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut recommendations = Vec::new();

    // Check 1: encryption enabled on the workspace.
    let enc_enabled = ws.encryption_enabled;
    if !enc_enabled {
        recommendations.push(
            "Enable workspace encryption so sensitive variables are marked clearly.".to_string(),
        );
    }

    // Check 2: master key configured on the server.
    let master_key_set =
        std::env::var("BYOK_ENCRYPTION_KEY").map(|v| !v.is_empty()).unwrap_or(false);
    if enc_enabled && !master_key_set {
        errors.push(
            "Workspace encryption is enabled but the server has no BYOK master key configured."
                .to_string(),
        );
    }

    // Check 3: suspicious names stored without is_secret.
    let mut suspicious_name_count = 0;
    for (env_name, v) in &all_vars {
        let lower = v.name.to_lowercase();
        let matched_name = DEFAULT_SENSITIVE_NAME_PATTERNS.iter().any(|p| lower.contains(p));
        if matched_name && !v.is_secret {
            suspicious_name_count += 1;
            warnings.push(format!(
                "{}.{} looks sensitive but is not marked as secret",
                env_name, v.name
            ));
        }
    }

    // Check 4: plaintext values that look like real secrets.
    let mut suspicious_value_count = 0;
    for (env_name, v) in &all_vars {
        if v.is_secret {
            continue;
        }
        if SUSPICIOUS_VALUE_PREFIXES.iter().any(|pfx| v.value.starts_with(pfx)) {
            suspicious_value_count += 1;
            warnings.push(format!(
                "{}.{} value matches a known secret pattern but is stored in plaintext",
                env_name, v.name
            ));
        }
    }

    if suspicious_name_count > 0 || suspicious_value_count > 0 {
        recommendations.push(
            "Mark matching variables `encrypted: true` to hide them in the UI and audit logs."
                .to_string(),
        );
    }

    let checks = vec![
        SecurityCheck {
            name: "workspace_encryption_enabled".to_string(),
            passed: enc_enabled,
            message: Some(if enc_enabled {
                "Workspace encryption is on".into()
            } else {
                "Workspace encryption is off".into()
            }),
        },
        SecurityCheck {
            name: "byok_master_key_configured".to_string(),
            passed: master_key_set,
            message: Some(if master_key_set {
                "Server BYOK master key is configured".into()
            } else {
                "Server BYOK master key is NOT configured".into()
            }),
        },
        SecurityCheck {
            name: "no_sensitive_named_plaintext_vars".to_string(),
            passed: suspicious_name_count == 0,
            message: Some(format!(
                "{suspicious_name_count} sensitively-named variable(s) are stored in plaintext"
            )),
        },
        SecurityCheck {
            name: "no_suspicious_valued_plaintext_vars".to_string(),
            passed: suspicious_value_count == 0,
            message: Some(format!(
                "{suspicious_value_count} variable value(s) match known secret patterns but are plaintext"
            )),
        },
    ];

    let passed = errors.is_empty() && warnings.is_empty();
    Ok(Json(SecurityCheckResult {
        passed,
        checks,
        is_secure: passed,
        warnings,
        errors,
        recommendations,
    }))
}
