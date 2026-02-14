//! Conformance test report parsing and display

use super::spec::ConformanceFeature;
use crate::error::{BenchError, Result};
use colored::*;
use std::collections::HashMap;
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

        // Map check results to features
        for feature in ConformanceFeature::all() {
            let check_name = feature.check_name();
            let category = feature.category();

            let entry = categories.entry(category).or_default();

            if let Some((passes, fails)) = self.check_results.get(check_name) {
                if *fails == 0 && *passes > 0 {
                    entry.passed += 1;
                } else {
                    entry.failed += 1;
                }
            }
            // Features not in results are not counted (not tested)
        }

        categories
    }

    /// Print the conformance report to stdout
    pub fn print_report(&self) {
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
        println!();
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
}
