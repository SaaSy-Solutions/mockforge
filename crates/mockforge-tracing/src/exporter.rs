//! Exporter configuration and utilities for Jaeger and OTLP

use std::time::Duration;
use thiserror::Error;

/// Exporter type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExporterType {
    Jaeger,
    Otlp,
}

/// Jaeger exporter configuration
#[derive(Debug, Clone)]
pub struct JaegerExporter {
    /// Jaeger agent endpoint
    pub endpoint: String,
    /// Maximum batch size for spans
    pub max_batch_size: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
}

impl Default for JaegerExporter {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:14268/api/traces".to_string(),
            max_batch_size: 512,
            max_queue_size: 2048,
            batch_timeout: Duration::from_secs(5),
        }
    }
}

impl JaegerExporter {
    /// Create a new Jaeger exporter with custom configuration
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            ..Default::default()
        }
    }

    /// Set maximum batch size
    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Set maximum queue size
    pub fn with_max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = size;
        self
    }

    /// Set batch timeout
    pub fn with_batch_timeout(mut self, timeout: Duration) -> Self {
        self.batch_timeout = timeout;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ExporterError> {
        if self.endpoint.is_empty() {
            return Err(ExporterError::InvalidEndpoint("Endpoint cannot be empty".to_string()));
        }

        if self.max_batch_size == 0 {
            return Err(ExporterError::InvalidConfig(
                "Max batch size must be greater than 0".to_string(),
            ));
        }

        if self.max_queue_size < self.max_batch_size {
            return Err(ExporterError::InvalidConfig(
                "Max queue size must be >= max batch size".to_string(),
            ));
        }

        Ok(())
    }
}

/// OTLP exporter configuration
#[derive(Debug, Clone)]
pub struct OtlpExporter {
    /// OTLP endpoint (e.g., "http://localhost:4317" for gRPC)
    pub endpoint: String,
    /// Protocol (grpc or http/protobuf)
    pub protocol: OtlpProtocol,
    /// Optional headers for authentication
    pub headers: Vec<(String, String)>,
    /// Timeout for export requests
    pub timeout: Duration,
    /// Compression (none, gzip)
    pub compression: Option<OtlpCompression>,
}

/// OTLP protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    HttpProtobuf,
}

/// OTLP compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpCompression {
    Gzip,
}

impl Default for OtlpExporter {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            protocol: OtlpProtocol::Grpc,
            headers: Vec::new(),
            timeout: Duration::from_secs(10),
            compression: None,
        }
    }
}

impl OtlpExporter {
    /// Create a new OTLP exporter with custom endpoint
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            ..Default::default()
        }
    }

    /// Set protocol
    pub fn with_protocol(mut self, protocol: OtlpProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// Add authentication header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, compression: OtlpCompression) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ExporterError> {
        if self.endpoint.is_empty() {
            return Err(ExporterError::InvalidEndpoint("Endpoint cannot be empty".to_string()));
        }

        // Validate URL format
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(ExporterError::InvalidEndpoint(
                "Endpoint must start with http:// or https://".to_string(),
            ));
        }

        Ok(())
    }
}

/// Exporter configuration errors
#[derive(Error, Debug)]
pub enum ExporterError {
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Export failed: {0}")]
    ExportFailed(String),
}

// Maintain backwards compatibility
pub type JaegerExporterError = ExporterError;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ExporterType Tests ====================

    #[test]
    fn test_exporter_type_debug() {
        assert_eq!(format!("{:?}", ExporterType::Jaeger), "Jaeger");
        assert_eq!(format!("{:?}", ExporterType::Otlp), "Otlp");
    }

    #[test]
    fn test_exporter_type_clone() {
        let exporter = ExporterType::Jaeger;
        let cloned = exporter.clone();
        assert_eq!(exporter, cloned);
    }

    #[test]
    fn test_exporter_type_eq() {
        assert_eq!(ExporterType::Jaeger, ExporterType::Jaeger);
        assert_eq!(ExporterType::Otlp, ExporterType::Otlp);
        assert_ne!(ExporterType::Jaeger, ExporterType::Otlp);
    }

    // ==================== JaegerExporter Tests ====================

    #[test]
    fn test_jaeger_default_config() {
        let exporter = JaegerExporter::default();
        assert_eq!(exporter.endpoint, "http://localhost:14268/api/traces");
        assert_eq!(exporter.max_batch_size, 512);
        assert_eq!(exporter.max_queue_size, 2048);
        assert!(exporter.validate().is_ok());
    }

    #[test]
    fn test_jaeger_custom_config() {
        let exporter = JaegerExporter::new("http://custom:14268/api/traces".to_string())
            .with_max_batch_size(256)
            .with_max_queue_size(1024)
            .with_batch_timeout(Duration::from_secs(10));

        assert_eq!(exporter.endpoint, "http://custom:14268/api/traces");
        assert_eq!(exporter.max_batch_size, 256);
        assert_eq!(exporter.max_queue_size, 1024);
        assert_eq!(exporter.batch_timeout, Duration::from_secs(10));
        assert!(exporter.validate().is_ok());
    }

    #[test]
    fn test_jaeger_invalid_config() {
        let exporter = JaegerExporter::new("http://localhost:14268".to_string())
            .with_max_batch_size(1024)
            .with_max_queue_size(512); // Less than batch size

        assert!(exporter.validate().is_err());
    }

    #[test]
    fn test_jaeger_empty_endpoint() {
        let exporter = JaegerExporter::new("".to_string());
        assert!(exporter.validate().is_err());
    }

    #[test]
    fn test_jaeger_zero_batch_size() {
        let exporter = JaegerExporter::new("http://localhost:14268/api/traces".to_string())
            .with_max_batch_size(0);
        let result = exporter.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExporterError::InvalidConfig(_)));
    }

    #[test]
    fn test_jaeger_debug() {
        let exporter = JaegerExporter::default();
        let debug_str = format!("{:?}", exporter);
        assert!(debug_str.contains("JaegerExporter"));
        assert!(debug_str.contains("endpoint"));
    }

    #[test]
    fn test_jaeger_clone() {
        let exporter =
            JaegerExporter::new("http://test:14268".to_string()).with_max_batch_size(100);
        let cloned = exporter.clone();
        assert_eq!(cloned.endpoint, exporter.endpoint);
        assert_eq!(cloned.max_batch_size, exporter.max_batch_size);
    }

    #[test]
    fn test_jaeger_queue_equals_batch() {
        // Queue size equal to batch size should be valid
        let exporter = JaegerExporter::new("http://localhost:14268/api/traces".to_string())
            .with_max_batch_size(512)
            .with_max_queue_size(512);
        assert!(exporter.validate().is_ok());
    }

    // ==================== OtlpExporter Tests ====================

    #[test]
    fn test_otlp_default_config() {
        let exporter = OtlpExporter::default();
        assert_eq!(exporter.endpoint, "http://localhost:4317");
        assert_eq!(exporter.protocol, OtlpProtocol::Grpc);
        assert!(exporter.headers.is_empty());
        assert_eq!(exporter.timeout, Duration::from_secs(10));
        assert!(exporter.compression.is_none());
        assert!(exporter.validate().is_ok());
    }

    #[test]
    fn test_otlp_custom_config() {
        let exporter = OtlpExporter::new("https://otel-collector:4317".to_string())
            .with_protocol(OtlpProtocol::HttpProtobuf)
            .with_header("Authorization".to_string(), "Bearer token123".to_string())
            .with_timeout(Duration::from_secs(30))
            .with_compression(OtlpCompression::Gzip);

        assert_eq!(exporter.endpoint, "https://otel-collector:4317");
        assert_eq!(exporter.protocol, OtlpProtocol::HttpProtobuf);
        assert_eq!(exporter.headers.len(), 1);
        assert_eq!(exporter.timeout, Duration::from_secs(30));
        assert_eq!(exporter.compression, Some(OtlpCompression::Gzip));
        assert!(exporter.validate().is_ok());
    }

    #[test]
    fn test_otlp_empty_endpoint() {
        let exporter = OtlpExporter::new("".to_string());
        assert!(exporter.validate().is_err());
    }

    #[test]
    fn test_otlp_invalid_endpoint_protocol() {
        let exporter = OtlpExporter::new("ftp://localhost:4317".to_string());
        assert!(exporter.validate().is_err());
    }

    #[test]
    fn test_otlp_multiple_headers() {
        let exporter = OtlpExporter::new("http://localhost:4317".to_string())
            .with_header("X-API-Key".to_string(), "key123".to_string())
            .with_header("X-Tenant-ID".to_string(), "tenant1".to_string());

        assert_eq!(exporter.headers.len(), 2);
    }

    #[test]
    fn test_otlp_https_endpoint() {
        let exporter = OtlpExporter::new("https://secure-collector:4317".to_string());
        assert!(exporter.validate().is_ok());
    }

    #[test]
    fn test_otlp_debug() {
        let exporter = OtlpExporter::default();
        let debug_str = format!("{:?}", exporter);
        assert!(debug_str.contains("OtlpExporter"));
        assert!(debug_str.contains("endpoint"));
    }

    #[test]
    fn test_otlp_clone() {
        let exporter = OtlpExporter::new("http://test:4317".to_string())
            .with_protocol(OtlpProtocol::HttpProtobuf)
            .with_compression(OtlpCompression::Gzip);
        let cloned = exporter.clone();
        assert_eq!(cloned.endpoint, exporter.endpoint);
        assert_eq!(cloned.protocol, exporter.protocol);
        assert_eq!(cloned.compression, exporter.compression);
    }

    // ==================== OtlpProtocol Tests ====================

    #[test]
    fn test_otlp_protocol_debug() {
        assert_eq!(format!("{:?}", OtlpProtocol::Grpc), "Grpc");
        assert_eq!(format!("{:?}", OtlpProtocol::HttpProtobuf), "HttpProtobuf");
    }

    #[test]
    fn test_otlp_protocol_clone() {
        let proto = OtlpProtocol::Grpc;
        let cloned = proto.clone();
        assert_eq!(proto, cloned);
    }

    #[test]
    fn test_otlp_protocol_copy() {
        let proto = OtlpProtocol::HttpProtobuf;
        let copied = proto;
        assert_eq!(OtlpProtocol::HttpProtobuf, copied);
    }

    #[test]
    fn test_otlp_protocol_eq() {
        assert_eq!(OtlpProtocol::Grpc, OtlpProtocol::Grpc);
        assert_ne!(OtlpProtocol::Grpc, OtlpProtocol::HttpProtobuf);
    }

    // ==================== OtlpCompression Tests ====================

    #[test]
    fn test_otlp_compression_debug() {
        assert_eq!(format!("{:?}", OtlpCompression::Gzip), "Gzip");
    }

    #[test]
    fn test_otlp_compression_clone() {
        let comp = OtlpCompression::Gzip;
        let cloned = comp.clone();
        assert_eq!(comp, cloned);
    }

    #[test]
    fn test_otlp_compression_copy() {
        let comp = OtlpCompression::Gzip;
        let copied = comp;
        assert_eq!(OtlpCompression::Gzip, copied);
    }

    #[test]
    fn test_otlp_compression_eq() {
        assert_eq!(OtlpCompression::Gzip, OtlpCompression::Gzip);
    }

    // ==================== ExporterError Tests ====================

    #[test]
    fn test_exporter_error_invalid_endpoint() {
        let error = ExporterError::InvalidEndpoint("test error".to_string());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Invalid endpoint"));
        assert!(error_str.contains("test error"));
    }

    #[test]
    fn test_exporter_error_invalid_config() {
        let error = ExporterError::InvalidConfig("config error".to_string());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Invalid configuration"));
        assert!(error_str.contains("config error"));
    }

    #[test]
    fn test_exporter_error_export_failed() {
        let error = ExporterError::ExportFailed("export error".to_string());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Export failed"));
        assert!(error_str.contains("export error"));
    }

    #[test]
    fn test_exporter_error_debug() {
        let error = ExporterError::InvalidEndpoint("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidEndpoint"));
    }

    #[test]
    fn test_jaeger_exporter_error_alias() {
        // Test the type alias for backwards compatibility
        let error: JaegerExporterError = ExporterError::InvalidEndpoint("test".to_string());
        assert!(matches!(error, ExporterError::InvalidEndpoint(_)));
    }

    // ==================== Validation Edge Cases ====================

    #[test]
    fn test_jaeger_validation_error_messages() {
        let exporter = JaegerExporter::new("".to_string());
        if let Err(e) = exporter.validate() {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("empty"));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_otlp_validation_error_messages() {
        let exporter = OtlpExporter::new("invalid-url".to_string());
        if let Err(e) = exporter.validate() {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("http://") || error_msg.contains("https://"));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_otlp_no_compression() {
        let exporter = OtlpExporter::new("http://localhost:4317".to_string());
        assert!(exporter.compression.is_none());
    }

    #[test]
    fn test_jaeger_default_batch_timeout() {
        let exporter = JaegerExporter::default();
        assert_eq!(exporter.batch_timeout, Duration::from_secs(5));
    }
}
