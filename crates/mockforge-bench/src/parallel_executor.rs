//! Parallel execution engine for multi-target bench testing
//!
//! Executes load tests against multiple targets in parallel with configurable
//! concurrency limits. Uses tokio for async execution and semaphores for
//! backpressure control.

use crate::command::BenchCommand;
use crate::error::{BenchError, Result};
use crate::executor::{K6Executor, K6Results};
use crate::k6_gen::{K6Config, K6ScriptGenerator};
use crate::reporter::TerminalReporter;
use crate::request_gen::RequestGenerator;
use crate::scenarios::LoadScenario;
use crate::spec_parser::SpecParser;
use crate::target_parser::TargetConfig;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

/// Result for a single target execution
#[derive(Debug, Clone)]
pub struct TargetResult {
    /// Target URL that was tested
    pub target_url: String,
    /// Index of the target (for ordering)
    pub target_index: usize,
    /// k6 test results
    pub results: K6Results,
    /// Output directory for this target
    pub output_dir: PathBuf,
    /// Whether the test succeeded
    pub success: bool,
    /// Error message if test failed
    pub error: Option<String>,
}

/// Aggregated results from all target executions
#[derive(Debug, Clone)]
pub struct AggregatedResults {
    /// Results for each target
    pub target_results: Vec<TargetResult>,
    /// Overall statistics
    pub total_targets: usize,
    pub successful_targets: usize,
    pub failed_targets: usize,
    /// Aggregated metrics across all targets
    pub aggregated_metrics: AggregatedMetrics,
}

/// Aggregated metrics across all targets
#[derive(Debug, Clone)]
pub struct AggregatedMetrics {
    /// Total requests across all targets
    pub total_requests: u64,
    /// Total failed requests across all targets
    pub total_failed_requests: u64,
    /// Average response time across all targets (ms)
    pub avg_duration_ms: f64,
    /// p95 response time across all targets (ms)
    pub p95_duration_ms: f64,
    /// p99 response time across all targets (ms)
    pub p99_duration_ms: f64,
    /// Overall error rate percentage
    pub error_rate: f64,
    /// Total RPS across all targets
    pub total_rps: f64,
    /// Average RPS per target
    pub avg_rps: f64,
    /// Total max VUs across all targets
    pub total_vus_max: u32,
}

impl AggregatedMetrics {
    /// Calculate aggregated metrics from target results
    fn from_results(results: &[TargetResult]) -> Self {
        let mut total_requests = 0u64;
        let mut total_failed_requests = 0u64;
        let mut durations = Vec::new();
        let mut p95_values = Vec::new();
        let mut p99_values = Vec::new();
        let mut total_rps = 0.0f64;
        let mut total_vus_max = 0u32;
        let mut successful_count = 0usize;

        for result in results {
            if result.success {
                total_requests += result.results.total_requests;
                total_failed_requests += result.results.failed_requests;
                durations.push(result.results.avg_duration_ms);
                p95_values.push(result.results.p95_duration_ms);
                p99_values.push(result.results.p99_duration_ms);
                total_rps += result.results.rps;
                total_vus_max += result.results.vus_max;
                successful_count += 1;
            }
        }

        let avg_duration_ms = if !durations.is_empty() {
            durations.iter().sum::<f64>() / durations.len() as f64
        } else {
            0.0
        };

        let p95_duration_ms = if !p95_values.is_empty() {
            p95_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let index = (p95_values.len() as f64 * 0.95).ceil() as usize - 1;
            p95_values[index.min(p95_values.len() - 1)]
        } else {
            0.0
        };

        let p99_duration_ms = if !p99_values.is_empty() {
            p99_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let index = (p99_values.len() as f64 * 0.99).ceil() as usize - 1;
            p99_values[index.min(p99_values.len() - 1)]
        } else {
            0.0
        };

        let error_rate = if total_requests > 0 {
            (total_failed_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let avg_rps = if successful_count > 0 {
            total_rps / successful_count as f64
        } else {
            0.0
        };

        Self {
            total_requests,
            total_failed_requests,
            avg_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_rate,
            total_rps,
            avg_rps,
            total_vus_max,
        }
    }
}

/// Parallel executor for multi-target bench testing
pub struct ParallelExecutor {
    /// Base command configuration (shared across all targets)
    base_command: BenchCommand,
    /// List of targets to test
    targets: Vec<TargetConfig>,
    /// Maximum number of concurrent executions
    max_concurrency: usize,
    /// Base output directory
    base_output: PathBuf,
}

impl ParallelExecutor {
    /// Create a new parallel executor
    pub fn new(
        base_command: BenchCommand,
        targets: Vec<TargetConfig>,
        max_concurrency: usize,
    ) -> Self {
        let base_output = base_command.output.clone();
        Self {
            base_command,
            targets,
            max_concurrency,
            base_output,
        }
    }

    /// Execute tests against all targets in parallel
    pub async fn execute_all(&self) -> Result<AggregatedResults> {
        let total_targets = self.targets.len();
        TerminalReporter::print_progress(&format!(
            "Starting parallel execution for {} targets (max concurrency: {})",
            total_targets, self.max_concurrency
        ));

        // Validate k6 installation
        if !K6Executor::is_k6_installed() {
            TerminalReporter::print_error("k6 is not installed");
            TerminalReporter::print_warning(
                "Install k6 from: https://k6.io/docs/get-started/installation/",
            );
            return Err(BenchError::K6NotFound);
        }

        // Load and parse spec(s) (shared across all targets)
        TerminalReporter::print_progress("Loading OpenAPI specification(s)...");
        let merged_spec = self.base_command.load_and_merge_specs().await?;
        let parser = SpecParser::from_spec(merged_spec);
        TerminalReporter::print_success("Specification(s) loaded");

        // Get operations
        let operations = if let Some(filter) = &self.base_command.operations {
            parser.filter_operations(filter)?
        } else {
            parser.get_operations()
        };

        if operations.is_empty() {
            return Err(BenchError::Other("No operations found in spec".to_string()));
        }

        TerminalReporter::print_success(&format!("Found {} operations", operations.len()));

        // Generate request templates (shared across all targets)
        TerminalReporter::print_progress("Generating request templates...");
        let templates: Vec<_> = operations
            .iter()
            .map(RequestGenerator::generate_template)
            .collect::<Result<Vec<_>>>()?;
        TerminalReporter::print_success("Request templates generated");

        // Pre-load per-target specs
        let mut per_target_data: HashMap<
            PathBuf,
            (Vec<crate::request_gen::RequestTemplate>, Option<String>),
        > = HashMap::new();
        {
            let mut unique_specs: Vec<PathBuf> = Vec::new();
            for t in &self.targets {
                if let Some(spec_path) = &t.spec {
                    if !unique_specs.contains(spec_path) {
                        unique_specs.push(spec_path.clone());
                    }
                }
            }
            for spec_path in &unique_specs {
                TerminalReporter::print_progress(&format!(
                    "Loading per-target spec: {}",
                    spec_path.display()
                ));
                match SpecParser::from_file(spec_path).await {
                    Ok(target_parser) => {
                        let target_ops = if let Some(filter) = &self.base_command.operations {
                            match target_parser.filter_operations(filter) {
                                Ok(ops) => ops,
                                Err(e) => {
                                    TerminalReporter::print_warning(&format!(
                                        "Failed to filter operations from {}: {}. Using shared spec.",
                                        spec_path.display(),
                                        e
                                    ));
                                    continue;
                                }
                            }
                        } else {
                            target_parser.get_operations()
                        };
                        let target_templates: Vec<_> = match target_ops
                            .iter()
                            .map(RequestGenerator::generate_template)
                            .collect::<Result<Vec<_>>>()
                        {
                            Ok(t) => t,
                            Err(e) => {
                                TerminalReporter::print_warning(&format!(
                                    "Failed to generate templates from {}: {}. Using shared spec.",
                                    spec_path.display(),
                                    e
                                ));
                                continue;
                            }
                        };
                        let target_base_path = if let Some(cli_bp) = &self.base_command.base_path {
                            if cli_bp.is_empty() {
                                None
                            } else {
                                Some(cli_bp.clone())
                            }
                        } else {
                            target_parser.get_base_path()
                        };
                        TerminalReporter::print_success(&format!(
                            "Loaded {} operations from {}",
                            target_templates.len(),
                            spec_path.display()
                        ));
                        per_target_data
                            .insert(spec_path.clone(), (target_templates, target_base_path));
                    }
                    Err(e) => {
                        TerminalReporter::print_warning(&format!(
                            "Failed to load per-target spec {}: {}. Targets using this spec will use the shared spec.",
                            spec_path.display(),
                            e
                        ));
                    }
                }
            }
        }

        // Parse base headers
        let base_headers = self.base_command.parse_headers()?;

        // Resolve base path (CLI option takes priority over spec's servers URL)
        let base_path = self.resolve_base_path(&parser);
        if let Some(ref bp) = base_path {
            TerminalReporter::print_progress(&format!("Using base path: {}", bp));
        }

        // Parse scenario
        let scenario = LoadScenario::from_str(&self.base_command.scenario)
            .map_err(BenchError::InvalidScenario)?;

        let duration_secs_val = BenchCommand::parse_duration(&self.base_command.duration)?;

        // Compute security testing flag
        let security_testing_enabled_val =
            self.base_command.security_test || self.base_command.wafbench_dir.is_some();

        // Pre-compute enhancement code once (same for all targets)
        let has_advanced_features = self.base_command.data_file.is_some()
            || self.base_command.error_rate.is_some()
            || self.base_command.security_test
            || self.base_command.parallel_create.is_some()
            || self.base_command.wafbench_dir.is_some();

        let enhancement_code = if has_advanced_features {
            let dummy_script = "export const options = {};";
            let enhanced = self.base_command.generate_enhanced_script(dummy_script)?;
            if let Some(pos) = enhanced.find("export const options") {
                enhanced[..pos].to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let multi_progress = MultiProgress::new();

        // Create progress bars for each target
        let progress_bars: Vec<ProgressBar> = (0..total_targets)
            .map(|i| {
                let pb = multi_progress.add(ProgressBar::new(1));
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
                        .unwrap(),
                );
                pb.set_message(format!("Target {}", i + 1));
                pb
            })
            .collect();

        // Spawn tasks for each target
        let mut handles: Vec<JoinHandle<Result<TargetResult>>> = Vec::new();

        for (index, target) in self.targets.iter().enumerate() {
            let target = target.clone();
            // Clone necessary fields from base_command instead of passing reference
            let duration = self.base_command.duration.clone();
            let vus = self.base_command.vus;
            let scenario_str = self.base_command.scenario.clone();
            let operations = self.base_command.operations.clone();
            let auth = self.base_command.auth.clone();
            let headers = self.base_command.headers.clone();
            let threshold_percentile = self.base_command.threshold_percentile.clone();
            let threshold_ms = self.base_command.threshold_ms;
            let max_error_rate = self.base_command.max_error_rate;
            let verbose = self.base_command.verbose;
            let skip_tls_verify = self.base_command.skip_tls_verify;

            // Select per-target templates/base_path if this target has a custom spec
            let (templates, base_path) = if let Some(spec_path) = &target.spec {
                if let Some((t, bp)) = per_target_data.get(spec_path) {
                    (t.clone(), bp.clone())
                } else {
                    (templates.clone(), base_path.clone())
                }
            } else {
                (templates.clone(), base_path.clone())
            };

            let base_headers = base_headers.clone();
            let scenario = scenario.clone();
            let duration_secs = duration_secs_val;
            let base_output = self.base_output.clone();
            let semaphore = semaphore.clone();
            let progress_bar = progress_bars[index].clone();
            let target_index = index;
            let security_testing_enabled = security_testing_enabled_val;
            let enhancement_code = enhancement_code.clone();

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.map_err(|e| {
                    BenchError::Other(format!("Failed to acquire semaphore: {}", e))
                })?;

                progress_bar.set_message(format!("Testing {}", target.url));

                // Execute test for this target
                let result = Self::execute_single_target_internal(
                    &duration,
                    vus,
                    &scenario_str,
                    &operations,
                    &auth,
                    &headers,
                    &threshold_percentile,
                    threshold_ms,
                    max_error_rate,
                    verbose,
                    skip_tls_verify,
                    base_path.as_ref(),
                    &target,
                    target_index,
                    &templates,
                    &base_headers,
                    &scenario,
                    duration_secs,
                    &base_output,
                    security_testing_enabled,
                    &enhancement_code,
                )
                .await;

                progress_bar.inc(1);
                progress_bar.finish_with_message(format!("Completed {}", target.url));

                result
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete and collect results
        let mut target_results = Vec::new();
        for (index, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(result)) => {
                    target_results.push(result);
                }
                Ok(Err(e)) => {
                    // Create error result
                    let target_url = self.targets[index].url.clone();
                    target_results.push(TargetResult {
                        target_url: target_url.clone(),
                        target_index: index,
                        results: K6Results::default(),
                        output_dir: self.base_output.join(format!("target_{}", index + 1)),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
                Err(e) => {
                    // Join error
                    let target_url = self.targets[index].url.clone();
                    target_results.push(TargetResult {
                        target_url: target_url.clone(),
                        target_index: index,
                        results: K6Results::default(),
                        output_dir: self.base_output.join(format!("target_{}", index + 1)),
                        success: false,
                        error: Some(format!("Task join error: {}", e)),
                    });
                }
            }
        }

        // Sort results by target index
        target_results.sort_by_key(|r| r.target_index);

        // Calculate aggregated metrics
        let aggregated_metrics = AggregatedMetrics::from_results(&target_results);

        let successful_targets = target_results.iter().filter(|r| r.success).count();
        let failed_targets = total_targets - successful_targets;

        Ok(AggregatedResults {
            target_results,
            total_targets,
            successful_targets,
            failed_targets,
            aggregated_metrics,
        })
    }

    /// Resolve the effective base path for API endpoints
    fn resolve_base_path(&self, parser: &SpecParser) -> Option<String> {
        // CLI option takes priority (including empty string to disable)
        if let Some(cli_base_path) = &self.base_command.base_path {
            if cli_base_path.is_empty() {
                return None;
            }
            return Some(cli_base_path.clone());
        }
        // Fall back to spec's base path
        parser.get_base_path()
    }

    /// Execute a single target test (internal method that doesn't require BenchCommand)
    #[allow(clippy::too_many_arguments)]
    async fn execute_single_target_internal(
        _duration: &str,
        vus: u32,
        _scenario_str: &str,
        _operations: &Option<String>,
        auth: &Option<String>,
        _headers: &Option<String>,
        threshold_percentile: &str,
        threshold_ms: u64,
        max_error_rate: f64,
        verbose: bool,
        skip_tls_verify: bool,
        base_path: Option<&String>,
        target: &TargetConfig,
        target_index: usize,
        templates: &[crate::request_gen::RequestTemplate],
        base_headers: &HashMap<String, String>,
        scenario: &LoadScenario,
        duration_secs: u64,
        base_output: &Path,
        security_testing_enabled: bool,
        enhancement_code: &str,
    ) -> Result<TargetResult> {
        // Merge target-specific headers with base headers
        let mut custom_headers = base_headers.clone();
        if let Some(target_headers) = &target.headers {
            custom_headers.extend(target_headers.clone());
        }

        // Use target-specific auth if provided, otherwise use base auth
        let auth_header = target.auth.as_ref().or(auth.as_ref()).cloned();

        // Create k6 config for this target
        let k6_config = K6Config {
            target_url: target.url.clone(),
            base_path: base_path.cloned(),
            scenario: scenario.clone(),
            duration_secs,
            max_vus: vus,
            threshold_percentile: threshold_percentile.to_string(),
            threshold_ms,
            max_error_rate,
            auth_header,
            custom_headers,
            skip_tls_verify,
            security_testing_enabled,
        };

        // Generate k6 script
        let generator = K6ScriptGenerator::new(k6_config, templates.to_vec());
        let mut script = generator.generate()?;

        // Apply pre-computed enhancement code (security definitions, etc.)
        if !enhancement_code.is_empty() {
            if let Some(pos) = script.find("export const options") {
                script.insert_str(pos, enhancement_code);
            }
        }

        // Validate script
        let validation_errors = K6ScriptGenerator::validate_script(&script);
        if !validation_errors.is_empty() {
            return Err(BenchError::Other(format!(
                "Script validation failed for target {}: {}",
                target.url,
                validation_errors.join(", ")
            )));
        }

        // Create output directory for this target
        let output_dir = base_output.join(format!("target_{}", target_index + 1));
        std::fs::create_dir_all(&output_dir)?;

        // Write script to file
        let script_path = output_dir.join("k6-script.js");
        std::fs::write(&script_path, script)?;

        // Execute k6
        let executor = K6Executor::new()?;
        let results = executor.execute(&script_path, Some(&output_dir), verbose).await;

        match results {
            Ok(k6_results) => Ok(TargetResult {
                target_url: target.url.clone(),
                target_index,
                results: k6_results,
                output_dir,
                success: true,
                error: None,
            }),
            Err(e) => Ok(TargetResult {
                target_url: target.url.clone(),
                target_index,
                results: K6Results::default(),
                output_dir,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_metrics_from_results() {
        let results = vec![
            TargetResult {
                target_url: "http://api1.com".to_string(),
                target_index: 0,
                results: K6Results {
                    total_requests: 100,
                    failed_requests: 5,
                    avg_duration_ms: 100.0,
                    p95_duration_ms: 200.0,
                    p99_duration_ms: 300.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output1"),
                success: true,
                error: None,
            },
            TargetResult {
                target_url: "http://api2.com".to_string(),
                target_index: 1,
                results: K6Results {
                    total_requests: 200,
                    failed_requests: 10,
                    avg_duration_ms: 150.0,
                    p95_duration_ms: 250.0,
                    p99_duration_ms: 350.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output2"),
                success: true,
                error: None,
            },
        ];

        let metrics = AggregatedMetrics::from_results(&results);
        assert_eq!(metrics.total_requests, 300);
        assert_eq!(metrics.total_failed_requests, 15);
        assert_eq!(metrics.avg_duration_ms, 125.0); // (100 + 150) / 2
    }

    #[test]
    fn test_aggregated_metrics_with_failed_targets() {
        let results = vec![
            TargetResult {
                target_url: "http://api1.com".to_string(),
                target_index: 0,
                results: K6Results {
                    total_requests: 100,
                    failed_requests: 5,
                    avg_duration_ms: 100.0,
                    p95_duration_ms: 200.0,
                    p99_duration_ms: 300.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output1"),
                success: true,
                error: None,
            },
            TargetResult {
                target_url: "http://api2.com".to_string(),
                target_index: 1,
                results: K6Results::default(),
                output_dir: PathBuf::from("output2"),
                success: false,
                error: Some("Network error".to_string()),
            },
        ];

        let metrics = AggregatedMetrics::from_results(&results);
        // Only successful target should be counted
        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.total_failed_requests, 5);
        assert_eq!(metrics.avg_duration_ms, 100.0);
    }

    #[test]
    fn test_aggregated_metrics_empty_results() {
        let results = vec![];
        let metrics = AggregatedMetrics::from_results(&results);
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_failed_requests, 0);
        assert_eq!(metrics.avg_duration_ms, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
    }

    #[test]
    fn test_aggregated_metrics_error_rate_calculation() {
        let results = vec![TargetResult {
            target_url: "http://api1.com".to_string(),
            target_index: 0,
            results: K6Results {
                total_requests: 1000,
                failed_requests: 50,
                avg_duration_ms: 100.0,
                p95_duration_ms: 200.0,
                p99_duration_ms: 300.0,
                ..Default::default()
            },
            output_dir: PathBuf::from("output1"),
            success: true,
            error: None,
        }];

        let metrics = AggregatedMetrics::from_results(&results);
        assert_eq!(metrics.error_rate, 5.0); // 50/1000 * 100
    }

    #[test]
    fn test_aggregated_metrics_p95_p99_calculation() {
        let results = vec![
            TargetResult {
                target_url: "http://api1.com".to_string(),
                target_index: 0,
                results: K6Results {
                    total_requests: 100,
                    failed_requests: 0,
                    avg_duration_ms: 100.0,
                    p95_duration_ms: 150.0,
                    p99_duration_ms: 200.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output1"),
                success: true,
                error: None,
            },
            TargetResult {
                target_url: "http://api2.com".to_string(),
                target_index: 1,
                results: K6Results {
                    total_requests: 100,
                    failed_requests: 0,
                    avg_duration_ms: 200.0,
                    p95_duration_ms: 250.0,
                    p99_duration_ms: 300.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output2"),
                success: true,
                error: None,
            },
            TargetResult {
                target_url: "http://api3.com".to_string(),
                target_index: 2,
                results: K6Results {
                    total_requests: 100,
                    failed_requests: 0,
                    avg_duration_ms: 300.0,
                    p95_duration_ms: 350.0,
                    p99_duration_ms: 400.0,
                    ..Default::default()
                },
                output_dir: PathBuf::from("output3"),
                success: true,
                error: None,
            },
        ];

        let metrics = AggregatedMetrics::from_results(&results);
        // p95 should be the 95th percentile of [150, 250, 350] = index 2 = 350
        // p99 should be the 99th percentile of [200, 300, 400] = index 2 = 400
        assert_eq!(metrics.p95_duration_ms, 350.0);
        assert_eq!(metrics.p99_duration_ms, 400.0);
    }
}
