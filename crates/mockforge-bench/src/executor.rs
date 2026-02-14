//! k6 execution and output handling

use crate::error::{BenchError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

/// k6 executor
pub struct K6Executor {
    k6_path: String,
}

impl K6Executor {
    /// Create a new k6 executor
    pub fn new() -> Result<Self> {
        let k6_path = which::which("k6")
            .map_err(|_| BenchError::K6NotFound)?
            .to_string_lossy()
            .to_string();

        Ok(Self { k6_path })
    }

    /// Check if k6 is installed
    pub fn is_k6_installed() -> bool {
        which::which("k6").is_ok()
    }

    /// Get k6 version
    pub async fn get_version(&self) -> Result<String> {
        let output = TokioCommand::new(&self.k6_path)
            .arg("version")
            .output()
            .await
            .map_err(|e| BenchError::K6ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(BenchError::K6ExecutionFailed("Failed to get k6 version".to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute a k6 script
    pub async fn execute(
        &self,
        script_path: &Path,
        output_dir: Option<&Path>,
        verbose: bool,
    ) -> Result<K6Results> {
        println!("Starting load test...\n");

        let mut cmd = TokioCommand::new(&self.k6_path);
        cmd.arg("run");

        // Add output options
        if let Some(dir) = output_dir {
            let summary_path = dir.join("summary.json");
            cmd.arg("--summary-export").arg(summary_path);
        }

        // Add verbosity
        if verbose {
            cmd.arg("--verbose");
        }

        cmd.arg(script_path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| BenchError::K6ExecutionFailed(e.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BenchError::K6ExecutionFailed("Failed to capture stdout".to_string()))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BenchError::K6ExecutionFailed("Failed to capture stderr".to_string()))?;

        // Stream output
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        // Create progress indicator
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap(),
        );
        spinner.set_message("Running load test...");

        // Read output lines
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                spinner.set_message(line.clone());
                if !line.is_empty() && !line.contains("running") && !line.contains("default") {
                    println!("{}", line);
                }
            }
            spinner.finish_and_clear();
        });

        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                if !line.is_empty() {
                    eprintln!("{}", line);
                }
            }
        });

        // Wait for completion
        let status =
            child.wait().await.map_err(|e| BenchError::K6ExecutionFailed(e.to_string()))?;

        if !status.success() {
            return Err(BenchError::K6ExecutionFailed(format!(
                "k6 exited with status: {}",
                status
            )));
        }

        // Parse results if output directory was specified
        let results = if let Some(dir) = output_dir {
            Self::parse_results(dir)?
        } else {
            K6Results::default()
        };

        Ok(results)
    }

    /// Parse k6 results from JSON output
    fn parse_results(output_dir: &Path) -> Result<K6Results> {
        let summary_path = output_dir.join("summary.json");

        if !summary_path.exists() {
            return Ok(K6Results::default());
        }

        let content = std::fs::read_to_string(summary_path)
            .map_err(|e| BenchError::ResultsParseError(e.to_string()))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| BenchError::ResultsParseError(e.to_string()))?;

        let duration_values = &json["metrics"]["http_req_duration"]["values"];

        Ok(K6Results {
            total_requests: json["metrics"]["http_reqs"]["values"]["count"].as_u64().unwrap_or(0),
            failed_requests: json["metrics"]["http_req_failed"]["values"]["passes"]
                .as_u64()
                .unwrap_or(0),
            avg_duration_ms: duration_values["avg"].as_f64().unwrap_or(0.0),
            p95_duration_ms: duration_values["p(95)"].as_f64().unwrap_or(0.0),
            p99_duration_ms: duration_values["p(99)"].as_f64().unwrap_or(0.0),
            rps: json["metrics"]["http_reqs"]["values"]["rate"].as_f64().unwrap_or(0.0),
            vus_max: json["metrics"]["vus_max"]["values"]["value"].as_u64().unwrap_or(0) as u32,
            min_duration_ms: duration_values["min"].as_f64().unwrap_or(0.0),
            max_duration_ms: duration_values["max"].as_f64().unwrap_or(0.0),
            med_duration_ms: duration_values["med"].as_f64().unwrap_or(0.0),
            p90_duration_ms: duration_values["p(90)"].as_f64().unwrap_or(0.0),
        })
    }
}

impl Default for K6Executor {
    fn default() -> Self {
        Self::new().expect("k6 not found")
    }
}

/// k6 test results
#[derive(Debug, Clone, Default)]
pub struct K6Results {
    pub total_requests: u64,
    pub failed_requests: u64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub rps: f64,
    pub vus_max: u32,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub med_duration_ms: f64,
    pub p90_duration_ms: f64,
}

impl K6Results {
    /// Get error rate as a percentage
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.failed_requests as f64 / self.total_requests as f64) * 100.0
    }

    /// Get success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        100.0 - self.error_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k6_results_error_rate() {
        let results = K6Results {
            total_requests: 100,
            failed_requests: 5,
            avg_duration_ms: 100.0,
            p95_duration_ms: 200.0,
            p99_duration_ms: 300.0,
            ..Default::default()
        };

        assert_eq!(results.error_rate(), 5.0);
        assert_eq!(results.success_rate(), 95.0);
    }

    #[test]
    fn test_k6_results_zero_requests() {
        let results = K6Results::default();
        assert_eq!(results.error_rate(), 0.0);
    }
}
