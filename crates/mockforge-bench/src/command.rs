//! Bench command implementation

use crate::error::{BenchError, Result};
use crate::executor::K6Executor;
use crate::k6_gen::{K6Config, K6ScriptGenerator};
use crate::parallel_executor::{AggregatedResults, ParallelExecutor};
use crate::param_overrides::ParameterOverrides;
use crate::reporter::TerminalReporter;
use crate::request_gen::RequestGenerator;
use crate::scenarios::LoadScenario;
use crate::spec_parser::SpecParser;
use crate::target_parser::parse_targets_file;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Bench command configuration
pub struct BenchCommand {
    pub spec: PathBuf,
    pub target: String,
    pub duration: String,
    pub vus: u32,
    pub scenario: String,
    pub operations: Option<String>,
    pub auth: Option<String>,
    pub headers: Option<String>,
    pub output: PathBuf,
    pub generate_only: bool,
    pub script_output: Option<PathBuf>,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub verbose: bool,
    pub skip_tls_verify: bool,
    /// Optional file containing multiple targets
    pub targets_file: Option<PathBuf>,
    /// Maximum number of parallel executions (for multi-target mode)
    pub max_concurrency: Option<u32>,
    /// Results format: "per-target", "aggregated", or "both"
    pub results_format: String,
    /// Optional file containing parameter value overrides (JSON or YAML)
    ///
    /// Allows users to provide custom values for path parameters, query parameters,
    /// headers, and request bodies instead of auto-generated placeholder values.
    pub params_file: Option<PathBuf>,
}

impl BenchCommand {
    /// Execute the bench command
    pub async fn execute(&self) -> Result<()> {
        // Check if we're in multi-target mode
        if let Some(targets_file) = &self.targets_file {
            return self.execute_multi_target(targets_file).await;
        }

        // Single target mode (existing behavior)
        // Print header
        TerminalReporter::print_header(
            self.spec.to_str().unwrap(),
            &self.target,
            0, // Will be updated later
            &self.scenario,
            Self::parse_duration(&self.duration)?,
        );

        // Validate k6 installation
        if !K6Executor::is_k6_installed() {
            TerminalReporter::print_error("k6 is not installed");
            TerminalReporter::print_warning(
                "Install k6 from: https://k6.io/docs/get-started/installation/",
            );
            return Err(BenchError::K6NotFound);
        }

        // Load and parse spec
        TerminalReporter::print_progress("Loading OpenAPI specification...");
        let parser = SpecParser::from_file(&self.spec).await?;
        TerminalReporter::print_success("Specification loaded");

        // Get operations
        TerminalReporter::print_progress("Extracting API operations...");
        let operations = if let Some(filter) = &self.operations {
            parser.filter_operations(filter)?
        } else {
            parser.get_operations()
        };

        if operations.is_empty() {
            return Err(BenchError::Other("No operations found in spec".to_string()));
        }

        TerminalReporter::print_success(&format!("Found {} operations", operations.len()));

        // Load parameter overrides if provided
        let param_overrides = if let Some(params_file) = &self.params_file {
            TerminalReporter::print_progress("Loading parameter overrides...");
            let overrides = ParameterOverrides::from_file(params_file)?;
            TerminalReporter::print_success(&format!(
                "Loaded parameter overrides ({} operation-specific, {} defaults)",
                overrides.operations.len(),
                if overrides.defaults.is_empty() { 0 } else { 1 }
            ));
            Some(overrides)
        } else {
            None
        };

        // Generate request templates
        TerminalReporter::print_progress("Generating request templates...");
        let templates: Vec<_> = operations
            .iter()
            .map(|op| {
                let op_overrides = param_overrides.as_ref().map(|po| {
                    po.get_for_operation(op.operation_id.as_deref(), &op.method, &op.path)
                });
                RequestGenerator::generate_template_with_overrides(op, op_overrides.as_ref())
            })
            .collect::<Result<Vec<_>>>()?;
        TerminalReporter::print_success("Request templates generated");

        // Parse headers
        let custom_headers = self.parse_headers()?;

        // Generate k6 script
        TerminalReporter::print_progress("Generating k6 load test script...");
        let scenario =
            LoadScenario::from_str(&self.scenario).map_err(BenchError::InvalidScenario)?;

        let k6_config = K6Config {
            target_url: self.target.clone(),
            scenario,
            duration_secs: Self::parse_duration(&self.duration)?,
            max_vus: self.vus,
            threshold_percentile: self.threshold_percentile.clone(),
            threshold_ms: self.threshold_ms,
            max_error_rate: self.max_error_rate,
            auth_header: self.auth.clone(),
            custom_headers,
            skip_tls_verify: self.skip_tls_verify,
        };

        let generator = K6ScriptGenerator::new(k6_config, templates);
        let script = generator.generate()?;
        TerminalReporter::print_success("k6 script generated");

        // Validate the generated script
        TerminalReporter::print_progress("Validating k6 script...");
        let validation_errors = K6ScriptGenerator::validate_script(&script);
        if !validation_errors.is_empty() {
            TerminalReporter::print_error("Script validation failed");
            for error in &validation_errors {
                eprintln!("  {}", error);
            }
            return Err(BenchError::Other(format!(
                "Generated k6 script has {} validation error(s). Please check the output above.",
                validation_errors.len()
            )));
        }
        TerminalReporter::print_success("Script validation passed");

        // Write script to file
        let script_path = if let Some(output) = &self.script_output {
            output.clone()
        } else {
            self.output.join("k6-script.js")
        };

        std::fs::create_dir_all(script_path.parent().unwrap())?;
        std::fs::write(&script_path, script)?;
        TerminalReporter::print_success(&format!("Script written to: {}", script_path.display()));

        // If generate-only mode, exit here
        if self.generate_only {
            println!("\nScript generated successfully. Run it with:");
            println!("  k6 run {}", script_path.display());
            return Ok(());
        }

        // Execute k6
        TerminalReporter::print_progress("Executing load test...");
        let executor = K6Executor::new()?;

        std::fs::create_dir_all(&self.output)?;

        let results = executor.execute(&script_path, Some(&self.output), self.verbose).await?;

        // Print results
        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary(&results, duration_secs);

        println!("\nResults saved to: {}", self.output.display());

        Ok(())
    }

    /// Execute multi-target bench testing
    async fn execute_multi_target(&self, targets_file: &PathBuf) -> Result<()> {
        TerminalReporter::print_progress("Parsing targets file...");
        let targets = parse_targets_file(targets_file)?;
        let num_targets = targets.len();
        TerminalReporter::print_success(&format!("Loaded {} targets", num_targets));

        if targets.is_empty() {
            return Err(BenchError::Other("No targets found in file".to_string()));
        }

        // Determine max concurrency
        let max_concurrency = self.max_concurrency.unwrap_or(10) as usize;
        let max_concurrency = max_concurrency.min(num_targets); // Don't exceed number of targets

        // Print header for multi-target mode
        TerminalReporter::print_header(
            self.spec.to_str().unwrap(),
            &format!("{} targets", num_targets),
            0,
            &self.scenario,
            Self::parse_duration(&self.duration)?,
        );

        // Create parallel executor
        let executor = ParallelExecutor::new(
            BenchCommand {
                // Clone all fields except targets_file (we don't need it in the executor)
                spec: self.spec.clone(),
                target: self.target.clone(), // Not used in multi-target mode, but kept for compatibility
                duration: self.duration.clone(),
                vus: self.vus,
                scenario: self.scenario.clone(),
                operations: self.operations.clone(),
                auth: self.auth.clone(),
                headers: self.headers.clone(),
                output: self.output.clone(),
                generate_only: self.generate_only,
                script_output: self.script_output.clone(),
                threshold_percentile: self.threshold_percentile.clone(),
                threshold_ms: self.threshold_ms,
                max_error_rate: self.max_error_rate,
                verbose: self.verbose,
                skip_tls_verify: self.skip_tls_verify,
                targets_file: None,
                max_concurrency: None,
                results_format: self.results_format.clone(),
                params_file: self.params_file.clone(),
            },
            targets,
            max_concurrency,
        );

        // Execute all targets
        let aggregated_results = executor.execute_all().await?;

        // Organize and report results
        self.report_multi_target_results(&aggregated_results)?;

        Ok(())
    }

    /// Report results for multi-target execution
    fn report_multi_target_results(&self, results: &AggregatedResults) -> Result<()> {
        // Print summary
        TerminalReporter::print_multi_target_summary(results);

        // Save aggregated summary if requested
        if self.results_format == "aggregated" || self.results_format == "both" {
            let summary_path = self.output.join("aggregated_summary.json");
            let summary_json = serde_json::json!({
                "total_targets": results.total_targets,
                "successful_targets": results.successful_targets,
                "failed_targets": results.failed_targets,
                "aggregated_metrics": {
                    "total_requests": results.aggregated_metrics.total_requests,
                    "total_failed_requests": results.aggregated_metrics.total_failed_requests,
                    "avg_duration_ms": results.aggregated_metrics.avg_duration_ms,
                    "p95_duration_ms": results.aggregated_metrics.p95_duration_ms,
                    "p99_duration_ms": results.aggregated_metrics.p99_duration_ms,
                    "error_rate": results.aggregated_metrics.error_rate,
                },
                "target_results": results.target_results.iter().map(|r| {
                    serde_json::json!({
                        "target_url": r.target_url,
                        "target_index": r.target_index,
                        "success": r.success,
                        "error": r.error,
                        "total_requests": r.results.total_requests,
                        "failed_requests": r.results.failed_requests,
                        "avg_duration_ms": r.results.avg_duration_ms,
                        "p95_duration_ms": r.results.p95_duration_ms,
                        "p99_duration_ms": r.results.p99_duration_ms,
                        "output_dir": r.output_dir.to_string_lossy(),
                    })
                }).collect::<Vec<_>>(),
            });

            std::fs::write(&summary_path, serde_json::to_string_pretty(&summary_json)?)?;
            TerminalReporter::print_success(&format!(
                "Aggregated summary saved to: {}",
                summary_path.display()
            ));
        }

        println!("\nResults saved to: {}", self.output.display());
        println!("  - Per-target results: {}", self.output.join("target_*").display());
        if self.results_format == "aggregated" || self.results_format == "both" {
            println!(
                "  - Aggregated summary: {}",
                self.output.join("aggregated_summary.json").display()
            );
        }

        Ok(())
    }

    /// Parse duration string (e.g., "30s", "5m", "1h") to seconds
    pub fn parse_duration(duration: &str) -> Result<u64> {
        let duration = duration.trim();

        if let Some(secs) = duration.strip_suffix('s') {
            secs.parse::<u64>()
                .map_err(|_| BenchError::Other(format!("Invalid duration: {}", duration)))
        } else if let Some(mins) = duration.strip_suffix('m') {
            mins.parse::<u64>()
                .map(|m| m * 60)
                .map_err(|_| BenchError::Other(format!("Invalid duration: {}", duration)))
        } else if let Some(hours) = duration.strip_suffix('h') {
            hours
                .parse::<u64>()
                .map(|h| h * 3600)
                .map_err(|_| BenchError::Other(format!("Invalid duration: {}", duration)))
        } else {
            // Try parsing as seconds without suffix
            duration
                .parse::<u64>()
                .map_err(|_| BenchError::Other(format!("Invalid duration: {}", duration)))
        }
    }

    /// Parse headers from command line format (Key:Value,Key2:Value2)
    pub fn parse_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();

        if let Some(header_str) = &self.headers {
            for pair in header_str.split(',') {
                let parts: Vec<&str> = pair.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(BenchError::Other(format!(
                        "Invalid header format: '{}'. Expected 'Key:Value'",
                        pair
                    )));
                }
                headers.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }

        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(BenchCommand::parse_duration("30s").unwrap(), 30);
        assert_eq!(BenchCommand::parse_duration("5m").unwrap(), 300);
        assert_eq!(BenchCommand::parse_duration("1h").unwrap(), 3600);
        assert_eq!(BenchCommand::parse_duration("60").unwrap(), 60);
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(BenchCommand::parse_duration("invalid").is_err());
        assert!(BenchCommand::parse_duration("30x").is_err());
    }

    #[test]
    fn test_parse_headers() {
        let cmd = BenchCommand {
            spec: PathBuf::from("test.yaml"),
            target: "http://localhost".to_string(),
            duration: "1m".to_string(),
            vus: 10,
            scenario: "ramp-up".to_string(),
            operations: None,
            auth: None,
            headers: Some("X-API-Key:test123,X-Client-ID:client456".to_string()),
            output: PathBuf::from("output"),
            generate_only: false,
            script_output: None,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            verbose: false,
            skip_tls_verify: false,
            targets_file: None,
            max_concurrency: None,
            results_format: "both".to_string(),
            params_file: None,
        };

        let headers = cmd.parse_headers().unwrap();
        assert_eq!(headers.get("X-API-Key"), Some(&"test123".to_string()));
        assert_eq!(headers.get("X-Client-ID"), Some(&"client456".to_string()));
    }
}
