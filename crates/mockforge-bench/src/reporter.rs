//! Result reporting and formatting

use crate::executor::K6Results;
use crate::parallel_executor::AggregatedResults;
use colored::*;

/// Terminal reporter for bench results
pub struct TerminalReporter;

impl TerminalReporter {
    /// Print a summary of the bench results.
    ///
    /// `cps_mode` is `true` when the bench was invoked with `--cps`. In that
    /// mode each request opens a fresh TCP/TLS connection, so we print an
    /// explicit "Connection Rate" line alongside RPS — Srikanth's round-5
    /// reply on Issue #79: "CPS without RPS Command is Working but Client
    /// dont report CPS Counts".
    pub fn print_summary(results: &K6Results, duration_secs: u64) {
        Self::print_summary_with_mode(results, duration_secs, false);
    }

    /// Like [`print_summary`] but lets the caller opt into the `--cps` view.
    ///
    /// Issue #79 round 6 — the connection-count lines now render unconditionally
    /// whenever k6 reported `http_req_connecting` samples (i.e. it actually
    /// opened TCP sockets), so non-`--cps` runs also surface client-side
    /// connection counts. Without `--cps` k6 reuses sockets, so the count
    /// equals "distinct connections opened", which is what Srikanth wanted
    /// alongside RPS.
    pub fn print_summary_with_mode(results: &K6Results, duration_secs: u64, cps_mode: bool) {
        Self::print_summary_full(results, duration_secs, cps_mode, None);
    }

    /// Like [`print_summary_with_mode`] but accepts `num_operations` (the count
    /// of operations the bench generated from the spec). When supplied, the
    /// summary surfaces iteration coverage: how many iterations completed and
    /// what fraction of the spec's operations got exercised end-to-end.
    ///
    /// Issue #79 round 10 — Srikanth's 11422-op spec ran for 600s with only
    /// `--vus 5`; many iterations were cancelled mid-way and the final
    /// summary didn't reveal which slice of the spec was actually covered.
    pub fn print_summary_full(
        results: &K6Results,
        duration_secs: u64,
        cps_mode: bool,
        num_operations: Option<u32>,
    ) {
        println!("\n{}", "=".repeat(60).bright_green());
        println!("{}", "Load Test Complete! ✓".bright_green().bold());
        println!("{}\n", "=".repeat(60).bright_green());

        println!("{}", "Summary:".bold());
        println!("  Total Requests:       {}", results.total_requests.to_string().cyan());
        println!(
            "  Successful:           {} ({}%)",
            (results.total_requests - results.failed_requests).to_string().green(),
            format!("{:.2}", results.success_rate()).green()
        );
        println!(
            "  Failed:               {} ({}%)",
            results.failed_requests.to_string().red(),
            format!("{:.2}", results.error_rate()).red()
        );

        println!("\n{}", "Response Times:".bold());
        println!("  Min:                  {}ms", format!("{:.2}", results.min_duration_ms).cyan());
        println!("  Avg:                  {}ms", format!("{:.2}", results.avg_duration_ms).cyan());
        println!("  Med:                  {}ms", format!("{:.2}", results.med_duration_ms).cyan());
        println!("  p90:                  {}ms", format!("{:.2}", results.p90_duration_ms).cyan());
        println!("  p95:                  {}ms", format!("{:.2}", results.p95_duration_ms).cyan());
        println!("  p99:                  {}ms", format!("{:.2}", results.p99_duration_ms).cyan());
        println!("  Max:                  {}ms", format!("{:.2}", results.max_duration_ms).cyan());

        println!("\n{}", "Throughput:".bold());
        if results.rps > 0.0 {
            println!("  RPS:                  {} req/s", format!("{:.1}", results.rps).cyan());
        } else {
            println!(
                "  RPS:                  {} req/s",
                format!("{:.1}", results.total_requests as f64 / duration_secs as f64).cyan()
            );
        }
        if results.vus_max > 0 {
            println!("  Max VUs:              {}", results.vus_max.to_string().cyan());
        }

        // Issue #79 (round 5) — Connections-per-second report. When the user
        // passed `--cps`, k6 ran with `noConnectionReuse: true` so each
        // request opened a new TCP/TLS connection. CPS therefore equals the
        // request rate; show it explicitly, plus connect/handshake timing.
        if cps_mode {
            let cps = if results.rps > 0.0 {
                results.rps
            } else {
                results.total_requests as f64 / duration_secs.max(1) as f64
            };
            println!("  CPS:                  {} conn/s (--cps)", format!("{:.1}", cps).cyan());
            println!("  Total Connections:    {}", results.total_requests.to_string().cyan());
        }

        // Issue #79 round 6 — Always surface client-side connection counts
        // when k6 actually opened sockets. Helpful even without `--cps`
        // because it lets you compare "k6 opened N connections" against
        // the server's `connections_total_opened` and detect whether
        // your proxy is keeping the upstream pool warm.
        //
        // Round 6 follow-up: `tcp_connect_samples` is now sourced from the
        // template's `mockforge_connections_opened` Counter (incremented when
        // `res.timings.connecting > 0`). That gives an accurate count for
        // both `--cps` (≈ total requests) and the pooled-reuse case (≈ vus_max)
        // — Srikanth's "Open/Closed Connection Counter not showing for RPS"
        // report on Issue #79.
        if results.tcp_connect_samples > 0 && !cps_mode {
            // Surface the count in non-CPS mode too — Srikanth's "open
            // connection on the client" ask.
            println!(
                "  Connections opened:   {} ({} conn/s avg)",
                results.tcp_connect_samples.to_string().cyan(),
                format!("{:.1}", results.tcp_connect_samples as f64 / duration_secs.max(1) as f64)
                    .cyan(),
            );

            // Issue #79 round 8 — Srikanth saw 7425 connections opened with
            // --vus 5, expecting ~5 (one per VU under pooled reuse). When
            // `tcp_connect_samples` is much larger than `vus_max`, the target
            // is closing the socket between requests (proxy upstream pool
            // disabled, server `Connection: close`, etc). Without --cps,
            // ratios > 5× indicate connection reuse isn't happening.
            if results.vus_max > 0 {
                let reuse_ratio = results.tcp_connect_samples as f64 / results.vus_max as f64;
                if reuse_ratio > 5.0 {
                    println!(
                        "  {}: {:.0}× more sockets opened than concurrent VUs — \
                         the target is closing connections (proxy pool disabled, \
                         `Connection: close`, or short upstream idle timeout).",
                        "Connection reuse NOT detected".yellow().bold(),
                        reuse_ratio,
                    );
                }
            }
        }
        // Print TCP/TLS timing whenever the avg is non-zero. Don't gate on
        // samples count — k6's Trend metric exposes avg/max in summary.json
        // but not count, so the count check was always false even when k6
        // had real samples. Issue #79 round 6 follow-up.
        if results.tcp_connect_avg_ms > 0.0 || results.tcp_connect_max_ms > 0.0 {
            println!(
                "  TCP connect:          avg {:.2}ms, max {:.2}ms",
                results.tcp_connect_avg_ms, results.tcp_connect_max_ms,
            );
        }
        if results.tls_handshake_avg_ms > 0.0 || results.tls_handshake_max_ms > 0.0 {
            println!(
                "  TLS handshake:        avg {:.2}ms, max {:.2}ms",
                results.tls_handshake_avg_ms, results.tls_handshake_max_ms,
            );
        }
        // Peak concurrent VUs — the upper bound on simultaneously-open
        // connections from the client. For HTTP/1.1 each VU holds at
        // most one socket; for HTTP/2 with multiplexing this is the
        // bound on streams, not sockets. Surface it as an open-connection
        // ceiling so users can sanity-check against the server's
        // `connections_open` gauge.
        if results.vus_max > 0
            && (cps_mode || results.tcp_connect_samples > 0 || results.tcp_connect_avg_ms > 0.0)
        {
            println!(
                "  Peak concurrent VUs:  {} (max open conns from client side)",
                results.vus_max.to_string().cyan(),
            );
        }

        // Issue #79 round 10 — iteration coverage. When --rps is supplied and
        // the spec has many operations per iteration, k6 may cancel iterations
        // mid-walk if the duration ends before a full pass completes. Surface
        // "iterations completed" alongside operation count so users see what
        // fraction of the spec was actually exercised.
        if results.iterations_completed > 0 {
            if let Some(num_ops) = num_operations {
                let expected_reqs_per_iter = num_ops as u64;
                let full_iter_reqs =
                    results.iterations_completed.saturating_mul(expected_reqs_per_iter);
                let partial_iter_reqs = results.total_requests.saturating_sub(full_iter_reqs);
                println!(
                    "  Iterations:           {} complete × {} ops = {} ops fully exercised",
                    results.iterations_completed.to_string().cyan(),
                    num_ops.to_string().cyan(),
                    full_iter_reqs.to_string().cyan(),
                );
                if partial_iter_reqs > 0 && num_ops > 1 {
                    println!(
                        "                        {} extra request(s) from a partially-completed \
                         iteration — duration ended mid-walk; not every op was hit on the last pass.",
                        partial_iter_reqs.to_string().yellow(),
                    );
                }
            } else {
                println!(
                    "  Iterations:           {} complete",
                    results.iterations_completed.to_string().cyan(),
                );
            }
        }

        // Issue #79 — server-injected chaos signals (latency / jitter / faults)
        // observed from MockForge response headers. Surfaces the slice of
        // total wire time that came from the chaos middleware vs the system
        // under test.
        if results.server_injected_latency_samples > 0
            || results.server_injected_jitter_samples > 0
            || results.server_reported_faults > 0
        {
            println!("\n{}", "Server-Injected (chaos):".bold());
            if results.server_injected_latency_samples > 0 {
                println!(
                    "  Latency samples:      {} (avg {:.2}ms, max {:.2}ms)",
                    results.server_injected_latency_samples.to_string().cyan(),
                    results.server_injected_latency_avg_ms,
                    results.server_injected_latency_max_ms,
                );
            }
            if results.server_injected_jitter_samples > 0 {
                println!(
                    "  Jitter samples:       {} (avg {:.2}ms)",
                    results.server_injected_jitter_samples.to_string().cyan(),
                    results.server_injected_jitter_avg_ms,
                );
            }
            if results.server_reported_faults > 0 {
                println!(
                    "  Fault-marked resps:   {}",
                    results.server_reported_faults.to_string().cyan(),
                );
            }
        }

        println!("\n{}", "=".repeat(60).bright_green());
    }

    /// Print header information
    pub fn print_header(
        spec_file: &str,
        target: &str,
        num_operations: usize,
        scenario: &str,
        duration_secs: u64,
    ) {
        println!("\n{}\n", "MockForge Bench - Load Testing Mode".bright_green().bold());
        println!("{}", "─".repeat(60).bright_black());

        println!("{}: {}", "Specification".bold(), spec_file.cyan());
        println!("{}: {}", "Target".bold(), target.cyan());
        println!("{}: {} endpoints", "Operations".bold(), num_operations.to_string().cyan());
        println!("{}: {}", "Scenario".bold(), scenario.cyan());
        println!("{}: {}s", "Duration".bold(), duration_secs.to_string().cyan());

        println!("{}\n", "─".repeat(60).bright_black());
    }

    /// Print progress message
    pub fn print_progress(message: &str) {
        println!("{} {}", "→".bright_green().bold(), message);
    }

    /// Print error message
    pub fn print_error(message: &str) {
        eprintln!("{} {}", "✗".bright_red().bold(), message.red());
    }

    /// Print success message
    pub fn print_success(message: &str) {
        println!("{} {}", "✓".bright_green().bold(), message.green());
    }

    /// Print warning message
    pub fn print_warning(message: &str) {
        println!("{} {}", "⚠".bright_yellow().bold(), message.yellow());
    }

    /// Print multi-target summary
    pub fn print_multi_target_summary(results: &AggregatedResults) {
        println!("\n{}", "=".repeat(60).bright_green());
        println!("{}", "Multi-Target Load Test Complete! ✓".bright_green().bold());
        println!("{}\n", "=".repeat(60).bright_green());

        println!("{}", "Overall Summary:".bold());
        println!("  Total Targets:        {}", results.total_targets.to_string().cyan());
        println!(
            "  Successful:           {} ({}%)",
            results.successful_targets.to_string().green(),
            format!(
                "{:.1}",
                (results.successful_targets as f64 / results.total_targets as f64) * 100.0
            )
            .green()
        );
        println!(
            "  Failed:               {} ({}%)",
            results.failed_targets.to_string().red(),
            format!(
                "{:.1}",
                (results.failed_targets as f64 / results.total_targets as f64) * 100.0
            )
            .red()
        );

        println!("\n{}", "Aggregated Metrics:".bold());
        println!(
            "  Total Requests:       {}",
            results.aggregated_metrics.total_requests.to_string().cyan()
        );
        println!(
            "  Failed Requests:      {} ({}%)",
            results.aggregated_metrics.total_failed_requests.to_string().red(),
            format!("{:.2}", results.aggregated_metrics.error_rate).red()
        );
        println!(
            "  Total RPS:            {} req/s",
            format!("{:.1}", results.aggregated_metrics.total_rps).cyan()
        );
        println!(
            "  Avg RPS/target:       {} req/s",
            format!("{:.1}", results.aggregated_metrics.avg_rps).cyan()
        );
        println!(
            "  Total VUs:            {}",
            results.aggregated_metrics.total_vus_max.to_string().cyan()
        );
        println!(
            "  Avg Response Time:    {}ms",
            format!("{:.2}", results.aggregated_metrics.avg_duration_ms).cyan()
        );
        println!(
            "  p95 Response Time:    {}ms",
            format!("{:.2}", results.aggregated_metrics.p95_duration_ms).cyan()
        );
        println!(
            "  p99 Response Time:    {}ms",
            format!("{:.2}", results.aggregated_metrics.p99_duration_ms).cyan()
        );

        // Issue #79 round 12 — multi-target was missing the connection /
        // iteration counters that single-target runs surface. Aggregate
        // across targets and print only when k6 actually opened sockets
        // / completed iterations on at least one target.
        if results.aggregated_metrics.total_connections_opened > 0 {
            println!(
                "  Total Connections:    {}",
                results.aggregated_metrics.total_connections_opened.to_string().cyan()
            );
            if results.aggregated_metrics.total_vus_max > 0 {
                let reuse_ratio = results.aggregated_metrics.total_connections_opened as f64
                    / results.aggregated_metrics.total_vus_max as f64;
                if reuse_ratio > 5.0 {
                    println!(
                        "  {}: {:.0}× more sockets opened than concurrent VUs across all targets — \
                         at least one target is closing connections (proxy pool disabled, \
                         `Connection: close`, or short upstream idle timeout).",
                        "Connection reuse NOT detected".yellow().bold(),
                        reuse_ratio,
                    );
                }
            }
        }
        if results.aggregated_metrics.total_iterations_completed > 0 {
            println!(
                "  Total Iterations:     {} complete (sum across all targets)",
                results.aggregated_metrics.total_iterations_completed.to_string().cyan()
            );
        }

        // Show per-target summary
        let print_target = |result: &crate::parallel_executor::TargetResult| {
            let status = if result.success {
                "✓".bright_green()
            } else {
                "✗".bright_red()
            };
            println!("  {} {}", status, result.target_url.cyan());
            if result.success {
                println!(
                    "      Requests: {}  RPS: {}  VUs: {}",
                    result.results.total_requests.to_string().white(),
                    format!("{:.1}", result.results.rps).white(),
                    result.results.vus_max.to_string().white(),
                );
                println!(
                    "      Latency: min={:.1}ms avg={:.1}ms med={:.1}ms p90={:.1}ms p95={:.1}ms p99={:.1}ms max={:.1}ms",
                    result.results.min_duration_ms,
                    result.results.avg_duration_ms,
                    result.results.med_duration_ms,
                    result.results.p90_duration_ms,
                    result.results.p95_duration_ms,
                    result.results.p99_duration_ms,
                    result.results.max_duration_ms,
                );
                // Issue #79 round 12 — per-target connection/iteration counts
                // (missing previously in multi-target output).
                if result.results.tcp_connect_samples > 0 || result.results.iterations_completed > 0
                {
                    println!(
                        "      Connections: {}  Iterations: {}",
                        result.results.tcp_connect_samples.to_string().white(),
                        result.results.iterations_completed.to_string().white(),
                    );
                }
            }
            if let Some(error) = &result.error {
                println!("      Error: {}", error.red());
            }
        };

        if results.total_targets <= 20 {
            println!("\n{}", "Per-Target Results:".bold());
            for result in &results.target_results {
                print_target(result);
            }
        } else {
            // Show top 10 and bottom 10
            println!("\n{}", "Top 10 Targets (by requests):".bold());
            let mut sorted_results = results.target_results.clone();
            sorted_results.sort_by_key(|r| r.results.total_requests);
            sorted_results.reverse();

            for result in sorted_results.iter().take(10) {
                print_target(result);
            }

            println!("\n{}", "Bottom 10 Targets:".bold());
            for result in sorted_results.iter().rev().take(10) {
                print_target(result);
            }
        }

        println!("\n{}", "=".repeat(60).bright_green());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_reporter_creation() {
        let _reporter = TerminalReporter;
    }
}
