//! Bench command implementation

use crate::crud_flow::{CrudFlowConfig, CrudFlowDetector};
use crate::data_driven::{DataDistribution, DataDrivenConfig, DataDrivenGenerator, DataMapping};
use crate::error::{BenchError, Result};
use crate::executor::K6Executor;
use crate::invalid_data::{InvalidDataConfig, InvalidDataGenerator, InvalidDataType};
use crate::k6_gen::{K6Config, K6ScriptGenerator};
use crate::mock_integration::{
    MockIntegrationConfig, MockIntegrationGenerator, MockServerDetector,
};
use crate::parallel_executor::{AggregatedResults, ParallelExecutor};
use crate::parallel_requests::{ParallelConfig, ParallelRequestGenerator};
use crate::param_overrides::ParameterOverrides;
use crate::reporter::TerminalReporter;
use crate::request_gen::RequestGenerator;
use crate::scenarios::LoadScenario;
use crate::security_payloads::{
    SecurityCategory, SecurityPayload, SecurityPayloads, SecurityTestConfig, SecurityTestGenerator,
};
use crate::spec_dependencies::{
    topological_sort, DependencyDetector, ExtractedValues, SpecDependencyConfig,
};
use crate::spec_parser::SpecParser;
use crate::target_parser::parse_targets_file;
use crate::wafbench::WafBenchLoader;
use mockforge_core::openapi::multi_spec::{
    load_specs_from_directory, load_specs_from_files, merge_specs, ConflictStrategy,
};
use mockforge_core::openapi::spec::OpenApiSpec;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Bench command configuration
pub struct BenchCommand {
    /// OpenAPI spec file(s) - can specify multiple
    pub spec: Vec<PathBuf>,
    /// Directory containing OpenAPI spec files (discovers .json, .yaml, .yml files)
    pub spec_dir: Option<PathBuf>,
    /// Conflict resolution strategy when merging multiple specs: "error" (default), "first", "last"
    pub merge_conflicts: String,
    /// Spec mode: "merge" (default) combines all specs, "sequential" runs them in order
    pub spec_mode: String,
    /// Dependency configuration file for cross-spec value passing (used with sequential mode)
    pub dependency_config: Option<PathBuf>,
    pub target: String,
    /// API base path prefix (e.g., "/api" or "/v2/api")
    /// If None, extracts from OpenAPI spec's servers URL
    pub base_path: Option<String>,
    pub duration: String,
    pub vus: u32,
    pub scenario: String,
    pub operations: Option<String>,
    /// Exclude operations from testing (comma-separated)
    ///
    /// Supports "METHOD /path" or just "METHOD" to exclude all operations of that type.
    pub exclude_operations: Option<String>,
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

    // === CRUD Flow Options ===
    /// Enable CRUD flow mode
    pub crud_flow: bool,
    /// Custom CRUD flow configuration file
    pub flow_config: Option<PathBuf>,
    /// Fields to extract from responses
    pub extract_fields: Option<String>,

    // === Parallel Execution Options ===
    /// Number of resources to create in parallel
    pub parallel_create: Option<u32>,

    // === Data-Driven Testing Options ===
    /// Test data file (CSV or JSON)
    pub data_file: Option<PathBuf>,
    /// Data distribution strategy
    pub data_distribution: String,
    /// Data column to field mappings
    pub data_mappings: Option<String>,
    /// Enable per-URI control mode (each row specifies method, uri, body, etc.)
    pub per_uri_control: bool,

    // === Invalid Data Testing Options ===
    /// Percentage of requests with invalid data
    pub error_rate: Option<f64>,
    /// Types of invalid data to generate
    pub error_types: Option<String>,

    // === Security Testing Options ===
    /// Enable security testing
    pub security_test: bool,
    /// Custom security payloads file
    pub security_payloads: Option<PathBuf>,
    /// Security test categories
    pub security_categories: Option<String>,
    /// Fields to target for security injection
    pub security_target_fields: Option<String>,

    // === WAFBench Integration ===
    /// WAFBench test directory or glob pattern for loading CRS attack patterns
    pub wafbench_dir: Option<String>,
}

impl BenchCommand {
    /// Load and merge specs from --spec files and --spec-dir
    pub async fn load_and_merge_specs(&self) -> Result<OpenApiSpec> {
        let mut all_specs: Vec<(PathBuf, OpenApiSpec)> = Vec::new();

        // Load specs from --spec flags
        if !self.spec.is_empty() {
            let specs = load_specs_from_files(self.spec.clone())
                .await
                .map_err(|e| BenchError::Other(format!("Failed to load spec files: {}", e)))?;
            all_specs.extend(specs);
        }

        // Load specs from --spec-dir if provided
        if let Some(spec_dir) = &self.spec_dir {
            let dir_specs = load_specs_from_directory(spec_dir).await.map_err(|e| {
                BenchError::Other(format!("Failed to load specs from directory: {}", e))
            })?;
            all_specs.extend(dir_specs);
        }

        if all_specs.is_empty() {
            return Err(BenchError::Other(
                "No spec files provided. Use --spec or --spec-dir.".to_string(),
            ));
        }

        // If only one spec, return it directly (extract just the OpenApiSpec)
        if all_specs.len() == 1 {
            // Safe to unwrap because we just checked len() == 1
            return Ok(all_specs.into_iter().next().expect("checked len() == 1 above").1);
        }

        // Merge multiple specs
        let conflict_strategy = match self.merge_conflicts.as_str() {
            "first" => ConflictStrategy::First,
            "last" => ConflictStrategy::Last,
            _ => ConflictStrategy::Error,
        };

        merge_specs(all_specs, conflict_strategy)
            .map_err(|e| BenchError::Other(format!("Failed to merge specs: {}", e)))
    }

    /// Get a display name for the spec(s)
    fn get_spec_display_name(&self) -> String {
        if self.spec.len() == 1 {
            self.spec[0].to_string_lossy().to_string()
        } else if !self.spec.is_empty() {
            format!("{} spec files", self.spec.len())
        } else if let Some(dir) = &self.spec_dir {
            format!("specs from {}", dir.display())
        } else {
            "no specs".to_string()
        }
    }

    /// Execute the bench command
    pub async fn execute(&self) -> Result<()> {
        // Check if we're in multi-target mode
        if let Some(targets_file) = &self.targets_file {
            return self.execute_multi_target(targets_file).await;
        }

        // Check if we're in sequential spec mode (for dependency handling)
        if self.spec_mode == "sequential" && (self.spec.len() > 1 || self.spec_dir.is_some()) {
            return self.execute_sequential_specs().await;
        }

        // Single target mode (existing behavior)
        // Print header
        TerminalReporter::print_header(
            &self.get_spec_display_name(),
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

        // Load and parse spec(s)
        TerminalReporter::print_progress("Loading OpenAPI specification(s)...");
        let merged_spec = self.load_and_merge_specs().await?;
        let parser = SpecParser::from_spec(merged_spec);
        if self.spec.len() > 1 || self.spec_dir.is_some() {
            TerminalReporter::print_success(&format!(
                "Loaded and merged {} specification(s)",
                self.spec.len() + self.spec_dir.as_ref().map(|_| 1).unwrap_or(0)
            ));
        } else {
            TerminalReporter::print_success("Specification loaded");
        }

        // Check for mock server integration
        let mock_config = self.build_mock_config().await;
        if mock_config.is_mock_server {
            TerminalReporter::print_progress("Mock server integration enabled");
        }

        // Check for CRUD flow mode
        if self.crud_flow {
            return self.execute_crud_flow(&parser).await;
        }

        // Get operations
        TerminalReporter::print_progress("Extracting API operations...");
        let mut operations = if let Some(filter) = &self.operations {
            parser.filter_operations(filter)?
        } else {
            parser.get_operations()
        };

        // Apply exclusions if provided
        if let Some(exclude) = &self.exclude_operations {
            let before_count = operations.len();
            operations = parser.exclude_operations(operations, exclude)?;
            let excluded_count = before_count - operations.len();
            if excluded_count > 0 {
                TerminalReporter::print_progress(&format!(
                    "Excluded {} operations matching '{}'",
                    excluded_count, exclude
                ));
            }
        }

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

        // Resolve base path (CLI option takes priority over spec's servers URL)
        let base_path = self.resolve_base_path(&parser);
        if let Some(ref bp) = base_path {
            TerminalReporter::print_progress(&format!("Using base path: {}", bp));
        }

        // Generate k6 script
        TerminalReporter::print_progress("Generating k6 load test script...");
        let scenario =
            LoadScenario::from_str(&self.scenario).map_err(BenchError::InvalidScenario)?;

        let k6_config = K6Config {
            target_url: self.target.clone(),
            base_path,
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
        let mut script = generator.generate()?;
        TerminalReporter::print_success("k6 script generated");

        // Check if any advanced features are enabled
        let has_advanced_features = self.data_file.is_some()
            || self.error_rate.is_some()
            || self.security_test
            || self.parallel_create.is_some();

        // Enhance script with advanced features
        if has_advanced_features {
            script = self.generate_enhanced_script(&script)?;
        }

        // Add mock server integration code
        if mock_config.is_mock_server {
            let setup_code = MockIntegrationGenerator::generate_setup(&mock_config);
            let teardown_code = MockIntegrationGenerator::generate_teardown(&mock_config);
            let helper_code = MockIntegrationGenerator::generate_vu_id_helper();

            // Insert mock server code after imports
            if let Some(import_end) = script.find("export const options") {
                script.insert_str(
                    import_end,
                    &format!(
                        "\n// === Mock Server Integration ===\n{}\n{}\n{}\n",
                        helper_code, setup_code, teardown_code
                    ),
                );
            }
        }

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

        if let Some(parent) = script_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&script_path, &script)?;
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
            &self.get_spec_display_name(),
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
                spec_dir: self.spec_dir.clone(),
                merge_conflicts: self.merge_conflicts.clone(),
                spec_mode: self.spec_mode.clone(),
                dependency_config: self.dependency_config.clone(),
                target: self.target.clone(), // Not used in multi-target mode, but kept for compatibility
                base_path: self.base_path.clone(),
                duration: self.duration.clone(),
                vus: self.vus,
                scenario: self.scenario.clone(),
                operations: self.operations.clone(),
                exclude_operations: self.exclude_operations.clone(),
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
                crud_flow: self.crud_flow,
                flow_config: self.flow_config.clone(),
                extract_fields: self.extract_fields.clone(),
                parallel_create: self.parallel_create,
                data_file: self.data_file.clone(),
                data_distribution: self.data_distribution.clone(),
                data_mappings: self.data_mappings.clone(),
                per_uri_control: self.per_uri_control,
                error_rate: self.error_rate,
                error_types: self.error_types.clone(),
                security_test: self.security_test,
                security_payloads: self.security_payloads.clone(),
                security_categories: self.security_categories.clone(),
                security_target_fields: self.security_target_fields.clone(),
                wafbench_dir: self.wafbench_dir.clone(),
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

    /// Resolve the effective base path for API endpoints
    ///
    /// Priority:
    /// 1. CLI --base-path option (if provided, even if empty string)
    /// 2. Base path extracted from OpenAPI spec's servers URL
    /// 3. None (no base path)
    ///
    /// An empty string from CLI explicitly disables base path.
    fn resolve_base_path(&self, parser: &SpecParser) -> Option<String> {
        // CLI option takes priority (including empty string to disable)
        if let Some(cli_base_path) = &self.base_path {
            if cli_base_path.is_empty() {
                // Empty string explicitly means "no base path"
                return None;
            }
            return Some(cli_base_path.clone());
        }

        // Fall back to spec's base path
        parser.get_base_path()
    }

    /// Build mock server integration configuration
    async fn build_mock_config(&self) -> MockIntegrationConfig {
        // Check if target looks like a mock server
        if MockServerDetector::looks_like_mock_server(&self.target) {
            // Try to detect if it's actually a MockForge server
            if let Ok(info) = MockServerDetector::detect(&self.target).await {
                if info.is_mockforge {
                    TerminalReporter::print_success(&format!(
                        "Detected MockForge server (version: {})",
                        info.version.as_deref().unwrap_or("unknown")
                    ));
                    return MockIntegrationConfig::mock_server();
                }
            }
        }
        MockIntegrationConfig::real_api()
    }

    /// Build CRUD flow configuration
    fn build_crud_flow_config(&self) -> Option<CrudFlowConfig> {
        if !self.crud_flow {
            return None;
        }

        // If flow_config file is provided, load it
        if let Some(config_path) = &self.flow_config {
            match CrudFlowConfig::from_file(config_path) {
                Ok(config) => return Some(config),
                Err(e) => {
                    TerminalReporter::print_warning(&format!(
                        "Failed to load flow config: {}. Using auto-detection.",
                        e
                    ));
                }
            }
        }

        // Parse extract fields
        let extract_fields = self
            .extract_fields
            .as_ref()
            .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec!["id".to_string(), "uuid".to_string()]);

        Some(CrudFlowConfig {
            flows: Vec::new(), // Will be auto-detected
            default_extract_fields: extract_fields,
        })
    }

    /// Build data-driven testing configuration
    fn build_data_driven_config(&self) -> Option<DataDrivenConfig> {
        let data_file = self.data_file.as_ref()?;

        let distribution = DataDistribution::from_str(&self.data_distribution)
            .unwrap_or(DataDistribution::UniquePerVu);

        let mappings = self
            .data_mappings
            .as_ref()
            .map(|m| DataMapping::parse_mappings(m).unwrap_or_default())
            .unwrap_or_default();

        Some(DataDrivenConfig {
            file_path: data_file.to_string_lossy().to_string(),
            distribution,
            mappings,
            csv_has_header: true,
            per_uri_control: self.per_uri_control,
            per_uri_columns: crate::data_driven::PerUriColumns::default(),
        })
    }

    /// Build invalid data testing configuration
    fn build_invalid_data_config(&self) -> Option<InvalidDataConfig> {
        let error_rate = self.error_rate?;

        let error_types = self
            .error_types
            .as_ref()
            .map(|types| InvalidDataConfig::parse_error_types(types).unwrap_or_default())
            .unwrap_or_default();

        Some(InvalidDataConfig {
            error_rate,
            error_types,
            target_fields: Vec::new(),
        })
    }

    /// Build security testing configuration
    fn build_security_config(&self) -> Option<SecurityTestConfig> {
        if !self.security_test {
            return None;
        }

        let categories = self
            .security_categories
            .as_ref()
            .map(|cats| SecurityTestConfig::parse_categories(cats).unwrap_or_default())
            .unwrap_or_else(|| {
                let mut default = std::collections::HashSet::new();
                default.insert(SecurityCategory::SqlInjection);
                default.insert(SecurityCategory::Xss);
                default
            });

        let target_fields = self
            .security_target_fields
            .as_ref()
            .map(|fields| fields.split(',').map(|f| f.trim().to_string()).collect())
            .unwrap_or_default();

        let custom_payloads_file =
            self.security_payloads.as_ref().map(|p| p.to_string_lossy().to_string());

        Some(SecurityTestConfig {
            enabled: true,
            categories,
            target_fields,
            custom_payloads_file,
            include_high_risk: false,
        })
    }

    /// Build parallel execution configuration
    fn build_parallel_config(&self) -> Option<ParallelConfig> {
        let count = self.parallel_create?;

        Some(ParallelConfig::new(count))
    }

    /// Load WAFBench payloads from the specified directory or pattern
    fn load_wafbench_payloads(&self) -> Vec<SecurityPayload> {
        let Some(ref wafbench_dir) = self.wafbench_dir else {
            return Vec::new();
        };

        let mut loader = WafBenchLoader::new();

        if let Err(e) = loader.load_from_pattern(wafbench_dir) {
            TerminalReporter::print_warning(&format!("Failed to load WAFBench tests: {}", e));
            return Vec::new();
        }

        let stats = loader.stats();

        if stats.files_processed == 0 {
            TerminalReporter::print_warning(&format!(
                "No WAFBench YAML files found matching '{}'",
                wafbench_dir
            ));
            return Vec::new();
        }

        TerminalReporter::print_progress(&format!(
            "Loaded {} WAFBench files, {} test cases, {} payloads",
            stats.files_processed, stats.test_cases_loaded, stats.payloads_extracted
        ));

        // Print category breakdown
        for (category, count) in &stats.by_category {
            TerminalReporter::print_progress(&format!("  - {}: {} tests", category, count));
        }

        // Report any parse errors
        for error in &stats.parse_errors {
            TerminalReporter::print_warning(&format!("  Parse error: {}", error));
        }

        loader.to_security_payloads()
    }

    /// Generate enhanced k6 script with advanced features
    fn generate_enhanced_script(&self, base_script: &str) -> Result<String> {
        let mut enhanced_script = base_script.to_string();
        let mut additional_code = String::new();

        // Add data-driven testing code
        if let Some(config) = self.build_data_driven_config() {
            TerminalReporter::print_progress("Adding data-driven testing support...");
            additional_code.push_str(&DataDrivenGenerator::generate_setup(&config));
            additional_code.push('\n');
            TerminalReporter::print_success("Data-driven testing enabled");
        }

        // Add invalid data generation code
        if let Some(config) = self.build_invalid_data_config() {
            TerminalReporter::print_progress("Adding invalid data testing support...");
            additional_code.push_str(&InvalidDataGenerator::generate_invalidation_logic());
            additional_code.push('\n');
            additional_code
                .push_str(&InvalidDataGenerator::generate_should_invalidate(config.error_rate));
            additional_code.push('\n');
            additional_code
                .push_str(&InvalidDataGenerator::generate_type_selection(&config.error_types));
            additional_code.push('\n');
            TerminalReporter::print_success(&format!(
                "Invalid data testing enabled ({}% error rate)",
                (self.error_rate.unwrap_or(0.0) * 100.0) as u32
            ));
        }

        // Add security testing code
        let security_config = self.build_security_config();
        let wafbench_payloads = self.load_wafbench_payloads();

        if security_config.is_some() || !wafbench_payloads.is_empty() {
            TerminalReporter::print_progress("Adding security testing support...");

            // Combine built-in payloads with WAFBench payloads
            let mut payload_list: Vec<SecurityPayload> = Vec::new();

            if let Some(ref config) = security_config {
                payload_list.extend(SecurityPayloads::get_payloads(config));
            }

            // Add WAFBench payloads
            if !wafbench_payloads.is_empty() {
                TerminalReporter::print_progress(&format!(
                    "Loading {} WAFBench attack patterns...",
                    wafbench_payloads.len()
                ));
                payload_list.extend(wafbench_payloads);
            }

            let target_fields =
                security_config.as_ref().map(|c| c.target_fields.clone()).unwrap_or_default();

            additional_code
                .push_str(&SecurityTestGenerator::generate_payload_selection(&payload_list));
            additional_code.push('\n');
            additional_code
                .push_str(&SecurityTestGenerator::generate_apply_payload(&target_fields));
            additional_code.push('\n');
            additional_code.push_str(&SecurityTestGenerator::generate_security_checks());
            additional_code.push('\n');
            TerminalReporter::print_success(&format!(
                "Security testing enabled ({} payloads)",
                payload_list.len()
            ));
        }

        // Add parallel execution code
        if let Some(config) = self.build_parallel_config() {
            TerminalReporter::print_progress("Adding parallel execution support...");
            additional_code.push_str(&ParallelRequestGenerator::generate_batch_helper(&config));
            additional_code.push('\n');
            TerminalReporter::print_success(&format!(
                "Parallel execution enabled (count: {})",
                config.count
            ));
        }

        // Insert additional code after the imports section
        if !additional_code.is_empty() {
            // Find the end of the import section
            if let Some(import_end) = enhanced_script.find("export const options") {
                enhanced_script.insert_str(
                    import_end,
                    &format!("\n// === Advanced Testing Features ===\n{}\n", additional_code),
                );
            }
        }

        Ok(enhanced_script)
    }

    /// Execute specs sequentially with dependency ordering and value passing
    async fn execute_sequential_specs(&self) -> Result<()> {
        TerminalReporter::print_progress("Sequential spec mode: Loading specs individually...");

        // Load all specs (without merging)
        let mut all_specs: Vec<(PathBuf, OpenApiSpec)> = Vec::new();

        if !self.spec.is_empty() {
            let specs = load_specs_from_files(self.spec.clone())
                .await
                .map_err(|e| BenchError::Other(format!("Failed to load spec files: {}", e)))?;
            all_specs.extend(specs);
        }

        if let Some(spec_dir) = &self.spec_dir {
            let dir_specs = load_specs_from_directory(spec_dir).await.map_err(|e| {
                BenchError::Other(format!("Failed to load specs from directory: {}", e))
            })?;
            all_specs.extend(dir_specs);
        }

        if all_specs.is_empty() {
            return Err(BenchError::Other(
                "No spec files found for sequential execution".to_string(),
            ));
        }

        TerminalReporter::print_success(&format!("Loaded {} spec(s)", all_specs.len()));

        // Load dependency config or auto-detect
        let execution_order = if let Some(config_path) = &self.dependency_config {
            TerminalReporter::print_progress("Loading dependency configuration...");
            let config = SpecDependencyConfig::from_file(config_path)?;

            if !config.disable_auto_detect && config.execution_order.is_empty() {
                // Auto-detect if config doesn't specify order
                self.detect_and_sort_specs(&all_specs)?
            } else {
                // Use configured order
                config.execution_order.iter().flat_map(|g| g.specs.clone()).collect()
            }
        } else {
            // Auto-detect dependencies
            self.detect_and_sort_specs(&all_specs)?
        };

        TerminalReporter::print_success(&format!(
            "Execution order: {}",
            execution_order
                .iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" → ")
        ));

        // Execute each spec in order
        let mut extracted_values = ExtractedValues::new();
        let total_specs = execution_order.len();

        for (index, spec_path) in execution_order.iter().enumerate() {
            let spec_name = spec_path.file_name().unwrap_or_default().to_string_lossy().to_string();

            TerminalReporter::print_progress(&format!(
                "[{}/{}] Executing spec: {}",
                index + 1,
                total_specs,
                spec_name
            ));

            // Find the spec in our loaded specs
            let spec = all_specs
                .iter()
                .find(|(p, _)| p == spec_path)
                .map(|(_, s)| s.clone())
                .ok_or_else(|| {
                    BenchError::Other(format!("Spec not found: {}", spec_path.display()))
                })?;

            // Execute this spec with any extracted values from previous specs
            let new_values = self.execute_single_spec(&spec, &spec_name, &extracted_values).await?;

            // Merge extracted values for the next spec
            extracted_values.merge(&new_values);

            TerminalReporter::print_success(&format!(
                "[{}/{}] Completed: {} (extracted {} values)",
                index + 1,
                total_specs,
                spec_name,
                new_values.values.len()
            ));
        }

        TerminalReporter::print_success(&format!(
            "Sequential execution complete: {} specs executed",
            total_specs
        ));

        Ok(())
    }

    /// Detect dependencies and return topologically sorted spec paths
    fn detect_and_sort_specs(&self, specs: &[(PathBuf, OpenApiSpec)]) -> Result<Vec<PathBuf>> {
        TerminalReporter::print_progress("Auto-detecting spec dependencies...");

        let mut detector = DependencyDetector::new();
        let dependencies = detector.detect_dependencies(specs);

        if dependencies.is_empty() {
            TerminalReporter::print_progress("No dependencies detected, using file order");
            return Ok(specs.iter().map(|(p, _)| p.clone()).collect());
        }

        TerminalReporter::print_progress(&format!(
            "Detected {} cross-spec dependencies",
            dependencies.len()
        ));

        for dep in &dependencies {
            TerminalReporter::print_progress(&format!(
                "  {} → {} (via field '{}')",
                dep.dependency_spec.file_name().unwrap_or_default().to_string_lossy(),
                dep.dependent_spec.file_name().unwrap_or_default().to_string_lossy(),
                dep.field_name
            ));
        }

        topological_sort(specs, &dependencies)
    }

    /// Execute a single spec and extract values for dependent specs
    async fn execute_single_spec(
        &self,
        spec: &OpenApiSpec,
        spec_name: &str,
        _external_values: &ExtractedValues,
    ) -> Result<ExtractedValues> {
        let parser = SpecParser::from_spec(spec.clone());

        // For now, we execute in CRUD flow mode if enabled, otherwise standard mode
        if self.crud_flow {
            // Execute CRUD flow and extract values
            self.execute_crud_flow_with_extraction(&parser, spec_name).await
        } else {
            // Execute standard benchmark (no value extraction in non-CRUD mode)
            self.execute_standard_spec(&parser, spec_name).await?;
            Ok(ExtractedValues::new())
        }
    }

    /// Execute CRUD flow with value extraction for sequential mode
    async fn execute_crud_flow_with_extraction(
        &self,
        parser: &SpecParser,
        spec_name: &str,
    ) -> Result<ExtractedValues> {
        let operations = parser.get_operations();
        let flows = CrudFlowDetector::detect_flows(&operations);

        if flows.is_empty() {
            TerminalReporter::print_warning(&format!("No CRUD flows detected in {}", spec_name));
            return Ok(ExtractedValues::new());
        }

        TerminalReporter::print_progress(&format!(
            "  {} CRUD flow(s) in {}",
            flows.len(),
            spec_name
        ));

        // Generate and execute the CRUD flow script
        let handlebars = handlebars::Handlebars::new();
        let template = include_str!("templates/k6_crud_flow.hbs");

        let custom_headers = self.parse_headers()?;
        let config = self.build_crud_flow_config().unwrap_or_default();

        // Load parameter overrides if provided (for body configurations)
        let param_overrides = if let Some(params_file) = &self.params_file {
            let overrides = ParameterOverrides::from_file(params_file)?;
            Some(overrides)
        } else {
            None
        };

        // Generate stages from scenario
        let duration_secs = Self::parse_duration(&self.duration)?;
        let scenario =
            LoadScenario::from_str(&self.scenario).map_err(BenchError::InvalidScenario)?;
        let stages = scenario.generate_stages(duration_secs, self.vus);

        // Resolve base path (CLI option takes priority over spec's servers URL)
        let api_base_path = self.resolve_base_path(parser);

        // Build headers JSON string for the template
        let mut all_headers = custom_headers.clone();
        if let Some(auth) = &self.auth {
            all_headers.insert("Authorization".to_string(), auth.clone());
        }
        let headers_json = serde_json::to_string(&all_headers).unwrap_or_else(|_| "{}".to_string());

        let data = serde_json::json!({
            "base_url": self.target,
            "flows": flows.iter().map(|f| {
                let sanitized_name = K6ScriptGenerator::sanitize_js_identifier(&f.name);
                serde_json::json!({
                    "name": sanitized_name.clone(),
                    "display_name": f.name,
                    "base_path": f.base_path,
                    "steps": f.steps.iter().enumerate().map(|(idx, s)| {
                        // Parse operation to get method and path
                        let parts: Vec<&str> = s.operation.splitn(2, ' ').collect();
                        let method_raw = if !parts.is_empty() {
                            parts[0].to_uppercase()
                        } else {
                            "GET".to_string()
                        };
                        let method = if !parts.is_empty() {
                            let m = parts[0].to_lowercase();
                            // k6 uses 'del' for DELETE
                            if m == "delete" { "del".to_string() } else { m }
                        } else {
                            "get".to_string()
                        };
                        let raw_path = if parts.len() >= 2 { parts[1] } else { "/" };
                        // Prepend API base path if configured
                        let path = if let Some(ref bp) = api_base_path {
                            format!("{}{}", bp, raw_path)
                        } else {
                            raw_path.to_string()
                        };
                        let is_get_or_head = method == "get" || method == "head";
                        // POST, PUT, PATCH typically have bodies
                        let has_body = matches!(method.as_str(), "post" | "put" | "patch");

                        // Look up body from params file if available
                        let body_value = if has_body {
                            param_overrides.as_ref()
                                .map(|po| po.get_for_operation(None, &method_raw, &raw_path))
                                .and_then(|oo| oo.body)
                                .unwrap_or_else(|| serde_json::json!({}))
                        } else {
                            serde_json::json!({})
                        };

                        // Serialize body as JSON string for the template
                        let body_json_str = serde_json::to_string(&body_value)
                            .unwrap_or_else(|_| "{}".to_string());

                        serde_json::json!({
                            "operation": s.operation,
                            "method": method,
                            "path": path,
                            "extract": s.extract,
                            "use_values": s.use_values,
                            "description": s.description,
                            "display_name": s.description.clone().unwrap_or_else(|| format!("Step {}", idx)),
                            "is_get_or_head": is_get_or_head,
                            "has_body": has_body,
                            "body": body_json_str,  // Body as JSON string for JS literal
                            "body_is_dynamic": false,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "extract_fields": config.default_extract_fields,
            "duration_secs": duration_secs,
            "max_vus": self.vus,
            "auth_header": self.auth,
            "custom_headers": custom_headers,
            "skip_tls_verify": self.skip_tls_verify,
            // Add missing template fields
            "stages": stages.iter().map(|s| serde_json::json!({
                "duration": s.duration,
                "target": s.target,
            })).collect::<Vec<_>>(),
            "threshold_percentile": self.threshold_percentile,
            "threshold_ms": self.threshold_ms,
            "max_error_rate": self.max_error_rate,
            "headers": headers_json,
            "dynamic_imports": Vec::<String>::new(),
            "dynamic_globals": Vec::<String>::new(),
        });

        let script = handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))?;

        // Write and execute script
        let script_path =
            self.output.join(format!("k6-{}-crud-flow.js", spec_name.replace('.', "_")));

        std::fs::create_dir_all(self.output.clone())?;
        std::fs::write(&script_path, &script)?;

        if !self.generate_only {
            let executor = K6Executor::new()?;
            let output_dir = self.output.join(format!("{}_results", spec_name.replace('.', "_")));
            std::fs::create_dir_all(&output_dir)?;

            executor.execute(&script_path, Some(&output_dir), self.verbose).await?;
        }

        // For now, return empty extracted values
        // TODO: Parse k6 output to extract actual values
        Ok(ExtractedValues::new())
    }

    /// Execute standard (non-CRUD) spec benchmark
    async fn execute_standard_spec(&self, parser: &SpecParser, spec_name: &str) -> Result<()> {
        let mut operations = if let Some(filter) = &self.operations {
            parser.filter_operations(filter)?
        } else {
            parser.get_operations()
        };

        if let Some(exclude) = &self.exclude_operations {
            operations = parser.exclude_operations(operations, exclude)?;
        }

        if operations.is_empty() {
            TerminalReporter::print_warning(&format!("No operations found in {}", spec_name));
            return Ok(());
        }

        TerminalReporter::print_progress(&format!(
            "  {} operations in {}",
            operations.len(),
            spec_name
        ));

        // Generate request templates
        let templates: Vec<_> = operations
            .iter()
            .map(RequestGenerator::generate_template)
            .collect::<Result<Vec<_>>>()?;

        // Parse headers
        let custom_headers = self.parse_headers()?;

        // Resolve base path
        let base_path = self.resolve_base_path(parser);

        // Generate k6 script
        let scenario =
            LoadScenario::from_str(&self.scenario).map_err(BenchError::InvalidScenario)?;

        let k6_config = K6Config {
            target_url: self.target.clone(),
            base_path,
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

        // Write and execute script
        let script_path = self.output.join(format!("k6-{}.js", spec_name.replace('.', "_")));

        std::fs::create_dir_all(self.output.clone())?;
        std::fs::write(&script_path, &script)?;

        if !self.generate_only {
            let executor = K6Executor::new()?;
            let output_dir = self.output.join(format!("{}_results", spec_name.replace('.', "_")));
            std::fs::create_dir_all(&output_dir)?;

            executor.execute(&script_path, Some(&output_dir), self.verbose).await?;
        }

        Ok(())
    }

    /// Execute CRUD flow testing mode
    async fn execute_crud_flow(&self, parser: &SpecParser) -> Result<()> {
        TerminalReporter::print_progress("Detecting CRUD operations...");

        let operations = parser.get_operations();
        let flows = CrudFlowDetector::detect_flows(&operations);

        if flows.is_empty() {
            return Err(BenchError::Other(
                "No CRUD flows detected in spec. Ensure spec has POST/GET/PUT/DELETE operations on related paths.".to_string(),
            ));
        }

        TerminalReporter::print_success(&format!("Detected {} CRUD flow(s)", flows.len()));

        for flow in &flows {
            TerminalReporter::print_progress(&format!(
                "  - {}: {} steps",
                flow.name,
                flow.steps.len()
            ));
        }

        // Generate CRUD flow script
        let handlebars = handlebars::Handlebars::new();
        let template = include_str!("templates/k6_crud_flow.hbs");

        let custom_headers = self.parse_headers()?;
        let config = self.build_crud_flow_config().unwrap_or_default();

        // Load parameter overrides if provided (for body configurations)
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

        // Generate stages from scenario
        let duration_secs = Self::parse_duration(&self.duration)?;
        let scenario =
            LoadScenario::from_str(&self.scenario).map_err(BenchError::InvalidScenario)?;
        let stages = scenario.generate_stages(duration_secs, self.vus);

        // Resolve base path (CLI option takes priority over spec's servers URL)
        let api_base_path = self.resolve_base_path(parser);
        if let Some(ref bp) = api_base_path {
            TerminalReporter::print_progress(&format!("Using base path: {}", bp));
        }

        // Build headers JSON string for the template
        let mut all_headers = custom_headers.clone();
        if let Some(auth) = &self.auth {
            all_headers.insert("Authorization".to_string(), auth.clone());
        }
        let headers_json = serde_json::to_string(&all_headers).unwrap_or_else(|_| "{}".to_string());

        let data = serde_json::json!({
            "base_url": self.target,
            "flows": flows.iter().map(|f| {
                // Sanitize flow name for use as JavaScript variable and k6 metric names
                let sanitized_name = K6ScriptGenerator::sanitize_js_identifier(&f.name);
                serde_json::json!({
                    "name": sanitized_name.clone(),  // Use sanitized name for variable names
                    "display_name": f.name,          // Keep original for comments/display
                    "base_path": f.base_path,
                    "steps": f.steps.iter().enumerate().map(|(idx, s)| {
                        // Parse operation to get method and path
                        let parts: Vec<&str> = s.operation.splitn(2, ' ').collect();
                        let method_raw = if !parts.is_empty() {
                            parts[0].to_uppercase()
                        } else {
                            "GET".to_string()
                        };
                        let method = if !parts.is_empty() {
                            let m = parts[0].to_lowercase();
                            // k6 uses 'del' for DELETE
                            if m == "delete" { "del".to_string() } else { m }
                        } else {
                            "get".to_string()
                        };
                        let raw_path = if parts.len() >= 2 { parts[1] } else { "/" };
                        // Prepend API base path if configured
                        let path = if let Some(ref bp) = api_base_path {
                            format!("{}{}", bp, raw_path)
                        } else {
                            raw_path.to_string()
                        };
                        let is_get_or_head = method == "get" || method == "head";
                        // POST, PUT, PATCH typically have bodies
                        let has_body = matches!(method.as_str(), "post" | "put" | "patch");

                        // Look up body from params file if available (use raw_path for matching)
                        let body_value = if has_body {
                            param_overrides.as_ref()
                                .map(|po| po.get_for_operation(None, &method_raw, raw_path))
                                .and_then(|oo| oo.body)
                                .unwrap_or_else(|| serde_json::json!({}))
                        } else {
                            serde_json::json!({})
                        };

                        // Serialize body as JSON string for the template
                        let body_json_str = serde_json::to_string(&body_value)
                            .unwrap_or_else(|_| "{}".to_string());

                        serde_json::json!({
                            "operation": s.operation,
                            "method": method,
                            "path": path,
                            "extract": s.extract,
                            "use_values": s.use_values,
                            "description": s.description,
                            "display_name": s.description.clone().unwrap_or_else(|| format!("Step {}", idx)),
                            "is_get_or_head": is_get_or_head,
                            "has_body": has_body,
                            "body": body_json_str,  // Body as JSON string for JS literal
                            "body_is_dynamic": false,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "extract_fields": config.default_extract_fields,
            "duration_secs": duration_secs,
            "max_vus": self.vus,
            "auth_header": self.auth,
            "custom_headers": custom_headers,
            "skip_tls_verify": self.skip_tls_verify,
            // Add missing template fields
            "stages": stages.iter().map(|s| serde_json::json!({
                "duration": s.duration,
                "target": s.target,
            })).collect::<Vec<_>>(),
            "threshold_percentile": self.threshold_percentile,
            "threshold_ms": self.threshold_ms,
            "max_error_rate": self.max_error_rate,
            "headers": headers_json,
            "dynamic_imports": Vec::<String>::new(),
            "dynamic_globals": Vec::<String>::new(),
        });

        let script = handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))?;

        // Validate the generated CRUD flow script
        TerminalReporter::print_progress("Validating CRUD flow script...");
        let validation_errors = K6ScriptGenerator::validate_script(&script);
        if !validation_errors.is_empty() {
            TerminalReporter::print_error("CRUD flow script validation failed");
            for error in &validation_errors {
                eprintln!("  {}", error);
            }
            return Err(BenchError::Other(format!(
                "CRUD flow script validation failed with {} error(s)",
                validation_errors.len()
            )));
        }

        TerminalReporter::print_success("CRUD flow script generated");

        // Write and execute script
        let script_path = if let Some(output) = &self.script_output {
            output.clone()
        } else {
            self.output.join("k6-crud-flow-script.js")
        };

        if let Some(parent) = script_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&script_path, &script)?;
        TerminalReporter::print_success(&format!("Script written to: {}", script_path.display()));

        if self.generate_only {
            println!("\nScript generated successfully. Run it with:");
            println!("  k6 run {}", script_path.display());
            return Ok(());
        }

        // Execute k6
        TerminalReporter::print_progress("Executing CRUD flow test...");
        let executor = K6Executor::new()?;
        std::fs::create_dir_all(&self.output)?;

        let results = executor.execute(&script_path, Some(&self.output), self.verbose).await?;

        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary(&results, duration_secs);

        Ok(())
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
            spec: vec![PathBuf::from("test.yaml")],
            spec_dir: None,
            merge_conflicts: "error".to_string(),
            spec_mode: "merge".to_string(),
            dependency_config: None,
            target: "http://localhost".to_string(),
            base_path: None,
            duration: "1m".to_string(),
            vus: 10,
            scenario: "ramp-up".to_string(),
            operations: None,
            exclude_operations: None,
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
            crud_flow: false,
            flow_config: None,
            extract_fields: None,
            parallel_create: None,
            data_file: None,
            data_distribution: "unique-per-vu".to_string(),
            data_mappings: None,
            per_uri_control: false,
            error_rate: None,
            error_types: None,
            security_test: false,
            security_payloads: None,
            security_categories: None,
            security_target_fields: None,
            wafbench_dir: None,
        };

        let headers = cmd.parse_headers().unwrap();
        assert_eq!(headers.get("X-API-Key"), Some(&"test123".to_string()));
        assert_eq!(headers.get("X-Client-ID"), Some(&"client456".to_string()));
    }

    #[test]
    fn test_get_spec_display_name() {
        let cmd = BenchCommand {
            spec: vec![PathBuf::from("test.yaml")],
            spec_dir: None,
            merge_conflicts: "error".to_string(),
            spec_mode: "merge".to_string(),
            dependency_config: None,
            target: "http://localhost".to_string(),
            base_path: None,
            duration: "1m".to_string(),
            vus: 10,
            scenario: "ramp-up".to_string(),
            operations: None,
            exclude_operations: None,
            auth: None,
            headers: None,
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
            crud_flow: false,
            flow_config: None,
            extract_fields: None,
            parallel_create: None,
            data_file: None,
            data_distribution: "unique-per-vu".to_string(),
            data_mappings: None,
            per_uri_control: false,
            error_rate: None,
            error_types: None,
            security_test: false,
            security_payloads: None,
            security_categories: None,
            security_target_fields: None,
            wafbench_dir: None,
        };

        assert_eq!(cmd.get_spec_display_name(), "test.yaml");

        // Test multiple specs
        let cmd_multi = BenchCommand {
            spec: vec![PathBuf::from("a.yaml"), PathBuf::from("b.yaml")],
            spec_dir: None,
            merge_conflicts: "error".to_string(),
            spec_mode: "merge".to_string(),
            dependency_config: None,
            target: "http://localhost".to_string(),
            base_path: None,
            duration: "1m".to_string(),
            vus: 10,
            scenario: "ramp-up".to_string(),
            operations: None,
            exclude_operations: None,
            auth: None,
            headers: None,
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
            crud_flow: false,
            flow_config: None,
            extract_fields: None,
            parallel_create: None,
            data_file: None,
            data_distribution: "unique-per-vu".to_string(),
            data_mappings: None,
            per_uri_control: false,
            error_rate: None,
            error_types: None,
            security_test: false,
            security_payloads: None,
            security_categories: None,
            security_target_fields: None,
            wafbench_dir: None,
        };

        assert_eq!(cmd_multi.get_spec_display_name(), "2 spec files");
    }
}
