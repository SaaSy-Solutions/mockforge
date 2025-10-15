use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::GenerationPattern;

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

impl VirtualFileConfig {
    pub fn to_file_fixture(self) -> crate::vfs::FileFixture {
        let content = match self.content {
            FileContentConfig::Static { content } => {
                crate::vfs::FileContent::Static(content.into_bytes())
            }
            FileContentConfig::Template { template } => crate::vfs::FileContent::Template(template),
            FileContentConfig::Generated { size, pattern } => {
                crate::vfs::FileContent::Generated { size, pattern }
            }
        };

        let metadata = crate::vfs::FileMetadata {
            permissions: self.permissions,
            owner: self.owner,
            group: self.group,
            size: 0, // Will be calculated when rendered
        };

        crate::vfs::FileFixture {
            path: self.path,
            content,
            metadata,
        }
    }
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

            // Check MIME type
            if let Some(mime_types) = &validation.mime_types {
                let guessed = mime_guess::from_path(filename).first_or_octet_stream();
                let guessed_str = guessed.as_ref();
                if !mime_types.iter().any(|allowed| allowed == guessed_str) {
                    return Err(format!(
                        "Invalid MIME type: {} (allowed: {:?})",
                        guessed_str, mime_types
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_mime_type() {
        let validation = FileValidation {
            max_size_bytes: None,
            allowed_extensions: None,
            mime_types: Some(vec!["text/plain".to_string(), "application/json".to_string()]),
        };

        let rule = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: Some(validation),
            storage: UploadStorage::Discard,
        };

        // Test valid MIME type (text/plain for .txt)
        let data = b"Hello world";
        let filename = "test.txt";
        assert!(rule.validate_file(data, filename).is_ok());

        // Test invalid MIME type (image/png for .txt, but mime_guess guesses text/plain)
        // Actually, for .txt it guesses text/plain, so let's test with a filename that guesses wrong
        // mime_guess guesses based on extension, so for "test.png" it would guess image/png
        let filename_invalid = "test.png";
        assert!(rule.validate_file(data, filename_invalid).is_err());

        // Test with no MIME types configured
        let rule_no_mime = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: Some(FileValidation {
                max_size_bytes: None,
                allowed_extensions: None,
                mime_types: None,
            }),
            storage: UploadStorage::Discard,
        };
        assert!(rule_no_mime.validate_file(data, filename).is_ok());
    }
}
