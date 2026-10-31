//! Structural fidelity between a mock definition and its OpenAPI/AsyncAPI source.
//!
//! This is a definition comparison, rather than a compatibility policy: added
//! optional fields and new operations are drift too. No network access occurs.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

pub mod async_sample;
mod asyncapi;
mod context;
mod delta;
mod operations;
mod projection;
mod references;

use delta::diff;
use operations::operations;

/// One change, with a JSON pointer relative to the operation's contract.
#[derive(Debug, Serialize)]
pub struct FieldDelta {
    /// JSON pointer; `/` and `~` in names are escaped per RFC 6901.
    pub location: String,
    /// `added`, `removed`, or `changed`, from mock to source.
    pub change: String,
    /// Value in the mock definition (absent for additions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<Value>,
    /// Value in the source specification (absent for removals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Value>,
}

/// Fidelity for one method and path, including endpoints present on one side only.
#[derive(Debug, Serialize)]
pub struct RouteFidelity {
    /// Uppercase HTTP method and OpenAPI path template.
    pub endpoint: String,
    /// `verified` or `drift-detected`.
    pub status: String,
    /// `unchanged`, `added`, `removed`, or `changed`, from mock to source.
    pub change: String,
    /// Field-level contract differences.
    pub deltas: Vec<FieldDelta>,
}

/// Deterministic report shared by command-line and admin consumers.
#[derive(Debug, Serialize)]
pub struct FidelityReport {
    /// Version of this serialized report format.
    pub format_version: u8,
    /// Number of routes with a changed definition or presence.
    pub routes_with_drift: usize,
    /// All routes, including unchanged routes with verified status.
    pub routes: Vec<RouteFidelity>,
}

/// Compare OpenAPI 3 or AsyncAPI 2/3 definitions. Inputs must be parsed/validated by the caller.
/// Local references are resolved, including recursive schemas; unsupported or
/// missing references and excessive expansion return an error, never a pass.
pub fn compare_mock_spec(mock: &Value, source: &Value) -> Result<FidelityReport, String> {
    compare_mock_spec_with_budget(mock, source, usize::MAX)
}

/// Compare with a per-document expansion byte budget for interactive consumers.
/// Exceeding the budget returns an error instead of a partial fidelity report.
pub fn compare_mock_spec_with_budget(
    mock: &Value,
    source: &Value,
    max_bytes: usize,
) -> Result<FidelityReport, String> {
    let asynchronous = mock.get("asyncapi").is_some();
    if asynchronous != source.get("asyncapi").is_some() {
        return Err("Cannot compare OpenAPI and AsyncAPI definitions".into());
    }
    let (mock_routes, source_routes) = if asynchronous {
        (asyncapi::operations(mock, max_bytes)?, asyncapi::operations(source, max_bytes)?)
    } else {
        (operations(mock, max_bytes)?, operations(source, max_bytes)?)
    };
    if mock_routes.is_empty() && source_routes.is_empty() {
        return Err("neither definition contains an operation or channel to compare".into());
    }
    let endpoints: BTreeSet<_> = mock_routes.keys().chain(source_routes.keys()).collect();
    let mut routes = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        let before = mock_routes.get(endpoint);
        let after = source_routes.get(endpoint);
        let mut deltas = Vec::new();
        diff(before, after, "", &mut deltas);
        let change = match (before, after, deltas.is_empty()) {
            (_, _, true) => "unchanged",
            (None, _, _) => "added",
            (_, None, _) => "removed",
            _ => "changed",
        };
        routes.push(RouteFidelity {
            endpoint: endpoint.clone(),
            status: if deltas.is_empty() {
                "verified"
            } else {
                "drift-detected"
            }
            .into(),
            change: change.into(),
            deltas,
        });
    }
    Ok(FidelityReport {
        format_version: 1,
        routes_with_drift: routes.iter().filter(|r| !r.deltas.is_empty()).count(),
        routes,
    })
}

#[cfg(test)]
mod tests;
