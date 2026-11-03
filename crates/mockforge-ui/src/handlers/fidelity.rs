//! Stateless, authenticated mock-definition and recorded-sample comparisons.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mockforge_contracts::fidelity::{compare_mock_spec_with_budget, FidelityReport};
use mockforge_openapi::spec::OpenApiSpec;
use mockforge_recorder::models::RecordedExchange;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod async_samples;
mod sample;

static COMPARISONS: once_cell::sync::Lazy<std::sync::Arc<tokio::sync::Semaphore>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(tokio::sync::Semaphore::new(2)));

/// Inputs are document contents, never file paths or upstream URLs.
#[derive(Deserialize)]
pub struct CompareRequest {
    /// OpenAPI or AsyncAPI definition used to generate the mock.
    pub mock_spec: String,
    /// Current source OpenAPI or AsyncAPI definition.
    pub source_spec: String,
    /// Optional recorder JSON export; an explicitly empty sample fails.
    pub sample: Option<Vec<RecordedExchange>>,
}

#[derive(Serialize)]
struct Assessment {
    definition: FidelityReport,
    sample: Option<Value>,
}

/// Compare on the blocking pool so schema work cannot stall the async server.
pub async fn compare(
    payload: Result<Json<CompareRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(rejection) => return (rejection.status(), Json(json!({"error":"Check the definition fields and use a Flight Recorder export containing HTTP exchanges or asynchronous message events"}))).into_response(),
    };
    let permit = match COMPARISONS.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({"error":"Comparisons are busy. Try again shortly."})),
            )
                .into_response()
        }
    };
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        assess(payload)
    })
    .await
    {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(message)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"error":message}))).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"Fidelity comparison failed; try again"})),
        )
            .into_response(),
    }
}

fn parse_document(content: &str) -> Result<Value, String> {
    if content.len() > 2 * 1024 * 1024 {
        return Err("Each definition must be at most 2 MiB".into());
    }
    let mut value: Value = serde_json::from_str(content)
        .or_else(|_| serde_yaml::from_str(content))
        .map_err(|e| format!("Invalid JSON or YAML definition: {e}"))?;
    if value.get("swagger").and_then(Value::as_str) == Some("2.0") {
        value = mockforge_import::import::swagger2_convert::convert_swagger2_to_openapi3(&value)?;
    }
    if value.get("asyncapi").is_some() {
        return Ok(value);
    }
    let spec =
        OpenApiSpec::from_json(value).map_err(|e| format!("Invalid OpenAPI 3 definition: {e}"))?;
    serde_json::to_value(&spec.spec).map_err(|e| e.to_string())
}

fn assess(payload: CompareRequest) -> Result<Assessment, String> {
    let mock = parse_document(&payload.mock_spec)?;
    let source = parse_document(&payload.source_spec)?;
    let definition = compare_mock_spec_with_budget(&mock, &source, 16 * 1024 * 1024)?;
    let sample = payload
        .sample
        .map(|records| {
            if source.get("asyncapi").is_some() {
                serde_json::to_value(async_samples::verify(&source, records)?)
                    .map_err(|e| e.to_string())
            } else {
                let parsed = OpenApiSpec::from_json(source.clone()).map_err(|e| e.to_string())?;
                serde_json::to_value(sample::verify(parsed, &source, records)?)
                    .map_err(|e| e.to_string())
            }
        })
        .transpose()?;
    Ok(Assessment { definition, sample })
}

#[cfg(test)]
mod tests;
