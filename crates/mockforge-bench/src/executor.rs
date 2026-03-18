//! k6 execution and output handling

use crate::error::{BenchError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

/// Extract `MOCKFORGE_FAILURE:` JSON payload from a k6 output line.
///
/// k6 may format console.log lines differently depending on output mode:
/// - Raw: `MOCKFORGE_FAILURE:{...}`
/// - Logfmt: `time="..." level=info msg="MOCKFORGE_FAILURE:{...}" source=console`
fn extract_failure_json(line: &str) -> Option<String> {
    let marker = "MOCKFORGE_FAILURE:";
    let start = line.find(marker)?;
    let json_start = start + marker.len();
    let json_str = &line[json_start..];
    // Trim trailing `" source=console` if present (k6 logfmt)
    let json_str = json_str.strip_suffix("\" source=console").unwrap_or(json_str).trim();
    if json_str.is_empty() {
        return None;
    }
    // k6 logfmt wraps msg in quotes and escapes inner quotes as \" and
    // backslashes as \\. Unescape in order: backslashes first, then quotes.
    // Only unescape if the raw string doesn't parse as JSON (raw mode output).
    if json_str.starts_with('{') && json_str.contains("\\\"") {
        Some(json_str.replace("\\\\", "\\").replace("\\\"", "\""))
    } else {
        Some(json_str.to_string())
    }
}

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

    /// Execute a k6 script.
    ///
    /// `api_port` — when set, overrides k6's default API server address (`localhost:6565`)
    /// to `localhost:<api_port>`. This prevents "address already in use" errors when
    /// running multiple k6 instances in parallel (e.g., multi-target bench).
    /// Pass `None` for single-target runs (uses k6's default).
    pub async fn execute(
        &self,
        script_path: &Path,
        output_dir: Option<&Path>,
        verbose: bool,
    ) -> Result<K6Results> {
        self.execute_with_port(script_path, output_dir, verbose, None).await
    }

    /// Execute a k6 script with an optional custom API server port.
    pub async fn execute_with_port(
        &self,
        script_path: &Path,
        output_dir: Option<&Path>,
        verbose: bool,
        api_port: Option<u16>,
    ) -> Result<K6Results> {
        println!("Starting load test...\n");

        let mut cmd = TokioCommand::new(&self.k6_path);
        cmd.arg("run");

        // When running multiple k6 instances in parallel, each needs its own API server port
        // to avoid "bind: address already in use" on the default port 6565.
        if let Some(port) = api_port {
            cmd.arg("--address").arg(format!("localhost:{}", port));
        }

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

        // Set working directory to output dir so handleSummary's relative
        // "summary.json" path lands next to the script.
        if let Some(dir) = output_dir {
            cmd.current_dir(dir);
        }

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

        // Collect failure details from k6's console.log output
        // k6 may emit console.log to either stdout or stderr depending on version/config
        let failure_details: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let fd_stdout = Arc::clone(&failure_details);
        let fd_stderr = Arc::clone(&failure_details);

        // Collect all k6 output for saving to a log file
        let log_lines: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let log_stdout = Arc::clone(&log_lines);
        let log_stderr = Arc::clone(&log_lines);

        // Read stdout lines, capturing MOCKFORGE_FAILURE markers
        let stdout_handle = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                log_stdout.lock().await.push(format!("[stdout] {}", line));
                if let Some(json_str) = extract_failure_json(&line) {
                    fd_stdout.lock().await.push(json_str);
                } else {
                    spinner.set_message(line.clone());
                    if !line.is_empty() && !line.contains("running") && !line.contains("default") {
                        println!("{}", line);
                    }
                }
            }
            spinner.finish_and_clear();
        });

        // Read stderr lines, capturing MOCKFORGE_FAILURE markers
        let stderr_handle = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                if !line.is_empty() {
                    log_stderr.lock().await.push(format!("[stderr] {}", line));
                    if let Some(json_str) = extract_failure_json(&line) {
                        fd_stderr.lock().await.push(json_str);
                    } else {
                        eprintln!("{}", line);
                    }
                }
            }
        });

        // Wait for completion
        let status =
            child.wait().await.map_err(|e| BenchError::K6ExecutionFailed(e.to_string()))?;

        // Wait for both reader tasks to finish processing all lines
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        // k6 exit code 99 = thresholds crossed. The test DID run and summary.json
        // should still be present. Only treat non-99 failures as hard errors.
        let exit_code = status.code().unwrap_or(-1);
        if !status.success() && exit_code != 99 {
            return Err(BenchError::K6ExecutionFailed(format!(
                "k6 exited with status: {}",
                status
            )));
        }
        if exit_code == 99 {
            tracing::warn!("k6 thresholds crossed (exit code 99) — results will still be parsed");
        }

        // Write failure details to file if any were captured
        if let Some(dir) = output_dir {
            let details = failure_details.lock().await;
            if !details.is_empty() {
                let failure_path = dir.join("conformance-failure-details.json");
                let parsed: Vec<serde_json::Value> =
                    details.iter().filter_map(|s| serde_json::from_str(s).ok()).collect();
                if let Ok(json) = serde_json::to_string_pretty(&parsed) {
                    let _ = std::fs::write(&failure_path, json);
                }
            }

            // Save full k6 output to a log file for debugging
            let lines = log_lines.lock().await;
            if !lines.is_empty() {
                let log_path = dir.join("k6-output.log");
                let _ = std::fs::write(&log_path, lines.join("\n"));
                println!("k6 output log saved to: {}", log_path.display());
            }
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
            failed_requests: json["metrics"]["http_req_failed"]["values"]["fails"]
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

    #[test]
    fn test_extract_failure_json_raw() {
        let line = r#"MOCKFORGE_FAILURE:{"check":"test","expected":"status === 200"}"#;
        let result = extract_failure_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["check"], "test");
    }

    #[test]
    fn test_extract_failure_json_logfmt() {
        let line = r#"time="2026-01-01T00:00:00Z" level=info msg="MOCKFORGE_FAILURE:{\"check\":\"test\",\"response\":{\"body\":\"{\\\"key\\\":\\\"val\\\"}\"}} " source=console"#;
        let result = extract_failure_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["check"], "test");
        assert_eq!(parsed["response"]["body"], r#"{"key":"val"}"#);
    }

    #[test]
    fn test_extract_failure_json_no_marker() {
        assert!(extract_failure_json("just a regular log line").is_none());
    }
}
