//! Consistency / Virtual Backends handlers (#461) — Phase 1 surface.
//!
//! The Virtual Backends UI surfaces three concerns:
//!   1. Lifecycle presets — predefined entity state machines (Subscription /
//!      Loan / Order Fulfillment / User Engagement). Hardcoded the same as
//!      `mockforge_data::persona_lifecycle::LifecyclePreset` so cloud users
//!      see the same library local users do.
//!   2. Virtual entities — workspace-scoped persisted rows that get written
//!      when a preset is applied to a persona. Empty for new workspaces.
//!   3. Snapshots — already cloud-enabled via `handlers::snapshots`, the UI
//!      reuses `cloudSnapshotsApi` from the existing `cloud-snapshots` page.
//!
//! Phase 1 ships preset read endpoints + the entities list (empty for new
//! workspaces) + an `apply` endpoint that materializes a virtual_entity row
//! for the persona.
//!
//! Automatic state transitions (the local engine's job) are not part of
//! cloud yet — the row reflects the applied initial state and stays there
//! until the user manually advances it. That's enough to make the UI live
//! for cloud users while we figure out where the transition driver belongs
//! server-side.
//!
//! Routes:
//!   GET    /api/v1/consistency/lifecycle-presets
//!   GET    /api/v1/consistency/lifecycle-presets/{preset_id}
//!   POST   /api/v1/workspaces/{workspace_id}/consistency/lifecycle-presets/apply
//!   GET    /api/v1/workspaces/{workspace_id}/consistency/entities[?entity_type=&persona_id=]
//!   GET    /api/v1/consistency/entities/{id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudWorkspace,
    AppState,
};

#[derive(Debug, Serialize)]
pub struct LifecyclePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub initial_state: &'static str,
    pub states: Vec<&'static str>,
    pub affected_endpoints: Vec<&'static str>,
}

const PRESETS: &[LifecyclePresetStatic] = &[
    LifecyclePresetStatic {
        id: "subscription",
        name: "Subscription",
        description: "Subscription lifecycle: NEW → ACTIVE → PAST_DUE → CANCELED",
        initial_state: "new",
        states: &["new", "active", "past_due", "canceled"],
        affected_endpoints: &["billing", "support", "subscription"],
    },
    LifecyclePresetStatic {
        id: "loan",
        name: "Loan",
        description: "Loan lifecycle: APPLICATION → APPROVED → ACTIVE → PAST_DUE → DEFAULTED",
        initial_state: "application",
        states: &["application", "approved", "active", "past_due", "defaulted"],
        affected_endpoints: &["loan", "loans", "credit", "application"],
    },
    LifecyclePresetStatic {
        id: "order_fulfillment",
        name: "Order Fulfillment",
        description:
            "Order fulfillment lifecycle: PENDING → PROCESSING → SHIPPED → DELIVERED → COMPLETED",
        initial_state: "pending",
        states: &["pending", "processing", "shipped", "delivered", "completed"],
        affected_endpoints: &["order", "orders", "fulfillment", "shipment", "delivery"],
    },
    LifecyclePresetStatic {
        id: "user_engagement",
        name: "User Engagement",
        description: "User engagement lifecycle: NEW → ACTIVE → CHURN_RISK → CHURNED",
        initial_state: "new",
        states: &["new", "active", "churn_risk", "churned"],
        affected_endpoints: &[
            "profile",
            "user",
            "users",
            "activity",
            "engagement",
            "notifications",
        ],
    },
];

/// Static-array variant — what we actually iterate. Public `LifecyclePreset`
/// is the Serialize-friendly view we hand to the UI.
struct LifecyclePresetStatic {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    initial_state: &'static str,
    states: &'static [&'static str],
    affected_endpoints: &'static [&'static str],
}

impl LifecyclePresetStatic {
    fn as_view(&self) -> LifecyclePreset {
        LifecyclePreset {
            id: self.id,
            name: self.name,
            description: self.description,
            initial_state: self.initial_state,
            states: self.states.to_vec(),
            affected_endpoints: self.affected_endpoints.to_vec(),
        }
    }
}

fn lookup_preset(id: &str) -> Option<&'static LifecyclePresetStatic> {
    let id_norm = id.to_lowercase().replace('-', "_");
    PRESETS.iter().find(|p| p.id == id_norm)
}

// --- preset read endpoints ------------------------------------------------

/// `GET /api/v1/consistency/lifecycle-presets`
pub async fn list_lifecycle_presets() -> Json<Vec<LifecyclePreset>> {
    Json(PRESETS.iter().map(LifecyclePresetStatic::as_view).collect())
}

/// `GET /api/v1/consistency/lifecycle-presets/{preset_id}`
pub async fn get_lifecycle_preset(
    Path(preset_id): Path<String>,
) -> ApiResult<Json<LifecyclePreset>> {
    let preset = lookup_preset(&preset_id).ok_or_else(|| {
        ApiError::InvalidRequest(format!("Unknown lifecycle preset '{preset_id}'"))
    })?;
    Ok(Json(preset.as_view()))
}

// --- virtual entities -----------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct VirtualEntity {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub persona_id: Option<String>,
    pub current_state: Option<String>,
    pub data: serde_json::Value,
    pub seen_in_protocols: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListEntitiesQuery {
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub persona_id: Option<String>,
}

/// `GET /api/v1/workspaces/{workspace_id}/consistency/entities`
pub async fn list_workspace_entities(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<ListEntitiesQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<VirtualEntity>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let rows: Vec<VirtualEntity> = sqlx::query_as::<_, VirtualEntity>(
        r#"
        SELECT id, workspace_id, entity_type, entity_id, persona_id,
               current_state, data, seen_in_protocols, created_at, updated_at
        FROM virtual_entities
        WHERE workspace_id = $1
          AND ($2::text IS NULL OR entity_type = $2)
          AND ($3::text IS NULL OR persona_id = $3)
        ORDER BY updated_at DESC
        LIMIT 500
        "#,
    )
    .bind(workspace_id)
    .bind(query.entity_type)
    .bind(query.persona_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// `GET /api/v1/consistency/entities/{id}`
pub async fn get_entity(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<VirtualEntity>> {
    let row: Option<VirtualEntity> = sqlx::query_as::<_, VirtualEntity>(
        r#"
        SELECT id, workspace_id, entity_type, entity_id, persona_id,
               current_state, data, seen_in_protocols, created_at, updated_at
        FROM virtual_entities
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(ApiError::Database)?;
    let entity = row.ok_or_else(|| ApiError::InvalidRequest("Entity not found".into()))?;
    authorize_workspace(&state, user_id, &headers, entity.workspace_id).await?;
    Ok(Json(entity))
}

// --- apply preset ---------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ApplyPresetRequest {
    /// Preset id (case-insensitive, `-` and `_` interchangeable).
    pub preset: String,
    /// Persona id the lifecycle is anchored to. Free-form string mirroring
    /// the local engine's `persona_id`.
    pub persona_id: String,
    /// Logical entity type for the row. Defaults to the preset id when
    /// omitted (e.g. applying `subscription` produces a row of type
    /// `subscription`).
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Stable entity key. Defaults to `{persona_id}:{entity_type}` so
    /// re-applying the same preset to the same persona is idempotent and
    /// resets state instead of duplicating rows.
    #[serde(default)]
    pub entity_id: Option<String>,
}

/// `POST /api/v1/workspaces/{workspace_id}/consistency/lifecycle-presets/apply`
///
/// Materializes the preset's initial state as a `virtual_entities` row.
/// Idempotent on `(workspace_id, entity_type, entity_id)` — re-applying
/// resets `current_state` and bumps `updated_at`. The actual time-based
/// transitions the local engine runs aren't modeled here; cloud-side
/// transition driving is a follow-up once we decide where it belongs.
pub async fn apply_lifecycle_preset(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ApplyPresetRequest>,
) -> ApiResult<Json<VirtualEntity>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let preset = lookup_preset(&req.preset).ok_or_else(|| {
        ApiError::InvalidRequest(format!(
            "Unknown lifecycle preset '{}'. Try one of: {}",
            req.preset,
            PRESETS.iter().map(|p| p.id).collect::<Vec<_>>().join(", ")
        ))
    })?;

    if req.persona_id.trim().is_empty() {
        return Err(ApiError::InvalidRequest("persona_id must not be empty".into()));
    }

    let entity_type = req
        .entity_type
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| preset.id.to_string());
    let entity_id = req
        .entity_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("{}:{}", req.persona_id, entity_type));

    let row: VirtualEntity = sqlx::query_as::<_, VirtualEntity>(
        r#"
        INSERT INTO virtual_entities
            (workspace_id, entity_type, entity_id, persona_id, current_state,
             data, seen_in_protocols)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (workspace_id, entity_type, entity_id) DO UPDATE
            SET persona_id    = EXCLUDED.persona_id,
                current_state = EXCLUDED.current_state,
                data          = EXCLUDED.data,
                updated_at    = NOW()
        RETURNING id, workspace_id, entity_type, entity_id, persona_id,
                  current_state, data, seen_in_protocols, created_at, updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(&entity_type)
    .bind(&entity_id)
    .bind(&req.persona_id)
    .bind(preset.initial_state)
    .bind(serde_json::json!({
        "preset_id": preset.id,
        "preset_name": preset.name,
        "applied_at": Utc::now(),
    }))
    .bind(serde_json::json!([]))
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_preset_case_and_separator_insensitive() {
        assert!(lookup_preset("subscription").is_some());
        assert!(lookup_preset("Subscription").is_some());
        assert!(lookup_preset("SUBSCRIPTION").is_some());
        assert!(lookup_preset("order_fulfillment").is_some());
        assert!(lookup_preset("order-fulfillment").is_some());
        assert!(lookup_preset("OrderFulfillment").is_none()); // case-changes only; we don't camel-split
        assert!(lookup_preset("not_a_preset").is_none());
    }

    #[test]
    fn all_four_presets_are_present() {
        let names: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        assert_eq!(
            names,
            vec![
                "subscription",
                "loan",
                "order_fulfillment",
                "user_engagement"
            ]
        );
    }

    #[test]
    fn presets_have_nonempty_state_machines() {
        for p in PRESETS {
            assert!(!p.states.is_empty(), "{} has no states", p.id);
            assert!(
                p.states.contains(&p.initial_state),
                "{} initial_state {} not in states {:?}",
                p.id,
                p.initial_state,
                p.states
            );
        }
    }
}
