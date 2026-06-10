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
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Round 23 (c-iii) — per-direction body cap when capturing
/// request/response payloads to `conformance-self-test-requests.jsonl`.
/// 16 KiB keeps a 1000-case run under ~32 MB even if every payload
/// fills the cap, while still preserving enough of a typical JSON body
/// (or a stack-trace error response) to debug from.
const CAPTURE_BODY_CAP_BYTES: usize = 16 * 1024;

/// Round 17.2 — cap on schema-driven negatives per operation. A spec
/// with 100 properties per body could produce hundreds of mutations
/// for a single operation; combined with thousands of operations
/// that's a runaway test matrix. 12 covers the highest-signal
/// mutations (type mismatch + required-removed + a few constraint
/// breaks) without exploding wall time on large specs.
const SCHEMA_MUTATION_CAP: usize = 12;

/// Round 25 (k) — content-type swap probes. For operations declaring a
/// JSON request body, each entry below produces one probe that lies
/// about Content-Type while keeping the JSON payload. A spec-compliant
/// server should respond 415 (or 400). Order matches the order
/// Srikanth listed in his round-23 reply: XML, YAML, multipart, and
/// the URL-encoded variant he added in round 24.
const CONTENT_TYPE_SWAP_VARIANTS: &[(&str, &str)] = &[
    ("application/xml", "request-body:content-type-mismatch:xml"),
    ("application/yaml", "request-body:content-type-mismatch:yaml"),
    ("multipart/form-data", "request-body:content-type-mismatch:multipart"),
    (
        "application/x-www-form-urlencoded",
        "request-body:content-type-mismatch:urlencoded",
    ),
];

/// Round 27 (k variant b) — embedded content payloads. Content-Type
/// stays `application/json` and the envelope IS valid JSON; we just
/// stuff a non-JSON snippet into a string field's value. The test
/// surfaces servers that try to parse string field contents (e.g.
/// XML-EE expanders, YAML loaders, urlencoded parsers) and crash on
/// the payload — a 5xx here is the finding. Label, payload pairs:
const EMBEDDED_CONTENT_VARIANTS: &[(&str, &str)] = &[
    ("request-body:embedded-content:xml", "<root><cmd>execute()</cmd></root>"),
    ("request-body:embedded-content:yaml", "key: value\n- item1\n- item2"),
    (
        "request-body:embedded-content:multipart",
        "--boundary\r\nContent-Disposition: form-data; name=\"x\"\r\n\r\nval\r\n--boundary--",
    ),
    ("request-body:embedded-content:urlencoded", "a=1&b=2&c=hello%20world"),
];

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
    /// Round 18.1 — base path to prepend to every spec path. When the
    /// spec declares `/users` and the deployed API is served under
    /// `/api`, `--base-path /api` should make the self-test hit
    /// `https://target/api/users` instead of `https://target/users`.
    /// Pre-fix this was ignored entirely and every operation 404'd
    /// (Srikanth's vCenter run on 0.3.152: 1275 positives, 1275 4xx).
    pub base_path: Option<String>,
    /// Round 18.5 — local source IPs to bind outgoing requests to.
    /// Each IP must already be assigned to an interface on the host.
    /// Operations round-robin through the resulting client pool.
    pub source_ips: Vec<IpAddr>,
    /// Round 18.5 — fake source IPs to advertise via forwarded-IP
    /// headers (used to exercise GEODB lookup at the destination).
    /// Rotated per operation.
    pub geo_source_ips: Vec<IpAddr>,
    /// Which forwarded-IP header(s) to populate when `geo_source_ips`
    /// is non-empty. Empty → no-op; default below sets the standard
    /// three-header set.
    pub geo_source_headers: Vec<String>,
    /// Round 23 (c-iii) — when `Some`, every probe captures method, URL,
    /// request headers/body and response status/headers/body into this
    /// sink. Caller drains it after `run_self_test` and writes
    /// `conformance-self-test-requests.jsonl`. None → no capture (zero
    /// extra allocations on the hot path).
    pub capture: Option<Arc<Mutex<Vec<CaseCapture>>>>,
    /// Round 25 — when true, validate every probe's response body
    /// against the spec's response schema for the actual status
    /// returned (closes round 21.3 / Srikanth's a2 / a3 ask). The
    /// validation result lands in `CaseCapture::response_schema_error`
    /// (None → matched, or no schema for that status). Default false:
    /// JSON-Schema validation of large response bodies adds wall-clock
    /// time and the user has to opt in.
    pub validate_response_schemas: bool,
}

/// Round 23 (c-iii) — one captured request/response pair, one per
/// probe (positive or negative). Serialised as a JSON line in
/// `conformance-self-test-requests.jsonl`. Headers are kept as
/// `BTreeMap` for stable ordering. Bodies are truncated to
/// `CAPTURE_BODY_CAP_BYTES`; `*_truncated` flags whether more was
/// dropped.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaseCapture {
    pub label: String,
    pub method: String,
    pub url: String,
    pub request_headers: BTreeMap<String, String>,
    pub request_body: Option<String>,
    pub request_body_truncated: bool,
    pub response_status: u16,
    pub response_headers: BTreeMap<String, String>,
    pub response_body: Option<String>,
    pub response_body_truncated: bool,
    pub error: Option<String>,
    /// Round 25 — when `validate_response_schemas` is on and the spec
    /// declares a schema for `response_status`, this carries the
    /// validation message (or None when the body matched, or no schema
    /// was declared for that status). Serialised verbatim in the JSONL
    /// and rendered in the HTML viewer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_schema_error: Option<String>,
    /// Round 28 — Srikanth's "Is it possible to put expected response
    /// code status in both jsonl and jsonl report" ask. Human-readable
    /// expected status range: `"2xx-3xx"` for positive probes,
    /// `"4xx"` for negatives. Lets users `jq` for misses
    /// (`.response_status as $s | .expected_status_range == "4xx"
    /// and ($s < 400 or $s >= 500)`) and powers the HTML viewer's
    /// "show mismatches only" filter.
    #[serde(default)]
    pub expected_status_range: String,
}

impl Default for SelfTestConfig {
    fn default() -> Self {
        Self {
            target_url: "http://localhost:3000".into(),
            skip_tls_verify: false,
            timeout: Duration::from_secs(15),
            extra_headers: Vec::new(),
            delay_between_requests: Duration::from_millis(0),
            base_path: None,
            source_ips: Vec::new(),
            geo_source_ips: Vec::new(),
            geo_source_headers: default_geo_source_headers(),
            capture: None,
            validate_response_schemas: false,
        }
    }
}

/// Truncate `body` to `CAPTURE_BODY_CAP_BYTES` on a UTF-8 boundary,
/// returning the trimmed string and whether truncation occurred. Used
/// for both request and response bodies in the capture sink.
fn truncate_body_for_capture(body: &str) -> (String, bool) {
    if body.len() <= CAPTURE_BODY_CAP_BYTES {
        return (body.to_string(), false);
    }
    let mut end = CAPTURE_BODY_CAP_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    (body[..end].to_string(), true)
}

/// Default forwarded-IP header set. Covers the three conventions a
/// real GEODB front-end is likely to read in this order of
/// preference: Cloudflare (`CF-Connecting-IP`), Akamai/CloudFront
/// (`True-Client-IP`), then the de-facto standard
/// `X-Forwarded-For`. Override via `--geo-source-header` to test a
/// specific stack.
pub fn default_geo_source_headers() -> Vec<String> {
    vec![
        "X-Forwarded-For".to_string(),
        "True-Client-IP".to_string(),
        "CF-Connecting-IP".to_string(),
    ]
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

    /// Round 18.1 — detect the "self-test target is misconfigured"
    /// case where every positive failed with the *same* status code.
    /// The classic example: `--base-path /api` was forgotten so every
    /// request hits a path the server doesn't know and returns 404.
    /// Pre-warning, the user saw all-green negative buckets (because
    /// "missing route" 404s look like "validator rejected") and no
    /// indication that the run was meaningless. Returns Some(status)
    /// when ≥10 positives all failed with the same status, else None.
    pub fn detect_target_misconfiguration(&self) -> Option<u16> {
        if self.positive_pass > 0 || self.positive_fail < 10 {
            return None;
        }
        let mut seen: Option<u16> = None;
        for op in &self.operations {
            let Some(p) = &op.positive else {
                continue;
            };
            if p.passed {
                return None;
            }
            match seen {
                None => seen = Some(p.actual_status),
                Some(s) if s != p.actual_status => return None,
                _ => {}
            }
        }
        seen
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
    // Round 18.5 — build a client pool when `source_ips` is set,
    // one reqwest::Client per IP, each bound to its local address.
    // Operations round-robin through the pool. Empty pool → single
    // default client (the pre-18.5 behaviour).
    let clients = build_client_pool(config)?;
    let client_cursor = AtomicUsize::new(0);
    let geo_cursor = AtomicUsize::new(0);

    let mut report = SelfTestReport::default();
    for op in operations {
        let client_idx = client_cursor.fetch_add(1, Ordering::Relaxed) % clients.len();
        let client = &clients[client_idx];
        let geo_ip = if config.geo_source_ips.is_empty() {
            None
        } else {
            let idx = geo_cursor.fetch_add(1, Ordering::Relaxed) % config.geo_source_ips.len();
            Some(config.geo_source_ips[idx])
        };
        let result = test_operation(client, config, op, geo_ip).await;
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

/// Round 18.5 — append GEODB forwarded-IP headers to the
/// operation's declared headers. Returns the original vec untouched
/// when `geo_ip` is None or `geo_headers` is empty.
///
/// If the operation already declares one of the geo headers (rare
/// but legal), we keep the operation's value — the caller's spec
/// wins.
fn effective_op_headers(
    base: &[(String, String)],
    geo_ip: Option<IpAddr>,
    geo_headers: &[String],
) -> Vec<(String, String)> {
    let mut out = base.to_vec();
    let Some(ip) = geo_ip else {
        return out;
    };
    let value = ip.to_string();
    for h in geo_headers {
        // Case-insensitive duplicate check: don't override the
        // spec's own declared value for the header.
        if out.iter().any(|(k, _)| k.eq_ignore_ascii_case(h)) {
            continue;
        }
        out.push((h.clone(), value.clone()));
    }
    out
}

/// Round 18.5 — build a pool of reqwest clients, one per declared
/// source IP. Empty `source_ips` → a single default client.
///
/// The OS must already have each `source_ip` assigned to an
/// interface; reqwest's `.local_address()` issues a `bind()` syscall
/// at connect time, so an IP the kernel doesn't recognise surfaces
/// as `EADDRNOTAVAIL` at request time, not at builder time.
fn build_client_pool(config: &SelfTestConfig) -> Result<Vec<Client>, reqwest::Error> {
    let make = |bind: Option<IpAddr>| -> Result<Client, reqwest::Error> {
        let mut builder = Client::builder().timeout(config.timeout);
        if config.skip_tls_verify {
            builder = builder.danger_accept_invalid_certs(true);
        }
        if let Some(addr) = bind {
            builder = builder.local_address(addr);
        }
        builder.build()
    };
    if config.source_ips.is_empty() {
        Ok(vec![make(None)?])
    } else {
        config.source_ips.iter().map(|ip| make(Some(*ip))).collect()
    }
}

async fn test_operation(
    client: &Client,
    config: &SelfTestConfig,
    op: &AnnotatedOperation,
    geo_ip: Option<IpAddr>,
) -> OperationResult {
    // Round 25 — track the sink length BEFORE we run any probes for
    // this operation, so that after the probes finish we can mutate
    // exactly the entries that belong to this op (the capture sink is
    // shared but `run_self_test` iterates operations sequentially).
    // Used by the response-schema validation pass below.
    let sink_start = config.capture.as_ref().and_then(|s| s.lock().ok().map(|g| g.len()));

    let url = build_url_with_base(
        &config.target_url,
        config.base_path.as_deref(),
        &op.path,
        &op.path_params,
    );
    let method = Method::from_bytes(op.method.to_uppercase().as_bytes()).unwrap_or(Method::GET);

    // Round 18.5 — pre-compute the operation's effective headers
    // with the geo source IP baked in. Doing it once here keeps the
    // per-case `send_case` calls below unchanged. When `geo_ip` is
    // None the result equals `op.header_params`.
    let op_headers = effective_op_headers(&op.header_params, geo_ip, &config.geo_source_headers);

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
        op_headers.clone(),
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
                op_headers.clone(),
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
                op_headers.clone(),
            )
            .await,
        );

        // Round 25 (k) — content-type swap probes.
        //
        // For operations declaring `application/json` request bodies, send
        // the SAME json payload (or a synthesised one) under four other
        // content types: `application/xml`, `application/yaml`,
        // `multipart/form-data`, `application/x-www-form-urlencoded`.
        // The spec says the endpoint accepts only JSON, so a strict server
        // should respond 415 Unsupported Media Type (or 400 if it tries
        // to parse and fails). A 2xx means the server is accepting
        // payloads outside its declared content negotiation, which is the
        // failure mode behind a lot of "we crashed on a malformed XML
        // upload" incidents.
        //
        // Variant (a) of Srikanth's round-23 g ask: lie about the
        // Content-Type header. The body shape is honest JSON; only the
        // header is swapped. Variant (b) (JSON envelope with embedded
        // non-JSON field values) is deferred to round 26 because it
        // requires a schema-aware field walker.
        if op
            .request_body_content_type
            .as_deref()
            .map(|ct| ct.contains("json"))
            .unwrap_or(false)
        {
            let payload = op.sample_body.as_deref().unwrap_or("{}");
            for (ct, label) in CONTENT_TYPE_SWAP_VARIANTS {
                negatives.push(
                    send_case_with_extra(
                        client,
                        config,
                        method.clone(),
                        &url,
                        label,
                        true,
                        Some(payload),
                        op.query_params.clone(),
                        // Strip any Content-Type already on the operation
                        // headers (the spec's positive value) so the
                        // probe's value is the only one the server sees.
                        op_headers
                            .iter()
                            .filter(|(k, _)| !k.eq_ignore_ascii_case("content-type"))
                            .cloned()
                            .collect(),
                        // The wrong Content-Type rides on `extra_headers`
                        // so it lands AFTER `send_case_with_extra`'s
                        // unconditional `application/json` insertion in
                        // request-body mode. Actually `send_case_with_extra`
                        // only sets Content-Type when a body is present
                        // AND there's no manual override; passing the
                        // override here wins because reqwest preserves
                        // the last-set header value.
                        vec![("Content-Type".to_string(), (*ct).to_string())],
                    )
                    .await,
                );
            }

            // Round 27 (k variant b) — embedded non-JSON content
            // inside a valid JSON envelope. Content-Type stays
            // application/json (honest) and the body parses as JSON;
            // only the string-valued payload changes. We expect 2xx-3xx
            // because the envelope is spec-shape, so the probe surfaces
            // servers that crash (5xx) trying to parse the embedded
            // snippet as XML/YAML/etc. A 4xx is also a finding because
            // it usually means the server's pattern/format validator
            // tripped on the payload contents, but the user can decide
            // from the JSONL whether that's a bug or correct narrow-
            // string-field behaviour.
            for (label, snippet) in EMBEDDED_CONTENT_VARIANTS {
                let payload = op.sample_body.as_deref().unwrap_or("{}");
                let body = embed_payload_in_first_string_field(payload, snippet);
                negatives.push(
                    send_case(
                        client,
                        config,
                        method.clone(),
                        &url,
                        label,
                        // expected_4xx=false: any non-2xx is a probe
                        // failure. 5xx in particular is "server panicked
                        // on the embedded content".
                        false,
                        Some(&body),
                        op.query_params.clone(),
                        op_headers.clone(),
                    )
                    .await,
                );
            }
        }

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
                            // Round 24 (f) — was `op.header_params`, which
                            // skipped the geo-IP header. Use `op_headers`
                            // so the geo IP rides with the negative probe
                            // too (positive vs negative coverage must be
                            // symmetric, otherwise a GEODB front-end sees
                            // the rotating IP only on positives).
                            op_headers.clone(),
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
                // Round 24 (f) — see schema-mutation note above. Use
                // `op_headers` (carries geo IP) instead of bare
                // `op.header_params`.
                op_headers.clone(),
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
            // Round 18.1 — honour `base_path` here too, otherwise the
            // probe URL differs from the positive case and the
            // resulting 404 is misattributed to "bad path param".
            let bad_url = build_url_with_base(
                &config.target_url,
                config.base_path.as_deref(),
                &url_with_placeholder,
                &[],
            );
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
                    op_headers.clone(),
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
                op_headers.clone(),
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
        // Round 24 (f) — security probes build req_headers from
        // `op.header_params` directly (we need the stripped-auth
        // variant), so the geo-IP header doesn't ride along
        // automatically. Append it here so a GEODB / WAF in front
        // of the auth layer still sees the rotating source IP.
        if let Some(ip) = geo_ip {
            let ip_str = ip.to_string();
            for h in &config.geo_source_headers {
                let already = req_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(h));
                if !already {
                    req_headers.push((h.clone(), ip_str.clone()));
                }
            }
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
        // Round 24 (f) — start from `op_headers` (so the geo IP rides
        // along) and only strip the first OPERATION-declared header.
        // Slicing past `op.header_params.len()` would otherwise risk
        // dropping the geo header itself; `op_headers` is built as
        // `op.header_params ++ geo` so index 0 is always operational.
        let mut h = op_headers.clone();
        if !h.is_empty() {
            h.remove(0);
        }
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

    // (w) Round 17.5 — OWASP/WAF unification.
    //
    // Pull one canonical payload per OWASP category from the existing
    // `SecurityPayloads` library and emit an injection probe per
    // category. Targets in priority order: (1) substitute the first
    // query param's value, (2) substitute the first string field of
    // the positive JSON body, (3) skip if neither is available.
    //
    // Label format `owasp:<category>`, so the existing
    // `negative_caught` / `negative_missed` rollup groups all OWASP
    // findings under one `owasp` bucket. Expected 4xx (server should
    // reject malicious input). A 5xx is a hard finding (server
    // crashed on the payload); a 2xx is a soft finding (input passed
    // through unfiltered — may or may not be a real vuln).
    //
    // Bounded: at most one probe per category (7 categories total).
    // Skips the operation entirely if no injection target is
    // available — open GET endpoints with no params get zero OWASP
    // probes, no false signal.
    for probe in build_owasp_probes(op) {
        negatives.push(
            send_case(
                client,
                config,
                method.clone(),
                &url,
                &probe.label,
                true,
                probe.body.as_deref(),
                probe.query,
                // Round 24 (f) — OWASP injection probes must also
                // carry the geo IP, otherwise a WAF / GEODB rule
                // tuned to a specific source IP would silently let
                // them through.
                op_headers.clone(),
            )
            .await,
        );
    }

    // Round 25 — response-body shape validation pass. For each capture
    // this op pushed onto the sink, look up the spec's schema for the
    // actual response status and validate. Result lands in
    // `response_schema_error` (Some(message) on failure, None on
    // pass or no-schema-for-this-status). Runs only when the user
    // opted in AND capture is on (we need the body).
    if config.validate_response_schemas {
        if let (Some(sink), Some(start)) = (config.capture.as_ref(), sink_start) {
            if !op.response_schemas.is_empty() {
                if let Ok(mut guard) = sink.lock() {
                    let end = guard.len();
                    for i in start..end {
                        let Some(entry) = guard.get_mut(i) else {
                            continue;
                        };
                        let Some(body) = entry.response_body.as_deref() else {
                            continue;
                        };
                        let Some(schema) = op.response_schemas.get(&entry.response_status) else {
                            continue;
                        };
                        entry.response_schema_error = validate_body_against_schema(body, schema);
                    }
                }
            }
        }
    }

    OperationResult {
        method: op.method.clone(),
        path: op.path.clone(),
        positive: Some(positive),
        negatives,
    }
}

/// Round 25 — validate a JSON body string against an OpenAPI response
/// schema (already converted to a `serde_json::Value`). Returns
/// `Some(message)` describing the first violation, or `None` on a
/// clean pass / non-JSON body / schema-build failure (in which case
/// the absence of an error means "we didn't have anything to compare
/// against", not "passed"; the caller-side semantics treat absence as
/// success because that's what the user sees as silence).
/// Round 27 (k variant b) — return a JSON body string identical to
/// `sample` except that the first string-valued leaf has been
/// replaced with `snippet`. Walks objects depth-first and stops at
/// the first string. If `sample` is not parseable JSON, or has no
/// string fields, falls back to wrapping the snippet under a `data`
/// key so the probe still has a body to send: `{"data": <snippet>}`.
/// The result is always valid JSON ready for `application/json`.
fn embed_payload_in_first_string_field(sample: &str, snippet: &str) -> String {
    let mut parsed: serde_json::Value = match serde_json::from_str(sample) {
        Ok(v) => v,
        Err(_) => return format!(r#"{{"data":{}}}"#, json_quote(snippet)),
    };
    if !replace_first_string(&mut parsed, snippet) {
        return format!(r#"{{"data":{}}}"#, json_quote(snippet));
    }
    serde_json::to_string(&parsed)
        .unwrap_or_else(|_| format!(r#"{{"data":{}}}"#, json_quote(snippet)))
}

/// Helper for `embed_payload_in_first_string_field`: recursively
/// walk the value and replace the FIRST string leaf encountered.
/// Returns true when a replacement happened. Honors document order
/// for objects (BTreeMap-backed `serde_json::Map` iterates in
/// insertion order) so the choice of which field to mutate is
/// stable across runs.
fn replace_first_string(v: &mut serde_json::Value, snippet: &str) -> bool {
    match v {
        serde_json::Value::String(s) => {
            *s = snippet.to_string();
            true
        }
        serde_json::Value::Object(map) => {
            for (_k, child) in map.iter_mut() {
                if replace_first_string(child, snippet) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => {
            for child in arr.iter_mut() {
                if replace_first_string(child, snippet) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Helper for `embed_payload_in_first_string_field`'s fallback: take
/// an arbitrary string and quote it for embedding inside a JSON
/// literal. `serde_json::to_string(&value)` handles escaping
/// correctly for unicode + control chars + quotes.
fn json_quote(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn validate_body_against_schema(body: &str, schema: &serde_json::Value) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let validator = jsonschema::validator_for(schema).ok()?;
    let mut errors = validator.iter_errors(&parsed);
    let first = errors.next()?;
    // Round 28 — Srikanth on 0.3.170 wanted the message to show the
    // actual expected schema alongside the kind label so it reads as
    // "expected schema {...} but got <kind>". We emit a compact JSON
    // serialisation of the schema as a suffix; the kind label still
    // names what went wrong in plain English for quick scanning.
    // Round 26 — Srikanth on 0.3.169: the prior `format!("{:?}", first.kind)
    // .split('(').next()` produced "Type { kind: Single" (broken Rust
    // syntax, mismatched braces). Switch to the human-readable mapping
    // already used in executor.rs: handle the common kinds (Type,
    // Required, AdditionalProperties, Enum, MinLength, MaxLength,
    // Minimum, Maximum, Pattern) explicitly; fall back to the
    // jsonschema crate's Display impl on the error (which produces
    // something like "{...} is not of type \"string\"") for the long
    // tail. Combined with `at <instance-path>` for the field location.
    let path = first.instance_path.to_string();
    let path = if path.is_empty() { "/" } else { path.as_str() };
    // Round 31 — Srikanth on 0.3.174 hit the vCenter case where the
    // error is "required field missing: comment" but the printed
    // schema was the WHOLE parent object schema (with descriptions of
    // every property), not just the missing field's sub-schema. The
    // jsonschema crate emits `Required` errors with
    // `instance_path == /` (the parent), so the round-30 sub-schema
    // walker had no extra info to focus the suffix. Carry the missing
    // property name out of the kind match so we can descend one more
    // step into `properties[property]` for the printed schema.
    let mut required_property: Option<String> = None;
    let kind_msg: String = match &first.kind {
        jsonschema::error::ValidationErrorKind::Type { kind } => {
            // `kind` is `TypeKind::Single(JsonType)` or
            // `TypeKind::Multiple(JsonTypeSet)`. `JsonType` has its
            // own `Display` impl ("string", "object", etc.).
            match kind {
                jsonschema::error::TypeKind::Single(t) => format!("expected type {t}"),
                jsonschema::error::TypeKind::Multiple(_) => "expected one of multiple types".into(),
            }
        }
        jsonschema::error::ValidationErrorKind::Required { property } => {
            // `property.to_string()` returns the Display of the JSON
            // value, which for a string is `"name"` (with quotes).
            // Strip them for the lookup; keep them in the human message.
            let raw = property.to_string();
            let unquoted = raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(&raw)
                .to_string();
            required_property = Some(unquoted);
            format!("required field missing: {property}")
        }
        jsonschema::error::ValidationErrorKind::AdditionalProperties { unexpected } => {
            format!("unexpected additional properties: {unexpected:?}")
        }
        jsonschema::error::ValidationErrorKind::Enum { options } => {
            format!("value not in allowed enum: {options}")
        }
        jsonschema::error::ValidationErrorKind::MinLength { limit } => {
            format!("string shorter than min length ({limit})")
        }
        jsonschema::error::ValidationErrorKind::MaxLength { limit } => {
            format!("string longer than max length ({limit})")
        }
        jsonschema::error::ValidationErrorKind::Minimum { limit } => {
            format!("value below minimum ({limit})")
        }
        jsonschema::error::ValidationErrorKind::Maximum { limit } => {
            format!("value above maximum ({limit})")
        }
        jsonschema::error::ValidationErrorKind::Pattern { pattern } => {
            format!("value did not match pattern {pattern}")
        }
        // Long tail: lean on jsonschema's Display impl, which is the
        // built-in human-readable error message ("X is not of type Y").
        // Strip trailing newlines so the JSONL line stays one line.
        _ => first.to_string().trim().to_string(),
    };
    // Round 30 — Srikanth on 0.3.173 asked how a deeper nested mismatch
    // reads. The prior output printed the WHOLE top-level schema even for
    // a single-field mismatch, which buried the actual constraint that
    // failed. Walk the instance pointer through the schema's properties
    // chain and print the most specific sub-schema we can find. Falls
    // back to the full schema for paths the walker can't resolve
    // (additionalProperties, oneOf, allOf, $ref un-resolved, etc.).
    let mut focused_schema = sub_schema_at_pointer(schema, path).unwrap_or_else(|| schema.clone());
    // Round 31 — for Required errors, descend one more step into
    // `properties[<missing>]` so the printed schema is the missing
    // field's own constraint, not the whole parent.
    if let Some(prop_name) = required_property.as_ref() {
        if let Some(prop_schema) =
            focused_schema.get("properties").and_then(|p| p.get(prop_name.as_str()))
        {
            focused_schema = prop_schema.clone();
        }
    }
    let schema_str = serde_json::to_string(&focused_schema).unwrap_or_else(|_| "<schema>".into());
    let schema_str = if schema_str.len() > 300 {
        format!("{}...", &schema_str[..300])
    } else {
        schema_str
    };
    // Round 29 — Srikanth on 0.3.172 was confused by `at /:` thinking
    // it referenced the URL path; it's actually a JSON pointer into
    // the RESPONSE BODY. Reword so that's unambiguous: explicit
    // "response body" prefix and a human label for the root case.
    let location = if path == "/" {
        "response body root".to_string()
    } else {
        format!("response body at {path}")
    };
    Some(format!("{location}: {kind_msg}; expected schema {schema_str}"))
}

/// Round 30 — walk a JSON-Pointer-style instance path through a JSON
/// Schema and return the sub-schema describing the value at that
/// position. For path `/name/age` on
/// `{"properties":{"name":{"properties":{"age":{"type":"integer"}}}}}`
/// returns `{"type":"integer"}`. Returns `None` for paths the walker
/// can't follow (array indices into `items` with no per-index schema,
/// `additionalProperties`, `oneOf`/`allOf`, unresolved `$ref`); callers
/// should fall back to the full schema in that case.
fn sub_schema_at_pointer(schema: &serde_json::Value, pointer: &str) -> Option<serde_json::Value> {
    if pointer.is_empty() || pointer == "/" {
        return Some(schema.clone());
    }
    let mut current = schema;
    for seg in pointer.trim_start_matches('/').split('/') {
        let unescaped = seg.replace("~1", "/").replace("~0", "~");
        if let Some(props) = current.get("properties") {
            if let Some(sub) = props.get(&unescaped) {
                current = sub;
                continue;
            }
        }
        if let Some(items) = current.get("items") {
            if items.is_object() {
                current = items;
                continue;
            }
        }
        return None;
    }
    Some(current.clone())
}

/// Round 17.5 — one OWASP injection probe to send.
#[derive(Debug, Clone)]
struct OwaspProbe {
    label: String,
    body: Option<String>,
    query: Vec<(String, String)>,
}

/// Build one OWASP probe per `SecurityCategory` for `op`. Targets the
/// first query param if any, else the first string field of the
/// positive JSON body. Returns empty if neither target is available.
fn build_owasp_probes(op: &AnnotatedOperation) -> Vec<OwaspProbe> {
    use crate::security_payloads::{SecurityCategory, SecurityPayloads};

    let categories = [
        SecurityCategory::SqlInjection,
        SecurityCategory::Xss,
        SecurityCategory::CommandInjection,
        SecurityCategory::PathTraversal,
        SecurityCategory::Ssti,
        SecurityCategory::LdapInjection,
        SecurityCategory::Xxe,
    ];

    // Pick an injection target ONCE per operation; reuse it across
    // categories. (A single op gets up to 7 probes — one per category
    // — all attacking the same field.)
    let injection_target = pick_injection_target(op);
    let Some(target) = injection_target else {
        return Vec::new();
    };

    let mut probes = Vec::new();
    for cat in categories {
        // Take the *first* payload from each category. The
        // collection's first entry is the canonical low-risk
        // representative; later entries include time-based / blind
        // probes that aren't useful as a one-shot rejection test.
        let Some(payload) = SecurityPayloads::get_by_category(cat).into_iter().next() else {
            continue;
        };
        let mut query = op.query_params.clone();
        let mut body = op.sample_body.clone();
        match &target {
            InjectionTarget::Query(idx) => {
                if let Some(slot) = query.get_mut(*idx) {
                    slot.1 = payload.payload.clone();
                }
            }
            InjectionTarget::BodyStringField(field) => {
                body = inject_into_body_field(body.as_deref(), field, &payload.payload);
            }
        }
        probes.push(OwaspProbe {
            label: format!("owasp:{}", cat),
            body,
            query,
        });
    }
    probes
}

#[derive(Debug, Clone)]
enum InjectionTarget {
    Query(usize),
    BodyStringField(String),
}

fn pick_injection_target(op: &AnnotatedOperation) -> Option<InjectionTarget> {
    if !op.query_params.is_empty() {
        return Some(InjectionTarget::Query(0));
    }
    let sample = op.sample_body.as_deref()?;
    let parsed: serde_json::Value = serde_json::from_str(sample).ok()?;
    let obj = parsed.as_object()?;
    for (k, v) in obj {
        if v.is_string() {
            return Some(InjectionTarget::BodyStringField(k.clone()));
        }
    }
    None
}

/// Replace the value of `field` in a JSON-object body with `payload`.
/// Returns the mutated body as a JSON string. Returns `None` if the
/// body doesn't parse as a JSON object.
fn inject_into_body_field(body: Option<&str>, field: &str, payload: &str) -> Option<String> {
    let raw = body?;
    let mut parsed: serde_json::Value = serde_json::from_str(raw).ok()?;
    let obj = parsed.as_object_mut()?;
    obj.insert(field.to_string(), serde_json::json!(payload));
    serde_json::to_string(&parsed).ok()
}

#[allow(clippy::too_many_arguments)]
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
    let mut req = client.request(method.clone(), url);
    let mut capture_headers: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in &query {
        req = req.query(&[(k.as_str(), v.as_str())]);
    }
    // Round 28 — reqwest's `.header(k, v)` APPENDS rather than replaces
    // (.headers().insert() would replace but isn't on the builder).
    // The previous round-25 fix relied on "last-write-wins" semantics
    // that don't exist; for content-type-swap probes the request went
    // out with BOTH `Content-Type: application/json` AND `Content-Type:
    // application/xml`, and axum's `Json<>` extractor picked the JSON
    // one and accepted, so the server-side validator never saw the
    // mismatch. Build a `HeaderMap` ourselves so the override
    // replaces the body-block default exactly once.
    let mut final_headers: reqwest::header::HeaderMap = reqwest::header::HeaderMap::new();
    if let Some(_b) = body {
        if let Ok(v) = reqwest::header::HeaderValue::from_str("application/json") {
            final_headers.insert(reqwest::header::CONTENT_TYPE, v);
        }
        capture_headers.insert("Content-Type".to_string(), "application/json".to_string());
    }
    for (k, v) in &headers {
        if let (Ok(hn), Ok(hv)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            final_headers.insert(hn, hv);
        }
        capture_headers.insert(k.clone(), v.clone());
    }
    for (k, v) in &extra_headers {
        if let (Ok(hn), Ok(hv)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            final_headers.insert(hn, hv);
        }
        capture_headers.insert(k.clone(), v.clone());
    }
    if let Some(b) = body {
        req = req.body(b.to_string());
    }
    req = req.headers(final_headers);
    let (actual_status, response_capture) = match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if let Some(sink) = &config.capture {
                let resp_headers: BTreeMap<String, String> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let text = resp.text().await.unwrap_or_default();
                let (rb, truncated) = truncate_body_for_capture(&text);
                (status, Some((Some((rb, truncated)), resp_headers, None, sink.clone())))
            } else {
                (status, None)
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if let Some(sink) = &config.capture {
                (0, Some((None, BTreeMap::new(), Some(err_str), sink.clone())))
            } else {
                (0, None)
            }
        }
    };
    let passed = if expected_4xx {
        (400..500).contains(&actual_status)
    } else {
        (200..400).contains(&actual_status)
    };
    if let Some((resp_body, resp_headers, error, sink)) = response_capture {
        let (request_body, request_body_truncated) = match body {
            Some(b) => {
                let (rb, t) = truncate_body_for_capture(b);
                (Some(rb), t)
            }
            None => (None, false),
        };
        let (response_body, response_body_truncated) = match resp_body {
            Some((rb, t)) => (Some(rb), t),
            None => (None, false),
        };
        let entry = CaseCapture {
            label: label.to_string(),
            method: method.to_string(),
            url: build_query_url(url, &query),
            request_headers: capture_headers,
            request_body,
            request_body_truncated,
            response_status: actual_status,
            response_headers: resp_headers,
            response_body,
            response_body_truncated,
            error,
            // Filled in by the per-operation validation pass after
            // every probe finishes; the capture itself is unaware of
            // the schema map.
            response_schema_error: None,
            // Round 28 — derive the expected range from the probe's
            // `expected_4xx` flag so the JSONL line and HTML viewer
            // can show mismatches without re-deriving on the read side.
            expected_status_range: if expected_4xx {
                "4xx".into()
            } else {
                "2xx-3xx".into()
            },
        };
        if let Ok(mut guard) = sink.lock() {
            guard.push(entry);
        }
    }
    CaseOutcome {
        label: label.to_string(),
        expected_4xx,
        actual_status,
        passed,
    }
}

// HTTP request shape needs all of these: client, config (for capture
// sink + extra headers), method, url, label (probe id), expected_4xx
// (pass/fail decision), body, query, headers. A struct wrapper would
// just move the arity from positional to field access without making
// the call sites clearer.
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
    // Forwarding to `send_case_with_extra` keeps the capture logic in
    // one place so request/response tracing can't drift between the
    // two entrypoints.
    send_case_with_extra(
        client,
        config,
        method,
        url,
        label,
        expected_4xx,
        body,
        query,
        headers,
        config.extra_headers.clone(),
    )
    .await
}

/// Round 23 (c-iii) — rebuild the query-stringified URL for capture so
/// the JSONL trace shows the URL that actually went over the wire
/// (reqwest applies `.query(..)` after the request URL string is
/// rendered, so capturing the raw `url` argument alone loses the
/// query params).
fn build_query_url(base: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return base.to_string();
    }
    let qs: String = query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    if base.contains('?') {
        format!("{base}&{qs}")
    } else {
        format!("{base}?{qs}")
    }
}

/// Substitute `{param}` placeholders in the spec path with their
/// sample values from `path_params`, then prepend `target_url`. Empty
/// values are kept as `{param}` so an upstream router still matches
/// the template — useful when `path_params` is empty and we want to
/// hit the same route the spec defines.
///
/// All current call sites went through `build_url_with_base` after
/// round 18.1, so this no-base-path helper is unused; keep it as the
/// documented shim for future external callers (one-arg simplification).
#[allow(dead_code)]
fn build_url(target: &str, path_template: &str, path_params: &[(String, String)]) -> String {
    build_url_with_base(target, None, path_template, path_params)
}

/// Round 18.1 — variant of `build_url` that takes a `base_path`
/// (e.g. `Some("/api")`). When set, prepends it to the spec path so a
/// spec declaring `/users` against a target served behind `/api`
/// resolves to `<target>/api/users`. `base_path` is normalised: leading
/// `/` is auto-added, trailing `/` is stripped.
fn build_url_with_base(
    target: &str,
    base_path: Option<&str>,
    path_template: &str,
    path_params: &[(String, String)],
) -> String {
    let mut url = path_template.to_string();
    for (name, value) in path_params {
        let placeholder = format!("{{{}}}", name);
        if !value.is_empty() {
            url = url.replace(&placeholder, value);
        }
    }
    let target = target.trim_end_matches('/');
    let prefix = match base_path {
        Some(bp) if !bp.is_empty() => {
            let trimmed = bp.trim_end_matches('/');
            if trimmed.starts_with('/') {
                trimmed.to_string()
            } else {
                format!("/{}", trimmed)
            }
        }
        _ => String::new(),
    };
    let path = if url.starts_with('/') {
        url
    } else {
        format!("/{url}")
    };
    format!("{target}{prefix}{path}")
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
            response_schemas: std::collections::BTreeMap::new(),
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

    /// Round 18.1 — a run where every positive 404s should be flagged
    /// as a likely target misconfiguration, not silently treated as a
    /// successful conformance run.
    #[test]
    fn detect_target_misconfiguration_when_all_positives_share_status() {
        let mut report = SelfTestReport {
            positive_pass: 0,
            positive_fail: 50,
            ..Default::default()
        };
        for i in 0..50 {
            report.operations.push(OperationResult {
                method: "GET".into(),
                path: format!("/r/{i}"),
                positive: Some(CaseOutcome {
                    label: "positive".into(),
                    expected_4xx: false,
                    actual_status: 404,
                    passed: false,
                }),
                negatives: Vec::new(),
            });
        }
        assert_eq!(report.detect_target_misconfiguration(), Some(404));
    }

    #[test]
    fn detect_target_misconfiguration_returns_none_when_some_pass() {
        let mut report = SelfTestReport {
            positive_pass: 5,
            positive_fail: 50,
            ..Default::default()
        };
        for i in 0..55 {
            report.operations.push(OperationResult {
                method: "GET".into(),
                path: format!("/r/{i}"),
                positive: Some(CaseOutcome {
                    label: "positive".into(),
                    expected_4xx: false,
                    actual_status: if i < 5 { 200 } else { 404 },
                    passed: i < 5,
                }),
                negatives: Vec::new(),
            });
        }
        assert_eq!(report.detect_target_misconfiguration(), None);
    }

    /// Round 18.1 — `--base-path /api` should prepend `/api` to
    /// every spec path. Pre-fix, the self-test ignored base_path and
    /// 404'd every positive when the deployed API was behind a path
    /// prefix.
    #[test]
    fn build_url_applies_base_path_when_present() {
        let url = build_url_with_base(
            "https://api.example.com",
            Some("/api"),
            "/users/{id}",
            &[("id".into(), "42".into())],
        );
        assert_eq!(url, "https://api.example.com/api/users/42");
    }

    /// Round 18.1 — base_path is normalised: missing leading slash
    /// gets one added, trailing slash is stripped, empty string is
    /// the same as None.
    #[test]
    fn build_url_normalises_base_path() {
        let no_slash = build_url_with_base("https://t", Some("api"), "/x", &[]);
        assert_eq!(no_slash, "https://t/api/x");
        let trailing = build_url_with_base("https://t", Some("/api/"), "/x", &[]);
        assert_eq!(trailing, "https://t/api/x");
        let empty = build_url_with_base("https://t", Some(""), "/x", &[]);
        assert_eq!(empty, "https://t/x");
        let none = build_url_with_base("https://t", None, "/x", &[]);
        assert_eq!(none, "https://t/x");
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

    /// Round 18.5 — when `geo_ip` is set, every default forwarded-
    /// IP header gets the IP appended (X-Forwarded-For,
    /// True-Client-IP, CF-Connecting-IP).
    #[test]
    fn effective_op_headers_appends_geo_ip_to_default_headers() {
        let ip: IpAddr = "203.0.113.42".parse().unwrap();
        let headers = effective_op_headers(
            &[("Accept".into(), "application/json".into())],
            Some(ip),
            &default_geo_source_headers(),
        );
        let names: Vec<&str> = headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"Accept"));
        assert!(names.contains(&"X-Forwarded-For"));
        assert!(names.contains(&"True-Client-IP"));
        assert!(names.contains(&"CF-Connecting-IP"));
        // Every geo header carries the same IP value.
        let geo_values: Vec<&str> =
            headers.iter().filter(|(k, _)| k != "Accept").map(|(_, v)| v.as_str()).collect();
        for v in geo_values {
            assert_eq!(v, "203.0.113.42");
        }
    }

    /// Round 18.5 — operations that already declare a forwarded-IP
    /// header (rare but legal — some specs hard-code one) keep their
    /// declared value; we don't clobber the spec.
    #[test]
    fn effective_op_headers_respects_spec_declared_header() {
        let ip: IpAddr = "203.0.113.99".parse().unwrap();
        let headers = effective_op_headers(
            &[("x-forwarded-for".into(), "10.0.0.1".into())],
            Some(ip),
            &["X-Forwarded-For".to_string()],
        );
        // The spec's lower-case value wins; we shouldn't add a
        // second X-Forwarded-For row that overrides it.
        let xff: Vec<&str> = headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("x-forwarded-for"))
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(xff, vec!["10.0.0.1"]);
    }

    /// Round 18.5 — None geo_ip and/or empty header list is a no-op.
    #[test]
    fn effective_op_headers_is_a_noop_without_geo_ip() {
        let base = vec![("Accept".into(), "json".into())];
        let h1 = effective_op_headers(&base, None, &default_geo_source_headers());
        assert_eq!(h1, base);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let h2 = effective_op_headers(&base, Some(ip), &[]);
        assert_eq!(h2, base);
    }

    /// Round 18.5 — empty `source_ips` builds a single default
    /// client; a non-empty list builds N clients each attempting to
    /// bind. We can't reliably test the actual bind on CI (no
    /// loopback aliases), but a loopback IP is always bind-able.
    #[test]
    fn build_client_pool_one_per_source_ip() {
        let mut cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        // Empty → one default client.
        assert_eq!(build_client_pool(&cfg).expect("default builds").len(), 1);
        // Non-empty → one per IP. Loopback bind is portable.
        cfg.source_ips = vec!["127.0.0.1".parse().unwrap()];
        assert_eq!(build_client_pool(&cfg).expect("bind loopback").len(), 1);
    }

    /// Round 18.5 — geo IPs round-robin across operations. Hits an
    /// unreachable target so we can inspect the case outcomes; the
    /// point is to confirm `op_headers` carried the geo IP through
    /// (CaseOutcome doesn't surface headers directly, so we just
    /// verify the run completes without panicking and the result
    /// shape is correct when source_ips is non-empty too).
    #[tokio::test]
    async fn run_self_test_with_geo_source_completes() {
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(200),
            geo_source_ips: vec![
                "203.0.113.1".parse().unwrap(),
                "203.0.113.2".parse().unwrap(),
            ],
            ..Default::default()
        };
        let ops = vec![
            op("GET", "/a", None, vec![], vec![], vec![]),
            op("GET", "/b", None, vec![], vec![], vec![]),
            op("GET", "/c", None, vec![], vec![], vec![]),
        ];
        let report = run_self_test(&ops, &cfg).await.expect("client builds");
        assert_eq!(report.operations.len(), 3);
    }

    /// Round 24 (f) — Srikanth saw the geo header on positive probes
    /// only; the four negative-probe call sites were passing
    /// `op.header_params` directly instead of `op_headers`, so the
    /// geo IP got dropped. This test runs a self-test that includes
    /// negative probes (uri-too-long, missing-query, etc.) under
    /// `--conformance-self-test-capture`, then asserts that EVERY
    /// captured probe (positive AND negative) carries one of the
    /// configured forwarded-IP headers.
    #[tokio::test]
    async fn geo_headers_present_on_every_probe_with_capture() {
        let sink: Arc<Mutex<Vec<CaseCapture>>> = Arc::new(Mutex::new(Vec::new()));
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(50),
            geo_source_ips: vec!["203.0.113.5".parse().unwrap()],
            capture: Some(sink.clone()),
            ..Default::default()
        };
        // An operation rich enough to trip several negative-probe
        // branches: header param (→ missing-header), query param
        // (→ missing-query), and a sample body (→ schema mutations
        // wouldn't fire without a schema, but uri-too-long always
        // does).
        let ops = vec![op(
            "GET",
            "/items",
            Some("{}"),
            vec![("id", "1")],
            vec![("X-Trace", "x")],
            vec![],
        )];
        let _ = run_self_test(&ops, &cfg).await.expect("client builds");
        let captures = sink.lock().unwrap();
        assert!(!captures.is_empty(), "self-test should record probes");
        // For every captured probe, at least one of the default geo
        // headers must be present and equal to the configured IP.
        let geo_headers: std::collections::HashSet<&str> =
            ["X-Forwarded-For", "True-Client-IP", "CF-Connecting-IP"].into_iter().collect();
        for c in captures.iter() {
            let has_geo = c
                .request_headers
                .iter()
                .any(|(k, v)| geo_headers.contains(k.as_str()) && v == "203.0.113.5");
            assert!(
                has_geo,
                "probe `{}` is missing the geo IP header; got headers: {:?}",
                c.label, c.request_headers
            );
        }
    }

    /// Round 25 (k) — operations with a JSON request body now get four
    /// content-type-swap probes (xml / yaml / multipart / urlencoded).
    /// Verify they:
    ///   1. fire only when the operation declares a JSON body
    ///   2. carry the wrong Content-Type the probe is testing for
    ///   3. don't fire on body-less operations
    #[tokio::test]
    async fn content_type_swap_probes_fire_for_json_bodies() {
        let sink: Arc<Mutex<Vec<CaseCapture>>> = Arc::new(Mutex::new(Vec::new()));
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(50),
            capture: Some(sink.clone()),
            ..Default::default()
        };
        let ops = vec![
            op("POST", "/users", Some("{\"name\":\"a\"}"), vec![], vec![], vec![]),
            op("GET", "/ping", None, vec![], vec![], vec![]),
        ];
        let _ = run_self_test(&ops, &cfg).await.expect("client builds");
        let captures = sink.lock().unwrap();

        let swap_labels: Vec<&str> = captures
            .iter()
            .filter(|c| c.label.starts_with("request-body:content-type-mismatch:"))
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(
            swap_labels.len(),
            4,
            "expected 4 content-type-swap probes (one per variant), got: {swap_labels:?}"
        );
        let expected_labels = [
            "request-body:content-type-mismatch:xml",
            "request-body:content-type-mismatch:yaml",
            "request-body:content-type-mismatch:multipart",
            "request-body:content-type-mismatch:urlencoded",
        ];
        for want in expected_labels {
            assert!(swap_labels.contains(&want), "missing swap probe `{want}`");
        }

        // Each swap probe must carry the wrong Content-Type it's
        // testing for — that's the whole point.
        for c in captures.iter() {
            let Some(suffix) = c.label.strip_prefix("request-body:content-type-mismatch:") else {
                continue;
            };
            let want_ct = match suffix {
                "xml" => "application/xml",
                "yaml" => "application/yaml",
                "multipart" => "multipart/form-data",
                "urlencoded" => "application/x-www-form-urlencoded",
                _ => continue,
            };
            let got_ct = c
                .request_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            assert_eq!(got_ct, want_ct, "swap probe `{}` sent wrong CT", c.label);
        }

        // The body-less operation must NOT produce content-type-swap
        // probes (no body → no content type to lie about).
        let body_less_swaps = captures
            .iter()
            .filter(|c| {
                c.label.starts_with("request-body:content-type-mismatch:")
                    && c.url.ends_with("/ping")
            })
            .count();
        assert_eq!(
            body_less_swaps, 0,
            "GET /ping has no request body; should not produce content-type-swap probes"
        );
    }

    /// Round 27 (k variant b) — Srikanth's round-23 follow-up on (k):
    /// JSON envelope with embedded non-JSON field values. For each
    /// JSON-body operation, four extra probes fire that send valid
    /// JSON with an XML/YAML/multipart/urlencoded snippet stuffed
    /// into a string field. Content-Type stays `application/json`;
    /// expected is 2xx-3xx (the body parses); a 5xx flags a server
    /// that crashed on the embedded content.
    #[tokio::test]
    async fn embedded_content_probes_fire_with_honest_content_type() {
        let sink: Arc<Mutex<Vec<CaseCapture>>> = Arc::new(Mutex::new(Vec::new()));
        let cfg = SelfTestConfig {
            target_url: "http://127.0.0.1:1".into(),
            timeout: Duration::from_millis(50),
            capture: Some(sink.clone()),
            ..Default::default()
        };
        let ops = vec![op(
            "POST",
            "/users",
            Some("{\"name\":\"alice\",\"age\":30}"),
            vec![],
            vec![],
            vec![],
        )];
        let _ = run_self_test(&ops, &cfg).await.expect("client builds");
        let captures = sink.lock().unwrap();
        let embedded: Vec<&CaseCapture> = captures
            .iter()
            .filter(|c| c.label.starts_with("request-body:embedded-content:"))
            .collect();
        assert_eq!(
            embedded.len(),
            4,
            "expected 4 embedded-content probes, got: {:?}",
            embedded.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        // Every embedded probe must carry the honest application/json
        // Content-Type (NOT lie like the variant-a content-type-swap
        // probes do) and a request body that still parses as JSON.
        for c in &embedded {
            let ct = c
                .request_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            assert!(
                ct.contains("application/json"),
                "embedded probe `{}` should keep Content-Type honest, got {ct}",
                c.label
            );
            let body = c.request_body.as_deref().unwrap_or("");
            assert!(
                serde_json::from_str::<serde_json::Value>(body).is_ok(),
                "embedded probe `{}` body should still be valid JSON, got: {body}",
                c.label
            );
        }
    }

    /// `embed_payload_in_first_string_field` walks objects depth-first
    /// and replaces only the FIRST string-valued leaf, leaving the
    /// surrounding structure intact.
    #[test]
    fn embed_payload_replaces_first_string_only() {
        let sample = r#"{"name":"alice","age":30,"tags":["admin","user"]}"#;
        let mutated = embed_payload_in_first_string_field(sample, "<x/>");
        let v: serde_json::Value = serde_json::from_str(&mutated).unwrap();
        assert_eq!(v["name"], serde_json::json!("<x/>"));
        // age stays an integer (not stringified by the mutation).
        assert_eq!(v["age"], serde_json::json!(30));
        // tags array's strings stay untouched (we only replace the
        // first encountered string leaf, depth-first).
        assert_eq!(v["tags"][0], serde_json::json!("admin"));
        assert_eq!(v["tags"][1], serde_json::json!("user"));
    }

    /// When the sample has NO string field, the helper falls back to
    /// `{"data": "<snippet>"}` so the probe still has something to
    /// POST. The fallback must produce valid JSON regardless of what
    /// characters the snippet contains.
    #[test]
    fn embed_payload_falls_back_when_no_string_field() {
        let no_strings = r#"{"a":1,"b":[2,3]}"#;
        let mutated = embed_payload_in_first_string_field(no_strings, "<x><y></y></x>");
        let v: serde_json::Value = serde_json::from_str(&mutated).unwrap();
        assert_eq!(v["data"], serde_json::json!("<x><y></y></x>"));
    }

    #[test]
    fn embed_payload_handles_invalid_json_sample() {
        let not_json = "garbage";
        let mutated = embed_payload_in_first_string_field(not_json, "a=1&b=2");
        let v: serde_json::Value = serde_json::from_str(&mutated).unwrap();
        assert_eq!(v["data"], serde_json::json!("a=1&b=2"));
    }

    /// Round 26 — Srikanth saw `at /: Type { kind: Single` in his
    /// 0.3.169 capture for the vCenter `infraprofile/configs` 202
    /// response (spec promised `type: string`, server returned a
    /// JSON object). The output was a broken-syntax debug string.
    /// This test reproduces his exact spec+body and asserts the
    /// message is readable.
    #[test]
    fn response_schema_error_message_is_readable() {
        let schema = serde_json::json!({"type": "string"});
        let body = r#"{"data":{},"id":"generated_id","status":"created"}"#;
        let err = validate_body_against_schema(body, &schema).expect("type-mismatch fires");
        // The message must NOT contain Rust debug syntax leftovers
        // ("Type { kind:", trailing "{" or "(" tokens). It SHOULD say
        // what type was expected.
        assert!(!err.contains("Type { kind"), "stale debug output: {err}");
        assert!(!err.contains("{ kind:"), "stale debug output: {err}");
        assert!(err.contains("string"), "should name expected type: {err}");
        // Round 29 — Srikanth on 0.3.172 was confused by `at /:`,
        // thinking it pointed to the URL path. The new format
        // explicitly says "response body root" for the root case
        // (and "response body at /<pointer>" for nested fields).
        assert!(
            err.contains("response body root"),
            "should label root explicitly so reader knows it's not the URL: {err}"
        );
        // Round 28 — Srikanth wanted the expected schema embedded
        // in the message so it reads as 'expected schema {"type":"string"}'.
        assert!(
            err.contains("expected schema") && err.contains("\"type\":\"string\""),
            "should include expected schema JSON: {err}"
        );
    }

    /// Round 29 — for non-root paths the format reads
    /// "response body at /name: ...". Catches the case where the
    /// root rewording accidentally dropped the JSON-pointer for
    /// nested fields.
    #[test]
    fn response_schema_error_uses_response_body_prefix_for_nested_paths() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {"name": {"type": "string"}}
        });
        let body = r#"{"name": 123}"#;
        let err = validate_body_against_schema(body, &schema).expect("type-mismatch fires");
        assert!(
            err.contains("response body at /name"),
            "nested path should read 'response body at /name': {err}"
        );
        assert!(!err.contains("response body root"), "wrong label for nested: {err}");
        // Round 30 — the "expected schema" suffix should be the
        // sub-schema at /name, not the entire object schema. Reader
        // shouldn't have to scan a 300-char object to find the
        // constraint that failed.
        assert!(
            err.contains(r#"expected schema {"type":"string"}"#),
            "should show only the /name sub-schema, not the full object: {err}"
        );
    }

    /// Round 30 — Srikanth asked how a deeper nested mismatch reads.
    /// Schema: `name.type` should be a string; body has it as a number.
    /// JSON pointer is `/name/type`.
    #[test]
    fn response_schema_error_uses_response_body_prefix_for_deep_nested_paths() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "object",
                    "properties": {"type": {"type": "string"}}
                }
            }
        });
        let body = r#"{"name": {"type": 123}}"#;
        let err = validate_body_against_schema(body, &schema).expect("type-mismatch fires");
        assert!(
            err.contains("response body at /name/type"),
            "deep nested path should read 'response body at /name/type': {err}"
        );
        // Round 30 — for deep paths the sub-schema is the leaf
        // {"type":"string"}, not the wrapping object schemas.
        assert!(
            err.contains(r#"expected schema {"type":"string"}"#),
            "should show only the /name/type leaf sub-schema: {err}"
        );
    }

    /// Round 30 — when the instance pointer can't be resolved through
    /// the schema's `properties` chain (e.g. additionalProperties hit),
    /// `sub_schema_at_pointer` returns None and the message falls back
    /// to the full schema. Verifies the fallback path is wired.
    #[test]
    fn sub_schema_at_pointer_falls_back_for_unresolvable_paths() {
        let schema = serde_json::json!({"type":"object","additionalProperties":true});
        // Walker can't resolve /unknown, so we get the full schema back.
        assert_eq!(
            sub_schema_at_pointer(&schema, "/unknown"),
            None,
            "unresolvable path should return None to trigger fallback"
        );
        // Root path returns the whole schema.
        assert_eq!(sub_schema_at_pointer(&schema, "/"), Some(schema.clone()));
        assert_eq!(sub_schema_at_pointer(&schema, ""), Some(schema));
    }

    #[test]
    fn response_schema_error_required_field_is_readable() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {"id": {"type": "integer"}}
        });
        let body = r#"{"other": 1}"#;
        let err = validate_body_against_schema(body, &schema).expect("required-missing fires");
        assert!(err.contains("required field missing"), "{err}");
        assert!(err.contains("id"), "{err}");
    }

    /// Round 31 — Srikanth's vCenter case on 0.3.174: the
    /// `Appliance.Recovery.Backup.SystemName.Archive.Info` schema has
    /// a multi-paragraph description and ~6 required fields, of which
    /// `comment` was missing in the response. Before this fix the
    /// printed schema was the WHOLE parent object schema (parent's
    /// description bleeding in, all sibling property schemas dumped)
    /// truncated to 300 chars; after the fix it's the missing field's
    /// own schema. Verifies (a) parent description is gone and
    /// (b) sibling property names don't appear in the message.
    #[test]
    fn response_schema_error_required_focuses_on_missing_field_only() {
        let schema = serde_json::json!({
            "description": "The Appliance.Recovery.Backup.SystemName.Archive.Info schema represents backup archive information.\n\nThis schema was added in vSphere API 6.7.",
            "type": "object",
            "required": ["comment", "location", "parts", "system_name", "timestamp", "version"],
            "properties": {
                "comment": {
                    "type": "string",
                    "description": "Custom comment added by the user for this backup."
                },
                "location": {"type": "string", "description": "Backup location URL."},
                "parts": {"type": "array", "items": {"type": "string"}},
                "system_name": {"type": "string"},
                "timestamp": {"type": "string", "format": "date-time"},
                "version": {"type": "string"}
            }
        });
        let body = r#"{"location":"x","parts":[],"system_name":"y","timestamp":"z","version":"v"}"#;
        let err = validate_body_against_schema(body, &schema).expect("required-missing fires");
        assert!(err.contains("required field missing: \"comment\""), "{err}");
        // Parent's description should not appear; only the `comment`
        // field's own description (if any) may.
        assert!(
            !err.contains("Appliance.Recovery.Backup"),
            "parent description should not bleed into focused schema: {err}"
        );
        // No sibling property names should appear in the focused schema
        // suffix.
        for sibling in ["location", "parts", "system_name", "timestamp", "version"] {
            assert!(
                !err.contains(&format!("\"{sibling}\"")),
                "sibling field {sibling} should not appear in focused schema: {err}"
            );
        }
    }

    #[test]
    fn response_schema_error_none_on_match() {
        let schema = serde_json::json!({"type": "string"});
        assert_eq!(validate_body_against_schema("\"hello\"", &schema), None);
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
