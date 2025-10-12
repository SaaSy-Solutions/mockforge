//! System metrics collection
//!
//! Provides background tasks for collecting system-level metrics including:
//! - Memory usage
//! - CPU usage
//! - Thread count
//! - Server uptime

use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, error, warn};

use crate::prometheus::MetricsRegistry;

/// Start system metrics collection background task
///
/// This task periodically collects system metrics and updates the Prometheus registry.
/// The collection interval is configurable (default: 15 seconds).
///
/// # Arguments
/// * `registry` - The metrics registry to update
/// * `collection_interval` - How often to collect metrics (default: 15 seconds)
///
/// # Example
/// ```no_run
/// use mockforge_observability::{get_global_registry, system_metrics::start_system_metrics_collector};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = get_global_registry();
///     start_system_metrics_collector(registry, Duration::from_secs(15));
/// }
/// ```
pub fn start_system_metrics_collector(
    registry: &'static MetricsRegistry,
    collection_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    let start_time = Instant::now();

    tokio::spawn(async move {
        let mut ticker = interval(collection_interval);
        debug!("System metrics collector started (interval: {:?})", collection_interval);

        loop {
            ticker.tick().await;

            // Collect and update metrics
            if let Err(e) = collect_and_update_metrics(registry, start_time).await {
                error!("Failed to collect system metrics: {}", e);
            }
        }
    })
}

/// Collect and update all system metrics
async fn collect_and_update_metrics(
    registry: &MetricsRegistry,
    start_time: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    // Update uptime
    let uptime_seconds = start_time.elapsed().as_secs_f64();
    registry.update_uptime(uptime_seconds);

    // Collect system information using sysinfo
    #[cfg(feature = "sysinfo")]
    {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        // Memory usage
        let memory_used = sys.used_memory() as f64;
        registry.update_memory_usage(memory_used);

        // CPU usage (global)
        let cpu_usage = sys.global_cpu_usage() as f64;
        registry.update_cpu_usage(cpu_usage);

        debug!(
            "System metrics updated - Memory: {:.2} MB, CPU: {:.2}%, Uptime: {:.2}s",
            memory_used / 1024.0 / 1024.0,
            cpu_usage,
            uptime_seconds
        );
    }

    // Collect thread count
    #[cfg(target_os = "linux")]
    {
        if let Ok(thread_count) = get_thread_count_linux() {
            registry.update_thread_count(thread_count as f64);
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // For non-Linux systems, use a simple estimation based on available parallelism
        if let Ok(parallelism) = std::thread::available_parallelism() {
            // This is an approximation - actual thread count may vary
            registry.update_thread_count(parallelism.get() as f64);
        }
    }

    Ok(())
}

/// Get thread count on Linux by reading /proc/self/status
#[cfg(target_os = "linux")]
fn get_thread_count_linux() -> Result<usize, std::io::Error> {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status")?;
    for line in status.lines() {
        if line.starts_with("Threads:") {
            if let Some(count_str) = line.split_whitespace().nth(1) {
                if let Ok(count) = count_str.parse::<usize>() {
                    return Ok(count);
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Thread count not found in /proc/self/status",
    ))
}

/// Configuration for system metrics collection
#[derive(Debug, Clone)]
pub struct SystemMetricsConfig {
    /// Enable system metrics collection
    pub enabled: bool,
    /// Collection interval in seconds
    pub interval_seconds: u64,
}

impl Default for SystemMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 15,
        }
    }
}

/// Start system metrics collector with configuration
pub fn start_with_config(
    registry: &'static MetricsRegistry,
    config: SystemMetricsConfig,
) -> Option<tokio::task::JoinHandle<()>> {
    if config.enabled {
        debug!("Starting system metrics collector with {:?}", config);
        Some(start_system_metrics_collector(
            registry,
            Duration::from_secs(config.interval_seconds),
        ))
    } else {
        warn!("System metrics collection is disabled");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::MetricsRegistry;

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let registry = MetricsRegistry::new();
        let start_time = Instant::now();

        // Run collection once
        let result = collect_and_update_metrics(&registry, start_time).await;
        assert!(result.is_ok());

        // Verify metrics were updated
        assert!(registry.uptime_seconds.get() > 0.0);
    }

    #[test]
    fn test_system_metrics_config_default() {
        let config = SystemMetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_seconds, 15);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_get_thread_count_linux() {
        let result = get_thread_count_linux();
        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count > 0);
    }
}
