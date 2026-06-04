//! Bench command implementation

use crate::crud_flow::{CrudFlowConfig, CrudFlowDetector};
use crate::data_driven::{DataDistribution, DataDrivenConfig, DataDrivenGenerator, DataMapping};
use crate::dynamic_params::{DynamicParamProcessor, DynamicPlaceholder};
use crate::error::{BenchError, Result};
use crate::executor::K6Executor;
use crate::invalid_data::{InvalidDataConfig, InvalidDataGenerator};
use crate::k6_gen::{K6Config, K6ScriptGenerator};
use crate::mock_integration::{
    MockIntegrationConfig, MockIntegrationGenerator, MockServerDetector,
};
use crate::owasp_api::{OwaspApiConfig, OwaspApiGenerator, OwaspCategory, ReportFormat};
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
use mockforge_openapi::multi_spec::{
    load_specs_from_directory, load_specs_from_files, merge_specs, ConflictStrategy,
};
use mockforge_openapi::spec::OpenApiSpec;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Parse a comma-separated header string into a `HashMap`.
///
/// Format: `Key:Value,Key2:Value2`
///
/// **Known limitation**: Header values containing commas will be incorrectly
/// split. Cookie headers with semicolons work fine, but Cookie values with
/// commas (e.g. `expires=Thu, 01 Jan 2099`) will break.
pub fn parse_header_string(input: &str) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();

    for pair in input.split(',') {
        let parts: Vec<&str> = pair.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(BenchError::Other(format!(
                "Invalid header format: '{}'. Expected 'Key:Value'",
                pair
            )));
        }
        headers.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
    }

    Ok(headers)
}

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
    /// Target requests-per-second. When `Some(n)`, the generated k6 script
    /// switches to `constant-arrival-rate` executor at `n` RPS with `vus`
    /// pre-allocated. When `None`, uses the legacy `ramping-vus` executor
    /// where RPS is implicit (VUs × 1 req/sec from the script's `sleep(1)`).
    /// Issue #79 — Srikanth's round-3 reply.
    pub target_rps: Option<u32>,
    /// When true, every k6 request opens a new TCP/TLS connection
    /// (`noConnectionReuse: true` and `--no-vu-connection-reuse`). Lets users
    /// drive a high connections-per-second rate to exercise connection-limit
    /// chaos and observe TCP-level fault injection. Issue #79.
    pub no_keep_alive: bool,
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
    /// When true, set `Transfer-Encoding: chunked` on every k6 request body so
    /// the server experiences chunked-encoded traffic. See
    /// `K6ScriptTemplateData::chunked_request_bodies` for caveats — k6's Go
    /// transport may still send Content-Length in some cases.
    pub chunked_request_bodies: bool,
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
    /// Cycle through ALL WAFBench payloads instead of random sampling
    pub wafbench_cycle_all: bool,

    // === OpenAPI 3.0.0 Conformance Testing ===
    /// Enable conformance testing mode
    pub conformance: bool,
    /// API key for conformance security tests
    pub conformance_api_key: Option<String>,
    /// Basic auth credentials for conformance security tests (user:pass)
    pub conformance_basic_auth: Option<String>,
    /// Conformance report output file
    pub conformance_report: PathBuf,
    /// Conformance categories to test (comma-separated, e.g. "parameters,security")
    pub conformance_categories: Option<String>,
    /// Conformance report format: "json" or "sarif"
    pub conformance_report_format: String,
    /// Custom headers to inject into every conformance request (for authentication).
    /// Each entry is "Header-Name: value" format.
    pub conformance_headers: Vec<String>,
    /// When true, test ALL operations for method/response/body categories
    /// instead of just one representative per feature check.
    pub conformance_all_operations: bool,
    /// Optional YAML file with custom conformance checks
    pub conformance_custom: Option<PathBuf>,
    /// Delay in milliseconds between consecutive conformance requests.
    /// Useful when testing against rate-limited APIs.
    pub conformance_delay_ms: u64,
    /// Use k6 for conformance test execution instead of the native Rust executor
    pub use_k6: bool,
    /// Regex filter for custom conformance checks — only checks whose name or
    /// path matches the pattern are included. Example: "wafcrs|ssl" to test
    /// only checks with "wafcrs" or "ssl" in the name/path.
    pub conformance_custom_filter: Option<String>,
    /// When true, export all request/response pairs to
    /// `conformance-requests.json` in the output directory.
    pub export_requests: bool,
    /// When true, validate each request against the OpenAPI spec and report
    /// violations to `conformance-request-violations.json`.
    pub validate_requests: bool,
    /// Issue #79 round 13 (4) — when true, replace the standard
    /// conformance run with a positive + per-category negative
    /// self-test driver. Verifies that the server actually rejects
    /// the negatives with 4xx (i.e. its validator is wired correctly).
    /// Useful to confirm the round-13 (3) validator-bypass fix took
    /// effect against the user's spec.
    pub conformance_self_test: bool,
    /// Round 23 (c-iii) — when true, capture every self-test probe's
    /// full request/response to `conformance-self-test-requests.jsonl`.
    /// No effect outside `--conformance-self-test`.
    pub conformance_self_test_capture: bool,

    /// Round 18.5 — local source IPs to bind self-test requests to.
    /// Each entry must be a valid `IpAddr` and already assigned to
    /// an interface on the host. Operations round-robin through the
    /// pool. Empty → one default client.
    pub source_ips: Vec<String>,
    /// Round 18.5 — fake source IPs to advertise via forwarded-IP
    /// headers (rotated per operation). Used for GEODB testing
    /// where the destination reads the IP from a header.
    pub geo_source_ips: Vec<String>,
    /// Round 18.5 — which forwarded-IP header(s) to populate when
    /// `geo_source_ips` is non-empty. Empty → default 3-header set
    /// (X-Forwarded-For, True-Client-IP, CF-Connecting-IP).
    pub geo_source_headers: Vec<String>,

    /// Round 21.1 — cap the HTML conformance report's missed-negative
    /// drill-down at N rows. `Some(0)` means no cap; `None` keeps the
    /// default of 200. The JSON report always carries the full set
    /// regardless of this knob — it only controls what the HTML drill
    /// view shows so a 50 000-violation run doesn't produce a 5 MB
    /// browser-choking HTML file by default.
    pub report_missed_cap: Option<u32>,

    // === OWASP API Security Top 10 Testing ===
    /// Enable OWASP API Security Top 10 testing mode
    pub owasp_api_top10: bool,
    /// OWASP API categories to test (comma-separated)
    pub owasp_categories: Option<String>,
    /// Authorization header name for OWASP auth tests
    pub owasp_auth_header: String,
    /// Valid authorization token for OWASP baseline requests
    pub owasp_auth_token: Option<String>,
    /// File containing admin/privileged paths to test
    pub owasp_admin_paths: Option<PathBuf>,
    /// Fields containing resource IDs for BOLA testing
    pub owasp_id_fields: Option<String>,
    /// OWASP report output file
    pub owasp_report: Option<PathBuf>,
    /// OWASP report format (json, sarif)
    pub owasp_report_format: String,
    /// Number of iterations per VU for OWASP tests (default: 1)
    pub owasp_iterations: u32,
}

/// Round 18.5 / 19 — parse a list of CLI IP strings. Each entry may be:
/// - a single IPv4/IPv6 (`10.0.0.5` / `2001:db8::1`)
/// - a comma-separated list (`10.0.0.5,10.0.0.6,2001:db8::1`)
/// - a CIDR range (`10.0.0.0/29` expands to 8 hosts;
///   `2001:db8::/126` expands to 4 IPv6 hosts)
///
/// CIDR ranges are capped at `MAX_CIDR_EXPANSION` (256) host
/// addresses to avoid OOM'ing on `/8` typos. The cap is generous
/// for GEODB testing (you want 20–100 IPs, not 10M) and the warning
/// names the cap so it's debuggable.
///
/// Malformed entries log a warning and are dropped; the bench
/// continues with whatever resolved cleanly.
fn parse_ip_list(raw: &[String], flag_name: &str) -> Vec<std::net::IpAddr> {
    use std::net::IpAddr;
    const MAX_CIDR_EXPANSION: usize = 256;
    let mut out = Vec::new();
    for entry in raw {
        for piece in entry.split(',') {
            let s = piece.trim();
            if s.is_empty() {
                continue;
            }
            // CIDR form: `ip/prefix`
            if let Some((addr_part, prefix_part)) = s.split_once('/') {
                let prefix: u32 = match prefix_part.parse() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{s}': bad CIDR prefix: {e}");
                        continue;
                    }
                };
                let net_addr: IpAddr = match addr_part.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{s}': bad CIDR address: {e}");
                        continue;
                    }
                };
                expand_cidr(net_addr, prefix, MAX_CIDR_EXPANSION, flag_name, s, &mut out);
                continue;
            }
            // Round 22.4 — range form: `start-end` (Srikanth (h)).
            // Lets users specify non-power-of-2 ranges without
            // finding a clean prefix. IPv4 only for now (the most
            // common case); IPv6 ranges with `:` collide with the
            // address literal so they'd need a different separator.
            if let Some((start_str, end_str)) = s.split_once('-') {
                let start_s = start_str.trim();
                let end_s = end_str.trim();
                // Reject ambiguous IPv6 ranges (contain `:`) so we
                // don't accidentally parse `2001:db8::1-2001:db8::5`
                // as a half address.
                if start_s.contains(':') || end_s.contains(':') {
                    tracing::warn!(target: "mockforge::bench", "--{flag_name} '{s}': IPv6 range syntax not supported (use CIDR like 2001:db8::/126 instead)");
                    continue;
                }
                let start: IpAddr = match start_s.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{s}': bad range start: {e}");
                        continue;
                    }
                };
                let end: IpAddr = match end_s.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{s}': bad range end: {e}");
                        continue;
                    }
                };
                expand_range(start, end, MAX_CIDR_EXPANSION, flag_name, s, &mut out);
                continue;
            }
            // Plain IP form
            match s.parse::<IpAddr>() {
                Ok(ip) => out.push(ip),
                Err(e) => {
                    tracing::warn!(target: "mockforge::bench", "ignoring malformed --{flag_name} value '{s}': {e}");
                }
            }
        }
    }
    out
}

/// Round 22.4 — expand an inclusive `start-end` IPv4 range to host
/// addresses, capped at `cap`. Returns silently on a backwards or
/// mixed-family range with a warning.
fn expand_range(
    start: std::net::IpAddr,
    end: std::net::IpAddr,
    cap: usize,
    flag_name: &str,
    raw: &str,
    out: &mut Vec<std::net::IpAddr>,
) {
    use std::net::{IpAddr, Ipv4Addr};
    let (start_v4, end_v4) = match (start, end) {
        (IpAddr::V4(a), IpAddr::V4(b)) => (a, b),
        _ => {
            tracing::warn!(target: "mockforge::bench", "--{flag_name} '{raw}': range start/end must both be IPv4");
            return;
        }
    };
    let start_u32 = u32::from(start_v4);
    let end_u32 = u32::from(end_v4);
    if end_u32 < start_u32 {
        tracing::warn!(target: "mockforge::bench", "--{flag_name} '{raw}': range end {end_v4} is before start {start_v4}");
        return;
    }
    let total = (end_u32 - start_u32).saturating_add(1) as usize;
    let take = total.min(cap);
    if total > cap {
        tracing::warn!(target: "mockforge::bench", "--{flag_name} '{raw}': range has {total} addresses, capping at {cap}");
    }
    for i in 0..take as u32 {
        out.push(IpAddr::V4(Ipv4Addr::from(start_u32 + i)));
    }
}

/// Expand a CIDR (IPv4 or IPv6) into individual host IPs, appending
/// to `out`. Capped at `cap` to prevent runaway expansion on a `/8`
/// typo. When the cap kicks in we log a warning and skip the rest.
fn expand_cidr(
    net: std::net::IpAddr,
    prefix: u32,
    cap: usize,
    flag_name: &str,
    raw: &str,
    out: &mut Vec<std::net::IpAddr>,
) {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    match net {
        IpAddr::V4(ipv4) => {
            if prefix > 32 {
                tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{raw}': IPv4 prefix must be <= 32");
                return;
            }
            let total: u64 = 1u64 << (32 - prefix);
            let take = total.min(cap as u64) as u32;
            if total > cap as u64 {
                tracing::warn!(target: "mockforge::bench", "--{flag_name} '{raw}': CIDR has {total} addresses, capping at {cap}");
            }
            let mask: u32 = if prefix == 0 {
                0
            } else {
                !0u32 << (32 - prefix)
            };
            let net_u32 = u32::from(ipv4) & mask;
            for i in 0..take {
                out.push(IpAddr::V4(Ipv4Addr::from(net_u32.wrapping_add(i))));
            }
        }
        IpAddr::V6(ipv6) => {
            if prefix > 128 {
                tracing::warn!(target: "mockforge::bench", "ignoring --{flag_name} '{raw}': IPv6 prefix must be <= 128");
                return;
            }
            // Total addresses = 2^(128-prefix). Cap at u128::MAX
            // conceptually but since `cap` is small (256) we just
            // iterate up to cap.
            let mask: u128 = if prefix == 0 {
                0
            } else {
                !0u128 << (128 - prefix)
            };
            let net_u128 = u128::from(ipv6) & mask;
            let remaining_bits = 128 - prefix;
            // Compute total carefully — for prefix=0 this is 2^128
            // which overflows; we just clamp via take.
            let total_capped = if remaining_bits >= 64 {
                cap as u128
            } else {
                (1u128 << remaining_bits).min(cap as u128)
            };
            if remaining_bits < 128 && (1u128 << remaining_bits) > cap as u128 {
                tracing::warn!(target: "mockforge::bench", "--{flag_name} '{raw}': IPv6 CIDR exceeds {cap} addresses, capping");
            }
            for i in 0..total_capped {
                out.push(IpAddr::V6(Ipv6Addr::from(net_u128.wrapping_add(i))));
            }
        }
    }
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
        // Round 23 — Srikanth flagged that k6 _does_ support per-VU source
        // IPs via `--local-ips` (the round-22 warning that said otherwise
        // was wrong). The k6 path now forwards `--source-ip` straight to
        // `k6 run --local-ips`, so the only case worth flagging is the
        // self-test+k6 combo: self-test returns before k6 ever launches,
        // so `--use-k6` on that command is a no-op.
        if self.conformance_self_test && self.use_k6 {
            TerminalReporter::print_warning(
                "--use-k6 has no effect with --conformance-self-test: the self-test driver runs and returns before k6 is invoked. Drop one or the other depending on whether you want the spec-driven self-test or a k6 bench run.",
            );
        }

        // Check if we're in multi-target mode
        if let Some(targets_file) = &self.targets_file {
            if self.conformance {
                return self.execute_multi_target_conformance(targets_file).await;
            }
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

        // Check for conformance testing mode (before spec loading — conformance doesn't need a user spec)
        if self.conformance {
            return self.execute_conformance_test().await;
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

        // Check for OWASP API Top 10 testing mode
        if self.owasp_api_top10 {
            return self.execute_owasp_test(&parser).await;
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

        let security_testing_enabled = self.security_test || self.wafbench_dir.is_some();

        // Issue #79 round 6 follow-up — Srikanth reported k6 emitting
        // "Insufficient VUs, reached 5 active VUs and cannot initialize more"
        // when running `--rps 100 --vus 5`. With the `constant-arrival-rate`
        // executor, k6 needs roughly `rps × avg_request_seconds` VUs to keep
        // up; if `--vus` is too low it can't sustain the rate. Warn pre-flight
        // so users know to bump `--vus` rather than chase the warning.
        //
        // Round 8 (#79): the static 100ms heuristic was wrong for fast targets
        // (~2ms latency). Probe the actual target first to measure baseline
        // latency, then derive a more accurate sizing recommendation. Fall
        // back to the 100ms heuristic only when the probe can't reach the
        // target (auth-gated endpoints, strict WAFs, etc).
        //
        // Round 9 (#79): factor in operation count. k6's constant-arrival-rate
        // counts ITERATIONS, not requests — and every iteration runs all N
        // operations sequentially, so required VUs scale with N. Srikanth's
        // 12-op spec at --rps 100 with 15ms latency needs ~19 VUs, not 3.
        let num_ops = operations.len() as u32;
        if let Some(rps) = self.target_rps {
            let probe =
                crate::preflight::probe_target_latency(&self.target, 3, self.skip_tls_verify).await;

            let (required_vus, basis) = match probe {
                Some(p) => (
                    p.required_vus(rps, num_ops),
                    format!("avg {:.1}ms (measured)", p.avg_latency.as_secs_f64() * 1000.0),
                ),
                None => {
                    // Static fallback: ~100ms heuristic × num_ops per iteration.
                    let fallback = (rps as u64)
                        .saturating_mul(num_ops.max(1) as u64)
                        .div_ceil(10)
                        .min(u32::MAX as u64) as u32;
                    (fallback, "~100ms (default — probe failed)".to_string())
                }
            };

            if self.vus < required_vus {
                // Round 10 (#79): Srikanth's 11422-op spec at --rps 100 produced
                // a recommendation of ~10,740 VUs, which is absurd in practice.
                // When the recommendation goes super-linear, the real fix is to
                // reduce the workload (use --operations filter), not bump VUs.
                // Cap the suggestion at 1000 and steer the user toward filtering.
                const VU_RECOMMENDATION_CAP: u32 = 1000;
                let recommendation = required_vus.max(self.vus + 1);
                if recommendation > VU_RECOMMENDATION_CAP {
                    TerminalReporter::print_warning(&format!(
                        "Workload is very large: --rps {} × {} ops/iteration × {} \
                         baseline ⇒ ~{} VUs needed end-to-end, far beyond what's \
                         practical to drive. Two ways to fix:\n  1. Reduce \
                         operations per iteration with `--operations 'pattern,…'` \
                         (or `--exclude-operations`) to focus the bench on a \
                         representative subset.\n  2. Drop `--rps` and use \
                         `--vus {}` alone — closed-model load runs as fast as \
                         the VU pool allows, bounded by latency, with no per-\
                         iteration deadline. Expect 1-iteration coverage of ~{} \
                         operations in {}s.",
                        rps,
                        num_ops,
                        basis,
                        recommendation,
                        self.vus.max(5),
                        num_ops,
                        Self::parse_duration(&self.duration).unwrap_or(0),
                    ));
                } else {
                    TerminalReporter::print_warning(&format!(
                        "--vus {} may be insufficient for --rps {} × {} ops/iteration \
                         (baseline latency {}). k6's constant-arrival-rate counts ITERATIONS \
                         and each runs every operation in the spec — required ≈ rps × ops × \
                         latency_secs VUs. Bump --vus to ~{} if you see \"Insufficient VUs\" \
                         warnings.",
                        self.vus, rps, num_ops, basis, recommendation,
                    ));
                }
            } else if probe.is_some() {
                TerminalReporter::print_progress(&format!(
                    "Pre-flight probe: target latency {}, {} ops/iteration — --vus {} \
                     is sufficient for --rps {}",
                    basis, num_ops, self.vus, rps,
                ));
            }
        }

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
            security_testing_enabled,
            chunked_request_bodies: self.chunked_request_bodies,
            target_rps: self.target_rps,
            no_keep_alive: self.no_keep_alive,
            // Round 22.3 — wire `--geo-source-ip` / `--geo-source-header`
            // through to the k6 generator so the rendered script
            // rotates the forwarded-IP headers per iteration. Pre-fix
            // these were Vec::new() and the script never set the
            // headers in bench mode.
            geo_source_ips: parse_ip_list(&self.geo_source_ips, "geo-source-ip")
                .into_iter()
                .map(|ip| ip.to_string())
                .collect(),
            geo_source_headers: if self.geo_source_headers.is_empty()
                && !self.geo_source_ips.is_empty()
            {
                crate::conformance::self_test::default_geo_source_headers()
            } else {
                self.geo_source_headers.clone()
            },
        };

        let generator = K6ScriptGenerator::new(k6_config, templates);
        let mut script = generator.generate()?;
        TerminalReporter::print_success("k6 script generated");

        // Check if any advanced features are enabled
        let has_advanced_features = self.data_file.is_some()
            || self.error_rate.is_some()
            || self.security_test
            || self.parallel_create.is_some()
            || self.wafbench_dir.is_some();

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
        let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));

        std::fs::create_dir_all(&self.output)?;

        let results = executor.execute(&script_path, Some(&self.output), self.verbose).await?;

        // Print results
        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary_full(
            &results,
            duration_secs,
            self.no_keep_alive,
            Some(num_ops),
        );

        println!("\nResults saved to: {}", self.output.display());

        Ok(())
    }

    /// Execute multi-target bench testing
    async fn execute_multi_target(&self, targets_file: &Path) -> Result<()> {
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
                target_rps: self.target_rps,
                no_keep_alive: self.no_keep_alive,
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
                chunked_request_bodies: self.chunked_request_bodies,
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
                wafbench_cycle_all: self.wafbench_cycle_all,
                owasp_api_top10: self.owasp_api_top10,
                owasp_categories: self.owasp_categories.clone(),
                owasp_auth_header: self.owasp_auth_header.clone(),
                owasp_auth_token: self.owasp_auth_token.clone(),
                owasp_admin_paths: self.owasp_admin_paths.clone(),
                owasp_id_fields: self.owasp_id_fields.clone(),
                owasp_report: self.owasp_report.clone(),
                owasp_report_format: self.owasp_report_format.clone(),
                owasp_iterations: self.owasp_iterations,
                conformance: false,
                conformance_api_key: None,
                conformance_basic_auth: None,
                conformance_report: PathBuf::from("conformance-report.json"),
                conformance_categories: None,
                conformance_report_format: "json".to_string(),
                conformance_headers: vec![],
                conformance_all_operations: false,
                conformance_custom: None,
                conformance_delay_ms: 0,
                use_k6: false,
                conformance_custom_filter: None,
                export_requests: false,
                validate_requests: false,
                conformance_self_test: false,
                conformance_self_test_capture: false,
                source_ips: Vec::new(),
                geo_source_ips: Vec::new(),
                geo_source_headers: Vec::new(),
                report_missed_cap: None,
            },
            targets,
            max_concurrency,
        );

        // Execute all targets
        let start_time = std::time::Instant::now();
        let aggregated_results = executor.execute_all().await?;
        let elapsed = start_time.elapsed();

        // Organize and report results
        self.report_multi_target_results(&aggregated_results, elapsed)?;

        Ok(())
    }

    /// Report results for multi-target execution
    fn report_multi_target_results(
        &self,
        results: &AggregatedResults,
        elapsed: std::time::Duration,
    ) -> Result<()> {
        // Print summary
        TerminalReporter::print_multi_target_summary(results);

        // Print elapsed time
        let total_secs = elapsed.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        if hours > 0 {
            println!("\n  Total Elapsed Time:   {}h {}m {}s", hours, minutes, seconds);
        } else if minutes > 0 {
            println!("\n  Total Elapsed Time:   {}m {}s", minutes, seconds);
        } else {
            println!("\n  Total Elapsed Time:   {}s", seconds);
        }

        // Save aggregated summary if requested
        if self.results_format == "aggregated" || self.results_format == "both" {
            let summary_path = self.output.join("aggregated_summary.json");
            let summary_json = serde_json::json!({
                "total_elapsed_seconds": elapsed.as_secs(),
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
                    "total_rps": results.aggregated_metrics.total_rps,
                    "avg_rps": results.aggregated_metrics.avg_rps,
                    "total_vus_max": results.aggregated_metrics.total_vus_max,
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
                        "min_duration_ms": r.results.min_duration_ms,
                        "med_duration_ms": r.results.med_duration_ms,
                        "p90_duration_ms": r.results.p90_duration_ms,
                        "p95_duration_ms": r.results.p95_duration_ms,
                        "p99_duration_ms": r.results.p99_duration_ms,
                        "max_duration_ms": r.results.max_duration_ms,
                        "rps": r.results.rps,
                        "vus_max": r.results.vus_max,
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

        // Write CSV with all per-target results for easy parsing
        let csv_path = self.output.join("all_targets.csv");
        let mut csv = String::from(
            "target_url,success,requests,failed,rps,vus,min_ms,avg_ms,med_ms,p90_ms,p95_ms,p99_ms,max_ms,error\n",
        );
        for r in &results.target_results {
            csv.push_str(&format!(
                "{},{},{},{},{:.1},{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{}\n",
                r.target_url,
                r.success,
                r.results.total_requests,
                r.results.failed_requests,
                r.results.rps,
                r.results.vus_max,
                r.results.min_duration_ms,
                r.results.avg_duration_ms,
                r.results.med_duration_ms,
                r.results.p90_duration_ms,
                r.results.p95_duration_ms,
                r.results.p99_duration_ms,
                r.results.max_duration_ms,
                r.error.as_deref().unwrap_or(""),
            ));
        }
        let _ = std::fs::write(&csv_path, &csv);

        println!("\nResults saved to: {}", self.output.display());
        println!("  - Per-target results: {}", self.output.join("target_*").display());
        println!("  - All targets CSV:    {}", csv_path.display());
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
        match &self.headers {
            Some(s) => parse_header_string(s),
            None => Ok(HashMap::new()),
        }
    }

    fn parse_extracted_values(output_dir: &Path) -> Result<ExtractedValues> {
        let extracted_path = output_dir.join("extracted_values.json");
        if !extracted_path.exists() {
            return Ok(ExtractedValues::new());
        }

        let content = std::fs::read_to_string(&extracted_path)
            .map_err(|e| BenchError::ResultsParseError(e.to_string()))?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| BenchError::ResultsParseError(e.to_string()))?;

        let mut extracted = ExtractedValues::new();
        if let Some(values) = parsed.as_object() {
            for (key, value) in values {
                extracted.set(key.clone(), value.clone());
            }
        }

        Ok(extracted)
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
                let mut default = HashSet::new();
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
            // Also report any parse errors that may explain why no files were processed
            if !stats.parse_errors.is_empty() {
                TerminalReporter::print_warning("Some files were found but failed to parse:");
                for error in &stats.parse_errors {
                    TerminalReporter::print_warning(&format!("  - {}", error));
                }
            }
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
    pub(crate) fn generate_enhanced_script(&self, base_script: &str) -> Result<String> {
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
        let security_requested = security_config.is_some() || self.wafbench_dir.is_some();

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

            additional_code.push_str(&SecurityTestGenerator::generate_payload_selection(
                &payload_list,
                self.wafbench_cycle_all,
            ));
            additional_code.push('\n');
            additional_code
                .push_str(&SecurityTestGenerator::generate_apply_payload(&target_fields));
            additional_code.push('\n');
            additional_code.push_str(&SecurityTestGenerator::generate_security_checks());
            additional_code.push('\n');

            let mode = if self.wafbench_cycle_all {
                "cycle-all"
            } else {
                "random"
            };
            TerminalReporter::print_success(&format!(
                "Security testing enabled ({} payloads, {} mode)",
                payload_list.len(),
                mode
            ));
        } else if security_requested {
            // User requested security testing (e.g., --wafbench-dir) but no payloads were loaded.
            // The template has security_testing_enabled=true so it renders calling code.
            // We must inject stub definitions to avoid undefined function references.
            TerminalReporter::print_warning(
                "Security testing was requested but no payloads were loaded. \
                 Ensure --wafbench-dir points to valid CRS YAML files or add --security-test.",
            );
            additional_code
                .push_str(&SecurityTestGenerator::generate_payload_selection(&[], false));
            additional_code.push('\n');
            additional_code.push_str(&SecurityTestGenerator::generate_apply_payload(&[]));
            additional_code.push('\n');
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

            // Find the spec in our loaded specs (match by full path or filename)
            let spec = all_specs
                .iter()
                .find(|(p, _)| {
                    p == spec_path
                        || p.file_name() == spec_path.file_name()
                        || p.file_name() == Some(spec_path.as_os_str())
                })
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
        let mut handlebars = handlebars::Handlebars::new();
        // Register json helper for serializing arrays/objects in templates
        handlebars.register_helper(
            "json",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).map(|v| v.value()).unwrap_or(&serde_json::Value::Null);
                    out.write(&serde_json::to_string(param).unwrap_or_else(|_| "[]".to_string()))?;
                    Ok(())
                },
            ),
        );
        let template = include_str!("templates/k6_crud_flow.hbs");
        let output_dir = self.output.join(format!("{}_results", spec_name.replace('.', "_")));

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

        // Track all dynamic placeholders across all operations
        let mut all_placeholders: HashSet<DynamicPlaceholder> = HashSet::new();

        let flows_data: Vec<serde_json::Value> = flows.iter().map(|f| {
            // Use the metric-name sanitizer (caps at 112 chars + hash suffix)
            // so deeply nested flow names don't blow past k6's 128-char limit
            // when concatenated with `_step{i}_latency`. See issue #79.
            let sanitized_name = K6ScriptGenerator::sanitize_k6_metric_name(&f.name);
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
                            .map(|po| po.get_for_operation(None, &method_raw, raw_path))
                            .and_then(|oo| oo.body)
                            .unwrap_or_else(|| serde_json::json!({}))
                    } else {
                        serde_json::json!({})
                    };

                    // Process body for dynamic placeholders like ${__VU}, ${__ITER}, etc.
                    let processed_body = DynamicParamProcessor::process_json_body(&body_value);

                    // Also check for ${extracted.xxx} placeholders which need runtime substitution
                    let body_has_extracted_placeholders = processed_body.value.contains("${extracted.");
                    let body_is_dynamic = processed_body.is_dynamic || body_has_extracted_placeholders;

                    serde_json::json!({
                        "operation": s.operation,
                        "method": method,
                        "path": path,
                        "extract": s.extract,
                        "use_values": s.use_values,
                        "use_body": s.use_body,
                        "merge_body": if s.merge_body.is_empty() { None } else { Some(&s.merge_body) },
                        "inject_attacks": s.inject_attacks,
                        "attack_types": s.attack_types,
                        "description": s.description,
                        "display_name": s.description.clone().unwrap_or_else(|| format!("Step {}", idx)),
                        "is_get_or_head": is_get_or_head,
                        "has_body": has_body,
                        "body": processed_body.value,
                        "body_is_dynamic": body_is_dynamic,
                        "_placeholders": processed_body.placeholders.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect();

        // Collect all placeholders from all steps
        for flow_data in &flows_data {
            if let Some(steps) = flow_data.get("steps").and_then(|s| s.as_array()) {
                for step in steps {
                    if let Some(placeholders_arr) =
                        step.get("_placeholders").and_then(|p| p.as_array())
                    {
                        for p_str in placeholders_arr {
                            if let Some(p_name) = p_str.as_str() {
                                match p_name {
                                    "VU" => {
                                        all_placeholders.insert(DynamicPlaceholder::VU);
                                    }
                                    "Iteration" => {
                                        all_placeholders.insert(DynamicPlaceholder::Iteration);
                                    }
                                    "Timestamp" => {
                                        all_placeholders.insert(DynamicPlaceholder::Timestamp);
                                    }
                                    "UUID" => {
                                        all_placeholders.insert(DynamicPlaceholder::UUID);
                                    }
                                    "Random" => {
                                        all_placeholders.insert(DynamicPlaceholder::Random);
                                    }
                                    "Counter" => {
                                        all_placeholders.insert(DynamicPlaceholder::Counter);
                                    }
                                    "Date" => {
                                        all_placeholders.insert(DynamicPlaceholder::Date);
                                    }
                                    "VuIter" => {
                                        all_placeholders.insert(DynamicPlaceholder::VuIter);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // Get required imports and globals based on placeholders used
        let required_imports = DynamicParamProcessor::get_required_imports(&all_placeholders);
        let required_globals = DynamicParamProcessor::get_required_globals(&all_placeholders);

        // Check if security testing is enabled
        let security_testing_enabled = self.wafbench_dir.is_some() || self.security_test;

        let data = serde_json::json!({
            "base_url": self.target,
            "flows": flows_data,
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
            "dynamic_imports": required_imports,
            "dynamic_globals": required_globals,
            "extracted_values_output_path": output_dir.join("extracted_values.json").to_string_lossy(),
            // Security testing settings
            "security_testing_enabled": security_testing_enabled,
            "has_custom_headers": !custom_headers.is_empty(),
        });

        let mut script = handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))?;

        // Enhance script with security testing support if enabled
        if security_testing_enabled {
            script = self.generate_enhanced_script(&script)?;
        }

        // Write and execute script
        let script_path =
            self.output.join(format!("k6-{}-crud-flow.js", spec_name.replace('.', "_")));

        std::fs::create_dir_all(self.output.clone())?;
        std::fs::write(&script_path, &script)?;

        if !self.generate_only {
            let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
            std::fs::create_dir_all(&output_dir)?;

            executor.execute(&script_path, Some(&output_dir), self.verbose).await?;

            let extracted = Self::parse_extracted_values(&output_dir)?;
            TerminalReporter::print_progress(&format!(
                "  Extracted {} value(s) from {}",
                extracted.values.len(),
                spec_name
            ));
            return Ok(extracted);
        }

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

        let security_testing_enabled = self.security_test || self.wafbench_dir.is_some();

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
            security_testing_enabled,
            chunked_request_bodies: self.chunked_request_bodies,
            target_rps: self.target_rps,
            no_keep_alive: self.no_keep_alive,
            // Round 22.3 — see other K6Config site above.
            geo_source_ips: parse_ip_list(&self.geo_source_ips, "geo-source-ip")
                .into_iter()
                .map(|ip| ip.to_string())
                .collect(),
            geo_source_headers: if self.geo_source_headers.is_empty()
                && !self.geo_source_ips.is_empty()
            {
                crate::conformance::self_test::default_geo_source_headers()
            } else {
                self.geo_source_headers.clone()
            },
        };

        let generator = K6ScriptGenerator::new(k6_config, templates);
        let mut script = generator.generate()?;

        // Enhance script with advanced features (security testing, etc.)
        let has_advanced_features = self.data_file.is_some()
            || self.error_rate.is_some()
            || self.security_test
            || self.parallel_create.is_some()
            || self.wafbench_dir.is_some();

        if has_advanced_features {
            script = self.generate_enhanced_script(&script)?;
        }

        // Write and execute script
        let script_path = self.output.join(format!("k6-{}.js", spec_name.replace('.', "_")));

        std::fs::create_dir_all(self.output.clone())?;
        std::fs::write(&script_path, &script)?;

        if !self.generate_only {
            let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
            let output_dir = self.output.join(format!("{}_results", spec_name.replace('.', "_")));
            std::fs::create_dir_all(&output_dir)?;

            executor.execute(&script_path, Some(&output_dir), self.verbose).await?;
        }

        Ok(())
    }

    /// Execute CRUD flow testing mode
    async fn execute_crud_flow(&self, parser: &SpecParser) -> Result<()> {
        // Check if a custom flow config is provided
        let config = self.build_crud_flow_config().unwrap_or_default();

        // Use flows from config if provided, otherwise auto-detect
        let flows = if !config.flows.is_empty() {
            TerminalReporter::print_progress("Using custom flow configuration...");
            config.flows.clone()
        } else {
            TerminalReporter::print_progress("Detecting CRUD operations...");
            let operations = parser.get_operations();
            CrudFlowDetector::detect_flows(&operations)
        };

        if flows.is_empty() {
            return Err(BenchError::Other(
                "No CRUD flows detected in spec. Ensure spec has POST/GET/PUT/DELETE operations on related paths.".to_string(),
            ));
        }

        if config.flows.is_empty() {
            TerminalReporter::print_success(&format!("Detected {} CRUD flow(s)", flows.len()));
        } else {
            TerminalReporter::print_success(&format!("Loaded {} custom flow(s)", flows.len()));
        }

        for flow in &flows {
            TerminalReporter::print_progress(&format!(
                "  - {}: {} steps",
                flow.name,
                flow.steps.len()
            ));
        }

        // Generate CRUD flow script
        let mut handlebars = handlebars::Handlebars::new();
        // Register json helper for serializing arrays/objects in templates
        handlebars.register_helper(
            "json",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).map(|v| v.value()).unwrap_or(&serde_json::Value::Null);
                    out.write(&serde_json::to_string(param).unwrap_or_else(|_| "[]".to_string()))?;
                    Ok(())
                },
            ),
        );
        let template = include_str!("templates/k6_crud_flow.hbs");

        let custom_headers = self.parse_headers()?;

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

        // Track all dynamic placeholders across all operations
        let mut all_placeholders: HashSet<DynamicPlaceholder> = HashSet::new();

        let flows_data: Vec<serde_json::Value> = flows.iter().map(|f| {
            // Sanitize flow name for use as JavaScript variable and k6 metric names.
            // Use the metric-name sanitizer (caps at 112 chars + hash suffix) so
            // deeply nested flow names don't blow past k6's 128-char limit when
            // concatenated with `_step{i}_latency`. See issue #79.
            let sanitized_name = K6ScriptGenerator::sanitize_k6_metric_name(&f.name);
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

                    // Process body for dynamic placeholders like ${__VU}, ${__ITER}, etc.
                    let processed_body = DynamicParamProcessor::process_json_body(&body_value);
                    // Note: all_placeholders is captured by the closure but we can't mutate it directly
                    // We'll collect placeholders separately below

                    // Also check for ${extracted.xxx} placeholders which need runtime substitution
                    let body_has_extracted_placeholders = processed_body.value.contains("${extracted.");
                    let body_is_dynamic = processed_body.is_dynamic || body_has_extracted_placeholders;

                    serde_json::json!({
                        "operation": s.operation,
                        "method": method,
                        "path": path,
                        "extract": s.extract,
                        "use_values": s.use_values,
                        "use_body": s.use_body,
                        "merge_body": if s.merge_body.is_empty() { None } else { Some(&s.merge_body) },
                        "inject_attacks": s.inject_attacks,
                        "attack_types": s.attack_types,
                        "description": s.description,
                        "display_name": s.description.clone().unwrap_or_else(|| format!("Step {}", idx)),
                        "is_get_or_head": is_get_or_head,
                        "has_body": has_body,
                        "body": processed_body.value,
                        "body_is_dynamic": body_is_dynamic,
                        "_placeholders": processed_body.placeholders.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect();

        // Collect all placeholders from all steps
        for flow_data in &flows_data {
            if let Some(steps) = flow_data.get("steps").and_then(|s| s.as_array()) {
                for step in steps {
                    if let Some(placeholders_arr) =
                        step.get("_placeholders").and_then(|p| p.as_array())
                    {
                        for p_str in placeholders_arr {
                            if let Some(p_name) = p_str.as_str() {
                                // Parse placeholder from debug string
                                match p_name {
                                    "VU" => {
                                        all_placeholders.insert(DynamicPlaceholder::VU);
                                    }
                                    "Iteration" => {
                                        all_placeholders.insert(DynamicPlaceholder::Iteration);
                                    }
                                    "Timestamp" => {
                                        all_placeholders.insert(DynamicPlaceholder::Timestamp);
                                    }
                                    "UUID" => {
                                        all_placeholders.insert(DynamicPlaceholder::UUID);
                                    }
                                    "Random" => {
                                        all_placeholders.insert(DynamicPlaceholder::Random);
                                    }
                                    "Counter" => {
                                        all_placeholders.insert(DynamicPlaceholder::Counter);
                                    }
                                    "Date" => {
                                        all_placeholders.insert(DynamicPlaceholder::Date);
                                    }
                                    "VuIter" => {
                                        all_placeholders.insert(DynamicPlaceholder::VuIter);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // Get required imports and globals based on placeholders used
        let required_imports = DynamicParamProcessor::get_required_imports(&all_placeholders);
        let required_globals = DynamicParamProcessor::get_required_globals(&all_placeholders);

        // Build invalid data config if error injection is enabled
        let invalid_data_config = self.build_invalid_data_config();
        let error_injection_enabled = invalid_data_config.is_some();
        let error_rate = self.error_rate.unwrap_or(0.0);
        let error_types: Vec<String> = invalid_data_config
            .as_ref()
            .map(|c| c.error_types.iter().map(|t| format!("{:?}", t)).collect())
            .unwrap_or_default();

        if error_injection_enabled {
            TerminalReporter::print_progress(&format!(
                "Error injection enabled ({}% rate)",
                (error_rate * 100.0) as u32
            ));
        }

        // Check if security testing is enabled
        let security_testing_enabled = self.wafbench_dir.is_some() || self.security_test;

        let data = serde_json::json!({
            "base_url": self.target,
            "flows": flows_data,
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
            "dynamic_imports": required_imports,
            "dynamic_globals": required_globals,
            "extracted_values_output_path": self
                .output
                .join("crud_flow_extracted_values.json")
                .to_string_lossy(),
            // Error injection settings
            "error_injection_enabled": error_injection_enabled,
            "error_rate": error_rate,
            "error_types": error_types,
            // Security testing settings
            "security_testing_enabled": security_testing_enabled,
            "has_custom_headers": !custom_headers.is_empty(),
        });

        let mut script = handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))?;

        // Enhance script with security testing support if enabled
        if security_testing_enabled {
            script = self.generate_enhanced_script(&script)?;
        }

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
        let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
        std::fs::create_dir_all(&self.output)?;

        let results = executor.execute(&script_path, Some(&self.output), self.verbose).await?;

        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary_with_mode(&results, duration_secs, self.no_keep_alive);

        Ok(())
    }

    /// Execute OpenAPI 3.0.0 conformance testing mode
    async fn execute_conformance_test(&self) -> Result<()> {
        use crate::conformance::generator::{ConformanceConfig, ConformanceGenerator};
        use crate::conformance::report::ConformanceReport;
        use crate::conformance::spec::ConformanceFeature;

        TerminalReporter::print_progress("OpenAPI 3.0.0 Conformance Testing Mode");

        // Conformance testing is a functional correctness check (1 VU, 1 iteration).
        // --vus and -d flags are always ignored in this mode.
        TerminalReporter::print_progress(
            "Conformance mode runs 1 VU, 1 iteration per endpoint (--vus and -d are ignored)",
        );

        // Parse category filter
        let categories = self.conformance_categories.as_ref().map(|cats_str| {
            cats_str
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    if let Some(canonical) = ConformanceFeature::category_from_cli_name(trimmed) {
                        Some(canonical.to_string())
                    } else {
                        TerminalReporter::print_warning(&format!(
                            "Unknown conformance category: '{}'. Valid categories: {}",
                            trimmed,
                            ConformanceFeature::cli_category_names()
                                .iter()
                                .map(|(cli, _)| *cli)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        None
                    }
                })
                .collect::<Vec<String>>()
        });

        // Parse custom headers from "Key: Value" format
        let custom_headers: Vec<(String, String)> = self
            .conformance_headers
            .iter()
            .filter_map(|h| {
                let (name, value) = h.split_once(':')?;
                Some((name.trim().to_string(), value.trim().to_string()))
            })
            .collect();

        if !custom_headers.is_empty() {
            TerminalReporter::print_progress(&format!(
                "Using {} custom header(s) for authentication",
                custom_headers.len()
            ));
        }

        if self.conformance_delay_ms > 0 {
            TerminalReporter::print_progress(&format!(
                "Using {}ms delay between conformance requests",
                self.conformance_delay_ms
            ));
        }

        // Ensure output dir exists so canonicalize works for the report path
        std::fs::create_dir_all(&self.output)?;

        let config = ConformanceConfig {
            target_url: self.target.clone(),
            api_key: self.conformance_api_key.clone(),
            basic_auth: self.conformance_basic_auth.clone(),
            skip_tls_verify: self.skip_tls_verify,
            categories,
            base_path: self.base_path.clone(),
            custom_headers,
            output_dir: Some(self.output.clone()),
            all_operations: self.conformance_all_operations,
            custom_checks_file: self.conformance_custom.clone(),
            request_delay_ms: self.conformance_delay_ms,
            custom_filter: self.conformance_custom_filter.clone(),
            export_requests: self.export_requests,
            validate_requests: self.validate_requests,
        };

        // Branch: spec-driven mode vs reference mode
        // Annotate operations if spec is provided (used by both native and k6 paths)
        // Round 18.1 — resolve the spec's base path so the self-test
        // path can prepend it to every URL. Pre-fix, self-test
        // ignored `--base-path /api` and hit the bare spec path,
        // returning 404 for every request on specs whose server is
        // proxied behind a base prefix.
        let mut resolved_base_path: Option<String> = None;
        let annotated_ops = if !self.spec.is_empty() {
            TerminalReporter::print_progress("Spec-driven conformance mode: analyzing spec...");
            let parser = SpecParser::from_file(&self.spec[0]).await?;
            resolved_base_path = self.resolve_base_path(&parser);

            // Issue #79 round 12 — Srikanth ran `--conformance --operations "GET,POST"`
            // and saw DELETE/PATCH exercised anyway. Conformance silently ignored
            // the filter. Apply it (and `--exclude-operations`) the same way the
            // regular bench path does so users can scope the run.
            let mut operations = if let Some(filter) = &self.operations {
                parser.filter_operations(filter)?
            } else {
                parser.get_operations()
            };
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

            let annotated =
                crate::conformance::spec_driven::SpecDrivenConformanceGenerator::annotate_operations(
                    &operations,
                    parser.spec(),
                );
            TerminalReporter::print_success(&format!(
                "Analyzed {} operations, found {} feature annotations",
                operations.len(),
                annotated.iter().map(|a| a.features.len()).sum::<usize>()
            ));
            Some(annotated)
        } else {
            None
        };

        // Issue #79 round 13 (4) — `--conformance-self-test` replaces
        // the standard conformance run with a positive + per-category
        // negative driver that verifies the server actually rejects
        // bad requests with 4xx. Wires the spec-annotated operations
        // through `conformance::self_test::run_self_test` and prints
        // the resulting pass/fail matrix.
        if self.conformance_self_test {
            let Some(ops) = annotated_ops else {
                TerminalReporter::print_error(
                    "--conformance-self-test requires --spec; no operations to test",
                );
                return Ok(());
            };
            let cfg = crate::conformance::self_test::SelfTestConfig {
                target_url: self.target.clone(),
                skip_tls_verify: self.skip_tls_verify,
                timeout: std::time::Duration::from_secs(30),
                // `custom_headers` was already moved into the
                // `ConformanceConfig` above; re-derive from `self` so
                // we don't borrow it twice.
                extra_headers: self
                    .conformance_headers
                    .iter()
                    .filter_map(|h| {
                        let (n, v) = h.split_once(':')?;
                        Some((n.trim().to_string(), v.trim().to_string()))
                    })
                    .collect(),
                delay_between_requests: std::time::Duration::from_millis(self.conformance_delay_ms),
                // Round 18.1 — honour `--base-path` (or the spec's
                // own first server prefix) so a deployment served
                // under a path-prefix doesn't 404 every positive.
                base_path: resolved_base_path.clone(),
                // Round 18.5 — GEODB multi-source-IP. Parse CLI IP
                // lists (malformed entries log a warning and are
                // dropped). Empty lists keep pre-18.5 behaviour.
                source_ips: parse_ip_list(&self.source_ips, "source-ip"),
                geo_source_ips: parse_ip_list(&self.geo_source_ips, "geo-source-ip"),
                geo_source_headers: if self.geo_source_headers.is_empty() {
                    crate::conformance::self_test::default_geo_source_headers()
                } else {
                    self.geo_source_headers.clone()
                },
                // Round 23 (c-iii) — opt-in request/response capture.
                // Constructed here so the sink Arc outlives the run and
                // we can drain it for the JSONL write below.
                capture: if self.conformance_self_test_capture {
                    Some(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())))
                } else {
                    None
                },
            };
            let capture_sink = cfg.capture.clone();
            TerminalReporter::print_progress(&format!(
                "Self-test mode: driving {} operations with positive + per-category negative cases",
                ops.len()
            ));
            let report = crate::conformance::self_test::run_self_test(&ops, &cfg)
                .await
                .map_err(|e| BenchError::Other(format!("self-test client error: {e}")))?;
            // Round 23 (c-iii) — drain the capture sink into a JSONL
            // file next to the JSON/HTML report. One CaseCapture per
            // line so the file is grep-able / streamable. Round 24
            // (d) — also emit a self-contained HTML viewer at
            // `conformance-self-test-requests.html` for users who
            // want to browse the capture without piping through `jq`.
            if let Some(sink) = capture_sink {
                if let Ok(guard) = sink.lock() {
                    let jsonl_path = self.output.join("conformance-self-test-requests.jsonl");
                    let mut lines = String::with_capacity(guard.len() * 256);
                    for entry in guard.iter() {
                        if let Ok(line) = serde_json::to_string(entry) {
                            lines.push_str(&line);
                            lines.push('\n');
                        }
                    }
                    let _ = std::fs::write(&jsonl_path, lines);
                    let html_path = self.output.join("conformance-self-test-requests.html");
                    let html =
                        crate::conformance::capture_html::render_capture_html(guard.as_slice());
                    let _ = std::fs::write(&html_path, html);
                    TerminalReporter::print_progress(&format!(
                        "Self-test request/response capture written to {} ({} entries) + {}",
                        jsonl_path.display(),
                        guard.len(),
                        html_path.display(),
                    ));
                }
            }
            TerminalReporter::print_progress(&report.render_summary());
            // Persist the JSON report alongside the regular conformance
            // report so it's grep-able next to the buffer dump from the
            // admin endpoint.
            let json_path = self.output.join("conformance-self-test.json");
            if let Ok(json) = serde_json::to_string_pretty(&report) {
                let _ = std::fs::write(&json_path, json);
                TerminalReporter::print_progress(&format!(
                    "Self-test report written to {}",
                    json_path.display()
                ));
            }
            // Round 18.1 — surface the "every positive failed with
            // the same status" case loudly. Without this, a user
            // who forgot `--base-path /api` saw 404 for every
            // request, but the per-category negative rollup looked
            // all-green (because 404 is in the 4xx range the
            // negatives expect). Now the run is correctly called
            // out as misconfigured before showing the (meaningless)
            // negative results.
            if let Some(status) = report.detect_target_misconfiguration() {
                let hint = match status {
                    404 => " Likely cause: spec paths don't match deployed routes — check --base-path and the spec's `servers` block.",
                    401 | 403 => " Likely cause: authentication header is missing or invalid — check --conformance-header.",
                    _ => "",
                };
                TerminalReporter::print_warning(&format!(
                    "Self-test misconfiguration: every positive case returned {status}.{hint} Negative results below are meaningless under this condition."
                ));
            } else if !report.all_passed() {
                TerminalReporter::print_warning(
                    "Self-test detected gaps — server let through at least one request that should have been a 4xx",
                );
            } else {
                TerminalReporter::print_success(
                    "Self-test passed — all positive cases accepted and all negative cases rejected",
                );
            }
            // Round 17.6 — emit a self-contained HTML report alongside
            // the JSON. Groups by category and surfaces the missed-
            // negative list directly so a user doesn't need to grep
            // through the JSON to find which routes failed which
            // checks. Optionally folds in a round-17.4 spec audit
            // report if one exists in the same output directory.
            let html_path = self.output.join("conformance-report.html");
            let audit_path = self.output.join("conformance-spec-audit.json");
            let audit_value = std::fs::read_to_string(&audit_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            // Round 21.1 — `--report-missed-cap N` lets the user
            // override the default 200-row HTML drill-down cap.
            // `--report-missed-cap 0` maps to `None` (no cap; show
            // everything). The JSON report always has the full set.
            let render_opts = crate::conformance::report_html::RenderOptions {
                missed_cap: match self.report_missed_cap {
                    Some(0) => None,
                    Some(n) => Some(n as usize),
                    None => Some(200),
                },
            };
            let html = crate::conformance::report_html::render_html_with_options(
                &report,
                audit_value.as_ref(),
                &render_opts,
            );
            if std::fs::write(&html_path, html).is_ok() {
                TerminalReporter::print_progress(&format!(
                    "HTML report written to {}",
                    html_path.display()
                ));
            }
            return Ok(());
        }

        // Request validation against OpenAPI spec (if --validate-requests is set)
        if self.validate_requests && !self.spec.is_empty() {
            TerminalReporter::print_progress("Validating requests against OpenAPI spec...");
            let violation_count = crate::conformance::request_validator::run_request_validation(
                &self.spec,
                self.conformance_custom.as_deref(),
                self.base_path.as_deref(),
                &self.output,
            )
            .await?;
            if violation_count > 0 {
                TerminalReporter::print_warning(&format!(
                    "{} request validation violation(s) found — see conformance-request-violations.json",
                    violation_count
                ));
            } else {
                TerminalReporter::print_success("All requests conform to the OpenAPI spec");
            }
        }

        // If generate-only OR --use-k6, use the k6 script generation path
        if self.generate_only || self.use_k6 {
            let script = if let Some(annotated) = &annotated_ops {
                let gen = crate::conformance::spec_driven::SpecDrivenConformanceGenerator::new(
                    config,
                    annotated.clone(),
                );
                let op_count = gen.operation_count();
                let (script, check_count) = gen.generate()?;
                TerminalReporter::print_success(&format!(
                    "Conformance: {} operations analyzed, {} unique checks generated",
                    op_count, check_count
                ));
                script
            } else {
                let generator = ConformanceGenerator::new(config);
                generator.generate()?
            };

            let script_path = self.output.join("k6-conformance.js");
            std::fs::write(&script_path, &script).map_err(|e| {
                BenchError::Other(format!("Failed to write conformance script: {}", e))
            })?;
            TerminalReporter::print_success(&format!(
                "Conformance script generated: {}",
                script_path.display()
            ));

            if self.generate_only {
                println!("\nScript generated. Run with:");
                println!("  k6 run {}", script_path.display());
                return Ok(());
            }

            // --use-k6: execute via k6
            if !K6Executor::is_k6_installed() {
                TerminalReporter::print_error("k6 is not installed");
                TerminalReporter::print_warning(
                    "Install k6 from: https://k6.io/docs/get-started/installation/",
                );
                return Err(BenchError::K6NotFound);
            }

            TerminalReporter::print_progress("Running conformance tests via k6...");
            let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
            executor.execute(&script_path, Some(&self.output), self.verbose).await?;

            let report_path = self.output.join("conformance-report.json");
            if report_path.exists() {
                let report = ConformanceReport::from_file(&report_path)?;
                report.print_report_with_options(self.conformance_all_operations);
                self.save_conformance_report(&report, &report_path)?;
            } else {
                TerminalReporter::print_warning(
                    "Conformance report not generated (k6 handleSummary may not have run)",
                );
            }

            return Ok(());
        }

        // Default: Native Rust executor (no k6 dependency)
        TerminalReporter::print_progress("Running conformance tests (native executor)...");

        let mut executor = crate::conformance::executor::NativeConformanceExecutor::new(config)?;

        executor = if let Some(annotated) = &annotated_ops {
            executor.with_spec_driven_checks(annotated)
        } else {
            executor.with_reference_checks()
        };
        executor = executor.with_custom_checks()?;

        TerminalReporter::print_success(&format!(
            "Executing {} conformance checks...",
            executor.check_count()
        ));

        let report = executor.execute().await?;
        report.print_report_with_options(self.conformance_all_operations);

        // Save failure details to a separate file for easy debugging
        let failure_details = report.failure_details();
        if !failure_details.is_empty() {
            let details_path = self.output.join("conformance-failure-details.json");
            if let Ok(json) = serde_json::to_string_pretty(&failure_details) {
                let _ = std::fs::write(&details_path, json);
                TerminalReporter::print_success(&format!(
                    "Failure details saved to: {}",
                    details_path.display()
                ));
            }
        }

        // Save report
        let report_path = self.output.join("conformance-report.json");
        let report_json = serde_json::to_string_pretty(&report.to_json())
            .map_err(|e| BenchError::Other(format!("Failed to serialize report: {}", e)))?;
        std::fs::write(&report_path, &report_json)
            .map_err(|e| BenchError::Other(format!("Failed to write report: {}", e)))?;
        TerminalReporter::print_success(&format!("Report saved to: {}", report_path.display()));

        self.save_conformance_report(&report, &report_path)?;

        Ok(())
    }

    /// Save conformance report in the requested format (SARIF or JSON copy)
    fn save_conformance_report(
        &self,
        report: &crate::conformance::report::ConformanceReport,
        report_path: &Path,
    ) -> Result<()> {
        if self.conformance_report_format == "sarif" {
            use crate::conformance::sarif::ConformanceSarifReport;
            ConformanceSarifReport::write(report, &self.target, &self.conformance_report)?;
            TerminalReporter::print_success(&format!(
                "SARIF report saved to: {}",
                self.conformance_report.display()
            ));
        } else if self.conformance_report != *report_path {
            std::fs::copy(report_path, &self.conformance_report)?;
            TerminalReporter::print_success(&format!(
                "Report saved to: {}",
                self.conformance_report.display()
            ));
        }
        Ok(())
    }

    /// Execute conformance tests against multiple targets from a targets file.
    ///
    /// Uses the native `NativeConformanceExecutor` (no k6 dependency). Targets are
    /// tested sequentially to avoid overwhelming them, and per-target headers from
    /// the targets file are merged with the base `--conformance-header` headers.
    async fn execute_multi_target_conformance(&self, targets_file: &Path) -> Result<()> {
        use crate::conformance::generator::{ConformanceConfig, ConformanceGenerator};
        use crate::conformance::report::ConformanceReport;
        use crate::conformance::spec::ConformanceFeature;

        TerminalReporter::print_progress("Multi-target OpenAPI 3.0.0 Conformance Testing Mode");

        // Parse targets file
        TerminalReporter::print_progress("Parsing targets file...");
        let targets = parse_targets_file(targets_file)?;
        let num_targets = targets.len();
        TerminalReporter::print_success(&format!("Loaded {} targets", num_targets));

        if targets.is_empty() {
            return Err(BenchError::Other("No targets found in file".to_string()));
        }

        TerminalReporter::print_progress(
            "Conformance mode runs 1 VU, 1 iteration per endpoint (--vus and -d are ignored)",
        );

        // Parse category filter (shared across all targets)
        let categories = self.conformance_categories.as_ref().map(|cats_str| {
            cats_str
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    if let Some(canonical) = ConformanceFeature::category_from_cli_name(trimmed) {
                        Some(canonical.to_string())
                    } else {
                        TerminalReporter::print_warning(&format!(
                            "Unknown conformance category: '{}'. Valid categories: {}",
                            trimmed,
                            ConformanceFeature::cli_category_names()
                                .iter()
                                .map(|(cli, _)| *cli)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        None
                    }
                })
                .collect::<Vec<String>>()
        });

        // Parse base custom headers from --conformance-header flags
        let base_custom_headers: Vec<(String, String)> = self
            .conformance_headers
            .iter()
            .filter_map(|h| {
                let (name, value) = h.split_once(':')?;
                Some((name.trim().to_string(), value.trim().to_string()))
            })
            .collect();

        if !base_custom_headers.is_empty() {
            TerminalReporter::print_progress(&format!(
                "Using {} base custom header(s) for authentication",
                base_custom_headers.len()
            ));
        }

        // Load spec once if provided (shared across all targets)
        let annotated_ops = if !self.spec.is_empty() {
            TerminalReporter::print_progress("Spec-driven conformance mode: analyzing spec...");
            let parser = SpecParser::from_file(&self.spec[0]).await?;
            let operations = parser.get_operations();
            let annotated =
                crate::conformance::spec_driven::SpecDrivenConformanceGenerator::annotate_operations(
                    &operations,
                    parser.spec(),
                );
            TerminalReporter::print_success(&format!(
                "Analyzed {} operations, found {} feature annotations",
                operations.len(),
                annotated.iter().map(|a| a.features.len()).sum::<usize>()
            ));
            Some(annotated)
        } else {
            None
        };

        // Ensure output dir exists
        std::fs::create_dir_all(&self.output)?;

        // Collect per-target results for the summary
        struct TargetResult {
            url: String,
            passed: usize,
            failed: usize,
            elapsed: std::time::Duration,
            report_json: serde_json::Value,
            owasp_coverage: Vec<crate::conformance::report::OwaspCoverageEntry>,
        }

        let mut target_results: Vec<TargetResult> = Vec::with_capacity(num_targets);
        let total_start = std::time::Instant::now();

        for (idx, target) in targets.iter().enumerate() {
            tracing::info!(
                "Running conformance tests against target {}/{}: {}",
                idx + 1,
                num_targets,
                target.url
            );
            TerminalReporter::print_progress(&format!(
                "\n--- Target {}/{}: {} ---",
                idx + 1,
                num_targets,
                target.url
            ));

            // Merge base headers with per-target headers
            let mut merged_headers = base_custom_headers.clone();
            if let Some(ref target_headers) = target.headers {
                for (name, value) in target_headers {
                    // Per-target headers override base headers with the same name
                    if let Some(existing) = merged_headers.iter_mut().find(|(n, _)| n == name) {
                        existing.1 = value.clone();
                    } else {
                        merged_headers.push((name.clone(), value.clone()));
                    }
                }
            }
            // Add auth header if present on target
            if let Some(ref auth) = target.auth {
                if let Some(existing) =
                    merged_headers.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case("Authorization"))
                {
                    existing.1 = auth.clone();
                } else {
                    merged_headers.push(("Authorization".to_string(), auth.clone()));
                }
            }

            // Per-target output dir (used by both native and k6 paths).
            // Created before the config so we can point the k6 script's
            // handleSummary at the per-target directory rather than the shared
            // parent output dir (otherwise every target would overwrite the
            // same conformance-report.json).
            let target_dir = self.output.join(format!("target_{}", idx));
            std::fs::create_dir_all(&target_dir)?;

            let config = ConformanceConfig {
                target_url: target.url.clone(),
                api_key: self.conformance_api_key.clone(),
                basic_auth: self.conformance_basic_auth.clone(),
                skip_tls_verify: self.skip_tls_verify,
                categories: categories.clone(),
                base_path: self.base_path.clone(),
                custom_headers: merged_headers,
                output_dir: Some(target_dir.clone()),
                all_operations: self.conformance_all_operations,
                custom_checks_file: self.conformance_custom.clone(),
                request_delay_ms: self.conformance_delay_ms,
                custom_filter: self.conformance_custom_filter.clone(),
                export_requests: self.export_requests,
                validate_requests: self.validate_requests,
            };

            let target_start = std::time::Instant::now();
            let report = if self.use_k6 {
                if !K6Executor::is_k6_installed() {
                    TerminalReporter::print_error("k6 is not installed");
                    TerminalReporter::print_warning(
                        "Install k6 from: https://k6.io/docs/get-started/installation/",
                    );
                    return Err(BenchError::K6NotFound);
                }

                let script = if let Some(ref annotated) = annotated_ops {
                    let gen = crate::conformance::spec_driven::SpecDrivenConformanceGenerator::new(
                        config.clone(),
                        annotated.clone(),
                    );
                    let (script, _check_count) = gen.generate()?;
                    script
                } else {
                    let generator = ConformanceGenerator::new(config.clone());
                    generator.generate()?
                };

                let script_path = target_dir.join("k6-conformance.js");
                std::fs::write(&script_path, &script).map_err(|e| {
                    BenchError::Other(format!("Failed to write conformance script: {}", e))
                })?;
                TerminalReporter::print_success(&format!(
                    "Conformance script generated: {}",
                    script_path.display()
                ));

                TerminalReporter::print_progress(&format!(
                    "Running conformance tests via k6 against {}...",
                    target.url
                ));
                let k6 = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
                // Unique k6 API port per target to avoid collisions.
                let api_port = 6565u16.saturating_add(idx as u16);
                k6.execute_with_port(&script_path, Some(&target_dir), self.verbose, Some(api_port))
                    .await?;

                let report_path = target_dir.join("conformance-report.json");
                if report_path.exists() {
                    ConformanceReport::from_file(&report_path)?
                } else {
                    TerminalReporter::print_warning(&format!(
                        "Conformance report not generated for target {} (k6 handleSummary may not have run)",
                        target.url
                    ));
                    continue;
                }
            } else {
                let mut executor =
                    crate::conformance::executor::NativeConformanceExecutor::new(config)?;

                executor = if let Some(ref annotated) = annotated_ops {
                    executor.with_spec_driven_checks(annotated)
                } else {
                    executor.with_reference_checks()
                };
                executor = executor.with_custom_checks()?;

                TerminalReporter::print_success(&format!(
                    "Executing {} conformance checks against {}...",
                    executor.check_count(),
                    target.url
                ));

                executor.execute().await?
            };
            let target_elapsed = target_start.elapsed();

            let report_json = report.to_json();

            // Extract pass/fail from the summary in the JSON
            let passed = report_json["summary"]["passed"].as_u64().unwrap_or(0) as usize;
            let failed = report_json["summary"]["failed"].as_u64().unwrap_or(0) as usize;
            let total_checks = passed + failed;
            let rate = if total_checks == 0 {
                0.0
            } else {
                (passed as f64 / total_checks as f64) * 100.0
            };

            TerminalReporter::print_success(&format!(
                "Target {}: {}/{} passed ({:.1}%) in {:.1}s",
                target.url,
                passed,
                total_checks,
                rate,
                target_elapsed.as_secs_f64()
            ));

            // Save per-target report (target_dir created above)
            let target_report_path = target_dir.join("conformance-report.json");
            let report_str = serde_json::to_string_pretty(&report_json)
                .map_err(|e| BenchError::Other(format!("Failed to serialize report: {}", e)))?;
            std::fs::write(&target_report_path, &report_str)
                .map_err(|e| BenchError::Other(format!("Failed to write report: {}", e)))?;

            // Save failure details if any
            let failure_details = report.failure_details();
            if !failure_details.is_empty() {
                let details_path = target_dir.join("conformance-failure-details.json");
                if let Ok(json) = serde_json::to_string_pretty(&failure_details) {
                    let _ = std::fs::write(&details_path, json);
                }
            }

            // Compute OWASP coverage for this target
            let owasp_coverage = report.owasp_coverage_data();

            target_results.push(TargetResult {
                url: target.url.clone(),
                passed,
                failed,
                elapsed: target_elapsed,
                report_json,
                owasp_coverage,
            });
        }

        let total_elapsed = total_start.elapsed();

        // Print summary table
        println!("\n{}", "=".repeat(80));
        println!("  Multi-Target Conformance Summary");
        println!("{}", "=".repeat(80));
        println!(
            "  {:<40} {:>8} {:>8} {:>8} {:>8}",
            "Target URL", "Passed", "Failed", "Rate", "Time"
        );
        println!("  {}", "-".repeat(76));

        let mut total_passed = 0usize;
        let mut total_failed = 0usize;

        for result in &target_results {
            let total_checks = result.passed + result.failed;
            let rate = if total_checks == 0 {
                0.0
            } else {
                (result.passed as f64 / total_checks as f64) * 100.0
            };

            // Truncate long URLs for display
            let display_url = if result.url.len() > 38 {
                format!("{}...", &result.url[..35])
            } else {
                result.url.clone()
            };

            println!(
                "  {:<40} {:>8} {:>8} {:>7.1}% {:>6.1}s",
                display_url,
                result.passed,
                result.failed,
                rate,
                result.elapsed.as_secs_f64()
            );

            total_passed += result.passed;
            total_failed += result.failed;
        }

        let grand_total = total_passed + total_failed;
        let overall_rate = if grand_total == 0 {
            0.0
        } else {
            (total_passed as f64 / grand_total as f64) * 100.0
        };

        println!("  {}", "-".repeat(76));
        println!(
            "  {:<40} {:>8} {:>8} {:>7.1}% {:>6.1}s",
            format!("TOTAL ({} targets)", num_targets),
            total_passed,
            total_failed,
            overall_rate,
            total_elapsed.as_secs_f64()
        );
        println!("{}", "=".repeat(80));

        // Print per-target OWASP coverage
        for result in &target_results {
            println!("\n  OWASP API Security Top 10 Coverage for {}:", result.url);
            for entry in &result.owasp_coverage {
                let status = if !entry.tested {
                    "-"
                } else if entry.all_passed {
                    "pass"
                } else {
                    "FAIL"
                };
                let via = if entry.via_categories.is_empty() {
                    String::new()
                } else {
                    format!(" (via {})", entry.via_categories.join(", "))
                };
                println!("    {:<12} {:<40} {}{}", entry.id, entry.name, status, via);
            }
        }

        // Save combined summary
        let per_target_summaries: Vec<serde_json::Value> = target_results
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let total_checks = r.passed + r.failed;
                let rate = if total_checks == 0 {
                    0.0
                } else {
                    (r.passed as f64 / total_checks as f64) * 100.0
                };
                let owasp_json: Vec<serde_json::Value> = r
                    .owasp_coverage
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "id": e.id,
                            "name": e.name,
                            "tested": e.tested,
                            "all_passed": e.all_passed,
                            "via_categories": e.via_categories,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "target_url": r.url,
                    "target_index": idx,
                    "checks_passed": r.passed,
                    "checks_failed": r.failed,
                    "total_checks": total_checks,
                    "pass_rate": rate,
                    "elapsed_seconds": r.elapsed.as_secs_f64(),
                    "report": r.report_json,
                    "owasp_coverage": owasp_json,
                })
            })
            .collect();

        let combined_summary = serde_json::json!({
            "total_targets": num_targets,
            "total_checks_passed": total_passed,
            "total_checks_failed": total_failed,
            "overall_pass_rate": overall_rate,
            "total_elapsed_seconds": total_elapsed.as_secs_f64(),
            "targets": per_target_summaries,
        });

        let summary_path = self.output.join("multi-target-conformance-summary.json");
        let summary_str = serde_json::to_string_pretty(&combined_summary)
            .map_err(|e| BenchError::Other(format!("Failed to serialize summary: {}", e)))?;
        std::fs::write(&summary_path, &summary_str)
            .map_err(|e| BenchError::Other(format!("Failed to write summary: {}", e)))?;
        TerminalReporter::print_success(&format!(
            "Combined summary saved to: {}",
            summary_path.display()
        ));

        Ok(())
    }

    /// Execute OWASP API Security Top 10 testing mode
    async fn execute_owasp_test(&self, parser: &SpecParser) -> Result<()> {
        TerminalReporter::print_progress("OWASP API Security Top 10 Testing Mode");

        // Parse custom headers from CLI
        let custom_headers = self.parse_headers()?;

        // Build OWASP configuration from CLI options
        let mut config = OwaspApiConfig::new()
            .with_auth_header(&self.owasp_auth_header)
            .with_verbose(self.verbose)
            .with_insecure(self.skip_tls_verify)
            .with_concurrency(self.vus as usize)
            .with_iterations(self.owasp_iterations as usize)
            .with_base_path(self.base_path.clone())
            .with_custom_headers(custom_headers);

        // Set valid auth token if provided
        if let Some(ref token) = self.owasp_auth_token {
            config = config.with_valid_auth_token(token);
        }

        // Parse categories if provided
        if let Some(ref cats_str) = self.owasp_categories {
            let categories: Vec<OwaspCategory> = cats_str
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    match trimmed.parse::<OwaspCategory>() {
                        Ok(cat) => Some(cat),
                        Err(e) => {
                            TerminalReporter::print_warning(&e);
                            None
                        }
                    }
                })
                .collect();

            if !categories.is_empty() {
                config = config.with_categories(categories);
            }
        }

        // Load admin paths from file if provided
        if let Some(ref admin_paths_file) = self.owasp_admin_paths {
            config.admin_paths_file = Some(admin_paths_file.clone());
            if let Err(e) = config.load_admin_paths() {
                TerminalReporter::print_warning(&format!("Failed to load admin paths file: {}", e));
            }
        }

        // Set ID fields if provided
        if let Some(ref id_fields_str) = self.owasp_id_fields {
            let id_fields: Vec<String> = id_fields_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !id_fields.is_empty() {
                config = config.with_id_fields(id_fields);
            }
        }

        // Set report path and format
        if let Some(ref report_path) = self.owasp_report {
            config = config.with_report_path(report_path);
        }
        if let Ok(format) = self.owasp_report_format.parse::<ReportFormat>() {
            config = config.with_report_format(format);
        }

        // Print configuration summary
        let categories = config.categories_to_test();
        TerminalReporter::print_success(&format!(
            "Testing {} OWASP categories: {}",
            categories.len(),
            categories.iter().map(|c| c.cli_name()).collect::<Vec<_>>().join(", ")
        ));

        if config.valid_auth_token.is_some() {
            TerminalReporter::print_progress("Using provided auth token for baseline requests");
        }

        // Create the OWASP generator
        TerminalReporter::print_progress("Generating OWASP security test script...");
        let generator = OwaspApiGenerator::new(config, self.target.clone(), parser);

        // Generate the script
        let script = generator.generate()?;
        TerminalReporter::print_success("OWASP security test script generated");

        // Write script to file
        let script_path = if let Some(output) = &self.script_output {
            output.clone()
        } else {
            self.output.join("k6-owasp-security-test.js")
        };

        if let Some(parent) = script_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&script_path, &script)?;
        TerminalReporter::print_success(&format!("Script written to: {}", script_path.display()));

        // If generate-only mode, exit here
        if self.generate_only {
            println!("\nOWASP security test script generated. Run it with:");
            println!("  k6 run {}", script_path.display());
            return Ok(());
        }

        // Execute k6
        TerminalReporter::print_progress("Executing OWASP security tests...");
        let executor = K6Executor::new()?.with_local_ips(self.source_ips.join(","));
        std::fs::create_dir_all(&self.output)?;

        let results = executor.execute(&script_path, Some(&self.output), self.verbose).await?;

        let duration_secs = Self::parse_duration(&self.duration)?;
        TerminalReporter::print_summary_with_mode(&results, duration_secs, self.no_keep_alive);

        println!("\nOWASP security test results saved to: {}", self.output.display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_duration() {
        assert_eq!(BenchCommand::parse_duration("30s").unwrap(), 30);
        assert_eq!(BenchCommand::parse_duration("5m").unwrap(), 300);
        assert_eq!(BenchCommand::parse_duration("1h").unwrap(), 3600);
        assert_eq!(BenchCommand::parse_duration("60").unwrap(), 60);
    }

    /// Round 22.4 — `start-end` IPv4 range syntax for non-power-of-2
    /// ranges. Srikanth (h): `--source-ip 10.0.0.5-10.0.0.27` for 23
    /// hosts without finding a clean prefix.
    #[test]
    fn parse_ip_list_ipv4_range_inclusive() {
        let v = parse_ip_list(&["10.0.0.5-10.0.0.27".into()], "source-ip");
        assert_eq!(v.len(), 23);
        assert_eq!(v.first().unwrap().to_string(), "10.0.0.5");
        assert_eq!(v.last().unwrap().to_string(), "10.0.0.27");
    }

    /// Round 22.4 — range with start > end is rejected with a warning
    /// (returns nothing for that entry rather than wrapping around).
    #[test]
    fn parse_ip_list_range_rejects_backwards() {
        let v = parse_ip_list(&["10.0.0.10-10.0.0.5".into()], "source-ip");
        assert!(v.is_empty(), "backwards range should produce no IPs; got {v:?}");
    }

    /// Round 22.4 — IPv6 ranges are intentionally rejected because
    /// `2001:db8::1-2001:db8::5` would ambiguously parse against the
    /// address literal's `:` separators. Users use CIDR for IPv6.
    #[test]
    fn parse_ip_list_rejects_ipv6_range_syntax() {
        let v = parse_ip_list(&["2001:db8::1-2001:db8::5".into()], "geo-source-ip");
        assert!(v.is_empty(), "IPv6 range should be rejected; got {v:?}");
    }

    /// Round 22.4 — range cap is the same 256 host limit as CIDR.
    #[test]
    fn parse_ip_list_range_capped_at_256() {
        let v = parse_ip_list(&["10.0.0.0-10.0.5.0".into()], "source-ip");
        assert_eq!(v.len(), 256);
        assert_eq!(v.first().unwrap().to_string(), "10.0.0.0");
    }

    /// Round 19 — single IPs and comma-separated lists already
    /// worked in 18.5; this regression-locks the parse paths.
    #[test]
    fn parse_ip_list_plain_and_comma() {
        let v = parse_ip_list(&["10.0.0.5".into(), "10.0.0.6,10.0.0.7".into()], "source-ip");
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].to_string(), "10.0.0.5");
        assert_eq!(v[2].to_string(), "10.0.0.7");
    }

    /// Round 19 — IPv4 CIDR expands to host count up to the cap.
    /// `/29` = 8 hosts (well under cap), all 8 enumerated.
    #[test]
    fn parse_ip_list_ipv4_cidr_29_expands_to_8() {
        let v = parse_ip_list(&["10.0.0.0/29".into()], "source-ip");
        assert_eq!(v.len(), 8);
        assert_eq!(v[0].to_string(), "10.0.0.0");
        assert_eq!(v[7].to_string(), "10.0.0.7");
    }

    /// Round 19 — IPv4 CIDR larger than the cap is truncated, not
    /// rejected. Cap is 256; `/8` would be 16M without the guard.
    #[test]
    fn parse_ip_list_ipv4_cidr_8_capped_at_256() {
        let v = parse_ip_list(&["10.0.0.0/8".into()], "source-ip");
        assert_eq!(v.len(), 256);
        assert_eq!(v[0].to_string(), "10.0.0.0");
        assert_eq!(v[255].to_string(), "10.0.0.255");
    }

    /// Round 19 — IPv6 CIDR also expands. `/126` = 4 hosts.
    #[test]
    fn parse_ip_list_ipv6_cidr_126_expands_to_4() {
        let v = parse_ip_list(&["2001:db8::/126".into()], "geo-source-ip");
        assert_eq!(v.len(), 4);
        assert!(v[0].is_ipv6());
        assert_eq!(v[0].to_string(), "2001:db8::");
        assert_eq!(v[3].to_string(), "2001:db8::3");
    }

    /// Round 19 — mixed IPv4 + IPv6 + CIDR in one call works.
    #[test]
    fn parse_ip_list_mixed_v4_v6_cidr() {
        let v = parse_ip_list(&["10.0.0.0/30,2001:db8::1,203.0.113.42".into()], "geo-source-ip");
        assert_eq!(v.len(), 6); // 4 from /30 + 1 + 1
        assert!(v.iter().any(|ip| ip.to_string() == "2001:db8::1"));
        assert!(v.iter().any(|ip| ip.to_string() == "203.0.113.42"));
    }

    /// Round 19 — malformed entries log and skip; the run continues
    /// with whatever resolved.
    #[test]
    fn parse_ip_list_skips_malformed() {
        let v = parse_ip_list(
            &[
                "10.0.0.5".into(),
                "not-an-ip".into(),
                "10.0.0.6".into(),
                "/24".into(),
                "1.2.3.4/200".into(),
            ],
            "source-ip",
        );
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].to_string(), "10.0.0.5");
        assert_eq!(v[1].to_string(), "10.0.0.6");
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
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
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
            wafbench_cycle_all: false,
            owasp_api_top10: false,
            owasp_categories: None,
            owasp_auth_header: "Authorization".to_string(),
            owasp_auth_token: None,
            owasp_admin_paths: None,
            owasp_id_fields: None,
            owasp_report: None,
            owasp_report_format: "json".to_string(),
            owasp_iterations: 1,
            conformance: false,
            conformance_api_key: None,
            conformance_basic_auth: None,
            conformance_report: PathBuf::from("conformance-report.json"),
            conformance_categories: None,
            conformance_report_format: "json".to_string(),
            conformance_headers: vec![],
            conformance_all_operations: false,
            conformance_custom: None,
            conformance_delay_ms: 0,
            use_k6: false,
            conformance_custom_filter: None,
            export_requests: false,
            validate_requests: false,
            conformance_self_test: false,
            conformance_self_test_capture: false,
            source_ips: Vec::new(),
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
            report_missed_cap: None,
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
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
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
            wafbench_cycle_all: false,
            owasp_api_top10: false,
            owasp_categories: None,
            owasp_auth_header: "Authorization".to_string(),
            owasp_auth_token: None,
            owasp_admin_paths: None,
            owasp_id_fields: None,
            owasp_report: None,
            owasp_report_format: "json".to_string(),
            owasp_iterations: 1,
            conformance: false,
            conformance_api_key: None,
            conformance_basic_auth: None,
            conformance_report: PathBuf::from("conformance-report.json"),
            conformance_categories: None,
            conformance_report_format: "json".to_string(),
            conformance_headers: vec![],
            conformance_all_operations: false,
            conformance_custom: None,
            conformance_delay_ms: 0,
            use_k6: false,
            conformance_custom_filter: None,
            export_requests: false,
            validate_requests: false,
            conformance_self_test: false,
            conformance_self_test_capture: false,
            source_ips: Vec::new(),
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
            report_missed_cap: None,
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
            chunked_request_bodies: false,
            target_rps: None,
            no_keep_alive: false,
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
            wafbench_cycle_all: false,
            owasp_api_top10: false,
            owasp_categories: None,
            owasp_auth_header: "Authorization".to_string(),
            owasp_auth_token: None,
            owasp_admin_paths: None,
            owasp_id_fields: None,
            owasp_report: None,
            owasp_report_format: "json".to_string(),
            owasp_iterations: 1,
            conformance: false,
            conformance_api_key: None,
            conformance_basic_auth: None,
            conformance_report: PathBuf::from("conformance-report.json"),
            conformance_categories: None,
            conformance_report_format: "json".to_string(),
            conformance_headers: vec![],
            conformance_all_operations: false,
            conformance_custom: None,
            conformance_delay_ms: 0,
            use_k6: false,
            conformance_custom_filter: None,
            export_requests: false,
            validate_requests: false,
            conformance_self_test: false,
            conformance_self_test_capture: false,
            source_ips: Vec::new(),
            geo_source_ips: Vec::new(),
            geo_source_headers: Vec::new(),
            report_missed_cap: None,
        };

        assert_eq!(cmd_multi.get_spec_display_name(), "2 spec files");
    }

    #[test]
    fn test_parse_extracted_values_from_output_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("extracted_values.json");
        std::fs::write(
            &path,
            r#"{
  "pool_id": "abc123",
  "count": 0,
  "enabled": false,
  "metadata": { "owner": "team-a" }
}"#,
        )
        .unwrap();

        let extracted = BenchCommand::parse_extracted_values(dir.path()).unwrap();
        assert_eq!(extracted.get("pool_id"), Some(&serde_json::json!("abc123")));
        assert_eq!(extracted.get("count"), Some(&serde_json::json!(0)));
        assert_eq!(extracted.get("enabled"), Some(&serde_json::json!(false)));
        assert_eq!(extracted.get("metadata"), Some(&serde_json::json!({"owner": "team-a"})));
    }

    #[test]
    fn test_parse_extracted_values_missing_file() {
        let dir = tempdir().unwrap();
        let extracted = BenchCommand::parse_extracted_values(dir.path()).unwrap();
        assert!(extracted.values.is_empty());
    }
}
