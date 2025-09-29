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
use once_cell::sync::OnceCell;
use rand::{rng, Rng};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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
    /// Environment variables available for substitution
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

/// Combined templating context with both chain and environment variables
#[derive(Debug, Clone)]
pub struct TemplatingContext {
    pub chain_context: Option<ChainTemplatingContext>,
    pub env_context: Option<EnvironmentTemplatingContext>,
}

impl TemplatingContext {
    /// Create empty context
    pub fn empty() -> Self {
        Self {
            chain_context: None,
            env_context: None,
        }
    }

    /// Create context with environment variables only
    pub fn with_env(variables: HashMap<String, String>) -> Self {
        Self {
            chain_context: None,
            env_context: Some(EnvironmentTemplatingContext::new(variables)),
        }
    }

    /// Create context with chain context only
    pub fn with_chain(chain_context: ChainTemplatingContext) -> Self {
        Self {
            chain_context: Some(chain_context),
            env_context: None,
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
        }
    }
}

/// Expand templating tokens in a JSON value recursively.
pub fn expand_tokens(v: &Value) -> Value {
    expand_tokens_with_context(v, &TemplatingContext::empty())
}

/// Expand templating tokens in a JSON value recursively with context.
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

/// Expand templating tokens in a string.
pub fn expand_str(input: &str) -> String {
    expand_str_with_context(input, &TemplatingContext::empty())
}

/// Expand templating tokens in a string with templating context
pub fn expand_str_with_context(input: &str, context: &TemplatingContext) -> String {
    // Basic replacements first (fast paths)
    let mut out = input.replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());
    out = out.replace("{{now}}", &Utc::now().to_rfc3339());

    // now±Nd (days), now±Nh (hours), now±Nm (minutes), now±Ns (seconds)
    out = replace_now_offset(&out);

    // Randoms
    if out.contains("{{rand.int}}") {
        let n: i64 = rng().random_range(0..=1_000_000);
        out = out.replace("{{rand.int}}", &n.to_string());
    }
    if out.contains("{{rand.float}}") {
        let n: f64 = rng().random();
        out = out.replace("{{rand.float}}", &format!("{:.6}", n));
    }
    out = replace_randint_ranges(&out);

    // Response function tokens (new response() syntax)
    if out.contains("response(") {
        out = replace_response_function(&out, context.chain_context.as_ref());
    }

    // Environment variables (check before chain context to allow env vars in chain expressions)
    if out.contains("{{") && context.env_context.is_some() {
        out = replace_env_tokens(&out, context.env_context.as_ref().unwrap());
    }

    // Chain context variables
    if out.contains("{{chain.") {
        out = replace_chain_tokens(&out, context.chain_context.as_ref());
    }

    // Faker tokens (can be disabled for determinism)
    let faker_enabled = std::env::var("MOCKFORGE_FAKE_TOKENS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    if faker_enabled {
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

pub trait FakerProvider {
    fn uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn email(&self) -> String {
        format!("user{}@example.com", rng().random_range(1000..=9999))
    }
    fn name(&self) -> String {
        "Alex Smith".to_string()
    }
    fn address(&self) -> String {
        "1 Main St".to_string()
    }
    fn phone(&self) -> String {
        "+1-555-0100".to_string()
    }
    fn company(&self) -> String {
        "Example Inc".to_string()
    }
    fn url(&self) -> String {
        "https://example.com".to_string()
    }
    fn ip(&self) -> String {
        "192.168.1.1".to_string()
    }
    fn color(&self) -> String {
        "blue".to_string()
    }
    fn word(&self) -> String {
        "word".to_string()
    }
    fn sentence(&self) -> String {
        "A sample sentence.".to_string()
    }
    fn paragraph(&self) -> String {
        "A sample paragraph.".to_string()
    }
}

pub fn register_faker_provider(provider: Arc<dyn FakerProvider + Send + Sync>) {
    let _ = FAKER_PROVIDER.set(provider);
}

fn replace_randint_ranges(input: &str) -> String {
    // Supports {{randInt a b}} and {{rand.int a b}}
    let re = Regex::new(r"\{\{\s*(?:randInt|rand\.int)\s+(-?\d+)\s+(-?\d+)\s*\}\}").unwrap();
    let mut s = input.to_string();
    loop {
        let mat = re.captures(&s);
        if let Some(caps) = mat {
            let a: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let b: i64 = caps.get(2).unwrap().as_str().parse().unwrap_or(100);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let n: i64 = rng().random_range(lo..=hi);
            s = re.replace(&s, n.to_string()).to_string();
        } else {
            break;
        }
    }
    s
}

fn replace_now_offset(input: &str) -> String {
    // {{ now+1d }}, {{now-2h}}, {{now+30m}}, {{now-10s}}
    let re = Regex::new(r"\{\{\s*now\s*([+-])\s*(\d+)\s*([smhd])\s*\}\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let sign = caps.get(1).unwrap().as_str();
        let amount: i64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
        let unit = caps.get(3).map(|m| m.as_str()).unwrap_or("d");
        let dur = match unit {
            "s" => ChronoDuration::seconds(amount),
            "m" => ChronoDuration::minutes(amount),
            "h" => ChronoDuration::hours(amount),
            _ => ChronoDuration::days(amount),
        };
        let ts = if sign == "+" {
            Utc::now() + dur
        } else {
            Utc::now() - dur
        };
        ts.to_rfc3339()
    })
    .to_string()
}

/// Replace environment variable tokens in a template string
fn replace_env_tokens(input: &str, env_context: &EnvironmentTemplatingContext) -> String {
    let re = Regex::new(r"\{\{\s*([^{}\s]+)\s*\}\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = caps.get(1).unwrap().as_str();

        // Skip built-in tokens (uuid, now, rand.*, faker.*, chain.*, encrypt.*, decrypt.*, secure.*)
        if matches!(var_name, "uuid" | "now")
            || var_name.starts_with("rand.")
            || var_name.starts_with("faker.")
            || var_name.starts_with("chain.")
            || var_name.starts_with("encrypt")
            || var_name.starts_with("decrypt")
            || var_name.starts_with("secure")
        {
            return caps.get(0).unwrap().as_str().to_string();
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
    let re = Regex::new(r"\{\{\s*chain\.([^}]+)\s*\}\}").unwrap();

    if let Some(context) = chain_context {
        re.replace_all(input, |caps: &regex::Captures| {
            let path = caps.get(1).unwrap().as_str();

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
    let re = Regex::new(r#"response\s*\(\s*['"]([^'"]*)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#).unwrap();

    if let Some(context) = chain_context {
        let result = re
            .replace_all(input, |caps: &regex::Captures| {
                let request_id = caps.get(1).unwrap().as_str();
                let json_path = caps.get(2).unwrap().as_str();

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
    // Key store is initialized at startup
    let key_store = init_key_store();

    // Default key ID for templating
    let default_key_id = "mockforge_default";

    let mut out = input.to_string();

    // Handle {{encrypt "text"}} or {{encrypt key_id "text"}}
    let encrypt_re =
        Regex::new(r#"\{\{\s*encrypt\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#).unwrap();

    // Handle {{secure "text"}} or {{secure key_id "text"}}
    let secure_re = Regex::new(r#"\{\{\s*secure\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#).unwrap();

    // Process encrypt tokens
    out = encrypt_re
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let plaintext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            match key_store.get_key(key_id) {
                Some(key) => match key.encrypt(plaintext, None) {
                    Ok(ciphertext) => ciphertext,
                    Err(_) => "<encryption_error>".to_string(),
                },
                None => {
                    // Create a default key if none exists
                    let password = std::env::var("MOCKFORGE_ENCRYPTION_KEY")
                        .unwrap_or_else(|_| "mockforge_default_encryption_key_2024".to_string());
                    match crate::encryption::EncryptionKey::from_password_pbkdf2(
                        &password,
                        None,
                        crate::encryption::EncryptionAlgorithm::Aes256Gcm,
                    ) {
                        Ok(key) => match key.encrypt(plaintext, None) {
                            Ok(ciphertext) => ciphertext,
                            Err(_) => "<encryption_error>".to_string(),
                        },
                        Err(_) => "<key_creation_error>".to_string(),
                    }
                }
            }
        })
        .to_string();

    // Process secure tokens (ChaCha20-Poly1305)
    out = secure_re
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let plaintext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            match key_store.get_key(key_id) {
                Some(key) => {
                    // Use ChaCha20-Poly1305 for secure() function
                    match key.encrypt_chacha20(plaintext, None) {
                        Ok(ciphertext) => ciphertext,
                        Err(_) => "<encryption_error>".to_string(),
                    }
                }
                None => {
                    // Create a default key if none exists
                    let password = std::env::var("MOCKFORGE_ENCRYPTION_KEY")
                        .unwrap_or_else(|_| "mockforge_default_encryption_key_2024".to_string());
                    match crate::encryption::EncryptionKey::from_password_pbkdf2(
                        &password,
                        None,
                        crate::encryption::EncryptionAlgorithm::ChaCha20Poly1305,
                    ) {
                        Ok(key) => match key.encrypt_chacha20(plaintext, None) {
                            Ok(ciphertext) => ciphertext,
                            Err(_) => "<encryption_error>".to_string(),
                        },
                        Err(_) => "<key_creation_error>".to_string(),
                    }
                }
            }
        })
        .to_string();

    // Handle {{decrypt "ciphertext"}} or {{decrypt key_id "ciphertext"}}
    let decrypt_re =
        Regex::new(r#"\{\{\s*decrypt\s+(?:([^\s}]+)\s+)?\s*"([^"]+)"\s*\}\}"#).unwrap();

    // Process decrypt tokens
    out = decrypt_re
        .replace_all(&out, |caps: &regex::Captures| {
            let key_id = caps.get(1).map(|m| m.as_str()).unwrap_or(default_key_id);
            let ciphertext = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            match key_store.get_key(key_id) {
                Some(key) => match key.decrypt(ciphertext, None) {
                    Ok(plaintext) => plaintext,
                    Err(_) => "<decryption_error>".to_string(),
                },
                None => {
                    // Create a default key if none exists
                    let password = std::env::var("MOCKFORGE_ENCRYPTION_KEY")
                        .unwrap_or_else(|_| "mockforge_default_encryption_key_2024".to_string());
                    match crate::encryption::EncryptionKey::from_password_pbkdf2(
                        &password,
                        None,
                        crate::encryption::EncryptionAlgorithm::Aes256Gcm,
                    ) {
                        Ok(key) => match key.decrypt(ciphertext, None) {
                            Ok(plaintext) => plaintext,
                            Err(_) => "<decryption_error>".to_string(),
                        },
                        Err(_) => "<key_creation_error>".to_string(),
                    }
                }
            }
        })
        .to_string();

    out
}

/// Replace file system tokens in a template string
fn replace_fs_tokens(input: &str) -> String {
    // Handle {{fs.readFile "path/to/file"}} or {{fs.readFile('path/to/file')}}
    let re = Regex::new(r#"\{\{\s*fs\.readFile\s*(?:\(?\s*(?:'([^']*)'|"([^"]*)")\s*\)?)?\s*\}\}"#)
        .unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let file_path = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");

        if file_path.is_empty() {
            return "<fs.readFile: empty path>".to_string();
        }

        match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => format!("<fs.readFile error: {}>", e),
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
    use crate::request_chaining::{ChainContext, ChainResponse, ChainTemplatingContext};
    use serde_json::json;

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
        // Create a temporary file for testing
        use std::fs;

        let temp_file = "/tmp/mockforge_test_file.txt";
        let test_content = "Hello, this is test content!";
        fs::write(temp_file, test_content).unwrap();

        // Test successful file reading
        let template = format!(r#"{{{{fs.readFile "{}"}}}}"#, temp_file);
        let result = expand_str(&template);
        assert_eq!(result, test_content);

        // Test with parentheses
        let template = format!(r#"{{{{fs.readFile('{}')}}}}"#, temp_file);
        let result = expand_str(&template);
        assert_eq!(result, test_content);

        // Test file not found
        let template = r#"{{fs.readFile "/nonexistent/file.txt"}}"#;
        let result = expand_str(template);
        assert!(result.contains("fs.readFile error:"));

        // Test empty path
        let template = r#"{{fs.readFile ""}}"#;
        let result = expand_str(template);
        assert_eq!(result, "<fs.readFile: empty path>");

        // Clean up
        let _ = fs::remove_file(temp_file);
    }
}
