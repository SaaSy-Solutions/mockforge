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
    // CVSS vector strings embed the base score as `CVSS:3.1/.../` plus
    // metric letters. We don't parse that — any field that isn't a plain
    // number (including `CVSS:...` vectors) returns None so the caller
    // falls back to the advisory's default bucket. If the score is a
    // plain number (e.g. "7.5") we map it by the standard CVSS v3 cutoffs.
    let n = score.trim().parse::<f32>().ok()?;
    Some(match n {
        x if x >= 9.0 => "critical",
        x if x >= 7.0 => "high",
        x if x >= 4.0 => "medium",
        _ => "low",
    })
}
