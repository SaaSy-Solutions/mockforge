//! Federation CRUD handlers

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_federation::ServiceBoundary;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{AuditEventType, FeatureType, Federation},
    AppState,
};

/// Parse the stored services JSON into typed `ServiceBoundary` values.
///
/// A null value (or a literal `null` cell in the database) is treated as an
/// empty service list — matching the behavior of `create_federation`.
fn parse_services(value: &serde_json::Value) -> Result<Vec<ServiceBoundary>, ApiError> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    serde_json::from_value(value.clone())
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid services payload: {e}")))
}

/// Validate the service list: duplicate detection and acyclic dependency graph.
///
/// Called from `create_federation` and `update_federation` so the UI's
/// dependency input surfaces a 400 instead of silently persisting a broken
/// federation.
fn validate_services(value: &serde_json::Value) -> Result<(), ApiError> {
    let services = parse_services(value)?;

    // Duplicate name / base_path detection.
    let mut names: HashSet<&str> = HashSet::new();
    let mut base_paths: HashSet<&str> = HashSet::new();
    for service in &services {
        if !names.insert(service.name.as_str()) {
            return Err(ApiError::InvalidRequest(format!(
                "Duplicate service name: '{}'",
                service.name
            )));
        }
        if !base_paths.insert(service.base_path.as_str()) {
            return Err(ApiError::InvalidRequest(format!(
                "Duplicate base_path: '{}'",
                service.base_path
            )));
        }
    }

    // Every dependency must reference a known service, and nothing may depend
    // on itself.
    for service in &services {
        for dep in &service.dependencies {
            if dep == &service.name {
                return Err(ApiError::InvalidRequest(format!(
                    "Service '{}' cannot depend on itself",
                    service.name
                )));
            }
            if !names.contains(dep.as_str()) {
                return Err(ApiError::InvalidRequest(format!(
                    "Service '{}' depends on unknown service '{}'",
                    service.name, dep
                )));
            }
        }
    }

    // Kahn's algorithm for cycle detection.
    let mut indegree: HashMap<&str, usize> =
        services.iter().map(|s| (s.name.as_str(), 0usize)).collect();
    for service in &services {
        for _ in &service.dependencies {
            if let Some(slot) = indegree.get_mut(service.name.as_str()) {
                *slot += 1;
            }
        }
    }
    let mut queue: Vec<&str> = indegree
        .iter()
        .filter_map(|(name, deg)| if *deg == 0 { Some(*name) } else { None })
        .collect();
    let mut visited = 0usize;
    while let Some(name) = queue.pop() {
        visited += 1;
        for service in &services {
            if service.dependencies.iter().any(|d| d == name) {
                if let Some(slot) = indegree.get_mut(service.name.as_str()) {
                    *slot -= 1;
                    if *slot == 0 {
                        queue.push(service.name.as_str());
                    }
                }
            }
        }
    }
    if visited != services.len() {
        return Err(ApiError::InvalidRequest(
            "Circular dependency detected in services".to_string(),
        ));
    }

    Ok(())
}

/// List all federations for the user's organization
pub async fn list_federations(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Federation>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federations = state.store.list_federations_by_org(org_ctx.org_id).await?;

    Ok(Json(federations))
}

/// Get a single federation by ID
pub async fn get_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    Ok(Json(federation))
}

/// Create a new federation
#[derive(Debug, Deserialize)]
pub struct CreateFederationRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub services: serde_json::Value,
}

pub async fn create_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateFederationRequest>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Federation name is required".to_string()));
    }

    // Default services to empty array if null
    let services = if request.services.is_null() {
        serde_json::json!([])
    } else {
        request.services
    };

    validate_services(&services)?;

    let federation = state
        .store
        .create_federation(
            org_ctx.org_id,
            user_id,
            request.name.trim(),
            &request.description,
            &services,
        )
        .await?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationCreate,
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
        )
        .await;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationCreated,
            format!("Federation '{}' created", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(federation))
}

/// Update an existing federation
#[derive(Debug, Deserialize)]
pub struct UpdateFederationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub services: Option<serde_json::Value>,
}

pub async fn update_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFederationRequest>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify federation exists and belongs to org
    let existing = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    if let Some(ref services) = request.services {
        validate_services(services)?;
    }

    let federation = state
        .store
        .update_federation(
            id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.services.as_ref(),
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationUpdate,
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
        )
        .await;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationUpdated,
            format!("Federation '{}' updated", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(federation))
}

/// Delete a federation
pub async fn delete_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify federation exists and belongs to org
    let federation = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    // Record audit log before deletion
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationDeleted,
            format!("Federation '{}' deleted", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
            ip_address,
            user_agent,
        )
        .await;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationDelete,
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
        )
        .await;

    state.store.delete_federation(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Request body for `POST /api/v1/federation/{id}/route`.
///
/// Only `path` is consumed today. `method`, `headers`, and `body` are accepted
/// so the admin UI can send a full request envelope forward-compatibly — a
/// future enhancement can use them for dry-run proxying.
#[derive(Debug, Deserialize)]
pub struct RouteFederationRequest {
    pub path: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// Response body for `POST /api/v1/federation/{id}/route`.
///
/// Shape matches `RouteResponse` in `crates/mockforge-ui/ui/src/hooks/useFederation.ts`.
#[derive(Debug, Serialize)]
pub struct RouteFederationResponse {
    pub workspace_id: Uuid,
    pub service: ServiceBoundary,
    pub service_path: String,
}

/// Resolve a request path against a federation's service boundaries.
///
/// Returns the matching workspace, the full `ServiceBoundary` (including its
/// `reality_level`, which downstream workspace handling consults — the router
/// itself is reality-level agnostic), and the path with the service's
/// `base_path` stripped off.
pub async fn route_federation_request(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<RouteFederationRequest>,
) -> ApiResult<Json<RouteFederationResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    let services = parse_services(&federation.services)?;

    // Longest-matching base_path wins, mirroring `Federation::find_service_by_path`.
    let service = services
        .iter()
        .filter(|s| s.matches_path(&request.path))
        .max_by_key(|s| s.base_path.len())
        .ok_or_else(|| {
            ApiError::InvalidRequest(format!(
                "No service in federation matches path '{}'",
                request.path
            ))
        })?;

    let service_path = service.extract_service_path(&request.path).ok_or_else(|| {
        ApiError::InvalidRequest(format!("Could not extract service path from '{}'", request.path))
    })?;

    Ok(Json(RouteFederationResponse {
        workspace_id: service.workspace_id,
        service: service.clone(),
        service_path,
    }))
}

// =====================================================================
// Federation-wide scenario activations
// =====================================================================

use crate::models::{FederationScenarioActivation, PerServiceActivationState};
use mockforge_scenarios::ServiceScenarioOverride;

/// Request body for `POST /api/v1/federation/{id}/scenarios/activate`.
///
/// Either a stored scenario (via `scenario_id`) or an inline manifest may be
/// activated. At least one must be provided; if both are given, the inline
/// `manifest` wins and `scenario_id` is recorded for audit.
#[derive(Debug, Deserialize)]
pub struct ActivateScenarioRequest {
    /// Registry scenario ID. Recorded for audit when present.
    #[serde(default)]
    pub scenario_id: Option<Uuid>,
    /// Inline scenario manifest JSON. Parsed as
    /// `mockforge_scenarios::ScenarioManifest` for validation.
    #[serde(default)]
    pub manifest: Option<serde_json::Value>,
    /// Display name of the scenario (falls back to `manifest.name`).
    #[serde(default)]
    pub scenario_name: Option<String>,
    /// Per-service overrides keyed by `ServiceBoundary.name`. Supplements
    /// anything already in the manifest's `service_overrides` field — these
    /// take precedence on key conflicts.
    #[serde(default)]
    pub service_overrides: HashMap<String, ServiceScenarioOverride>,
}

/// Response shape for activation endpoints. Matches the UI's hook types.
#[derive(Debug, Serialize)]
pub struct FederationScenarioActivationResponse {
    pub id: Uuid,
    pub federation_id: Uuid,
    pub scenario_id: Option<Uuid>,
    pub scenario_name: String,
    pub manifest_snapshot: serde_json::Value,
    pub service_overrides: serde_json::Value,
    pub status: String,
    pub per_service_state: Vec<PerServiceActivationState>,
    pub activated_by: Uuid,
    pub activated_at: chrono::DateTime<chrono::Utc>,
    pub deactivated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FederationScenarioActivationResponse {
    fn from_row(row: FederationScenarioActivation) -> Self {
        let per_service_state = row.parse_per_service_state().unwrap_or_default();
        Self {
            id: row.id,
            federation_id: row.federation_id,
            scenario_id: row.scenario_id,
            scenario_name: row.scenario_name,
            manifest_snapshot: row.manifest_snapshot,
            service_overrides: row.service_overrides,
            status: row.status,
            per_service_state,
            activated_by: row.activated_by,
            activated_at: row.activated_at,
            deactivated_at: row.deactivated_at,
        }
    }
}

/// Merge scenario-provided overrides with request-provided overrides.
///
/// Request overrides win on key conflict. Exposed to tests.
fn merge_overrides(
    from_manifest: &HashMap<String, ServiceScenarioOverride>,
    from_request: &HashMap<String, ServiceScenarioOverride>,
) -> HashMap<String, ServiceScenarioOverride> {
    let mut out = from_manifest.clone();
    for (k, v) in from_request {
        out.insert(k.clone(), v.clone());
    }
    out
}

/// Validate overrides reference real services and contain in-range values.
fn validate_activation_overrides(
    federation_services: &[ServiceBoundary],
    overrides: &HashMap<String, ServiceScenarioOverride>,
) -> Result<(), ApiError> {
    let known: HashSet<&str> = federation_services.iter().map(|s| s.name.as_str()).collect();
    for (service_name, override_cfg) in overrides {
        if !known.contains(service_name.as_str()) {
            return Err(ApiError::InvalidRequest(format!(
                "Override references unknown service '{service_name}'"
            )));
        }
        override_cfg
            .validate()
            .map_err(|e| ApiError::InvalidRequest(format!("Override for '{service_name}': {e}")))?;
    }
    Ok(())
}

/// Build the initial per-service state — every federation service starts in
/// `pending` state; the runtime poller flips it to `applied`.
fn build_initial_per_service_state(services: &[ServiceBoundary]) -> Vec<PerServiceActivationState> {
    services
        .iter()
        .map(|s| PerServiceActivationState {
            service_name: s.name.clone(),
            workspace_id: s.workspace_id,
            status: "pending".to_string(),
            error: None,
            last_observed_at: None,
        })
        .collect()
}

/// `POST /api/v1/federation/{id}/scenarios/activate`
///
/// Activate a scenario across every service in the federation. Rejects
/// activation when a scenario is already active (deactivate first).
pub async fn activate_federation_scenario(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(federation_id): Path<Uuid>,
    Json(request): Json<ActivateScenarioRequest>,
) -> ApiResult<Json<FederationScenarioActivationResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;
    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    if state
        .store
        .find_active_federation_scenario_activation(federation_id)
        .await?
        .is_some()
    {
        return Err(ApiError::InvalidRequest(
            "Federation already has an active scenario; deactivate it first".to_string(),
        ));
    }

    let manifest_value = request
        .manifest
        .ok_or_else(|| ApiError::InvalidRequest("manifest is required".to_string()))?;

    let manifest: mockforge_scenarios::ScenarioManifest =
        serde_json::from_value(manifest_value.clone())
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid scenario manifest: {e}")))?;

    let services = parse_services(&federation.services)?;
    let merged_overrides = merge_overrides(&manifest.service_overrides, &request.service_overrides);
    validate_activation_overrides(&services, &merged_overrides)?;

    let merged_overrides_json = serde_json::to_value(&merged_overrides)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to encode overrides: {e}")))?;
    let per_service_state = build_initial_per_service_state(&services);
    let per_service_state_json = serde_json::to_value(&per_service_state).map_err(|e| {
        ApiError::InvalidRequest(format!("Failed to encode per-service state: {e}"))
    })?;

    let scenario_name = request.scenario_name.clone().unwrap_or_else(|| manifest.name.clone());

    let activation = state
        .store
        .create_federation_scenario_activation(
            federation_id,
            request.scenario_id,
            &scenario_name,
            &manifest_value,
            &merged_overrides_json,
            &per_service_state_json,
            user_id,
        )
        .await?;

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationScenarioActivate,
            Some(serde_json::json!({
                "federation_id": federation_id,
                "activation_id": activation.id,
                "scenario_name": scenario_name,
            })),
        )
        .await;

    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationScenarioActivated,
            format!("Scenario '{scenario_name}' activated on federation '{}'", federation.name),
            Some(serde_json::json!({
                "federation_id": federation_id,
                "activation_id": activation.id,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(FederationScenarioActivationResponse::from_row(activation)))
}

/// `GET /api/v1/federation/{id}/scenarios/active`
pub async fn get_active_federation_scenario(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(federation_id): Path<Uuid>,
) -> ApiResult<Json<Option<FederationScenarioActivationResponse>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;
    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    let activation = state.store.find_active_federation_scenario_activation(federation_id).await?;

    Ok(Json(activation.map(FederationScenarioActivationResponse::from_row)))
}

/// `DELETE /api/v1/federation/{id}/scenarios/active`
///
/// Deactivates the currently-active scenario. Workspaces observing the poll
/// endpoint will stop receiving overrides on their next tick.
pub async fn deactivate_federation_scenario(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(federation_id): Path<Uuid>,
) -> ApiResult<Json<FederationScenarioActivationResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;
    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    let active = state
        .store
        .find_active_federation_scenario_activation(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("No active scenario to deactivate".to_string()))?;

    let deactivated = state
        .store
        .deactivate_federation_scenario_activation(active.id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Failed to deactivate scenario".to_string()))?;

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationScenarioDeactivate,
            Some(serde_json::json!({
                "federation_id": federation_id,
                "activation_id": deactivated.id,
                "scenario_name": deactivated.scenario_name,
            })),
        )
        .await;

    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationScenarioDeactivated,
            format!(
                "Scenario '{}' deactivated on federation '{}'",
                deactivated.scenario_name, federation.name
            ),
            Some(serde_json::json!({
                "federation_id": federation_id,
                "activation_id": deactivated.id,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(FederationScenarioActivationResponse::from_row(deactivated)))
}

/// Request body for `POST /api/v1/federation/{id}/scenarios/active/report`.
#[derive(Debug, Deserialize)]
pub struct ReportPerServiceStateRequest {
    pub service_name: String,
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
}

/// `POST /api/v1/federation/{id}/scenarios/active/report`
///
/// Runtime-side callback: a workspace (or runtime poller) reports that it has
/// observed and applied (or failed to apply) its share of the active
/// scenario. This flips the per-service state from `pending` → `applied`
/// (or `failed`).
pub async fn report_federation_scenario_state(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(federation_id): Path<Uuid>,
    Json(request): Json<ReportPerServiceStateRequest>,
) -> ApiResult<Json<FederationScenarioActivationResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;
    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    if !matches!(request.status.as_str(), "pending" | "applied" | "failed") {
        return Err(ApiError::InvalidRequest(format!(
            "status must be one of pending|applied|failed, got '{}'",
            request.status
        )));
    }

    let active = state
        .store
        .find_active_federation_scenario_activation(federation_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("No active scenario".to_string()))?;

    let mut entries = active
        .parse_per_service_state()
        .map_err(|e| ApiError::InvalidRequest(format!("Corrupt per-service state: {e}")))?;

    let entry =
        entries
            .iter_mut()
            .find(|s| s.service_name == request.service_name)
            .ok_or_else(|| {
                ApiError::InvalidRequest(format!(
                    "Service '{}' not in federation",
                    request.service_name
                ))
            })?;

    entry.status = request.status.clone();
    entry.error = request.error.clone();
    entry.last_observed_at = Some(chrono::Utc::now());

    let entries_json = serde_json::to_value(&entries).map_err(|e| {
        ApiError::InvalidRequest(format!("Failed to encode per-service state: {e}"))
    })?;

    let updated = state
        .store
        .update_federation_scenario_per_service_state(active.id, &entries_json)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Activation disappeared mid-update".to_string()))?;

    Ok(Json(FederationScenarioActivationResponse::from_row(updated)))
}

// DTOs are defined in `mockforge_scenarios::federation_runtime` so the
// registry server and runtime-side poller share the exact same wire shape.
pub use mockforge_scenarios::{WorkspaceActiveScenarioEntry, WorkspaceActiveScenariosResponse};

/// `GET /api/v1/workspaces/{workspace_id}/active-scenarios`
///
/// Runtime poll endpoint. Returns every federation-scenario override that
/// currently applies to this workspace. Idempotent, cheap, safe to call on
/// an interval.
pub async fn get_workspace_active_scenarios(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<WorkspaceActiveScenariosResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = state
        .store
        .find_cloud_workspace_by_id(workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;
    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let activations =
        state.store.find_active_federation_scenarios_for_workspace(workspace_id).await?;

    let mut entries = Vec::new();
    for activation in activations {
        // Pull the federation name for the response. Not fatal if missing.
        let federation_name = state
            .store
            .find_federation_by_id(activation.federation_id)
            .await
            .ok()
            .flatten()
            .map(|f| f.name)
            .unwrap_or_default();

        // Find each service in this federation bound to the target workspace,
        // and look up that service's override (if any).
        let services_json = state
            .store
            .find_federation_by_id(activation.federation_id)
            .await?
            .map(|f| f.services)
            .unwrap_or_else(|| serde_json::json!([]));
        let services: Vec<ServiceBoundary> =
            serde_json::from_value(services_json).unwrap_or_default();

        let overrides_map: HashMap<String, ServiceScenarioOverride> =
            serde_json::from_value(activation.service_overrides.clone()).unwrap_or_default();

        for svc in services.iter().filter(|s| s.workspace_id == workspace_id) {
            entries.push(WorkspaceActiveScenarioEntry {
                activation_id: activation.id,
                federation_id: activation.federation_id,
                federation_name: federation_name.clone(),
                scenario_name: activation.scenario_name.clone(),
                service_name: svc.name.clone(),
                override_config: overrides_map.get(&svc.name).cloned(),
            });
        }
    }

    Ok(Json(WorkspaceActiveScenariosResponse {
        workspace_id,
        entries,
    }))
}

#[cfg(test)]
mod tests {
    use super::{
        build_initial_per_service_state, merge_overrides, parse_services,
        validate_activation_overrides, validate_services,
    };
    use mockforge_federation::{ServiceBoundary, ServiceRealityLevel};
    use mockforge_scenarios::ServiceScenarioOverride;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn service(name: &str, base_path: &str, deps: &[&str]) -> serde_json::Value {
        json!({
            "name": name,
            "workspace_id": Uuid::new_v4().to_string(),
            "base_path": base_path,
            "reality_level": "real",
            "dependencies": deps,
        })
    }

    #[test]
    fn parse_services_accepts_null_as_empty() {
        assert!(parse_services(&serde_json::Value::Null).unwrap().is_empty());
    }

    #[test]
    fn parse_services_accepts_empty_array() {
        assert!(parse_services(&json!([])).unwrap().is_empty());
    }

    #[test]
    fn parse_services_rejects_malformed_payload() {
        let bad = json!([{"name": "missing-fields"}]);
        assert!(parse_services(&bad).is_err());
    }

    #[test]
    fn validate_services_accepts_valid_graph() {
        let services = json!([
            service("auth", "/auth", &[]),
            service("catalog", "/catalog", &["auth"]),
            service("checkout", "/checkout", &["auth", "catalog"]),
        ]);
        validate_services(&services).unwrap();
    }

    #[test]
    fn validate_services_rejects_duplicate_name() {
        let services = json!([
            service("auth", "/auth", &[]),
            service("auth", "/other", &[]),
        ]);
        let err = validate_services(&services).unwrap_err().to_string();
        assert!(err.contains("Duplicate service name"), "got: {err}");
    }

    #[test]
    fn validate_services_rejects_duplicate_base_path() {
        let services = json!([
            service("auth", "/api", &[]),
            service("payments", "/api", &[]),
        ]);
        let err = validate_services(&services).unwrap_err().to_string();
        assert!(err.contains("Duplicate base_path"), "got: {err}");
    }

    #[test]
    fn validate_services_rejects_unknown_dependency() {
        let services = json!([service("auth", "/auth", &["missing"]),]);
        let err = validate_services(&services).unwrap_err().to_string();
        assert!(err.contains("depends on unknown service"), "got: {err}");
    }

    #[test]
    fn validate_services_rejects_self_dependency() {
        let services = json!([service("auth", "/auth", &["auth"]),]);
        let err = validate_services(&services).unwrap_err().to_string();
        assert!(err.contains("cannot depend on itself"), "got: {err}");
    }

    #[test]
    fn validate_services_rejects_cycle() {
        let services = json!([
            service("a", "/a", &["b"]),
            service("b", "/b", &["c"]),
            service("c", "/c", &["a"]),
        ]);
        let err = validate_services(&services).unwrap_err().to_string();
        assert!(err.contains("Circular dependency"), "got: {err}");
    }

    #[test]
    fn validate_services_accepts_empty_and_null() {
        validate_services(&json!([])).unwrap();
        validate_services(&serde_json::Value::Null).unwrap();
    }

    fn boundary(name: &str, base_path: &str) -> ServiceBoundary {
        ServiceBoundary::new(
            name.to_string(),
            Uuid::new_v4(),
            base_path.to_string(),
            ServiceRealityLevel::Real,
        )
    }

    #[test]
    fn merge_overrides_request_wins_on_conflict() {
        let mut from_manifest = HashMap::new();
        from_manifest.insert(
            "auth".to_string(),
            ServiceScenarioOverride {
                chaos_level: Some(0.1),
                ..Default::default()
            },
        );
        from_manifest.insert(
            "payments".to_string(),
            ServiceScenarioOverride {
                chaos_level: Some(0.2),
                ..Default::default()
            },
        );

        let mut from_request = HashMap::new();
        from_request.insert(
            "auth".to_string(),
            ServiceScenarioOverride {
                chaos_level: Some(0.9),
                ..Default::default()
            },
        );

        let merged = merge_overrides(&from_manifest, &from_request);
        assert_eq!(merged["auth"].chaos_level, Some(0.9));
        assert_eq!(merged["payments"].chaos_level, Some(0.2));
    }

    #[test]
    fn validate_activation_overrides_rejects_unknown_service() {
        let services = vec![boundary("auth", "/auth")];
        let mut overrides = HashMap::new();
        overrides.insert("missing".to_string(), ServiceScenarioOverride::default());

        let err = validate_activation_overrides(&services, &overrides).unwrap_err().to_string();
        assert!(err.contains("unknown service"), "got: {err}");
    }

    #[test]
    fn validate_activation_overrides_rejects_out_of_range_chaos_level() {
        let services = vec![boundary("auth", "/auth")];
        let mut overrides = HashMap::new();
        overrides.insert(
            "auth".to_string(),
            ServiceScenarioOverride {
                chaos_level: Some(2.5),
                ..Default::default()
            },
        );

        let err = validate_activation_overrides(&services, &overrides).unwrap_err().to_string();
        assert!(err.contains("chaos_level"), "got: {err}");
    }

    #[test]
    fn validate_activation_overrides_rejects_bad_reality_level() {
        let services = vec![boundary("auth", "/auth")];
        let mut overrides = HashMap::new();
        overrides.insert(
            "auth".to_string(),
            ServiceScenarioOverride {
                reality_level: Some("bogus".to_string()),
                ..Default::default()
            },
        );

        let err = validate_activation_overrides(&services, &overrides).unwrap_err().to_string();
        assert!(err.contains("reality_level"), "got: {err}");
    }

    #[test]
    fn validate_activation_overrides_accepts_valid_payload() {
        let services = vec![boundary("auth", "/auth"), boundary("payments", "/payments")];
        let mut overrides = HashMap::new();
        overrides.insert(
            "payments".to_string(),
            ServiceScenarioOverride {
                reality_level: Some("chaos_driven".to_string()),
                chaos_level: Some(0.7),
                failure_rate: Some(0.2),
                latency_ms: Some(150),
                ..Default::default()
            },
        );
        validate_activation_overrides(&services, &overrides).unwrap();
    }

    #[test]
    fn build_initial_per_service_state_seeds_pending_for_every_service() {
        let services = vec![
            boundary("a", "/a"),
            boundary("b", "/b"),
            boundary("c", "/c"),
        ];
        let state = build_initial_per_service_state(&services);
        assert_eq!(state.len(), 3);
        assert!(state.iter().all(|s| s.status == "pending"));
        assert!(state.iter().all(|s| s.error.is_none()));
        assert!(state.iter().all(|s| s.last_observed_at.is_none()));
    }
}
