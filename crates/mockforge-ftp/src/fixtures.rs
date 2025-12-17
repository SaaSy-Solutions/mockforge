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
    fn test_ftp_fixture_debug() {
        let fixture = FtpFixture {
            identifier: "test-id".to_string(),
            name: "Test Fixture".to_string(),
            description: Some("A test fixture".to_string()),
            virtual_files: vec![],
            upload_rules: vec![],
        };

        let debug = format!("{:?}", fixture);
        assert!(debug.contains("test-id"));
    }

    #[test]
    fn test_ftp_fixture_clone() {
        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![],
        };

        let cloned = fixture.clone();
        assert_eq!(fixture.identifier, cloned.identifier);
    }

    #[test]
    fn test_virtual_file_config_to_file_fixture_static() {
        let config = VirtualFileConfig {
            path: std::path::PathBuf::from("/test.txt"),
            content: FileContentConfig::Static {
                content: "Hello World".to_string(),
            },
            permissions: "644".to_string(),
            owner: "user".to_string(),
            group: "group".to_string(),
        };

        let fixture = config.to_file_fixture();
        assert_eq!(fixture.path, std::path::PathBuf::from("/test.txt"));
        assert_eq!(fixture.metadata.permissions, "644");
    }

    #[test]
    fn test_virtual_file_config_to_file_fixture_template() {
        let config = VirtualFileConfig {
            path: std::path::PathBuf::from("/template.txt"),
            content: FileContentConfig::Template {
                template: "Hello {{name}}".to_string(),
            },
            permissions: "755".to_string(),
            owner: "root".to_string(),
            group: "admin".to_string(),
        };

        let fixture = config.to_file_fixture();
        assert_eq!(fixture.metadata.owner, "root");
    }

    #[test]
    fn test_virtual_file_config_to_file_fixture_generated() {
        let config = VirtualFileConfig {
            path: std::path::PathBuf::from("/generated.bin"),
            content: FileContentConfig::Generated {
                size: 1024,
                pattern: GenerationPattern::Random,
            },
            permissions: "600".to_string(),
            owner: "user".to_string(),
            group: "user".to_string(),
        };

        let fixture = config.to_file_fixture();
        assert_eq!(fixture.metadata.permissions, "600");
    }

    #[test]
    fn test_upload_rule_matches_path() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*\.txt$".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        assert!(rule.matches_path("/uploads/file.txt"));
        assert!(rule.matches_path("/uploads/test.txt"));
        assert!(!rule.matches_path("/uploads/file.pdf"));
        assert!(!rule.matches_path("/other/file.txt"));
    }

    #[test]
    fn test_upload_rule_matches_path_invalid_regex() {
        let rule = UploadRule {
            path_pattern: "[invalid regex(".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        assert!(!rule.matches_path("/any/path"));
    }

    #[test]
    fn test_validate_file_max_size() {
        let validation = FileValidation {
            max_size_bytes: Some(100),
            allowed_extensions: None,
            mime_types: None,
        };

        let rule = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: Some(validation),
            storage: UploadStorage::Discard,
        };

        // Test file within size limit
        let small_data = b"small file";
        assert!(rule.validate_file(small_data, "test.txt").is_ok());

        // Test file exceeding size limit
        let large_data = vec![0u8; 200];
        let result = rule.validate_file(&large_data, "test.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_validate_file_extensions() {
        let validation = FileValidation {
            max_size_bytes: None,
            allowed_extensions: Some(vec!["txt".to_string(), "pdf".to_string()]),
            mime_types: None,
        };

        let rule = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: Some(validation),
            storage: UploadStorage::Discard,
        };

        let data = b"test content";

        // Test valid extensions
        assert!(rule.validate_file(data, "file.txt").is_ok());
        assert!(rule.validate_file(data, "document.pdf").is_ok());
        assert!(rule.validate_file(data, "FILE.TXT").is_ok()); // Case insensitive

        // Test invalid extension
        let result = rule.validate_file(data, "image.png");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid file extension"));
    }

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

    #[test]
    fn test_validate_file_no_validation() {
        let rule = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Discard,
        };

        let data = vec![0u8; 10000];
        assert!(rule.validate_file(&data, "any.file").is_ok());
    }

    #[test]
    fn test_validate_file_combined_validations() {
        let validation = FileValidation {
            max_size_bytes: Some(1000),
            allowed_extensions: Some(vec!["txt".to_string()]),
            mime_types: Some(vec!["text/plain".to_string()]),
        };

        let rule = UploadRule {
            path_pattern: ".*".to_string(),
            auto_accept: true,
            validation: Some(validation),
            storage: UploadStorage::Discard,
        };

        let data = b"valid content";

        // All validations pass
        assert!(rule.validate_file(data, "file.txt").is_ok());

        // Size validation fails
        let large_data = vec![0u8; 2000];
        assert!(rule.validate_file(&large_data, "file.txt").is_err());

        // Extension validation fails
        assert!(rule.validate_file(data, "file.pdf").is_err());

        // MIME type validation fails
        assert!(rule.validate_file(data, "file.png").is_err());
    }

    #[test]
    fn test_file_validation_debug() {
        let validation = FileValidation {
            max_size_bytes: Some(100),
            allowed_extensions: Some(vec!["txt".to_string()]),
            mime_types: Some(vec!["text/plain".to_string()]),
        };

        let debug = format!("{:?}", validation);
        assert!(debug.contains("FileValidation"));
    }

    #[test]
    fn test_upload_storage_variants() {
        let discard = UploadStorage::Discard;
        let memory = UploadStorage::Memory;
        let file = UploadStorage::File {
            path: std::path::PathBuf::from("/tmp/uploads"),
        };

        let _ = format!("{:?}", discard);
        let _ = format!("{:?}", memory);
        let _ = format!("{:?}", file);
    }

    #[test]
    fn test_file_content_config_variants() {
        let static_content = FileContentConfig::Static {
            content: "test".to_string(),
        };
        let template = FileContentConfig::Template {
            template: "Hello {{name}}".to_string(),
        };
        let generated = FileContentConfig::Generated {
            size: 100,
            pattern: GenerationPattern::Random,
        };

        let _ = format!("{:?}", static_content);
        let _ = format!("{:?}", template);
        let _ = format!("{:?}", generated);
    }
}
