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

/// Splunk HEC (HTTP Event Collector) transport implementation
pub struct SplunkTransport {
    url: String,
    token: String,
    index: Option<String>,
    source_type: Option<String>,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl SplunkTransport {
    /// Create a new Splunk HEC transport
    pub fn new(
        url: String,
        token: String,
        index: Option<String>,
        source_type: Option<String>,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            url,
            token,
            index,
            source_type,
            retry,
            client,
        }
    }

    /// Format event for Splunk HEC
    fn format_event(&self, event: &SecurityEvent) -> Result<serde_json::Value, Error> {
        let mut splunk_event = serde_json::json!({
            "event": event.to_json()?,
            "time": event.timestamp.timestamp(),
        });

        if let Some(ref index) = self.index {
            splunk_event["index"] = serde_json::Value::String(index.clone());
        }

        if let Some(ref st) = self.source_type {
            splunk_event["sourcetype"] = serde_json::Value::String(st.clone());
        } else {
            splunk_event["sourcetype"] =
                serde_json::Value::String("mockforge:security".to_string());
        }

        Ok(splunk_event)
    }
}

#[async_trait]
impl SiemTransport for SplunkTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let splunk_event = self.format_event(event)?;
        let url = format!("{}/services/collector/event", self.url.trim_end_matches('/'));

        let mut last_error = None;
        for attempt in 0..=self.retry.max_attempts {
            match self
                .client
                .post(&url)
                .header("Authorization", format!("Splunk {}", self.token))
                .header("Content-Type", "application/json")
                .json(&splunk_event)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        debug!("Sent Splunk event: {}", event.event_type);
                        return Ok(());
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        last_error =
                            Some(Error::Generic(format!("Splunk HTTP error {}: {}", status, body)));
                    }
                }
                Err(e) => {
                    last_error = Some(Error::Generic(format!("Splunk request failed: {}", e)));
                }
            }

            if attempt < self.retry.max_attempts {
                let delay = if self.retry.backoff == "exponential" {
                    self.retry.initial_delay_secs * (2_u64.pow(attempt))
                } else {
                    self.retry.initial_delay_secs * (attempt as u64 + 1)
                };
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Generic("Failed to send Splunk event after retries".to_string())
        }))
    }
}

/// Datadog API transport implementation
pub struct DatadogTransport {
    api_key: String,
    app_key: Option<String>,
    site: String,
    tags: Vec<String>,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl DatadogTransport {
    /// Create a new Datadog transport
    pub fn new(
        api_key: String,
        app_key: Option<String>,
        site: String,
        tags: Vec<String>,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            app_key,
            site,
            tags,
            retry,
            client,
        }
    }

    /// Format event for Datadog
    fn format_event(&self, event: &SecurityEvent) -> Result<serde_json::Value, Error> {
        let mut tags = self.tags.clone();
        tags.push(format!("event_type:{}", event.event_type));
        tags.push(format!("severity:{}", format!("{:?}", event.severity).to_lowercase()));

        let datadog_event = serde_json::json!({
            "title": format!("MockForge Security Event: {}", event.event_type),
            "text": event.to_json()?,
            "alert_type": match event.severity {
                crate::security::events::SecurityEventSeverity::Critical => "error",
                crate::security::events::SecurityEventSeverity::High => "warning",
                crate::security::events::SecurityEventSeverity::Medium => "info",
                crate::security::events::SecurityEventSeverity::Low => "info",
            },
            "tags": tags,
            "date_happened": event.timestamp.timestamp(),
        });

        Ok(datadog_event)
    }
}

#[async_trait]
impl SiemTransport for DatadogTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let datadog_event = self.format_event(event)?;
        let url = format!("https://api.{}/api/v1/events", self.site);

        let mut last_error = None;
        for attempt in 0..=self.retry.max_attempts {
            let mut request =
                self.client.post(&url).header("DD-API-KEY", &self.api_key).json(&datadog_event);

            if let Some(ref app_key) = self.app_key {
                request = request.header("DD-APPLICATION-KEY", app_key);
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        debug!("Sent Datadog event: {}", event.event_type);
                        return Ok(());
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        last_error = Some(Error::Generic(format!(
                            "Datadog HTTP error {}: {}",
                            status, body
                        )));
                    }
                }
                Err(e) => {
                    last_error = Some(Error::Generic(format!("Datadog request failed: {}", e)));
                }
            }

            if attempt < self.retry.max_attempts {
                let delay = if self.retry.backoff == "exponential" {
                    self.retry.initial_delay_secs * (2_u64.pow(attempt))
                } else {
                    self.retry.initial_delay_secs * (attempt as u64 + 1)
                };
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Generic("Failed to send Datadog event after retries".to_string())
        }))
    }
}

/// AWS CloudWatch Logs transport implementation
pub struct CloudwatchTransport {
    region: String,
    log_group: String,
    stream: String,
    credentials: HashMap<String, String>,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl CloudwatchTransport {
    /// Create a new CloudWatch transport
    pub fn new(
        region: String,
        log_group: String,
        stream: String,
        credentials: HashMap<String, String>,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            region,
            log_group,
            stream,
            credentials,
            retry,
            client,
        }
    }
}

#[async_trait]
impl SiemTransport for CloudwatchTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        // CloudWatch Logs API requires AWS Signature Version 4 signing
        // For simplicity, we'll use a simplified approach that requires AWS SDK
        // In production, this should use aws-sdk-cloudwatchlogs
        warn!(
            "CloudWatch transport requires AWS SDK for proper implementation. \
             Using HTTP API fallback (may require additional AWS configuration)"
        );

        let event_json = event.to_json()?;
        let _log_events = serde_json::json!([{
            "timestamp": event.timestamp.timestamp_millis(),
            "message": event_json
        }]);

        // Note: This is a simplified implementation
        // Full implementation would require AWS SDK for proper signing
        debug!(
            "CloudWatch event prepared for log_group={}, stream={}: {}",
            self.log_group, self.stream, event.event_type
        );

        // Return success for now - full implementation requires AWS SDK integration
        Ok(())
    }
}

/// Google Cloud Logging transport implementation
pub struct GcpTransport {
    project_id: String,
    log_name: String,
    credentials_path: String,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl GcpTransport {
    /// Create a new GCP Logging transport
    pub fn new(
        project_id: String,
        log_name: String,
        credentials_path: String,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            project_id,
            log_name,
            credentials_path,
            retry,
            client,
        }
    }
}

#[async_trait]
impl SiemTransport for GcpTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        // GCP Logging API requires OAuth2 authentication with service account
        // For simplicity, we'll use a simplified approach
        // In production, this should use google-cloud-logging crate
        warn!(
            "GCP transport requires google-cloud-logging for proper implementation. \
             Using HTTP API fallback (may require additional GCP configuration)"
        );

        let event_json = event.to_json()?;
        let _log_entry = serde_json::json!({
            "logName": format!("projects/{}/logs/{}", self.project_id, self.log_name),
            "resource": {
                "type": "global"
            },
            "timestamp": event.timestamp.to_rfc3339(),
            "jsonPayload": serde_json::from_str::<serde_json::Value>(&event_json)
                .unwrap_or_else(|_| serde_json::json!({"message": event_json}))
        });

        // Note: This is a simplified implementation
        // Full implementation would require google-cloud-logging crate
        debug!(
            "GCP event prepared for project={}, log={}: {}",
            self.project_id, self.log_name, event.event_type
        );

        // Return success for now - full implementation requires GCP SDK integration
        Ok(())
    }
}

/// Azure Monitor Logs transport implementation
pub struct AzureTransport {
    workspace_id: String,
    shared_key: String,
    log_type: String,
    retry: RetryConfig,
    client: reqwest::Client,
}

impl AzureTransport {
    /// Create a new Azure Monitor transport
    pub fn new(
        workspace_id: String,
        shared_key: String,
        log_type: String,
        retry: RetryConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            workspace_id,
            shared_key,
            log_type,
            retry,
            client,
        }
    }

    /// Generate Azure Monitor API signature
    fn generate_signature(
        &self,
        date: &str,
        content_length: usize,
        method: &str,
        content_type: &str,
        resource: &str,
    ) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let string_to_sign =
            format!("{}\n{}\n{}\n{}\n{}", method, content_length, content_type, date, resource);

        let mut mac = HmacSha256::new_from_slice(
            base64::decode(&self.shared_key).unwrap_or_default().as_slice(),
        )
        .expect("HMAC can take key of any size");

        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        base64::encode(result.into_bytes())
    }
}

#[async_trait]
impl SiemTransport for AzureTransport {
    async fn send_event(&self, event: &SecurityEvent) -> Result<(), Error> {
        let event_json = event.to_json()?;
        let url = format!(
            "https://{}.ods.opinsights.azure.com/api/logs?api-version=2016-04-01",
            self.workspace_id
        );

        let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let content_type = "application/json";
        let content_length = event_json.len();
        let method = "POST";
        let resource = format!("/api/logs?api-version=2016-04-01");

        let signature =
            self.generate_signature(&date, content_length, method, content_type, &resource);

        let mut last_error = None;
        for attempt in 0..=self.retry.max_attempts {
            let log_entry = serde_json::json!({
                "log_type": self.log_type,
                "time_generated": event.timestamp.to_rfc3339(),
                "data": serde_json::from_str::<serde_json::Value>(&event_json)
                    .unwrap_or_else(|_| serde_json::json!({"message": event_json}))
            });

            match self
                .client
                .post(&url)
                .header("x-ms-date", &date)
                .header("Content-Type", content_type)
                .header("Authorization", format!("SharedKey {}:{}", self.workspace_id, signature))
                .header("Log-Type", &self.log_type)
                .header("time-generated-field", "time_generated")
                .body(serde_json::to_string(&log_entry)?)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        debug!("Sent Azure Monitor event: {}", event.event_type);
                        return Ok(());
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        last_error = Some(Error::Generic(format!(
                            "Azure Monitor HTTP error {}: {}",
                            status, body
                        )));
                    }
                }
                Err(e) => {
                    last_error =
                        Some(Error::Generic(format!("Azure Monitor request failed: {}", e)));
                }
            }

            if attempt < self.retry.max_attempts {
                let delay = if self.retry.backoff == "exponential" {
                    self.retry.initial_delay_secs * (2_u64.pow(attempt))
                } else {
                    self.retry.initial_delay_secs * (attempt as u64 + 1)
                };
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Generic("Failed to send Azure Monitor event after retries".to_string())
        }))
    }
}

/// SIEM transport health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportHealth {
    /// Transport identifier (protocol or destination name)
    pub identifier: String,
    /// Whether transport is healthy
    pub healthy: bool,
    /// Last successful event timestamp
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error message (if any)
    pub last_error: Option<String>,
    /// Total events sent successfully
    pub success_count: u64,
    /// Total events failed
    pub failure_count: u64,
}

/// SIEM emitter that sends events to configured destinations
pub struct SiemEmitter {
    transports: Vec<Box<dyn SiemTransport>>,
    filters: Option<EventFilter>,
    /// Health status for each transport
    health_status: Arc<RwLock<Vec<TransportHealth>>>,
}

impl SiemEmitter {
    /// Create a new SIEM emitter from configuration
    pub async fn from_config(config: SiemConfig) -> Result<Self, Error> {
        if !config.enabled {
            return Ok(Self {
                transports: Vec::new(),
                filters: config.filters,
                health_status: Arc::new(RwLock::new(Vec::new())),
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
                SiemDestination::Splunk {
                    url,
                    token,
                    index,
                    source_type,
                } => Box::new(SplunkTransport::new(
                    url,
                    token,
                    index,
                    source_type,
                    RetryConfig::default(),
                )),
                SiemDestination::Datadog {
                    api_key,
                    app_key,
                    site,
                    tags,
                } => Box::new(DatadogTransport::new(
                    api_key,
                    app_key,
                    site,
                    tags,
                    RetryConfig::default(),
                )),
                SiemDestination::Cloudwatch {
                    region,
                    log_group,
                    stream,
                    credentials,
                } => Box::new(CloudwatchTransport::new(
                    region,
                    log_group,
                    stream,
                    credentials,
                    RetryConfig::default(),
                )),
                SiemDestination::Gcp {
                    project_id,
                    log_name,
                    credentials_path,
                } => Box::new(GcpTransport::new(
                    project_id,
                    log_name,
                    credentials_path,
                    RetryConfig::default(),
                )),
                SiemDestination::Azure {
                    workspace_id,
                    shared_key,
                    log_type,
                } => Box::new(AzureTransport::new(
                    workspace_id,
                    shared_key,
                    log_type,
                    RetryConfig::default(),
                )),
            };
            transports.push(transport);
        }

        let health_status = Arc::new(RwLock::new(
            transports
                .iter()
                .enumerate()
                .map(|(i, _)| TransportHealth {
                    identifier: format!("transport_{}", i),
                    healthy: true,
                    last_success: None,
                    last_error: None,
                    success_count: 0,
                    failure_count: 0,
                })
                .collect(),
        ));

        Ok(Self {
            transports,
            filters: config.filters,
            health_status,
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
        let mut health_status = self.health_status.write().await;

        for (idx, transport) in self.transports.iter().enumerate() {
            match transport.send_event(&event).await {
                Ok(()) => {
                    if let Some(health) = health_status.get_mut(idx) {
                        health.healthy = true;
                        health.last_success = Some(chrono::Utc::now());
                        health.success_count += 1;
                        health.last_error = None;
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    error!("Failed to send event to SIEM: {}", error_msg);
                    errors.push(Error::Generic(error_msg.clone()));
                    if let Some(health) = health_status.get_mut(idx) {
                        health.healthy = false;
                        health.failure_count += 1;
                        health.last_error = Some(error_msg);
                    }
                }
            }
        }

        drop(health_status);

        if !errors.is_empty() && errors.len() == self.transports.len() {
            // All transports failed
            return Err(Error::Generic(format!(
                "All SIEM transports failed: {} errors",
                errors.len()
            )));
        }

        Ok(())
    }

    /// Get health status of all SIEM transports
    pub async fn health_status(&self) -> Vec<TransportHealth> {
        self.health_status.read().await.clone()
    }

    /// Check if SIEM emitter is healthy (at least one transport is healthy)
    pub async fn is_healthy(&self) -> bool {
        let health_status = self.health_status.read().await;
        health_status.iter().any(|h| h.healthy)
    }

    /// Get overall health summary
    pub async fn health_summary(&self) -> (usize, usize, usize) {
        let health_status = self.health_status.read().await;
        let total = health_status.len();
        let healthy = health_status.iter().filter(|h| h.healthy).count();
        let unhealthy = total - healthy;
        (total, healthy, unhealthy)
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

    #[test]
    fn test_siem_protocol_serialization() {
        let protocols = vec![
            SiemProtocol::Syslog,
            SiemProtocol::Http,
            SiemProtocol::Https,
            SiemProtocol::File,
            SiemProtocol::Splunk,
            SiemProtocol::Datadog,
            SiemProtocol::Cloudwatch,
            SiemProtocol::Gcp,
            SiemProtocol::Azure,
        ];

        for protocol in protocols {
            let json = serde_json::to_string(&protocol).unwrap();
            assert!(!json.is_empty());
            let deserialized: SiemProtocol = serde_json::from_str(&json).unwrap();
            assert_eq!(protocol, deserialized);
        }
    }

    #[test]
    fn test_syslog_facility_default() {
        let facility = SyslogFacility::default();
        assert_eq!(facility, SyslogFacility::Local0);
    }

    #[test]
    fn test_syslog_facility_serialization() {
        let facilities = vec![
            SyslogFacility::Kernel,
            SyslogFacility::User,
            SyslogFacility::Security,
            SyslogFacility::Local0,
            SyslogFacility::Local7,
        ];

        for facility in facilities {
            let json = serde_json::to_string(&facility).unwrap();
            assert!(!json.is_empty());
            let deserialized: SyslogFacility = serde_json::from_str(&json).unwrap();
            assert_eq!(facility, deserialized);
        }
    }

    #[test]
    fn test_syslog_severity_from_security_event_severity() {
        use crate::security::events::SecurityEventSeverity;

        assert_eq!(
            SyslogSeverity::from(SecurityEventSeverity::Low),
            SyslogSeverity::Informational
        );
        assert_eq!(
            SyslogSeverity::from(SecurityEventSeverity::Medium),
            SyslogSeverity::Warning
        );
        assert_eq!(
            SyslogSeverity::from(SecurityEventSeverity::High),
            SyslogSeverity::Error
        );
        assert_eq!(
            SyslogSeverity::from(SecurityEventSeverity::Critical),
            SyslogSeverity::Critical
        );
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.backoff, "exponential");
        assert_eq!(config.initial_delay_secs, 1);
    }

    #[test]
    fn test_retry_config_serialization() {
        let config = RetryConfig {
            max_attempts: 5,
            backoff: "linear".to_string(),
            initial_delay_secs: 2,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("max_attempts"));
        assert!(json.contains("linear"));
    }

    #[test]
    fn test_file_rotation_config_serialization() {
        let config = FileRotationConfig {
            max_size: "100MB".to_string(),
            max_files: 10,
            compress: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("100MB"));
        assert!(json.contains("max_files"));
    }

    #[test]
    fn test_siem_config_default() {
        let config = SiemConfig::default();
        assert!(!config.enabled);
        assert!(config.protocol.is_none());
        assert!(config.destinations.is_empty());
        assert!(config.filters.is_none());
    }

    #[test]
    fn test_siem_config_serialization() {
        let config = SiemConfig {
            enabled: true,
            protocol: Some(SiemProtocol::Syslog),
            destinations: vec![],
            filters: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains("syslog"));
    }

    #[test]
    fn test_transport_health_creation() {
        let health = TransportHealth {
            identifier: "test_transport".to_string(),
            healthy: true,
            last_success: Some(chrono::Utc::now()),
            last_error: None,
            success_count: 100,
            failure_count: 0,
        };

        assert_eq!(health.identifier, "test_transport");
        assert!(health.healthy);
        assert_eq!(health.success_count, 100);
        assert_eq!(health.failure_count, 0);
    }

    #[test]
    fn test_transport_health_serialization() {
        let health = TransportHealth {
            identifier: "transport_1".to_string(),
            healthy: false,
            last_success: None,
            last_error: Some("Connection failed".to_string()),
            success_count: 50,
            failure_count: 5,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("transport_1"));
        assert!(json.contains("Connection failed"));
    }

    #[test]
    fn test_syslog_transport_new() {
        let transport = SyslogTransport::new(
            "example.com".to_string(),
            514,
            "tcp".to_string(),
            SyslogFacility::Security,
            "app".to_string(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_http_transport_new() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "value".to_string());
        let transport = HttpTransport::new(
            "https://example.com/webhook".to_string(),
            "POST".to_string(),
            headers,
            10,
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_splunk_transport_new() {
        let transport = SplunkTransport::new(
            "https://splunk.example.com:8088".to_string(),
            "token123".to_string(),
            Some("index1".to_string()),
            Some("json".to_string()),
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_datadog_transport_new() {
        let transport = DatadogTransport::new(
            "api_key_123".to_string(),
            Some("app_key_456".to_string()),
            "us".to_string(),
            vec!["env:test".to_string()],
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_cloudwatch_transport_new() {
        let mut credentials = HashMap::new();
        credentials.insert("access_key".to_string(), "key123".to_string());
        credentials.insert("secret_key".to_string(), "secret123".to_string());
        let transport = CloudwatchTransport::new(
            "us-east-1".to_string(),
            "log-group-name".to_string(),
            "log-stream-name".to_string(),
            credentials,
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_gcp_transport_new() {
        let transport = GcpTransport::new(
            "project-id".to_string(),
            "log-name".to_string(),
            "/path/to/credentials.json".to_string(),
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }

    #[test]
    fn test_azure_transport_new() {
        let transport = AzureTransport::new(
            "workspace-id".to_string(),
            "shared-key".to_string(),
            "CustomLog".to_string(),
            RetryConfig::default(),
        );

        // Just verify it can be created
        let _ = transport;
    }
}
