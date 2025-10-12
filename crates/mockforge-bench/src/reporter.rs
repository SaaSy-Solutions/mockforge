//! Result reporting and formatting

use crate::executor::K6Results;
use colored::*;

/// Terminal reporter for bench results
pub struct TerminalReporter;

impl TerminalReporter {
    /// Print a summary of the bench results
    pub fn print_summary(results: &K6Results, duration_secs: u64) {
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
        println!("  Avg:                  {}ms", format!("{:.2}", results.avg_duration_ms).cyan());
        println!("  p95:                  {}ms", format!("{:.2}", results.p95_duration_ms).cyan());
        println!("  p99:                  {}ms", format!("{:.2}", results.p99_duration_ms).cyan());

        println!(
            "\n  Throughput:           {} req/s",
            format!("{:.1}", results.total_requests as f64 / duration_secs as f64).cyan()
        );

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_reporter_creation() {
        let _reporter = TerminalReporter;
    }
}
