//! Bench command implementation

use crate::error::{BenchError, Result};
use crate::executor::K6Executor;
use crate::k6_gen::{K6Config, K6ScriptGenerator};
use crate::reporter::TerminalReporter;
use crate::request_gen::RequestGenerator;
use crate::scenarios::LoadScenario;
use crate::spec_parser::SpecParser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
}

impl BenchCommand {
    /// Execute the bench command
    pub async fn execute(&self) -> Result<()> {
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
            TerminalReporter::print_warning("Install k6 from: https://k6.io/docs/get-started/installation/");
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

        // Generate request templates
        TerminalReporter::print_progress("Generating request templates...");
        let templates: Vec<_> = operations
            .iter()
            .map(|op| RequestGenerator::generate_template(op))
            .collect::<Result<Vec<_>>>()?;
        TerminalReporter::print_success("Request templates generated");

        // Parse headers
        let custom_headers = self.parse_headers()?;

        // Generate k6 script
        TerminalReporter::print_progress("Generating k6 load test script...");
        let scenario = LoadScenario::from_str(&self.scenario)
            .map_err(|e| BenchError::InvalidScenario(e))?;

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
        };

        let generator = K6ScriptGenerator::new(k6_config, templates);
        let script = generator.generate()?;
        TerminalReporter::print_success("k6 script generated");

        // Write script to file
        let script_path = if let Some(output) = &self.script_output {
            output.clone()
        } else {
            self.output.join("k6-script.js")
        };

        std::fs::create_dir_all(script_path.parent().unwrap())?;
        std::fs::write(&script_path, script)?;
        TerminalReporter::print_success(&format!(
            "Script written to: {}",
            script_path.display()
        ));

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

        let results = executor
            .execute(&script_path, Some(&self.output), self.verbose)
            .await?;

        // Print results
        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary(&results, duration_secs);

        println!("\nResults saved to: {}", self.output.display());

        Ok(())
    }

    /// Parse duration string (e.g., "30s", "5m", "1h") to seconds
    fn parse_duration(duration: &str) -> Result<u64> {
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
    fn parse_headers(&self) -> Result<HashMap<String, String>> {
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
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            verbose: false,
        };

        let headers = cmd.parse_headers().unwrap();
        assert_eq!(headers.get("X-API-Key"), Some(&"test123".to_string()));
        assert_eq!(headers.get("X-Client-ID"), Some(&"client456".to_string()));
    }
}
