//! Per-endpoint send/received summary derived from the
//! `CaseCapture` JSONL sink.
//!
//! Issue #79 round 32 — Srikanth on 0.3.176: "HTML(Manual
//! Verification)/JSON(for Automation Verification) would help where we
//! show API Endpoint details. Something like:
//! `[GET/POST/PUT/...]: <send_request_count>, 2xx or 3xx or 4xx or 5xx
//! count separately Per end Point` and per-(method, path) request
//! body / response body length p95."
//!
//! The bench already records every request/response in
//! `conformance-self-test-requests.jsonl`. This module rolls them up
//! per (method, resolved-path) so a human (or `jq`) doesn't have to
//! re-aggregate from scratch. v1 groups by the resolved URL path
//! (everything after the host, minus the query string); a future
//! round can collapse to the spec's path template once we surface the
//! `op.path` template on each `CaseCapture` entry.
//!
//! Output:
//! - `conformance-per-endpoint.json` next to the existing
//!   `conformance-self-test.json` for automation.
//! - HTML section spliced into `conformance-report.html` for humans.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::self_test::CaseCapture;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerEndpointSummary {
    /// HTTP method, uppercase.
    pub method: String,
    /// Round 33 (#823) — spec path template (e.g. `/users/{id}`)
    /// pre path-param substitution. Falls back to the resolved URL
    /// path when the capture predates the template field.
    pub path: String,
    /// Round 33 (#823) — basename of the OpenAPI spec the probes for
    /// this endpoint came from. `None` for single-spec runs that didn't
    /// stamp a label, or for legacy captures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    pub sent: usize,
    pub status_2xx: usize,
    pub status_3xx: usize,
    pub status_4xx: usize,
    pub status_5xx: usize,
    /// Network errors (`response_status == 0`).
    pub errors: usize,
    /// Length stats on the captured REQUEST body (bytes). `None` when
    /// no request body was sent on any probe for this endpoint.
    pub request_body_len: Option<LenStats>,
    /// Length stats on the captured RESPONSE body (bytes). `None`
    /// when no captured response body had content.
    pub response_body_len: Option<LenStats>,
    /// Length stats on the resolved query string (raw bytes after
    /// `?`). `None` when no probe carried a query string for this
    /// endpoint.
    pub query_len: Option<LenStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LenStats {
    pub samples: usize,
    pub avg: f64,
    pub p50: u64,
    pub p95: u64,
    pub max: u64,
}

impl LenStats {
    fn from_samples(mut samples: Vec<u64>) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        samples.sort_unstable();
        let n = samples.len();
        let sum: u64 = samples.iter().sum();
        let avg = sum as f64 / n as f64;
        let pick = |q: f64| -> u64 {
            // Nearest-rank percentile, 1-indexed. Matches k6's
            // `http_req_duration{tag:p95}` calculation closely enough
            // for spot checks.
            let idx = (q * n as f64).ceil() as usize;
            let idx = idx.clamp(1, n) - 1;
            samples[idx]
        };
        Some(LenStats {
            samples: n,
            avg,
            p50: pick(0.50),
            p95: pick(0.95),
            max: *samples.last().unwrap(),
        })
    }
}

/// Build the per-endpoint summary. Pass the captured slice as
/// produced by the conformance self-test sink.
///
/// Round 33 (#823) — grouping key is `(method, path_template, spec)`
/// when the capture carries a non-empty `path_template`, and falls
/// back to `(method, resolved-path)` otherwise so this stays
/// compatible with older capture files that don't have the field.
pub fn build_summary(captures: &[CaseCapture]) -> Vec<PerEndpointSummary> {
    let mut by_key: BTreeMap<(String, String, Option<String>), EndpointAccumulator> =
        BTreeMap::new();

    for c in captures {
        let (resolved_path, query) = split_url(&c.url);
        // Prefer the spec template; resolved URL path is the fallback
        // only for legacy captures that predate `path_template`.
        let path = if c.path_template.is_empty() {
            resolved_path
        } else {
            c.path_template.clone()
        };
        let key = (c.method.to_ascii_uppercase(), path, c.spec_label.clone());
        let entry = by_key.entry(key).or_default();
        entry.sent += 1;
        match c.response_status {
            0 => entry.errors += 1,
            s if (200..300).contains(&s) => entry.status_2xx += 1,
            s if (300..400).contains(&s) => entry.status_3xx += 1,
            s if (400..500).contains(&s) => entry.status_4xx += 1,
            s if (500..600).contains(&s) => entry.status_5xx += 1,
            _ => {}
        }
        if let Some(body) = &c.request_body {
            entry.request_lens.push(body.len() as u64);
        }
        if let Some(body) = &c.response_body {
            entry.response_lens.push(body.len() as u64);
        }
        if let Some(q) = query {
            if !q.is_empty() {
                entry.query_lens.push(q.len() as u64);
            }
        }
    }

    let mut out: Vec<PerEndpointSummary> = by_key
        .into_iter()
        .map(|((method, path, spec), acc)| PerEndpointSummary {
            spec,
            method,
            path,
            sent: acc.sent,
            status_2xx: acc.status_2xx,
            status_3xx: acc.status_3xx,
            status_4xx: acc.status_4xx,
            status_5xx: acc.status_5xx,
            errors: acc.errors,
            request_body_len: LenStats::from_samples(acc.request_lens),
            response_body_len: LenStats::from_samples(acc.response_lens),
            query_len: LenStats::from_samples(acc.query_lens),
        })
        .collect();
    // Sort by sent count desc, then by (method, path) for stable order.
    out.sort_by(|a, b| b.sent.cmp(&a.sent).then(a.method.cmp(&b.method)).then(a.path.cmp(&b.path)));
    out
}

#[derive(Default)]
struct EndpointAccumulator {
    sent: usize,
    status_2xx: usize,
    status_3xx: usize,
    status_4xx: usize,
    status_5xx: usize,
    errors: usize,
    request_lens: Vec<u64>,
    response_lens: Vec<u64>,
    query_lens: Vec<u64>,
}

/// Return `(path, query)` from a fully-qualified URL. Falls back to
/// returning the input unchanged as the path when parsing fails (so
/// the summary still groups, just without query metrics).
fn split_url(url: &str) -> (String, Option<String>) {
    // Strip scheme + host. URLs the bench produces always start with
    // a scheme; defensive against the rare relative-URL case.
    let after_host = if let Some(idx) = url.find("://") {
        let rest = &url[idx + 3..];
        match rest.find('/') {
            Some(i) => &rest[i..],
            None => "/",
        }
    } else {
        url
    };
    match after_host.find('?') {
        Some(i) => (after_host[..i].to_string(), Some(after_host[i + 1..].to_string())),
        None => (after_host.to_string(), None),
    }
}

/// Render the per-endpoint summary as a self-contained HTML
/// `<section>` block suitable for splicing into
/// `conformance-report.html`. Uses the same `<table>` styling the
/// rest of the report already has.
pub fn render_html_section(summaries: &[PerEndpointSummary]) -> String {
    if summaries.is_empty() {
        return String::new();
    }
    // Round 33 (#823) — show the Spec column only when at least one row
    // carries a spec label, so single-spec runs don't get an empty
    // column and multi-spec runs can attribute rows.
    let show_spec = summaries.iter().any(|s| s.spec.is_some());
    let mut out = String::from("<h2 id=\"per-endpoint\">Per-endpoint traffic summary</h2>\n");
    out.push_str(
        "<p class=\"small\">Aggregated from the JSONL capture sink. Path is the spec template; lengths are byte counts on the captured (truncated) bodies.</p>\n",
    );
    out.push_str("<table>\n<thead><tr>");
    if show_spec {
        out.push_str("<th>Spec</th>");
    }
    out.push_str(
        "<th>Method</th><th>Path</th>\
         <th>Sent</th><th>2xx</th><th>3xx</th><th>4xx</th><th>5xx</th><th>Err</th>\
         <th>Req p95 (B)</th><th>Resp p95 (B)</th><th>Query p95 (B)</th>\
         </tr></thead>\n<tbody>\n",
    );
    for s in summaries {
        let req = s
            .request_body_len
            .as_ref()
            .map(|l| l.p95.to_string())
            .unwrap_or_else(|| "-".to_string());
        let resp = s
            .response_body_len
            .as_ref()
            .map(|l| l.p95.to_string())
            .unwrap_or_else(|| "-".to_string());
        let query = s
            .query_len
            .as_ref()
            .map(|l| l.p95.to_string())
            .unwrap_or_else(|| "-".to_string());
        out.push_str("<tr>");
        if show_spec {
            let spec_cell = s.spec.as_deref().unwrap_or("-");
            out.push_str(&format!("<td><code>{}</code></td>", html_escape(spec_cell)));
        }
        out.push_str(&format!(
            "<td><code>{}</code></td><td><code>{}</code></td>\
             <td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>\
             <td>{}</td><td>{}</td><td>{}</td></tr>\n",
            html_escape(&s.method),
            html_escape(&s.path),
            s.sent,
            s.status_2xx,
            s.status_3xx,
            s.status_4xx,
            s.status_5xx,
            s.errors,
            req,
            resp,
            query,
        ));
    }
    out.push_str("</tbody></table>\n");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(
        method: &str,
        url: &str,
        status: u16,
        req: Option<&str>,
        resp: Option<&str>,
    ) -> CaseCapture {
        cap_with_template(method, url, status, req, resp, "")
    }

    fn cap_with_template(
        method: &str,
        url: &str,
        status: u16,
        req: Option<&str>,
        resp: Option<&str>,
        path_template: &str,
    ) -> CaseCapture {
        CaseCapture {
            label: "x".to_string(),
            method: method.to_string(),
            url: url.to_string(),
            request_headers: BTreeMap::new(),
            request_body: req.map(|s| s.to_string()),
            request_body_truncated: false,
            response_status: status,
            response_headers: BTreeMap::new(),
            response_body: resp.map(|s| s.to_string()),
            response_body_truncated: false,
            error: None,
            response_schema_error: None,
            expected_status_range: "2xx-3xx".to_string(),
            path_template: path_template.to_string(),
            spec_label: None,
        }
    }

    #[test]
    fn groups_by_method_and_resolved_path() {
        let caps = vec![
            cap("GET", "https://host/api/foo", 200, None, Some("hello")),
            cap("GET", "https://host/api/foo", 404, None, Some("not found")),
            cap("POST", "https://host/api/bar", 201, Some(r#"{"x":1}"#), Some(r#"{"id":7}"#)),
        ];
        let s = build_summary(&caps);
        assert_eq!(s.len(), 2, "two distinct (method, path) groups");
        let foo = s.iter().find(|x| x.path == "/api/foo").unwrap();
        assert_eq!(foo.sent, 2);
        assert_eq!(foo.status_2xx, 1);
        assert_eq!(foo.status_4xx, 1);
        assert!(foo.request_body_len.is_none(), "no request bodies on GET probes");
        assert!(foo.response_body_len.is_some());
        let bar = s.iter().find(|x| x.path == "/api/bar").unwrap();
        assert!(bar.request_body_len.is_some());
        assert_eq!(bar.request_body_len.as_ref().unwrap().samples, 1);
    }

    #[test]
    fn strips_query_string_into_separate_metric() {
        let caps = vec![
            cap("GET", "https://host/api/x?a=1&b=2", 200, None, Some("ok")),
            cap("GET", "https://host/api/x?c=3", 200, None, Some("ok")),
        ];
        let s = build_summary(&caps);
        assert_eq!(s.len(), 1, "query string strip must collapse into one group");
        let row = &s[0];
        assert_eq!(row.path, "/api/x");
        assert_eq!(row.sent, 2);
        let qlen = row.query_len.as_ref().expect("query stats present");
        assert_eq!(qlen.samples, 2);
        assert_eq!(qlen.max, 7); // "a=1&b=2" is 7 bytes
    }

    #[test]
    fn p95_is_nearest_rank() {
        let stats = LenStats::from_samples(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap();
        assert_eq!(stats.p50, 5);
        assert_eq!(stats.p95, 10);
        assert_eq!(stats.max, 10);
    }

    #[test]
    fn empty_input_renders_to_empty_html() {
        assert_eq!(render_html_section(&[]), "");
    }

    /// Round 33 (#823) — Srikanth's vCenter spec resolves the same
    /// path template to many distinct URLs (`/users/{id}` →
    /// `/users/test-value`, `/users/abc`, etc). Without template
    /// grouping the report blows up to one row per VU. With it, every
    /// hit on the same `(method, path_template)` collapses into one row.
    #[test]
    fn template_grouping_collapses_distinct_resolved_urls() {
        let caps = vec![
            cap_with_template(
                "GET",
                "https://host/api/users/test-value",
                200,
                None,
                Some("ok"),
                "/users/{id}",
            ),
            cap_with_template(
                "GET",
                "https://host/api/users/abc",
                404,
                None,
                Some("nf"),
                "/users/{id}",
            ),
            cap_with_template(
                "GET",
                "https://host/api/users/zzz",
                200,
                None,
                Some("ok"),
                "/users/{id}",
            ),
        ];
        let s = build_summary(&caps);
        assert_eq!(s.len(), 1, "all three URLs collapse into one template-grouped row");
        let row = &s[0];
        assert_eq!(row.path, "/users/{id}", "path field carries the spec template");
        assert_eq!(row.sent, 3);
        assert_eq!(row.status_2xx, 2);
        assert_eq!(row.status_4xx, 1);
    }

    /// Round 33 (#823) — when probes from two different specs share
    /// the same `(method, path_template)` they stay separate rows, so
    /// a multi-spec run keeps the attribution.
    #[test]
    fn spec_label_keeps_same_template_rows_separate() {
        let mut a = cap_with_template(
            "POST",
            "https://host/api/foo",
            201,
            Some("body"),
            Some("ok"),
            "/foo",
        );
        a.spec_label = Some("specA.yaml".to_string());
        let mut b = cap_with_template(
            "POST",
            "https://host/api/foo",
            201,
            Some("body"),
            Some("ok"),
            "/foo",
        );
        b.spec_label = Some("specB.yaml".to_string());
        let caps = vec![a, b];
        let s = build_summary(&caps);
        assert_eq!(s.len(), 2, "different specs must not collapse same-template rows");
        let labels: Vec<Option<&str>> = s.iter().map(|x| x.spec.as_deref()).collect();
        assert!(labels.contains(&Some("specA.yaml")));
        assert!(labels.contains(&Some("specB.yaml")));
    }

    /// Round 33 (#823) — the HTML section only emits a Spec column
    /// when at least one row carries a spec label. Keeps the
    /// single-spec single-target run from showing a useless column.
    #[test]
    fn html_spec_column_only_appears_with_labels() {
        let no_label = vec![cap_with_template(
            "GET",
            "https://h/a",
            200,
            None,
            Some("x"),
            "/a",
        )];
        let html_no = render_html_section(&build_summary(&no_label));
        assert!(!html_no.contains("<th>Spec</th>"), "single-spec runs hide the column");

        let mut labelled = cap_with_template("GET", "https://h/b", 200, None, Some("x"), "/b");
        labelled.spec_label = Some("spec.yaml".to_string());
        let html_yes = render_html_section(&build_summary(&[labelled]));
        assert!(html_yes.contains("<th>Spec</th>"), "labelled runs surface the column");
        assert!(html_yes.contains("spec.yaml"), "spec label rendered in the row");
    }

    /// Round 33 (#823) — captures with empty `path_template` (e.g.
    /// legacy JSONL on disk) still group by resolved path, so we
    /// don't break backward compatibility.
    #[test]
    fn empty_template_falls_back_to_resolved_path() {
        let caps = vec![
            cap("GET", "https://host/api/foo", 200, None, Some("ok")),
            cap("GET", "https://host/api/foo", 200, None, Some("ok")),
        ];
        let s = build_summary(&caps);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].path, "/api/foo");
        assert_eq!(s[0].sent, 2);
    }
}
