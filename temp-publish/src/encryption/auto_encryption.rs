//! Automatic encryption configuration and processing
//!
//! This module provides functionality for automatically encrypting sensitive data
//! in requests and responses, including configuration, pattern matching, and processing.

use crate::encryption::algorithms::{EncryptionEngine, EncryptionKey};
use crate::encryption::errors::{EncryptionError, EncryptionResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for automatic encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoEncryptionConfig {
    /// Whether auto-encryption is enabled
    pub enabled: bool,
    /// Encryption key ID to use for auto-encryption
    pub key_id: String,
    /// Patterns for fields to encrypt automatically
    pub field_patterns: Vec<FieldPattern>,
    /// Headers to encrypt automatically
    pub header_patterns: Vec<String>,
    /// Whether to encrypt environment variables
    pub encrypt_environment_variables: bool,
    /// Whether to encrypt request/response bodies
    pub encrypt_request_bodies: bool,
    /// Whether to encrypt response bodies
    pub encrypt_response_bodies: bool,
    /// Custom encryption rules
    pub custom_rules: Vec<EncryptionRule>,
}

impl Default for AutoEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key_id: "default".to_string(),
            field_patterns: Vec::new(),
            header_patterns: Vec::new(),
            encrypt_environment_variables: false,
            encrypt_request_bodies: false,
            encrypt_response_bodies: false,
            custom_rules: Vec::new(),
        }
    }
}

/// Pattern for matching fields to encrypt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPattern {
    /// Pattern to match field names (regex)
    pub pattern: String,
    /// Whether this pattern is case-sensitive
    pub case_sensitive: bool,
    /// Encryption algorithm to use
    pub algorithm: Option<crate::encryption::algorithms::EncryptionAlgorithm>,
}

/// Request context for rule evaluation
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: std::collections::HashMap<String, String>,
    /// Content type from headers
    pub content_type: Option<String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(
        method: String,
        path: String,
        headers: std::collections::HashMap<String, String>,
    ) -> Self {
        let content_type =
            headers.get("content-type").or_else(|| headers.get("Content-Type")).cloned();

        Self {
            method,
            path,
            headers,
            content_type,
        }
    }
}

/// Custom encryption rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionRule {
    /// Name of the rule
    pub name: String,
    /// Conditions for applying the rule
    pub conditions: Vec<RuleCondition>,
    /// Actions to take when rule matches
    pub actions: Vec<RuleAction>,
}

/// Condition for encryption rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Field name matches pattern
    FieldMatches { pattern: String },
    /// Header exists with value
    HeaderExists {
        name: String,
        value_pattern: Option<String>,
    },
    /// Request path matches pattern
    PathMatches { pattern: String },
    /// HTTP method matches
    MethodMatches { method: String },
    /// Content type matches
    ContentTypeMatches { pattern: String },
}

/// Action for encryption rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Encrypt the field
    EncryptField { field_path: String },
    /// Encrypt header value
    EncryptHeader { header_name: String },
    /// Skip encryption for this request
    SkipEncryption,
    /// Use specific algorithm
    UseAlgorithm {
        algorithm: crate::encryption::algorithms::EncryptionAlgorithm,
    },
}

/// Result of automatic encryption processing
#[derive(Debug, Clone)]
pub struct AutoEncryptionResult {
    /// Whether any data was encrypted
    pub encrypted: bool,
    /// Number of fields encrypted
    pub fields_encrypted: usize,
    /// Number of headers encrypted
    pub headers_encrypted: usize,
    /// Encryption metadata for decryption
    pub metadata: EncryptionMetadata,
}

/// Metadata for tracking encryption operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Map of encrypted field paths to encryption info
    pub encrypted_fields: HashMap<String, FieldEncryptionInfo>,
    /// Map of encrypted headers to encryption info
    pub encrypted_headers: HashMap<String, HeaderEncryptionInfo>,
    /// Timestamp of encryption
    pub encrypted_at: chrono::DateTime<chrono::Utc>,
}

/// Information about field encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEncryptionInfo {
    /// Original field path
    pub field_path: String,
    /// Encryption algorithm used
    pub algorithm: crate::encryption::algorithms::EncryptionAlgorithm,
    /// Whether encryption was successful
    pub success: bool,
    /// Error message if encryption failed
    pub error: Option<String>,
}

/// Information about header encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEncryptionInfo {
    /// Header name
    pub header_name: String,
    /// Encryption algorithm used
    pub algorithm: crate::encryption::algorithms::EncryptionAlgorithm,
    /// Whether encryption was successful
    pub success: bool,
    /// Error message if encryption failed
    pub error: Option<String>,
}

/// Automatic encryption processor
#[derive(Debug, Clone)]
pub struct AutoEncryptionProcessor {
    /// Configuration for auto-encryption
    config: AutoEncryptionConfig,
    /// Encryption key for operations
    encryption_key: Option<EncryptionKey>,
    /// Compiled regex patterns
    compiled_patterns: Vec<(Regex, FieldPattern)>,
}

impl AutoEncryptionProcessor {
    /// Create a new auto-encryption processor
    pub fn new(config: AutoEncryptionConfig) -> Self {
        let compiled_patterns = Self::compile_patterns(&config.field_patterns);

        Self {
            config,
            encryption_key: None,
            compiled_patterns,
        }
    }

    /// Set the encryption key
    pub fn set_encryption_key(&mut self, key: EncryptionKey) {
        self.encryption_key = Some(key);
    }

    /// Check if auto-encryption is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.encryption_key.is_some()
    }

    /// Process a request for automatic encryption
    pub fn process_request(
        &self,
        request_data: &mut serde_json::Value,
        request_context: Option<&RequestContext>,
    ) -> EncryptionResult<AutoEncryptionResult> {
        if !self.is_enabled() {
            return Ok(AutoEncryptionResult {
                encrypted: false,
                fields_encrypted: 0,
                headers_encrypted: 0,
                metadata: EncryptionMetadata {
                    encrypted_fields: HashMap::new(),
                    encrypted_headers: HashMap::new(),
                    encrypted_at: chrono::Utc::now(),
                },
            });
        }

        let mut fields_encrypted = 0;
        let mut encrypted_fields = HashMap::new();

        // Encrypt fields in request body
        if self.config.encrypt_request_bodies {
            fields_encrypted += self.encrypt_fields_in_value(
                request_data,
                "",
                &mut encrypted_fields,
                request_context,
            )?;
        }

        Ok(AutoEncryptionResult {
            encrypted: fields_encrypted > 0,
            fields_encrypted,
            headers_encrypted: 0, // Headers handled separately
            metadata: EncryptionMetadata {
                encrypted_fields,
                encrypted_headers: HashMap::new(),
                encrypted_at: chrono::Utc::now(),
            },
        })
    }

    /// Process a response for automatic encryption
    pub fn process_response(
        &self,
        response_data: &mut serde_json::Value,
        request_context: Option<&RequestContext>,
    ) -> EncryptionResult<AutoEncryptionResult> {
        if !self.is_enabled() || !self.config.encrypt_response_bodies {
            return Ok(AutoEncryptionResult {
                encrypted: false,
                fields_encrypted: 0,
                headers_encrypted: 0,
                metadata: EncryptionMetadata {
                    encrypted_fields: HashMap::new(),
                    encrypted_headers: HashMap::new(),
                    encrypted_at: chrono::Utc::now(),
                },
            });
        }

        let mut fields_encrypted = 0;
        let mut encrypted_fields = HashMap::new();

        // Encrypt fields in response body
        fields_encrypted += self.encrypt_fields_in_value(
            response_data,
            "",
            &mut encrypted_fields,
            request_context,
        )?;

        Ok(AutoEncryptionResult {
            encrypted: fields_encrypted > 0,
            fields_encrypted,
            headers_encrypted: 0,
            metadata: EncryptionMetadata {
                encrypted_fields,
                encrypted_headers: HashMap::new(),
                encrypted_at: chrono::Utc::now(),
            },
        })
    }

    /// Encrypt fields in a JSON value based on patterns
    fn encrypt_fields_in_value(
        &self,
        value: &mut serde_json::Value,
        current_path: &str,
        encrypted_fields: &mut HashMap<String, FieldEncryptionInfo>,
        request_context: Option<&RequestContext>,
    ) -> EncryptionResult<usize> {
        let mut count = 0;

        match value {
            serde_json::Value::Object(map) => {
                let mut fields_to_encrypt = Vec::new();

                // Find fields that match patterns
                for (key, _) in map.iter() {
                    let field_path = if current_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", current_path, key)
                    };

                    if self.should_encrypt_field(key, &field_path, request_context) {
                        fields_to_encrypt.push(key.clone());
                    }
                }

                // Encrypt matching fields
                for field_name in fields_to_encrypt {
                    let field_path = if current_path.is_empty() {
                        field_name.clone()
                    } else {
                        format!("{}.{}", current_path, field_name)
                    };

                    if let Some(field_value) = map.get(&field_name) {
                        if let Some(string_value) = field_value.as_str() {
                            if let Some(encryption_key) = &self.encryption_key {
                                match EncryptionEngine::encrypt_string(encryption_key, string_value)
                                {
                                    Ok(encrypted) => {
                                        let encrypted_json = serde_json::to_value(&encrypted)
                                            .map_err(|e| {
                                                EncryptionError::serialization_error(e.to_string())
                                            })?;
                                        map.insert(field_name.clone(), encrypted_json);

                                        encrypted_fields.insert(
                                            field_path.clone(),
                                            FieldEncryptionInfo {
                                                field_path: field_path.clone(),
                                                algorithm: crate::encryption::algorithms::EncryptionAlgorithm::Aes256Gcm,
                                                success: true,
                                                error: None,
                                            },
                                        );
                                        count += 1;
                                    }
                                    Err(e) => {
                                        encrypted_fields.insert(
                                            field_path.clone(),
                                            FieldEncryptionInfo {
                                                field_path: field_path.clone(),
                                                algorithm: crate::encryption::algorithms::EncryptionAlgorithm::Aes256Gcm,
                                                success: false,
                                                error: Some(e.to_string()),
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Recursively process nested objects
                for (_, v) in map.iter_mut() {
                    let nested_path = if current_path.is_empty() {
                        String::new()
                    } else {
                        current_path.to_string()
                    };
                    count += self.encrypt_fields_in_value(
                        v,
                        &nested_path,
                        encrypted_fields,
                        request_context,
                    )?;
                }
            }
            serde_json::Value::Array(arr) => {
                for (index, item) in arr.iter_mut().enumerate() {
                    let nested_path = if current_path.is_empty() {
                        format!("[{}]", index)
                    } else {
                        format!("{}.[{}]", current_path, index)
                    };
                    count += self.encrypt_fields_in_value(
                        item,
                        &nested_path,
                        encrypted_fields,
                        request_context,
                    )?;
                }
            }
            _ => {}
        }

        Ok(count)
    }

    /// Check if a field should be encrypted
    fn should_encrypt_field(
        &self,
        field_name: &str,
        field_path: &str,
        request_context: Option<&RequestContext>,
    ) -> bool {
        // Check custom rules first
        for rule in &self.config.custom_rules {
            if self.rule_matches(rule, field_name, field_path, request_context) {
                for action in &rule.actions {
                    match action {
                        RuleAction::EncryptField { .. } => return true,
                        RuleAction::SkipEncryption => return false,
                        _ => {}
                    }
                }
            }
        }

        // Check field patterns
        for (regex, pattern) in &self.compiled_patterns {
            let text_to_match = if pattern.case_sensitive {
                field_path.to_string()
            } else {
                field_path.to_lowercase()
            };

            if regex.is_match(&text_to_match) {
                return true;
            }
        }

        false
    }

    /// Check if a rule matches the current context
    fn rule_matches(
        &self,
        rule: &EncryptionRule,
        field_name: &str,
        field_path: &str,
        request_context: Option<&RequestContext>,
    ) -> bool {
        for condition in &rule.conditions {
            match condition {
                RuleCondition::FieldMatches { pattern } => {
                    if !Self::matches_pattern(field_name, pattern)
                        && !Self::matches_pattern(field_path, pattern)
                    {
                        return false;
                    }
                }
                RuleCondition::HeaderExists {
                    name,
                    value_pattern,
                } => {
                    if let Some(ctx) = request_context {
                        let header_exists = ctx.headers.contains_key(name);
                        if !header_exists {
                            return false;
                        }
                        if let Some(pattern) = value_pattern {
                            if let Some(header_value) = ctx.headers.get(name) {
                                if !Self::matches_pattern(header_value, pattern) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                    } else {
                        // If no request context, skip this condition
                        continue;
                    }
                }
                RuleCondition::PathMatches { pattern } => {
                    if let Some(ctx) = request_context {
                        if !Self::matches_pattern(&ctx.path, pattern) {
                            return false;
                        }
                    } else {
                        // If no request context, skip this condition
                        continue;
                    }
                }
                RuleCondition::MethodMatches { method } => {
                    if let Some(ctx) = request_context {
                        if !ctx.method.eq_ignore_ascii_case(method) {
                            return false;
                        }
                    } else {
                        // If no request context, skip this condition
                        continue;
                    }
                }
                RuleCondition::ContentTypeMatches { pattern } => {
                    if let Some(ctx) = request_context {
                        if let Some(content_type) = &ctx.content_type {
                            if !Self::matches_pattern(content_type, pattern) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    } else {
                        // If no request context available, rule cannot match
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Helper function to check if text matches a pattern (supports regex)
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        match Regex::new(pattern) {
            Ok(regex) => regex.is_match(text),
            Err(_) => {
                // If pattern is invalid regex, treat as literal string match
                text.contains(pattern)
            }
        }
    }

    /// Compile regex patterns from field patterns
    fn compile_patterns(field_patterns: &[FieldPattern]) -> Vec<(Regex, FieldPattern)> {
        let mut compiled = Vec::new();

        for pattern in field_patterns {
            match Regex::new(&pattern.pattern) {
                Ok(regex) => {
                    compiled.push((regex, pattern.clone()));
                }
                Err(e) => {
                    // Log error but continue with other patterns
                    eprintln!("Failed to compile regex pattern '{}': {}", pattern.pattern, e);
                }
            }
        }

        compiled
    }

    /// Get default field patterns for common sensitive data
    pub fn get_default_field_patterns() -> Vec<FieldPattern> {
        vec![
            FieldPattern {
                pattern: r"(?i)password".to_string(),
                case_sensitive: false,
                algorithm: None,
            },
            FieldPattern {
                pattern: r"(?i)secret".to_string(),
                case_sensitive: false,
                algorithm: None,
            },
            FieldPattern {
                pattern: r"(?i)token".to_string(),
                case_sensitive: false,
                algorithm: None,
            },
            FieldPattern {
                pattern: r"(?i)key".to_string(),
                case_sensitive: false,
                algorithm: None,
            },
            FieldPattern {
                pattern: r"(?i)auth".to_string(),
                case_sensitive: false,
                algorithm: None,
            },
        ]
    }

    /// Get default header patterns for sensitive headers
    pub fn get_default_header_patterns() -> Vec<String> {
        vec![
            "authorization".to_string(),
            "x-api-key".to_string(),
            "x-auth-token".to_string(),
            "cookie".to_string(),
        ]
    }

    /// Validate the auto-encryption configuration
    pub fn validate_config(&self) -> EncryptionResult<()> {
        if self.config.enabled && self.encryption_key.is_none() {
            return Err(EncryptionError::auto_encryption_config_error(
                "Auto-encryption enabled but no encryption key provided",
            ));
        }

        for pattern in &self.config.field_patterns {
            if pattern.pattern.is_empty() {
                return Err(EncryptionError::auto_encryption_config_error("Empty field pattern"));
            }

            if let Err(e) = Regex::new(&pattern.pattern) {
                return Err(EncryptionError::auto_encryption_config_error(format!(
                    "Invalid regex pattern '{}': {}",
                    pattern.pattern, e
                )));
            }
        }

        for rule in &self.config.custom_rules {
            if rule.name.is_empty() {
                return Err(EncryptionError::auto_encryption_config_error(
                    "Encryption rule name cannot be empty",
                ));
            }

            if rule.conditions.is_empty() {
                return Err(EncryptionError::auto_encryption_config_error(
                    "Encryption rule must have at least one condition",
                ));
            }

            if rule.actions.is_empty() {
                return Err(EncryptionError::auto_encryption_config_error(
                    "Encryption rule must have at least one action",
                ));
            }
        }

        Ok(())
    }

    /// Create a default configuration
    pub fn default_config() -> AutoEncryptionConfig {
        AutoEncryptionConfig {
            enabled: false,
            key_id: "auto_encryption_key".to_string(),
            field_patterns: Self::get_default_field_patterns(),
            header_patterns: Self::get_default_header_patterns(),
            encrypt_environment_variables: true,
            encrypt_request_bodies: true,
            encrypt_response_bodies: false,
            custom_rules: Vec::new(),
        }
    }
}

impl Default for AutoEncryptionProcessor {
    fn default() -> Self {
        Self::new(AutoEncryptionConfig::default())
    }
}
