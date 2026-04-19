//! OSV-format vulnerability advisory records.
//!
//! The `osv_vulnerabilities` table is a persistent cache of upstream
//! security advisories (OSV.dev, GHSA, CVE) keyed by
//! `(advisory_id, ecosystem, package_name)`. The plugin scanner consults
//! this cache while walking an SBOM rather than making per-scan HTTP
//! calls; a background sync worker refreshes it on a schedule.
//!
//! We deliberately don't attempt to reuse the `osv`-crate types here:
//! upstream's schema has a large surface area (semver ranges, severity
//! vectors, credit lists, related advisories), and we only need enough to
//! answer "does this (name, version) tuple have a known advisory?" The
//! raw JSON is kept in `extra_json` for later upgrades.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsvAdvisory {
    pub id: Uuid,
    pub advisory_id: String,
    pub ecosystem: String,
    pub package_name: String,
    pub severity: String,
    pub summary: String,
    pub affected_versions: serde_json::Value,
    pub extra_json: Option<serde_json::Value>,
    pub modified_at: DateTime<Utc>,
    pub imported_at: DateTime<Utc>,
}

/// A single component-level match produced by the scanner when walking an
/// SBOM against the advisory cache. `title` / `description` are the strings
/// the scanner surfaces in its finding output; `severity` maps to the
/// scanner's finding severity enum.
#[derive(Debug, Clone)]
pub struct OsvMatch {
    pub advisory_id: String,
    pub severity: String,
    pub summary: String,
}

/// Upstream advisory shape, trimmed to just the fields the importer reads.
/// Anything not modeled here is preserved verbatim in `extra_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsvImportRecord {
    /// Upstream id — e.g. `GHSA-xxx-xxx-xxx`, `CVE-2024-1234`.
    pub id: String,

    /// Optional ISO-8601 `modified` timestamp from the advisory. Used to
    /// pick the newer record when the same `advisory_id` is imported
    /// twice.
    #[serde(default)]
    pub modified: Option<String>,

    /// Human-readable summary. Falls back to `details` on import.
    #[serde(default)]
    pub summary: Option<String>,

    #[serde(default)]
    pub details: Option<String>,

    /// Each `affected[]` entry names a single package and the version
    /// ranges it applies to. Kept alongside the record so the scanner can
    /// do precise range checks later.
    pub affected: Vec<OsvAffected>,

    /// CVSS or SSVC severity. First entry is used as the coarse bucket.
    #[serde(default)]
    pub severity: Vec<OsvSeverity>,

    /// GHSA + a handful of other feeds park a qualitative severity label
    /// (`"CRITICAL"`, `"HIGH"`, …) inside `database_specific.severity`
    /// when the top-level CVSS vector is absent. We keep the whole
    /// `database_specific` object loose since its shape varies by feed.
    #[serde(default, rename = "database_specific")]
    pub database_specific: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsvAffected {
    pub package: OsvPackage,
    #[serde(default)]
    pub ranges: Vec<serde_json::Value>,
    #[serde(default)]
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsvPackage {
    pub ecosystem: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsvSeverity {
    #[serde(rename = "type")]
    pub kind: String,
    pub score: String,
}

impl OsvImportRecord {
    /// Map the record's severity vector to one of the four coarse buckets
    /// the scanner surfaces. OSV files mix CVSS 3.1 vectors and plain
    /// numeric strings; we handle the common shapes and default to
    /// `"medium"` for anything unrecognized rather than dropping the
    /// advisory on the floor.
    pub fn severity_bucket(&self) -> &'static str {
        // Prefer a CVSS score if one is present.
        for s in &self.severity {
            if let Some(bucket) = cvss_bucket(&s.score) {
                return bucket;
            }
        }
        // GHSA fallback: `database_specific.severity` is a qualitative
        // label ("CRITICAL", "HIGH", …) that the feed emits when it
        // couldn't or didn't want to publish the full vector. Reliable
        // enough for the scanner's coarse bucketing.
        if let Some(label) = self
            .database_specific
            .as_ref()
            .and_then(|d| d.get("severity"))
            .and_then(|v| v.as_str())
        {
            match label.trim().to_ascii_lowercase().as_str() {
                "critical" => return "critical",
                "high" => return "high",
                "moderate" | "medium" => return "medium",
                "low" => return "low",
                _ => {}
            }
        }
        "medium"
    }

    /// Best-effort summary. OSV advisories may omit `summary` in favor of
    /// a multi-paragraph `details` body; we take the first non-empty of
    /// the two.
    pub fn human_summary(&self) -> String {
        for s in [&self.summary, &self.details].into_iter().flatten() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return first_line(trimmed).to_string();
            }
        }
        format!("{} — no human summary available", self.id)
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("").trim()
}

fn cvss_bucket(score: &str) -> Option<&'static str> {
    // Three shapes land here, in rough order of frequency from OSV feeds:
    //
    // 1. CVSS v3/v4 **vector string** (`CVSS:3.1/AV:N/AC:L/...`) — the
    //    canonical OSV form. We parse the vector with the `cvss` crate,
    //    read off its computed base score, and bucket it.
    // 2. Plain numeric string (`"7.5"`) — some feeds pre-compute the
    //    score and drop the vector. Handle it the obvious way.
    // 3. Anything else — we return None and let the caller fall back.
    //
    // Cutoffs are the CVSS v3.0/3.1 qualitative severity rating scale:
    // 0.0=None, 0.1-3.9=Low, 4.0-6.9=Medium, 7.0-8.9=High, 9.0-10.0=Critical.
    // We collapse None into Low so every matched CVSS produces one of four
    // buckets; the bucket set is fixed by the DB CHECK constraint.
    let trimmed = score.trim();

    let base_score: Option<f32> = if trimmed.starts_with("CVSS:") {
        // Try v3.x first (most common in OSV dumps today), then v4 (for
        // newer advisories). We don't need to distinguish the version
        // after parsing — both expose a base score in 0..=10.
        if let Ok(v3) = trimmed.parse::<cvss::v3::Base>() {
            Some(v3.score().value() as f32)
        } else if let Ok(v4) = trimmed.parse::<cvss::v4::Vector>() {
            Some(v4.score().value() as f32)
        } else {
            None
        }
    } else {
        trimmed.parse::<f32>().ok()
    };

    let n = base_score?;
    Some(match n {
        x if x >= 9.0 => "critical",
        x if x >= 7.0 => "high",
        x if x >= 4.0 => "medium",
        _ => "low",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity-check CVSS vector parsing across the three shapes we see
    /// in the wild: full v3.1 vectors, v4 vectors, and pre-computed
    /// numeric scores. The specific vectors here come from real OSV
    /// advisories — `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H` is
    /// `GHSA-xvch-5gv4-984h`'s (remote code execution, score 9.8).
    #[test]
    fn cvss_bucket_parses_v3_vector() {
        assert_eq!(cvss_bucket("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"), Some("critical"));
        assert_eq!(cvss_bucket("CVSS:3.1/AV:L/AC:L/PR:L/UI:R/S:U/C:H/I:H/A:H"), Some("high"));
        assert_eq!(cvss_bucket("CVSS:3.0/AV:L/AC:H/PR:L/UI:R/S:U/C:L/I:L/A:N"), Some("low"));
    }

    #[test]
    fn cvss_bucket_parses_numeric_string() {
        assert_eq!(cvss_bucket("9.8"), Some("critical"));
        assert_eq!(cvss_bucket("7.5"), Some("high"));
        assert_eq!(cvss_bucket("5.0"), Some("medium"));
        assert_eq!(cvss_bucket("3.1"), Some("low"));
    }

    #[test]
    fn cvss_bucket_returns_none_on_junk() {
        // Anything that isn't a number or a CVSS vector returns None so
        // the caller can fall through to the `database_specific` label or
        // the "medium" default.
        assert_eq!(cvss_bucket(""), None);
        assert_eq!(cvss_bucket("unknown"), None);
        assert_eq!(cvss_bucket("CVSS:3.1/invalid"), None);
    }

    #[test]
    fn severity_bucket_prefers_cvss_over_database_specific() {
        // Both signals disagree on purpose — CVSS wins. This is what we
        // want: the numerical vector is the more precise signal.
        let rec: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "X",
            "summary": "",
            "affected": [],
            "severity": [{
                "type": "CVSS_V3",
                "score": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
            }],
            "database_specific": {"severity": "LOW"}
        }))
        .unwrap();
        assert_eq!(rec.severity_bucket(), "critical");
    }

    #[test]
    fn severity_bucket_falls_back_to_database_specific() {
        // No top-level CVSS, GHSA-style qualitative label present.
        let rec: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "X",
            "summary": "",
            "affected": [],
            "severity": [],
            "database_specific": {"severity": "HIGH"}
        }))
        .unwrap();
        assert_eq!(rec.severity_bucket(), "high");

        // MODERATE → medium (GHSA's spelling for medium).
        let mod_rec: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "Y",
            "affected": [],
            "severity": [],
            "database_specific": {"severity": "MODERATE"}
        }))
        .unwrap();
        assert_eq!(mod_rec.severity_bucket(), "medium");
    }

    #[test]
    fn severity_bucket_defaults_to_medium_when_no_signal() {
        let rec: OsvImportRecord = serde_json::from_value(serde_json::json!({
            "id": "X",
            "affected": [],
        }))
        .unwrap();
        assert_eq!(rec.severity_bucket(), "medium");
    }
}
