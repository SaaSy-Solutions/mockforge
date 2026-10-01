//! Contract Diff / Verification / Fitness Functions handlers
//! (cloud-enablement task #8 / Phase 1).
//!
//! Phase 1 surface: monitored-service CRUD, fitness-function CRUD,
//! verification-suite CRUD, diff-run + finding read paths. Probe worker
//! / scheduler / IncidentBus wiring land in follow-up slices.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/monitored-services
//!   POST   /api/v1/workspaces/{workspace_id}/monitored-services
//!   DELETE /api/v1/monitored-services/{id}
//!   GET    /api/v1/monitored-services/{id}/diffs
//!   GET    /api/v1/contract-diff-runs/{id}
//!   GET    /api/v1/contract-diff-runs/{id}/findings
//!
//!   GET    /api/v1/workspaces/{workspace_id}/fitness-functions
//!   POST   /api/v1/workspaces/{workspace_id}/fitness-functions
//!   DELETE /api/v1/fitness-functions/{id}
//!
//!   GET    /api/v1/workspaces/{workspace_id}/verification-suites
//!   POST   /api/v1/workspaces/{workspace_id}/verification-suites
//!   DELETE /api/v1/verification-suites/{id}

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::contract_verification::CreateMonitoredService;
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        CloudWorkspace, ContractDiffFinding, ContractDiffRun, FitnessFunction, MonitoredService,
        TestRun, VerificationSuite,
    },
    store::with_org_context,
    AppState,
};

const DEFAULT_RUN_LIMIT: i64 = 50;
const MAX_RUN_LIMIT: i64 = 500;

// --- monitored services ----------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/monitored-services`
pub async fn list_monitored_services(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<MonitoredService>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(MonitoredService::list_by_workspace(&mut **tx, workspace_id).await?) },
        )
    })
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateMonitoredServiceRequest {
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub openapi_spec_url: Option<String>,
    #[serde(default)]
    pub openapi_spec_inline: Option<serde_json::Value>,
    #[serde(default)]
    pub auth_config: Option<serde_json::Value>,
    pub traffic_source: String,
    #[serde(default)]
    pub traffic_source_ref: Option<String>,
    /// Optional per-service probe cadence override in seconds (#720).
    /// Must be > 0 when present; omit to follow the global default.
    #[serde(default)]
    pub probe_interval_secs: Option<i32>,
}

/// `POST /api/v1/workspaces/{workspace_id}/monitored-services`
pub async fn create_monitored_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateMonitoredServiceRequest>,
) -> ApiResult<Json<MonitoredService>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if request.base_url.trim().is_empty() {
        return Err(ApiError::InvalidRequest("base_url must not be empty".into()));
    }
    if !MonitoredService::is_valid_traffic_source(&request.traffic_source) {
        return Err(ApiError::InvalidRequest(format!(
            "traffic_source must be one of: {}",
            MonitoredService::VALID_TRAFFIC_SOURCES.join(", ")
        )));
    }
    if request.probe_interval_secs.is_some_and(|v| v <= 0) {
        return Err(ApiError::InvalidRequest(
            "probe_interval_secs must be a positive number of seconds".into(),
        ));
    }

    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(MonitoredService::create(
                &mut **tx,
                CreateMonitoredService {
                    workspace_id,
                    name: &request.name,
                    base_url: &request.base_url,
                    openapi_spec_url: request.openapi_spec_url.as_deref(),
                    openapi_spec_inline: request.openapi_spec_inline.as_ref(),
                    auth_config: request.auth_config.as_ref(),
                    traffic_source: &request.traffic_source,
                    traffic_source_ref: request.traffic_source_ref.as_deref(),
                    probe_interval_secs: request.probe_interval_secs,
                },
            )
            .await?)
        })
    })
    .await?;
    Ok(Json(row))
}

/// `DELETE /api/v1/monitored-services/{id}`
pub async fn delete_monitored_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_svc, org_id) = load_authorized_service(&state, user_id, &headers, id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(MonitoredService::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Monitored service not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// `GET /api/v1/monitored-services/{id}/diffs`
pub async fn list_service_diff_runs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ContractDiffRun>>> {
    let (_svc, org_id) = load_authorized_service(&state, user_id, &headers, id).await?;

    let runs = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(ContractDiffRun::list_by_service(&mut **tx, id, MAX_RUN_LIMIT).await?)
        })
    })
    .await?;
    let _ = DEFAULT_RUN_LIMIT; // reserved for future ?limit= query
    Ok(Json(runs))
}

/// `GET /api/v1/contract-diff-runs/{id}`
pub async fn get_diff_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ContractDiffRun>> {
    let (run, _org_id) = load_authorized_diff_run(&state, user_id, &headers, id).await?;
    Ok(Json(run))
}

/// `GET /api/v1/contract-diff-runs/{id}/findings`
pub async fn list_diff_findings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ContractDiffFinding>>> {
    let (_run, org_id) = load_authorized_diff_run(&state, user_id, &headers, id).await?;

    let findings = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(ContractDiffFinding::list_by_run(&mut **tx, id).await?) })
    })
    .await?;
    Ok(Json(findings))
}

/// `POST /api/v1/monitored-services/{id}/diff`
///
/// Triggers a contract diff run. Same lifecycle pattern as #4 test runs:
/// pushes a test_runs row + Redis job with kind='contract_diff'. The
/// runner-side ContractExecutor synthesizes findings until real impl
/// (ai_contract_diff pipeline) lands.
pub async fn trigger_diff_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestRun>> {
    // `org_id` is the service's workspace org, verified against the caller.
    let (svc, org_id) = load_authorized_service(&state, user_id, &headers, id).await?;

    let svc_id = svc.id;
    let run = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(TestRun::enqueue(
                &mut **tx,
                EnqueueTestRun {
                    suite_id: svc_id,
                    org_id,
                    kind: "contract_diff",
                    triggered_by: "manual",
                    triggered_by_user: Some(user_id),
                    git_ref: None,
                    git_sha: None,
                },
            )
            .await?)
        })
    })
    .await?;

    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: svc.id,
            kind: "contract_diff",
            payload: serde_json::json!({
                "service_name": svc.name,
                "base_url": svc.base_url,
                "openapi_spec_url": svc.openapi_spec_url,
                "traffic_source": svc.traffic_source,
                "workspace_id": svc.workspace_id,
            }),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue contract_diff run");
    }

    Ok(Json(run))
}

// --- fitness functions -----------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/fitness-functions`
pub async fn list_fitness_functions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<FitnessFunction>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(FitnessFunction::list_by_workspace(&mut **tx, workspace_id).await?) },
        )
    })
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateFitnessFunctionRequest {
    pub name: String,
    pub kind: String,
    pub config: serde_json::Value,
}

/// `POST /api/v1/workspaces/{workspace_id}/fitness-functions`
pub async fn create_fitness_function(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateFitnessFunctionRequest>,
) -> ApiResult<Json<FitnessFunction>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !FitnessFunction::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            FitnessFunction::VALID_KINDS.join(", ")
        )));
    }
    // `custom_query` runs arbitrary user-supplied evaluator code on
    // self-hosted MockForge — fine for trusted single-tenant
    // deployments, not safe to honour on shared cloud workers
    // (would let any workspace owner execute code on the runner
    // pool). Cloud rejects the kind at this boundary so a row in
    // the state matching `kind='custom_query'` never exists in
    // cloud, even if the user tried to bypass the UI gate.
    if request.kind == "custom_query" {
        return Err(ApiError::InvalidRequest(
            "kind 'custom_query' is self-hosted only — \
             arbitrary evaluator code isn't supported on cloud workers. \
             Use latency_threshold, error_rate, or contract_stability instead, \
             or run a self-hosted MockForge instance."
                .into(),
        ));
    }

    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(FitnessFunction::create(
                &mut **tx,
                workspace_id,
                &request.name,
                &request.kind,
                &request.config,
            )
            .await?)
        })
    })
    .await?;
    Ok(Json(row))
}

/// `PATCH /api/v1/fitness-functions/{id}`
///
/// Replace name + kind + config on an existing fitness function. The
/// surface mirrors `create_fitness_function` so the cloud UI can reuse
/// the same form payload for both create and edit.
///
/// 404 InvalidRequest when the row doesn't exist or the caller's org
/// doesn't own its workspace (cross-org access surfaces as not-found
/// to avoid leaking existence — matches `delete_fitness_function`).
pub async fn update_fitness_function(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateFitnessFunctionRequest>,
) -> ApiResult<Json<FitnessFunction>> {
    // Existence + auth check via the existing row's workspace_id, mirroring
    // delete_fitness_function. The body's workspace_id is implicit (we don't
    // allow re-homing fitness functions across workspaces in the same call).
    let (_existing, org_id) =
        load_authorized_fitness_function(&state, user_id, &headers, id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !FitnessFunction::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            FitnessFunction::VALID_KINDS.join(", ")
        )));
    }

    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(FitnessFunction::update(
                &mut **tx,
                id,
                request.name.trim(),
                &request.kind,
                &request.config,
            )
            .await?)
        })
    })
    .await?
    // Lost a race with a concurrent delete — surface as not-found, same
    // as the cross-org check above.
    .ok_or_else(|| ApiError::InvalidRequest("Fitness function not found".into()))?;
    Ok(Json(row))
}

/// `DELETE /api/v1/fitness-functions/{id}`
pub async fn delete_fitness_function(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_fn_row, org_id) = load_authorized_fitness_function(&state, user_id, &headers, id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(FitnessFunction::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Fitness function not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// --- verification suites ---------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/verification-suites`
pub async fn list_verification_suites(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<VerificationSuite>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(VerificationSuite::list_by_workspace(&mut **tx, workspace_id).await?) },
        )
    })
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateVerificationSuiteRequest {
    pub name: String,
    #[serde(default)]
    pub contract_check_ids: Vec<Uuid>,
    #[serde(default)]
    pub fitness_function_ids: Vec<Uuid>,
}

/// `POST /api/v1/workspaces/{workspace_id}/verification-suites`
pub async fn create_verification_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateVerificationSuiteRequest>,
) -> ApiResult<Json<VerificationSuite>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if request.contract_check_ids.is_empty() && request.fitness_function_ids.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Suite must reference at least one contract check or fitness function".into(),
        ));
    }

    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(VerificationSuite::create(
                &mut **tx,
                workspace_id,
                &request.name,
                &request.contract_check_ids,
                &request.fitness_function_ids,
            )
            .await?)
        })
    })
    .await?;
    Ok(Json(row))
}

/// `DELETE /api/v1/verification-suites/{id}`
pub async fn delete_verification_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = resolve_org(&state, user_id, &headers).await?;
    let suite = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(VerificationSuite::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Verification suite not found".into()))?;
    check_workspace_in_org(&state, org_id, suite.workspace_id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(VerificationSuite::delete(&mut **tx, id).await?) })
    })
    .await?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Verification suite not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Verify `workspace_id` belongs to the caller's org; returns that org id.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<Uuid> {
    let org_id = resolve_org(state, user_id, headers).await?;
    check_workspace_in_org(state, org_id, workspace_id).await?;
    Ok(org_id)
}

/// Resolve the caller's org (header / default org).
async fn resolve_org(state: &AppState, user_id: Uuid, headers: &HeaderMap) -> ApiResult<Uuid> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    Ok(ctx.org_id)
}

/// Verify `workspace_id` belongs to the already-resolved caller org.
/// See `captures::as_not_found`: a loaded row whose workspace fails the org
/// check must answer exactly like a missing row.
fn as_not_found(err: ApiError, message: &str) -> ApiError {
    match err {
        ApiError::InvalidRequest(_) => ApiError::InvalidRequest(message.into()),
        other => other,
    }
}

async fn check_workspace_in_org(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<()> {
    // Bound to the caller's org: a workspace in another org reads as absent
    // and yields the same "Workspace not found" as an explicit mismatch.
    let workspace = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(())
}

/// Load a monitored service bound to the caller's org, then verify its
/// workspace. A service in another org reads as absent and yields the same
/// "Monitored service not found" as a missing row.
async fn load_authorized_service(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(MonitoredService, Uuid)> {
    let org_id = resolve_org(state, user_id, headers).await?;
    let svc = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(MonitoredService::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Monitored service not found".into()))?;
    check_workspace_in_org(state, org_id, svc.workspace_id)
        .await
        .map_err(|e| as_not_found(e, "Monitored service not found"))?;
    Ok((svc, org_id))
}

/// Load a diff run and its monitored service bound to the caller's org, then
/// verify the service's workspace. Either row in another org reads as absent
/// and yields the same "Diff run not found" as a missing row.
async fn load_authorized_diff_run(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(ContractDiffRun, Uuid)> {
    let org_id = resolve_org(state, user_id, headers).await?;
    let found = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            let Some(run) = ContractDiffRun::find_by_id(&mut **tx, id).await? else {
                return Ok(None);
            };
            let svc = MonitoredService::find_by_id(&mut **tx, run.monitored_service_id).await?;
            Ok(svc.map(|svc| (run, svc)))
        })
    })
    .await?;
    let (run, svc) = found.ok_or_else(|| ApiError::InvalidRequest("Diff run not found".into()))?;
    check_workspace_in_org(state, org_id, svc.workspace_id)
        .await
        .map_err(|e| as_not_found(e, "Diff run not found"))?;
    Ok((run, org_id))
}

/// Load a fitness function bound to the caller's org, then verify its
/// workspace. A function in another org reads as "Fitness function not found".
async fn load_authorized_fitness_function(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(FitnessFunction, Uuid)> {
    let org_id = resolve_org(state, user_id, headers).await?;
    let row = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(FitnessFunction::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Fitness function not found".into()))?;
    check_workspace_in_org(state, org_id, row.workspace_id)
        .await
        .map_err(|e| as_not_found(e, "Fitness function not found"))?;
    Ok((row, org_id))
}
