use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// FTP fixture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtpFixture {
    pub identifier: String,
    pub name: String,
    pub description: Option<String>,
    pub virtual_files: Vec<VirtualFileConfig>,
    pub upload_rules: Vec<UploadRule>,
}

/// Configuration for virtual files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFileConfig {
    pub path: PathBuf,
    pub content: FileContentConfig,
    pub permissions: String,
    pub owner: String,
    pub group: String,
}

/// File content configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileContentConfig {
    #[serde(rename = "static")]
    Static { content: String },
    #[serde(rename = "template")]
    Template { template: String },
    #[serde(rename = "generated")]
    Generated {
        size: usize,
        pattern: GenerationPattern,
    },
}

/// Upload rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRule {
    pub path_pattern: String,
    pub auto_accept: bool,
    pub validation: Option<FileValidation>,
    pub storage: UploadStorage,
}

/// File validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileValidation {
    pub max_size_bytes: Option<u64>,
    pub allowed_extensions: Option<Vec<String>>,
    pub mime_types: Option<Vec<String>>,
}

/// Upload storage options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UploadStorage {
    #[serde(rename = "discard")]
    Discard,
    #[serde(rename = "memory")]
    Memory,
    #[serde(rename = "file")]
    File { path: PathBuf },
}

/// Generation patterns for synthetic files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationPattern {
    Random,
    Zeros,
    Ones,
    Incremental,
}

impl UploadRule {
    pub fn matches_path(&self, path: &str) -> bool {
        match Regex::new(&self.path_pattern) {
            Ok(regex) => regex.is_match(path),
            Err(_) => false,
        }
    }

    pub fn validate_file(&self, data: &[u8], filename: &str) -> Result<(), String> {
        if let Some(validation) = &self.validation {
            // Check file size
            if let Some(max_size) = validation.max_size_bytes {
                if data.len() as u64 > max_size {
                    return Err(format!(
                        "File too large: {} bytes (max: {})",
                        data.len(),
                        max_size
                    ));
                }
            }

            // Check file extension
            if let Some(extensions) = &validation.allowed_extensions {
                let has_valid_ext = extensions.iter().any(|ext| {
                    filename.to_lowercase().ends_with(&format!(".{}", ext.to_lowercase()))
                });
                if !extensions.is_empty() && !has_valid_ext {
                    return Err(format!("Invalid file extension. Allowed: {:?}", extensions));
                }
            }

            // TODO: MIME type validation would require additional dependencies
        }

        Ok(())
    }
}
