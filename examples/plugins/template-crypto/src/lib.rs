//! Crypto Template Plugin for MockForge
//!
//! Provides encryption/decryption functions, hashing, and secure random
//! data generation for use in MockForge response templates.

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString}};
use argon2::password_hash::{PasswordVerifier, rand_core::OsRng};
use base64::{Engine as _, engine::general_purpose};
use rand::{RngCore, thread_rng};
use std::collections::HashMap;

/// Crypto Template Plugin
#[derive(Debug)]
pub struct CryptoTemplatePlugin {
    config: CryptoConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    /// Default encryption key (should be overridden in production)
    pub default_key: Option<String>,
    /// Key derivation parameters
    pub key_derivation: KeyDerivationConfig,
    /// Random data generation limits
    pub random_limits: RandomLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationConfig {
    pub algorithm: String, // "argon2id"
    pub memory_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomLimits {
    pub max_bytes: usize,
    pub max_string_length: usize,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            default_key: None,
            key_derivation: KeyDerivationConfig {
                algorithm: "argon2id".to_string(),
                memory_cost_kib: 65536, // 64MB
                time_cost: 3,
                parallelism: 4,
            },
            random_limits: RandomLimits {
                max_bytes: 1024,
                max_string_length: 256,
            },
        }
    }
}

impl CryptoTemplatePlugin {
    pub fn new() -> Self {
        Self {
            config: CryptoConfig::default(),
        }
    }

    /// Generate encryption key from password
    fn derive_key(&self, password: &str, salt: Option<&str>) -> PluginResult<Key<Aes256Gcm>> {
        let salt_string = if let Some(s) = salt {
            SaltString::from_b64(s).map_err(|e| {
                PluginError::invalid_input(format!("Invalid salt: {}", e))
            })?
        } else {
            SaltString::generate(&mut OsRng)
        };

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                self.config.key_derivation.memory_cost_kib,
                self.config.key_derivation.time_cost,
                self.config.key_derivation.parallelism,
                None,
            ).map_err(|e| PluginError::config(format!("Invalid Argon2 params: {}", e)))?
        );

        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| PluginError::execution(format!("Key derivation failed: {}", e)))?;

        let hash_bytes = password_hash.hash.unwrap().as_bytes();
        if hash_bytes.len() < 32 {
            return PluginResult::failure("Derived key too short".to_string(), 0);
        }

        let key_bytes = &hash_bytes[..32];
        let key = Key::<Aes256Gcm>::from_slice(key_bytes);
        PluginResult::success(*key)
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(&self, data: &str, password: Option<&str>, salt: Option<&str>) -> PluginResult<String> {
        let key = if let Some(pwd) = password {
            match self.derive_key(pwd, salt) {
                PluginResult { success: true, data: Some(key), .. } => key,
                PluginResult { success: false, error: Some(err), .. } => {
                    return PluginResult::failure(format!("Key derivation failed: {}", err), 0);
                }
                _ => return PluginResult::failure("Key derivation failed".to_string(), 0),
            }
        } else if let Some(default_key) = &self.config.default_key {
            let key_bytes = hex::decode(default_key).map_err(|e| {
                PluginError::config(format!("Invalid default key: {}", e))
            })?;
            if key_bytes.len() != 32 {
                return PluginResult::failure("Default key must be 32 bytes".to_string(), 0);
            }
            Key::<Aes256Gcm>::from_slice(&key_bytes)
        } else {
            return PluginResult::failure("No encryption key provided".to_string(), 0);
        };

        let cipher = Aes256Gcm::new(&key);
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, data.as_bytes()).map_err(|e| {
            PluginError::execution(format!("Encryption failed: {}", e))
        })?;

        // Combine nonce + ciphertext and base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);
        let encoded = general_purpose::STANDARD.encode(&combined);

        PluginResult::success(encoded)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(&self, encrypted_data: &str, password: Option<&str>, salt: Option<&str>) -> PluginResult<String> {
        let combined = general_purpose::STANDARD.decode(encrypted_data).map_err(|e| {
            PluginError::invalid_input(format!("Invalid base64: {}", e))
        })?;

        if combined.len() < 12 {
            return PluginResult::failure("Encrypted data too short".to_string(), 0);
        }

        let nonce_bytes = &combined[..12];
        let ciphertext = &combined[12..];
        let nonce = Nonce::from_slice(nonce_bytes);

        let key = if let Some(pwd) = password {
            match self.derive_key(pwd, salt) {
                PluginResult { success: true, data: Some(key), .. } => key,
                PluginResult { success: false, error: Some(err), .. } => {
                    return PluginResult::failure(format!("Key derivation failed: {}", err), 0);
                }
                _ => return PluginResult::failure("Key derivation failed".to_string(), 0),
            }
        } else if let Some(default_key) = &self.config.default_key {
            let key_bytes = hex::decode(default_key).map_err(|e| {
                PluginError::config(format!("Invalid default key: {}", e))
            })?;
            if key_bytes.len() != 32 {
                return PluginResult::failure("Default key must be 32 bytes".to_string(), 0);
            }
            Key::<Aes256Gcm>::from_slice(&key_bytes)
        } else {
            return PluginResult::failure("No decryption key provided".to_string(), 0);
        };

        let cipher = Aes256Gcm::new(&key);
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
            PluginError::execution(format!("Decryption failed: {}", e))
        })?;

        let result = String::from_utf8(plaintext).map_err(|e| {
            PluginError::invalid_input(format!("Invalid UTF-8: {}", e))
        })?;

        PluginResult::success(result)
    }

    /// Generate secure random bytes
    fn generate_random_bytes(&self, length: usize) -> PluginResult<String> {
        if length > self.config.random_limits.max_bytes {
            return PluginResult::failure(
                format!("Requested length {} exceeds maximum {}", length, self.config.random_limits.max_bytes),
                0
            );
        }

        let mut bytes = vec![0u8; length];
        thread_rng().fill_bytes(&mut bytes);
        let encoded = general_purpose::STANDARD.encode(&bytes);

        PluginResult::success(encoded)
    }

    /// Generate random string
    fn generate_random_string(&self, length: usize, charset: Option<&str>) -> PluginResult<String> {
        if length > self.config.random_limits.max_string_length {
            return PluginResult::failure(
                format!("Requested length {} exceeds maximum {}", length, self.config.random_limits.max_string_length),
                0
            );
        }

        let charset = charset.unwrap_or("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");
        let charset_bytes = charset.as_bytes();

        if charset_bytes.is_empty() {
            return PluginResult::failure("Empty charset provided".to_string(), 0);
        }

        let mut result = String::with_capacity(length);
        let mut rng = thread_rng();

        for _ in 0..length {
            let idx = rng.next_u32() as usize % charset_bytes.len();
            result.push(charset_bytes[idx] as char);
        }

        PluginResult::success(result)
    }

    /// Hash data using Argon2
    fn hash_data(&self, data: &str, salt: Option<&str>) -> PluginResult<String> {
        let salt_string = if let Some(s) = salt {
            SaltString::from_b64(s).map_err(|e| {
                PluginError::invalid_input(format!("Invalid salt: {}", e))
            })?
        } else {
            SaltString::generate(&mut OsRng)
        };

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                self.config.key_derivation.memory_cost_kib,
                self.config.key_derivation.time_cost,
                self.config.key_derivation.parallelism,
                None,
            ).map_err(|e| PluginError::config(format!("Invalid Argon2 params: {}", e)))?
        );

        let password_hash = argon2.hash_password(data.as_bytes(), &salt_string)
            .map_err(|e| PluginError::execution(format!("Hashing failed: {}", e)))?;

        PluginResult::success(password_hash.to_string())
    }
}

#[async_trait::async_trait]
impl TemplatePlugin for CryptoTemplatePlugin {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        _context: &PluginContext,
    ) -> PluginResult<Value> {
        match function_name {
            "encrypt" => {
                if args.is_empty() {
                    return PluginResult::failure("encrypt requires at least one argument".to_string(), 0);
                }

                let data = args[0].as_str()
                    .ok_or_else(|| PluginError::invalid_input("First argument must be a string"))?;

                let password = args.get(1).and_then(|v| v.as_str());
                let salt = args.get(2).and_then(|v| v.as_str());

                match self.encrypt_data(data, password, salt) {
                    PluginResult { success: true, data: Some(result), .. } => {
                        PluginResult::success(serde_json::json!(result))
                    }
                    PluginResult { success: false, error: Some(err), .. } => {
                        PluginResult::failure(err, 0)
                    }
                    _ => PluginResult::failure("Encryption failed".to_string(), 0),
                }
            }

            "decrypt" => {
                if args.is_empty() {
                    return PluginResult::failure("decrypt requires at least one argument".to_string(), 0);
                }

                let encrypted_data = args[0].as_str()
                    .ok_or_else(|| PluginError::invalid_input("First argument must be a string"))?;

                let password = args.get(1).and_then(|v| v.as_str());
                let salt = args.get(2).and_then(|v| v.as_str());

                match self.decrypt_data(encrypted_data, password, salt) {
                    PluginResult { success: true, data: Some(result), .. } => {
                        PluginResult::success(serde_json::json!(result))
                    }
                    PluginResult { success: false, error: Some(err), .. } => {
                        PluginResult::failure(err, 0)
                    }
                    _ => PluginResult::failure("Decryption failed".to_string(), 0),
                }
            }

            "random_bytes" => {
                let length = args.get(0)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(32) as usize;

                match self.generate_random_bytes(length) {
                    PluginResult { success: true, data: Some(result), .. } => {
                        PluginResult::success(serde_json::json!(result))
                    }
                    PluginResult { success: false, error: Some(err), .. } => {
                        PluginResult::failure(err, 0)
                    }
                    _ => PluginResult::failure("Random bytes generation failed".to_string(), 0),
                }
            }

            "random_string" => {
                let length = args.get(0)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(16) as usize;

                let charset = args.get(1).and_then(|v| v.as_str());

                match self.generate_random_string(length, charset) {
                    PluginResult { success: true, data: Some(result), .. } => {
                        PluginResult::success(serde_json::json!(result))
                    }
                    PluginResult { success: false, error: Some(err), .. } => {
                        PluginResult::failure(err, 0)
                    }
                    _ => PluginResult::failure("Random string generation failed".to_string(), 0),
                }
            }

            "hash" => {
                if args.is_empty() {
                    return PluginResult::failure("hash requires at least one argument".to_string(), 0);
                }

                let data = args[0].as_str()
                    .ok_or_else(|| PluginError::invalid_input("First argument must be a string"))?;

                let salt = args.get(1).and_then(|v| v.as_str());

                match self.hash_data(data, salt) {
                    PluginResult { success: true, data: Some(result), .. } => {
                        PluginResult::success(serde_json::json!(result))
                    }
                    PluginResult { success: false, error: Some(err), .. } => {
                        PluginResult::failure(err, 0)
                    }
                    _ => PluginResult::failure("Hashing failed".to_string(), 0),
                }
            }

            _ => PluginResult::failure(
                format!("Unknown function: {}", function_name),
                0
            ),
        }
    }

    fn get_functions(&self) -> Vec<TemplateFunction> {
        vec![
            TemplateFunction {
                name: "encrypt".to_string(),
                description: "Encrypt data using AES-256-GCM".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "data".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Data to encrypt".to_string(),
                    },
                    FunctionParameter {
                        name: "password".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Encryption password (optional)".to_string(),
                    },
                    FunctionParameter {
                        name: "salt".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Salt for key derivation (optional)".to_string(),
                    },
                ],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "decrypt".to_string(),
                description: "Decrypt data using AES-256-GCM".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "encrypted_data".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Base64-encoded encrypted data".to_string(),
                    },
                    FunctionParameter {
                        name: "password".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Decryption password (optional)".to_string(),
                    },
                    FunctionParameter {
                        name: "salt".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Salt for key derivation (optional)".to_string(),
                    },
                ],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "random_bytes".to_string(),
                description: "Generate secure random bytes".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "length".to_string(),
                        param_type: "integer".to_string(),
                        required: false,
                        description: "Number of bytes to generate (default: 32)".to_string(),
                    },
                ],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "random_string".to_string(),
                description: "Generate random string with specified charset".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "length".to_string(),
                        param_type: "integer".to_string(),
                        required: false,
                        description: "String length (default: 16)".to_string(),
                    },
                    FunctionParameter {
                        name: "charset".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Character set to use (default: alphanumeric)".to_string(),
                    },
                ],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "hash".to_string(),
                description: "Hash data using Argon2".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "data".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Data to hash".to_string(),
                    },
                    FunctionParameter {
                        name: "salt".to_string(),
                        param_type: "string".to_string(),
                        required: false,
                        description: "Salt for hashing (optional)".to_string(),
                    },
                ],
                return_type: "string".to_string(),
            },
        ]
    }
}

mockforge_plugin_core::export_plugin!(CryptoTemplatePlugin);
