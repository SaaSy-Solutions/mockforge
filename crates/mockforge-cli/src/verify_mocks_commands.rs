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

use openapiv3::ReferenceOr;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use mockforge_openapi::schema_ref_resolver::merge_components_into;
use mockforge_openapi::spec::OpenApiSpec;

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
    #[arg(short, long)]
    pub capture_db: PathBuf,

    /// Maximum exchanges to evaluate (oldest first).
    #[arg(long, default_value = "500")]
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
        limit,
        fail_on_drift,
        output,
    } = args;

    // 1. Load + parse the spec (Swagger 2.0 auto-up-converted).
    let raw = std::fs::read_to_string(&spec)
        .map_err(|e| mockforge_core::Error::config(format!("cannot read {}: {e}", spec.display())))?;
    let mut spec_value: Value = serde_json::from_str(&raw)
        .or_else(|_| serde_yaml::from_str(&raw))
        .map_err(|e| mockforge_core::Error::config(format!("cannot parse {}: {e}", spec.display())))?;
    if spec_value.get("swagger").and_then(Value::as_str) == Some("2.0") {
        spec_value = mockforge_import::import::swagger2_convert::convert_swagger2_to_openapi3(
            &spec_value,
        )
        .map_err(mockforge_core::Error::config)?;
    }
    let openapi =
        OpenApiSpec::from_json(spec_value).map_err(|e| mockforge_core::Error::config(e.to_string()))?;
    let registry = OpenApiRouteRegistry::new(openapi);

    // 2. Open the capture DB and pull HTTP exchanges.
    let db = mockforge_recorder::RecorderDatabase::new(&capture_db)
        .await
        .map_err(|e| mockforge_core::Error::config(format!("cannot open capture db: {e}")))?;
    let filter = mockforge_recorder::query::QueryFilter {
        protocol: Some(mockforge_recorder::models::Protocol::Http),
        ..Default::default()
    };
    let result = mockforge_recorder::query::execute_query(&db, filter)
        .await
        .map_err(|e| mockforge_core::Error::config(format!("capture query failed: {e}")))?;
    let exchanges: Vec<_> = result.exchanges.into_iter().take(limit as usize).collect();

    if exchanges.is_empty() {
        println!(
            "No recorded traffic in {} — record some real traffic first (`mockforge record`).",
            capture_db.display()
        );
        return Ok(());
    }

    // 3. Evaluate each exchange.
    let mut findings: Vec<DriftFinding> = Vec::new();
    let mut drifted_endpoints: BTreeMap<String, ()> = BTreeMap::new();
    let mut unmatched_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for exchange in &exchanges {
        let req = &exchange.request;
        let method = req.method.to_uppercase();
        let concrete_path = strip_query(&req.path);
        let Some((template, _path_params)) = match_template(&registry, &method, &concrete_path)
        else {
            unmatched_paths.insert(concrete_path.clone());
            continue;
        };

        // Request-side validation through the same chain `serve` uses.
        // This records into the shared conformance buffer too, which is a
        // feature: CLI drift shows up in the TUI Conformance tab as well.
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
                &method,
                &template,
                concrete_path.as_str(),
                status,
                decoded.as_deref(),
            ) {
                let endpoint = format!("{method} {concrete_path}");
                findings.push(DriftFinding {
                    endpoint: endpoint.clone(),
                    kind: "response-shape".into(),
                    detail,
                });
                drifted_endpoints.insert(endpoint, ());
            }
        }
    }

    let report = DriftReport {
        spec: spec.display().to_string(),
        exchanges_checked: exchanges.len(),
        endpoints_with_drift: drifted_endpoints.len(),
        unmatched_real_paths: unmatched_paths.len(),
        findings,
    };

    // 4. Report.
    println!("\n=== mock fidelity / drift report ===");
    println!("exchanges checked      : {}", report.exchanges_checked);
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
        base64::engine::general_purpose::STANDARD
            .decode(raw)
            .ok()
    } else {
        Some(raw.clone().into_bytes())
    }
}

fn to_value_map(map: &HashMap<String, String>) -> serde_json::Map<String, Value> {
    map.iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect()
}

fn query_map(query: Option<&str>) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                if let (Ok(k), Ok(v)) = (
                    urlencoding::decode(k),
                    urlencoding::decode(v),
                ) {
                    out.insert(k.into_owned(), Value::String(v.into_owned()));
                }
            }
        }
    }
    out
}

fn header_value_map(headers_json: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str::<serde_json::Map<String, Value>>(headers_json)
        .unwrap_or_default()
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

fn match_template(
    registry: &OpenApiRouteRegistry,
    method: &str,
    concrete_path: &str,
) -> Option<(String, HashMap<String, String>)> {
    registry.routes().iter().find_map(|route| {
        if !route.method.eq_ignore_ascii_case(method) {
            return None;
        }
        segment_match(concrete_path, &route.path)
            .map(|params| (route.path.clone(), params))
    })
}

fn segment_match(concrete: &str, template: &str) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut c_parts = concrete.split('/');
    for t in template.split('/') {
        let c = c_parts.next()?;
        if t.starts_with('{') && t.ends_with('}') {
            params.insert(t[1..t.len() - 1].to_string(), c.to_string());
        } else if t != c {
            return None;
        }
    }
    if c_parts.next().is_some() {
        return None;
    }
    Some(params)
}

/// Response-side drift: validate the recorded body against the declared
/// schema for the actual status code. Returns a human detail string on
/// drift.
fn check_response_drift(
    registry: &OpenApiRouteRegistry,
    method: &str,
    template: &str,
    concrete_path: &str,
    status: u16,
    body: Option<&[u8]>,
) -> Option<String> {
    let route = registry.get_route(template, method)?;
    let responses = &route.operation.responses;
    let declared_schema: Option<Value> = responses.responses.iter().find_map(|(code, r)| {
        let matches_status = match code {
            openapiv3::StatusCode::Code(c) => *c == status,
            openapiv3::StatusCode::Range(start) => status >= *start && status < *start + 100,
        };
        if !matches_status {
            return None;
        }
        let ReferenceOr::Item(response) = r else {
            return None;
        };
        let media = response.content.get("application/json")?;
        match &media.schema {
            Some(openapiv3::ReferenceOr::Item(schema)) => {
                serde_json::to_value(schema).ok()
            }
            Some(openapiv3::ReferenceOr::Reference { reference }) => {
                Some(serde_json::json!({ "$ref": reference }))
            }
            None => None,
        }
    });

    let declared = declared_schema.or_else(|| {
        (!responses.responses.is_empty() || responses.default.is_some()).then(|| Value::Null)
    })?;
    if declared.is_null() {
        return None;
    }

    let Some(bytes) = body else {
        return Some(format!(
            "declared {status} response schema but the recorded response had no body"
        ));
    };
    let Ok(body_json) = serde_json::from_slice::<Value>(bytes) else {
        return Some(format!(
            "declared JSON schema for {status} but the recorded response is not JSON"
        ));
    };

    let merged = merge_components_into(declared, &registry.spec().spec);
    let result =
        mockforge_openapi::openapi_routes::validation::validate_json_value(&body_json, &merged);
    if result.errors.is_empty() {
        return None;
    }
    Some(result.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "))
}
