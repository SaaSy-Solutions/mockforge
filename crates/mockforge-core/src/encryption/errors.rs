//! Error types and handling for encryption operations
//!
//! This module provides comprehensive error handling for all encryption-related
//! operations, including encryption, decryption, key management, and validation.

use std::fmt;
use thiserror::Error;

/// Errors that can occur during encryption/decryption operations
#[derive(Error, Debug)]
pub enum EncryptionError {
    /// Invalid key length or format
    #[error("Invalid key: {message}")]
    InvalidKey { message: String },

    /// Invalid nonce/IV length or format
    #[error("Invalid nonce: {message}")]
    InvalidNonce { message: String },

    /// Authentication failed during decryption
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    /// Invalid ciphertext format
    #[error("Invalid ciphertext: {message}")]
    InvalidCiphertext { message: String },

    /// Key derivation failed
    #[error("Key derivation failed: {message}")]
    KeyDerivationFailed { message: String },

    /// Key not found in key store
    #[error("Key not found: {key_id}")]
    KeyNotFound { key_id: String },

    /// Insufficient permissions for key operation
    #[error("Access denied: {message}")]
    AccessDenied { message: String },

    /// Key store operation failed
    #[error("Key store error: {message}")]
    KeyStoreError { message: String },

    /// Random number generation failed
    #[error("Random generation failed: {message}")]
    RandomGenerationFailed { message: String },

    /// Invalid algorithm configuration
    #[error("Invalid algorithm: {message}")]
    InvalidAlgorithm { message: String },

    /// Cipher operation failed
    #[error("Cipher operation failed: {message}")]
    CipherOperationFailed { message: String },

    /// Base64 encoding/decoding failed
    #[error("Base64 operation failed: {message}")]
    Base64Error { message: String },

    /// Serialization/deserialization failed
    #[error("Serialization failed: {message}")]
    SerializationError { message: String },

    /// Template processing failed
    #[error("Template processing failed: {message}")]
    TemplateError { message: String },

    /// Auto-encryption configuration error
    #[error("Auto-encryption configuration error: {message}")]
    AutoEncryptionConfigError { message: String },

    /// Workspace encryption error
    #[error("Workspace encryption error: {message}")]
    WorkspaceEncryptionError { message: String },

    /// Generic encryption error
    #[error("Encryption error: {message}")]
    Generic { message: String },
}

impl EncryptionError {
    /// Create a new invalid key error
    pub fn invalid_key(message: impl Into<String>) -> Self {
        Self::InvalidKey {
            message: message.into(),
        }
    }

    /// Create a new invalid nonce error
    pub fn invalid_nonce(message: impl Into<String>) -> Self {
        Self::InvalidNonce {
            message: message.into(),
        }
    }

    /// Create a new authentication failed error
    pub fn authentication_failed(message: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            message: message.into(),
        }
    }

    /// Create a new invalid ciphertext error
    pub fn invalid_ciphertext(message: impl Into<String>) -> Self {
        Self::InvalidCiphertext {
            message: message.into(),
        }
    }

    /// Create a new key derivation failed error
    pub fn key_derivation_failed(message: impl Into<String>) -> Self {
        Self::KeyDerivationFailed {
            message: message.into(),
        }
    }

    /// Create a new key not found error
    pub fn key_not_found(key_id: impl Into<String>) -> Self {
        Self::KeyNotFound {
            key_id: key_id.into(),
        }
    }

    /// Create a new access denied error
    pub fn access_denied(message: impl Into<String>) -> Self {
        Self::AccessDenied {
            message: message.into(),
        }
    }

    /// Create a new key store error
    pub fn key_store_error(message: impl Into<String>) -> Self {
        Self::KeyStoreError {
            message: message.into(),
        }
    }

    /// Create a new random generation failed error
    pub fn random_generation_failed(message: impl Into<String>) -> Self {
        Self::RandomGenerationFailed {
            message: message.into(),
        }
    }

    /// Create a new invalid algorithm error
    pub fn invalid_algorithm(message: impl Into<String>) -> Self {
        Self::InvalidAlgorithm {
            message: message.into(),
        }
    }

    /// Create a new cipher operation failed error
    pub fn cipher_operation_failed(message: impl Into<String>) -> Self {
        Self::CipherOperationFailed {
            message: message.into(),
        }
    }

    /// Create a new base64 error
    pub fn base64_error(message: impl Into<String>) -> Self {
        Self::Base64Error {
            message: message.into(),
        }
    }

    /// Create a new serialization error
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError {
            message: message.into(),
        }
    }

    /// Create a new template error
    pub fn template_error(message: impl Into<String>) -> Self {
        Self::TemplateError {
            message: message.into(),
        }
    }

    /// Create a new auto-encryption config error
    pub fn auto_encryption_config_error(message: impl Into<String>) -> Self {
        Self::AutoEncryptionConfigError {
            message: message.into(),
        }
    }

    /// Create a new workspace encryption error
    pub fn workspace_encryption_error(message: impl Into<String>) -> Self {
        Self::WorkspaceEncryptionError {
            message: message.into(),
        }
    }

    /// Create a new generic error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::InvalidKey { .. }
            | Self::InvalidNonce { .. }
            | Self::AuthenticationFailed { .. }
            | Self::InvalidCiphertext { .. }
            | Self::KeyNotFound { .. }
            | Self::InvalidAlgorithm { .. }
            | Self::Base64Error { .. }
            | Self::SerializationError { .. }
            | Self::TemplateError { .. }
            | Self::AutoEncryptionConfigError { .. } => false,

            Self::KeyDerivationFailed { .. }
            | Self::AccessDenied { .. }
            | Self::KeyStoreError { .. }
            | Self::RandomGenerationFailed { .. }
            | Self::CipherOperationFailed { .. }
            | Self::WorkspaceEncryptionError { .. }
            | Self::Generic { .. } => true,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AuthenticationFailed { .. } | Self::AccessDenied { .. } => {
                ErrorSeverity::Critical
            }

            Self::InvalidKey { .. }
            | Self::InvalidNonce { .. }
            | Self::InvalidCiphertext { .. }
            | Self::InvalidAlgorithm { .. } => ErrorSeverity::High,

            Self::KeyDerivationFailed { .. }
            | Self::KeyStoreError { .. }
            | Self::RandomGenerationFailed { .. }
            | Self::CipherOperationFailed { .. } => ErrorSeverity::Medium,

            Self::Base64Error { .. }
            | Self::SerializationError { .. }
            | Self::TemplateError { .. }
            | Self::AutoEncryptionConfigError { .. }
            | Self::WorkspaceEncryptionError { .. }
            | Self::Generic { .. } => ErrorSeverity::Low,

            Self::KeyNotFound { .. } => ErrorSeverity::Info,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational messages
    Info,
    /// Low severity errors
    Low,
    /// Medium severity errors
    Medium,
    /// High severity errors
    High,
    /// Critical severity errors
    Critical,
}

/// Result type alias for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation that failed
    pub operation: String,
    /// Additional context information
    pub context: HashMap<String, String>,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            context: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Enhanced error with context
#[derive(Debug)]
pub struct ContextualError {
    /// The underlying encryption error
    pub error: EncryptionError,
    /// Additional context information
    pub context: ErrorContext,
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Encryption error in {} at {}: {} (context: {:?})",
            self.context.operation, self.context.timestamp, self.error, self.context.context
        )
    }
}

impl std::error::Error for ContextualError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum ErrorRecoveryStrategy {
    /// Retry with exponential backoff
    RetryWithBackoff {
        max_attempts: usize,
        base_delay_ms: u64,
    },
    /// Use fallback encryption method
    FallbackMethod,
    /// Skip encryption for this operation
    SkipEncryption,
    /// Request user intervention
    ManualIntervention,
    /// Fail fast
    FailFast,
}

impl EncryptionError {
    /// Suggest recovery strategy based on error type
    pub fn suggested_recovery(&self) -> ErrorRecoveryStrategy {
        match self {
            Self::RandomGenerationFailed { .. } => ErrorRecoveryStrategy::RetryWithBackoff {
                max_attempts: 3,
                base_delay_ms: 100,
            },

            Self::KeyStoreError { .. } | Self::KeyDerivationFailed { .. } => {
                ErrorRecoveryStrategy::ManualIntervention
            }

            Self::AuthenticationFailed { .. } | Self::AccessDenied { .. } => {
                ErrorRecoveryStrategy::FailFast
            }

            Self::InvalidKey { .. } | Self::InvalidNonce { .. } => {
                ErrorRecoveryStrategy::FallbackMethod
            }

            _ => ErrorRecoveryStrategy::RetryWithBackoff {
                max_attempts: 2,
                base_delay_ms: 50,
            },
        }
    }

    /// Convert to contextual error
    pub fn with_context(self, context: ErrorContext) -> ContextualError {
        ContextualError {
            error: self,
            context,
        }
    }
}

use std::collections::HashMap;
