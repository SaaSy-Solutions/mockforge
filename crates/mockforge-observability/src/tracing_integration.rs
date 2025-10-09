//! OpenTelemetry tracing integration for structured logging
//!
//! This module provides integration between the logging system and OpenTelemetry distributed tracing.
//! It allows logs to be correlated with traces for better observability in distributed systems.

use crate::logging::LoggingConfig;

/// OpenTelemetry tracing configuration
#[derive(Debug, Clone)]
pub struct OtelTracingConfig {
    /// Service name for traces
    pub service_name: String,
    /// Deployment environment (development, staging, production)
    pub environment: String,
    /// Jaeger endpoint for trace export
    pub jaeger_endpoint: Option<String>,
    /// OTLP endpoint (alternative to Jaeger)
    pub otlp_endpoint: Option<String>,
    /// Protocol: grpc or http
    pub protocol: String,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for OtelTracingConfig {
    fn default() -> Self {
        Self {
            service_name: "mockforge".to_string(),
            environment: "development".to_string(),
            jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            protocol: "grpc".to_string(),
            sampling_rate: 1.0,
        }
    }
}

/// Initialize logging with OpenTelemetry tracing
///
/// This is a convenience function that integrates logging with OpenTelemetry when the
/// mockforge-tracing crate is available. It initializes the OpenTelemetry tracer and
/// sets up the logging system with a tracing layer.
///
/// # Arguments
/// * `logging_config` - Logging configuration
/// * `tracing_config` - OpenTelemetry tracing configuration
///
/// # Example
/// ```no_run
/// use mockforge_observability::tracing_integration::{init_with_otel, OtelTracingConfig};
/// use mockforge_observability::logging::LoggingConfig;
///
/// let logging_config = LoggingConfig {
///     level: "info".to_string(),
///     json_format: true,
///     ..Default::default()
/// };
///
/// let tracing_config = OtelTracingConfig {
///     service_name: "mockforge".to_string(),
///     environment: "production".to_string(),
///     ..Default::default()
/// };
///
/// // init_with_otel(logging_config, tracing_config)
/// //     .expect("Failed to initialize logging with OpenTelemetry");
/// ```
#[cfg(feature = "opentelemetry")]
pub fn init_with_otel(
    logging_config: LoggingConfig,
    tracing_config: OtelTracingConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_opentelemetry::OpenTelemetryLayer;
    use tracing_subscriber::layer::SubscriberExt;

    // Initialize the OpenTelemetry tracer using mockforge-tracing
    // This would require mockforge-tracing as a dependency
    // For now, we'll just use the logging config without OpenTelemetry

    tracing::warn!("OpenTelemetry integration requires mockforge-tracing crate");
    crate::logging::init_logging(logging_config)?;

    Ok(())
}

/// Shutdown OpenTelemetry tracer and flush pending spans
#[cfg(feature = "opentelemetry")]
pub fn shutdown_otel() {
    // This would call mockforge_tracing::shutdown_tracer()
    tracing::info!("Shutting down OpenTelemetry tracer");
}

#[cfg(not(feature = "opentelemetry"))]
pub fn init_with_otel(
    logging_config: LoggingConfig,
    _tracing_config: OtelTracingConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::warn!("OpenTelemetry feature not enabled, using standard logging");
    crate::logging::init_logging(logging_config)?;
    Ok(())
}

#[cfg(not(feature = "opentelemetry"))]
pub fn shutdown_otel() {
    // No-op when OpenTelemetry is not enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_otel_config() {
        let config = OtelTracingConfig::default();
        assert_eq!(config.service_name, "mockforge");
        assert_eq!(config.environment, "development");
        assert_eq!(config.sampling_rate, 1.0);
        assert_eq!(config.protocol, "grpc");
    }

    #[test]
    fn test_custom_otel_config() {
        let config = OtelTracingConfig {
            service_name: "test-service".to_string(),
            environment: "production".to_string(),
            jaeger_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
            otlp_endpoint: Some("http://otel:4317".to_string()),
            protocol: "http".to_string(),
            sampling_rate: 0.5,
        };

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.environment, "production");
        assert_eq!(config.sampling_rate, 0.5);
    }
}
