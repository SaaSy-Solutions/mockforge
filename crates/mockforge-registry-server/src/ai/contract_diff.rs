//! AI-assisted contract drift scoring (#348).
//!
//! The structural `ContractExecutor` in `mockforge-test-runner` already
//! computes which declared spec endpoints have traffic vs. which are
//! orphaned, and which traffic endpoints aren't declared at all. That's
//! cheap and free.
//!
//! What this module adds is a *second pass* that takes a small sample
//! of recent captured exchanges per declared endpoint and asks an LLM
//! to score how the actual request/response shape compares to what the
//! spec promises. Findings come back tagged
//! `breaking | non_breaking | cosmetic` plus a confidence score.
//!
//! ## Why this is opt-in
//!
//! - Costs real LLM tokens (BYOK or platform credits, depending on the
//!   org's setup).
//! - Sampling captures means full request/response bodies leave the
//!   registry's network, so users with strict data-handling rules
//!   should be able to keep it disabled.
//!
//! Both are surfaced as a `ai_drift_enabled: bool` flag on the
//! `test_suite.config` blob; the runner only fires the scoring callback
//! when set true.
//!
//! ## What lives here vs. what lives in the handler
//!
//! Everything in this module is pure (no DB, no HTTP) so the prompt
//! template, sample-shaping, and LLM-response parsing are unit-testable
//! without fixtures. The HTTP handler at
//! `handlers::internal_contract_diff` owns the orchestration: pulling
//! sample captures, calling `run_completion_for_org`, and recording
//! usage.

use serde::{Deserialize, Serialize};

/// Severity buckets returned by the AI scorer. Matches the existing
/// `severity` field on `diff_finding` events so the UI's
/// severity-grouped renderer renders both structural and AI findings
/// without code changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    /// Behaviour observed in traffic that the spec explicitly forbids
    /// (missing required field, wrong type, status code outside the
    /// declared set, removed-but-still-served endpoint).
    Breaking,
    /// Behaviour the spec doesn't strictly forbid but that real
    /// consumers would notice (added optional field in a response,
    /// looser validation than declared).
    NonBreaking,
    /// Stylistic drift only (case differences, whitespace, ordering of
    /// optional fields). Not actionable, kept so the model can flag
    /// without distorting severity counts.
    Cosmetic,
}

impl DriftSeverity {
    /// Wire string used in `diff_finding` event payloads.
    pub fn as_str(&self) -> &'static str {
        match self {
            DriftSeverity::Breaking => "breaking",
            DriftSeverity::NonBreaking => "non_breaking",
            DriftSeverity::Cosmetic => "cosmetic",
        }
    }
}

/// One AI-scored drift finding for a single endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFinding {
    /// `breaking | non_breaking | cosmetic`.
    pub severity: DriftSeverity,
    /// `"POST /api/checkout"` — same shape the structural pass uses.
    pub endpoint: String,
    /// Short, human-readable description of the drift.
    pub description: String,
    /// Model's self-reported confidence in `[0.0, 1.0]`. Out-of-range
    /// values are clamped on parse.
    pub confidence: f64,
    /// Free-text rationale the UI can show on click-through. Empty
    /// string when the model didn't supply one.
    #[serde(default)]
    pub rationale: String,
}

/// One sampled exchange that the scorer can reason about. Bodies are
/// the verbatim request/response strings from `runtime_captures` —
/// already filtered to JSON-shaped payloads upstream.
#[derive(Debug, Clone, Serialize)]
pub struct SampledExchange {
    pub method: String,
    pub path: String,
    pub status_code: Option<i32>,
    /// Truncated to a safe length before being included in the prompt
    /// (see [`MAX_BODY_CHARS`]). The truncation is stable so the model
    /// gets a deterministic view across re-runs.
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

/// Cap per-body characters embedded in the prompt. Picked to stay well
/// inside even Haiku-class context windows once the system prompt and
/// spec excerpt are added: 6KB × ~10 exchanges ≈ 60KB of body data,
/// plus ~10KB of spec excerpt and prompt ≈ 70KB total. Comfortable
/// margin under a 128k context.
pub const MAX_BODY_CHARS: usize = 6_000;

/// Marker appended to truncated bodies so the model can tell it didn't
/// see the whole thing. Length deliberately stable so
/// [`truncate_body`] can reserve space for it.
const TRUNCATION_MARKER: &str = "\n… (truncated)";

/// Truncate a body string at a UTF-8 boundary, appending an
/// `… (truncated)` marker so the model knows it didn't see the whole
/// thing. Returns the original string when it's already small enough.
/// Reserves space for the marker so the result is always shorter than
/// the input — even when the original was barely over `max_chars`.
pub fn truncate_body(body: &str, max_chars: usize) -> String {
    if body.chars().count() <= max_chars {
        return body.to_string();
    }
    // Reserve room for the marker so the final length stays under the
    // input's. Saturating-sub guards against pathologically tiny
    // `max_chars` values (anything < marker length collapses to a
    // bare marker, which is fine — the model still gets the signal).
    let marker_len = TRUNCATION_MARKER.chars().count();
    let take = max_chars.saturating_sub(marker_len);
    let mut out: String = body.chars().take(take).collect();
    out.push_str(TRUNCATION_MARKER);
    out
}

/// Build the system + user prompts the LLM sees. Pure function so the
/// shape can be locked down in tests.
///
/// The system prompt teaches the model:
/// 1. The severity vocabulary it must emit (breaking / non_breaking /
///    cosmetic).
/// 2. The strict JSON-array output format.
/// 3. How to weigh sample size when reporting confidence.
///
/// The user prompt carries the spec excerpt followed by the
/// per-endpoint sample exchanges, formatted as a fenced JSON block so
/// the model can scan them mechanically.
pub fn build_prompt(spec_excerpt: &str, exchanges: &[SampledExchange]) -> (String, String) {
    let system = "You are a contract-drift reviewer for OpenAPI APIs. Compare actual \
HTTP exchanges against the declared spec and report only meaningful drift.\n\n\
You MUST emit a single JSON array (no prose, no markdown fences) where each \
element has these exact keys: severity (\"breaking\" | \"non_breaking\" | \
\"cosmetic\"), endpoint (\"METHOD /path\"), description (one short sentence), \
confidence (0.0 to 1.0), rationale (one or two sentences explaining what \
you observed).\n\n\
Severity rules:\n\
- breaking: required field missing, wrong type, undeclared status code, \
endpoint serving traffic that the spec forbids.\n\
- non_breaking: extra optional field present, loosened validation, \
behaviour the spec doesn't forbid.\n\
- cosmetic: case-only differences, ordering, whitespace, trailing slashes.\n\n\
Confidence rules: If you saw fewer than 2 exchanges for an endpoint, cap \
confidence at 0.6. If exchanges all show the same drift, use 0.85+. If \
exchanges disagree with each other, drop to 0.5.\n\n\
If you find no drift at all, emit an empty array `[]`."
        .to_string();

    let mut user = String::new();
    user.push_str("# Declared OpenAPI spec (excerpt)\n\n```yaml\n");
    user.push_str(spec_excerpt);
    user.push_str("\n```\n\n# Sampled exchanges\n\n");

    if exchanges.is_empty() {
        user.push_str("(no exchanges sampled — declared endpoints had no recent traffic)\n");
    } else {
        for ex in exchanges {
            user.push_str(&format!(
                "## {} {}\nstatus: {}\n",
                ex.method,
                ex.path,
                ex.status_code.map(|s| s.to_string()).unwrap_or_else(|| "?".to_string()),
            ));
            if let Some(body) = &ex.request_body {
                user.push_str("\n### request body\n```\n");
                user.push_str(&truncate_body(body, MAX_BODY_CHARS));
                user.push_str("\n```\n");
            }
            if let Some(body) = &ex.response_body {
                user.push_str("\n### response body\n```\n");
                user.push_str(&truncate_body(body, MAX_BODY_CHARS));
                user.push_str("\n```\n");
            }
            user.push('\n');
        }
    }

    user.push_str(
        "\n# Task\n\nEmit the JSON array of findings. No other output. \
If nothing drifted, emit `[]`.",
    );

    (system, user)
}

/// Parse the model's response into structured findings. Tolerates:
/// - Plain JSON arrays.
/// - JSON arrays inside ```json``` fences (handled by
///   [`crate::handlers::ai_studio::extract_json_payload`] upstream).
/// - Single-object responses where the model forgot the array wrapper
///   (we wrap it for them).
/// - Out-of-range confidence values (clamped to `[0.0, 1.0]`).
/// - Unknown severity strings (entry skipped with a warning).
///
/// Returns an empty vec when nothing usable was found — the executor
/// emits a degraded-but-fine "AI scoring returned no findings" log
/// rather than failing the whole run.
pub fn parse_findings(json: &serde_json::Value) -> Vec<AiFinding> {
    let raw_array: Vec<serde_json::Value> = match json {
        serde_json::Value::Array(items) => items.clone(),
        // Single-object response — wrap it.
        obj @ serde_json::Value::Object(_) => vec![obj.clone()],
        _ => return Vec::new(),
    };

    let mut out = Vec::with_capacity(raw_array.len());
    for item in raw_array {
        let severity_str = item.get("severity").and_then(|v| v.as_str()).unwrap_or("");
        let severity = match severity_str {
            "breaking" => DriftSeverity::Breaking,
            "non_breaking" => DriftSeverity::NonBreaking,
            "cosmetic" => DriftSeverity::Cosmetic,
            _ => continue, // unknown severity — drop the entry rather than guess
        };
        let endpoint = item
            .get("endpoint")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let Some(endpoint) = endpoint else { continue };
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let raw_conf = item.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let confidence = raw_conf.clamp(0.0, 1.0);
        let rationale = item
            .get("rationale")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        out.push(AiFinding {
            severity,
            endpoint,
            description,
            confidence,
            rationale,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex(method: &str, path: &str, body: &str) -> SampledExchange {
        SampledExchange {
            method: method.to_string(),
            path: path.to_string(),
            status_code: Some(200),
            request_body: None,
            response_body: Some(body.to_string()),
        }
    }

    #[test]
    fn truncate_short_body_unchanged() {
        let s = "abc";
        assert_eq!(truncate_body(s, 100), "abc");
    }

    #[test]
    fn truncate_appends_marker() {
        let s = "x".repeat(MAX_BODY_CHARS + 10);
        let out = truncate_body(&s, MAX_BODY_CHARS);
        assert!(out.ends_with("… (truncated)"));
        // Original body chars + marker
        assert!(out.chars().count() < s.chars().count());
    }

    #[test]
    fn truncate_handles_multibyte_at_boundary() {
        // Marker chars: emoji plus surrounding text. Must not split codepoints.
        let s = "héllo wörld 🎉".repeat(2_000);
        let out = truncate_body(&s, 100);
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn build_prompt_with_no_exchanges() {
        let (system, user) = build_prompt("paths: {}", &[]);
        assert!(system.contains("contract-drift"));
        assert!(user.contains("(no exchanges sampled"));
        assert!(user.contains("paths: {}"));
    }

    #[test]
    fn build_prompt_includes_method_path_status() {
        let (_system, user) =
            build_prompt("openapi: 3.0.0", &[ex("POST", "/api/checkout", r#"{"item":"x"}"#)]);
        assert!(user.contains("POST /api/checkout"));
        assert!(user.contains("status: 200"));
        assert!(user.contains(r#"{"item":"x"}"#));
    }

    #[test]
    fn parse_findings_well_formed_array() {
        let json = serde_json::json!([
            {
                "severity": "breaking",
                "endpoint": "POST /api/checkout",
                "description": "Required field missing",
                "confidence": 0.9,
                "rationale": "All 3 sampled responses omitted `created_at`."
            },
            {
                "severity": "cosmetic",
                "endpoint": "GET /api/users",
                "description": "Trailing slash in path",
                "confidence": 0.4,
                "rationale": ""
            }
        ]);
        let findings = parse_findings(&json);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, DriftSeverity::Breaking);
        assert_eq!(findings[0].endpoint, "POST /api/checkout");
        assert!((findings[0].confidence - 0.9).abs() < 0.001);
        assert_eq!(findings[1].severity, DriftSeverity::Cosmetic);
        assert!(findings[1].rationale.is_empty());
    }

    #[test]
    fn parse_findings_clamps_confidence_out_of_range() {
        let json = serde_json::json!([
            { "severity": "breaking", "endpoint": "GET /a", "description": "x", "confidence": 1.7 },
            { "severity": "breaking", "endpoint": "GET /b", "description": "y", "confidence": -0.5 }
        ]);
        let findings = parse_findings(&json);
        assert_eq!(findings.len(), 2);
        assert!((findings[0].confidence - 1.0).abs() < 0.001);
        assert!(findings[1].confidence.abs() < 0.001);
    }

    #[test]
    fn parse_findings_drops_unknown_severity() {
        let json = serde_json::json!([
            { "severity": "blocker", "endpoint": "GET /a", "description": "x", "confidence": 0.5 },
            { "severity": "breaking", "endpoint": "GET /b", "description": "y", "confidence": 0.5 }
        ]);
        let findings = parse_findings(&json);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].endpoint, "GET /b");
    }

    #[test]
    fn parse_findings_drops_missing_endpoint() {
        let json = serde_json::json!([
            { "severity": "breaking", "description": "x", "confidence": 0.5 }
        ]);
        assert!(parse_findings(&json).is_empty());
    }

    #[test]
    fn parse_findings_wraps_single_object() {
        let json = serde_json::json!({
            "severity": "non_breaking",
            "endpoint": "GET /a",
            "description": "Extra optional field",
            "confidence": 0.7
        });
        let findings = parse_findings(&json);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, DriftSeverity::NonBreaking);
    }

    #[test]
    fn parse_findings_empty_array() {
        let json = serde_json::json!([]);
        assert!(parse_findings(&json).is_empty());
    }

    #[test]
    fn parse_findings_non_array_non_object_returns_empty() {
        assert!(parse_findings(&serde_json::json!("nope")).is_empty());
        assert!(parse_findings(&serde_json::json!(42)).is_empty());
        assert!(parse_findings(&serde_json::json!(null)).is_empty());
    }

    #[test]
    fn drift_severity_wire_strings() {
        assert_eq!(DriftSeverity::Breaking.as_str(), "breaking");
        assert_eq!(DriftSeverity::NonBreaking.as_str(), "non_breaking");
        assert_eq!(DriftSeverity::Cosmetic.as_str(), "cosmetic");
    }
}
