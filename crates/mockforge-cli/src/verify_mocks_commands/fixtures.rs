//! Verify a complete HTTP replay fixture set against its source specification.

use super::{match_template, responses, OpenApiRouteRegistry, Value};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

mod files;

#[derive(Debug, Serialize)]
pub(super) struct FixtureFinding {
    pub file: Option<String>,
    pub kind: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<responses::ResponseDrift>,
}

#[derive(Debug, Serialize)]
pub(super) struct FixtureRoute {
    pub endpoint: String,
    pub status: String,
    pub change: String,
    pub files: Vec<String>,
    pub findings: Vec<FixtureFinding>,
}

#[derive(Debug, Serialize)]
pub(super) struct FixtureReport {
    pub files_checked: usize,
    /// Replay fixtures contain no request bodies; this mode checks responses.
    pub validation_scope: &'static str,
    pub routes: Vec<FixtureRoute>,
}

pub(super) fn verify(
    path: &Path,
    registry: &OpenApiRouteRegistry,
    spec: &Value,
) -> mockforge_core::Result<FixtureReport> {
    let fixtures = files::load(path)?;
    let mut routes: BTreeMap<_, _> = registry
        .routes()
        .iter()
        .map(|route| {
            let endpoint = format!("{} {}", route.method.to_uppercase(), route.path);
            (
                endpoint.clone(),
                FixtureRoute {
                    endpoint,
                    status: "verified".into(),
                    change: "unchanged".into(),
                    files: Vec::new(),
                    findings: Vec::new(),
                },
            )
        })
        .collect();
    for (file, fixture) in &fixtures {
        let method = fixture.fingerprint.method.to_uppercase();
        let path = super::strip_query(&fixture.fingerprint.path);
        let matched = match_template(registry, &method, &path);
        let endpoint =
            format!("{method} {}", matched.as_ref().map(|m| m.0.as_str()).unwrap_or(&path));
        let route = routes.entry(endpoint.clone()).or_insert_with(|| FixtureRoute {
            endpoint,
            status: "drift-detected".into(),
            change: "removed".into(),
            files: Vec::new(),
            findings: Vec::new(),
        });
        route.files.push(file.clone());
        if let Some((template, _)) = matched {
            if let Some(response) = responses::check_response_drift(
                registry,
                spec,
                &method,
                &template,
                fixture.status_code,
                Some(fixture.response_body.as_bytes()),
            ) {
                route.findings.push(FixtureFinding {
                    file: Some(file.clone()),
                    kind: "response".into(),
                    detail: response.detail.clone(),
                    response: Some(response),
                });
            }
        } else {
            route.findings.push(FixtureFinding {
                file: Some(file.clone()),
                kind: "undeclared-endpoint".into(),
                detail: "fixture endpoint is absent from the source specification".into(),
                response: None,
            });
        }
    }
    for route in routes.values_mut() {
        if route.files.is_empty() {
            route.change = "added".into();
            route.findings.push(FixtureFinding {
                file: None,
                kind: "missing-fixture".into(),
                detail: "source operation has no fixture in this set".into(),
                response: None,
            });
        }
        if !route.findings.is_empty() {
            route.status = "drift-detected".into();
            if route.change == "unchanged" {
                route.change = "changed".into();
            }
        }
    }
    Ok(FixtureReport { files_checked: fixtures.len(), validation_scope: "HTTP routes, statuses and JSON response schemas; request bodies are not stored in replay fixtures", routes: routes.into_values().collect() })
}

#[cfg(test)]
mod tests;
