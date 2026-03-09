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

/// Conformance test report
pub struct ConformanceReport {
    /// Per-check results: check_name -> (passes, fails)
    check_results: HashMap<String, (u64, u64)>,
}

impl ConformanceReport {
    /// Parse a conformance report from k6's handleSummary JSON output
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BenchError::Other(format!("Failed to read conformance report: {}", e)))?;
        Self::from_json(&content)
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

        Ok(Self { check_results })
    }

    /// Get results grouped by category
    pub fn by_category(&self) -> HashMap<&'static str, CategoryResult> {
        let mut categories: HashMap<&'static str, CategoryResult> = HashMap::new();

        // Initialize all categories
        for cat in ConformanceFeature::categories() {
            categories.insert(cat, CategoryResult::default());
        }

        // Map check results to features.
        // In --conformance-all-operations mode, check names are path-qualified
        // (e.g. "constraint:required:/users") so we match by prefix as well as
        // exact name.
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
                // Try prefix match (all-operations mode: "constraint:required:/path")
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

        categories
    }

    /// Print the conformance report to stdout
    pub fn print_report(&self) {
        self.print_report_with_options(false);
    }

    /// Print the conformance report with options controlling detail level
    pub fn print_report_with_options(&self, all_operations: bool) {
        let categories = self.by_category();

        println!("\n{}", "OpenAPI 3.0.0 Conformance Report".bold());
        println!("{}", "=".repeat(64).bright_green());

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

        for cat_name in ConformanceFeature::categories() {
            if let Some(result) = categories.get(cat_name) {
                let total = result.total();
                if total == 0 {
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
            }

            if !all_operations {
                println!();
                println!(
                    "{}",
                    "Tip: Use --conformance-all-operations (without --conformance-categories) to see which specific endpoints failed across all categories."
                        .yellow()
                );
            }
        }

        // OWASP API Top 10 coverage section
        self.print_owasp_coverage();

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
                // Prefix match (all-operations mode)
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
