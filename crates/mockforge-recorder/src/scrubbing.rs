//! Capture Scrubbing & Deterministic Replay
//!
//! Provides functionality to scrub sensitive data from recorded requests/responses
//! and normalize non-deterministic values for reproducible diffs and replays.
//!
//! # Features
//!
//! - **Regex-based scrubbing**: Remove or replace sensitive data patterns
//! - **Deterministic normalization**: Replace timestamps, UUIDs, and random IDs
//! - **Field-level scrubbing**: Target specific JSON fields or headers
//! - **Capture filtering**: Selectively record based on status codes or patterns
//!
//! # Environment Variables
//!
//! - `MOCKFORGE_CAPTURE_SCRUB`: JSON configuration for scrubbing rules
//! - `MOCKFORGE_CAPTURE_FILTER`: JSON configuration for capture filtering
//! - `MOCKFORGE_CAPTURE_DETERMINISTIC`: Enable deterministic mode (default: false)
//!
//! # Example
//!
//! ```bash
//! export MOCKFORGE_CAPTURE_SCRUB='{
//!   "rules": [
//!     {"type": "field", "field": "email", "replacement": "user@example.com"},
//!     {"type": "uuid", "replacement": "00000000-0000-0000-0000-{{counter:012}}"},
//!     {"type": "email", "replacement": "user{{counter}}@example.com"}
//!   ]
//! }'
//!
//! export MOCKFORGE_CAPTURE_FILTER='{"status_codes": [500, 502, 503, 504]}'
//! export MOCKFORGE_CAPTURE_DETERMINISTIC=true
//! ```
//!
//! `{{counter}}` in the replacement of a built-in rule (`uuid`, `email`,
//! `ip_address`, `credit_card`) is the matched value's pseudonym: the same
//! value is replaced the same way wherever it appears, and two different
//! values never share one. A replacement without it is used for every match.
//!
//! If `MOCKFORGE_CAPTURE_SCRUB` is set but cannot be read, nothing sensitive
//! reaches the recording: everything but the method, the path and the status
//! is redacted until the configuration is fixed.

use crate::{RecordedRequest, RecordedResponse, RecorderError, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

/// Global scrubber instance loaded from environment
static GLOBAL_SCRUBBER: Lazy<Arc<Scrubber>> = Lazy::new(|| {
    Arc::new(Scrubber::from_env().unwrap_or_else(|error| {
        warn!(
            "Failed to load scrubber from environment: {error}. Recording with everything \
             redacted until MOCKFORGE_CAPTURE_SCRUB is fixed."
        );
        Scrubber::redact_everything()
    }))
});

/// Global filter instance loaded from environment
static GLOBAL_FILTER: Lazy<Arc<CaptureFilter>> = Lazy::new(|| {
    Arc::new(CaptureFilter::from_env().unwrap_or_else(|e| {
        warn!("Failed to load capture filter from environment: {}", e);
        CaptureFilter::default()
    }))
});

/// Regex pattern for matching UUIDs
static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap()
});

/// Regex pattern for matching email addresses
static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap());

/// Regex pattern for matching IPv4 addresses
static IPV4_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap());

/// Regex pattern for matching credit card numbers
static CREDIT_CARD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap());

/// Configuration for scrubbing sensitive data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrubConfig {
    /// Rules for scrubbing data
    #[serde(default)]
    pub rules: Vec<ScrubRule>,

    /// Enable deterministic mode (normalize timestamps, IDs, etc.)
    #[serde(default)]
    pub deterministic: bool,

    /// Counter seed for deterministic IDs (used in deterministic mode)
    #[serde(default)]
    pub counter_seed: u64,
}

/// A single scrubbing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ScrubRule {
    /// Scrub by regex pattern
    Regex {
        /// Regex pattern to match
        pattern: String,
        /// Replacement string (can include capture groups like $1, $2)
        replacement: String,
        /// Target location (headers, body, all)
        #[serde(default = "default_target")]
        target: ScrubTarget,
    },

    /// Scrub specific JSON field
    Field {
        /// JSON field path (e.g., "user.email", "data.id")
        field: String,
        /// Replacement value
        replacement: String,
        /// Target location
        #[serde(default = "default_target")]
        target: ScrubTarget,
    },

    /// Scrub specific header
    Header {
        /// Header name (case-insensitive)
        name: String,
        /// Replacement value
        replacement: String,
    },

    /// Scrub all UUIDs (replace with deterministic counter)
    Uuid {
        /// Replacement pattern (use {{counter}} for deterministic counter)
        #[serde(default = "default_uuid_replacement")]
        replacement: String,
    },

    /// Scrub email addresses
    Email {
        /// Replacement value
        #[serde(default = "default_email_replacement")]
        replacement: String,
    },

    /// Scrub IP addresses
    IpAddress {
        /// Replacement value
        #[serde(default = "default_ip_replacement")]
        replacement: String,
    },

    /// Scrub credit card numbers
    CreditCard {
        /// Replacement value
        #[serde(default = "default_creditcard_replacement")]
        replacement: String,
    },
}

/// Target location for scrubbing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScrubTarget {
    /// Scrub in headers only
    Headers,
    /// Scrub in body only
    Body,
    /// Scrub in both headers and body
    All,
}

fn default_target() -> ScrubTarget {
    ScrubTarget::All
}

fn default_uuid_replacement() -> String {
    "00000000-0000-0000-0000-{{counter:012}}".to_string()
}

fn default_email_replacement() -> String {
    "user@example.com".to_string()
}

fn default_ip_replacement() -> String {
    "127.0.0.1".to_string()
}

fn default_creditcard_replacement() -> String {
    "XXXX-XXXX-XXXX-XXXX".to_string()
}

/// Configuration for filtering which requests to capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureFilterConfig {
    /// Only capture requests with these status codes
    #[serde(default)]
    pub status_codes: Vec<u16>,

    /// Only capture requests matching these patterns (regex)
    #[serde(default)]
    pub path_patterns: Vec<String>,

    /// Only capture requests with these methods
    #[serde(default)]
    pub methods: Vec<String>,

    /// Exclude requests matching these patterns
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    /// Only capture errors (status >= 400)
    #[serde(default)]
    pub errors_only: bool,

    /// Capture sample rate (0.0 - 1.0, e.g., 0.1 = 10%)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    1.0
}

impl Default for CaptureFilterConfig {
    fn default() -> Self {
        Self {
            status_codes: Vec::new(),
            path_patterns: Vec::new(),
            methods: Vec::new(),
            exclude_paths: Vec::new(),
            errors_only: false,
            sample_rate: default_sample_rate(),
        }
    }
}

/// How many values a scrubber remembers pseudonyms for. Beyond this a new
/// value still gets its own number, but is not remembered, so it may get
/// another one the next time it appears.
const DEFAULT_PSEUDONYM_CACHE: usize = 100_000;

/// What a value is replaced with when the scrubbing configuration could not
/// be read and everything is redacted.
const REDACTED: &str = "[redacted]";

/// The body recorded instead of the real one when everything is redacted. It
/// says why, so a recording explains itself.
const REDACTED_BODY: &str =
    "[redacted: MOCKFORGE_CAPTURE_SCRUB could not be read. Fix it to record bodies.]";

/// The built-in rules that give a value a pseudonym. A value's number is
/// remembered per rule, so an address and an identifier never share one.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Pseudonym {
    Uuid,
    Email,
    IpAddress,
    CreditCard,
}

/// Handles scrubbing of sensitive data from recorded requests/responses
pub struct Scrubber {
    config: ScrubConfig,
    compiled_regexes: Vec<(Regex, String, ScrubTarget)>,
    deterministic_counter: std::sync::atomic::AtomicU64,
    /// The values that already have a pseudonym, so a value is replaced the
    /// same way wherever it appears.
    pseudonyms: Mutex<HashMap<(Pseudonym, String), u64>>,
    /// How many values may be remembered at once.
    pseudonym_cache: usize,
    /// Set when the configuration could not be read: then everything is
    /// redacted rather than nothing.
    redact_all: bool,
}

impl Default for Scrubber {
    fn default() -> Self {
        Self {
            config: ScrubConfig::default(),
            compiled_regexes: Vec::new(),
            deterministic_counter: std::sync::atomic::AtomicU64::new(0),
            pseudonyms: Mutex::new(HashMap::new()),
            pseudonym_cache: DEFAULT_PSEUDONYM_CACHE,
            redact_all: false,
        }
    }
}

impl Scrubber {
    /// A scrubber for when the configuration could not be read. It keeps the
    /// method, the path and the status, which routing needs, and redacts the
    /// rest, so an unreadable configuration never records raw data.
    pub fn redact_everything() -> Self {
        Self {
            redact_all: true,
            ..Self::default()
        }
    }

    /// Whether everything is redacted because the configuration could not be
    /// read.
    pub fn redacts_everything(&self) -> bool {
        self.redact_all
    }
    /// Create a new scrubber with the given configuration
    pub fn new(config: ScrubConfig) -> Result<Self> {
        let mut compiled_regexes = Vec::new();

        // Compile regex patterns
        for rule in &config.rules {
            if let ScrubRule::Regex {
                pattern,
                replacement,
                target,
            } = rule
            {
                let regex = Regex::new(pattern).map_err(|e| {
                    RecorderError::InvalidFilter(format!(
                        "Invalid regex pattern '{}': {}",
                        pattern, e
                    ))
                })?;
                compiled_regexes.push((regex, replacement.clone(), *target));
            }
        }

        Ok(Self {
            deterministic_counter: std::sync::atomic::AtomicU64::new(config.counter_seed),
            config,
            compiled_regexes,
            ..Self::default()
        })
    }

    /// A scrubber that remembers pseudonyms for at most `cache` values.
    #[cfg(test)]
    fn with_pseudonym_cache(config: ScrubConfig, cache: usize) -> Result<Self> {
        Ok(Self {
            pseudonym_cache: cache,
            ..Self::new(config)?
        })
    }

    /// Load scrubber from MOCKFORGE_CAPTURE_SCRUB environment variable
    pub fn from_env() -> Result<Self> {
        let scrub_json = std::env::var("MOCKFORGE_CAPTURE_SCRUB").ok();
        let deterministic = std::env::var("MOCKFORGE_CAPTURE_DETERMINISTIC")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let mut config = if let Some(json) = scrub_json {
            serde_json::from_str::<ScrubConfig>(&json).map_err(|e| {
                RecorderError::InvalidFilter(format!("Invalid MOCKFORGE_CAPTURE_SCRUB JSON: {}", e))
            })?
        } else {
            ScrubConfig::default()
        };

        config.deterministic = deterministic;

        Self::new(config)
    }

    /// Get the global scrubber instance
    pub fn global() -> Arc<Self> {
        Arc::clone(&GLOBAL_SCRUBBER)
    }

    /// Scrub a recorded request
    pub fn scrub_request(&self, request: &mut RecordedRequest) {
        if self.redact_all {
            redact_headers(&mut request.headers);
            if request.body.is_some() {
                request.body = Some(REDACTED_BODY.to_string());
                request.body_encoding = "utf8".to_string();
            }
            if let Some(ref mut query) = request.query_params {
                *query = REDACTED.to_string();
            }
            if let Some(ref mut value) = request.trace_id {
                *value = REDACTED.to_string();
            }
            if let Some(ref mut value) = request.span_id {
                *value = REDACTED.to_string();
            }
            if let Some(ref mut value) = request.client_ip {
                *value = REDACTED.to_string();
            }
            return;
        }

        // Scrub headers
        if let Ok(mut headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            self.scrub_headers(&mut headers);
            if let Ok(json) = serde_json::to_string(&headers) {
                request.headers = json;
            }
        }

        // Scrub body
        if let Some(ref mut body) = request.body {
            if request.body_encoding == "utf8" {
                *body = self.scrub_string(body, ScrubTarget::Body);
            }
        }

        // Scrub query params
        if let Some(ref mut query) = request.query_params {
            *query = self.scrub_string(query, ScrubTarget::Body);
        }

        // Normalize timestamp in deterministic mode
        if self.config.deterministic {
            request.timestamp = Self::normalize_timestamp(request.timestamp);
        }

        // Scrub sensitive fields
        if let Some(ref mut trace_id) = request.trace_id {
            *trace_id = self.scrub_string(trace_id, ScrubTarget::All);
        }
        if let Some(ref mut span_id) = request.span_id {
            *span_id = self.scrub_string(span_id, ScrubTarget::All);
        }
        if let Some(ref mut client_ip) = request.client_ip {
            *client_ip = self.scrub_string(client_ip, ScrubTarget::All);
        }
    }

    /// Scrub a recorded response
    pub fn scrub_response(&self, response: &mut RecordedResponse) {
        if self.redact_all {
            redact_headers(&mut response.headers);
            if response.body.is_some() {
                response.body = Some(REDACTED_BODY.to_string());
                response.body_encoding = "utf8".to_string();
            }
            return;
        }

        // Scrub headers
        if let Ok(mut headers) = serde_json::from_str::<HashMap<String, String>>(&response.headers)
        {
            self.scrub_headers(&mut headers);
            if let Ok(json) = serde_json::to_string(&headers) {
                response.headers = json;
            }
        }

        // Scrub body
        if let Some(ref mut body) = response.body {
            if response.body_encoding == "utf8" {
                *body = self.scrub_string(body, ScrubTarget::Body);
            }
        }

        // Normalize timestamp in deterministic mode
        if self.config.deterministic {
            response.timestamp = Self::normalize_timestamp(response.timestamp);
        }
    }

    /// Scrub headers map
    fn scrub_headers(&self, headers: &mut HashMap<String, String>) {
        for rule in &self.config.rules {
            if let ScrubRule::Header { name, replacement } = rule {
                // Case-insensitive header matching
                let key = headers.keys().find(|k| k.eq_ignore_ascii_case(name)).cloned();
                if let Some(key) = key {
                    headers.insert(key, replacement.clone());
                }
            }
        }

        // Scrub header values with regex rules
        for (key, value) in headers.iter_mut() {
            *value = self.scrub_string(value, ScrubTarget::Headers);

            // Also scrub by field name
            for rule in &self.config.rules {
                if let ScrubRule::Field {
                    field,
                    replacement,
                    target,
                } = rule
                {
                    if (*target == ScrubTarget::Headers || *target == ScrubTarget::All)
                        && key.eq_ignore_ascii_case(field)
                    {
                        *value = replacement.clone();
                    }
                }
            }
        }
    }

    /// Scrub a string value
    fn scrub_string(&self, input: &str, location: ScrubTarget) -> String {
        let mut result = input.to_string();

        // Apply built-in rules
        for rule in &self.config.rules {
            match rule {
                ScrubRule::Uuid { replacement } => {
                    if location == ScrubTarget::All || location == ScrubTarget::Body {
                        result = self.scrub_uuids(&result, replacement);
                    }
                }
                ScrubRule::Email { replacement } => {
                    if location == ScrubTarget::All || location == ScrubTarget::Body {
                        result = self.scrub_emails(&result, replacement);
                    }
                }
                ScrubRule::IpAddress { replacement } => {
                    if location == ScrubTarget::All || location == ScrubTarget::Body {
                        result = self.scrub_ips(&result, replacement);
                    }
                }
                ScrubRule::CreditCard { replacement } => {
                    if location == ScrubTarget::All || location == ScrubTarget::Body {
                        result = self.scrub_credit_cards(&result, replacement);
                    }
                }
                ScrubRule::Field {
                    field,
                    replacement,
                    target,
                } if *target == location || *target == ScrubTarget::All => {
                    result = self.scrub_json_field(&result, field, replacement);
                }
                _ => {}
            }
        }

        // Apply regex rules
        for (regex, replacement, target) in &self.compiled_regexes {
            if *target == location || *target == ScrubTarget::All {
                result = regex.replace_all(&result, replacement.as_str()).to_string();
            }
        }

        result
    }

    /// Scrub UUIDs. Each UUID keeps its pseudonym wherever it appears.
    fn scrub_uuids(&self, input: &str, replacement: &str) -> String {
        // A UUID is the same UUID in either case, so both get one pseudonym.
        self.scrub_pattern(&UUID_REGEX, Pseudonym::Uuid, input, replacement, true)
    }

    /// Scrub email addresses
    fn scrub_emails(&self, input: &str, replacement: &str) -> String {
        self.scrub_pattern(&EMAIL_REGEX, Pseudonym::Email, input, replacement, false)
    }

    /// Scrub IP addresses
    fn scrub_ips(&self, input: &str, replacement: &str) -> String {
        self.scrub_pattern(&IPV4_REGEX, Pseudonym::IpAddress, input, replacement, false)
    }

    /// Scrub credit card numbers
    fn scrub_credit_cards(&self, input: &str, replacement: &str) -> String {
        self.scrub_pattern(&CREDIT_CARD_REGEX, Pseudonym::CreditCard, input, replacement, false)
    }

    /// Replace every match of `pattern` in `input`.
    fn scrub_pattern(
        &self,
        pattern: &Regex,
        kind: Pseudonym,
        input: &str,
        replacement: &str,
        fold_case: bool,
    ) -> String {
        pattern
            .replace_all(input, |found: &regex::Captures| {
                let value = if fold_case {
                    found[0].to_ascii_lowercase()
                } else {
                    found[0].to_string()
                };
                self.pseudonymize(kind, &value, replacement)
            })
            .to_string()
    }

    /// Replace one matched value. A replacement with `{{counter}}` or
    /// `{{counter:WIDTH}}` becomes the value's pseudonym, zero-padded to the
    /// width when one is given; any other replacement is used as it is, for
    /// every match.
    fn pseudonymize(&self, kind: Pseudonym, value: &str, replacement: &str) -> String {
        if !replacement.contains("{{counter") {
            return replacement.to_string();
        }
        Self::render_counter(replacement, self.number_for(kind, value))
    }

    /// A replacement with every `{{counter}}` and `{{counter:WIDTH}}` filled
    /// in with `number`.
    fn render_counter(replacement: &str, number: u64) -> String {
        const MARKER: &str = "{{counter";
        let mut rendered = String::with_capacity(replacement.len());
        let mut rest = replacement;
        while let Some(at) = rest.find(MARKER) {
            rendered.push_str(&rest[..at]);
            let after = &rest[at + MARKER.len()..];
            let Some(end) = after.find("}}") else {
                rendered.push_str(MARKER);
                rest = after;
                continue;
            };
            let inside = &after[..end];
            let width = inside.strip_prefix(':').and_then(|width| width.parse::<usize>().ok());
            if !inside.is_empty() && width.is_none() {
                // Text that only starts like a marker, such as `{{counterfeit}}`.
                rendered.push_str(MARKER);
                rest = after;
                continue;
            }
            match width {
                Some(width) => rendered.push_str(&format!("{number:0width$}")),
                None => rendered.push_str(&number.to_string()),
            }
            rest = &after[end + 2..];
        }
        rendered.push_str(rest);
        rendered
    }

    /// The number a value is replaced with. A value keeps the number it was
    /// given the first time it was seen, so it reads the same everywhere.
    fn number_for(&self, kind: Pseudonym, value: &str) -> u64 {
        let key = (kind, value.to_string());
        let mut pseudonyms =
            self.pseudonyms.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(number) = pseudonyms.get(&key) {
            return *number;
        }
        let number = self.deterministic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if pseudonyms.len() < self.pseudonym_cache {
            pseudonyms.insert(key, number);
        }
        number
    }

    /// Scrub specific JSON field
    fn scrub_json_field(&self, input: &str, field_path: &str, replacement: &str) -> String {
        // Try to parse as JSON
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(input) {
            if self.scrub_json_value(&mut json, field_path, replacement) {
                if let Ok(result) = serde_json::to_string(&json) {
                    return result;
                }
            }
        }
        input.to_string()
    }

    /// Recursively scrub JSON value
    #[allow(clippy::only_used_in_recursion)]
    fn scrub_json_value(
        &self,
        value: &mut serde_json::Value,
        field_path: &str,
        replacement: &str,
    ) -> bool {
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.is_empty() {
            return false;
        }

        if parts.len() == 1 {
            // Base case: scrub this field
            if let Some(obj) = value.as_object_mut() {
                if obj.contains_key(parts[0]) {
                    obj.insert(
                        parts[0].to_string(),
                        serde_json::Value::String(replacement.to_string()),
                    );
                    return true;
                }
            }
        } else {
            // Recursive case: navigate deeper
            if let Some(obj) = value.as_object_mut() {
                if let Some(child) = obj.get_mut(parts[0]) {
                    let remaining = parts[1..].join(".");
                    return self.scrub_json_value(child, &remaining, replacement);
                }
            }
        }

        false
    }

    /// Normalize timestamp to a deterministic value
    fn normalize_timestamp(timestamp: DateTime<Utc>) -> DateTime<Utc> {
        // Normalize to start of day
        timestamp
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("0 is valid for hours/minutes/seconds")
            .and_utc()
    }
}

/// Replace every header value, keeping the names an exchange is understood
/// by. Headers that are not a JSON object are emptied.
fn redact_headers(headers: &mut String) {
    let redacted = match serde_json::from_str::<HashMap<String, String>>(headers) {
        Ok(map) => {
            let redacted: HashMap<String, String> =
                map.keys().map(|name| (name.clone(), REDACTED.to_string())).collect();
            serde_json::to_string(&redacted)
        }
        Err(_) => Ok("{}".to_string()),
    };
    if let Ok(redacted) = redacted {
        *headers = redacted;
    }
}

/// Handles filtering of which requests to capture
#[derive(Default)]
pub struct CaptureFilter {
    config: CaptureFilterConfig,
    path_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
}

impl CaptureFilter {
    /// Create a new capture filter with the given configuration
    pub fn new(config: CaptureFilterConfig) -> Result<Self> {
        let mut path_patterns = Vec::new();
        for pattern in &config.path_patterns {
            let regex = Regex::new(pattern).map_err(|e| {
                RecorderError::InvalidFilter(format!("Invalid path pattern '{}': {}", pattern, e))
            })?;
            path_patterns.push(regex);
        }

        let mut exclude_patterns = Vec::new();
        for pattern in &config.exclude_paths {
            let regex = Regex::new(pattern).map_err(|e| {
                RecorderError::InvalidFilter(format!(
                    "Invalid exclude pattern '{}': {}",
                    pattern, e
                ))
            })?;
            exclude_patterns.push(regex);
        }

        Ok(Self {
            config,
            path_patterns,
            exclude_patterns,
        })
    }

    /// Load filter from MOCKFORGE_CAPTURE_FILTER environment variable
    pub fn from_env() -> Result<Self> {
        let filter_json = std::env::var("MOCKFORGE_CAPTURE_FILTER").ok();

        let config = if let Some(json) = filter_json {
            serde_json::from_str::<CaptureFilterConfig>(&json).map_err(|e| {
                RecorderError::InvalidFilter(format!(
                    "Invalid MOCKFORGE_CAPTURE_FILTER JSON: {}",
                    e
                ))
            })?
        } else {
            CaptureFilterConfig::default()
        };

        Self::new(config)
    }

    /// Get the global filter instance
    pub fn global() -> Arc<Self> {
        Arc::clone(&GLOBAL_FILTER)
    }

    /// Check if a request should be captured
    pub fn should_capture(&self, method: &str, path: &str, status_code: Option<u16>) -> bool {
        debug!(
            "should_capture called: method={}, path={}, status_code={:?}",
            method, path, status_code
        );
        debug!("  errors_only={}, status_codes={:?}, path_patterns count={}, exclude_patterns count={}",
               self.config.errors_only, self.config.status_codes, self.path_patterns.len(), self.exclude_patterns.len());

        // Check sample rate
        if self.config.sample_rate < 1.0 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            let hash = hasher.finish();
            let sample = (hash % 1000) as f64 / 1000.0;

            if sample > self.config.sample_rate {
                debug!(
                    "Skipping capture due to sample rate: {} > {}",
                    sample, self.config.sample_rate
                );
                return false;
            }
        }

        // Check errors_only
        if self.config.errors_only {
            if let Some(code) = status_code {
                if code < 400 {
                    debug!("Skipping capture: not an error (status {})", code);
                    return false;
                }
            } else {
                // No status code yet, we can't determine if it's an error
                // In this case, we should allow it and filter later when we have the status
                debug!("errors_only is set but no status code provided, allowing for now");
            }
        }

        // Check status code filter
        if !self.config.status_codes.is_empty() {
            if let Some(code) = status_code {
                if !self.config.status_codes.contains(&code) {
                    debug!("Skipping capture: status code {} not in filter", code);
                    return false;
                }
            } else {
                // No status code yet, allow it and filter later
                debug!("status_codes filter set but no status code provided, allowing for now");
            }
        }

        // Check method filter
        if !self.config.methods.is_empty()
            && !self.config.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
        {
            debug!("Skipping capture: method {} not in filter", method);
            return false;
        }

        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if pattern.is_match(path) {
                debug!("Skipping capture: path {} matches exclude pattern", path);
                return false;
            }
        }

        // Check path patterns (if specified, path must match)
        if !self.path_patterns.is_empty() {
            let matches = self.path_patterns.iter().any(|p| p.is_match(path));
            if !matches {
                debug!("Skipping capture: path {} does not match any pattern", path);
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ScrubConfig Tests ====================

    #[test]
    fn test_scrub_config_default() {
        let config = ScrubConfig::default();
        assert!(config.rules.is_empty());
        assert!(!config.deterministic);
        assert_eq!(config.counter_seed, 0);
    }

    #[test]
    fn test_scrub_config_serialize() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Email {
                replacement: "user@example.com".to_string(),
            }],
            deterministic: true,
            counter_seed: 100,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("email"));
        assert!(json.contains("deterministic"));
    }

    // ==================== ScrubTarget Tests ====================

    #[test]
    fn test_scrub_target_equality() {
        assert_eq!(ScrubTarget::All, ScrubTarget::All);
        assert_ne!(ScrubTarget::Headers, ScrubTarget::Body);
    }

    #[test]
    fn test_scrub_target_default() {
        assert_eq!(default_target(), ScrubTarget::All);
    }

    // ==================== ScrubRule Tests ====================

    #[test]
    fn test_scrub_rule_regex_serialize() {
        let rule = ScrubRule::Regex {
            pattern: r"\d+".to_string(),
            replacement: "XXX".to_string(),
            target: ScrubTarget::Body,
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("regex"));
        assert!(json.contains("\\\\d+"));
    }

    #[test]
    fn test_scrub_rule_header_serialize() {
        let rule = ScrubRule::Header {
            name: "Authorization".to_string(),
            replacement: "Bearer ***".to_string(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("header"));
        assert!(json.contains("Authorization"));
    }

    // ==================== Scrubber Tests ====================

    #[test]
    fn test_scrubber_default() {
        let scrubber = Scrubber::default();
        assert!(scrubber.config.rules.is_empty());
        assert!(scrubber.compiled_regexes.is_empty());
    }

    #[test]
    fn test_scrub_email() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Email {
                replacement: "user@example.com".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = r#"{"email": "john.doe@company.com", "name": "John"}"#;
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("user@example.com"));
        assert!(!result.contains("john.doe@company.com"));
    }

    #[test]
    fn test_scrub_multiple_emails() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Email {
                replacement: "redacted@example.com".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Contact: john@test.com and jane@test.org";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert_eq!(result.matches("redacted@example.com").count(), 2);
    }

    #[test]
    fn test_scrub_uuid() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Uuid {
                replacement: "00000000-0000-0000-0000-{{counter:012}}".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Request ID: 123e4567-e89b-12d3-a456-426614174000";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("00000000-0000-0000-0000-000000000000"));
        assert!(!result.contains("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn test_scrub_uuid_counter_increments() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Uuid {
                replacement: "00000000-0000-0000-0000-{{counter:012}}".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input1 = "ID: 123e4567-e89b-12d3-a456-426614174000";
        let input2 = "ID: abc12345-e89b-12d3-a456-426614174000";

        let result1 = scrubber.scrub_string(input1, ScrubTarget::All);
        let result2 = scrubber.scrub_string(input2, ScrubTarget::All);

        assert!(result1.contains("000000000000"));
        assert!(result2.contains("000000000001"));
    }

    fn uuid_scrubber(replacement: &str, seed: u64) -> Scrubber {
        Scrubber::new(ScrubConfig {
            rules: vec![ScrubRule::Uuid {
                replacement: replacement.to_string(),
            }],
            deterministic: false,
            counter_seed: seed,
        })
        .unwrap()
    }

    const USER: &str = "123e4567-e89b-12d3-a456-426614174000";
    const ORDER: &str = "abc12345-e89b-12d3-a456-426614174000";

    #[test]
    fn test_the_same_uuid_keeps_its_pseudonym() {
        let scrubber = uuid_scrubber("00000000-0000-0000-0000-{{counter:012}}", 0);
        let first = scrubber.scrub_string(&format!("user {USER}"), ScrubTarget::All);
        let again = scrubber.scrub_string(&format!("order for {USER}"), ScrubTarget::All);
        let other = scrubber.scrub_string(&format!("user {ORDER}"), ScrubTarget::All);
        let upper =
            scrubber.scrub_string(&format!("user {}", USER.to_uppercase()), ScrubTarget::All);
        assert_eq!(first, "user 00000000-0000-0000-0000-000000000000");
        assert_eq!(
            again, "order for 00000000-0000-0000-0000-000000000000",
            "the same identifier reads the same wherever it appears"
        );
        assert_eq!(other, "user 00000000-0000-0000-0000-000000000001");
        assert_eq!(upper, first, "a UUID in upper case is the same UUID");
    }

    #[test]
    fn test_every_builtin_rule_can_give_distinct_pseudonyms() {
        let scrubber = Scrubber::new(ScrubConfig {
            rules: vec![
                ScrubRule::Email {
                    replacement: "user{{counter}}@example.com".to_string(),
                },
                ScrubRule::IpAddress {
                    replacement: "10.0.0.{{counter}}".to_string(),
                },
                ScrubRule::CreditCard {
                    replacement: "XXXX-XXXX-XXXX-{{counter:04}}".to_string(),
                },
            ],
            deterministic: false,
            counter_seed: 0,
        })
        .unwrap();
        let first = scrubber.scrub_string(
            "john@test.com paid with 4111-1111-1111-1111 from 203.0.113.7",
            ScrubTarget::All,
        );
        let again = scrubber.scrub_string(
            "jane@test.org paid with 5555-5555-5555-5555 from 203.0.113.7 for john@test.com",
            ScrubTarget::All,
        );
        assert_eq!(first, "user0@example.com paid with XXXX-XXXX-XXXX-0002 from 10.0.0.1");
        assert_eq!(
            again,
            "user3@example.com paid with XXXX-XXXX-XXXX-0004 from 10.0.0.1 for user0@example.com",
            "the address, the payer and the card are told apart, and stay the same"
        );
    }

    #[test]
    fn test_a_replacement_without_a_counter_is_used_for_every_match() {
        let scrubber = uuid_scrubber("00000000-0000-0000-0000-000000000000", 0);
        let first = scrubber.scrub_string(USER, ScrubTarget::All);
        let second = scrubber.scrub_string(ORDER, ScrubTarget::All);
        assert_eq!(first, "00000000-0000-0000-0000-000000000000");
        assert_eq!(
            first, second,
            "a constant replacement still collapses every value, as configured"
        );
        assert_eq!(
            scrubber.deterministic_counter.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "no number is spent on a constant replacement"
        );
    }

    #[test]
    fn test_numbers_start_from_the_counter_seed() {
        let scrubber = uuid_scrubber("{{counter}}", 100);
        assert_eq!(scrubber.scrub_string(&format!("{USER} {ORDER}"), ScrubTarget::All), "100 101");
    }

    #[test]
    fn test_beyond_the_cache_a_value_is_not_remembered() {
        let scrubber = Scrubber::with_pseudonym_cache(
            ScrubConfig {
                rules: vec![ScrubRule::Uuid {
                    replacement: "{{counter}}".to_string(),
                }],
                deterministic: false,
                counter_seed: 0,
            },
            1,
        )
        .unwrap();
        assert_eq!(scrubber.scrub_string(USER, ScrubTarget::All), "0");
        assert_eq!(
            scrubber.scrub_string(ORDER, ScrubTarget::All),
            "1",
            "a value past the cache still gets its own number"
        );
        assert_eq!(scrubber.scrub_string(ORDER, ScrubTarget::All), "2", "but is not remembered");
        assert_eq!(
            scrubber.scrub_string(USER, ScrubTarget::All),
            "0",
            "while a remembered one keeps it"
        );
    }

    #[test]
    fn test_one_uuid_gets_one_pseudonym_when_scrubbed_from_many_threads() {
        // Each thread sees the same two identifiers, half of them in the
        // other order, and reports them in a fixed order.
        let scrubber = Arc::new(uuid_scrubber("{{counter}}", 0));
        let handles: Vec<_> = (0..8)
            .map(|thread| {
                let scrubber = Arc::clone(&scrubber);
                std::thread::spawn(move || {
                    let scrub = |value: &str| scrubber.scrub_string(value, ScrubTarget::All);
                    if thread % 2 == 0 {
                        (scrub(USER), scrub(ORDER))
                    } else {
                        let order = scrub(ORDER);
                        (scrub(USER), order)
                    }
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
        assert!(results.iter().all(|pair| pair == &results[0]), "{results:?}");
        assert_ne!(results[0].0, results[0].1);
    }

    #[test]
    fn test_a_counter_can_have_any_width() {
        assert_eq!(Scrubber::render_counter("{{counter}}", 42), "42");
        assert_eq!(Scrubber::render_counter("{{counter:012}}", 42), "000000000042");
        assert_eq!(Scrubber::render_counter("id-{{counter:04}}", 42), "id-0042");
        assert_eq!(Scrubber::render_counter("{{counter:6}}/{{counter}}", 42), "000042/42");
        // Text that only looks like a marker is left alone.
        assert_eq!(
            Scrubber::render_counter("{{counterfeit}} {{counter", 42),
            "{{counterfeit}} {{counter"
        );
    }

    #[test]
    fn test_only_an_unreadable_configuration_redacts_everything() {
        assert!(!Scrubber::default().redacts_everything());
        assert!(!uuid_scrubber("{{counter}}", 0).redacts_everything());
        assert!(Scrubber::redact_everything().redacts_everything());
        assert!(
            REDACTED_BODY.contains("MOCKFORGE_CAPTURE_SCRUB"),
            "a recording says why it is redacted"
        );
    }

    #[test]
    fn test_scrub_ip_address() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::IpAddress {
                replacement: "127.0.0.1".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Client IP: 192.168.1.100";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("127.0.0.1"));
        assert!(!result.contains("192.168.1.100"));
    }

    #[test]
    fn test_scrub_credit_card() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::CreditCard {
                replacement: "XXXX-XXXX-XXXX-XXXX".to_string(),
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Card: 1234-5678-9012-3456";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("XXXX-XXXX-XXXX-XXXX"));
        assert!(!result.contains("1234-5678-9012-3456"));
    }

    #[test]
    fn test_scrub_json_field() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Field {
                field: "user.email".to_string(),
                replacement: "redacted@example.com".to_string(),
                target: ScrubTarget::All,
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = r#"{"user": {"email": "secret@company.com", "name": "John"}}"#;
        let result = scrubber.scrub_string(input, ScrubTarget::Body);

        assert!(result.contains("redacted@example.com"));
        assert!(!result.contains("secret@company.com"));
    }

    #[test]
    fn test_scrub_json_field_top_level() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Field {
                field: "email".to_string(),
                replacement: "redacted".to_string(),
                target: ScrubTarget::Body,
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = r#"{"email": "secret@test.com"}"#;
        let result = scrubber.scrub_string(input, ScrubTarget::Body);

        assert!(result.contains("redacted"));
    }

    #[test]
    fn test_scrub_regex_pattern() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Regex {
                pattern: r"secret-\w+".to_string(),
                replacement: "secret-REDACTED".to_string(),
                target: ScrubTarget::All,
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Token: secret-abc123";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("secret-REDACTED"));
        assert!(!result.contains("secret-abc123"));
    }

    #[test]
    fn test_scrub_regex_invalid_pattern() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Regex {
                pattern: r"[invalid".to_string(),
                replacement: "x".to_string(),
                target: ScrubTarget::All,
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let result = Scrubber::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_scrub_target_body_only() {
        let config = ScrubConfig {
            rules: vec![ScrubRule::Regex {
                pattern: r"test".to_string(),
                replacement: "REDACTED".to_string(),
                target: ScrubTarget::Body,
            }],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let result_body = scrubber.scrub_string("test data", ScrubTarget::Body);
        let result_headers = scrubber.scrub_string("test data", ScrubTarget::Headers);

        assert_eq!(result_body, "REDACTED data");
        assert_eq!(result_headers, "test data"); // Should not be scrubbed
    }

    #[test]
    fn test_scrub_multiple_rules() {
        let config = ScrubConfig {
            rules: vec![
                ScrubRule::Email {
                    replacement: "user@example.com".to_string(),
                },
                ScrubRule::IpAddress {
                    replacement: "0.0.0.0".to_string(),
                },
            ],
            deterministic: false,
            counter_seed: 0,
        };

        let scrubber = Scrubber::new(config).unwrap();
        let input = "Email: john@test.com, IP: 192.168.1.1";
        let result = scrubber.scrub_string(input, ScrubTarget::All);

        assert!(result.contains("user@example.com"));
        assert!(result.contains("0.0.0.0"));
    }

    // ==================== CaptureFilterConfig Tests ====================

    #[test]
    fn test_capture_filter_config_default() {
        let config = CaptureFilterConfig::default();
        assert!(config.status_codes.is_empty());
        assert!(config.path_patterns.is_empty());
        assert!(config.methods.is_empty());
        assert!(config.exclude_paths.is_empty());
        assert!(!config.errors_only);
        assert_eq!(config.sample_rate, 1.0);
    }

    // ==================== CaptureFilter Tests ====================

    #[test]
    fn test_capture_filter_default() {
        let filter = CaptureFilter::default();
        // Default should capture everything
        assert!(filter.should_capture("GET", "/api/test", Some(200)));
    }

    #[test]
    fn test_capture_filter_status_code() {
        let config = CaptureFilterConfig {
            status_codes: vec![500, 502, 503],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("GET", "/api/test", Some(500)));
        assert!(filter.should_capture("POST", "/api/test", Some(502)));
        assert!(!filter.should_capture("GET", "/api/test", Some(200)));
        assert!(!filter.should_capture("GET", "/api/test", Some(404)));
    }

    #[test]
    fn test_capture_filter_status_code_without_status() {
        let config = CaptureFilterConfig {
            status_codes: vec![500],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();
        // Should allow when no status provided (filter later)
        assert!(filter.should_capture("GET", "/api/test", None));
    }

    #[test]
    fn test_capture_filter_errors_only() {
        let config = CaptureFilterConfig {
            errors_only: true,
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("GET", "/api/test", Some(400)));
        assert!(filter.should_capture("GET", "/api/test", Some(500)));
        assert!(!filter.should_capture("GET", "/api/test", Some(200)));
        assert!(!filter.should_capture("GET", "/api/test", Some(304)));
    }

    #[test]
    fn test_capture_filter_path_pattern() {
        let config = CaptureFilterConfig {
            path_patterns: vec![r"^/api/v1/.*".to_string()],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("GET", "/api/v1/users", None));
        assert!(filter.should_capture("POST", "/api/v1/orders", None));
        assert!(!filter.should_capture("GET", "/api/v2/users", None));
        assert!(!filter.should_capture("GET", "/health", None));
    }

    #[test]
    fn test_capture_filter_multiple_path_patterns() {
        let config = CaptureFilterConfig {
            path_patterns: vec![r"^/api/v1/.*".to_string(), r"^/internal/.*".to_string()],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("GET", "/api/v1/users", None));
        assert!(filter.should_capture("GET", "/internal/admin", None));
        assert!(!filter.should_capture("GET", "/public/docs", None));
    }

    #[test]
    fn test_capture_filter_exclude() {
        let config = CaptureFilterConfig {
            exclude_paths: vec![r"/health".to_string(), r"/metrics".to_string()],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("GET", "/api/users", None));
        assert!(!filter.should_capture("GET", "/health", None));
        assert!(!filter.should_capture("GET", "/metrics", None));
    }

    #[test]
    fn test_capture_filter_methods() {
        let config = CaptureFilterConfig {
            methods: vec!["POST".to_string(), "PUT".to_string()],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("POST", "/api/users", None));
        assert!(filter.should_capture("PUT", "/api/users/1", None));
        assert!(!filter.should_capture("GET", "/api/users", None));
        assert!(!filter.should_capture("DELETE", "/api/users/1", None));
    }

    #[test]
    fn test_capture_filter_methods_case_insensitive() {
        let config = CaptureFilterConfig {
            methods: vec!["POST".to_string()],
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        assert!(filter.should_capture("POST", "/api/users", None));
        assert!(filter.should_capture("post", "/api/users", None));
        assert!(filter.should_capture("Post", "/api/users", None));
    }

    #[test]
    fn test_capture_filter_invalid_path_pattern() {
        let config = CaptureFilterConfig {
            path_patterns: vec![r"[invalid".to_string()],
            ..Default::default()
        };

        let result = CaptureFilter::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_capture_filter_invalid_exclude_pattern() {
        let config = CaptureFilterConfig {
            exclude_paths: vec![r"[invalid".to_string()],
            ..Default::default()
        };

        let result = CaptureFilter::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_capture_filter_combined_filters() {
        let config = CaptureFilterConfig {
            path_patterns: vec![r"^/api/.*".to_string()],
            methods: vec!["POST".to_string()],
            errors_only: true,
            ..Default::default()
        };

        let filter = CaptureFilter::new(config).unwrap();

        // Must match all conditions
        assert!(filter.should_capture("POST", "/api/users", Some(500)));
        assert!(!filter.should_capture("GET", "/api/users", Some(500))); // Wrong method
        assert!(!filter.should_capture("POST", "/other/path", Some(500))); // Wrong path
        assert!(!filter.should_capture("POST", "/api/users", Some(200))); // Not an error
    }

    // ==================== Default Value Function Tests ====================

    #[test]
    fn test_default_uuid_replacement() {
        let replacement = default_uuid_replacement();
        assert!(replacement.contains("{{counter:012}}"));
    }

    #[test]
    fn test_default_email_replacement() {
        let replacement = default_email_replacement();
        assert_eq!(replacement, "user@example.com");
    }

    #[test]
    fn test_default_ip_replacement() {
        let replacement = default_ip_replacement();
        assert_eq!(replacement, "127.0.0.1");
    }

    #[test]
    fn test_default_creditcard_replacement() {
        let replacement = default_creditcard_replacement();
        assert_eq!(replacement, "XXXX-XXXX-XXXX-XXXX");
    }

    #[test]
    fn test_default_sample_rate() {
        let rate = default_sample_rate();
        assert_eq!(rate, 1.0);
    }
}
