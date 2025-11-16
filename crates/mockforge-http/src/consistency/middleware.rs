//! Consistency middleware for HTTP
//!
//! This middleware ensures HTTP requests/responses use the unified state
//! from the consistency engine (persona, scenario, reality level, etc.)

use crate::consistency::HttpAdapter;
use axum::{
    body::Body,
    extract::Request,
    http::Response,
    middleware::Next,
};
use mockforge_core::consistency::ConsistencyEngine;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Consistency middleware state
#[derive(Clone)]
pub struct ConsistencyMiddlewareState {
    /// Consistency engine
    pub engine: Arc<ConsistencyEngine>,
    /// HTTP adapter
    pub adapter: Arc<HttpAdapter>,
}

/// Consistency middleware
///
/// This middleware:
/// 1. Extracts workspace ID from request (header, query param, or default)
/// 2. Gets unified state from consistency engine
/// 3. Inserts state into request extensions for handlers to use
/// 4. Ensures responses reflect the unified state
pub async fn consistency_middleware(
    req: Request,
    next: Next,
) -> Response<Body> {
    // Extract workspace ID from request
    // Priority: X-MockForge-Workspace header > query param > default
    let workspace_id = req
        .headers()
        .get("X-MockForge-Workspace")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            req.uri()
                .query()
                .and_then(|q| {
                    q.split('&')
                        .find_map(|pair| {
                            let mut parts = pair.splitn(2, '=');
                            if parts.next() == Some("workspace") {
                                parts.next().and_then(|v| {
                                    urlencoding::decode(v).ok().map(|decoded| decoded.to_string())
                                })
                            } else {
                                None
                            }
                        })
                })
        })
        .unwrap_or_else(|| "default".to_string());

    // Get state from extensions (set by router)
    let state = req.extensions().get::<ConsistencyMiddlewareState>();

    if let Some(state) = state {
        // Get unified state for workspace
        if let Some(unified_state) = state.engine.get_state(&workspace_id).await {
            // Extract values for headers before moving unified_state
            let persona_id = unified_state.active_persona.as_ref().map(|p| p.id.clone());
            let scenario_id = unified_state.active_scenario.clone();
            let reality_level = unified_state.reality_level.value();
            let reality_ratio = unified_state.reality_continuum_ratio;
            let chaos_rules: Vec<String> = unified_state
                .active_chaos_rules
                .iter()
                .map(|r| r.name.clone())
                .collect();
            let request_id = uuid::Uuid::new_v4().to_string();

            // Insert unified state into request extensions for handlers
            let mut req = req;
            req.extensions_mut().insert(unified_state);

            // Continue with request processing
            let mut response = next.run(req).await;

            // Add X-Ray headers to response for browser extension
            response.headers_mut().insert(
                "X-MockForge-Workspace",
                workspace_id.parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-MockForge-Request-ID",
                request_id.parse().unwrap(),
            );
            if let Some(ref persona_id) = persona_id {
                response.headers_mut().insert(
                    "X-MockForge-Persona",
                    persona_id.parse().unwrap(),
                );
            }
            if let Some(ref scenario_id) = scenario_id {
                response.headers_mut().insert(
                    "X-MockForge-Scenario",
                    scenario_id.parse().unwrap(),
                );
            }
            response.headers_mut().insert(
                "X-MockForge-Reality-Level",
                reality_level.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-MockForge-Reality-Ratio",
                reality_ratio.to_string().parse().unwrap(),
            );
            if !chaos_rules.is_empty() {
                response.headers_mut().insert(
                    "X-MockForge-Chaos-Rules",
                    chaos_rules.join(",").parse().unwrap(),
                );
            }

            return response;
        } else {
            debug!("No unified state found for workspace {}", workspace_id);
        }
    }

    // Continue without unified state if not available
    next.run(req).await
}

