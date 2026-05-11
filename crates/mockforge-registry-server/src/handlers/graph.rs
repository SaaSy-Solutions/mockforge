//! Workspace graph handler (#460) — read-only view of services + flows in a
//! workspace. Returns a `GraphData` payload shaped to match the local
//! `/__mockforge/graph` endpoint so the same `GraphPage` UI can render either
//! mode.
//!
//! Phase 1: nodes only (one per service, one per flow), single workspace
//! cluster, no edges. Edge derivation from flow.config refs is a follow-up.
//!
//! Route: `GET /api/v1/workspaces/{workspace_id}/graph`

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{cloud_service::CloudService, CloudWorkspace, Flow},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub clusters: Vec<GraphCluster>,
}

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(rename = "nodeType")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "edgeType")]
    pub edge_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct GraphCluster {
    pub id: String,
    pub label: String,
    #[serde(rename = "clusterType")]
    pub cluster_type: String,
    #[serde(rename = "nodeIds")]
    pub node_ids: Vec<String>,
    pub metadata: serde_json::Value,
}

pub async fn get_workspace_graph(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<GraphData>> {
    let workspace = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    let services = CloudService::find_by_workspace(state.db.pool(), workspace.org_id, workspace_id)
        .await
        .map_err(ApiError::Database)?;
    let flows = Flow::list_by_workspace(state.db.pool(), workspace_id, None)
        .await
        .map_err(ApiError::Database)?;

    let mut nodes: Vec<GraphNode> = Vec::with_capacity(services.len() + flows.len());
    let mut node_ids: Vec<String> = Vec::with_capacity(services.len() + flows.len());

    for s in &services {
        let id = format!("service:{}", s.id);
        node_ids.push(id.clone());
        nodes.push(GraphNode {
            id,
            label: s.name.clone(),
            node_type: "service".into(),
            protocol: derive_service_protocol(s),
            metadata: serde_json::json!({
                "kind": "service",
                "description": s.description,
                "base_url": s.base_url,
                "enabled": s.enabled,
            }),
        });
    }

    for f in &flows {
        let id = format!("flow:{}", f.id);
        node_ids.push(id.clone());
        nodes.push(GraphNode {
            id,
            label: f.name.clone(),
            node_type: "service".into(),
            protocol: None,
            metadata: serde_json::json!({
                "kind": "flow",
                "flow_kind": f.kind,
                "description": f.description,
            }),
        });
    }

    let clusters = vec![GraphCluster {
        id: format!("workspace:{}", workspace_id),
        label: workspace.name.clone(),
        cluster_type: "workspace".into(),
        node_ids,
        metadata: serde_json::json!({ "workspace_id": workspace_id }),
    }];

    Ok(Json(GraphData {
        nodes,
        edges: Vec::new(),
        clusters,
    }))
}

fn derive_service_protocol(s: &CloudService) -> Option<String> {
    s.routes
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("protocol"))
        .and_then(|p| p.as_str())
        .map(str::to_lowercase)
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(workspace)
}
