//! SARIF 2.1.0 report output for conformance testing
//!
//! Converts conformance test results into SARIF format for CI/CD integration,
//! GitHub Code Scanning, and VS Code SARIF Viewer.

use super::report::ConformanceReport;
use super::spec::ConformanceFeature;
use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// SARIF report generator for conformance results
pub struct ConformanceSarifReport;

impl ConformanceSarifReport {
    /// Convert a ConformanceReport into SARIF 2.1.0 JSON
    pub fn from_conformance_report(
        report: &ConformanceReport,
        target_url: &str,
    ) -> serde_json::Value {
        let sarif = Self::build_sarif(report, target_url);
        serde_json::to_value(sarif).unwrap_or_default()
    }

    /// Write SARIF report to a file
    pub fn write(report: &ConformanceReport, target_url: &str, path: &Path) -> Result<()> {
        let sarif = Self::build_sarif(report, target_url);
        let json = serde_json::to_string_pretty(&sarif)
            .map_err(|e| BenchError::Other(format!("Failed to serialize SARIF: {}", e)))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
            .map_err(|e| BenchError::Other(format!("Failed to write SARIF report: {}", e)))
    }

    fn build_sarif(report: &ConformanceReport, target_url: &str) -> SarifReport {
        let mut results = Vec::new();
        let mut rules = Vec::new();
        let mut rule_ids: HashSet<String> = HashSet::new();

        let check_results = report.raw_check_results();

        for feature in ConformanceFeature::all() {
            let check_name = feature.check_name();
            let rule_id = check_name.to_string();

            // Add rule definition if not already present
            if rule_ids.insert(rule_id.clone()) {
                rules.push(SarifRule {
                    id: rule_id.clone(),
                    name: format!("{:?}", feature),
                    short_description: SarifMessage {
                        text: format!("{} - {}", feature.category(), check_name),
                    },
                    full_description: SarifMessage {
                        text: format!(
                            "OpenAPI 3.0.0 conformance check: {} (category: {})",
                            check_name,
                            feature.category()
                        ),
                    },
                    help: SarifMessage {
                        text: format!(
                            "See the OpenAPI 3.0.0 specification: {}",
                            feature.spec_url()
                        ),
                    },
                    default_configuration: SarifConfiguration {
                        level: "note".to_string(),
                    },
                });
            }

            // Add result if the check was actually run
            if let Some((passes, fails)) = check_results.get(check_name) {
                let passed = *fails == 0 && *passes > 0;
                let level = if passed { "note" } else { "error" };
                let message = if passed {
                    format!("PASSED: {} (category: {})", check_name, feature.category())
                } else {
                    format!("FAILED: {} (category: {})", check_name, feature.category())
                };

                results.push(SarifResult {
                    rule_id: rule_id.clone(),
                    level: level.to_string(),
                    message: SarifMessage { text: message },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: target_url.to_string(),
                            },
                        },
                    }],
                });
            }
        }

        SarifReport {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "mockforge-conformance".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        information_uri: "https://github.com/SaaSy-Solutions/mockforge".to_string(),
                        rules,
                    },
                },
                results,
            }],
        }
    }
}

// SARIF 2.1.0 format structures

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: String,
    version: String,
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: String,
    name: String,
    short_description: SarifMessage,
    full_description: SarifMessage,
    help: SarifMessage,
    default_configuration: SarifConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifConfiguration {
    level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(json: &str) -> ConformanceReport {
        ConformanceReport::from_json(json).unwrap()
    }

    #[test]
    fn test_sarif_structure() {
        let report = make_report(
            r#"{
            "checks": {
                "param:path:string": { "passes": 1, "fails": 0 },
                "method:GET": { "passes": 1, "fails": 0 }
            },
            "overall": { "overall_pass_rate": 1.0 }
        }"#,
        );

        let sarif =
            ConformanceSarifReport::from_conformance_report(&report, "http://localhost:3000");

        assert_eq!(sarif["version"], "2.1.0");
        assert!(sarif["$schema"].as_str().unwrap().contains("sarif-schema-2.1.0"));
        assert!(sarif["runs"].is_array());
        assert_eq!(sarif["runs"].as_array().unwrap().len(), 1);

        let run = &sarif["runs"][0];
        assert_eq!(run["tool"]["driver"]["name"], "mockforge-conformance");
        assert!(run["results"].is_array());
    }

    #[test]
    fn test_sarif_severity_levels() {
        let report = make_report(
            r#"{
            "checks": {
                "param:path:string": { "passes": 1, "fails": 0 },
                "body:json": { "passes": 0, "fails": 1 }
            },
            "overall": { "overall_pass_rate": 0.5 }
        }"#,
        );

        let sarif =
            ConformanceSarifReport::from_conformance_report(&report, "http://localhost:3000");

        let results = sarif["runs"][0]["results"].as_array().unwrap();

        // Find the passing result
        let passed = results.iter().find(|r| r["ruleId"] == "param:path:string").unwrap();
        assert_eq!(passed["level"], "note");
        assert!(passed["message"]["text"].as_str().unwrap().starts_with("PASSED"));

        // Find the failing result
        let failed = results.iter().find(|r| r["ruleId"] == "body:json").unwrap();
        assert_eq!(failed["level"], "error");
        assert!(failed["message"]["text"].as_str().unwrap().starts_with("FAILED"));
    }

    #[test]
    fn test_sarif_rules() {
        let report = make_report(
            r#"{
            "checks": {
                "param:path:string": { "passes": 1, "fails": 0 }
            },
            "overall": { "overall_pass_rate": 1.0 }
        }"#,
        );

        let sarif =
            ConformanceSarifReport::from_conformance_report(&report, "http://localhost:3000");

        let rules = sarif["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert!(!rules.is_empty());

        // Rules should include param:path:string
        let rule = rules.iter().find(|r| r["id"] == "param:path:string").unwrap();
        assert!(rule["help"]["text"].as_str().unwrap().contains("openapis.org"));
    }

    #[test]
    fn test_sarif_locations() {
        let report = make_report(
            r#"{
            "checks": {
                "method:GET": { "passes": 1, "fails": 0 }
            },
            "overall": {}
        }"#,
        );

        let sarif =
            ConformanceSarifReport::from_conformance_report(&report, "https://api.example.com");

        let result = &sarif["runs"][0]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["ruleId"] == "method:GET")
            .unwrap();
        assert_eq!(
            result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "https://api.example.com"
        );
    }

    #[test]
    fn test_sarif_empty_report() {
        let report = make_report(r#"{ "checks": {} }"#);
        let sarif =
            ConformanceSarifReport::from_conformance_report(&report, "http://localhost:3000");

        assert_eq!(sarif["version"], "2.1.0");
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty()); // No results when no checks were run
    }

    #[test]
    fn test_sarif_write_and_read_roundtrip() {
        let report = make_report(
            r#"{
            "checks": {
                "param:path:string": { "passes": 1, "fails": 0 },
                "body:json": { "passes": 0, "fails": 1 },
                "method:GET": { "passes": 1, "fails": 0 }
            },
            "overall": {}
        }"#,
        );

        let dir = std::env::temp_dir().join("mf-sarif-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("conformance.sarif");

        ConformanceSarifReport::write(&report, "http://localhost:3000", &path).unwrap();

        // Read back and validate
        let content = std::fs::read_to_string(&path).unwrap();
        let sarif: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(sarif["version"], "2.1.0");
        assert!(sarif["$schema"].as_str().unwrap().contains("sarif-schema-2.1.0"));

        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 3);

        // Verify passed/failed levels
        let passed: Vec<_> = results.iter().filter(|r| r["level"] == "note").collect();
        let failed: Vec<_> = results.iter().filter(|r| r["level"] == "error").collect();
        assert_eq!(passed.len(), 2);
        assert_eq!(failed.len(), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
