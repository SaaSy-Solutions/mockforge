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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try to initialize OpenTelemetry tracing if mockforge-tracing is available
    #[cfg(feature = "mockforge-tracing")]
    {
        use mockforge_tracing::{init_tracer, ExporterType, TracingConfig};

        // Map protocol string to ExporterType enum
        let exporter_type = match tracing_config.protocol.as_str() {
            "grpc" | "http" if tracing_config.otlp_endpoint.is_some() => ExporterType::Otlp,
            _ => ExporterType::Jaeger, // Default to Jaeger
        };

        let tracing_cfg = TracingConfig {
            service_name: tracing_config.service_name.clone(),
            service_version: None, // Optional field
            environment: tracing_config.environment.clone(),
            jaeger_endpoint: tracing_config.jaeger_endpoint.clone(),
            otlp_endpoint: tracing_config.otlp_endpoint.clone(),
            exporter_type,
            sampling_rate: tracing_config.sampling_rate,
        };

        match init_tracer(tracing_cfg) {
            Ok(_) => {
                tracing::info!("OpenTelemetry tracing initialized successfully");
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize OpenTelemetry tracing: {}. Using logging only.",
                    e
                );
            }
        }
    }

    #[cfg(not(feature = "mockforge-tracing"))]
    {
        tracing::warn!("OpenTelemetry feature enabled but mockforge-tracing crate not available. Using logging only.");
    }

    // Always initialize logging
    crate::logging::init_logging(logging_config)?;

    Ok(())
}

/// Shutdown OpenTelemetry tracer and flush pending spans
#[cfg(feature = "opentelemetry")]
pub fn shutdown_otel() {
    #[cfg(feature = "mockforge-tracing")]
    {
        use mockforge_tracing::shutdown_tracer;
        shutdown_tracer();
        tracing::info!("OpenTelemetry tracer shut down");
    }

    #[cfg(not(feature = "mockforge-tracing"))]
    {
        tracing::debug!("OpenTelemetry shutdown called but mockforge-tracing not available");
    }
}

#[cfg(not(feature = "opentelemetry"))]
pub fn init_with_otel(
    logging_config: LoggingConfig,
    _tracing_config: OtelTracingConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
