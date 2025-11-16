//! Drift tracking middleware
//!
//! This middleware integrates drift budget evaluation and consumer usage tracking
//! with contract diff analysis.

use axum::{
    body::Body,
    extract::{Request, State},
    http::Response,
    middleware::Next,
};
use mockforge_core::{
    ai_contract_diff::ContractDiffAnalyzer,
    contract_drift::{DriftBudgetEngine, DriftResult},
    consumer_contracts::{ConsumerBreakingChangeDetector, UsageRecorder},
    incidents::{IncidentManager, IncidentSeverity, IncidentType},
    openapi::OpenApiSpec,
};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

/// State for drift tracking middleware
#[derive(Clone)]
pub struct DriftTrackingState {
    /// Contract diff analyzer
    pub diff_analyzer: Option<Arc<ContractDiffAnalyzer>>,
    /// OpenAPI spec (if available)
    pub spec: Option<Arc<OpenApiSpec>>,
    /// Drift budget engine
    pub drift_engine: Arc<DriftBudgetEngine>,
    /// Incident manager
    pub incident_manager: Arc<IncidentManager>,
    /// Usage recorder for consumer contracts
    pub usage_recorder: Arc<UsageRecorder>,
    /// Consumer breaking change detector
    pub consumer_detector: Arc<ConsumerBreakingChangeDetector>,
    /// Whether drift tracking is enabled
    pub enabled: bool,
}

/// Middleware to track drift and consumer usage (with state from extensions)
///
/// This middleware requires response body buffering middleware to be applied first.
/// The response body is extracted from buffered response extensions.
pub async fn drift_tracking_middleware_with_extensions(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Extract state from request extensions
    let state = req.extensions().get::<DriftTrackingState>().cloned();

    let state = if let Some(state) = state {
        state
    } else {
        // No state available, skip drift tracking
        return next.run(req).await;
    };

    if !state.enabled {
        return next.run(req).await;
    }

    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract consumer identifier from request
    let consumer_id = extract_consumer_id(&req);

    // Process request and get response
    let response = next.run(req).await;

    // Extract response body for consumer usage tracking
    let response_body = extract_response_body(&response);

    // Record consumer usage if consumer is identified
    if let Some(ref consumer_id) = consumer_id {
        if let Some(body) = &response_body {
            state
                .usage_recorder
                .record_usage(consumer_id, &path, &method, Some(body))
                .await;
        }
    }

    // Perform contract diff analysis if analyzer and spec are available
    if let (Some(ref analyzer), Some(ref spec)) = (&state.diff_analyzer, &state.spec) {
        // Create captured request from the actual request
        // Note: In a full implementation, we'd need to capture the request body
        // For now, we'll analyze based on path and method
        let captured = mockforge_core::ai_contract_diff::CapturedRequest::new(&method, &path, "drift_tracking")
            .with_response(response.status().as_u16(), response_body.clone());

        // Analyze request against contract
        match analyzer.analyze(&captured, spec).await {
            Ok(diff_result) => {
                // Evaluate drift budget
                let drift_result = state.drift_engine.evaluate(&diff_result, &path, &method);

                // Create incident if budget is exceeded or breaking changes detected
                if drift_result.should_create_incident {
                    let incident_type = if drift_result.breaking_changes > 0 {
                        IncidentType::BreakingChange
                    } else {
                        IncidentType::ThresholdExceeded
                    };

                    let severity = determine_severity(&drift_result);

                    let details = serde_json::json!({
                        "breaking_changes": drift_result.breaking_changes,
                        "non_breaking_changes": drift_result.non_breaking_changes,
                        "breaking_mismatches": drift_result.breaking_mismatches,
                        "non_breaking_mismatches": drift_result.non_breaking_mismatches,
                        "budget_exceeded": drift_result.budget_exceeded,
                    });

                    // Create incident
                    let _incident = state
                        .incident_manager
                        .create_incident(
                            path.clone(),
                            method.clone(),
                            incident_type,
                            severity,
                            details,
                            None, // budget_id
                            None, // workspace_id
                        )
                        .await;

                    warn!(
                        "Drift incident created: {} {} - {} breaking changes, {} non-breaking changes",
                        method, path, drift_result.breaking_changes, drift_result.non_breaking_changes
                    );
                }

                // Check for consumer-specific violations
                if let Some(ref consumer_id) = consumer_id {
                    let violations = state
                        .consumer_detector
                        .detect_violations(consumer_id, &path, &method, &diff_result, None)
                        .await;

                    if !violations.is_empty() {
                        warn!(
                            "Consumer {} has {} violations on {} {}",
                            consumer_id,
                            violations.len(),
                            method,
                            path
                        );
                    }
                }
            }
            Err(e) => {
                debug!("Contract diff analysis failed: {}", e);
            }
        }
    }

    response
}

/// Extract consumer identifier from request
fn extract_consumer_id(req: &Request<Body>) -> Option<String> {
    // Try to extract from various sources:
    // 1. X-Consumer-ID header
    if let Some(consumer_id) = req
        .headers()
        .get("x-consumer-id")
        .and_then(|h| h.to_str().ok())
    {
        return Some(consumer_id.to_string());
    }

    // 2. X-Workspace-ID header (for workspace-based consumers)
    if let Some(workspace_id) = req
        .headers()
        .get("x-workspace-id")
        .and_then(|h| h.to_str().ok())
    {
        return Some(format!("workspace:{}", workspace_id));
    }

    // 3. API key from header
    if let Some(api_key) = req
        .headers()
        .get("x-api-key")
        .or_else(|| req.headers().get("authorization"))
        .and_then(|h| h.to_str().ok())
    {
        // Hash the API key for privacy
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        return Some(format!("api_key:{}", hash));
    }

    None
}

/// Extract response body as JSON value
fn extract_response_body(response: &Response<Body>) -> Option<Value> {
    // Try to get buffered response from extensions
    if let Some(buffered) = crate::middleware::get_buffered_response(response) {
        return buffered.json();
    }

    // If not buffered, try to parse from response body
    // Note: This requires the response body to be buffered by upstream middleware
    None
}

/// Determine incident severity from drift result
fn determine_severity(drift_result: &DriftResult) -> IncidentSeverity {
    if drift_result.breaking_changes > 0 {
        // Check if any breaking mismatch is critical
        if drift_result
            .breaking_mismatches
            .iter()
            .any(|m| m.severity == mockforge_core::ai_contract_diff::MismatchSeverity::Critical)
        {
            return IncidentSeverity::Critical;
        }
        // Check if any breaking mismatch is high
        if drift_result
            .breaking_mismatches
            .iter()
            .any(|m| m.severity == mockforge_core::ai_contract_diff::MismatchSeverity::High)
        {
            return IncidentSeverity::High;
        }
        return IncidentSeverity::Medium;
    }

    // Non-breaking changes are lower severity
    if drift_result.non_breaking_changes > 5 {
        IncidentSeverity::Medium
    } else {
        IncidentSeverity::Low
    }
}
