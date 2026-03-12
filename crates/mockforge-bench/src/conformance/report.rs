//! Conformance test report parsing and display

use super::spec::ConformanceFeature;
use crate::error::{BenchError, Result};
use crate::owasp_api::categories::OwaspCategory;
use colored::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Per-category conformance result
#[derive(Debug, Clone, Default)]
pub struct CategoryResult {
    pub passed: usize,
    pub failed: usize,
}

impl CategoryResult {
    pub fn total(&self) -> usize {
        self.passed + self.failed
    }

    pub fn rate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total() as f64) * 100.0
        }
    }
}

/// Detail of a single conformance check failure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailureDetail {
    /// Check name that failed
    pub check: String,
    /// Request information
    pub request: FailureRequest,
    /// Response information
    pub response: FailureResponse,
    /// What the check expected
    pub expected: String,
}

/// Request details for a failed check
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailureRequest {
    /// HTTP method
    #[serde(default)]
    pub method: String,
    /// Full URL
    #[serde(default)]
    pub url: String,
    /// Request headers (k6 sends arrays per header, we flatten to first value)
    #[serde(default, deserialize_with = "deserialize_headers")]
    pub headers: HashMap<String, String>,
    /// Request body (truncated)
    #[serde(default)]
    pub body: String,
}

/// Response details for a failed check
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailureResponse {
    /// HTTP status code
    #[serde(default)]
    pub status: u16,
    /// Response headers (k6 may send arrays or strings)
    #[serde(default, deserialize_with = "deserialize_headers")]
    pub headers: HashMap<String, String>,
    /// Response body (truncated)
    #[serde(default)]
    pub body: String,
}

/// Deserialize headers that may be `{key: "value"}` or `{key: ["value"]}` (k6 format)
fn deserialize_headers<'de, D>(
    deserializer: D,
) -> std::result::Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let map: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            let val = match &v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(arr) => {
                    arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                }
                other => other.to_string(),
            };
            (k, val)
        })
        .collect())
}

/// Extract the base name of a custom check.
/// Custom sub-checks have format "custom:name:header:..." or "custom:name:body:..."
/// The base name is just the primary check (e.g., "custom:pets-returns-200").
fn extract_custom_base_name(check_name: &str) -> String {
    // "custom:" prefix is 7 chars. Find the next colon after that.
    let after_prefix = &check_name[7..];
    if let Some(pos) = after_prefix.find(":header:").or(after_prefix.find(":body:")) {
        check_name[..7 + pos].to_string()
    } else {
        check_name.to_string()
    }
}

/// Conformance test report
pub struct ConformanceReport {
    /// Per-check results: check_name -> (passes, fails)
    check_results: HashMap<String, (u64, u64)>,
    /// Detailed failure information
    failure_details: Vec<FailureDetail>,
}

impl ConformanceReport {
    /// Construct a report directly from check results and failure details.
    /// Used by `NativeConformanceExecutor` to build a report without k6.
    pub fn from_results(
        check_results: HashMap<String, (u64, u64)>,
        failure_details: Vec<FailureDetail>,
    ) -> Self {
        Self {
            check_results,
            failure_details,
        }
    }

    /// Serialize the report to JSON.
    ///
    /// Includes both the raw `checks` map (for CLI/k6 compat) and structured
    /// `summary`, `categories`, and `failures` fields (for UI consumption).
    pub fn to_json(&self) -> serde_json::Value {
        let mut checks = serde_json::Map::new();
        for (name, (passes, fails)) in &self.check_results {
            checks.insert(
                name.clone(),
                serde_json::json!({
                    "passes": passes,
                    "fails": fails,
                }),
            );
        }

        // Compute structured category results for UI
        let by_cat = self.by_category();
        let mut categories_json = serde_json::Map::new();
        for (cat_name, cat_result) in &by_cat {
            categories_json.insert(
                (*cat_name).to_string(),
                serde_json::json!({
                    "passed": cat_result.passed,
                    "total": cat_result.total(),
                    "rate": cat_result.rate(),
                }),
            );
        }

        // Compute summary
        let total_passed: usize = by_cat.values().map(|r| r.passed).sum();
        let total: usize = by_cat.values().map(|r| r.total()).sum();
        let overall_rate = if total == 0 {
            0.0
        } else {
            (total_passed as f64 / total as f64) * 100.0
        };

        // Transform failure details into UI-friendly format
        let failures: Vec<serde_json::Value> = self
            .failure_details
            .iter()
            .map(|d| {
                let category = Self::category_for_check(&d.check);
                serde_json::json!({
                    "check_name": d.check,
                    "category": category,
                    "expected": d.expected,
                    "actual": format!("status {}", d.response.status),
                    "details": format!("{} {}", d.request.method, d.request.url),
                })
            })
            .collect();

        let mut result = serde_json::json!({
            "checks": checks,
            "summary": {
                "total_checks": total,
                "passed": total_passed,
                "failed": total - total_passed,
                "overall_rate": overall_rate,
            },
            "categories": categories_json,
            "failures": failures,
        });

        // Keep raw failure_details for backward compat
        if !self.failure_details.is_empty() {
            result["failure_details"] = serde_json::to_value(&self.failure_details)
                .unwrap_or(serde_json::Value::Array(Vec::new()));
        }
        result
    }

    /// Determine the category for a check name based on its prefix
    fn category_for_check(check_name: &str) -> &'static str {
        let prefix = check_name.split(':').next().unwrap_or("");
        match prefix {
            "param" => "Parameters",
            "body" => "Request Bodies",
            "response" => "Response Codes",
            "schema" => "Schema Types",
            "compose" => "Composition",
            "format" => "String Formats",
            "constraint" => "Constraints",
            "security" => "Security",
            "method" => "HTTP Methods",
            "content" => "Content Types",
            "validation" | "response_validation" => "Response Validation",
            "custom" => "Custom",
            _ => "Other",
        }
    }

    /// Get the failure details
    pub fn failure_details(&self) -> &[FailureDetail] {
        &self.failure_details
    }

    /// Parse a conformance report from k6's handleSummary JSON output
    ///
    /// Also loads failure details from `conformance-failure-details.json` in the same directory.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BenchError::Other(format!("Failed to read conformance report: {}", e)))?;
        let mut report = Self::from_json(&content)?;

        // Load failure details from sibling file
        if let Some(parent) = path.parent() {
            let details_path = parent.join("conformance-failure-details.json");
            if details_path.exists() {
                if let Ok(details_json) = std::fs::read_to_string(&details_path) {
                    if let Ok(details) = serde_json::from_str::<Vec<FailureDetail>>(&details_json) {
                        report.failure_details = details;
                    }
                }
            }
        }

        Ok(report)
    }

    /// Parse from JSON string
    pub fn from_json(json_str: &str) -> Result<Self> {
        let json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| BenchError::Other(format!("Failed to parse conformance JSON: {}", e)))?;

        let mut check_results = HashMap::new();

        if let Some(checks) = json.get("checks").and_then(|c| c.as_object()) {
            for (name, result) in checks {
                let passes = result.get("passes").and_then(|v| v.as_u64()).unwrap_or(0);
                let fails = result.get("fails").and_then(|v| v.as_u64()).unwrap_or(0);
                check_results.insert(name.clone(), (passes, fails));
            }
        }

        Ok(Self {
            check_results,
            failure_details: Vec::new(),
        })
    }

    /// Get results grouped by category.
    ///
    /// Includes all standard categories plus a synthetic "Custom" category
    /// for any check names starting with "custom:".
    pub fn by_category(&self) -> HashMap<&'static str, CategoryResult> {
        let mut categories: HashMap<&'static str, CategoryResult> = HashMap::new();

        // Initialize all categories
        for cat in ConformanceFeature::categories() {
            categories.insert(cat, CategoryResult::default());
        }

        // Map check results to features.
        // Check names are path-qualified (e.g. "constraint:required:/users")
        // so we match by prefix as well as exact name.
        for feature in ConformanceFeature::all() {
            let check_name = feature.check_name();
            let category = feature.category();

            let entry = categories.entry(category).or_default();

            // First try exact match (reference mode)
            if let Some((passes, fails)) = self.check_results.get(check_name) {
                if *fails == 0 && *passes > 0 {
                    entry.passed += 1;
                } else {
                    entry.failed += 1;
                }
            } else {
                // Try prefix match (path-qualified: "constraint:required:/path")
                let prefix = format!("{}:", check_name);
                for (name, (passes, fails)) in &self.check_results {
                    if name.starts_with(&prefix) {
                        if *fails == 0 && *passes > 0 {
                            entry.passed += 1;
                        } else {
                            entry.failed += 1;
                        }
                    }
                }
                // Features not in results are not counted
            }
        }

        // Aggregate custom checks (check names starting with "custom:")
        let custom_entry = categories.entry("Custom").or_default();
        // Track which top-level custom check names we've already counted
        let mut counted_custom: HashSet<String> = HashSet::new();
        for (name, (passes, fails)) in &self.check_results {
            if name.starts_with("custom:") {
                // Only count the primary check (status), not sub-checks (header/body)
                // Sub-checks have format "custom:name:header:..." or "custom:name:body:..."
                // Primary checks are just "custom:something" with exactly one colon after "custom"
                // We count each unique top-level custom check once
                let base_name = extract_custom_base_name(name);
                if counted_custom.insert(base_name) {
                    if *fails == 0 && *passes > 0 {
                        custom_entry.passed += 1;
                    } else {
                        custom_entry.failed += 1;
                    }
                }
            }
        }

        categories
    }

    /// Print the conformance report to stdout
    pub fn print_report(&self) {
        self.print_report_with_options(false);
    }

    /// Print the conformance report with options controlling detail level
    pub fn print_report_with_options(&self, all_operations: bool) {
        let categories = self.by_category();

        // Count detected features and active categories
        let total_possible = ConformanceFeature::all().len();
        let active_cats: usize = ConformanceFeature::categories()
            .iter()
            .filter(|c| categories.get(*c).is_some_and(|r| r.total() > 0))
            .count();
        let detected: usize =
            categories.iter().filter(|(k, _)| *k != &"Custom").map(|(_, v)| v.total()).sum();

        println!("\n{}", "OpenAPI 3.0.0 Conformance Report".bold());
        println!("{}", "=".repeat(64).bright_green());

        println!(
            "{}",
            format!(
                "Spec Analysis: {} of {} features detected across {} categories",
                detected, total_possible, active_cats
            )
            .bright_cyan()
        );
        println!();

        println!(
            "{:<20} {:>8} {:>8} {:>8} {:>8}",
            "Category".bold(),
            "Passed".green().bold(),
            "Failed".red().bold(),
            "Total".bold(),
            "Rate".bold()
        );
        println!("{}", "-".repeat(64));

        let mut total_passed = 0usize;
        let mut total_failed = 0usize;
        let mut empty_categories: Vec<&str> = Vec::new();

        // Build the list of categories to display (standard + Custom if present)
        let all_cat_names: Vec<&str> = {
            let mut cats: Vec<&str> = ConformanceFeature::categories().to_vec();
            if categories.get("Custom").is_some_and(|r| r.total() > 0) {
                cats.push("Custom");
            }
            cats
        };

        for cat_name in &all_cat_names {
            if let Some(result) = categories.get(cat_name) {
                let total = result.total();
                if total == 0 {
                    // Show empty categories with dimmed "not in spec" indicator
                    println!(
                        "{:<20} {:>8} {:>8} {:>8} {:>8}",
                        cat_name.bright_black(),
                        "-".bright_black(),
                        "-".bright_black(),
                        "-".bright_black(),
                        "not in spec".bright_black()
                    );
                    empty_categories.push(cat_name);
                    continue;
                }
                total_passed += result.passed;
                total_failed += result.failed;

                let rate_str = format!("{:.0}%", result.rate());
                let rate_colored = if result.rate() >= 100.0 {
                    rate_str.green()
                } else if result.rate() >= 80.0 {
                    rate_str.yellow()
                } else {
                    rate_str.red()
                };

                println!(
                    "{:<20} {:>8} {:>8} {:>8} {:>8}",
                    cat_name,
                    result.passed.to_string().green(),
                    result.failed.to_string().red(),
                    total,
                    rate_colored
                );
            }
        }

        println!("{}", "=".repeat(64).bright_green());

        let grand_total = total_passed + total_failed;
        let overall_rate = if grand_total > 0 {
            (total_passed as f64 / grand_total as f64) * 100.0
        } else {
            0.0
        };
        let rate_str = format!("{:.0}%", overall_rate);
        let rate_colored = if overall_rate >= 100.0 {
            rate_str.green()
        } else if overall_rate >= 80.0 {
            rate_str.yellow()
        } else {
            rate_str.red()
        };

        println!(
            "{:<20} {:>8} {:>8} {:>8} {:>8}",
            "Total:".bold(),
            total_passed.to_string().green(),
            total_failed.to_string().red(),
            grand_total,
            rate_colored
        );

        // Print failed checks detail section
        let failed_checks: Vec<_> =
            self.check_results.iter().filter(|(_, (_, fails))| *fails > 0).collect();

        if !failed_checks.is_empty() {
            println!();
            println!("{}", "Failed Checks:".red().bold());
            let mut sorted_failures: Vec<_> = failed_checks.into_iter().collect();
            sorted_failures.sort_by_key(|(name, _)| (*name).clone());
            for (name, (passes, fails)) in sorted_failures {
                println!(
                    "  {} ({} passed, {} failed)",
                    name.red(),
                    passes.to_string().green(),
                    fails.to_string().red()
                );

                // Show failure details if available
                for detail in &self.failure_details {
                    if detail.check == *name {
                        println!(
                            "    {} {} {}",
                            "→".bright_black(),
                            detail.request.method.yellow(),
                            detail.request.url.bright_black()
                        );
                        println!(
                            "      Expected: {}  Actual status: {}",
                            detail.expected.yellow(),
                            detail.response.status.to_string().red()
                        );
                        if !detail.response.body.is_empty() {
                            let body_preview = if detail.response.body.len() > 200 {
                                format!("{}...", &detail.response.body[..200])
                            } else {
                                detail.response.body.clone()
                            };
                            println!("      Response body: {}", body_preview.bright_black());
                        }
                    }
                }
            }

            if !all_operations {
                println!();
                println!(
                    "{}",
                    "Tip: Use --conformance-all-operations (without --conformance-categories) to see which specific endpoints failed across all categories."
                        .yellow()
                );
            }

            if !self.failure_details.is_empty() {
                println!();
                println!(
                    "{}",
                    "Full failure details saved to conformance-report.json (see failure_details array)."
                        .bright_black()
                );
            }
        }

        // OWASP API Top 10 coverage section
        self.print_owasp_coverage();

        // Coverage tips for empty categories
        if !empty_categories.is_empty() {
            println!();
            println!("{}", "Coverage Tips".bold());
            println!("{}", "-".repeat(64));
            for cat in &empty_categories {
                if *cat == "Custom" {
                    continue;
                }
                println!(
                    "  {} {}: {}",
                    "->".bright_cyan(),
                    cat,
                    ConformanceFeature::category_hint(cat).bright_black()
                );
            }
            println!();
            println!(
                "{}",
                "Use --conformance-custom <file.yaml> to add custom checks for any category."
                    .bright_black()
            );
        }

        println!();
    }

    /// Print OWASP API Security Top 10 coverage based on tested features
    fn print_owasp_coverage(&self) {
        println!();
        println!("{}", "OWASP API Security Top 10 Coverage".bold());
        println!("{}", "=".repeat(64).bright_green());

        // Build a map of feature check_name → passed/failed status
        let mut feature_status: HashMap<&str, bool> = HashMap::new(); // true = all passed
        for feature in ConformanceFeature::all() {
            let check_name = feature.check_name();

            // Exact match (reference mode)
            if let Some((passes, fails)) = self.check_results.get(check_name) {
                let passed = *fails == 0 && *passes > 0;
                feature_status
                    .entry(check_name)
                    .and_modify(|prev| *prev = *prev && passed)
                    .or_insert(passed);
            } else {
                // Prefix match (path-qualified check names)
                let prefix = format!("{}:", check_name);
                for (name, (passes, fails)) in &self.check_results {
                    if name.starts_with(&prefix) {
                        let passed = *fails == 0 && *passes > 0;
                        feature_status
                            .entry(check_name)
                            .and_modify(|prev| *prev = *prev && passed)
                            .or_insert(passed);
                    }
                }
            }
        }

        for category in OwaspCategory::all() {
            let id = category.identifier();
            let name = category.short_name();

            // Find features that map to this OWASP category and were tested
            let mut tested = false;
            let mut all_passed = true;
            let mut via_categories: HashSet<&str> = HashSet::new();

            for feature in ConformanceFeature::all() {
                if !feature.related_owasp().contains(&id) {
                    continue;
                }
                if let Some(&passed) = feature_status.get(feature.check_name()) {
                    tested = true;
                    if !passed {
                        all_passed = false;
                    }
                    via_categories.insert(feature.category());
                }
            }

            let (status, via) = if !tested {
                ("-".bright_black(), String::new())
            } else {
                let mut cats: Vec<&str> = via_categories.into_iter().collect();
                cats.sort();
                let via_str = format!(" (via {})", cats.join(", "));
                if all_passed {
                    ("✓".green(), via_str)
                } else {
                    ("⚠".yellow(), format!("{} — has failures", via_str))
                }
            };

            println!("  {:<12} {:<40} {}{}", id, name, status, via);
        }
    }

    /// Get raw per-check results (for SARIF conversion)
    pub fn raw_check_results(&self) -> &HashMap<String, (u64, u64)> {
        &self.check_results
    }

    /// Overall pass rate (0.0 - 100.0)
    pub fn overall_rate(&self) -> f64 {
        let categories = self.by_category();
        let total_passed: usize = categories.values().map(|r| r.passed).sum();
        let total: usize = categories.values().map(|r| r.total()).sum();
        if total == 0 {
            0.0
        } else {
            (total_passed as f64 / total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conformance_report() {
        let json = r#"{
            "checks": {
                "param:path:string": { "passes": 1, "fails": 0 },
                "param:path:integer": { "passes": 1, "fails": 0 },
                "body:json": { "passes": 0, "fails": 1 },
                "method:GET": { "passes": 1, "fails": 0 }
            },
            "overall": { "overall_pass_rate": 0.75 }
        }"#;

        let report = ConformanceReport::from_json(json).unwrap();
        let categories = report.by_category();

        let params = categories.get("Parameters").unwrap();
        assert_eq!(params.passed, 2);

        let bodies = categories.get("Request Bodies").unwrap();
        assert_eq!(bodies.failed, 1);
    }

    #[test]
    fn test_empty_report() {
        let json = r#"{ "checks": {} }"#;
        let report = ConformanceReport::from_json(json).unwrap();
        assert_eq!(report.overall_rate(), 0.0);
    }

    #[test]
    fn test_owasp_coverage_with_failures() {
        // response:404 maps to API8 + API9, body:json maps to API4 + API8
        // response:404 fails, so API8 and API9 should show as having failures
        // body:json passes, so API4 should show as passing
        let json = r#"{
            "checks": {
                "response:404": { "passes": 0, "fails": 1 },
                "body:json": { "passes": 1, "fails": 0 },
                "method:GET": { "passes": 1, "fails": 0 }
            },
            "overall": {}
        }"#;

        let report = ConformanceReport::from_json(json).unwrap();
        // Print the report to verify visually (--nocapture)
        report.print_report_with_options(false);
    }
}
