//! Cloud Plugins control-plane API (Phase 1, Task #6).
//!
//! Attach / detach / list plugins on a hosted-mock deployment. The
//! data layer (`hosted_mock_plugins` table, `HostedMockPlugin` model)
//! shipped in #389; this is the HTTP surface that admins use to
//! manage attachments.
//!
//! Routes (mirrors the existing hosted-mocks routing — org_id resolves
//! from the `X-Organization-Id` header rather than the path):
//!   GET    /api/v1/hosted-mocks/{deployment_id}/plugins
//!   POST   /api/v1/hosted-mocks/{deployment_id}/plugins
//!   PATCH  /api/v1/hosted-mocks/{deployment_id}/plugins/{attachment_id}
//!   DELETE /api/v1/hosted-mocks/{deployment_id}/plugins/{attachment_id}
//!
//! Authorization: caller must be a member of the deployment's org and
//! have `Permission::HostedMockUpdate` (managing plugins is a
//! hosted-mock configuration change).
//!
//! Trust model:
//!   - Signature verification at attach is deferred to the runtime
//!     boot path (see `cloud-trust-permissions-rfc.md` §7.2 step 3).
//!     Plugins were verified at publish; the runtime re-verifies at
//!     boot. The middle (attach-time) check is the redundant layer
//!     and can be added when the org-trust-root lookup infrastructure
//!     is fully wired.
//!   - `permissions_json` is validated for *shape* here. Validating
//!     `manifest ∩ grant` (RFC §4.1) requires reading the plugin's
//!     declared capabilities, which lives in the WASM-bundle metadata
//!     fetched at boot — for v1 this validation also defers to the
//!     plugin-host. The server does enforce: deny-all defaults if the
//!     grant is missing keys, and reject unknown top-level keys.
//!
//! Plan limits enforced here: `max_plugins_per_mock` from
//! `organizations.limits_json` (-1 = unlimited, 0 = feature disabled).

use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mockforge_registry_core::models::{
    feature_usage::{FeatureType, FeatureUsage, PluginInvokeAggregateRow},
    hosted_mock_plugin::AttachHostedMockPlugin,
    AuditEventType, HostedMockPlugin,
};

use crate::{
    error::{ApiError, ApiResult},
    middleware::{
        permission_check::PermissionChecker, permissions::Permission, resolve_org_context, AuthUser,
    },
    AppState,
};

/// Top-level keys allowed in a `permissions_json` grant. Anything else
/// is rejected at attach. Mirrors `cloud-trust-permissions-rfc.md`
/// §4.2.
const PERMISSION_SECTIONS: &[&str] = &["egress", "env", "request", "response", "storage"];

/// Hard cap on grant payload size to keep a misbehaving client from
/// stuffing the JSONB column. 32 KiB is generous for any realistic
/// permission grant.
const MAX_PERMISSIONS_BYTES: usize = 32 * 1024;

// ─── Request / response shapes ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AttachRequest {
    /// Plugin name (matches `plugins.name`). The handler resolves this
    /// to the canonical `plugin_id` so callers don't have to.
    pub plugin_name: String,
    /// Version string (matches `plugin_versions.version`). Pinned at
    /// attach time; bumping requires a re-attach call.
    pub version: String,
    /// Plugin-specific runtime config (publisher's `ConfigSchema` as
    /// JSON). Distinct from `permissions`.
    #[serde(default = "empty_object")]
    pub config: serde_json::Value,
    /// Permission grant. Default is the deny-all empty object.
    #[serde(default = "empty_object")]
    pub permissions: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateAttachmentRequest {
    /// Update the permission grant. Replaces the existing grant in
    /// full — partial updates aren't supported because they make the
    /// "what's actually granted" question harder to reason about.
    #[serde(default)]
    pub permissions: Option<serde_json::Value>,
    /// Toggle the attachment. Disabled rows stay in the table so the
    /// audit trail is preserved.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Update the plugin-specific config.
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AttachmentResponse {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub plugin_id: Uuid,
    pub plugin_version_id: Uuid,
    pub config: serde_json::Value,
    pub permissions: serde_json::Value,
    pub enabled: bool,
    pub attached_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<HostedMockPlugin> for AttachmentResponse {
    fn from(row: HostedMockPlugin) -> Self {
        Self {
            id: row.id,
            deployment_id: row.deployment_id,
            plugin_id: row.plugin_id,
            plugin_version_id: row.plugin_version_id,
            config: row.config_json,
            permissions: row.permissions_json,
            enabled: row.enabled,
            attached_at: row.attached_at,
            updated_at: row.updated_at,
        }
    }
}

// ─── Routes ──────────────────────────────────────────────────────────

/// `GET /api/v1/hosted-mocks/{deployment_id}/plugins`
pub async fn list_attachments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<AttachmentResponse>>> {
    authorize_deployment(&state, user_id, &headers, deployment_id, Permission::HostedMockUpdate)
        .await?;

    let rows = HostedMockPlugin::list_by_deployment(state.db.pool(), deployment_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(rows.into_iter().map(AttachmentResponse::from).collect()))
}

/// `POST /api/v1/hosted-mocks/{deployment_id}/plugins`
pub async fn attach_plugin(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AttachRequest>,
) -> ApiResult<Json<AttachmentResponse>> {
    let org_ctx = authorize_deployment(
        &state,
        user_id,
        &headers,
        deployment_id,
        Permission::HostedMockUpdate,
    )
    .await?;

    // Validate the grant payload before any storage work — fail fast
    // on shape errors.
    validate_permissions(&request.permissions)?;

    // Resolve plugin + version. Missing either is a 400, not a 404,
    // because both come from a request body the client is composing.
    let plugin = state
        .store
        .find_plugin_by_name(&request.plugin_name)
        .await
        .map_err(|e| ApiError::Database(sqlx::Error::Protocol(e.to_string())))?
        .ok_or_else(|| {
            ApiError::InvalidRequest(format!("Plugin '{}' not found", request.plugin_name))
        })?;

    let plugin_version = state
        .store
        .find_plugin_version(plugin.id, &request.version)
        .await
        .map_err(|e| ApiError::Database(sqlx::Error::Protocol(e.to_string())))?
        .ok_or_else(|| {
            ApiError::InvalidRequest(format!(
                "Plugin '{}' has no version '{}'",
                request.plugin_name, request.version
            ))
        })?;

    if plugin_version.yanked {
        return Err(ApiError::InvalidRequest(format!(
            "Plugin '{}' version '{}' is yanked and cannot be attached",
            request.plugin_name, request.version
        )));
    }

    // Plan-limit enforcement. Skip on UPSERT (re-attach of the same
    // plugin doesn't increase the count) so this is a count-against-
    // distinct-plugins check.
    enforce_plan_limit(&state, &org_ctx, deployment_id, plugin.id).await?;

    let row = HostedMockPlugin::attach(
        state.db.pool(),
        AttachHostedMockPlugin {
            deployment_id,
            plugin_id: plugin.id,
            plugin_version_id: plugin_version.id,
            config_json: &request.config,
            permissions_json: &request.permissions,
            enabled: request.enabled,
            attached_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    // Telemetry + audit. Order: feature_usage first (cheap), audit
    // log second. Both are best-effort — failures here don't undo the
    // attach (matches the pattern in hosted_mocks::create_deployment).
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::PluginAttach,
            Some(serde_json::json!({
                "deployment_id": deployment_id,
                "plugin_id": plugin.id,
                "plugin_name": plugin.name,
                "version": request.version,
            })),
        )
        .await;

    let (ip_address, user_agent) = client_metadata(&headers);
    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::PluginAttached,
            format!(
                "Plugin '{}@{}' attached to deployment {}",
                plugin.name, request.version, deployment_id
            ),
            Some(serde_json::json!({
                "deployment_id": deployment_id,
                "plugin_id": plugin.id,
                "plugin_name": plugin.name,
                "version": request.version,
                "permissions": request.permissions,
            })),
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    Ok(Json(AttachmentResponse::from(row)))
}

/// `PATCH /api/v1/hosted-mocks/{deployment_id}/plugins/{attachment_id}`
pub async fn update_attachment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((deployment_id, attachment_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdateAttachmentRequest>,
) -> ApiResult<Json<AttachmentResponse>> {
    authorize_deployment(&state, user_id, &headers, deployment_id, Permission::HostedMockUpdate)
        .await?;

    // Load the row and verify it belongs to this deployment. Cross-
    // deployment writes via path manipulation get a "not found" so we
    // don't leak whether the attachment exists in another deployment.
    let existing = load_authorized_attachment(&state, deployment_id, attachment_id).await?;

    // Validate updated permissions if provided.
    if let Some(ref new_perms) = request.permissions {
        validate_permissions(new_perms)?;
    }

    // Build the updated row. We do this with an UPSERT against the
    // existing fields rather than a partial UPDATE because the
    // `attach` method is the only mutating path on the model — keeps
    // the model surface small.
    let row = HostedMockPlugin::attach(
        state.db.pool(),
        AttachHostedMockPlugin {
            deployment_id,
            plugin_id: existing.plugin_id,
            plugin_version_id: existing.plugin_version_id,
            config_json: request.config.as_ref().unwrap_or(&existing.config_json),
            permissions_json: request.permissions.as_ref().unwrap_or(&existing.permissions_json),
            enabled: request.enabled.unwrap_or(existing.enabled),
            attached_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(AttachmentResponse::from(row)))
}

/// `DELETE /api/v1/hosted-mocks/{deployment_id}/plugins/{attachment_id}`
pub async fn detach_plugin(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((deployment_id, attachment_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = authorize_deployment(
        &state,
        user_id,
        &headers,
        deployment_id,
        Permission::HostedMockUpdate,
    )
    .await?;

    let existing = load_authorized_attachment(&state, deployment_id, attachment_id).await?;

    let deleted = HostedMockPlugin::delete(state.db.pool(), attachment_id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        // Lost a race — already detached. Idempotent: surface 200, not
        // 404, since the desired end state holds.
        return Ok(Json(serde_json::json!({ "deleted": false })));
    }

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::PluginDetach,
            Some(serde_json::json!({
                "deployment_id": deployment_id,
                "plugin_id": existing.plugin_id,
            })),
        )
        .await;

    let (ip_address, user_agent) = client_metadata(&headers);
    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::PluginDetached,
            format!(
                "Plugin attachment {} detached from deployment {}",
                attachment_id, deployment_id
            ),
            Some(serde_json::json!({
                "deployment_id": deployment_id,
                "plugin_id": existing.plugin_id,
                "attachment_id": attachment_id,
            })),
            ip_address.as_deref(),
            user_agent.as_deref(),
        )
        .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ─── Metering (Issue #417) ───────────────────────────────────────────

/// Per-plugin row in the deployment usage response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUsageEntry {
    /// `hosted_mock_plugins.id`. Stable per (deployment, plugin) — a
    /// re-attach after detach gets a new id, so historical data from a
    /// previous attachment surfaces with its old id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<Uuid>,
    /// `plugins.name` as snapshot in the metric's metadata at write time.
    /// Optional because the OTLP aggregator may not have populated it
    /// yet; renders as "—" in the UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
    /// SUM of wall-time across all buckets for this attachment in the
    /// current billing period.
    pub invoke_ms: i64,
    /// MAX peak memory across buckets, in MB. Optional in v1 — the OTLP
    /// aggregator's MemoryTracker integration (PR #396) may not populate
    /// this yet. Surfaces as "—" in the UI when missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_peak_mb: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentPluginUsageResponse {
    /// First instant of the current billing period (start of UTC month).
    pub period_start: DateTime<Utc>,
    /// Exclusive — first instant of the next billing period.
    pub period_end: DateTime<Utc>,
    /// Per-attachment breakdown, ordered by `invoke_ms` desc.
    pub by_plugin: Vec<PluginUsageEntry>,
    /// Sum of `invoke_ms` across all `by_plugin` entries.
    pub deployment_total_invoke_ms: i64,
    /// `organizations.limits_json -> max_plugin_invoke_ms_per_month`.
    /// `-1` = unlimited; `0` = feature disabled. UI maps both to
    /// distinct affordances ("∞" / "upgrade to enable").
    pub plan_limit_invoke_ms_per_month: i64,
    /// `organizations.limits_json -> max_plugin_memory_mb`. Same `-1`/
    /// `0` semantics. Per-attachment cap rather than per-deployment.
    pub plan_limit_memory_mb: i64,
}

/// `GET /api/v1/hosted-mocks/{deployment_id}/plugins/usage`
///
/// Rolled-up per-plugin metering for the deployment in the current
/// billing period. Source: `feature_usage` rows where
/// `feature = 'plugin_invoke_ms'` and
/// `metadata->>'deployment_id' = {deployment_id}`. The OTLP pipeline
/// (Phase 2 — see migration `20250101000074`) writes those rows; this
/// endpoint just aggregates them.
///
/// Returns `by_plugin = []` and `deployment_total_invoke_ms = 0` when
/// the pipeline hasn't populated any rows yet — UI renders that as
/// "no usage this period" rather than erroring.
pub async fn get_plugin_usage(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<DeploymentPluginUsageResponse>> {
    let org_ctx =
        authorize_deployment(&state, user_id, &headers, deployment_id, Permission::HostedMockRead)
            .await?;

    let (period_start, period_end) = current_billing_period();

    let rows = FeatureUsage::aggregate_plugin_invoke_ms_by_deployment(
        state.db.pool(),
        org_ctx.org_id,
        deployment_id,
        period_start,
    )
    .await
    .map_err(ApiError::Database)?;

    let by_plugin: Vec<PluginUsageEntry> = rows.into_iter().map(into_usage_entry).collect();
    let deployment_total_invoke_ms: i64 = by_plugin.iter().map(|p| p.invoke_ms).sum();

    let limits = &org_ctx.org.limits_json;
    let plan_limit_invoke_ms_per_month = limits
        .get("max_plugin_invoke_ms_per_month")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let plan_limit_memory_mb =
        limits.get("max_plugin_memory_mb").and_then(|v| v.as_i64()).unwrap_or(0);

    Ok(Json(DeploymentPluginUsageResponse {
        period_start,
        period_end,
        by_plugin,
        deployment_total_invoke_ms,
        plan_limit_invoke_ms_per_month,
        plan_limit_memory_mb,
    }))
}

/// Convert a SQL aggregate row into the API entry. Lossy on a
/// malformed `attachment_id` — drops the field rather than rejecting
/// the whole row, since one bad bucket shouldn't poison the response.
fn into_usage_entry(row: PluginInvokeAggregateRow) -> PluginUsageEntry {
    PluginUsageEntry {
        attachment_id: row.attachment_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        plugin_name: row.plugin_name,
        plugin_version: row.plugin_version,
        invoke_ms: row.invoke_ms,
        memory_peak_mb: row.memory_peak_mb,
    }
}

/// First instant of the current UTC month + first instant of the next
/// month (exclusive end). Mirrors the `DATE_TRUNC('month', NOW())`
/// convention used throughout `usage_counters`.
fn current_billing_period() -> (DateTime<Utc>, DateTime<Utc>) {
    use chrono::{Datelike, NaiveDate, TimeZone};
    let now = Utc::now();
    let start_date = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .expect("month/year are valid by construction");
    let (next_year, next_month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    let end_date = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .expect("next month/year are valid by construction");
    let start = Utc.from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).expect("midnight is valid"));
    let end = Utc.from_utc_datetime(&end_date.and_hms_opt(0, 0, 0).expect("midnight is valid"));
    (start, end)
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Verify the caller belongs to the deployment's org, holds `permission`
/// on it, and the deployment exists within their resolved org. Returns
/// the resolved org context for downstream telemetry. Cross-org access
/// surfaces as "Deployment not found" rather than "forbidden" to avoid
/// leaking existence (matches the convention in
/// `hosted_mocks::delete_deployment` and
/// `notification_channels::load_authorized_channel`).
///
/// Mutating routes pass `Permission::HostedMockUpdate`; the read-only
/// usage endpoint passes `Permission::HostedMockRead`.
async fn authorize_deployment(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    deployment_id: Uuid,
    permission: Permission,
) -> ApiResult<crate::middleware::org_context::OrgContext> {
    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    let checker = PermissionChecker::new(state);
    checker.require_permission(user_id, org_ctx.org_id, permission).await?;

    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".into()))?;
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".into()));
    }

    Ok(org_ctx)
}

/// Load an attachment and verify it belongs to `deployment_id`.
/// Cross-deployment access surfaces as "not found" rather than
/// "forbidden" to avoid leaking existence.
async fn load_authorized_attachment(
    state: &AppState,
    deployment_id: Uuid,
    attachment_id: Uuid,
) -> ApiResult<HostedMockPlugin> {
    let row = HostedMockPlugin::find_by_id(state.db.pool(), attachment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Plugin attachment not found".into()))?;
    if row.deployment_id != deployment_id {
        return Err(ApiError::InvalidRequest("Plugin attachment not found".into()));
    }
    Ok(row)
}

/// Validate the `permissions_json` payload's *shape*. Per-key
/// `manifest ∩ grant` enforcement happens at the runtime layer; here
/// we only catch obvious client errors:
///   - Must be a JSON object.
///   - Top-level keys must be from `PERMISSION_SECTIONS`.
///   - Total payload size capped to keep the JSONB column tidy.
fn validate_permissions(value: &serde_json::Value) -> ApiResult<()> {
    let obj = value
        .as_object()
        .ok_or_else(|| ApiError::InvalidRequest("permissions must be a JSON object".into()))?;

    let allowed: HashSet<&str> = PERMISSION_SECTIONS.iter().copied().collect();
    for key in obj.keys() {
        if !allowed.contains(key.as_str()) {
            return Err(ApiError::InvalidRequest(format!(
                "permissions: unknown top-level key '{}' (allowed: {})",
                key,
                PERMISSION_SECTIONS.join(", "),
            )));
        }
    }

    let serialized = serde_json::to_vec(value)
        .map_err(|e| ApiError::InvalidRequest(format!("permissions failed to serialize: {}", e)))?;
    if serialized.len() > MAX_PERMISSIONS_BYTES {
        return Err(ApiError::InvalidRequest(format!(
            "permissions payload too large: {} bytes (max {} bytes)",
            serialized.len(),
            MAX_PERMISSIONS_BYTES,
        )));
    }

    Ok(())
}

/// Reject the attach if the deployment already has the plan-tier max
/// number of distinct active plugins. Re-attach of the same plugin is
/// not blocked (the UPSERT updates the existing row).
async fn enforce_plan_limit(
    state: &AppState,
    org_ctx: &crate::middleware::org_context::OrgContext,
    deployment_id: Uuid,
    plugin_id: Uuid,
) -> ApiResult<()> {
    let limits = &org_ctx.org.limits_json;
    // -1 = unlimited (per existing convention in hosted_mocks).
    // None or 0 = feature disabled — explicit upgrade required.
    let max = limits.get("max_plugins_per_mock").and_then(|v| v.as_i64()).unwrap_or(0);
    if max < 0 {
        return Ok(());
    }

    // Quick existence check first — re-attach doesn't bump the count.
    let already_attached = HostedMockPlugin::list_by_deployment(state.db.pool(), deployment_id)
        .await
        .map_err(ApiError::Database)?
        .iter()
        .any(|p| p.plugin_id == plugin_id && p.enabled);
    if already_attached {
        return Ok(());
    }

    let active = HostedMockPlugin::count_active_by_deployment(state.db.pool(), deployment_id)
        .await
        .map_err(ApiError::Database)?;
    if active >= max {
        return Err(ApiError::InvalidRequest(format!(
            "Plugin attachment limit reached: your plan allows {} active plugins per hosted mock. Upgrade to attach more.",
            max
        )));
    }
    Ok(())
}

fn client_metadata(headers: &HeaderMap) -> (Option<String>, Option<String>) {
    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
    (ip, ua)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_permissions_accepts_empty_object() {
        let v = serde_json::json!({});
        assert!(validate_permissions(&v).is_ok());
    }

    #[test]
    fn validate_permissions_accepts_known_sections() {
        let v = serde_json::json!({
            "egress": { "allow": ["*.stripe.com"] },
            "env": { "read": ["MY_FLAG"] },
            "request": { "read_body": true },
            "response": { "modify_body": true },
            "storage": { "kv_namespace": null },
        });
        assert!(validate_permissions(&v).is_ok());
    }

    #[test]
    fn validate_permissions_rejects_unknown_key() {
        let v = serde_json::json!({ "filesystem": { "read": "/etc" } });
        let err = validate_permissions(&v).unwrap_err();
        match err {
            ApiError::InvalidRequest(msg) => {
                assert!(msg.contains("unknown top-level key 'filesystem'"));
            }
            other => panic!("expected InvalidRequest, got {:?}", other),
        }
    }

    #[test]
    fn validate_permissions_rejects_non_object() {
        let v = serde_json::json!(["not", "an", "object"]);
        let err = validate_permissions(&v).unwrap_err();
        assert!(matches!(err, ApiError::InvalidRequest(_)));
    }

    #[test]
    fn validate_permissions_rejects_oversized_payload() {
        // Build something that exceeds MAX_PERMISSIONS_BYTES via a
        // huge value under a known-good key.
        let large = "x".repeat(MAX_PERMISSIONS_BYTES + 100);
        let v = serde_json::json!({ "egress": { "allow": [large] } });
        let err = validate_permissions(&v).unwrap_err();
        match err {
            ApiError::InvalidRequest(msg) => assert!(msg.contains("too large")),
            other => panic!("expected InvalidRequest, got {:?}", other),
        }
    }

    #[test]
    fn current_billing_period_starts_at_month_boundary() {
        let (start, end) = current_billing_period();
        // Both endpoints are at midnight UTC.
        assert_eq!(start.time(), chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(end.time(), chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        // Day-of-month is 1 for both — start of *some* month, exclusive
        // end at start of *next* month.
        assert_eq!(chrono::Datelike::day(&start), 1);
        assert_eq!(chrono::Datelike::day(&end), 1);
        // End is strictly after start.
        assert!(end > start);
        // The window covers ≥28 and ≤31 days (every calendar month).
        let span = end - start;
        assert!(
            span.num_days() >= 28 && span.num_days() <= 31,
            "span = {} days",
            span.num_days()
        );
    }

    #[test]
    fn into_usage_entry_drops_malformed_attachment_id() {
        let row = PluginInvokeAggregateRow {
            attachment_id: Some("not-a-uuid".to_string()),
            plugin_name: Some("foo".to_string()),
            plugin_version: Some("1.0.0".to_string()),
            invoke_ms: 100,
            memory_peak_mb: Some(42),
        };
        let entry = into_usage_entry(row);
        assert_eq!(entry.attachment_id, None);
        assert_eq!(entry.plugin_name, Some("foo".to_string()));
        assert_eq!(entry.invoke_ms, 100);
        assert_eq!(entry.memory_peak_mb, Some(42));
    }

    #[test]
    fn into_usage_entry_parses_well_formed_attachment_id() {
        let id = Uuid::new_v4();
        let row = PluginInvokeAggregateRow {
            attachment_id: Some(id.to_string()),
            plugin_name: None,
            plugin_version: None,
            invoke_ms: 0,
            memory_peak_mb: None,
        };
        let entry = into_usage_entry(row);
        assert_eq!(entry.attachment_id, Some(id));
        assert_eq!(entry.invoke_ms, 0);
        assert_eq!(entry.memory_peak_mb, None);
    }
}
