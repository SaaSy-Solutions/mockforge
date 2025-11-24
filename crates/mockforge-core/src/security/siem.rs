//! SIEM (Security Information and Event Management) integration for MockForge
//!
//! This module provides integration with SIEM systems for security event monitoring and compliance.
//! Supports multiple transport methods including Syslog, HTTP/HTTPS, File-based export, and
//! cloud SIEM systems (Splunk, Datadog, AWS CloudWatch, GCP Logging, Azure Monitor).

use crate::security::events::SecurityEvent;
use crate::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// SIEM protocol types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SiemProtocol {
    /// Syslog (RFC 5424)
    Syslog,
    /// HTTP/HTTPS webhook
    Http,
    /// HTTPS webhook
    Https,
    /// File-based export
    File,
    /// Splunk HEC (HTTP Event Collector)
    Splunk,
    /// Datadog API
    Datadog,
    /// AWS CloudWatch Logs
    Cloudwatch,
    /// Google Cloud Logging
    Gcp,
    /// Azure Monitor Logs
    Azure,
}

/// Syslog facility codes (RFC 5424)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SyslogFacility {
    /// Kernel messages
    Kernel = 0,
    /// User-level messages
    User = 1,
    /// Mail system
    Mail = 2,
    /// System daemons
    Daemon = 3,
    /// Security/authorization messages
    Security = 4,
    /// Messages generated internally by syslogd
    Syslogd = 5,
    /// Line printer subsystem
    LinePrinter = 6,
    /// Network news subsystem
    NetworkNews = 7,
    /// UUCP subsystem
    Uucp = 8,
    /// Clock daemon
    Clock = 9,
    /// Security/authorization messages (alternative)
    Security2 = 10,
    /// FTP daemon
    Ftp = 11,
    /// NTP subsystem
    Ntp = 12,
    /// Log audit
    LogAudit = 13,
    /// Log alert
    LogAlert = 14,
    /// Local use 0
    #[default]
    Local0 = 16,
    /// Local use 1
    Local1 = 17,
    /// Local use 2
    Local2 = 18,
    /// Local use 3
    Local3 = 19,
    /// Local use 4
    Local4 = 20,
    /// Local use 5
    Local5 = 21,
    /// Local use 6
    Local6 = 22,
    /// Local use 7
    Local7 = 23,
}

/// Syslog severity levels (RFC 5424)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogSeverity {
    /// System is unusable
    Emergency = 0,
    /// Action must be taken immediately
    Alert = 1,
    /// Critical conditions
    Critical = 2,
    /// Error conditions
    Error = 3,
    /// Warning conditions
    Warning = 4,
    /// Normal but significant condition
    Notice = 5,
    /// Informational messages
    Informational = 6,
    /// Debug-level messages
    Debug = 7,
}

impl From<crate::security::events::SecurityEventSeverity> for SyslogSeverity {
    fn from(severity: crate::security::events::SecurityEventSeverity) -> Self {
        match severity {
            crate::security::events::SecurityEventSeverity::Low => SyslogSeverity::Informational,
            crate::security::events::SecurityEventSeverity::Medium => SyslogSeverity::Warning,
            crate::security::events::SecurityEventSeverity::High => SyslogSeverity::Error,
            crate::security::events::SecurityEventSeverity::Critical => SyslogSeverity::Critical,
        }
    }
}

/// Retry configuration for SIEM delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Backoff strategy: "exponential" or "linear"
    #[serde(default = "default_backoff")]
    pub backoff: String,
    /// Initial delay in seconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_secs: u64,
}

fn default_backoff() -> String {
    "exponential".to_string()
}

fn default_initial_delay() -> u64 {
    1
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: "exponential".to_string(),
            initial_delay_secs: 1,
        }
    }
}

/// File rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileRotationConfig {
    /// Maximum file size (e.g., "100MB", "1GB")
    pub max_size: String,
    /// Maximum number of files to keep
    pub max_files: u32,
    /// Whether to compress rotated files
    #[serde(default)]
    pub compress: bool,
}

/// Event filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EventFilter {
    /// Include patterns (e.g., ["auth.*", "authz.*"])
    pub include: Option<Vec<String>>,
    /// Exclude patterns (e.g., ["severity:low"])
    pub exclude: Option<Vec<String>>,
    /// Additional filter conditions (not implemented in initial version)
    pub conditions: Option<Vec<String>>,
}

impl EventFilter {
    /// Check if an event should be included based on filters
    pub fn should_include(&self, event: &SecurityEvent) -> bool {
        // Check include patterns
        if let Some(ref includes) = self.include {
            let mut matched = false;
            for pattern in includes {
                if self.matches_pattern(&event.event_type, pattern) {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return false;
            }
        }

        // Check exclude patterns
        if let Some(ref excludes) = self.exclude {
            for pattern in excludes {
                if pattern.starts_with("severity:") {
                    let severity_str = pattern.strip_prefix("severity:").unwrap_or("");
                    if severity_str == "low"
                        && event.severity == crate::security::events::SecurityEventSeverity::Low
                    {
                        return false;
                    }
                    if severity_str == "medium"
                        && event.severity == crate::security::events::SecurityEventSeverity::Medium
                    {
                        return false;
                    }
                    if severity_str == "high"
                        && event.severity == crate::security::events::SecurityEventSeverity::High
                    {
                        return false;
                    }
                    if severity_str == "critical"
                        && event.severity
                            == crate::security::events::SecurityEventSeverity::Critical
                    {
                        return false;
                    }
                } else if self.matches_pattern(&event.event_type, pattern) {
                    return false;
                }
            }
        }

        true
    }

    fn matches_pattern(&self, event_type: &str, pattern: &str) -> bool {
        // Simple glob pattern matching (e.g., "auth.*" matches "auth.success")
        if pattern.ends_with(".*") {
            let prefix = pattern.strip_suffix(".*").unwrap_or("");
            event_type.starts_with(prefix)
        } else {
            event_type == pattern
        }
    }
}

/// SIEM destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "protocol")]
pub enum SiemDestination {
    /// Syslog destination
    #[serde(rename = "syslog")]
    Syslog {
        /// Syslog host
        host: String,
        /// Syslog port
        port: u16,
        /// Transport protocol (udp or tcp)
        #[serde(default = "default_syslog_protocol", rename = "transport")]
        transport: String,
        /// Syslog facility
        #[serde(default)]
        facility: SyslogFacility,
        /// Tag/application name
        #[serde(default = "default_tag")]
        tag: String,
    },
    /// HTTP/HTTPS webhook destination
    #[serde(rename = "http")]
    Http {
        /// Webhook URL
        url: String,
        /// HTTP method (default: POST)
        #[serde(default = "default_http_method")]
        method: String,
        /// Custom headers
        #[serde(default)]
        headers: HashMap<String, String>,
        /// Request timeout in seconds
        #[serde(default = "default_timeout")]
        timeout: u64,
        /// Retry configuration
        #[serde(default)]
        retry: RetryConfig,
    },
    /// HTTPS webhook destination (alias for http with https URL)
    #[serde(rename = "https")]
    Https {
        /// Webhook URL
        url: String,
        /// HTTP method (default: POST)
        #[serde(default = "default_http_method")]
        method: String,
        /// Custom headers
        #[serde(default)]
        headers: HashMap<String, String>,
        /// Request timeout in seconds
        #[serde(default = "default_timeout")]
        timeout: u64,
        /// Retry configuration
        #[serde(default)]
        retry: RetryConfig,
    },
    /// File-based export destination
    #[serde(rename = "file")]
    File {
        /// File path
        path: String,
        /// File format (jsonl or json)
        #[serde(default = "default_file_format")]
        format: String,
        /// File rotation configuration
        rotation: Option<FileRotationConfig>,
    },
    /// Splunk HEC destination
    #[serde(rename = "splunk")]
    Splunk {
        /// Splunk HEC URL
        url: String,
        /// Splunk HEC token
        token: String,
        /// Splunk index
        index: Option<String>,
        /// Source type
        source_type: Option<String>,
    },
    /// Datadog API destination
    #[serde(rename = "datadog")]
    Datadog {
        /// Datadog API key
        api_key: String,
        /// Datadog application key (optional)
        app_key: Option<String>,
        /// Datadog site (default: datadoghq.com)
        #[serde(default = "default_datadog_site")]
        site: String,
        /// Additional tags
        #[serde(default)]
        tags: Vec<String>,
    },
    /// AWS CloudWatch Logs destination
    #[serde(rename = "cloudwatch")]
    Cloudwatch {
        /// AWS region
        region: String,
        /// Log group name
        log_group: String,
        /// Log stream name
        stream: String,
        /// AWS credentials (access_key_id, secret_access_key)
        credentials: HashMap<String, String>,
    },
    /// Google Cloud Logging destination
    #[serde(rename = "gcp")]
    Gcp {
        /// GCP project ID
        project_id: String,
        /// Log name
        log_name: String,
        /// Service account credentials path
        credentials_path: String,
    },
    /// Azure Monitor Logs destination
    #[serde(rename = "azure")]
    Azure {
        /// Azure workspace ID
        workspace_id: String,
        /// Azure shared key
        shared_key: String,
        /// Log type
        log_type: String,
    },
}

fn default_syslog_protocol() -> String {
    "udp".to_string()
}

fn default_tag() -> String {
    "mockforge".to_string()
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    5
}

fn default_file_format() -> String {
    "jsonl".to_string()
}

fn default_datadog_site() -> String {
    "datadoghq.com".to_string()
}

/// SIEM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default)]
pub struct SiemConfig {
    /// Whether SIEM integration is enabled
    pub enabled: bool,
    /// SIEM protocol (if single protocol)
    pub protocol: Option<SiemProtocol>,
    /// SIEM destinations
    pub destinations: Vec<SiemDestination>,
    /// Event filters
    pub filters: Option<EventFilter>,
}

/// Trait for SIEM transport implementations
#[async_trait]
pub trait SiemTransport: Send + Sync {
    /// Send a security event to the SIEM system
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error>;
}

/// Syslog transport implementation
pub struct SyslogTransport {
    host: String,
    port: u16,
    use_tcp: bool,
    facility: SyslogFacility,
    tag: String,
}

impl SyslogTransport {
    /// Create a new syslog transport
    ///
    /// # Arguments
    /// * `host` - Syslog server hostname or IP address
    /// * `port` - Syslog server port (typically 514)
    /// * `protocol` - Transport protocol ("udp" or "tcp")
    /// * `facility` - Syslog facility code
    /// * `tag` - Application tag/name
    pub fn new(
        host: String,
        port: u16,
        protocol: String,
        facility: SyslogFacility,
        tag: String,
    ) -> Self {
        Self {
            host,
            port,
            use_tcp: protocol == "tcp",
            facility,
            tag,
        }
    }

    /// Format event as RFC 5424 syslog message
    fn format_syslog_message(&self, event: &SecurityEvent) -> String {
        let severity: SyslogSeverity = event.severity.into();
        let priority = (self.facility as u8) * 8 + severity as u8;
        let timestamp = event.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let hostname = "mockforge"; // Could be configurable
        let app_name = &self.tag;
        let proc_id = "-";
        let msg_id = "-";
        let structured_data = "-"; // Could include event metadata
        let msg = event.to_json().unwrap_or_else(|_| "{}".to_string());

        format!(
            "<{}>1 {} {} {} {} {} {} {}",
            priority, timestamp, hostname, app_name, proc_id, msg_id, structured_data, msg
        )
    }
}

#[async_trait]
impl SiemTransport for SyslogTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let message = self.format_syslog_message(event);

        if self.use_tcp {
            // TCP syslog
            use tokio::net::TcpStream;
            let addr = format!("{}:{}", self.host, self.port);
            let mut stream = TcpStream::connect(&addr).await.map_err(|e| {
                Error::Generic(format!("Failed to connect to syslog server: {}", e))
            })?;
            stream
                .write_all(message.as_bytes())
                .await
                .map_err(|e| Error::Generic(format!("Failed to send syslog message: {}", e)))?;
        } else {
            // UDP syslog
            use tokio::net::UdpSocket;
            let socket = UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|e| Error::Generic(format!("Failed to bind UDP socket: {}", e)))?;
            let addr = format!("{}:{}", self.host, self.port);
            socket
                .send_to(message.as_bytes(), &addr)
                .await
                .map_err(|e| Error::Generic(format!("Failed to send UDP syslog message: {}", e)))?;
        }

        debug!("Sent syslog event: {}", event.event_type);
        Ok(())
    }
}

/// HTTP transport implementation
pub struct HttpTransport {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    timeout: u64,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl HttpTransport {
    /// Create a new HTTP transport
    ///
    /// # Arguments
    /// * `url` - Webhook URL endpoint
    /// * `method` - HTTP method (POST, PUT, PATCH)
    /// * `headers` - Custom HTTP headers to include
    /// * `timeout` - Request timeout in seconds
    /// * `retry` - Retry configuration
    pub fn new(
        url: String,
        method: String,
        headers: HashMap<String, String>,
        timeout: u64,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            url,
            method,
            headers,
            timeout,
            retry,
            client,
        }
    }
}

#[async_trait]
impl SiemTransport for HttpTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let event_json = event.to_json()?;
        let mut request = match self.method.as_str() {
            "POST" => self.client.post(&self.url),
            "PUT" => self.client.put(&self.url),
            "PATCH" => self.client.patch(&self.url),
            _ => return Err(Error::Generic(format!("Unsupported HTTP method: {}", self.method))),
        };

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Set content type if not specified
        if !self.headers.contains_key("Content-Type") {
            request = request.header("Content-Type", "application/json");
        }

        request = request.body(event_json);

        // Retry logic
        let mut last_error = None;
        for attempt in 0..=self.retry.max_attempts {
            match request.try_clone() {
                Some(req) => match req.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            debug!("Sent HTTP event to {}: {}", self.url, event.event_type);
                            return Ok(());
                        } else {
                            let status = response.status();
                            last_error = Some(Error::Generic(format!("HTTP error: {}", status)));
                        }
                    }
                    Err(e) => {
                        last_error = Some(Error::Generic(format!("HTTP request failed: {}", e)));
                    }
                },
                None => {
                    // Request body was consumed, recreate
                    let event_json = event.to_json()?;
                    let mut req = match self.method.as_str() {
                        "POST" => self.client.post(&self.url),
                        "PUT" => self.client.put(&self.url),
                        "PATCH" => self.client.patch(&self.url),
                        _ => break,
                    };
                    for (key, value) in &self.headers {
                        req = req.header(key, value);
                    }
                    if !self.headers.contains_key("Content-Type") {
                        req = req.header("Content-Type", "application/json");
                    }
                    req = req.body(event_json);
                    request = req;
                    continue;
                }
            }

            if attempt < self.retry.max_attempts {
                // Calculate backoff delay
                let delay = if self.retry.backoff == "exponential" {
                    self.retry.initial_delay_secs * (2_u64.pow(attempt))
                } else {
                    self.retry.initial_delay_secs * (attempt as u64 + 1)
                };
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Generic("Failed to send HTTP event after retries".to_string())
        }))
    }
}

/// File transport implementation
pub struct FileTransport {
    path: PathBuf,
    format: String,
    writer: Arc<RwLock<Option<BufWriter<File>>>>,
}

impl FileTransport {
    /// Create a new file transport
    ///
    /// # Arguments
    /// * `path` - File path for event output
    /// * `format` - File format ("jsonl" or "json")
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or opened
    pub async fn new(path: String, format: String) -> Result<Self, Error> {
        let path = PathBuf::from(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Generic(format!("Failed to create directory: {}", e)))?;
        }

        // Open file for appending
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| Error::Generic(format!("Failed to open file: {}", e)))?;

        let writer = Arc::new(RwLock::new(Some(BufWriter::new(file))));

        Ok(Self {
            path,
            format,
            writer,
        })
    }
}

#[async_trait]
impl SiemTransport for FileTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let mut writer_guard = self.writer.write().await;

        if let Some(ref mut writer) = *writer_guard {
            let line = if self.format == "jsonl" {
                format!("{}\n", event.to_json()?)
            } else {
                // JSON array format (would need to manage array structure)
                format!("{}\n", event.to_json()?)
            };

            writer
                .write_all(line.as_bytes())
                .await
                .map_err(|e| Error::Generic(format!("Failed to write to file: {}", e)))?;

            writer
                .flush()
                .await
                .map_err(|e| Error::Generic(format!("Failed to flush file: {}", e)))?;

            debug!("Wrote event to file {}: {}", self.path.display(), event.event_type);
            Ok(())
        } else {
            Err(Error::Generic("File writer not initialized".to_string()))
        }
    }
}

/// SIEM emitter that sends events to configured destinations
pub struct SiemEmitter {
    transports: Vec<Box<dyn SiemTransport>>,
    filters: Option<EventFilter>,
}

impl SiemEmitter {
    /// Create a new SIEM emitter from configuration
    pub async fn from_config(config: SiemConfig) -> Result<Self, Error> {
        if !config.enabled {
            return Ok(Self {
                transports: Vec::new(),
                filters: config.filters,
            });
        }

        let mut transports: Vec<Box<dyn SiemTransport>> = Vec::new();

        for dest in config.destinations {
            let transport: Box<dyn SiemTransport> = match dest {
                SiemDestination::Syslog {
                    host,
                    port,
                    transport,
                    facility,
                    tag,
                } => Box::new(SyslogTransport::new(host, port, transport, facility, tag)),
                SiemDestination::Http {
                    url,
                    method,
                    headers,
                    timeout,
                    retry,
                } => Box::new(HttpTransport::new(url, method, headers, timeout, retry)),
                SiemDestination::Https {
                    url,
                    method,
                    headers,
                    timeout,
                    retry,
                } => Box::new(HttpTransport::new(url, method, headers, timeout, retry)),
                SiemDestination::File { path, format, .. } => {
                    Box::new(FileTransport::new(path, format).await?)
                }
                SiemDestination::Splunk { .. }
                | SiemDestination::Datadog { .. }
                | SiemDestination::Cloudwatch { .. }
                | SiemDestination::Gcp { .. }
                | SiemDestination::Azure { .. } => {
                    warn!("Cloud SIEM integration not yet implemented: {:?}", dest);
                    continue;
                }
            };
            transports.push(transport);
        }

        Ok(Self {
            transports,
            filters: config.filters,
        })
    }

    /// Emit a security event to all configured SIEM destinations
    pub async fn emit(&self, event: SecurityEvent) -> Result<(), Error> {
        // Apply filters
        if let Some(ref filter) = self.filters {
            if !filter.should_include(&event) {
                debug!("Event filtered out: {}", event.event_type);
                return Ok(());
            }
        }

        // Send to all transports
        let mut errors = Vec::new();
        for transport in &self.transports {
            match transport.send_event(&event).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to send event to SIEM: {}", e);
                    errors.push(e);
                }
            }
        }

        if !errors.is_empty() && errors.len() == self.transports.len() {
            // All transports failed
            return Err(Error::Generic(format!(
                "All SIEM transports failed: {} errors",
                errors.len()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::events::{EventActor, EventOutcome, EventTarget, SecurityEventType};

    #[test]
    fn test_event_filter_include() {
        let filter = EventFilter {
            include: Some(vec!["auth.*".to_string()]),
            exclude: None,
            conditions: None,
        };

        let event =
            crate::security::events::SecurityEvent::new(SecurityEventType::AuthSuccess, None, None);

        assert!(filter.should_include(&event));

        let event = crate::security::events::SecurityEvent::new(
            SecurityEventType::ConfigChanged,
            None,
            None,
        );

        assert!(!filter.should_include(&event));
    }

    #[test]
    fn test_event_filter_exclude() {
        let filter = EventFilter {
            include: None,
            exclude: Some(vec!["severity:low".to_string()]),
            conditions: None,
        };

        let event =
            crate::security::events::SecurityEvent::new(SecurityEventType::AuthSuccess, None, None);

        assert!(!filter.should_include(&event));

        let event =
            crate::security::events::SecurityEvent::new(SecurityEventType::AuthFailure, None, None);

        assert!(filter.should_include(&event));
    }

    #[tokio::test]
    async fn test_syslog_transport_format() {
        let transport = SyslogTransport::new(
            "localhost".to_string(),
            514,
            "udp".to_string(),
            SyslogFacility::Local0,
            "mockforge".to_string(),
        );

        let event =
            crate::security::events::SecurityEvent::new(SecurityEventType::AuthSuccess, None, None);

        let message = transport.format_syslog_message(&event);
        assert!(message.starts_with("<"));
        assert!(message.contains("mockforge"));
    }
}
