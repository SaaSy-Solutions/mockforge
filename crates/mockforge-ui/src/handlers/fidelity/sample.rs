//! Check supplied HTTP recorder exports without recording offline errors as live traffic.

use base64::Engine;
use mockforge_contracts::http_route_match::match_template;
use mockforge_openapi::{openapi_routes::OpenApiRouteRegistry, spec::OpenApiSpec};
use mockforge_recorder::models::{Protocol, RecordedExchange};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

mod request;
mod response;

#[derive(Serialize)]
pub(super) struct SampleReport {
    exchanges_checked: usize,
    latest_capture_at: String,
    pub routes: Vec<SampleRoute>,
}

#[derive(Serialize)]
pub(super) struct SampleRoute {
    endpoint: String,
    status: &'static str,
    exchanges_checked: usize,
    findings: Vec<String>,
    notes: Vec<String>,
}

pub(super) fn verify(
    source: OpenApiSpec,
    spec: &Value,
    records: Vec<RecordedExchange>,
) -> Result<SampleReport, String> {
    if records.len() > 500 {
        return Err("A sample must contain at most 500 exchanges".into());
    }
    let records: Vec<_> =
        records.into_iter().filter(|r| r.request.protocol == Protocol::Http).collect();
    let latest_capture_at = records
        .iter()
        .map(|r| r.request.timestamp)
        .max()
        .ok_or("The sample contains no HTTP exchanges")?
        .to_rfc3339();
    let registry = OpenApiRouteRegistry::new_for_contract_verification(source);
    let mut routes: BTreeMap<_, _> = registry
        .routes()
        .iter()
        .map(|r| {
            let endpoint = format!("{} {}", r.method.to_uppercase(), r.path);
            (endpoint.clone(), new_route(endpoint))
        })
        .collect();
    let mut detail_bytes = 0usize;
    for record in &records {
        let method = record.request.method.to_uppercase();
        let path = record.request.path.split('?').next().unwrap_or(&record.request.path);
        let matched = match_template(&registry, &method, path);
        let endpoint =
            format!("{method} {}", matched.as_ref().map(|v| v.0.as_str()).unwrap_or(path));
        let result = routes.entry(endpoint.clone()).or_insert_with(|| new_route(endpoint));
        let previous_details = result.findings.len();
        let previous_notes = result.notes.len();
        result.exchanges_checked += 1;
        if let Some((template, params)) = matched {
            let (findings, notes) = request::check(&registry, record, &method, &template, params)?;
            result.findings.extend(findings);
            result.notes.extend(notes);
            let (findings, notes) = response::check(&registry, spec, record, &method, &template)?;
            result.findings.extend(findings);
            result.notes.extend(notes);
        } else {
            result
                .findings
                .push("Recorded endpoint is absent from the source specification".into());
        }
        detail_bytes += result.findings[previous_details..]
            .iter()
            .chain(result.notes[previous_notes..].iter())
            .map(String::len)
            .sum::<usize>();
        if detail_bytes > 1024 * 1024 {
            return Err(
                "Sample comparison produced too many details; export a smaller sample".into()
            );
        }
        result.status = if !result.findings.is_empty() {
            "drift-detected"
        } else if !result.notes.is_empty() {
            "not-verified"
        } else {
            "verified"
        };
    }
    Ok(SampleReport {
        exchanges_checked: records.len(),
        latest_capture_at,
        routes: routes.into_values().collect(),
    })
}

fn new_route(endpoint: String) -> SampleRoute {
    SampleRoute {
        endpoint,
        status: "not-sampled",
        exchanges_checked: 0,
        findings: Vec::new(),
        notes: Vec::new(),
    }
}

fn decode(body: &Option<String>, encoding: &str) -> Result<Option<Vec<u8>>, String> {
    body.as_ref()
        .map(|body| match encoding {
            "base64" => base64::engine::general_purpose::STANDARD
                .decode(body)
                .map_err(|_| "Invalid base64 recorded body".into()),
            "utf8" | "utf-8" => Ok(body.as_bytes().to_vec()),
            _ => Err("Unsupported recorded body encoding".into()),
        })
        .transpose()
}
