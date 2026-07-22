//! k6 execution and output handling

use crate::error::{BenchError, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

/// Extract a `MOCKFORGE_<KIND>:` JSON payload from a k6 output line.
///
/// k6 emits these via `console.log`, which goes through one of two paths
/// depending on the runner config:
/// - **Raw**: `MOCKFORGE_EXCHANGE:{"check":"...", ...}` straight to stdout.
/// - **Logfmt**: `time="..." level=info msg="MOCKFORGE_EXCHANGE:{...}" source=console`
///   where the JSON's `"` are escaped as `\"` and `\` as `\\` so it fits
///   inside the `msg="..."` field.
///
/// Round 46 (#79) — Srikanth on 0.3.190: a multipart upload landed `[]`
/// in `conformance-requests.json` even though `MOCKFORGE_EXCHANGE:` was
/// present in the k6 log. Root cause: the previous parser used a naive
/// `replace("\\\\", "\\").replace("\\\"", "\"")` chain. With binary
/// multipart bytes the JSON content includes sequences like `\\"`
/// (literal backslash followed by literal quote inside a JSON string),
/// which logfmt-escapes to `\\\\\"`. The replace chain processed `\\`
/// → `\` first, leaving `\\\"`, then `\"` → `"`, mangling the JSON.
/// Replaced with a single character walk that consumes one logfmt
/// escape at a time. Also rewrote the suffix-strip to scan for the
/// matching closing `"` of the `msg="..."` field instead of a
/// fixed-string suffix so we tolerate any trailing logfmt fields k6
/// might add.
fn extract_mockforge_marker_json(line: &str, marker: &str) -> Option<String> {
    let start = line.find(marker)?;
    let json_start = start + marker.len();
    let rest = &line[json_start..];

    // Is this the logfmt-wrapped form? The `msg="` opener sits 5 bytes
    // before the marker. (Plain `msg=MOCKFORGE_...` would also be valid
    // logfmt for a value with no spaces, but k6 always quote-wraps.)
    let is_logfmt = start >= 5 && line.as_bytes().get(start - 5..start) == Some(b"msg=\"");
    if is_logfmt {
        // Walk forward until the unescaped closing `"` of msg=. Inside
        // the field, `\\` is one escaped backslash and `\"` is one
        // escaped quote — those bytes belong to the JSON content. Any
        // unescaped `"` is the field terminator.
        let bytes = rest.as_bytes();
        let mut i = 0;
        let mut out = String::with_capacity(rest.len());
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'"' {
                // End of msg= field.
                return Some(out);
            }
            if b == b'\\' && i + 1 < bytes.len() {
                let next = bytes[i + 1];
                match next {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    // Other escapes (`\n`, `\r`, `\t`, `\uXXXX`) are
                    // PART of the JSON content — keep them verbatim so
                    // serde_json::from_str interprets them.
                    other => {
                        out.push('\\');
                        out.push(other as char);
                    }
                }
                i += 2;
                continue;
            }
            // Non-ASCII multi-byte UTF-8 codepoint or plain ASCII char.
            // `rest` is a `&str` so we can rely on UTF-8 boundaries.
            let ch_start = i;
            // Advance i past the codepoint.
            i += 1;
            while i < bytes.len() && (bytes[i] & 0b1100_0000) == 0b1000_0000 {
                i += 1;
            }
            out.push_str(&rest[ch_start..i]);
        }
        // Reached EOL without a closing quote — return what we have so
        // the downstream parser can decide whether to keep it.
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    } else {
        // Raw form: rest of the line is the JSON, possibly with trailing
        // whitespace. No escape processing needed.
        let trimmed = rest.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

/// Extract `MOCKFORGE_EXCHANGE:` JSON payload from a k6 output line (--export-requests).
fn extract_exchange_json(line: &str) -> Option<String> {
    extract_mockforge_marker_json(line, "MOCKFORGE_EXCHANGE:")
}

/// Extract `MOCKFORGE_FAILURE:` JSON payload from a k6 output line.
fn extract_failure_json(line: &str) -> Option<String> {
    extract_mockforge_marker_json(line, "MOCKFORGE_FAILURE:")
}

/// Round 47 (#79) — extract `MOCKFORGE_NETWORK_EVENT:` JSON payload.
/// Emitted by the k6 captureExchange when `res.status === 0`, capturing
/// the wire-level failure with a classified `kind`.
fn extract_network_event_json(line: &str) -> Option<String> {
    extract_mockforge_marker_json(line, "MOCKFORGE_NETWORK_EVENT:")
}

/// k6 executor
pub struct K6Executor {
    k6_path: String,
    /// Comma-joined IPs/ranges/CIDRs forwarded to `k6 run --local-ips`.
    /// Empty → flag omitted. Populated by callers that pass through the
    /// CLI's `--source-ip`; lets a VU make requests from one of several
    /// bound interfaces (k6 supports this natively, contrary to my
    /// round-22 warning).
    local_ips: String,
    /// Round 56 (#79) — when true, set `K6_DISCARD_RESPONSE_BODIES=true` so k6
    /// does not buffer every response body in memory. On long, high-concurrency
    /// multi-target runs (Srikanth on 0.3.203 saw `k6 ... signal: 9 (SIGKILL)`,
    /// i.e. the OOM killer) the buffered bodies plus k6's own metric
    /// accumulation exhaust RAM. Plain load only checks status codes, so
    /// dropping the bodies is safe there.
    discard_response_bodies: bool,
    /// Round 61 (#79) — value for `k6 run --dns "policy=<...>"`. Empty → flag
    /// omitted (k6 default `preferIPv4`). Srikanth on 0.3.208 GEODB-tests a WAF
    /// via hostnames (his proxy routes by Host/SNI, so he can't pass bracket
    /// IPs), but needs those hostnames resolved to their AAAA/IPv6 record;
    /// k6/Go default to IPv4, which then can't be dialed from his IPv6
    /// `--local-ips` source ("no suitable address found"). `preferIPv6` /
    /// `onlyIPv6` fix that while keeping the hostname on the wire.
    dns_policy: String,
}

impl K6Executor {
    /// Create a new k6 executor
    pub fn new() -> Result<Self> {
        let k6_path = which::which("k6")
            .map_err(|_| BenchError::K6NotFound)?
            .to_string_lossy()
            .to_string();

        Ok(Self {
            k6_path,
            local_ips: String::new(),
            discard_response_bodies: false,
            dns_policy: String::new(),
        })
    }

    /// Set the `--local-ips` value for subsequent k6 invocations.
    /// Accepts a comma-joined list of IPs, ranges (`10.0.0.1-10.0.0.5`),
    /// and/or CIDRs (`192.168.0.0/24`) - same syntax k6 expects.
    pub fn with_local_ips(mut self, local_ips: impl Into<String>) -> Self {
        self.local_ips = local_ips.into();
        self
    }

    /// Round 56 (#79) — enable `K6_DISCARD_RESPONSE_BODIES` so k6 does not hold
    /// response bodies in memory. Use for plain load runs (status-only checks);
    /// do NOT use where the script inspects/extracts response bodies.
    pub fn with_discard_response_bodies(mut self, discard: bool) -> Self {
        self.discard_response_bodies = discard;
        self
    }

    /// Round 61 (#79) — set the `--dns` resolution policy (e.g. `preferIPv6`,
    /// `onlyIPv6`, `preferIPv4`, `onlyIPv4`, `any`). Empty string → flag omitted
    /// (k6 default). Passed to k6 as `--dns "policy=<value>"`.
    pub fn with_dns_policy(mut self, policy: impl Into<String>) -> Self {
        self.dns_policy = policy.into();
        self
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
    /// Pass `Some(0)` to bind an OS-assigned ephemeral port — the collision-proof
    /// choice for parallel runs, since the kernel never hands out a busy port
    /// (see the k6 `CannotStartRESTAPI` / exit-106 fix in `parallel_executor`).
    /// Pass `None` for single-target runs (uses k6's default `localhost:6565`).
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

        // `--local-ips` rotates each VU through a pool of source IPs that
        // must already be bound on the host (CIDRs/ranges accepted). This
        // gives the k6 path the same source-IP coverage as the native
        // self-test driver's `--source-ip`.
        if !self.local_ips.is_empty() {
            cmd.arg("--local-ips").arg(&self.local_ips);
        }

        // Round 56 (#79) — drop response bodies to bound k6's memory on long,
        // high-concurrency runs (guards against the SIGKILL/OOM Srikanth hit).
        if self.discard_response_bodies {
            cmd.env("K6_DISCARD_RESPONSE_BODIES", "true");
        }

        // Round 61 (#79) — force a DNS resolution policy so hostname targets can
        // be pinned to IPv6 (or IPv4). Needed for GEODB IPv6 tests where the
        // proxy routes by Host/SNI (so the target must stay a hostname) but the
        // dial has to use the AAAA record to match an IPv6 `--local-ips` source.
        if !self.dns_policy.is_empty() {
            cmd.arg("--dns").arg(format!("policy={}", self.dns_policy));
        }

        // summary.json is written by the k6 script's handleSummary() function
        // (relative to CWD, set to output_dir below). We no longer use
        // --summary-export as it's deprecated in newer k6 versions and
        // conflicts with handleSummary when both try to write the same file.

        // Add verbosity
        if verbose {
            cmd.arg("--verbose");
        }

        // Use absolute path for the script so it's found regardless of CWD.
        let abs_script =
            std::fs::canonicalize(script_path).unwrap_or_else(|_| script_path.to_path_buf());
        cmd.arg(&abs_script);

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

        // Collect request/response exchanges for --export-requests
        let exchange_details: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let ex_stdout = Arc::clone(&exchange_details);
        let ex_stderr = Arc::clone(&exchange_details);

        // Round 47 (#79) — collect wire-level network events the
        // k6 script emits on status=0 (connect / tls / timeout). Same
        // shape as the native + self-test sinks so we can write a
        // unified `conformance-network-events.json`.
        let network_events: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let ne_stdout = Arc::clone(&network_events);
        let ne_stderr = Arc::clone(&network_events);

        // Collect all k6 output for saving to a log file
        let log_lines: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let log_stdout = Arc::clone(&log_lines);
        let log_stderr = Arc::clone(&log_lines);

        // Read stdout lines, capturing MOCKFORGE_FAILURE / MOCKFORGE_EXCHANGE / MOCKFORGE_NETWORK_EVENT markers
        let stdout_handle = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_lines.next_line().await {
                log_stdout.lock().await.push(format!("[stdout] {}", line));
                if let Some(json_str) = extract_failure_json(&line) {
                    fd_stdout.lock().await.push(json_str);
                } else if let Some(json_str) = extract_exchange_json(&line) {
                    ex_stdout.lock().await.push(json_str);
                } else if let Some(json_str) = extract_network_event_json(&line) {
                    ne_stdout.lock().await.push(json_str);
                } else {
                    spinner.set_message(line.clone());
                    if !line.is_empty() && !line.contains("running") && !line.contains("default") {
                        println!("{}", line);
                    }
                }
            }
            spinner.finish_and_clear();
        });

        // Read stderr lines, capturing MOCKFORGE_FAILURE / MOCKFORGE_EXCHANGE / MOCKFORGE_NETWORK_EVENT markers
        let stderr_handle = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                if !line.is_empty() {
                    log_stderr.lock().await.push(format!("[stderr] {}", line));
                    if let Some(json_str) = extract_failure_json(&line) {
                        fd_stderr.lock().await.push(json_str);
                    } else if let Some(json_str) = extract_exchange_json(&line) {
                        ex_stderr.lock().await.push(json_str);
                    } else if let Some(json_str) = extract_network_event_json(&line) {
                        ne_stderr.lock().await.push(json_str);
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

            // Write exchange details (--export-requests) if any were captured
            let exchanges = exchange_details.lock().await;
            if !exchanges.is_empty() {
                let exchange_path = dir.join("conformance-requests.json");
                let parsed: Vec<serde_json::Value> =
                    exchanges.iter().filter_map(|s| serde_json::from_str(s).ok()).collect();
                if let Ok(json) = serde_json::to_string_pretty(&parsed) {
                    let _ = std::fs::write(&exchange_path, json);
                    tracing::info!(
                        "Exported {} request/response pairs to {}",
                        parsed.len(),
                        exchange_path.display()
                    );
                }
            }

            // Round 47 (#79) — write the wire-level events sink. We
            // ALWAYS write the file (empty array when nothing failed)
            // so a caller can tell "everything succeeded" from "nobody
            // looked" at a glance.
            let net_events = network_events.lock().await;
            let net_path = dir.join("conformance-network-events.json");
            let parsed: Vec<serde_json::Value> =
                net_events.iter().filter_map(|s| serde_json::from_str(s).ok()).collect();
            if let Ok(json) = serde_json::to_string_pretty(&parsed) {
                let _ = std::fs::write(&net_path, json);
                if !parsed.is_empty() {
                    tracing::warn!(
                        "Recorded {} wire-level network event(s) to {}",
                        parsed.len(),
                        net_path.display()
                    );
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

        let server_latency = &json["metrics"]["mockforge_server_injected_latency_ms"]["values"];
        let server_jitter = &json["metrics"]["mockforge_server_injected_jitter_ms"]["values"];
        let server_fault = &json["metrics"]["mockforge_server_fault_total"]["values"]["count"];

        // Issue #79 (round 5) — surface TCP connect / TLS handshake stats and
        // a connection-rate count for `--cps` runs.
        //
        // Round 6 follow-up: k6's `http_req_connecting` Trend doesn't expose a
        // `count` field in summary.json (only avg/min/med/max/p90/p95), so we
        // can't use it for "connections opened". The template now feeds a
        // dedicated Counter, `mockforge_connections_opened`, every time a
        // request's `res.timings.connecting > 0`. That gives us an accurate
        // count for both `--cps` (≈ total_requests) and pooled-reuse (≈ vus_max)
        // runs. The Trend is still useful for the avg/max timing display.
        let tcp_connecting = &json["metrics"]["http_req_connecting"]["values"];
        let tls_handshake = &json["metrics"]["http_req_tls_handshaking"]["values"];
        let mf_conns_opened = &json["metrics"]["mockforge_connections_opened"]["values"]["count"];

        Ok(K6Results {
            total_requests: json["metrics"]["http_reqs"]["values"]["count"].as_u64().unwrap_or(0),
            // k6 Rate metric: `passes` = count of non-zero values.
            // For http_req_failed, non-zero means the request failed.
            // So `passes` = failed request count, `fails` = successful request count.
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
            server_injected_latency_samples: server_latency["count"].as_u64().unwrap_or(0),
            server_injected_latency_avg_ms: server_latency["avg"].as_f64().unwrap_or(0.0),
            server_injected_latency_max_ms: server_latency["max"].as_f64().unwrap_or(0.0),
            server_injected_jitter_samples: server_jitter["count"].as_u64().unwrap_or(0),
            server_injected_jitter_avg_ms: server_jitter["avg"].as_f64().unwrap_or(0.0),
            server_reported_faults: server_fault.as_u64().unwrap_or(0),
            // Counter from the template, not the Trend's count (which is
            // absent in k6 summary JSON).
            tcp_connect_samples: mf_conns_opened.as_u64().unwrap_or(0),
            tcp_connect_avg_ms: tcp_connecting["avg"].as_f64().unwrap_or(0.0),
            tcp_connect_max_ms: tcp_connecting["max"].as_f64().unwrap_or(0.0),
            // TLS handshake Trend has no `count` either; gate display on avg>0.
            tls_handshake_samples: if tls_handshake["avg"].as_f64().unwrap_or(0.0) > 0.0 {
                // Use connection count as a proxy — every new TLS session
                // requires a handshake.
                mf_conns_opened.as_u64().unwrap_or(0)
            } else {
                0
            },
            tls_handshake_avg_ms: tls_handshake["avg"].as_f64().unwrap_or(0.0),
            tls_handshake_max_ms: tls_handshake["max"].as_f64().unwrap_or(0.0),
            iterations_completed: json["metrics"]["iterations"]["values"]["count"]
                .as_u64()
                .unwrap_or(0),
        })
    }
}

impl Default for K6Executor {
    fn default() -> Self {
        Self::new().expect("k6 not found")
    }
}

/// k6 test results
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    /// Issue #79 — client-side visibility into MockForge-injected latency,
    /// parsed from the `X-Mockforge-Injected-Latency-Ms` response header that
    /// the chaos middleware sets. Zero when chaos isn't firing or the target
    /// isn't MockForge.
    #[serde(default)]
    pub server_injected_latency_samples: u64,
    #[serde(default)]
    pub server_injected_latency_avg_ms: f64,
    #[serde(default)]
    pub server_injected_latency_max_ms: f64,
    #[serde(default)]
    pub server_injected_jitter_samples: u64,
    #[serde(default)]
    pub server_injected_jitter_avg_ms: f64,
    /// Count of responses that carried an `X-Mockforge-Fault` header.
    #[serde(default)]
    pub server_reported_faults: u64,
    /// Issue #79 (round 5) — TCP connect samples / timing. With `--cps`
    /// (`noConnectionReuse: true`) k6 records one connect per request, so
    /// `tcp_connect_samples` equals connections opened. Without `--cps` this
    /// is typically a small count (k6 reuses pooled connections), so it tells
    /// you whether reuse was actually happening.
    #[serde(default)]
    pub tcp_connect_samples: u64,
    #[serde(default)]
    pub tcp_connect_avg_ms: f64,
    #[serde(default)]
    pub tcp_connect_max_ms: f64,
    /// TLS handshake samples / timing — same shape as TCP connect, but only
    /// non-zero for HTTPS targets.
    #[serde(default)]
    pub tls_handshake_samples: u64,
    #[serde(default)]
    pub tls_handshake_avg_ms: f64,
    #[serde(default)]
    pub tls_handshake_max_ms: f64,
    /// Issue #79 round 10 — k6 iteration counter from `iterations.values.count`.
    /// For `constant-arrival-rate` (`--rps`), this is the number of full
    /// iterations completed within the duration. When `iterations × num_ops`
    /// is much less than `total_requests`, mid-iteration cancellation truncated
    /// the run and not every operation in the spec was exercised.
    #[serde(default)]
    pub iterations_completed: u64,
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
    fn discard_response_bodies_defaults_off_and_builder_flips_it() {
        // Round 56 (#79) — guards the OOM fix for multi-target load runs.
        // Default must stay off so body-inspecting paths (extract/conformance)
        // are unaffected; the builder opts a run in.
        let exec = K6Executor {
            k6_path: "k6".to_string(),
            local_ips: String::new(),
            discard_response_bodies: false,
            dns_policy: String::new(),
        };
        assert!(!exec.discard_response_bodies);
        let exec = exec.with_discard_response_bodies(true);
        assert!(exec.discard_response_bodies);
    }

    #[test]
    fn dns_policy_defaults_empty_and_builder_sets_it() {
        // Round 61 (#79) — empty default → k6's `--dns` flag is omitted; the
        // builder records the policy string the executor turns into
        // `--dns "policy=<value>"`.
        let exec = K6Executor {
            k6_path: "k6".to_string(),
            local_ips: String::new(),
            discard_response_bodies: false,
            dns_policy: String::new(),
        };
        assert!(exec.dns_policy.is_empty());
        let exec = exec.with_dns_policy("preferIPv6");
        assert_eq!(exec.dns_policy, "preferIPv6");
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

    /// Round 46 (#79) — regression: Srikanth's multipart upload landed
    /// `[]` in `conformance-requests.json` because the old
    /// `replace("\\\\","\\").replace("\\\"","\"")` chain misparsed
    /// adjacent backslashes inside the JSON body (binary multipart
    /// bytes encoded as `\\u00XX` etc.). Pin both shapes here.
    #[test]
    fn test_extract_exchange_logfmt_with_backslash_escapes() {
        // A JSON body that contains a JSON-encoded `` (one escape
        // sequence the validator survives). Logfmt wraps it: each `\`
        // becomes `\\`, each `"` becomes `\"`.
        let line = r#"time="2026-06-26T10:00:00Z" level=info msg="MOCKFORGE_EXCHANGE:{\"check\":\"u\",\"request\":{\"body\":\"--bnd\\r\\n\\u001a\"}}" source=console"#;
        let result = extract_exchange_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["check"], "u");
        // The unescape preserves the JSON's `\r\n` and `` so the
        // downstream consumer can interpret them as JSON escapes.
        assert_eq!(parsed["request"]["body"], "--bnd\r\n\u{001a}");
    }

    #[test]
    fn test_extract_exchange_raw_no_logfmt_wrapping() {
        let line =
            r#"MOCKFORGE_EXCHANGE:{"check":"x","request":{"body":""},"response":{"status":200}}"#;
        let result = extract_exchange_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["check"], "x");
        assert_eq!(parsed["response"]["status"], 200);
    }

    /// The end of `msg="..."` is a single unescaped `"`, not the old
    /// fixed-string `" source=console`. If k6 ever appends another
    /// logfmt field (or omits source=), we still get the JSON out.
    #[test]
    fn test_extract_exchange_logfmt_tolerates_extra_trailing_fields() {
        let line = r#"msg="MOCKFORGE_EXCHANGE:{\"check\":\"t\"}" source=console vu=1 iter=0"#;
        let result = extract_exchange_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["check"], "t");
    }

    /// Round 46 — JSON-encoded backslash inside a JSON string (`\\u00XX`
    /// in the JSON, `\\\\u00XX` in logfmt) must round-trip cleanly.
    /// The naive `.replace` chain choked on this exact pattern.
    #[test]
    fn test_extract_exchange_double_backslash_followed_by_quote() {
        // JSON content: `\\"x"` is `\` then `"x"`. Logfmt:
        // `\\\\\"x\"` (4 backslashes + escaped quote + x + escaped quote).
        let line = r#"msg="MOCKFORGE_EXCHANGE:{\"k\":\"a\\\\\\\"x\\\"\"}" source=console"#;
        let result = extract_exchange_json(line).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["k"], r#"a\"x""#);
    }
}
