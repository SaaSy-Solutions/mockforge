//! Response generation trace instrumentation
//!
//! This module provides helpers to instrument response generation
//! and collect trace data for debugging and observability.

use crate::openapi::response::ResponseGenerator;
use crate::openapi::OpenApiSpec;
use crate::{Result};
use crate::reality_continuum::response_trace::ResponseGenerationTrace;
use crate::openapi::response_selection::ResponseSelectionMode;
use openapiv3::Operation;
use serde_json::Value;
use std::time::Instant;

/// Generate a response with trace collection
///
/// This function wraps the response generation and collects trace data
/// about how the response was generated.
///
/// # Arguments
/// * `spec` - The OpenAPI specification
/// * `operation` - The operation to generate a response for
/// * `status_code` - The HTTP status code
/// * `content_type` - Optional content type
/// * `expand_tokens` - Whether to expand template tokens
/// * `scenario` - Optional scenario name
/// * `selection_mode` - Response selection mode
/// * `selector` - Optional response selector
/// * `persona` - Optional persona for data generation
///
/// # Returns
/// A tuple of (response_value, trace_data)
pub fn generate_response_with_trace(
    spec: &OpenApiSpec,
    operation: &Operation,
    status_code: u16,
    content_type: Option<&str>,
    expand_tokens: bool,
    scenario: Option<&str>,
    selection_mode: Option<ResponseSelectionMode>,
    selector: Option<&crate::openapi::response_selection::ResponseSelector>,
    persona: Option<&crate::intelligent_behavior::config::Persona>,
) -> Result<(Value, ResponseGenerationTrace)> {
    let start_time = Instant::now();
    let mut trace = ResponseGenerationTrace::new();

    // Record selection mode
    let actual_mode = selection_mode.unwrap_or(ResponseSelectionMode::First);
    trace.response_selection_mode = actual_mode;

    // Record scenario if provided
    if let Some(scenario_name) = scenario {
        trace.selected_example = Some(scenario_name.to_string());
    }

    // Record template expansion setting
    trace.add_metadata(
        "expand_tokens".to_string(),
        serde_json::json!(expand_tokens),
    );

    // Record persona if provided
    if let Some(p) = persona {
        trace.add_metadata(
            "persona_name".to_string(),
            serde_json::json!(p.name),
        );
    }

    // Generate the response
    let response = ResponseGenerator::generate_response_with_scenario_and_mode_and_persona(
        spec,
        operation,
        status_code,
        content_type,
        expand_tokens,
        scenario,
        selection_mode,
        selector,
        persona,
    )?;

    // Record final payload
    trace.set_final_payload(response.clone());

    // Record generation time
    let generation_time_ms = start_time.elapsed().as_millis() as u64;
    trace.add_metadata(
        "generation_time_ms".to_string(),
        serde_json::json!(generation_time_ms),
    );

    // Record operation ID if available
    if let Some(operation_id) = &operation.operation_id {
        trace.add_metadata(
            "operation_id".to_string(),
            serde_json::json!(operation_id),
        );
    }

    // Record status code
    trace.add_metadata(
        "status_code".to_string(),
        serde_json::json!(status_code),
    );

    // Try to determine which example was selected
    // This is a simplified version - full implementation would need to
    // instrument the actual selection logic
    if let Some(sel) = selector {
        if actual_mode == ResponseSelectionMode::Sequential {
            let seq_index = sel.get_sequential_index();
            trace.add_metadata(
                "sequential_index".to_string(),
                serde_json::json!(seq_index),
            );
        }
    }

    // Record content type
    if let Some(ct) = content_type {
        trace.add_metadata(
            "content_type".to_string(),
            serde_json::json!(ct),
        );
    }

    Ok((response, trace))
}
