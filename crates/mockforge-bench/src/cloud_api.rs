//! Cloud-friendly entry points for invoking bench and conformance runs
//! programmatically.
//!
//! The CLI in `command.rs` is the primary user-facing surface, but it assumes
//! the caller can supply paths on disk and is OK with stdout reporting. Cloud
//! callers (the registry server) need to:
//!
//! 1. Pass the OpenAPI spec as raw bytes (no filesystem coordination).
//! 2. Receive every artifact that was written to the run's output directory
//!    as in-memory bytes, so they can be persisted to Postgres / Tigris.
//! 3. Read structured `K6Results` without re-parsing `summary.json`.
//!
//! This module provides exactly that. Each `run_*` function:
//!
//! * Creates a private tempdir,
//! * Writes the supplied spec bytes into it,
//! * Builds a [`BenchCommand`] with cloud-appropriate defaults,
//! * Executes the run,
//! * Slurps every file produced under the output dir into a
//!   [`CloudRunArtifacts`] map.
//!
//! The CLI is unchanged — it still uses [`BenchCommand`] directly. Progress
//! reporting (`TerminalReporter`) still goes to stdout; suppressing or
//! redirecting it is intentionally out of scope for this module and will be
//! handled by a follow-up that introduces a `ProgressSink`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::command::BenchCommand;
use crate::error::{BenchError, Result};
use crate::executor::{K6Executor, K6Results};

/// Format hint for OpenAPI specs supplied as raw bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpecFormat {
    /// Pretty-printed JSON or a single-line JSON document.
    Json,
    /// YAML 1.1/1.2 document.
    Yaml,
    /// Sniff from the first non-whitespace byte: `{`/`[` → JSON, otherwise YAML.
    #[default]
    Auto,
}

impl SpecFormat {
    fn extension(self, bytes: &[u8]) -> &'static str {
        match self {
            SpecFormat::Json => "json",
            SpecFormat::Yaml => "yaml",
            SpecFormat::Auto => match bytes.iter().find(|b| !b.is_ascii_whitespace()) {
                Some(b'{') | Some(b'[') => "json",
                _ => "yaml",
            },
        }
    }
}

/// Inputs for [`run_bench`] — a k6 load test against an external URL.
///
/// All fields mirror the CLI flags one-for-one, minus filesystem-only options
/// (`--output`, `--script-output`, `--targets-file`, etc.) that have no
/// meaning in a hosted context.
#[derive(Debug, Clone)]
pub struct CloudBenchInputs {
    pub spec_bytes: Vec<u8>,
    pub spec_format: SpecFormat,
    pub target_url: String,
    pub base_path: Option<String>,
    pub duration: String,
    pub vus: u32,
    pub scenario: String,
    pub operations: Option<String>,
    pub exclude_operations: Option<String>,
    pub auth: Option<String>,
    /// Comma-separated `Key:Value,Key2:Value2` header list (matches CLI
    /// `--headers` semantics — see [`crate::command::parse_header_string`]).
    pub headers: Option<String>,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub skip_tls_verify: bool,
    pub chunked_request_bodies: bool,
}

impl Default for CloudBenchInputs {
    fn default() -> Self {
        Self {
            spec_bytes: Vec::new(),
            spec_format: SpecFormat::Auto,
            target_url: String::new(),
            base_path: None,
            duration: "30s".to_string(),
            vus: 10,
            scenario: "constant".to_string(),
            operations: None,
            exclude_operations: None,
            auth: None,
            headers: None,
            threshold_percentile: "p(95)".to_string(),
            threshold_ms: 1000,
            max_error_rate: 0.01,
            skip_tls_verify: false,
            chunked_request_bodies: false,
        }
    }
}

/// Inputs for [`run_conformance`] — OpenAPI 3.0.0 conformance testing against
/// an external URL.
///
/// Setting `spec_bytes` enables spec-driven mode (preferred). Leaving it `None`
/// falls through to the reference-check mode that the underlying executor
/// supports.
#[derive(Debug, Clone)]
pub struct CloudConformanceInputs {
    pub spec_bytes: Option<Vec<u8>>,
    pub spec_format: SpecFormat,
    pub target_url: String,
    pub base_path: Option<String>,
    pub api_key: Option<String>,
    /// `user:pass` for HTTP basic auth.
    pub basic_auth: Option<String>,
    /// Comma-separated category list (e.g. `"parameters,security"`).
    pub categories: Option<String>,
    /// `Header-Name: value` strings, one per entry.
    pub headers: Vec<String>,
    pub all_operations: bool,
    pub request_delay_ms: u64,
    /// When true, route conformance through k6 instead of the native Rust
    /// executor. Native is faster and the default.
    pub use_k6: bool,
    pub skip_tls_verify: bool,
    /// `"json"` (default) or `"sarif"`.
    pub report_format: String,
    pub export_requests: bool,
    pub validate_requests: bool,
}

impl Default for CloudConformanceInputs {
    fn default() -> Self {
        Self {
            spec_bytes: None,
            spec_format: SpecFormat::Auto,
            target_url: String::new(),
            base_path: None,
            api_key: None,
            basic_auth: None,
            categories: None,
            headers: Vec::new(),
            all_operations: false,
            request_delay_ms: 0,
            use_k6: false,
            skip_tls_verify: false,
            report_format: "json".to_string(),
            export_requests: false,
            validate_requests: false,
        }
    }
}

/// Every artifact produced by a cloud run.
///
/// Each file under the run's output directory is read into `files` keyed by
/// its filename. `k6_results` is populated from `summary.json` when the run
/// went through k6.
#[derive(Debug, Default, Clone)]
pub struct CloudRunArtifacts {
    pub k6_results: Option<K6Results>,
    pub files: HashMap<String, Vec<u8>>,
}

impl CloudRunArtifacts {
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(Vec::as_slice)
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        self.get(name).map(|b| String::from_utf8_lossy(b).into_owned())
    }

    pub fn get_json(&self, name: &str) -> Option<serde_json::Value> {
        self.get(name).and_then(|b| serde_json::from_slice(b).ok())
    }
}

/// Run a k6 load test against [`CloudBenchInputs::target_url`] and return all
/// produced artifacts in memory.
///
/// Requires the `k6` binary on `$PATH`. Returns [`BenchError::K6NotFound`] if
/// not present.
pub async fn run_bench(inputs: CloudBenchInputs) -> Result<CloudRunArtifacts> {
    if inputs.target_url.trim().is_empty() {
        return Err(BenchError::Other("target_url is required".to_string()));
    }
    if inputs.spec_bytes.is_empty() {
        return Err(BenchError::Other("spec_bytes is required for bench runs".to_string()));
    }
    if !K6Executor::is_k6_installed() {
        return Err(BenchError::K6NotFound);
    }

    let workdir = TempDir::new()
        .map_err(|e| BenchError::Other(format!("Failed to create tempdir: {}", e)))?;
    let spec_path = write_spec(workdir.path(), &inputs.spec_bytes, inputs.spec_format)?;
    let output_dir = workdir.path().join("output");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| BenchError::Other(format!("Failed to create output dir: {}", e)))?;

    let cmd = BenchCommand {
        spec: vec![spec_path],
        target: inputs.target_url,
        base_path: inputs.base_path,
        duration: inputs.duration,
        vus: inputs.vus,
        scenario: inputs.scenario,
        operations: inputs.operations,
        exclude_operations: inputs.exclude_operations,
        auth: inputs.auth,
        headers: inputs.headers,
        threshold_percentile: inputs.threshold_percentile,
        threshold_ms: inputs.threshold_ms,
        max_error_rate: inputs.max_error_rate,
        skip_tls_verify: inputs.skip_tls_verify,
        chunked_request_bodies: inputs.chunked_request_bodies,
        ..default_bench_command(&output_dir)
    };

    cmd.execute().await?;
    read_artifacts(&output_dir)
}

/// Run an OpenAPI 3.0.0 conformance test against
/// [`CloudConformanceInputs::target_url`].
///
/// When `use_k6` is false (default) the native Rust executor runs in-process —
/// no k6 binary required. When `use_k6` is true, k6 must be on `$PATH`.
pub async fn run_conformance(inputs: CloudConformanceInputs) -> Result<CloudRunArtifacts> {
    if inputs.target_url.trim().is_empty() {
        return Err(BenchError::Other("target_url is required".to_string()));
    }
    if inputs.use_k6 && !K6Executor::is_k6_installed() {
        return Err(BenchError::K6NotFound);
    }

    let workdir = TempDir::new()
        .map_err(|e| BenchError::Other(format!("Failed to create tempdir: {}", e)))?;
    let output_dir = workdir.path().join("output");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| BenchError::Other(format!("Failed to create output dir: {}", e)))?;

    let spec_paths = if let Some(bytes) = &inputs.spec_bytes {
        vec![write_spec(workdir.path(), bytes, inputs.spec_format)?]
    } else {
        Vec::new()
    };

    let report_path = output_dir.join("conformance-report.json");
    let cmd = BenchCommand {
        spec: spec_paths,
        target: inputs.target_url,
        base_path: inputs.base_path,
        skip_tls_verify: inputs.skip_tls_verify,
        conformance: true,
        conformance_api_key: inputs.api_key,
        conformance_basic_auth: inputs.basic_auth,
        conformance_report: report_path,
        conformance_categories: inputs.categories,
        conformance_report_format: inputs.report_format,
        conformance_headers: inputs.headers,
        conformance_all_operations: inputs.all_operations,
        conformance_delay_ms: inputs.request_delay_ms,
        use_k6: inputs.use_k6,
        export_requests: inputs.export_requests,
        validate_requests: inputs.validate_requests,
        ..default_bench_command(&output_dir)
    };

    cmd.execute().await?;
    read_artifacts(&output_dir)
}

/// Build a [`BenchCommand`] populated with sensible defaults for a single-spec
/// run targeting the given output directory.
///
/// Caller should overwrite the fields relevant to their run via
/// `..default_bench_command(&output_dir)` struct update syntax.
fn default_bench_command(output_dir: &Path) -> BenchCommand {
    BenchCommand {
        spec: Vec::new(),
        spec_dir: None,
        merge_conflicts: "error".to_string(),
        spec_mode: "merge".to_string(),
        dependency_config: None,
        target: String::new(),
        base_path: None,
        duration: "30s".to_string(),
        vus: 10,
        scenario: "constant".to_string(),
        operations: None,
        exclude_operations: None,
        auth: None,
        headers: None,
        output: output_dir.to_path_buf(),
        generate_only: false,
        script_output: None,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 1000,
        max_error_rate: 0.01,
        verbose: false,
        skip_tls_verify: false,
        chunked_request_bodies: false,
        targets_file: None,
        max_concurrency: None,
        results_format: "aggregated".to_string(),
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
        wafbench_cycle_all: false,
        conformance: false,
        conformance_api_key: None,
        conformance_basic_auth: None,
        conformance_report: output_dir.join("conformance-report.json"),
        conformance_categories: None,
        conformance_report_format: "json".to_string(),
        conformance_headers: Vec::new(),
        conformance_all_operations: false,
        conformance_custom: None,
        conformance_delay_ms: 0,
        use_k6: false,
        conformance_custom_filter: None,
        export_requests: false,
        validate_requests: false,
        owasp_api_top10: false,
        owasp_categories: None,
        owasp_auth_header: "Authorization".to_string(),
        owasp_auth_token: None,
        owasp_admin_paths: None,
        owasp_id_fields: None,
        owasp_report: None,
        owasp_report_format: "json".to_string(),
        owasp_iterations: 1,
    }
}

fn write_spec(dir: &Path, bytes: &[u8], format: SpecFormat) -> Result<PathBuf> {
    let filename = format!("spec.{}", format.extension(bytes));
    let path = dir.join(filename);
    std::fs::write(&path, bytes)
        .map_err(|e| BenchError::Other(format!("Failed to write spec to tempdir: {}", e)))?;
    Ok(path)
}

fn read_artifacts(output_dir: &Path) -> Result<CloudRunArtifacts> {
    let mut files = HashMap::new();
    if output_dir.exists() {
        let entries = std::fs::read_dir(output_dir)
            .map_err(|e| BenchError::Other(format!("Failed to read output dir: {}", e)))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| BenchError::Other(format!("Failed to read entry: {}", e)))?;
            let metadata = entry
                .metadata()
                .map_err(|e| BenchError::Other(format!("Failed to stat entry: {}", e)))?;
            if !metadata.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let bytes = std::fs::read(entry.path()).map_err(|e| {
                BenchError::Other(format!("Failed to read artifact {}: {}", name, e))
            })?;
            files.insert(name, bytes);
        }
    }

    let k6_results = files.get("summary.json").and_then(|bytes| parse_k6_summary(bytes).ok());

    Ok(CloudRunArtifacts { k6_results, files })
}

fn parse_k6_summary(bytes: &[u8]) -> Result<K6Results> {
    let json: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| BenchError::ResultsParseError(e.to_string()))?;
    let duration_values = &json["metrics"]["http_req_duration"]["values"];
    Ok(K6Results {
        total_requests: json["metrics"]["http_reqs"]["values"]["count"].as_u64().unwrap_or(0),
        // See `K6Executor::parse_results` for the rationale on why
        // http_req_failed.passes is the failure count.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_format_extension_for_json_bytes() {
        assert_eq!(SpecFormat::Auto.extension(b"  {\"openapi\":\"3.0.0\"}"), "json");
        assert_eq!(SpecFormat::Auto.extension(b"openapi: 3.0.0\n"), "yaml");
        assert_eq!(SpecFormat::Json.extension(b"openapi: 3.0.0"), "json");
        assert_eq!(SpecFormat::Yaml.extension(b"{}"), "yaml");
    }

    #[test]
    fn write_spec_round_trips_bytes() {
        let dir = TempDir::new().unwrap();
        let path = write_spec(dir.path(), b"openapi: 3.0.0\n", SpecFormat::Yaml).unwrap();
        assert!(path.ends_with("spec.yaml"));
        let read_back = std::fs::read(&path).unwrap();
        assert_eq!(read_back, b"openapi: 3.0.0\n");
    }

    #[test]
    fn read_artifacts_collects_top_level_files_only() {
        let dir = TempDir::new().unwrap();
        let out = dir.path();
        std::fs::write(out.join("summary.json"), br#"{"metrics":{}}"#).unwrap();
        std::fs::write(out.join("k6-output.log"), b"hello").unwrap();
        // Subdirectory should be ignored.
        std::fs::create_dir(out.join("nested")).unwrap();
        std::fs::write(out.join("nested").join("ignored.txt"), b"nope").unwrap();

        let artifacts = read_artifacts(out).unwrap();
        assert_eq!(artifacts.files.len(), 2);
        assert!(artifacts.files.contains_key("summary.json"));
        assert!(artifacts.files.contains_key("k6-output.log"));
        assert!(!artifacts.files.contains_key("ignored.txt"));
    }

    #[test]
    fn parse_k6_summary_handles_minimal_input() {
        let bytes = br#"{"metrics":{}}"#;
        let r = parse_k6_summary(bytes).unwrap();
        assert_eq!(r.total_requests, 0);
        assert_eq!(r.failed_requests, 0);
        assert_eq!(r.error_rate(), 0.0);
    }

    #[test]
    fn parse_k6_summary_extracts_values() {
        let bytes = br#"{
            "metrics": {
                "http_reqs": {"values": {"count": 100, "rate": 33.5}},
                "http_req_failed": {"values": {"passes": 4}},
                "http_req_duration": {"values": {
                    "avg": 12.3, "med": 10.0, "min": 1.0, "max": 50.0,
                    "p(90)": 20.0, "p(95)": 25.0, "p(99)": 40.0
                }},
                "vus_max": {"values": {"value": 10}}
            }
        }"#;
        let r = parse_k6_summary(bytes).unwrap();
        assert_eq!(r.total_requests, 100);
        assert_eq!(r.failed_requests, 4);
        assert_eq!(r.rps, 33.5);
        assert_eq!(r.p95_duration_ms, 25.0);
        assert_eq!(r.vus_max, 10);
    }

    #[test]
    fn cloud_run_artifacts_get_helpers() {
        let mut a = CloudRunArtifacts::default();
        a.files.insert("hello.txt".to_string(), b"world".to_vec());
        a.files.insert("payload.json".to_string(), br#"{"x":1}"#.to_vec());

        assert_eq!(a.get("hello.txt").unwrap(), b"world");
        assert_eq!(a.get_string("hello.txt").unwrap(), "world");
        assert_eq!(a.get_json("payload.json").unwrap()["x"], 1);
        assert!(a.get("missing").is_none());
    }

    #[tokio::test]
    async fn run_bench_rejects_empty_target() {
        let inputs = CloudBenchInputs {
            spec_bytes: br#"{"openapi":"3.0.0"}"#.to_vec(),
            ..Default::default()
        };
        let err = run_bench(inputs).await.unwrap_err();
        assert!(matches!(err, BenchError::Other(_)));
    }

    #[tokio::test]
    async fn run_bench_rejects_empty_spec() {
        let inputs = CloudBenchInputs {
            target_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let err = run_bench(inputs).await.unwrap_err();
        assert!(matches!(err, BenchError::Other(_)));
    }

    #[tokio::test]
    async fn run_conformance_rejects_empty_target() {
        let inputs = CloudConformanceInputs::default();
        let err = run_conformance(inputs).await.unwrap_err();
        assert!(matches!(err, BenchError::Other(_)));
    }
}
