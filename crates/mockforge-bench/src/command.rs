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
    SecurityCategory, SecurityPayloads, SecurityTestConfig, SecurityTestGenerator,
};
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

        std::fs::create_dir_all(script_path.parent().unwrap())?;
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
                error_rate: self.error_rate,
                error_types: self.error_types.clone(),
                security_test: self.security_test,
                security_payloads: self.security_payloads.clone(),
                security_categories: self.security_categories.clone(),
                security_target_fields: self.security_target_fields.clone(),
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
        if let Some(config) = self.build_security_config() {
            TerminalReporter::print_progress("Adding security testing support...");
            let payload_list = SecurityPayloads::get_payloads(&config);
            additional_code
                .push_str(&SecurityTestGenerator::generate_payload_selection(&payload_list));
            additional_code.push('\n');
            additional_code
                .push_str(&SecurityTestGenerator::generate_apply_payload(&config.target_fields));
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

        let data = serde_json::json!({
            "base_url": self.target,
            "flows": flows.iter().map(|f| {
                // Sanitize flow name for use as JavaScript variable and k6 metric names
                let sanitized_name = K6ScriptGenerator::sanitize_js_identifier(&f.name);
                serde_json::json!({
                    "name": sanitized_name.clone(),  // Use sanitized name for variable names
                    "display_name": f.name,          // Keep original for comments/display
                    "base_path": f.base_path,
                    "steps": f.steps.iter().map(|s| {
                        serde_json::json!({
                            "operation": s.operation,
                            "extract": s.extract,
                            "use_values": s.use_values,
                            "description": s.description,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "extract_fields": config.default_extract_fields,
            "duration_secs": Self::parse_duration(&self.duration)?,
            "max_vus": self.vus,
            "auth_header": self.auth,
            "custom_headers": custom_headers,
            "skip_tls_verify": self.skip_tls_verify,
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

        std::fs::create_dir_all(script_path.parent().unwrap())?;
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
            spec: PathBuf::from("test.yaml"),
            target: "http://localhost".to_string(),
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
            error_rate: None,
            error_types: None,
            security_test: false,
            security_payloads: None,
            security_categories: None,
            security_target_fields: None,
        };

        let headers = cmd.parse_headers().unwrap();
        assert_eq!(headers.get("X-API-Key"), Some(&"test123".to_string()));
        assert_eq!(headers.get("X-Client-ID"), Some(&"client456".to_string()));
    }
}
