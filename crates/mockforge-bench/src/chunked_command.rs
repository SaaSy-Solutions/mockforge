//! `mockforge bench-chunked` driver: single-target, spec-driven, and
//! multi-target (`--targets-file`) chunked-encoding runs.
//!
//! [`crate::chunked_bench::run`] drives one URL. This module layers the CLI
//! modes on top of it:
//!
//! - **Single target** (`--target URL`, no `--spec`): one run against the URL.
//! - **Spec-driven** (`--spec`): one run per POST/PUT/PATCH operation, with
//!   `--target` as the base URL.
//! - **Multi-target** (`--targets-file`, issue #79): every target in the file
//!   runs the single-target or spec-driven flow in parallel (bounded by
//!   `--max-concurrency`). It uses the same file format as
//!   `mockforge bench --targets-file`, including per-target `auth`, `headers`
//!   and `spec`. Per-target artifacts go to `<output>/target_<N>/`, and a
//!   roll-up goes to `<output>/chunked-multi-target-summary.json`.
//!
//! Any mode can loop as a campaign (`--rounds` / `--repeat-until`, issue
//! #79): each pass writes to `<output>/round_<N>/`, per-round stats go to
//! `<output>/campaign.jsonl` + `<output>/round-summaries/`, and
//! `--keep-rounds` prunes old `round_*` dirs, matching `mockforge bench`.

use crate::chunked_bench::{
    build_json_body, is_json_content_type, run, ChunkedBenchConfig, ChunkedBenchResult,
};
use crate::parallel_executor::ParallelExecutor;
use crate::request_gen::RequestGenerator;
use crate::spec_parser::SpecParser;
use crate::target_parser::{parse_targets_file, TargetConfig};
use anyhow::{bail, Context};
use futures::StreamExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Options shared by every `bench-chunked` mode.
#[derive(Debug, Clone)]
pub struct ChunkedCommand {
    /// Single target URL. Mutually exclusive with `targets_file`.
    pub target: Option<String>,
    /// File with multiple targets (same format as `bench --targets-file`).
    pub targets_file: Option<PathBuf>,
    /// Max targets running at once in multi-target mode.
    pub max_concurrency: u32,
    pub spec: Option<PathBuf>,
    pub base_path: Option<String>,
    pub operation_id: Option<String>,
    pub method: String,
    pub concurrency: u32,
    pub duration: Duration,
    pub chunk_size_bytes: usize,
    pub total_size_bytes: usize,
    /// Send raw `X` filler regardless of the declared Content-Type.
    pub raw_body: bool,
    pub chunk_interval_ms: u64,
    pub headers: HashMap<String, String>,
    pub insecure: bool,
    pub validate_requests: bool,
    pub export_requests: bool,
    pub output: PathBuf,
    /// Per-target cap on request starts per second (shared by its workers).
    pub rps: Option<u32>,
    /// New TCP/TLS connection per request, so connections/s = requests/s.
    pub cps: bool,
    /// Campaign: run the whole plan this many times.
    pub rounds: Option<u32>,
    /// Campaign: keep starting new rounds until this much wall clock elapsed.
    pub repeat_until: Option<Duration>,
    /// Campaign: keep only the newest N `round_*` dirs.
    pub keep_rounds: Option<u32>,
}

/// One target's resolved run plan.
#[derive(Debug, Clone)]
struct TargetRun {
    /// `""` in single-target mode, `"[target_N] "` in multi-target mode.
    prefix: String,
    url: String,
    headers: HashMap<String, String>,
    spec: Option<PathBuf>,
    output: PathBuf,
}

/// Roll-up of one target's run(s).
#[derive(Debug, Clone, Default)]
struct TargetOutcome {
    total_requests: u64,
    successful: u64,
    failed: u64,
    bytes_sent: u64,
    elapsed: Duration,
    status_counts: HashMap<u16, u64>,
    /// An operation errored out entirely (vs. individual requests failing).
    run_failed: bool,
}

impl TargetOutcome {
    fn absorb(&mut self, r: &ChunkedBenchResult) {
        self.total_requests += r.total_requests;
        self.successful += r.successful;
        self.failed += r.failed;
        self.bytes_sent += r.bytes_sent;
        self.elapsed += r.elapsed;
        for (code, n) in &r.status_counts {
            *self.status_counts.entry(*code).or_insert(0) += n;
        }
    }
}

impl ChunkedCommand {
    /// Run the bench. Returns `Ok(true)` when every run completed, `Ok(false)`
    /// when at least one operation or target failed to run.
    pub async fn execute(&self) -> anyhow::Result<bool> {
        match (&self.target, &self.targets_file) {
            (Some(_), Some(_)) => bail!("--target and --targets-file are mutually exclusive"),
            (None, None) => bail!("either --target or --targets-file is required"),
            _ => {}
        }
        if self.rounds == Some(0) {
            bail!("--rounds must be >= 1 (or omit it for a single pass / --repeat-until)");
        }
        println!(
            "Request-start limit per target: {}",
            self.rps
                .map(|n| format!("{n} req/s"))
                .unwrap_or_else(|| "none (--rps not set)".into())
        );
        println!(
            "Connections: {}",
            if self.cps {
                "new TCP/TLS connection per request (--cps)"
            } else {
                "pooled (--cps not set)"
            }
        );
        if self.raw_body {
            println!("Request bodies: raw X filler; declared Content-Type preserved (--raw-body)");
        }
        let max_rounds = self.rounds.unwrap_or(if self.repeat_until.is_some() {
            u32::MAX
        } else {
            1
        });
        let looping = max_rounds > 1 || self.repeat_until.is_some();
        if !looping {
            if self.keep_rounds.is_some() {
                eprintln!(
                    "Note: --keep-rounds only applies to campaign runs (--repeat-until / --rounds > 1); ignoring"
                );
            }
            return Ok(self.execute_pass(&self.output).await?.0);
        }

        match self.repeat_until {
            Some(until) => println!(
                "Campaign: re-running every pass until {}s of wall clock (or --rounds {})",
                until.as_secs(),
                self.rounds.map(|n| n.to_string()).unwrap_or_else(|| "unlimited".into())
            ),
            None => println!("Campaign: {max_rounds} round(s)"),
        }
        if let Some(keep) = self.keep_rounds {
            println!(
                "Round pruning: keeping newest {keep} round_* dir(s); per-round stats persist in campaign.jsonl + round-summaries/"
            );
        }

        let campaign_start = std::time::Instant::now();
        let mut all_ok = true;
        let mut round: u32 = 0;
        while round < max_rounds {
            if let Some(until) = self.repeat_until {
                if round > 0 && campaign_start.elapsed() >= until {
                    println!("Reached --repeat-until; stopping after {round} round(s)");
                    break;
                }
            }
            round += 1;
            let round_output = self.output.join(format!("round_{round}"));
            println!(
                "\n=== Round {round} (campaign elapsed {}s) → {} ===",
                campaign_start.elapsed().as_secs(),
                round_output.display()
            );
            let round_start = std::time::Instant::now();
            let (ok, summary) = self.execute_pass(&round_output).await?;
            all_ok &= ok;
            write_round_record(
                &self.output,
                round,
                campaign_start.elapsed(),
                round_start.elapsed(),
                ok,
                summary,
            )?;
            if let Some(keep) = self.keep_rounds {
                ParallelExecutor::prune_old_rounds(&self.output, keep);
            }
        }
        Ok(all_ok)
    }

    /// One pass over the whole plan, writing artifacts under `output`.
    /// Returns whether every run completed plus a JSON roll-up of the pass.
    async fn execute_pass(&self, output: &Path) -> anyhow::Result<(bool, serde_json::Value)> {
        match (&self.target, &self.targets_file) {
            (Some(target), _) => {
                let run = TargetRun {
                    prefix: String::new(),
                    url: target.clone(),
                    headers: self.headers.clone(),
                    spec: self.spec.clone(),
                    output: output.to_path_buf(),
                };
                let o = self.run_target(&run).await?;
                let summary = serde_json::json!({
                    "url": target,
                    "total_requests": o.total_requests,
                    "successful": o.successful,
                    "failed": o.failed,
                    "bytes_sent": o.bytes_sent,
                    "status_counts": o.status_counts,
                    "run_failed": o.run_failed,
                    "elapsed_seconds": o.elapsed.as_secs_f64(),
                    "measured_rps": measured_rps(o.total_requests, o.elapsed),
                    "rps_per_target": self.rps,
                    "rate_limit_enabled": self.rps.is_some(),
                    "new_connection_per_request": self.cps,
                    "raw_body": self.raw_body,
                });
                Ok((!o.run_failed, summary))
            }
            (None, Some(file)) => self.execute_multi_target(file, output).await,
            (None, None) => bail!("either --target or --targets-file is required"),
        }
    }

    async fn execute_multi_target(
        &self,
        file: &Path,
        output: &Path,
    ) -> anyhow::Result<(bool, serde_json::Value)> {
        let targets = parse_targets_file(file)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("failed to read targets file {}", file.display()))?;
        if targets.is_empty() {
            bail!("no targets found in {}", file.display());
        }
        let runs = plan_target_runs(targets, &self.headers, self.spec.as_ref(), output);
        let max_concurrency = self.max_concurrency.max(1) as usize;
        println!(
            "→ Chunked bench across {} targets from {:?} (max {} in parallel)",
            runs.len(),
            file,
            max_concurrency
        );

        let mut outcomes: Vec<(usize, TargetRun, anyhow::Result<TargetOutcome>)> =
            futures::stream::iter(runs.into_iter().enumerate())
                .map(|(i, run)| async move {
                    let res = self.run_target(&run).await;
                    (i, run, res)
                })
                .buffer_unordered(max_concurrency)
                .collect()
                .await;
        outcomes.sort_by_key(|(i, _, _)| *i);

        println!();
        println!("Multi-target chunked summary:");
        let mut all_ok = true;
        let mut rows = Vec::with_capacity(outcomes.len());
        for (i, run, res) in &outcomes {
            let name = format!("target_{}", i + 1);
            match res {
                Ok(o) => {
                    if o.run_failed {
                        all_ok = false;
                    }
                    println!(
                        "  {:<10} {}  requests={} ok={} failed={} bytes={}{}",
                        name,
                        run.url,
                        o.total_requests,
                        o.successful,
                        o.failed,
                        o.bytes_sent,
                        if o.run_failed {
                            "  (some operations failed to run)"
                        } else {
                            ""
                        }
                    );
                    rows.push(serde_json::json!({
                        "target": name,
                        "url": run.url,
                        "spec": run.spec.as_ref().map(|p| p.display().to_string()),
                        // Per-target files (exports / violations) exist only in spec mode.
                        "output_dir": run.spec.as_ref().map(|_| run.output.display().to_string()),
                        "total_requests": o.total_requests,
                        "successful": o.successful,
                        "failed": o.failed,
                        "bytes_sent": o.bytes_sent,
                        "status_counts": o.status_counts,
                        "run_failed": o.run_failed,
                        "elapsed_seconds": o.elapsed.as_secs_f64(),
                        "measured_rps": measured_rps(o.total_requests, o.elapsed),
                    }));
                }
                Err(e) => {
                    all_ok = false;
                    println!("  {:<10} {}  ERROR: {:#}", name, run.url, e);
                    rows.push(serde_json::json!({
                        "target": name,
                        "url": run.url,
                        "error": format!("{e:#}"),
                    }));
                }
            }
        }

        std::fs::create_dir_all(output)
            .with_context(|| format!("failed to create output directory {:?}", output))?;
        let path = output.join("chunked-multi-target-summary.json");
        let payload = serde_json::json!({
            "targets_file": file.display().to_string(),
            "concurrency_per_target": self.concurrency,
            "max_concurrency": max_concurrency,
            "duration_secs": self.duration.as_secs(),
            "chunk_size_bytes": self.chunk_size_bytes,
            "total_size_bytes": self.total_size_bytes,
            "chunk_interval_ms": self.chunk_interval_ms,
            "rps_per_target": self.rps,
            "rate_limit_enabled": self.rps.is_some(),
            "new_connection_per_request": self.cps,
            "raw_body": self.raw_body,
            "targets": rows,
        });
        std::fs::write(&path, serde_json::to_string_pretty(&payload)?)
            .with_context(|| format!("failed to write {:?}", path))?;
        println!("📝 Wrote {:?}", path);
        Ok((all_ok, payload))
    }

    fn bench_config(
        &self,
        url: String,
        method: reqwest::Method,
        headers: &HashMap<String, String>,
        json_seed: Option<&serde_json::Value>,
    ) -> ChunkedBenchConfig {
        // A JSON Content-Type gets a real JSON document padded to
        // --total-size-bytes; WAFs reject `X` filler as malformed JSON (#79).
        let body = (!self.raw_body && is_json_content_type(headers))
            .then(|| Arc::new(build_json_body(json_seed, self.total_size_bytes)));
        if let Some(b) = &body {
            if b.len() > self.total_size_bytes {
                eprintln!(
                    "Note: --total-size-bytes {} is below the smallest valid JSON body; sending {} bytes",
                    self.total_size_bytes,
                    b.len()
                );
            }
        }
        ChunkedBenchConfig {
            target_url: url,
            method,
            concurrency: self.concurrency,
            duration: self.duration,
            chunk_size_bytes: self.chunk_size_bytes,
            total_size_bytes: self.total_size_bytes,
            chunk_interval_ms: self.chunk_interval_ms,
            headers: headers.clone(),
            skip_tls_verify: self.insecure,
            rps: self.rps,
            no_keep_alive: self.cps,
            body,
        }
    }

    async fn run_target(&self, t: &TargetRun) -> anyhow::Result<TargetOutcome> {
        match &t.spec {
            Some(spec) => self.run_spec(t, spec).await,
            None => self.run_single(t).await,
        }
    }

    async fn run_single(&self, t: &TargetRun) -> anyhow::Result<TargetOutcome> {
        let p = &t.prefix;
        // Only warn once: in multi-target mode every target would repeat it.
        if p.is_empty() {
            if self.base_path.is_some() {
                eprintln!(
                    "Note: --base-path has no effect without --spec; \
                     single-target mode uses --target verbatim"
                );
            }
            if self.validate_requests {
                eprintln!("Note: --validate-requests requires --spec (skipped)");
            }
            if self.export_requests {
                eprintln!("Note: --export-requests requires --spec (skipped)");
            }
        }
        let method = reqwest::Method::from_bytes(self.method.to_uppercase().as_bytes())
            .map_err(|_| anyhow::anyhow!("Invalid HTTP method: {}", self.method))?;
        if !p.is_empty() {
            println!("{p}→ {} {}", self.method.to_uppercase(), t.url);
        }
        let r = run(self.bench_config(t.url.clone(), method, &t.headers, None))
            .await
            .context("chunked bench failed")?;
        print_result(
            p,
            if p.is_empty() {
                "single-target"
            } else {
                &t.url
            },
            &r,
        );
        let mut outcome = TargetOutcome::default();
        outcome.absorb(&r);
        Ok(outcome)
    }

    async fn run_spec(&self, t: &TargetRun, spec_path: &Path) -> anyhow::Result<TargetOutcome> {
        let p = &t.prefix;
        let parser = SpecParser::from_file(spec_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse spec {:?}: {}", spec_path, e))?;
        let base_url = t.url.trim_end_matches('/').to_string();
        // CLI --base-path > spec.servers > none. Empty string from CLI
        // explicitly disables (matches `mockforge bench` semantics).
        let effective_base_path: Option<String> = match &self.base_path {
            Some(bp) if bp.is_empty() => None,
            Some(bp) => Some(bp.clone()),
            None => parser.get_base_path(),
        };
        let want_methods = ["POST", "PUT", "PATCH"];
        let ops: Vec<_> = parser
            .get_operations()
            .into_iter()
            .filter(|op| want_methods.contains(&op.method.to_uppercase().as_str()))
            .filter(|op| match &self.operation_id {
                Some(id) => op.operation_id.as_deref() == Some(id.as_str()),
                None => true,
            })
            .collect();
        if ops.is_empty() {
            bail!(
                "No POST/PUT/PATCH operations matched in {:?}{}",
                spec_path,
                self.operation_id
                    .as_deref()
                    .map(|id| format!(" (operation-id={id})"))
                    .unwrap_or_default()
            );
        }

        // Pre-flight validation. Any failure here aborts before touching
        // the network so the bench doesn't half-run.
        let mut violations: Vec<serde_json::Value> = Vec::new();
        // (label, method, url, op.path, generated request body)
        let mut planned: Vec<(String, String, String, String, Option<serde_json::Value>)> =
            Vec::with_capacity(ops.len());
        for op in &ops {
            let label = op.display_name();
            match RequestGenerator::generate_template(op) {
                Ok(template) => {
                    let path_with_base = match &effective_base_path {
                        Some(bp) if !bp.is_empty() => {
                            format!("{}{}", bp, template.generate_path())
                        }
                        _ => template.generate_path().to_string(),
                    };
                    let url = format!("{}{}", base_url, path_with_base);
                    if reqwest::Method::from_bytes(op.method.to_uppercase().as_bytes()).is_err() {
                        violations.push(serde_json::json!({
                            "operation": label,
                            "method": op.method,
                            "path": op.path,
                            "kind": "invalid_method",
                            "detail": format!("`{}` is not a valid HTTP method", op.method),
                        }));
                    } else {
                        planned.push((
                            label,
                            op.method.to_uppercase(),
                            url,
                            op.path.clone(),
                            template.body.clone(),
                        ));
                    }
                }
                Err(e) => {
                    violations.push(serde_json::json!({
                        "operation": label,
                        "method": op.method,
                        "path": op.path,
                        "kind": "template_build_failure",
                        "detail": e.to_string(),
                    }));
                }
            }
        }

        if self.validate_requests {
            if !violations.is_empty() {
                std::fs::create_dir_all(&t.output)
                    .with_context(|| format!("Failed to create output directory {:?}", t.output))?;
                let path = t.output.join("chunked-request-violations.json");
                let payload = serde_json::json!({
                    "spec": spec_path.display().to_string(),
                    "base_url": base_url,
                    "base_path": effective_base_path,
                    "violations": violations,
                });
                std::fs::write(&path, serde_json::to_string_pretty(&payload)?)
                    .with_context(|| format!("Failed to write {:?}", path))?;
                bail!("--validate-requests: {} violation(s); see {:?}", violations.len(), path);
            }
            println!("{p}✅ --validate-requests: all {} operations OK", planned.len());
        }

        println!(
            "{p}→ {} chunked bench {} from {:?} (base {}{})",
            planned.len(),
            if planned.len() == 1 {
                "operation"
            } else {
                "operations"
            },
            spec_path,
            base_url,
            effective_base_path
                .as_deref()
                .map(|bp| format!(", base-path {}", bp))
                .unwrap_or_default()
        );

        let mut outcome = TargetOutcome::default();
        let mut export_records: Vec<serde_json::Value> = Vec::new();
        for (label, method_str, url, op_path, json_seed) in &planned {
            let method = match reqwest::Method::from_bytes(method_str.as_bytes()) {
                Ok(m) => m,
                Err(_) => {
                    eprintln!("{p}✗ {}: invalid method `{}`", label, method_str);
                    outcome.run_failed = true;
                    continue;
                }
            };
            println!();
            println!("{p}→ {} {}  ({})", method_str, op_path, url);
            let cfg = self.bench_config(url.clone(), method, &t.headers, json_seed.as_ref());
            match run(cfg).await {
                Ok(r) => {
                    print_result(p, label, &r);
                    outcome.absorb(&r);
                    if self.export_requests {
                        export_records.push(export_record(
                            self, label, method_str, url, op_path, &t.headers, &r,
                        ));
                    }
                }
                Err(e) => {
                    eprintln!("{p}✗ {}: chunked bench failed: {}", label, e);
                    outcome.run_failed = true;
                }
            }
        }

        if self.export_requests {
            std::fs::create_dir_all(&t.output)
                .with_context(|| format!("Failed to create output directory {:?}", t.output))?;
            let path = t.output.join("chunked-requests.json");
            let payload = serde_json::json!({
                "spec": spec_path.display().to_string(),
                "base_url": base_url,
                "base_path": effective_base_path,
                "operations": export_records,
            });
            std::fs::write(&path, serde_json::to_string_pretty(&payload)?)
                .with_context(|| format!("Failed to write {:?}", path))?;
            println!(
                "{p}📝 --export-requests: wrote {} operations to {:?}",
                export_records.len(),
                path
            );
        }

        Ok(outcome)
    }
}

/// Resolve each targets-file entry into a run plan: normalized URL, CLI
/// headers overlaid with the target's `headers` and `auth`, the target's
/// `spec` falling back to the CLI `--spec`, and `<output>/target_<N>/`.
fn plan_target_runs(
    targets: Vec<TargetConfig>,
    cli_headers: &HashMap<String, String>,
    cli_spec: Option<&PathBuf>,
    output: &Path,
) -> Vec<TargetRun> {
    targets
        .into_iter()
        .enumerate()
        .map(|(i, mut t)| {
            t.normalize_url();
            let mut headers = cli_headers.clone();
            if let Some(h) = &t.headers {
                headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            if let Some(auth) = &t.auth {
                headers.insert("Authorization".to_string(), auth.clone());
            }
            let name = format!("target_{}", i + 1);
            TargetRun {
                prefix: format!("[{name}] "),
                url: t.url,
                headers,
                spec: t.spec.or_else(|| cli_spec.cloned()),
                output: output.join(name),
            }
        })
        .collect()
}

/// Campaign bookkeeping for one finished round: a pruning-proof copy under
/// `<output>/round-summaries/` and one line appended to
/// `<output>/campaign.jsonl`.
fn write_round_record(
    output: &Path,
    round: u32,
    campaign_elapsed: Duration,
    round_elapsed: Duration,
    ok: bool,
    pass: serde_json::Value,
) -> anyhow::Result<()> {
    use std::io::Write;
    let record = serde_json::json!({
        "round": round,
        "campaign_elapsed_seconds": campaign_elapsed.as_secs(),
        "round_elapsed_seconds": round_elapsed.as_secs_f64(),
        "all_runs_completed": ok,
        "pass": pass,
    });
    let summaries = output.join("round-summaries");
    std::fs::create_dir_all(&summaries)
        .with_context(|| format!("failed to create {:?}", summaries))?;
    std::fs::write(
        summaries.join(format!("round_{round}.json")),
        serde_json::to_string_pretty(&record)?,
    )?;
    let mut line = serde_json::to_string(&record)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output.join("campaign.jsonl"))?
        .write_all(line.as_bytes())
        .context("failed to append campaign.jsonl")?;
    Ok(())
}

fn export_record(
    cmd: &ChunkedCommand,
    label: &str,
    method: &str,
    url: &str,
    op_path: &str,
    headers: &HashMap<String, String>,
    r: &ChunkedBenchResult,
) -> serde_json::Value {
    serde_json::json!({
        "operation": label,
        "method": method,
        "url": url,
        "spec_path": op_path,
        "headers": headers,
        "chunk_size_bytes": cmd.chunk_size_bytes,
        "total_size_bytes": cmd.total_size_bytes,
        "chunk_interval_ms": cmd.chunk_interval_ms,
        "concurrency": cmd.concurrency,
        "rps": cmd.rps,
        "rate_limit_enabled": cmd.rps.is_some(),
        "new_connection_per_request": cmd.cps,
        "raw_body": cmd.raw_body,
        "duration_secs": cmd.duration.as_secs(),
        "result": {
            "total_requests": r.total_requests,
            "successful": r.successful,
            "failed": r.failed,
            "bytes_sent": r.bytes_sent,
            "elapsed_ms": r.elapsed.as_millis(),
            "req_per_sec": r.req_per_sec,
            "avg_latency_ms": r.avg_latency_ms,
            "p50_ms": r.p50_ms,
            "p95_ms": r.p95_ms,
            "p99_ms": r.p99_ms,
            "status_counts": r.status_counts,
            "error_samples": r.error_samples.iter().map(|s| serde_json::json!({
                "status": s.status,
                "server_header": s.server_header,
                "body_excerpt": s.body_excerpt,
            })).collect::<Vec<_>>(),
        },
    })
}

/// Completed request attempts per second across a target's sequential operations.
fn measured_rps(requests: u64, elapsed: Duration) -> f64 {
    if elapsed.is_zero() {
        0.0
    } else {
        requests as f64 / elapsed.as_secs_f64()
    }
}

/// Print one result block. The block is built first and printed with one
/// call so parallel targets don't interleave their lines.
fn print_result(prefix: &str, label: &str, r: &ChunkedBenchResult) {
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(prefix);
        out.push_str(&s);
        out.push('\n');
    };
    line(format!("Chunked bench [{}] complete in {:?}", label, r.elapsed));
    line(format!("  Total requests: {}", r.total_requests));
    line(format!("  Successful:     {}", r.successful));
    line(format!("  Failed:         {}", r.failed));
    line(format!("  Bytes sent:     {}", r.bytes_sent));
    line(format!("  Throughput:     {:.2} req/s", r.req_per_sec));
    line(format!(
        "  Latency:        avg={:.1}ms p50={}ms p95={}ms p99={}ms",
        r.avg_latency_ms, r.p50_ms, r.p95_ms, r.p99_ms
    ));
    if !r.status_counts.is_empty() {
        line("  Status codes:".to_string());
        let mut codes: Vec<_> = r.status_counts.iter().collect();
        codes.sort_by_key(|(k, _)| **k);
        for (code, n) in codes {
            line(format!("    {} = {}", code, n));
        }
    }
    // Surface the captured error responses so the user can tell whether
    // errors came from MockForge, an upstream proxy, a CDN, etc. Hint at the
    // most common cause when 5xx is involved (proxy upstream timeout on long
    // chunked uploads).
    if !r.error_samples.is_empty() {
        line("  Error response samples:".to_string());
        for s in &r.error_samples {
            let server = s.server_header.as_deref().unwrap_or("(no Server header)");
            line(format!("    [{}] Server: {}", s.status, server));
            if !s.body_excerpt.is_empty() {
                line(format!("       body: {}", s.body_excerpt.replace('\n', " ")));
            }
        }
        if r.status_counts.keys().any(|c| (500..600).contains(c)) {
            line(
                "  Hint: 5xx responses with `Server:` revealing a proxy/LB usually \
                 mean the proxy timed out reading from upstream. Each chunked request \
                 takes >= (total_size_bytes / chunk_size_bytes) * chunk_interval_ms; \
                 if that exceeds the proxy's upstream timeout, errors are inevitable."
                    .to_string(),
            );
        }
    }
    print!("{out}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_target_runs_merges_headers_auth_spec_and_output() {
        let mut target_headers = HashMap::new();
        target_headers.insert("X-Env".to_string(), "b".to_string());
        let targets = vec![
            TargetConfig::from_url("10.0.0.1:8080".to_string()),
            TargetConfig {
                url: "https://b.example".to_string(),
                auth: Some("Bearer t".to_string()),
                headers: Some(target_headers),
                spec: Some(PathBuf::from("b.yaml")),
            },
        ];
        let mut cli_headers = HashMap::new();
        cli_headers.insert("X-Env".to_string(), "cli".to_string());
        let cli_spec = PathBuf::from("cli.yaml");

        let runs = plan_target_runs(targets, &cli_headers, Some(&cli_spec), Path::new("out"));

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].url, "http://10.0.0.1:8080");
        assert_eq!(runs[0].headers.get("X-Env").map(String::as_str), Some("cli"));
        assert_eq!(runs[0].spec.as_deref(), Some(Path::new("cli.yaml")));
        assert_eq!(runs[0].output, Path::new("out").join("target_1"));
        assert_eq!(runs[0].prefix, "[target_1] ");

        assert_eq!(runs[1].headers.get("X-Env").map(String::as_str), Some("b"));
        assert_eq!(runs[1].headers.get("Authorization").map(String::as_str), Some("Bearer t"));
        assert_eq!(runs[1].spec.as_deref(), Some(Path::new("b.yaml")));
        assert_eq!(runs[1].output, Path::new("out").join("target_2"));
    }

    fn cmd(target: Option<&str>, targets_file: Option<PathBuf>) -> ChunkedCommand {
        ChunkedCommand {
            target: target.map(str::to_string),
            targets_file,
            max_concurrency: 10,
            spec: None,
            base_path: None,
            operation_id: None,
            method: "POST".to_string(),
            concurrency: 1,
            duration: Duration::from_millis(10),
            chunk_size_bytes: 1024,
            total_size_bytes: 4096,
            raw_body: false,
            chunk_interval_ms: 0,
            headers: HashMap::new(),
            insecure: false,
            validate_requests: false,
            export_requests: false,
            output: PathBuf::from("bench-results"),
            rps: None,
            cps: false,
            rounds: None,
            repeat_until: None,
            keep_rounds: None,
        }
    }

    #[tokio::test]
    async fn rejects_target_and_targets_file_together() {
        let c = cmd(Some("http://127.0.0.1:1"), Some(PathBuf::from("t.txt")));
        assert!(c.execute().await.is_err());
    }

    #[tokio::test]
    async fn rejects_neither_target_nor_targets_file() {
        assert!(cmd(None, None).execute().await.is_err());
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mf-chunked-{name}-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn campaign_rounds_write_records_and_prune_round_dirs() {
        let mut server = mockito::Server::new_async().await;
        let _m = server.mock("POST", "/upload").with_status(200).create_async().await;
        let dir = scratch_dir("campaign");
        let targets = dir.join("targets.txt");
        std::fs::write(&targets, format!("{}/upload\n{}/upload\n", server.url(), server.url()))
            .unwrap();
        let mut c = cmd(None, Some(targets));
        c.output = dir.join("out");
        c.duration = Duration::from_millis(50);
        c.rounds = Some(3);
        c.keep_rounds = Some(1);

        assert!(c.execute().await.unwrap());

        let jsonl = std::fs::read_to_string(c.output.join("campaign.jsonl")).unwrap();
        let rounds: Vec<serde_json::Value> =
            jsonl.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(rounds.len(), 3);
        assert_eq!(rounds[2]["round"], 3);
        assert_eq!(rounds[2]["pass"]["targets"].as_array().unwrap().len(), 2);
        for n in 1..=3 {
            assert!(c.output.join(format!("round-summaries/round_{n}.json")).exists());
        }
        assert!(!c.output.join("round_1").exists());
        assert!(!c.output.join("round_2").exists());
        assert!(c.output.join("round_3/chunked-multi-target-summary.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn campaign_repeat_until_stops_on_wall_clock() {
        let mut server = mockito::Server::new_async().await;
        let _m = server.mock("POST", "/upload").with_status(200).create_async().await;
        let dir = scratch_dir("until");
        let mut c = cmd(Some(&format!("{}/upload", server.url())), None);
        c.output = dir.clone();
        // Short rounds against a roomy budget: a loaded CI runner can stretch
        // one round's setup well past its nominal duration, so assert only that
        // the campaign repeated and then stopped, not an exact count.
        c.duration = Duration::from_millis(50);
        c.repeat_until = Some(Duration::from_millis(500));

        assert!(c.execute().await.unwrap());

        let n = std::fs::read_to_string(dir.join("campaign.jsonl")).unwrap().lines().count();
        assert!((2..=11).contains(&n), "expected 2-11 rounds of 50ms in 500ms, got {n}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn raw_body_preserves_json_headers_in_spec_campaign_and_exports() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/upload")
            .match_header("content-type", "application/json")
            .match_header("transfer-encoding", "chunked")
            .match_body(mockito::Matcher::Exact("X".repeat(4096)))
            .with_status(400)
            .expect_at_least(2)
            .create_async()
            .await;
        let dir = tempfile::tempdir().unwrap();
        let spec = dir.path().join("api.json");
        std::fs::write(
            &spec,
            serde_json::json!({
                "openapi": "3.0.3", "info": {"title": "test", "version": "1"},
                "paths": {"/upload": {"post": {
                    "operationId": "upload",
                    "requestBody": {"content": {"application/json": {"schema": {
                        "type": "object", "properties": {"name": {"type": "string"}}
                    }}}},
                    "responses": {"200": {"description": "ok"}}
                }}}
            })
            .to_string(),
        )
        .unwrap();
        let targets = dir.path().join("targets.json");
        std::fs::write(
            &targets,
            serde_json::json!([
                {"url": server.url(), "headers": {"Content-Type": "application/json"}}
            ])
            .to_string(),
        )
        .unwrap();
        let mut c = cmd(None, Some(targets));
        c.spec = Some(spec);
        c.raw_body = true;
        c.output = dir.path().join("out");
        c.rounds = Some(2);
        c.rps = Some(10);
        c.cps = true;
        c.export_requests = true;
        assert!(c.execute().await.unwrap());
        m.assert_async().await;
        let record: serde_json::Value = serde_json::from_slice(
            &std::fs::read(c.output.join("round-summaries/round_2.json")).unwrap(),
        )
        .unwrap();
        assert!(record["round_elapsed_seconds"].as_f64().unwrap() > 0.0);
        let pass = &record["pass"];
        assert_eq!(pass["raw_body"], true);
        assert_eq!(pass["rps_per_target"], 10);
        assert_eq!(pass["rate_limit_enabled"], true);
        assert_eq!(pass["new_connection_per_request"], true);
        let row = &pass["targets"][0];
        let requests = row["total_requests"].as_u64().unwrap();
        let elapsed = row["elapsed_seconds"].as_f64().unwrap();
        assert!(requests > 0);
        assert_eq!(row["measured_rps"].as_f64().unwrap(), requests as f64 / elapsed);
        assert_eq!(row["status_counts"]["400"], requests);
        let exported: serde_json::Value = serde_json::from_slice(
            &std::fs::read(c.output.join("round_2/target_1/chunked-requests.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(exported["operations"][0]["raw_body"], true);
        assert_eq!(exported["operations"][0]["headers"]["Content-Type"], "application/json");
    }

    #[tokio::test]
    async fn automatic_json_and_raw_body_work_without_spec() {
        let mut server = mockito::Server::new_async().await;
        let json = server
            .mock("POST", "/json")
            .match_header("content-type", "application/vnd.test+json")
            .match_header("transfer-encoding", "chunked")
            .match_body(mockito::Matcher::PartialJson(
                serde_json::json!({"_padding": "X".repeat(4081)}),
            ))
            .with_status(200)
            .expect_at_least(1)
            .create_async()
            .await;
        let raw = server
            .mock("POST", "/raw")
            .match_header("content-type", "application/vnd.test+json")
            .match_header("transfer-encoding", "chunked")
            .match_body(mockito::Matcher::Exact("X".repeat(4096)))
            .with_status(400)
            .expect_at_least(1)
            .create_async()
            .await;
        let mut c = cmd(Some(&format!("{}/json", server.url())), None);
        c.headers.insert("Content-Type".into(), "application/vnd.test+json".into());
        let (_, summary) = c.execute_pass(Path::new("unused")).await.unwrap();
        assert!(summary["total_requests"].as_u64().unwrap() > 0);
        assert_eq!(summary["rps_per_target"], serde_json::Value::Null);
        assert_eq!(summary["rate_limit_enabled"], false);
        assert_eq!(summary["new_connection_per_request"], false);
        assert!(summary["measured_rps"].as_f64().unwrap() > 0.0);
        json.assert_async().await;
        c.target = Some(format!("{}/raw", server.url()));
        c.raw_body = true;
        let (_, summary) = c.execute_pass(Path::new("unused")).await.unwrap();
        assert!(summary["status_counts"]["400"].as_u64().unwrap() > 0);
        raw.assert_async().await;
    }

    #[test]
    fn round_records_distinguish_round_time_from_campaign_time() {
        let dir = tempfile::tempdir().unwrap();
        write_round_record(
            dir.path(),
            2,
            Duration::from_secs(30),
            Duration::from_millis(12500),
            true,
            serde_json::json!({}),
        )
        .unwrap();
        let record: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("round-summaries/round_2.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(record["campaign_elapsed_seconds"], 30);
        assert_eq!(record["round_elapsed_seconds"], 12.5);
        let jsonl: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("campaign.jsonl")).unwrap(),
        )
        .unwrap();
        assert_eq!(jsonl, record);
    }

    #[tokio::test]
    async fn rejects_zero_rounds() {
        let mut c = cmd(Some("http://127.0.0.1:1"), None);
        c.rounds = Some(0);
        assert!(c.execute().await.is_err());
    }
}
