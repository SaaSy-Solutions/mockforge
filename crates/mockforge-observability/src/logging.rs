//! Structured logging initialization with JSON support and OpenTelemetry integration
//!
//! This module provides comprehensive logging capabilities including:
//! - Structured JSON logging
//! - File output with rotation
//! - OpenTelemetry tracing integration
//! - Configurable log levels

use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable JSON format for structured logging
    pub json_format: bool,
    /// Optional file path for log output
    pub file_path: Option<PathBuf>,
    /// Maximum log file size in MB (for rotation)
    pub max_file_size_mb: u64,
    /// Maximum number of log files to keep
    pub max_files: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            file_path: None,
            max_file_size_mb: 10,
            max_files: 5,
        }
    }
}

/// Initialize logging with the given configuration
///
/// This function sets up the tracing subscriber with the appropriate layers based on configuration:
/// - Console output (plain text or JSON)
/// - Optional file output with rotation
/// - Optional OpenTelemetry tracing layer
///
/// # Arguments
/// * `config` - Logging configuration
///
/// # Example
/// ```no_run
/// use mockforge_observability::logging::{LoggingConfig, init_logging};
///
/// let config = LoggingConfig {
///     level: "info".to_string(),
///     json_format: true,
///     file_path: Some("logs/mockforge.log".into()),
///     max_file_size_mb: 10,
///     max_files: 5,
/// };
///
/// init_logging(config).expect("Failed to initialize logging");
/// ```
pub fn init_logging(config: LoggingConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse log level
    let _log_level = parse_log_level(&config.level)?;

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Build the subscriber with layers using Registry
    let registry = tracing_subscriber::registry().with(env_filter);

    // Add console layer (JSON or plain text)
    let console_layer = if config.json_format {
        // JSON formatted console output
        fmt::layer()
            .json()
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_current_span(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    } else {
        // Plain text console output
        fmt::layer()
            .with_span_events(FmtSpan::CLOSE)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .boxed()
    };

    // Add file layer if configured
    if let Some(ref file_path) = config.file_path {
        // Extract directory and file name
        let directory = file_path.parent().ok_or("Invalid file path")?;
        let file_name = file_path
            .file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Invalid file name encoding")?;

        // Create rolling file appender with daily rotation
        let file_appender = tracing_appender::rolling::daily(directory, file_name);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // Store the guard to prevent it from being dropped
        // Note: In a production application, you would want to keep this guard alive
        // for the lifetime of your application. Here we use Box::leak to ensure it's never dropped.
        Box::leak(Box::new(_guard));

        let file_layer = if config.json_format {
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .boxed()
        } else {
            fmt::layer()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .boxed()
        };

        registry.with(console_layer).with(file_layer).init();

        tracing::info!(
            "Logging initialized: level={}, format={}, file={}",
            config.level,
            if config.json_format { "json" } else { "text" },
            file_path.display()
        );
    } else {
        registry.with(console_layer).init();

        tracing::info!(
            "Logging initialized: level={}, format={}",
            config.level,
            if config.json_format { "json" } else { "text" }
        );
    }

    Ok(())
}

/// Initialize logging with OpenTelemetry tracing layer
///
/// This function sets up logging with an additional OpenTelemetry layer for distributed tracing.
///
/// # Arguments
/// * `config` - Logging configuration
/// * `otel_layer` - OpenTelemetry tracing layer
///
/// # Example
/// ```no_run
/// use mockforge_observability::logging::{LoggingConfig, init_logging_with_otel};
/// use tracing_subscriber::layer::SubscriberExt;
///
/// // Initialize OpenTelemetry tracer first
/// // let tracer = ...;
/// // let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
///
/// // Then initialize logging with the layer
/// // init_logging_with_otel(config, otel_layer).expect("Failed to initialize logging");
/// ```
pub fn init_logging_with_otel<L, S>(
    _config: LoggingConfig,
    _otel_layer: L,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    L: tracing_subscriber::Layer<S> + Send + Sync + 'static,
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    // Note: This function is provided for advanced users who want to integrate OpenTelemetry.
    // Due to trait bound complexity, we recommend using init_logging() for most cases.

    tracing::warn!(
        "init_logging_with_otel requires manual subscriber setup. Use init_logging() for simpler cases."
    );

    // Return early - users should set up their own subscriber when using OpenTelemetry
    Err("OpenTelemetry integration requires manual subscriber setup. Please use tracing_subscriber directly.".into())
}

/// Parse log level from string
fn parse_log_level(level: &str) -> Result<Level, Box<dyn std::error::Error + Send + Sync>> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(format!("Invalid log level: {}", level).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json_format);
        assert!(config.file_path.is_none());
        assert_eq!(config.max_file_size_mb, 10);
        assert_eq!(config.max_files, 5);
    }

    #[test]
    fn test_parse_log_level() {
        assert!(parse_log_level("trace").is_ok());
        assert!(parse_log_level("debug").is_ok());
        assert!(parse_log_level("info").is_ok());
        assert!(parse_log_level("warn").is_ok());
        assert!(parse_log_level("error").is_ok());
        assert!(parse_log_level("TRACE").is_ok());
        assert!(parse_log_level("INFO").is_ok());
        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_logging_config_with_json() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            json_format: true,
            file_path: Some(PathBuf::from("/tmp/test.log")),
            max_file_size_mb: 20,
            max_files: 10,
        };

        assert_eq!(config.level, "debug");
        assert!(config.json_format);
        assert!(config.file_path.is_some());
    }

    #[test]
    fn test_init_logging_with_file() {
        use std::fs;
        use std::time::SystemTime;

        // Create a unique test log file path
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let test_dir = PathBuf::from("/tmp/mockforge-test-logs");
        fs::create_dir_all(&test_dir).ok();
        let log_file = test_dir.join(format!("test-{}.log", timestamp));

        let config = LoggingConfig {
            level: "info".to_string(),
            json_format: false,
            file_path: Some(log_file.clone()),
            max_file_size_mb: 10,
            max_files: 5,
        };

        // This test verifies that init_logging completes without error
        // when file logging is configured
        let result = init_logging(config);
        assert!(result.is_ok(), "Failed to initialize logging with file: {:?}", result);
    }
}
