//! Positive + per-category negative request driver against a live server.
//!
//! Issue #79 round 13 (4) — Srikanth's (e) ask: a way to test both
//! positive and negative compliance scenarios separately, where the
//! positive cases should pass and the negative cases should be
//! rejected.
//!
//! This module sits *alongside* the existing conformance executor
//! (which drives k6 / native checks on a single positive call per
//! operation). The self-test driver synthesises per-category
//! deliberately-bad requests and asserts that the server actually
//! rejects them with a 4xx — useful when verifying that
//! `validate_request_with_all` is wired correctly for the user's spec
//! (the exact gap that round-13 (3) fixed).
//!
//! Scope of the initial MVP: covers the highest-signal negatives —
//! empty body when one is required, missing required query/header
//! params, and wrong-type path params. Doesn't try to mutate every
//! field of a JSON-Schema-validated body; that's a follow-up.

use super::spec_driven::{AnnotatedOperation, ApiKeyLocation, SecuritySchemeInfo};
use reqwest::{Client, Method};
use std::collections::BTreeMap;
use std::time::Duration;

/// Round 17.2 — cap on schema-driven negatives per operation. A spec
/// with 100 properties per body could produce hundreds of mutations
/// for a single operation; combined with thousands of operations
/// that's a runaway test matrix. 12 covers the highest-signal
/// mutations (type mismatch + required-removed + a few constraint
/// breaks) without exploding wall time on large specs.
const SCHEMA_MUTATION_CAP: usize = 12;

/// Configuration for a self-test run.
#[derive(Debug, Clone)]
pub struct SelfTestConfig {
    pub target_url: String,
    pub skip_tls_verify: bool,
    pub timeout: Duration,
    /// Optional extra headers to attach to every request (e.g. auth).
    pub extra_headers: Vec<(String, String)>,
    /// Delay between requests to avoid hammering the server.
    pub delay_between_requests: Duration,
}

impl Default for SelfTestConfig {
    fn default() -> Self {
        Self {
            target_url: "http://localhost:3000".into(),
            skip_tls_verify: false,
            timeout: Duration::from_secs(15),
            extra_headers: Vec::new(),
            delay_between_requests: Duration::from_millis(0),
        }
    }
}

/// Outcome of a single test case (positive or negative).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaseOutcome {
    pub label: String,
    pub expected_4xx: bool,
    pub actual_status: u16,
    /// True when the response status matches expectation
    /// (positive → 2xx-3xx, negative → 4xx).
    pub passed: bool,
}

/// All cases run against one annotated operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OperationResult {
    pub method: String,
    pub path: String,
    pub positive: Option<CaseOutcome>,
    pub negatives: Vec<CaseOutcome>,
}

/// Summary report rolled up across all operations.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SelfTestReport {
    pub positive_pass: usize,
    pub positive_fail: usize,
    /// Per category: count of negative cases the server correctly
    /// rejected with a 4xx (we caught the spec violation).
    pub negative_caught: BTreeMap<String, usize>,
    /// Per category: count of negative cases that should have been
    /// rejected but came back with a non-4xx (validator gap).
    pub negative_missed: BTreeMap<String, usize>,
    pub operations: Vec<OperationResult>,
}

impl SelfTestReport {
    /// All-pass means every positive case got 2xx-3xx and every
    /// negative case got 4xx.
    pub fn all_passed(&self) -> bool {
        self.positive_fail == 0 && self.negative_missed.values().sum::<usize>() == 0
    }

    /// Human-readable summary string. One line for positives, one per
    /// category for negatives. Designed to slot into existing
    /// `TerminalReporter` output.
    pub fn render_summary(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Positives: {} pass / {} fail\n",
            self.positive_pass, self.positive_fail
        ));
        let mut keys: Vec<&String> =
            self.negative_caught.keys().chain(self.negative_missed.keys()).collect();
        keys.sort();
        keys.dedup();
        for cat in keys {
            let caught = self.negative_caught.get(cat).copied().unwrap_or(0);
            let missed = self.negative_missed.get(cat).copied().unwrap_or(0);
            let mark = if missed == 0 { "✓" } else { "⚠" };
            out.push_str(&format!(
                "Negatives [{}]: {} caught / {} missed  {}\n",
                cat, caught, missed, mark
            ));
        }
        out
    }
}

/// Execute the self-test plan against `config.target_url` for every
/// `AnnotatedOperation`. Returns the aggregated report; callers
/// decide how to display it (e.g. via `render_summary` or by writing
/// the JSON serialisation to disk).
pub async fn run_self_test(
    operations: &[AnnotatedOperation],
    config: &SelfTestConfig,
) -> Result<SelfTestReport, reqwest::Error> {
    let mut builder = Client::builder().timeout(config.timeout);
    if config.skip_tls_verify {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder.build()?;

    let mut report = SelfTestReport::default();
    for op in operations {
        let result = test_operation(&client, config, op).await;
        if let Some(p) = &result.positive {
            if p.passed {
                report.positive_pass += 1;
            } else {
                report.positive_fail += 1;
            }
        }
        for neg in &result.negatives {
            let cat = neg.label.split(':').next().unwrap_or("other").to_string();
            if neg.passed {
                *report.negative_caught.entry(cat).or_insert(0) += 1;
            } else {
                *report.negative_missed.entry(cat).or_insert(0) += 1;
            }
        }
        report.operations.push(result);
        if !config.delay_between_requests.is_zero() {
            tokio::time::sleep(config.delay_between_requests).await;
        }
    }
    Ok(report)
}

async fn test_operation(
    client: &Client,
    config: &SelfTestConfig,
    op: &AnnotatedOperation,
) -> OperationResult {
    let url = build_url(&config.target_url, &op.path, &op.path_params);
    let method = Method::from_bytes(op.method.to_uppercase().as_bytes()).unwrap_or(Method::GET);

    // ── Positive case ────────────────────────────────────────────
    let positive = send_case(
        client,
        config,
        method.clone(),
        &url,
        "positive",
        false,
        op.sample_body.as_deref(),
        op.query_params.clone(),
        op.header_params.clone(),
    )
    .await;

    // ── Negative cases ───────────────────────────────────────────
    let mut negatives = Vec::new();

    // (a) empty body when one is required.
    //
    // Round 16 — drop the `sample_body.is_some()` precondition. Operations
    // whose body annotator couldn't synthesize a sample previously got
    // zero negatives (so the self-test reported "all passing" even on
    // POST /resource with a required body). The spec saying the operation
    // *has* a request body is enough — an empty object is a valid
    // negative regardless of whether we have a positive sample.
    if op.request_body_content_type.is_some() {
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &url,
                "request-body:empty",
                true,
                Some("{}"),
                op.query_params.clone(),
                op.header_params.clone(),
            )
            .await,
        );

        // (b) wrong-shaped body (array instead of object) — exercises
        // top-level type validation independently of which fields are
        // required.
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &url,
                "request-body:wrong-type",
                true,
                Some("[]"),
                op.query_params.clone(),
                op.header_params.clone(),
            )
            .await,
        );

        // Round 17.2 — schema-aware negatives.
        //
        // When both a positive sample AND the resolved body schema are
        // available, mutate the sample per-field (type mismatch,
        // min/max bounds, pattern, enum out-of-range, required-field
        // removal) and assert each is rejected with 4xx. Capped at
        // SCHEMA_MUTATION_CAP per operation so a 100-property body
        // doesn't explode the test matrix.
        if let (Some(sample_str), Some(schema)) =
            (op.sample_body.as_deref(), op.request_body_schema.as_ref())
        {
            if let Ok(sample) = serde_json::from_str::<serde_json::Value>(sample_str) {
                let mutations = super::schema_mutator::mutate_body(&sample, schema);
                for m in mutations.into_iter().take(SCHEMA_MUTATION_CAP) {
                    let body_str = serde_json::to_string(&m.body).unwrap_or_default();
                    negatives.push(
                        send_case(
                            client,
                            config,
                            method.clone(),
                            &url,
                            &m.label,
                            true,
                            Some(&body_str),
                            op.query_params.clone(),
                            op.header_params.clone(),
                        )
                        .await,
                    );
                }
            }
        }
    }

    // Round 17.2 — URI-length probe. Spec-agnostic but schema-aware in
    // spirit: most servers cap URIs at 8 KB or so. Append a 9 KB query
    // string to the URL and expect 414 URI Too Long (or 400). Skipped
    // for operations that already have a heavy positive query.
    {
        let pad = "p=".to_string() + &"x".repeat(9_000);
        let bad_url = if url.contains('?') {
            format!("{url}&{pad}")
        } else {
            format!("{url}?{pad}")
        };
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &bad_url,
                "parameters:uri-too-long",
                true,
                op.sample_body.as_deref(),
                op.query_params.clone(),
                op.header_params.clone(),
            )
            .await,
        );
    }

    // (e) Round 16 — path-param type probe. Send the first path
    // parameter as a literal `"self-test-invalid-id"`: a string that
    // contains hyphens, won't parse as an integer, won't parse as a
    // UUID, and won't match any typical regex pattern. Operations
    // whose spec types the param as `integer` or `string` with a
    // `format`/`pattern` will catch this (caught: server returned
    // 4xx); operations whose spec lets path params be free-form
    // strings will let it through (missed: server returned 2xx).
    // Either outcome is informative: a category that's all "missed"
    // tells the user their spec is loose on path-param types, which
    // is itself worth knowing. Addresses Srikanth's "always all
    // passing" report — operations with a path param now produce at
    // least one probe instead of zero.
    if !op.path_params.is_empty() {
        let mut url_with_placeholder = op.path.clone();
        if let Some((first_name, _)) = op.path_params.first() {
            // Substitute every other path-param with its sample so the
            // route shape stays intact and only the first param is bad.
            for (name, value) in op.path_params.iter().skip(1) {
                if !value.is_empty() {
                    url_with_placeholder =
                        url_with_placeholder.replace(&format!("{{{name}}}"), value);
                }
            }
            // Substitute the first param with a guaranteed-invalid
            // sentinel that's unlikely to match any reasonable schema:
            // contains characters disallowed in numeric IDs *and* UUIDs.
            url_with_placeholder =
                url_with_placeholder.replace(&format!("{{{first_name}}}"), "self-test-invalid-id");
            let target = config.target_url.trim_end_matches('/');
            let bad_url = if url_with_placeholder.starts_with('/') {
                format!("{}{}", target, url_with_placeholder)
            } else {
                format!("{}/{}", target, url_with_placeholder)
            };
            negatives.push(
                send_case(
                    client,
                    config,
                    method.clone(),
                    &bad_url,
                    "parameters:bad-path-param",
                    true,
                    op.sample_body.as_deref(),
                    op.query_params.clone(),
                    op.header_params.clone(),
                )
                .await,
            );
        }
    }

    // (c) drop the first required query param
    if !op.query_params.is_empty() {
        let mut q = op.query_params.clone();
        q.remove(0);
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &url,
                "parameters:missing-query",
                true,
                op.sample_body.as_deref(),
                q,
                op.header_params.clone(),
            )
            .await,
        );
    }

    // (s) Round 17.3 — security probes.
    //
    // Operations whose spec declares a security requirement get a
    // dedicated set of negatives. The point isn't to test whether the
    // server's *real* auth works (the positive case already does that
    // via `extra_headers`) — it's to check whether deliberately-bad
    // credentials are still rejected, which is exactly the failure
    // mode that lets an attacker through a half-wired validator.
    //
    // Each probe replaces or omits the relevant auth credential and
    // expects 401 / 403. A 2xx here is a hard finding: "spec says
    // this endpoint is protected, server let unauthenticated /
    // wrong-credential traffic through".
    //
    // Bounded: at most one probe per declared scheme kind, so an
    // operation with 3 security requirements doesn't 4× the request
    // volume. Skips entirely when `op.security_schemes` is empty.
    for probe in build_security_probes(&op.security_schemes) {
        // Strip any pre-existing Authorization or known API-key
        // header from extra_headers + header_params so the probe
        // value is the *only* credential the server sees.
        let stripped_extra = strip_auth(&config.extra_headers, &op.security_schemes);
        let stripped_headers = strip_auth(&op.header_params, &op.security_schemes);
        let stripped_query = strip_auth_query(&op.query_params, &op.security_schemes);
        let mut req_headers = stripped_headers;
        for (k, v) in &probe.headers {
            req_headers.push((k.clone(), v.clone()));
        }
        let mut req_query = stripped_query;
        for (k, v) in &probe.query {
            req_query.push((k.clone(), v.clone()));
        }
        negatives.push(
            send_case_with_extra(
                client,
                config,
                method.clone(),
                &url,
                &probe.label,
                true,
                op.sample_body.as_deref(),
                req_query,
                req_headers,
                stripped_extra,
            )
            .await,
        );
    }

    // (d) drop the first required header
    if !op.header_params.is_empty() {
        let mut h = op.header_params.clone();
        h.remove(0);
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &url,
                "parameters:missing-header",
                true,
                op.sample_body.as_deref(),
                op.query_params.clone(),
                h,
            )
            .await,
        );
    }

    OperationResult {
        method: op.method.clone(),
        path: op.path.clone(),
        positive: Some(positive),
        negatives,
    }
}

/// Round 17.3 — one synthesised bad credential to send.
#[derive(Debug, Clone)]
struct SecurityProbe {
    /// Self-test label, e.g. `security:bad-bearer`.
    label: String,
    /// Headers to attach to the probe request.
    headers: Vec<(String, String)>,
    /// Query parameters to attach (API key in query case).
    query: Vec<(String, String)>,
}

/// For each declared security scheme, produce one bad-credential
/// probe plus a single "no auth at all" probe that exercises the
/// missing-credential code path. Deduplicates by scheme kind so an
/// operation declaring `[bearer, bearer]` only yields one Bearer
/// probe.
fn build_security_probes(schemes: &[SecuritySchemeInfo]) -> Vec<SecurityProbe> {
    if schemes.is_empty() {
        return Vec::new();
    }
    let mut probes: Vec<SecurityProbe> = Vec::new();
    let mut seen_bearer = false;
    let mut seen_basic = false;
    // `(loc_tag, name)` — ApiKeyLocation doesn't implement Ord, so
    // we tag it with a short discriminant string for dedup.
    let mut seen_apikey: std::collections::BTreeSet<(&'static str, String)> = Default::default();
    for s in schemes {
        match s {
            SecuritySchemeInfo::Bearer if !seen_bearer => {
                seen_bearer = true;
                probes.push(SecurityProbe {
                    label: "security:bad-bearer".into(),
                    headers: vec![(
                        "Authorization".into(),
                        "Bearer self-test-invalid-token".into(),
                    )],
                    query: Vec::new(),
                });
            }
            SecuritySchemeInfo::Basic if !seen_basic => {
                seen_basic = true;
                // base64("self-test:invalid") — valid base64, wrong creds.
                probes.push(SecurityProbe {
                    label: "security:bad-basic".into(),
                    headers: vec![(
                        "Authorization".into(),
                        "Basic c2VsZi10ZXN0OmludmFsaWQ=".into(),
                    )],
                    query: Vec::new(),
                });
            }
            SecuritySchemeInfo::ApiKey { location, name } => {
                let loc_tag = match location {
                    ApiKeyLocation::Header => "header",
                    ApiKeyLocation::Query => "query",
                    ApiKeyLocation::Cookie => "cookie",
                };
                if seen_apikey.contains(&(loc_tag, name.clone())) {
                    continue;
                }
                seen_apikey.insert((loc_tag, name.clone()));
                let label = format!("security:bad-apikey:{}", name);
                let bad = "self-test-invalid-key".to_string();
                match location {
                    ApiKeyLocation::Header => probes.push(SecurityProbe {
                        label,
                        headers: vec![(name.clone(), bad)],
                        query: Vec::new(),
                    }),
                    ApiKeyLocation::Query => probes.push(SecurityProbe {
                        label,
                        headers: Vec::new(),
                        query: vec![(name.clone(), bad)],
                    }),
                    ApiKeyLocation::Cookie => probes.push(SecurityProbe {
                        label,
                        headers: vec![("Cookie".into(), format!("{}={}", name, bad))],
                        query: Vec::new(),
                    }),
                }
            }
            _ => {}
        }
    }
    // Always add a "no auth at all" probe when *any* security scheme
    // is declared — useful even if all schemes failed to resolve to a
    // testable kind, because it surfaces validators that aren't
    // checking auth presence at all.
    probes.push(SecurityProbe {
        label: "security:no-auth".into(),
        headers: Vec::new(),
        query: Vec::new(),
    });
    probes
}

/// Remove Authorization and any API-key headers declared by the
/// operation's security schemes from `headers`, so a security probe
/// can supply its own credential (or none) cleanly.
fn strip_auth(
    headers: &[(String, String)],
    schemes: &[SecuritySchemeInfo],
) -> Vec<(String, String)> {
    let mut apikey_headers: std::collections::BTreeSet<String> = Default::default();
    for s in schemes {
        if let SecuritySchemeInfo::ApiKey {
            location: ApiKeyLocation::Header,
            name,
        } = s
        {
            apikey_headers.insert(name.to_lowercase());
        }
        if let SecuritySchemeInfo::ApiKey {
            location: ApiKeyLocation::Cookie,
            ..
        } = s
        {
            apikey_headers.insert("cookie".into());
        }
    }
    headers
        .iter()
        .filter(|(k, _)| {
            let lk = k.to_lowercase();
            lk != "authorization" && !apikey_headers.contains(&lk)
        })
        .cloned()
        .collect()
}

/// Remove API-key query parameters declared by the operation's
/// security schemes from `query`, so a probe can supply its own.
fn strip_auth_query(
    query: &[(String, String)],
    schemes: &[SecuritySchemeInfo],
) -> Vec<(String, String)> {
    let mut apikey_query: std::collections::BTreeSet<String> = Default::default();
    for s in schemes {
        if let SecuritySchemeInfo::ApiKey {
            location: ApiKeyLocation::Query,
            name,
        } = s
        {
            apikey_query.insert(name.clone());
        }
    }
    query.iter().filter(|(k, _)| !apikey_query.contains(k)).cloned().collect()
}

/// Variant of `send_case` that takes an explicit `extra_headers`
/// (rather than reading them from `config`). Used by security probes
/// to substitute or strip the configured Authorization header.
#[allow(clippy::too_many_arguments)]
async fn send_case_with_extra(
    client: &Client,
    config: &SelfTestConfig,
    method: Method,
    url: &str,
    label: &str,
    expected_4xx: bool,
    body: Option<&str>,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    extra_headers: Vec<(String, String)>,
) -> CaseOutcome {
    let _ = config; // signature parity; timeout already on `client`.
    let mut req = client.request(method, url);
    for (k, v) in &query {
        req = req.query(&[(k.as_str(), v.as_str())]);
    }
    for (k, v) in &headers {
        req = req.header(k, v);
    }
    for (k, v) in &extra_headers {
        req = req.header(k, v);
    }
    if let Some(b) = body {
        req = req
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(b.to_string());
    }
    let actual_status = match req.send().await {
        Ok(resp) => resp.status().as_u16(),
        Err(_) => 0,
    };
    let passed = if expected_4xx {
        (400..500).contains(&actual_status)
    } else {
        (200..400).contains(&actual_status)
    };
    CaseOutcome {
        label: label.to_string(),
        expected_4xx,
        actual_status,
        passed,
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_case(
    client: &Client,
    config: &SelfTestConfig,
    method: Method,
    url: &str,
    label: &str,
    expected_4xx: bool,
    body: Option<&str>,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
) -> CaseOutcome {
    let mut req = client.request(method, url);
    for (k, v) in &query {
        req = req.query(&[(k.as_str(), v.as_str())]);
    }
    for (k, v) in &headers {
        req = req.header(k, v);
    }
    for (k, v) in &config.extra_headers {
        req = req.header(k, v);
    }
    if let Some(b) = body {
        req = req
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(b.to_string());
    }

    let actual_status = match req.send().await {
        Ok(resp) => resp.status().as_u16(),
        Err(e) if e.is_timeout() => 0,
        Err(_) => 0,
    };

    let passed = if expected_4xx {
        (400..500).contains(&actual_status)
    } else {
        (200..400).contains(&actual_status)
    };

    CaseOutcome {
        label: label.to_string(),
        expected_4xx,
        actual_status,
        passed,
    }
}

/// Substitute `{param}` placeholders in the spec path with their
/// sample values from `path_params`, then prepend `target_url`. Empty
/// values are kept as `{param}` so an upstream router still matches
/// the template — useful when `path_params` is empty and we want to
/// hit the same route the spec defines.
fn build_url(target: &str, path_template: &str, path_params: &[(String, String)]) -> String {
    let mut url = path_template.to_string();
    for (name, value) in path_params {
        let placeholder = format!("{{{}}}", name);
        if !value.is_empty() {
            url = url.replace(&placeholder, value);
        }
    }
    let target = target.trim_end_matches('/');
    if url.starts_with('/') {
        format!("{}{}", target, url)
    } else {
        format!("{}/{}", target, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(
        method: &str,
        path: &str,
        body: Option<&str>,
        query: Vec<(&str, &str)>,
        headers: Vec<(&str, &str)>,
        path_params: Vec<(&str, &str)>,
    ) -> AnnotatedOperation {
        AnnotatedOperation {
            method: method.into(),
            path: path.into(),
            features: Vec::new(),
            request_body_content_type: body.map(|_| "application/json".into()),
            sample_body: body.map(|s| s.to_string()),
            query_params: query.into_iter().map(|(a, b)| (a.into(), b.into())).collect(),
            header_params: headers.into_iter().map(|(a, b)| (a.into(), b.into())).collect(),
            path_params: path_params.into_iter().map(|(a, b)| (a.into(), b.into())).collect(),
            response_schema: None,
            request_body_schema: None,
            security_schemes: Vec::new(),
        }
    }

    #[test]
    fn build_url_substitutes_path_params() {
        let url = build_url(
            "https://api.test/",
            "/users/{id}/posts/{pid}",
            &[("id".into(), "42".into()), ("pid".into(), "7".into())],
        );
        assert_eq!(url, "https://api.test/users/42/posts/7");
    }

    #[test]
    fn build_url_keeps_placeholders_when_no_sample() {
        let url = build_url("https://api.test", "/users/{id}", &[]);
        assert_eq!(url, "https://api.test/users/{id}");
    }

    #[test]
    fn report_summary_calls_out_misses() {
        let r = SelfTestReport {
            positive_pass: 3,
            positive_fail: 0,
            negative_caught: BTreeMap::from([("request-body".into(), 2)]),
            negative_missed: BTreeMap::from([("request-body".into(), 1)]),
            operations: Vec::new(),
        };
        let summary = r.render_summary();
        assert!(summary.contains("Positives: 3 pass / 0 fail"));
        assert!(summary.contains("Negatives [request-body]: 2 caught / 1 missed"));
        assert!(summary.contains("⚠"));
        assert!(!r.all_passed());
    }

    #[test]
    fn report_all_passed_when_no_miss() {
        let r = SelfTestReport {
            positive_pass: 5,
            positive_fail: 0,
            negative_caught: BTreeMap::from([("parameters".into(), 3)]),
            negative_missed: BTreeMap::new(),
            operations: Vec::new(),
        };
        assert!(r.all_passed());
        assert!(r.render_summary().contains("✓"));
    }

    #[tokio::test]
    async fn run_self_test_against_unreachable_target_marks_all_failed() {
        // Use an obviously-dead port so we exercise the timeout/error
        // path without needing a live server in tests.
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let ops = vec![op(
            "POST",
            "/users",
            Some("{\"name\":\"a\"}"),
            vec![],
            vec![],
            vec![],
        )];
        let report = run_self_test(&ops, &cfg).await.expect("client builds");
        // All cases hit the connect-error path → actual_status=0.
        // Positive expects 2xx-3xx → 0 is fail. Negatives expect 4xx
        // → 0 is also fail (we missed catching).
        assert_eq!(report.positive_fail, 1);
        assert!(report.negative_missed.values().sum::<usize>() >= 1);
        assert!(!report.all_passed());
    }

    /// Round 17.2 — operations with both a positive sample AND a
    /// resolved request-body schema produce schema-driven negatives
    /// in addition to the spec-agnostic empty/wrong-type ones. The
    /// labels carry the field path so a per-category report can tell
    /// you exactly which field caught.
    #[tokio::test]
    async fn schema_driven_negatives_fire_when_schema_present() {
        use openapiv3::{ObjectType, ReferenceOr, Schema, SchemaData, SchemaKind, Type};
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        // Build an operation whose schema has a required `name` string
        // and an `age` integer. The mutator should produce, at
        // minimum: required-removed:name, required-removed:age,
        // type-mismatch:name, type-mismatch:age, integer-as-float:age,
        // plus the root-level type-mismatch.
        let mut obj = ObjectType::default();
        obj.properties.insert(
            "name".to_string(),
            ReferenceOr::Item(Box::new(Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::String(Default::default())),
            })),
        );
        obj.properties.insert(
            "age".to_string(),
            ReferenceOr::Item(Box::new(Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::Integer(Default::default())),
            })),
        );
        obj.required = vec!["name".into(), "age".into()];
        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(obj)),
        };

        let mut o =
            op("POST", "/users", Some(r#"{"name":"Ada","age":30}"#), vec![], vec![], vec![]);
        o.request_body_schema = Some(schema);
        let report = run_self_test(&[o], &cfg).await.expect("client builds");
        // Bucket labels from the operation result.
        let labels: std::collections::BTreeSet<String> = report
            .operations
            .iter()
            .flat_map(|op| op.negatives.iter().map(|n| n.label.clone()))
            .collect();
        assert!(
            labels.iter().any(|l| l.starts_with("request-body:type-mismatch:")),
            "missing type-mismatch negative; got {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.starts_with("request-body:required-removed:")),
            "missing required-removed negative; got {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "parameters:uri-too-long"),
            "missing URI-length negative; got {labels:?}"
        );
    }

    /// Round 16 — operations with a body OR a path-param now produce
    /// negatives even without a sample body. Previously a POST whose
    /// body annotator failed produced *zero* negatives, so the self-test
    /// always reported "all passing" for that endpoint.
    #[tokio::test]
    async fn no_sample_body_still_produces_request_body_negatives() {
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        // POST with a body content type but no sample (annotator gap).
        let ops = vec![op("POST", "/x", None, vec![], vec![], vec![])];
        // No sample_body but request_body_content_type set:
        let mut ops_fixed = ops;
        ops_fixed[0].request_body_content_type = Some("application/json".into());
        let report = run_self_test(&ops_fixed, &cfg).await.expect("client builds");
        // Both request-body negatives (empty + wrong-type) should fire,
        // landing in `negative_missed` because the unreachable target
        // returns no 4xx. The point: count > 0.
        assert!(
            report.negative_missed.values().sum::<usize>() >= 2,
            "expected ≥2 request-body negatives, got {:?}",
            report.negative_missed
        );
    }

    /// Round 16 — operations with a path-param now get a probe even
    /// when there's no body / required query / required header.
    /// Previously `/teams/{team-id}` with no other required fields
    /// produced zero negatives → always "all passing".
    #[tokio::test]
    async fn path_param_only_endpoint_produces_a_probe() {
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let ops = vec![op(
            "GET",
            "/teams/{team-id}",
            None,
            vec![],
            vec![],
            vec![("team-id", "1")],
        )];
        let report = run_self_test(&ops, &cfg).await.expect("client builds");
        let total: usize = report.negative_caught.values().sum::<usize>()
            + report.negative_missed.values().sum::<usize>();
        assert!(total >= 1, "expected ≥1 path-param probe, got {:?}", report);
    }

    /// Round 17.3 — operations declaring a Bearer + ApiKey-header
    /// scheme should produce one bad-bearer + one bad-apikey + one
    /// no-auth probe (three negatives, no duplicates).
    #[test]
    fn build_security_probes_one_per_scheme_kind() {
        let probes = build_security_probes(&[
            SecuritySchemeInfo::Bearer,
            SecuritySchemeInfo::Bearer, // duplicate kind, should dedup
            SecuritySchemeInfo::ApiKey {
                location: ApiKeyLocation::Header,
                name: "X-API-Key".into(),
            },
        ]);
        let labels: Vec<&str> = probes.iter().map(|p| p.label.as_str()).collect();
        assert!(labels.contains(&"security:bad-bearer"));
        assert!(labels.contains(&"security:bad-apikey:X-API-Key"));
        assert!(labels.contains(&"security:no-auth"));
        assert_eq!(probes.len(), 3);
    }

    #[test]
    fn build_security_probes_empty_when_no_schemes() {
        assert!(build_security_probes(&[]).is_empty());
    }

    #[test]
    fn strip_auth_removes_authorization_case_insensitive() {
        let h = strip_auth(
            &[
                ("authorization".into(), "Bearer real".into()),
                ("Accept".into(), "application/json".into()),
            ],
            &[SecuritySchemeInfo::Bearer],
        );
        let keys: Vec<&str> = h.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["Accept"]);
    }

    #[test]
    fn strip_auth_removes_apikey_header_and_cookie() {
        let h = strip_auth(
            &[
                ("X-API-Key".into(), "real".into()),
                ("Cookie".into(), "session=abc".into()),
                ("Accept".into(), "json".into()),
            ],
            &[
                SecuritySchemeInfo::ApiKey {
                    location: ApiKeyLocation::Header,
                    name: "X-API-Key".into(),
                },
                SecuritySchemeInfo::ApiKey {
                    location: ApiKeyLocation::Cookie,
                    name: "session".into(),
                },
            ],
        );
        let keys: Vec<&str> = h.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["Accept"]);
    }

    /// Round 17.3 — an operation with a declared Bearer scheme should
    /// produce at least the `security:bad-bearer` + `security:no-auth`
    /// negatives. Unreachable target → these land in `negative_missed`.
    #[tokio::test]
    async fn security_probes_fire_when_schemes_declared() {
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let mut o = op("GET", "/me", None, vec![], vec![], vec![]);
        o.security_schemes = vec![SecuritySchemeInfo::Bearer];
        let report = run_self_test(&[o], &cfg).await.expect("client builds");
        let labels: std::collections::BTreeSet<String> = report
            .operations
            .iter()
            .flat_map(|op| op.negatives.iter().map(|n| n.label.clone()))
            .collect();
        assert!(
            labels.iter().any(|l| l == "security:bad-bearer"),
            "missing bad-bearer probe; got {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "security:no-auth"),
            "missing no-auth probe; got {labels:?}"
        );
    }

    #[test]
    fn json_serialises_report() {
        let r = SelfTestReport {
            positive_pass: 1,
            positive_fail: 0,
            negative_caught: BTreeMap::new(),
            negative_missed: BTreeMap::new(),
            operations: vec![OperationResult {
                method: "GET".into(),
                path: "/x".into(),
                positive: Some(CaseOutcome {
                    label: "positive".into(),
                    expected_4xx: false,
                    actual_status: 200,
                    passed: true,
                }),
                negatives: Vec::new(),
            }],
        };
        let json = serde_json::to_value(&r).expect("serialises");
        assert_eq!(json["positive_pass"], serde_json::json!(1));
        assert_eq!(json["operations"][0]["positive"]["actual_status"], serde_json::json!(200));
    }
}
