//! Pillars: [DevX]
//!
//! Extended templating system for MockForge with request chaining support
//!
//! This module provides template expansion with support for:
//! - Standard tokens (UUID, timestamps, random data, faker)
//! - Request chaining context variables
//! - End-to-end encryption functions

use crate::encryption::init_key_store;
use crate::request_chaining::ChainTemplatingContext;
use crate::Config;
use chrono::{Duration as ChronoDuration, Utc};
use mockforge_foundation::time_travel::VirtualClock;
use once_cell::sync::{Lazy, OnceCell};
use rand::{rng, Rng};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// Pre-compiled regex patterns for templating
static RANDINT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*(?:randInt|rand\.int)\s+(-?\d+)\s+(-?\d+)\s*\}\}")
        .expect("RANDINT_RE regex pattern is valid")
});

static NOW_OFFSET_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*now\s*([+-])\s*(\d+)\s*([smhd])\s*\}\}")
        .expect("NOW_OFFSET_RE regex pattern is valid")
});

static ENV_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*([^{}\s]+)\s*\}\}").expect("ENV_TOKEN_RE regex pattern is valid")
});

static CHAIN_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{\s*chain\.([^}]+)\s*\}\}").expect("CHAIN_TOKEN_RE regex pattern is valid")
});

static RESPONSE_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"response\s*\(\s*['"]([^'"]*)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#)
        .expect("RESPONSE_FN_RE regex pattern is valid")
});

static ENCRYPT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\{\{\s*encrypt\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#)
        .expect("ENCRYPT_RE regex pattern is valid")
});

static SECURE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\{\{\s*secure\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#)
        .expect("SECURE_RE regex pattern is valid")
});

static DECRYPT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\{\{\s*decrypt\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#)
        .expect("DECRYPT_RE regex pattern is valid")
});

static FS_READFILE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\{\{\s*fs\.readFile\s*(?:\(?\s*(?:'([^']*)'|"([^"]*)")\s*\)?)?\s*\}\}"#)
        .expect("FS_READFILE_RE regex pattern is valid")
});

/// Template engine for processing template strings with various token types
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    /// Configuration for the template engine
    _config: Config,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            _config: Config::default(),
        }
    }

    /// Create a new template engine with configuration
    pub fn new_with_config(
        config: Config,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self { _config: config })
    }

    /// Expand templating tokens in a string
    pub fn expand_str(&self, input: &str) -> String {
        expand_str(input)
    }

    /// Expand templating tokens in a string with context
    pub fn expand_str_with_context(&self, input: &str, context: &TemplatingContext) -> String {
        expand_str_with_context(input, context)
    }

    /// Expand templating tokens in a JSON value
    pub fn expand_tokens(&self, value: &Value) -> Value {
        expand_tokens(value)
    }

    /// Expand templating tokens in a JSON value with context
    pub fn expand_tokens_with_context(&self, value: &Value, context: &TemplatingContext) -> Value {
        expand_tokens_with_context(value, context)
    }
}

/// Context for environment variables during template expansion
#[derive(Debug, Clone)]
pub struct EnvironmentTemplatingContext {
    /// Map of environment variable names to values
    pub variables: HashMap<String, String>,
}

impl EnvironmentTemplatingContext {
    /// Create a new environment context
    pub fn new(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }

    /// Get a variable value by name
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }
}

/// Combined templating context with chain, environment, and time variables
#[derive(Debug, Clone)]
pub struct TemplatingContext {
    /// Request chaining context for accessing previous request responses
    pub chain_context: Option<ChainTemplatingContext>,
    /// Environment variable context for variable substitution
    pub env_context: Option<EnvironmentTemplatingContext>,
    /// Virtual clock for time-based template tokens
    pub virtual_clock: Option<Arc<VirtualClock>>,
}

impl TemplatingContext {
    /// Create empty context
    pub fn empty() -> Self {
        Self {
            chain_context: None,
            env_context: None,
            virtual_clock: None,
        }
    }

    /// Create context with environment variables only
    pub fn with_env(variables: HashMap<String, String>) -> Self {
        Self {
            chain_context: None,
            env_context: Some(EnvironmentTemplatingContext::new(variables)),
            virtual_clock: None,
        }
    }

    /// Create context with chain context only
    pub fn with_chain(chain_context: ChainTemplatingContext) -> Self {
        Self {
            chain_context: Some(chain_context),
            env_context: None,
            virtual_clock: None,
        }
    }

    /// Create context with both chain and environment contexts
    pub fn with_both(
        chain_context: ChainTemplatingContext,
        variables: HashMap<String, String>,
    ) -> Self {
        Self {
            chain_context: Some(chain_context),
            env_context: Some(EnvironmentTemplatingContext::new(variables)),
            virtual_clock: None,
        }
    }

    /// Create context with virtual clock
    pub fn with_virtual_clock(clock: Arc<VirtualClock>) -> Self {
        Self {
            chain_context: None,
            env_context: None,
            virtual_clock: Some(clock),
        }
    }

    /// Add virtual clock to existing context
    pub fn with_clock(mut self, clock: Arc<VirtualClock>) -> Self {
        self.virtual_clock = Some(clock);
        self
    }
}

/// Expand templating tokens in a JSON value recursively
///
/// Processes all string values in the JSON structure and expands template tokens
/// like `{{uuid}}`, `{{now}}`, `{{faker.email}}`, etc.
///
/// # Arguments
/// * `v` - JSON value to process
///
/// # Returns
/// New JSON value with all template tokens expanded
pub fn expand_tokens(v: &Value) -> Value {
    expand_tokens_with_context(v, &TemplatingContext::empty())
}

/// Expand templating tokens in a JSON value recursively with context
///
/// Similar to `expand_tokens` but uses the provided context for chain variables,
/// environment variables, and virtual clock.
///
/// # Arguments
/// * `v` - JSON value to process
/// * `context` - Templating context with chain, environment, and time information
///
/// # Returns
/// New JSON value with all template tokens expanded
pub fn expand_tokens_with_context(v: &Value, context: &TemplatingContext) -> Value {
    match v {
        Value::String(s) => Value::String(expand_str_with_context(s, context)),
        Value::Array(a) => {
            Value::Array(a.iter().map(|item| expand_tokens_with_context(item, context)).collect())
        }
        Value::Object(o) => {
            let mut map = serde_json::Map::new();
            for (k, vv) in o {
                map.insert(k.clone(), expand_tokens_with_context(vv, context));
            }
            Value::Object(map)
        }
        _ => v.clone(),
    }
}

/// Expand templating tokens in a string
///
/// Processes template tokens in the input string and replaces them with generated values.
/// Supports UUID, timestamps, random numbers, faker data, environment variables, and more.
///
/// # Arguments
/// * `input` - String containing template tokens (e.g., "{{uuid}}", "{{now}}")
///
/// # Returns
/// String with all template tokens replaced
pub fn expand_str(input: &str) -> String {
    expand_str_with_context(input, &TemplatingContext::empty())
}

/// Expand templating tokens in a string with templating context
///
/// Similar to `expand_str` but uses the provided context for chain variables,
/// environment variables, and virtual clock operations.
///
/// # Arguments
/// * `input` - String containing template tokens
/// * `context` - Templating context with chain, environment, and time information
///
/// # Returns
/// String with all template tokens replaced
pub fn expand_str_with_context(input: &str, context: &TemplatingContext) -> String {
    // Early return if no template tokens present (common case optimization)
    if !input.contains("{{") {
        return input.to_string();
    }

    let mut out = input.to_string();

    // Basic replacements first (fast paths) - only if tokens are present
    if out.contains("{{uuid}}") {
        out = out.replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());
    }

    // UUID-only expansion is complete. Bare response() expressions still need
    // their original chain pass even when no braced tokens remain.
    if !out.contains("{{") && !out.contains("response(") {
        return out;
    }

    // Only get current time if we need it (for {{now}} or time offsets)
    let needs_time = out.contains("{{now}}") || NOW_OFFSET_RE.is_match(&out);
    let current_time = if needs_time {
        if let Some(clock) = &context.virtual_clock {
            Some(clock.now())
        } else {
            Some(Utc::now())
        }
    } else {
        None
    };

    if let Some(time) = current_time {
        if out.contains("{{now}}") {
            out = out.replace("{{now}}", &time.to_rfc3339());
        }
        // now±Nd (days), now±Nh (hours), now±Nm (minutes), now±Ns (seconds)
        if NOW_OFFSET_RE.is_match(&out) {
            out = replace_now_offset_with_time(&out, time);
        }
    }

    // Randoms - only process if tokens are present
    if out.contains("{{rand.int}}") {
        let n: i64 = rng().random_range(0..=1_000_000);
        out = out.replace("{{rand.int}}", &n.to_string());
    }
    if out.contains("{{rand.float}}") {
        let n: f64 = rng().random();
        out = out.replace("{{rand.float}}", &format!("{:.6}", n));
    }
    if RANDINT_RE.is_match(&out) {
        out = replace_randint_ranges(&out);
    }

    // Response function tokens (new response() syntax)
    if out.contains("response(") {
        out = replace_response_function(&out, context.chain_context.as_ref());
    }

    // Environment variables (check before chain context to allow env vars in chain expressions)
    if out.contains("{{") {
        if let Some(env_ctx) = context.env_context.as_ref() {
            out = replace_env_tokens(&out, env_ctx);
        }
    }

    // Chain context variables
    if out.contains("{{chain.") {
        out = replace_chain_tokens(&out, context.chain_context.as_ref());
    }

    // Faker tokens (can be disabled for determinism)
    // Cache the environment variable check using OnceCell for better performance
    static FAKER_ENABLED: Lazy<bool> = Lazy::new(|| {
        std::env::var("MOCKFORGE_FAKE_TOKENS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
    });
    if *FAKER_ENABLED && out.contains("{{faker.") {
        out = replace_faker_tokens(&out);
    }

    // File system tokens
    if out.contains("{{fs.readFile") {
        out = replace_fs_tokens(&out);
    }

    // Encryption tokens
    if out.contains("{{encrypt") || out.contains("{{decrypt") || out.contains("{{secure") {
        out = replace_encryption_tokens(&out);
    }

    out
}

// Provider wiring (optional)
static FAKER_PROVIDER: OnceCell<Arc<dyn FakerProvider + Send + Sync>> = OnceCell::new();

/// Provider trait for generating fake data in templates
///
/// Implement this trait to customize how fake data is generated for template tokens
/// like `{{faker.email}}`, `{{faker.name}}`, etc.
pub trait FakerProvider {
    /// Generate a random UUID
    fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    /// Generate a fake email address
    fn email(&self) -> String {
        format!("user{}@example.com", rng().random_range(1000..=9999))
    }
    /// Generate a fake person name
    fn name(&self) -> String {
        "Alex Smith".to_string()
    }
    /// Generate a fake street address
    fn address(&self) -> String {
        "1 Main St".to_string()
    }
    /// Generate a fake phone number
    fn phone(&self) -> String {
        "+1-555-0100".to_string()
    }
    /// Generate a fake company name
    fn company(&self) -> String {
        "Example Inc".to_string()
    }
    /// Generate a fake URL
    fn url(&self) -> String {
        "https://example.com".to_string()
    }
    /// Generate a fake IP address
    fn ip(&self) -> String {
        "192.168.1.1".to_string()
    }
    /// Generate a fake color name
    fn color(&self) -> String {
        "blue".to_string()
    }
    /// Generate a fake word
    fn word(&self) -> String {
        "word".to_string()
    }
    /// Generate a fake sentence
    fn sentence(&self) -> String {
        "A sample sentence.".to_string()
    }
    /// Generate a fake paragraph
    fn paragraph(&self) -> String {
        "A sample paragraph.".to_string()
    }
}

/// Register a custom faker provider for template token generation
///
/// This allows you to replace the default faker implementation with a custom one
/// that can generate more realistic or customized fake data.
///
/// # Arguments
/// * `provider` - Custom faker provider implementation
pub fn register_faker_provider(provider: Arc<dyn FakerProvider + Send + Sync>) {
    let _ = FAKER_PROVIDER.set(provider);
}

fn replace_randint_ranges(input: &str) -> String {
    // Supports {{randInt a b}} and {{rand.int a b}}
    let mut s = input.to_string();
    loop {
        let mat = RANDINT_RE.captures(&s);
        if let Some(caps) = mat {
            let a: i64 = caps.get(1).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            let b: i64 = caps.get(2).map(|m| m.as_str().parse().unwrap_or(100)).unwrap_or(100);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let n: i64 = rng().random_range(lo..=hi);
            s = RANDINT_RE.replace(&s, n.to_string()).to_string();
        } else {
            break;
        }
    }
    s
}

fn replace_now_offset_with_time(input: &str, current_time: chrono::DateTime<Utc>) -> String {
    // {{ now+1d }}, {{now-2h}}, {{now+30m}}, {{now-10s}}
    NOW_OFFSET_RE
        .replace_all(input, |caps: &regex::Captures| {
            let sign = caps.get(1).map(|m| m.as_str()).unwrap_or("+");
            let amount: i64 = caps.get(2).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            let unit = caps.get(3).map(|m| m.as_str()).unwrap_or("d");
            let dur = match unit {
                "s" => ChronoDuration::seconds(amount),
                "m" => ChronoDuration::minutes(amount),
                "h" => ChronoDuration::hours(amount),
                _ => ChronoDuration::days(amount),
            };
            let ts = if sign == "+" {
                current_time + dur
            } else {
                current_time - dur
            };
            ts.to_rfc3339()
        })
        .to_string()
}

/// Replace environment variable tokens in a template string
fn replace_env_tokens(input: &str, env_context: &EnvironmentTemplatingContext) -> String {
    ENV_TOKEN_RE
        .replace_all(input, |caps: &regex::Captures| {
            let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

            // Skip built-in tokens (uuid, now, rand.*, faker.*, chain.*, encrypt.*, decrypt.*, secure.*)
            if matches!(var_name, "uuid" | "now")
                || var_name.starts_with("rand.")
                || var_name.starts_with("faker.")
                || var_name.starts_with("chain.")
                || var_name.starts_with("encrypt")
                || var_name.starts_with("decrypt")
                || var_name.starts_with("secure")
            {
                return caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default();
            }

            // Look up the variable in environment context
            match env_context.get_variable(var_name) {
                Some(value) => value.clone(),
                None => format!("{{{{{}}}}}", var_name), // Keep original if not found
            }
        })
        .to_string()
}

/// Replace chain context tokens in a template string
fn replace_chain_tokens(input: &str, chain_context: Option<&ChainTemplatingContext>) -> String {
    if let Some(context) = chain_context {
        CHAIN_TOKEN_RE
            .replace_all(input, |caps: &regex::Captures| {
                let path = caps.get(1).map(|m| m.as_str()).unwrap_or("");

                match context.extract_value(path) {
                    Some(Value::String(s)) => s,
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    Some(val) => serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string()),
                    None => "null".to_string(), // Return null for missing values instead of empty string
                }
            })
            .to_string()
    } else {
        // No chain context available, return input unchanged
        input.to_string()
    }
}

/// Replace response function tokens (new response() syntax)
fn replace_response_function(
    input: &str,
    chain_context: Option<&ChainTemplatingContext>,
) -> String {
    // Match response('request_id', 'jsonpath') - handle both single and double quotes
    if let Some(context) = chain_context {
        let result = RESPONSE_FN_RE
            .replace_all(input, |caps: &regex::Captures| {
                let request_id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let json_path = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                // Build the full path like "request_id.json_path"
                let full_path = if json_path.is_empty() {
                    request_id.to_string()
                } else {
                    format!("{}.{}", request_id, json_path)
                };

                match context.extract_value(&full_path) {
                    Some(Value::String(s)) => s,
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    Some(val) => serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string()),
                    None => "null".to_string(), // Return null for missing values
                }
            })
            .to_string();

        result
    } else {
        // No chain context available, return input unchanged
        input.to_string()
    }
}

/// Replace encryption tokens in a template string
fn replace_encryption_tokens(input: &str) -> String {
    let password = std::env::var("MOCKFORGE_ENCRYPTION_KEY").ok();
    replace_encryption_tokens_with(input, password.as_deref())
}

/// Salt for deriving the template key from `MOCKFORGE_ENCRYPTION_KEY`. It is
/// fixed so `{{decrypt}}` derives the same key `{{encrypt}}` used; without it
/// PBKDF2 picked a random salt per call and nothing could ever be decrypted.
const TEMPLATE_KEY_SALT: &[u8] = b"mockforge-template-encryption-v1";

/// Derive the fallback key used when the key store has no key for the
/// requested id. There is deliberately no built-in password: without
/// `MOCKFORGE_ENCRYPTION_KEY` the tokens fail instead of encrypting with a key
/// published in the source (#1273).
fn fallback_encryption_key(
    password: Option<&str>,
    algorithm: crate::encryption::EncryptionAlgorithm,
) -> Result<crate::encryption::EncryptionKey, &'static str> {
    let password = password.filter(|p| !p.trim().is_empty()).ok_or("no key configured")?;
    crate::encryption::EncryptionKey::from_password_pbkdf2(
        password,
        Some(TEMPLATE_KEY_SALT),
        algorithm,
    )
    .map_err(|_| "key creation failed")
}

fn replace_encryption_tokens_with(input: &str, password: Option<&str>) -> String {
    use crate::encryption::EncryptionAlgorithm;

    // Key store is initialized at startup
    let key_store = init_key_store();

    // Default key ID for templating
    let default_key_id = "mockforge_default";

    let mut out = input.to_string();

    // Process encrypt tokens
    out = ENCRYPT_RE
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let plaintext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let result = match key_store.get_key(key_id) {
                Some(key) => key.encrypt(plaintext, None).map_err(|_| "encryption failed"),
                None => fallback_encryption_key(password, EncryptionAlgorithm::Aes256Gcm)
                    .and_then(|key| key.encrypt(plaintext, None).map_err(|_| "encryption failed")),
            };
            result.unwrap_or_else(|e| format!("<encryption_error: {e}>"))
        })
        .to_string();

    // Process secure tokens (ChaCha20-Poly1305)
    out = SECURE_RE
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let plaintext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let result = match key_store.get_key(key_id) {
                Some(key) => key.encrypt_chacha20(plaintext, None).map_err(|_| "encryption failed"),
                None => fallback_encryption_key(password, EncryptionAlgorithm::ChaCha20Poly1305)
                    .and_then(|key| {
                        key.encrypt_chacha20(plaintext, None).map_err(|_| "encryption failed")
                    }),
            };
            result.unwrap_or_else(|e| format!("<encryption_error: {e}>"))
        })
        .to_string();

    // Process decrypt tokens
    out = DECRYPT_RE
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let ciphertext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let result = match key_store.get_key(key_id) {
                Some(key) => key.decrypt(ciphertext, None).map_err(|_| "decryption failed"),
                None => fallback_encryption_key(password, EncryptionAlgorithm::Aes256Gcm)
                    .and_then(|key| key.decrypt(ciphertext, None).map_err(|_| "decryption failed")),
            };
            result.unwrap_or_else(|e| format!("<decryption_error: {e}>"))
        })
        .to_string();

    out
}

/// Replace file system tokens in a template string
fn replace_fs_tokens(input: &str) -> String {
    // Handle {{fs.readFile "path/to/file"}} or {{fs.readFile('path/to/file')}}
    FS_READFILE_RE
        .replace_all(input, |caps: &regex::Captures| {
            let file_path = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");

            if file_path.is_empty() {
                return "<fs.readFile: empty path>".to_string();
            }

            // Sandboxed to the configured fixtures root (#1015).
            match crate::template_fs::read_template_file(file_path) {
                Ok(content) => content,
                Err(e) => {
                    tracing::warn!(path = %file_path, error = %e, "fs.readFile refused");
                    format!("<fs.readFile error: {}>", e)
                }
            }
        })
        .to_string()
}

fn replace_faker_tokens(input: &str) -> String {
    // If a provider is registered (e.g., from mockforge-data), use it; else fallback
    if let Some(provider) = FAKER_PROVIDER.get() {
        return replace_with_provider(input, provider.as_ref());
    }
    replace_with_fallback(input)
}

fn replace_with_provider(input: &str, p: &dyn FakerProvider) -> String {
    let mut out = input.to_string();
    let map = [
        ("{{faker.uuid}}", p.uuid()),
        ("{{faker.email}}", p.email()),
        ("{{faker.name}}", p.name()),
        ("{{faker.address}}", p.address()),
        ("{{faker.phone}}", p.phone()),
        ("{{faker.company}}", p.company()),
        ("{{faker.url}}", p.url()),
        ("{{faker.ip}}", p.ip()),
        ("{{faker.color}}", p.color()),
        ("{{faker.word}}", p.word()),
        ("{{faker.sentence}}", p.sentence()),
        ("{{faker.paragraph}}", p.paragraph()),
    ];
    for (pat, val) in map {
        if out.contains(pat) {
            out = out.replace(pat, &val);
        }
    }
    out
}

fn replace_with_fallback(input: &str) -> String {
    let mut out = input.to_string();
    if out.contains("{{faker.uuid}}") {
        out = out.replace("{{faker.uuid}}", &uuid::Uuid::new_v4().to_string());
    }
    if out.contains("{{faker.email}}") {
        let user: String = (0..8).map(|_| (b'a' + (rng().random::<u8>() % 26)) as char).collect();
        let dom: String = (0..6).map(|_| (b'a' + (rng().random::<u8>() % 26)) as char).collect();
        out = out.replace("{{faker.email}}", &format!("{}@{}.example", user, dom));
    }
    if out.contains("{{faker.name}}") {
        let firsts = ["Alex", "Sam", "Taylor", "Jordan", "Casey", "Riley"];
        let lasts = ["Smith", "Lee", "Patel", "Garcia", "Kim", "Brown"];
        let fi = rng().random::<u8>() as usize % firsts.len();
        let li = rng().random::<u8>() as usize % lasts.len();
        out = out.replace("{{faker.name}}", &format!("{} {}", firsts[fi], lasts[li]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // #1273: encrypt/secure/decrypt used to fall back to a password in the
    // source. Without a configured key they must fail, never encrypt.
    #[test]
    fn test_encryption_tokens_fail_without_configured_key() {
        for password in [None, Some(""), Some("   ")] {
            let out = replace_encryption_tokens_with(
                r#"{{encrypt "a"}}|{{secure "b"}}|{{decrypt "c"}}"#,
                password,
            );
            assert_eq!(
                out,
                "<encryption_error: no key configured>|<encryption_error: no key configured>|\
                 <decryption_error: no key configured>"
            );
        }
    }

    #[test]
    fn test_encryption_tokens_round_trip_with_configured_key() {
        use base64::Engine as _;
        let password = Some("operator-chosen-secret");
        let ciphertext = replace_encryption_tokens_with(r#"{{encrypt "hello"}}"#, password);
        assert_ne!(ciphertext, "hello");
        assert!(base64::engine::general_purpose::STANDARD.decode(&ciphertext).is_ok());
        let decrypted =
            replace_encryption_tokens_with(&format!(r#"{{{{decrypt "{ciphertext}"}}}}"#), password);
        assert_eq!(decrypted, "hello");

        // secure() uses ChaCha20-Poly1305 and also needs a configured key.
        let secured = replace_encryption_tokens_with(r#"{{secure "test message"}}"#, password);
        assert!(base64::engine::general_purpose::STANDARD.decode(&secured).is_ok());

        // A different key cannot read it.
        let wrong = replace_encryption_tokens_with(
            &format!(r#"{{{{decrypt "{ciphertext}"}}}}"#),
            Some("another-secret"),
        );
        assert_eq!(wrong, "<decryption_error: decryption failed>");
    }
    use crate::request_chaining::{ChainContext, ChainResponse, ChainTemplatingContext};
    use serde_json::json;

    #[test]
    fn finished_uuid_expansion_preserves_repetition_and_freshness() {
        let first = expand_str("{{uuid}}|{{uuid}}");
        let (left, right) = first.split_once('|').unwrap();
        assert_eq!(left, right);
        assert_eq!(uuid::Uuid::parse_str(left).unwrap().get_version_num(), 4);
        assert_ne!(left, expand_str("{{uuid}}"));
    }

    #[test]
    fn uuid_expansion_preserves_remaining_environment_and_unknown_tokens() {
        let context = TemplatingContext::with_env(HashMap::from([(
            "fixture".into(),
            "fixture-value".into(),
        )]));
        let result = expand_str_with_context("{{uuid}}|{{fixture}}|{{unknown}}", &context);
        let parts: Vec<_> = result.split('|').collect();
        assert!(uuid::Uuid::parse_str(parts[0]).is_ok());
        assert_eq!(&parts[1..], &["fixture-value", "{{unknown}}"]);
    }

    #[test]
    fn uuid_expansion_preserves_bare_response_function_processing() {
        let mut chain = ChainContext::new();
        chain.store_response(
            "fixture".into(),
            ChainResponse {
                status: 200,
                headers: HashMap::new(),
                body: Some(json!({"id": 42})),
                duration_ms: 1,
                executed_at: "2026-10-14T00:00:00Z".into(),
                error: None,
            },
        );
        let context = TemplatingContext::with_chain(ChainTemplatingContext::new(chain));
        let result = expand_str_with_context("{{uuid}}|response('fixture', 'id')", &context);
        let (id, value) = result.split_once('|').unwrap();
        assert!(uuid::Uuid::parse_str(id).is_ok());
        assert_eq!(value, "42");
    }

    #[test]
    fn test_expand_str_with_context() {
        let chain_context = ChainTemplatingContext::new(ChainContext::new());
        let context = TemplatingContext::with_chain(chain_context);
        let result = expand_str_with_context("{{uuid}}", &context);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_replace_env_tokens() {
        let mut vars = HashMap::new();
        vars.insert("api_key".to_string(), "secret123".to_string());
        let env_context = EnvironmentTemplatingContext::new(vars);
        let result = replace_env_tokens("{{api_key}}", &env_context);
        assert_eq!(result, "secret123");
    }

    #[test]
    fn test_replace_chain_tokens() {
        let chain_ctx = ChainContext::new();
        let template_ctx = ChainTemplatingContext::new(chain_ctx);
        let context = Some(&template_ctx);
        // Note: This test would need a proper response stored in the chain context
        let result = replace_chain_tokens("{{chain.test.body}}", context);
        assert_eq!(result, "null");
    }

    #[test]
    fn test_response_function() {
        // Test with no chain context
        let result = replace_response_function(r#"response('login', 'body.user_id')"#, None);
        assert_eq!(result, r#"response('login', 'body.user_id')"#);

        // Test with chain context but no matching response
        let chain_ctx = ChainContext::new();
        let template_ctx = ChainTemplatingContext::new(chain_ctx);
        let context = Some(&template_ctx);
        let result = replace_response_function(r#"response('login', 'body.user_id')"#, context);
        assert_eq!(result, "null");

        // Test with stored response
        let mut chain_ctx = ChainContext::new();
        let response = ChainResponse {
            status: 200,
            headers: HashMap::new(),
            body: Some(json!({"user_id": 12345})),
            duration_ms: 150,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };
        chain_ctx.store_response("login".to_string(), response);
        let template_ctx = ChainTemplatingContext::new(chain_ctx);
        let context = Some(&template_ctx);

        let result = replace_response_function(r#"response('login', 'user_id')"#, context);
        assert_eq!(result, "12345");
    }

    #[test]
    fn test_fs_readfile() {
        // One test owns the process-wide default root to avoid racing itself.
        if std::env::var(crate::template_fs::ROOT_ENV).is_ok()
            || std::env::var(crate::template_fs::HOSTED_ENV).is_ok()
        {
            return;
        }
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("spec");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("body.txt"), "Hello, fixture!").unwrap();
        std::fs::write(dir.path().join("secret.txt"), "top-secret").unwrap();
        crate::template_fs::set_default_root(Some(root.clone()));

        // Legitimate reads inside the spec directory still work, both syntaxes.
        assert_eq!(expand_str(r#"{{fs.readFile "body.txt"}}"#), "Hello, fixture!");
        assert_eq!(expand_str(r#"{{fs.readFile('body.txt')}}"#), "Hello, fixture!");

        // Escapes are refused and never leak content.
        for t in [
            r#"{{fs.readFile "../secret.txt"}}"#,
            r#"{{fs.readFile "/etc/passwd"}}"#,
            r#"{{fs.readFile "/proc/self/environ"}}"#,
        ] {
            let out = expand_str(t);
            assert!(out.starts_with("<fs.readFile error:"), "{t} -> {out}");
            assert!(!out.contains("top-secret") && !out.contains("root:"), "{t} -> {out}");
        }

        // Disabled when no root is registered.
        crate::template_fs::set_default_root(None);
        let out = expand_str(r#"{{fs.readFile "body.txt"}}"#);
        assert!(out.starts_with("<fs.readFile error: disabled"), "{out}");

        // Empty path
        assert_eq!(expand_str(r#"{{fs.readFile ""}}"#), "<fs.readFile: empty path>");
    }
}
