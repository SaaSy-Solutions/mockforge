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

mod asynchronous;
mod fixtures;
use mockforge_contracts::http_route_match::match_template;
use mockforge_contracts::response_validation as responses;
use responses::{check_response, ResponseAssessment};

/// Verify mocks still match the real API (drift detection, #849).
///
/// Replays a recorder capture of real upstream traffic against the
/// spec your mocks are generated from and reports drift: endpoints the
/// mock doesn't know, schema violations on recorded bodies, and
/// responses whose status isn't declared.
#[derive(clap::Args, Debug)]
pub(crate) struct VerifyMocksArgs {
    /// Path to the OpenAPI or AsyncAPI 2/3 source specification
    /// (3.x natively; Swagger 2.0 is up-converted automatically).
    #[arg(short, long)]
    pub spec: PathBuf,

    /// Recorder sqlite database holding real captured traffic
    /// (what `mockforge record` writes).
    #[arg(short, long, required_unless_present_any = ["mock_spec", "fixtures"])]
    pub capture_db: Option<PathBuf>,

    /// OpenAPI or AsyncAPI definition used to generate the mock; compare with --spec.
    #[arg(long, required_unless_present_any = ["capture_db", "fixtures"])]
    pub mock_spec: Option<PathBuf>,

    /// HTTP replay or AsyncAPI recorder JSON fixture file/directory; checks every fixture.
    #[arg(long, required_unless_present_any = ["capture_db", "mock_spec"])]
    pub fixtures: Option<PathBuf>,

    /// Maximum recent HTTP exchanges or asynchronous message events (newest first).
    #[arg(long, default_value = "500", value_parser = clap::value_parser!(i32).range(1..))]
    pub limit: i32,

    /// Exit with code 1 when any drift is found (CI gate).
    #[arg(long)]
    pub fail_on_drift: bool,

    /// Write the structured drift report as JSON here.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Keep this run's result as the project's latest verification
    /// (`.mockforge/reports/verify-mocks.json` beside the project's config
    /// file), where `mockforge status` reports it.
    #[arg(long)]
    pub record: bool,
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
    endpoints_unverified: usize,
    unmatched_real_paths: usize,
    findings: Vec<DriftFinding>,
}

pub(crate) async fn handle_verify_mocks_command(
    args: VerifyMocksArgs,
) -> mockforge_core::Result<()> {
    let raw = load_fidelity_document(&args.spec)?;
    if raw.get("asyncapi").is_some() {
        return asynchronous::handle(args, raw).await;
    }
    let VerifyMocksArgs {
        spec,
        capture_db,
        mock_spec,
        fixtures,
        limit,
        fail_on_drift,
        output,
        record,
    } = args;

    let openapi =
        OpenApiSpec::from_json(raw).map_err(|e| mockforge_core::Error::config(e.to_string()))?;
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

    // An empty capture is not drift: report it, and with `--record` store a
    // `no-traffic` verification so status surfaces it instead of a pass. A
    // definition comparison (`--mock-spec`) or fixture check (`--fixtures`)
    // still runs and is reported below.
    if let (Some(capture_db), true) = (capture_db.as_ref(), exchanges.is_empty()) {
        println!(
            "No recorded traffic in {} — record some real traffic first (`mockforge record`).",
            capture_db.display()
        );
        if definition.is_none() && fixtures.is_none() {
            if record {
                record_result(&empty_capture_report(&spec), Some(capture_db.as_path())).await?;
            }
            return Ok(());
        }
    }
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
    let mut unverified_endpoints: BTreeMap<String, ()> = BTreeMap::new();
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
                kind: if route.status == "not-verified" {
                    "fixture-unverified"
                } else {
                    "fixture"
                }
                .into(),
                detail: route
                    .findings
                    .iter()
                    .map(|f| format!("{}: {}", f.kind, f.detail))
                    .collect::<Vec<_>>()
                    .join("; "),
            });
            if route.status == "not-verified" {
                unverified_endpoints.insert(route.endpoint.clone(), ());
            } else {
                drifted_endpoints.insert(route.endpoint.clone(), ());
            }
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
        let headers = header_value_map(&req.headers);
        if let Err((status, payload)) = registry.run_validation_with_recording_ex(
            &template,
            &method,
            &to_value_map(&extract_path_params(&concrete_path, &template)),
            &query_map(req.query_params.as_deref()),
            &headers,
            &cookie_value_map(&headers),
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
            let response_headers = header_value_map(&response.headers);
            let media = response_headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                .and_then(|(_, value)| value.as_str());
            let assessment = check_response(
                &registry,
                &response_spec,
                &method,
                &template,
                status,
                media,
                decoded.as_deref(),
            );
            let endpoint = format!("{method} {concrete_path}");
            match assessment {
                ResponseAssessment::Verified => {}
                ResponseAssessment::Drift(detail) => {
                    findings.push(DriftFinding {
                        endpoint: endpoint.clone(),
                        kind: "response-shape".into(),
                        detail: detail.detail,
                    });
                    drifted_endpoints.insert(endpoint, ());
                }
                ResponseAssessment::Unverified(detail) => {
                    findings.push(DriftFinding {
                        endpoint: endpoint.clone(),
                        kind: "response-unverified".into(),
                        detail,
                    });
                    unverified_endpoints.insert(endpoint, ());
                }
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
        endpoints_unverified: unverified_endpoints.len(),
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
    println!("endpoints not verified : {}", report.endpoints_unverified);
    println!(
        "unmatched real paths   : {} (traffic to endpoints not in the spec)",
        report.unmatched_real_paths
    );
    for f in &report.findings {
        println!("[{}] {}\n       {}", f.kind, f.endpoint, f.detail);
    }

    if let Some(out) = &output {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
        std::fs::write(out, json)
            .map_err(|e| mockforge_core::Error::config(format!("cannot write {out:?}: {e}")))?;
        println!("structured report written to {}", out.display());
    }
    if record {
        record_result(&report, capture_db.as_deref()).await?;
    }

    if fail_on_drift && !report.findings.is_empty() {
        eprintln!(
            "verify-mocks: {} endpoint(s) with drift and {} not verified; failing (--fail-on-drift)",
            report.endpoints_with_drift, report.endpoints_unverified
        );
        std::process::exit(1);
    }
    Ok(())
}

/// Keep `report` as the project's latest verification for `mockforge status`.
fn empty_capture_report(spec: &std::path::Path) -> DriftReport {
    DriftReport {
        spec: spec.display().to_string(),
        definition: None,
        fixtures: None,
        exchanges_checked: 0,
        endpoints_with_drift: 0,
        endpoints_unverified: 0,
        unmatched_real_paths: 0,
        findings: Vec::new(),
    }
}

fn verification_record(
    report: &DriftReport,
    capture_db: Option<&std::path::Path>,
) -> crate::status_commands::VerificationRecord {
    use crate::status_commands::{
        VerificationFinding, VerificationOutcome, VerificationRecord, VERIFICATION_SCHEMA_VERSION,
    };
    // Definition drift found without traffic is still drift.
    let outcome = if !report.findings.is_empty() {
        VerificationOutcome::Drift
    } else if report.exchanges_checked == 0 {
        VerificationOutcome::NoTraffic
    } else {
        VerificationOutcome::Faithful
    };
    VerificationRecord {
        schema_version: VERIFICATION_SCHEMA_VERSION,
        recorded_at: chrono::Utc::now().to_rfc3339(),
        mockforge_version: env!("CARGO_PKG_VERSION").to_string(),
        spec: report.spec.clone(),
        capture_db: capture_db.map(|p| p.display().to_string()).unwrap_or_default(),
        outcome,
        exchanges_checked: report.exchanges_checked,
        endpoints_with_drift: report.endpoints_with_drift,
        unmatched_real_paths: report.unmatched_real_paths,
        findings: report
            .findings
            .iter()
            .map(|f| VerificationFinding {
                endpoint: f.endpoint.clone(),
                kind: f.kind.clone(),
                detail: f.detail.clone(),
            })
            .collect(),
    }
}

async fn record_result(
    report: &DriftReport,
    capture_db: Option<&std::path::Path>,
) -> mockforge_core::Result<()> {
    use crate::status_commands::{project_root, VERIFICATION_FILE};
    let cwd = std::env::current_dir().map_err(|e| {
        mockforge_core::Error::config(format!("cannot read the current directory: {e}"))
    })?;
    let path = project_root(&cwd).await.join(VERIFICATION_FILE);
    let value = verification_record(report, capture_db);
    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            mockforge_core::Error::config(format!("cannot create {}: {e}", parent.display()))
        })?;
    }
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, json)
        .and_then(|()| std::fs::rename(&partial, &path))
        .map_err(|e| {
            mockforge_core::Error::config(format!("cannot write {}: {e}", path.display()))
        })?;
    println!("verification recorded in {}", path.display());
    Ok(())
}

// --- helpers -------------------------------------------------------------

fn load_fidelity_spec(spec: &PathBuf) -> mockforge_core::Result<OpenApiSpec> {
    OpenApiSpec::from_json(load_fidelity_document(spec)?)
        .map_err(|e| mockforge_core::Error::config(e.to_string()))
}

fn load_fidelity_document(spec: &PathBuf) -> mockforge_core::Result<Value> {
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
    Ok(spec_value)
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

fn cookie_value_map(headers: &serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .and_then(|(_, value)| value.as_str())
        .unwrap_or_default()
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .map(|(name, value)| (name.to_string(), Value::String(value.to_string())))
        .collect()
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
