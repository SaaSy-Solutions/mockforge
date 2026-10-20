//! `mockforge verify-mocks` (#849) — Mock Fidelity / drift detection.
//!
//! Verifies that a spec-driven mock still matches reality: replays a
//! recorded capture of REAL upstream traffic (recorder sqlite DB) against
//! the loaded OpenAPI spec and reports every exchange that would no
//! longer conform — missing/renamed endpoints, schema drift on request or
//! response bodies, undeclared statuses.
//!
//! Exit codes: 0 = faithful, 1 = drift detected (with `--fail-on-drift`),
//! 2 = usage error.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use mockforge_openapi::spec::OpenApiSpec;

mod fixtures;
use mockforge_contracts::http_route_match::match_template;
use mockforge_contracts::response_validation as responses;
use responses::check_response_drift;

/// Verify mocks still match the real API (drift detection, #849).
///
/// Replays a recorder capture of real upstream traffic against the
/// spec your mocks are generated from and reports drift: endpoints the
/// mock doesn't know, schema violations on recorded bodies, and
/// responses whose status isn't declared.
#[derive(clap::Args, Debug)]
pub(crate) struct VerifyMocksArgs {
    /// Path to the OpenAPI spec the mocks are generated from
    /// (3.x natively; Swagger 2.0 is up-converted automatically).
    #[arg(short, long)]
    pub spec: PathBuf,

    /// Recorder sqlite database holding real captured traffic
    /// (what `mockforge record` writes).
    #[arg(short, long, required_unless_present_any = ["mock_spec", "fixtures"])]
    pub capture_db: Option<PathBuf>,

    /// OpenAPI definition used to generate the mock; compare it with --spec.
    #[arg(long, required_unless_present_any = ["capture_db", "fixtures"])]
    pub mock_spec: Option<PathBuf>,

    /// HTTP replay fixture JSON file or directory; checks every fixture.
    #[arg(long, required_unless_present_any = ["capture_db", "mock_spec"])]
    pub fixtures: Option<PathBuf>,

    /// Maximum recent HTTP exchanges to evaluate (newest first).
    #[arg(long, default_value = "500", value_parser = clap::value_parser!(i32).range(1..))]
    pub limit: i32,

    /// Exit with code 1 when any drift is found (CI gate).
    #[arg(long)]
    pub fail_on_drift: bool,

    /// Write the structured drift report as JSON here.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// One aggregated drift finding for an endpoint.
#[derive(Debug, Clone, Serialize)]
struct DriftFinding {
    endpoint: String,
    kind: String,
    detail: String,
}

/// Human + JSON report.
#[derive(Debug, Serialize)]
struct DriftReport {
    spec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<mockforge_contracts::fidelity::FidelityReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixtures: Option<fixtures::FixtureReport>,
    exchanges_checked: usize,
    endpoints_with_drift: usize,
    unmatched_real_paths: usize,
    findings: Vec<DriftFinding>,
}

pub(crate) async fn handle_verify_mocks_command(
    args: VerifyMocksArgs,
) -> mockforge_core::Result<()> {
    let VerifyMocksArgs {
        spec,
        capture_db,
        mock_spec,
        fixtures,
        limit,
        fail_on_drift,
        output,
    } = args;

    let openapi = load_fidelity_spec(&spec)?;
    let definition = mock_spec
        .as_ref()
        .map(|path| {
            let mock = load_fidelity_spec(path)?;
            let mock = serde_json::to_value(&mock.spec)
                .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
            let source = serde_json::to_value(&openapi.spec)
                .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
            mockforge_contracts::fidelity::compare_mock_spec(&mock, &source)
                .map_err(mockforge_core::Error::config)
        })
        .transpose()?;
    let registry = OpenApiRouteRegistry::new_for_contract_verification(openapi);
    let exchanges = load_fidelity_capture(capture_db.as_ref(), limit).await?;
    let response_spec = serde_json::to_value(&registry.spec().spec).map_err(|e| {
        mockforge_core::Error::config(format!("cannot inspect response contract: {e}"))
    })?;

    let fixture_report = fixtures
        .as_deref()
        .map(|path| fixtures::verify(path, &registry, &response_spec))
        .transpose()?;

    // 3. Evaluate each exchange.
    let mut findings: Vec<DriftFinding> = Vec::new();
    let mut drifted_endpoints: BTreeMap<String, ()> = BTreeMap::new();
    let mut unmatched_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    if let Some(report) = &definition {
        for route in report.routes.iter().filter(|r| !r.deltas.is_empty()) {
            findings.push(DriftFinding {
                endpoint: route.endpoint.clone(),
                kind: "definition".into(),
                detail: format!(
                    "{} endpoint definition; {} field delta(s)",
                    route.change,
                    route.deltas.len()
                ),
            });
            drifted_endpoints.insert(route.endpoint.clone(), ());
        }
    }

    if let Some(report) = &fixture_report {
        for route in report.routes.iter().filter(|r| !r.findings.is_empty()) {
            findings.push(DriftFinding {
                endpoint: route.endpoint.clone(),
                kind: "fixture".into(),
                detail: route
                    .findings
                    .iter()
                    .map(|f| format!("{}: {}", f.kind, f.detail))
                    .collect::<Vec<_>>()
                    .join("; "),
            });
            drifted_endpoints.insert(route.endpoint.clone(), ());
        }
    }

    for exchange in &exchanges {
        let req = &exchange.request;
        let method = req.method.to_uppercase();
        let concrete_path = strip_query(&req.path);
        let Some((template, _path_params)) = match_template(&registry, &method, &concrete_path)
        else {
            unmatched_paths.insert(concrete_path.clone());
            let endpoint = format!("{method} {concrete_path}");
            findings.push(DriftFinding {
                endpoint: endpoint.clone(),
                kind: "undeclared-endpoint".into(),
                detail: "recorded endpoint is absent from the source spec".into(),
            });
            drifted_endpoints.insert(endpoint, ());
            continue;
        };

        // Request-side validation through the same chain `serve` uses.
        // CI checks enforce the contract independently of serve-time policy.
        let decoded_request = decode_body(&req.body, &req.body_encoding);
        let body_bytes: Option<&[u8]> = decoded_request.as_deref();
        if let Err((status, payload)) = registry.run_validation_with_recording_ex(
            &template,
            &method,
            &to_value_map(&extract_path_params(&concrete_path, &template)),
            &query_map(req.query_params.as_deref()),
            &header_value_map(&req.headers),
            &Default::default(),
            body_json(body_bytes).as_ref(),
            body_present(body_bytes),
        ) {
            let endpoint = format!("{method} {concrete_path}");
            findings.push(DriftFinding {
                endpoint: endpoint.clone(),
                kind: "request".into(),
                detail: format!(
                    "real upstream traffic fails the spec (status {status}): {}",
                    payload
                ),
            });
            drifted_endpoints.insert(endpoint, ());
        }

        // Response-side schema drift.
        if let Some(response) = &exchange.response {
            let status = response.status_code as u16;
            let decoded = decode_body(&response.body, &response.body_encoding);
            if let Some(detail) = check_response_drift(
                &registry,
                &response_spec,
                &method,
                &template,
                status,
                decoded.as_deref(),
            ) {
                let endpoint = format!("{method} {concrete_path}");
                findings.push(DriftFinding {
                    endpoint: endpoint.clone(),
                    kind: "response-shape".into(),
                    detail: detail.detail,
                });
                drifted_endpoints.insert(endpoint, ());
            }
        } else {
            let endpoint = format!("{method} {concrete_path}");
            findings.push(DriftFinding {
                endpoint: endpoint.clone(),
                kind: "missing-response".into(),
                detail: "recorded request has no response; response fidelity cannot be verified"
                    .into(),
            });
            drifted_endpoints.insert(endpoint, ());
        }
    }

    let report = DriftReport {
        spec: spec.display().to_string(),
        definition,
        fixtures: fixture_report,
        exchanges_checked: exchanges.len(),
        endpoints_with_drift: drifted_endpoints.len(),
        unmatched_real_paths: unmatched_paths.len(),
        findings,
    };

    // 4. Report.
    println!("\n=== mock fidelity / drift report ===");
    println!("exchanges checked      : {}", report.exchanges_checked);
    if let Some(fixtures) = &report.fixtures {
        println!("fixture files checked  : {}", fixtures.files_checked);
    }
    println!("endpoints with drift   : {}", report.endpoints_with_drift);
    println!(
        "unmatched real paths   : {} (traffic to endpoints not in the spec)",
        report.unmatched_real_paths
    );
    for f in &report.findings {
        println!("DRIFT [{}] {}\n       {}", f.kind, f.endpoint, f.detail);
    }

    if let Some(out) = &output {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
        std::fs::write(out, json)
            .map_err(|e| mockforge_core::Error::config(format!("cannot write {out:?}: {e}")))?;
        println!("structured report written to {}", out.display());
    }

    if fail_on_drift && !report.findings.is_empty() {
        eprintln!(
            "verify-mocks: drift detected across {} endpoint(s); failing (--fail-on-drift)",
            report.endpoints_with_drift
        );
        std::process::exit(1);
    }
    Ok(())
}

// --- helpers -------------------------------------------------------------

fn load_fidelity_spec(spec: &PathBuf) -> mockforge_core::Result<OpenApiSpec> {
    let raw = std::fs::read_to_string(spec).map_err(|e| {
        mockforge_core::Error::config(format!("cannot read {}: {e}", spec.display()))
    })?;
    let mut spec_value: Value = serde_json::from_str(&raw)
        .or_else(|_| serde_yaml::from_str(&raw))
        .map_err(|e| {
            mockforge_core::Error::config(format!("cannot parse {}: {e}", spec.display()))
        })?;
    if spec_value.get("swagger").and_then(Value::as_str) == Some("2.0") {
        spec_value =
            mockforge_import::import::swagger2_convert::convert_swagger2_to_openapi3(&spec_value)
                .map_err(mockforge_core::Error::config)?;
    }
    OpenApiSpec::from_json(spec_value).map_err(|e| mockforge_core::Error::config(e.to_string()))
}

async fn load_fidelity_capture(
    capture_db: Option<&PathBuf>,
    limit: i32,
) -> mockforge_core::Result<Vec<mockforge_recorder::models::RecordedExchange>> {
    let Some(capture_db) = capture_db else {
        return Ok(Vec::new());
    };
    let db = mockforge_recorder::RecorderDatabase::open_read_only(capture_db)
        .await
        .map_err(|e| mockforge_core::Error::config(format!("cannot open capture db: {e}")))?;
    let requests = db
        .list_recent_for_protocol(mockforge_recorder::models::Protocol::Http, limit)
        .await
        .map_err(|e| mockforge_core::Error::config(format!("capture query failed: {e}")))?;
    let mut exchanges = Vec::with_capacity(requests.len());
    for request in requests {
        let response = db.get_response(&request.id).await.map_err(|e| {
            mockforge_core::Error::config(format!("capture response lookup failed: {e}"))
        })?;
        exchanges.push(mockforge_recorder::models::RecordedExchange { request, response });
    }
    if exchanges.is_empty() {
        return Err(mockforge_core::Error::config(
            "No recorded HTTP traffic to validate; capture real traffic first".to_string(),
        ));
    }
    Ok(exchanges)
}

fn strip_query(path: &str) -> String {
    path.split('?').next().unwrap_or(path).to_string()
}

fn body_present(bytes: Option<&[u8]>) -> bool {
    bytes.map(|b| !b.is_empty()).unwrap_or(false)
}

fn body_json(bytes: Option<&[u8]>) -> Option<Value> {
    bytes.and_then(|b| serde_json::from_slice(b).ok())
}

/// Decode a recorded body to bytes (utf8 passthrough, base64 otherwise).
fn decode_body(body: &Option<String>, encoding: &str) -> Option<Vec<u8>> {
    let raw = body.as_ref()?;
    if encoding == "base64" {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(raw).ok()
    } else {
        Some(raw.clone().into_bytes())
    }
}

fn to_value_map(map: &HashMap<String, String>) -> serde_json::Map<String, Value> {
    map.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect()
}

fn query_map(query: Option<&str>) -> serde_json::Map<String, Value> {
    // Match the parser used by the HTTP validation middleware, including form
    // spaces, encoded literal plus signs, and parameters without an '='.
    url::form_urlencoded::parse(query.unwrap_or_default().as_bytes())
        .map(|(key, value)| (key.into_owned(), Value::String(value.into_owned())))
        .collect()
}

fn header_value_map(headers_json: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str::<serde_json::Map<String, Value>>(headers_json).unwrap_or_default()
}

fn extract_path_params(concrete: &str, template: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let mut c_parts = concrete.split('/');
    for t in template.split('/') {
        let Some(c) = c_parts.next() else { break };
        if t.starts_with('{') && t.ends_with('}') {
            params.insert(t[1..t.len() - 1].to_string(), c.to_string());
        }
    }
    params
}

#[cfg(test)]
mod tests;
