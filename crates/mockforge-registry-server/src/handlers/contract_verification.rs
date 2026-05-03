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
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        CloudWorkspace, ContractDiffFinding, ContractDiffRun, FitnessFunction, MonitoredService,
        VerificationSuite,
    },
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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = MonitoredService::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
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
}

/// `POST /api/v1/workspaces/{workspace_id}/monitored-services`
pub async fn create_monitored_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateMonitoredServiceRequest>,
) -> ApiResult<Json<MonitoredService>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

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

    let row = MonitoredService::create(
        state.db.pool(),
        CreateMonitoredService {
            workspace_id,
            name: &request.name,
            base_url: &request.base_url,
            openapi_spec_url: request.openapi_spec_url.as_deref(),
            openapi_spec_inline: request.openapi_spec_inline.as_ref(),
            auth_config: request.auth_config.as_ref(),
            traffic_source: &request.traffic_source,
            traffic_source_ref: request.traffic_source_ref.as_deref(),
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

/// `DELETE /api/v1/monitored-services/{id}`
pub async fn delete_monitored_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let svc = MonitoredService::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Monitored service not found".into()))?;
    authorize_workspace(&state, user_id, &headers, svc.workspace_id).await?;

    let deleted = MonitoredService::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
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
    let svc = MonitoredService::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Monitored service not found".into()))?;
    authorize_workspace(&state, user_id, &headers, svc.workspace_id).await?;

    let runs = ContractDiffRun::list_by_service(state.db.pool(), id, MAX_RUN_LIMIT)
        .await
        .map_err(ApiError::Database)?;
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
    let run = ContractDiffRun::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Diff run not found".into()))?;
    let svc = MonitoredService::find_by_id(state.db.pool(), run.monitored_service_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Diff run not found".into()))?;
    authorize_workspace(&state, user_id, &headers, svc.workspace_id).await?;
    Ok(Json(run))
}

/// `GET /api/v1/contract-diff-runs/{id}/findings`
pub async fn list_diff_findings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ContractDiffFinding>>> {
    let run = ContractDiffRun::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Diff run not found".into()))?;
    let svc = MonitoredService::find_by_id(state.db.pool(), run.monitored_service_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Diff run not found".into()))?;
    authorize_workspace(&state, user_id, &headers, svc.workspace_id).await?;

    let findings = ContractDiffFinding::list_by_run(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(findings))
}

// --- fitness functions -----------------------------------------------------

/// `GET /api/v1/workspaces/{workspace_id}/fitness-functions`
pub async fn list_fitness_functions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<FitnessFunction>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = FitnessFunction::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !FitnessFunction::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            FitnessFunction::VALID_KINDS.join(", ")
        )));
    }

    let row = FitnessFunction::create(
        state.db.pool(),
        workspace_id,
        &request.name,
        &request.kind,
        &request.config,
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

/// `DELETE /api/v1/fitness-functions/{id}`
pub async fn delete_fitness_function(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let fn_row = FitnessFunction::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Fitness function not found".into()))?;
    authorize_workspace(&state, user_id, &headers, fn_row.workspace_id).await?;

    let deleted = FitnessFunction::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows = VerificationSuite::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
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
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if request.contract_check_ids.is_empty() && request.fitness_function_ids.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Suite must reference at least one contract check or fitness function".into(),
        ));
    }

    let row = VerificationSuite::create(
        state.db.pool(),
        workspace_id,
        &request.name,
        &request.contract_check_ids,
        &request.fitness_function_ids,
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

/// `DELETE /api/v1/verification-suites/{id}`
pub async fn delete_verification_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let suite = VerificationSuite::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Verification suite not found".into()))?;
    authorize_workspace(&state, user_id, &headers, suite.workspace_id).await?;

    let deleted = VerificationSuite::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Verification suite not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(())
}
