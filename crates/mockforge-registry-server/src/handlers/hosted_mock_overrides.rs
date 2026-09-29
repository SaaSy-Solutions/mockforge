//! Response override rules for a hosted mock.
//!
//! The registry is the source of truth: rules live in the deployment's
//! `metadata_json.override_rules`, reach new machines through the
//! `MOCKFORGE_HTTP_OVERRIDES` env var, and are pushed to a running machine's
//! admin server (`{app}.internal:9080/__mockforge/overrides`, private 6PN)
//! on every save.

use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_core::{OverrideRule, Overrides};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::resilience::{admin_base_url, authorize_deployment},
    middleware::{permission_check::PermissionChecker, permissions::Permission, AuthUser},
    models::HostedMock,
    AppState,
};

const METADATA_KEY: &str = "override_rules";
const PUSH_TIMEOUT: Duration = Duration::from_secs(3);

/// Whether the running machine picked up a saved rule set
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSync {
    /// The machine is serving the new rules now
    Applied,
    /// The machine runs a MockForge version without the overrides API; the
    /// rules apply after the next redeploy
    Outdated,
    /// The machine could not be reached; the rules apply when it restarts
    Unreachable,
}

/// Request and response body for the overrides endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct OverrideRulesBody {
    /// Rules in application order
    pub rules: Vec<OverrideRule>,
}

/// Response to a save
#[derive(Debug, Serialize)]
pub struct SaveOverridesResponse {
    /// The stored rules
    pub rules: Vec<OverrideRule>,
    /// Whether the running machine picked them up
    pub runtime: RuntimeSync,
}

/// The rules stored on a deployment. Malformed data reads as no rules.
pub fn stored_rules(deployment: &HostedMock) -> Vec<OverrideRule> {
    deployment
        .metadata_json
        .get(METADATA_KEY)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// The `MOCKFORGE_HTTP_OVERRIDES` entry for a machine's env, if it has rules
pub fn overrides_env(deployment: &HostedMock) -> Option<(String, String)> {
    let rules = stored_rules(deployment);
    if rules.is_empty() {
        return None;
    }
    serde_json::to_string(&rules)
        .ok()
        .map(|json| (mockforge_core::overrides::OVERRIDES_INLINE_ENV.to_string(), json))
}

/// `GET /api/v1/hosted-mocks/{deployment_id}/overrides`
pub async fn get_overrides(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<OverrideRulesBody>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    Ok(Json(OverrideRulesBody {
        rules: stored_rules(&deployment),
    }))
}

/// `PUT /api/v1/hosted-mocks/{deployment_id}/overrides`
pub async fn put_overrides(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<OverrideRulesBody>,
) -> ApiResult<Json<SaveOverridesResponse>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    PermissionChecker::new(&state)
        .require_permission(user_id, deployment.org_id, Permission::HostedMockUpdate)
        .await?;

    let validated =
        Overrides::new(body.rules).map_err(|e| ApiError::ValidationFailed(e.to_string()))?;
    let rules = validated.rules().to_vec();
    let rules_json = serde_json::to_value(&rules)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("serialize override rules: {e}")))?;

    // #832: write under the org GUC so the hosted_mocks RLS policy scopes the
    // UPDATE to the caller's org on the runtime (NOBYPASSRLS) pool.
    let deployment_id_c = deployment.id;
    let updated =
        crate::store::with_org_context(state.db.runtime_pool(), deployment.org_id, move |tx| {
            Box::pin(async move {
                sqlx::query(
                    "UPDATE hosted_mocks SET metadata_json = jsonb_set(\
                     COALESCE(metadata_json, '{}'::jsonb), '{override_rules}', $1, true), \
                     updated_at = NOW() WHERE id = $2",
                )
                .bind(&rules_json)
                .bind(deployment_id_c)
                .execute(&mut **tx)
                .await
                .map(|result| result.rows_affected())
                .map_err(Into::into)
            })
        })
        .await?;
    if updated == 0 {
        return Err(ApiError::InvalidRequest("Deployment not found".into()));
    }

    let runtime = push_to_runtime(&deployment, &rules).await;
    tracing::info!(%deployment_id, rules = rules.len(), ?runtime, "saved hosted-mock override rules");
    Ok(Json(SaveOverridesResponse { rules, runtime }))
}

async fn push_to_runtime(deployment: &HostedMock, rules: &[OverrideRule]) -> RuntimeSync {
    let url = format!("{}/__mockforge/overrides", admin_base_url(deployment));
    let client = match reqwest::Client::builder().timeout(PUSH_TIMEOUT).build() {
        Ok(client) => client,
        Err(e) => {
            tracing::warn!(error = %e, "override push client build failed");
            return RuntimeSync::Unreachable;
        }
    };
    match client.put(&url).json(&serde_json::json!({ "rules": rules })).send().await {
        Ok(r) if r.status().is_success() => RuntimeSync::Applied,
        Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND => RuntimeSync::Outdated,
        Ok(r) => {
            tracing::warn!(url, status = %r.status(), "override push rejected by runtime");
            RuntimeSync::Unreachable
        }
        Err(e) => {
            tracing::warn!(url, error = %e, "override push failed");
            RuntimeSync::Unreachable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn deployment(metadata: serde_json::Value) -> HostedMock {
        let now = chrono::Utc::now();
        HostedMock {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            project_id: None,
            name: "demo".into(),
            slug: "demo".into(),
            description: None,
            config_json: json!({}),
            openapi_spec_url: None,
            status: "active".into(),
            deployment_url: None,
            internal_url: None,
            region: "iad".into(),
            instance_type: "shared-cpu-1x".into(),
            health_check_url: None,
            last_health_check: None,
            health_status: "unknown".into(),
            error_message: None,
            metadata_json: metadata,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    #[test]
    fn stored_rules_become_the_machine_env() {
        let with_rules = deployment(json!({
            "flyio_machine_id": "m1",
            "override_rules": [{"targets": ["*"], "patch": [{"op": "add", "path": "/x", "value": 1}]}]
        }));
        assert_eq!(stored_rules(&with_rules).len(), 1);
        let (key, value) = overrides_env(&with_rules).unwrap();
        assert_eq!(key, "MOCKFORGE_HTTP_OVERRIDES");
        let parsed: Vec<OverrideRule> = serde_json::from_str(&value).unwrap();
        assert!(Overrides::new(parsed).is_ok());
    }

    #[test]
    fn no_or_malformed_rules_mean_no_env() {
        assert!(overrides_env(&deployment(json!({}))).is_none());
        assert!(overrides_env(&deployment(json!({"override_rules": "nope"}))).is_none());
    }

    #[test]
    fn runtime_sync_serializes_snake_case() {
        assert_eq!(serde_json::to_value(RuntimeSync::Outdated).unwrap(), json!("outdated"));
    }
}
