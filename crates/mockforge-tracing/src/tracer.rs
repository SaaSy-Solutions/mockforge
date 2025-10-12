//! OpenTelemetry tracer initialization and configuration

use crate::exporter::ExporterType;
use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_sdk::Resource;
use std::error::Error;
use std::time::Duration;

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for traces
    pub service_name: String,
    /// Exporter type (Jaeger or OTLP)
    pub exporter_type: ExporterType,
    /// Jaeger endpoint (e.g., "http://localhost:14268/api/traces")
    pub jaeger_endpoint: Option<String>,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
    /// Environment (e.g., "development", "production")
    pub environment: String,
    /// Service version
    pub service_version: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "mockforge".to_string(),
            exporter_type: ExporterType::Jaeger,
            jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
            otlp_endpoint: None,
            sampling_rate: 1.0,
            environment: "development".to_string(),
            service_version: None,
        }
    }
}

impl TracingConfig {
    /// Create config for Jaeger exporter
    pub fn with_jaeger(service_name: String, endpoint: String) -> Self {
        Self {
            service_name,
            exporter_type: ExporterType::Jaeger,
            jaeger_endpoint: Some(endpoint),
            otlp_endpoint: None,
            ..Default::default()
        }
    }

    /// Create config for OTLP exporter
    pub fn with_otlp(service_name: String, endpoint: String) -> Self {
        Self {
            service_name,
            exporter_type: ExporterType::Otlp,
            jaeger_endpoint: None,
            otlp_endpoint: Some(endpoint),
            ..Default::default()
        }
    }

    /// Set sampling rate
    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate;
        self
    }

    /// Set environment
    pub fn with_environment(mut self, env: String) -> Self {
        self.environment = env;
        self
    }

    /// Set service version
    pub fn with_service_version(mut self, version: String) -> Self {
        self.service_version = Some(version);
        self
    }
}

/// Initialize the OpenTelemetry tracer
pub fn init_tracer(config: TracingConfig) -> Result<Tracer, Box<dyn Error + Send + Sync>> {
    match config.exporter_type {
        ExporterType::Jaeger => init_jaeger_tracer(config),
        ExporterType::Otlp => init_otlp_tracer(config),
    }
}

/// Initialize Jaeger tracer
fn init_jaeger_tracer(config: TracingConfig) -> Result<Tracer, Box<dyn Error + Send + Sync>> {
    let endpoint = config.jaeger_endpoint.ok_or("Jaeger endpoint not configured")?;

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(&config.service_name)
        .with_endpoint(&endpoint)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracer)
}

/// Initialize OTLP tracer
fn init_otlp_tracer(config: TracingConfig) -> Result<Tracer, Box<dyn Error + Send + Sync>> {
    let endpoint = config.otlp_endpoint.ok_or("OTLP endpoint not configured")?;

    // Build resource attributes
    let mut resource_attrs = vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ];

    if let Some(version) = config.service_version {
        resource_attrs.push(KeyValue::new("service.version", version));
    }

    let resource = Resource::new(resource_attrs);

    // Create OTLP exporter with gRPC protocol
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&endpoint)
        .with_timeout(Duration::from_secs(10));

    // Build tracer pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_resource(resource)
                .with_sampler(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(
                    config.sampling_rate,
                )),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracer)
}

/// Shutdown the tracer and flush pending spans
pub fn shutdown_tracer() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "mockforge");
        assert_eq!(config.sampling_rate, 1.0);
        assert_eq!(config.environment, "development");
        assert_eq!(config.exporter_type, ExporterType::Jaeger);
        assert!(config.jaeger_endpoint.is_some());
        assert!(config.otlp_endpoint.is_none());
    }

    #[test]
    fn test_jaeger_config() {
        let config = TracingConfig::with_jaeger(
            "test-service".to_string(),
            "http://custom:14268/api/traces".to_string(),
        )
        .with_sampling_rate(0.5)
        .with_environment("staging".to_string())
        .with_service_version("1.0.0".to_string());

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.exporter_type, ExporterType::Jaeger);
        assert_eq!(config.jaeger_endpoint, Some("http://custom:14268/api/traces".to_string()));
        assert_eq!(config.sampling_rate, 0.5);
        assert_eq!(config.environment, "staging");
        assert_eq!(config.service_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_otlp_config() {
        let config = TracingConfig::with_otlp(
            "test-service".to_string(),
            "http://otel-collector:4317".to_string(),
        )
        .with_sampling_rate(0.8)
        .with_environment("production".to_string())
        .with_service_version("2.0.0".to_string());

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.exporter_type, ExporterType::Otlp);
        assert_eq!(config.otlp_endpoint, Some("http://otel-collector:4317".to_string()));
        assert!(config.jaeger_endpoint.is_none());
        assert_eq!(config.sampling_rate, 0.8);
        assert_eq!(config.environment, "production");
        assert_eq!(config.service_version, Some("2.0.0".to_string()));
    }
}
