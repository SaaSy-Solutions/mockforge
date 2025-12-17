use crate::fixtures::{FtpFixture, UploadRule};
use crate::vfs::VirtualFileSystem;
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Tracked upload information
#[derive(Debug, Clone)]
pub struct UploadRecord {
    pub id: String,
    pub path: std::path::PathBuf,
    pub size: u64,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub rule_name: Option<String>,
}

/// FTP Spec Registry for MockForge
#[derive(Debug, Clone)]
pub struct FtpSpecRegistry {
    pub fixtures: Vec<FtpFixture>,
    pub vfs: Arc<VirtualFileSystem>,
    pub uploads: Arc<std::sync::RwLock<Vec<UploadRecord>>>,
}

impl FtpSpecRegistry {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            vfs: Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/"))),
            uploads: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn with_fixtures(mut self, fixtures: Vec<FtpFixture>) -> Result<Self> {
        // Load virtual files into VFS fixtures
        let mut vfs_fixtures = Vec::new();
        for fixture in &fixtures {
            for virtual_file in &fixture.virtual_files {
                vfs_fixtures.push(virtual_file.clone().to_file_fixture());
            }
        }
        self.vfs
            .load_fixtures(vfs_fixtures)
            .map_err(|e| mockforge_core::Error::from(e.to_string()))?;

        self.fixtures = fixtures;
        Ok(self)
    }

    pub fn with_vfs(mut self, vfs: Arc<VirtualFileSystem>) -> Self {
        self.vfs = vfs;
        self
    }

    pub fn find_upload_rule(&self, path: &str) -> Option<&UploadRule> {
        for fixture in &self.fixtures {
            for rule in &fixture.upload_rules {
                if rule.matches_path(path) {
                    return Some(rule);
                }
            }
        }
        None
    }

    pub fn record_upload(
        &self,
        path: std::path::PathBuf,
        size: u64,
        rule_name: Option<String>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let record = UploadRecord {
            id: id.clone(),
            path,
            size,
            uploaded_at: chrono::Utc::now(),
            rule_name,
        };

        let mut uploads = self.uploads.write().unwrap();
        uploads.push(record);

        Ok(id)
    }

    pub fn get_uploads(&self) -> Vec<UploadRecord> {
        self.uploads.read().unwrap().clone()
    }

    pub fn get_upload(&self, id: &str) -> Option<UploadRecord> {
        self.uploads.read().unwrap().iter().find(|u| u.id == id).cloned()
    }
}

impl Default for FtpSpecRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecRegistry for FtpSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Ftp
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.fixtures
            .iter()
            .flat_map(|fixture| {
                fixture.virtual_files.iter().map(|file| SpecOperation {
                    name: format!("{}:{}", fixture.name, file.path.display()),
                    path: file.path.to_string_lossy().to_string(),
                    operation_type: "RETR".to_string(),
                    input_schema: None,
                    output_schema: None,
                    metadata: HashMap::from([
                        (
                            "description".to_string(),
                            fixture.description.clone().unwrap_or_default(),
                        ),
                        ("permissions".to_string(), file.permissions.clone()),
                        ("owner".to_string(), file.owner.clone()),
                    ]),
                })
            })
            .collect()
    }

    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation> {
        self.fixtures
            .iter()
            .flat_map(|fixture| &fixture.virtual_files)
            .find(|file| file.path.to_string_lossy() == path)
            .map(|file| SpecOperation {
                name: path.to_string(),
                path: path.to_string(),
                operation_type: operation.to_string(),
                input_schema: None,
                output_schema: None,
                metadata: HashMap::from([
                    ("permissions".to_string(), file.permissions.clone()),
                    ("owner".to_string(), file.owner.clone()),
                    ("group".to_string(), file.group.clone()),
                ]),
            })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        if request.protocol != Protocol::Ftp {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Invalid protocol for FTP registry".to_string(),
                path: Some("protocol".to_string()),
                code: Some("invalid_protocol".to_string()),
            }]));
        }

        // Basic validation - operation should be a valid FTP command
        let valid_operations = [
            "RETR", "STOR", "LIST", "DELE", "MKD", "RMD", "CWD", "PWD", "SIZE", "MDTM",
        ];
        if !valid_operations.contains(&request.operation.as_str()) {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: format!("Unsupported FTP operation: {}", request.operation),
                path: Some("operation".to_string()),
                code: Some("unsupported_operation".to_string()),
            }]));
        }

        Ok(ValidationResult::success())
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        match request.operation.as_str() {
            "RETR" => {
                // Download file
                let path = std::path::Path::new(&request.path);
                if let Some(file) = self.vfs.get_file(path) {
                    let content = file
                        .render_content()
                        .map_err(|e| mockforge_core::Error::from(e.to_string()))?;
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(150), // Opening data connection
                        body: content,
                        metadata: HashMap::from([
                            ("size".to_string(), file.metadata.size.to_string()),
                            ("path".to_string(), request.path.clone()),
                        ]),
                        content_type: "application/octet-stream".to_string(),
                    })
                } else {
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(550), // File not found
                        body: b"File not found".to_vec(),
                        metadata: HashMap::new(),
                        content_type: "text/plain".to_string(),
                    })
                }
            }
            "STOR" => {
                // Upload file
                let path = &request.path;
                if let Some(rule) = self.find_upload_rule(path) {
                    if let Some(body) = &request.body {
                        // Validate upload
                        if let Err(validation_error) = rule.validate_file(body, path) {
                            return Ok(ProtocolResponse {
                                status: ResponseStatus::FtpStatus(550), // Permission denied
                                body: validation_error.into_bytes(),
                                metadata: HashMap::new(),
                                content_type: "text/plain".to_string(),
                            });
                        }

                        if rule.auto_accept {
                            Ok(ProtocolResponse {
                                status: ResponseStatus::FtpStatus(226), // Transfer complete
                                body: b"Transfer complete".to_vec(),
                                metadata: HashMap::from([
                                    ("path".to_string(), path.clone()),
                                    ("size".to_string(), body.len().to_string()),
                                ]),
                                content_type: "text/plain".to_string(),
                            })
                        } else {
                            Ok(ProtocolResponse {
                                status: ResponseStatus::FtpStatus(550), // Permission denied
                                body: b"Upload rejected by rule".to_vec(),
                                metadata: HashMap::new(),
                                content_type: "text/plain".to_string(),
                            })
                        }
                    } else {
                        Ok(ProtocolResponse {
                            status: ResponseStatus::FtpStatus(550), // Bad request
                            body: b"No file data provided".to_vec(),
                            metadata: HashMap::new(),
                            content_type: "text/plain".to_string(),
                        })
                    }
                } else {
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(550), // Permission denied
                        body: b"No upload rule matches this path".to_vec(),
                        metadata: HashMap::new(),
                        content_type: "text/plain".to_string(),
                    })
                }
            }
            "LIST" => {
                // Directory listing
                let path = std::path::Path::new(&request.path);
                let files = self.vfs.list_files(path);
                let listing = files
                    .iter()
                    .map(|file| {
                        format!(
                            "-rw-r--r-- 1 {} {} {} {} {} {}",
                            file.metadata.owner,
                            file.metadata.group,
                            file.metadata.size,
                            file.modified_at.format("%b %d %H:%M"),
                            file.path.file_name().unwrap_or_default().to_string_lossy(),
                            ""
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(ProtocolResponse {
                    status: ResponseStatus::FtpStatus(226), // Transfer complete
                    body: listing.into_bytes(),
                    metadata: HashMap::from([
                        ("path".to_string(), request.path.clone()),
                        ("count".to_string(), files.len().to_string()),
                    ]),
                    content_type: "text/plain".to_string(),
                })
            }
            "DELE" => {
                // Delete file
                let path = std::path::Path::new(&request.path);
                if self.vfs.get_file(path).is_some() {
                    self.vfs
                        .remove_file(path)
                        .map_err(|e| mockforge_core::Error::from(e.to_string()))?;
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(250), // File deleted
                        body: b"File deleted".to_vec(),
                        metadata: HashMap::from([("path".to_string(), request.path.clone())]),
                        content_type: "text/plain".to_string(),
                    })
                } else {
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(550), // File not found
                        body: b"File not found".to_vec(),
                        metadata: HashMap::new(),
                        content_type: "text/plain".to_string(),
                    })
                }
            }
            "PWD" => {
                // Print working directory
                Ok(ProtocolResponse {
                    status: ResponseStatus::FtpStatus(257), // Current directory
                    body: format!("\"{}\"", request.path).into_bytes(),
                    metadata: HashMap::from([("path".to_string(), request.path.clone())]),
                    content_type: "text/plain".to_string(),
                })
            }
            "SIZE" => {
                // Get file size
                let path = std::path::Path::new(&request.path);
                if let Some(file) = self.vfs.get_file(path) {
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(213), // File size
                        body: file.metadata.size.to_string().into_bytes(),
                        metadata: HashMap::from([
                            ("path".to_string(), request.path.clone()),
                            ("size".to_string(), file.metadata.size.to_string()),
                        ]),
                        content_type: "text/plain".to_string(),
                    })
                } else {
                    Ok(ProtocolResponse {
                        status: ResponseStatus::FtpStatus(550), // File not found
                        body: b"File not found".to_vec(),
                        metadata: HashMap::new(),
                        content_type: "text/plain".to_string(),
                    })
                }
            }
            _ => {
                Ok(ProtocolResponse {
                    status: ResponseStatus::FtpStatus(502), // Command not implemented
                    body: b"Command not implemented".to_vec(),
                    metadata: HashMap::new(),
                    content_type: "text/plain".to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{
        FileContentConfig, FileValidation, FtpFixture, UploadRule, UploadStorage, VirtualFileConfig,
    };
    use crate::vfs::{FileContent, FileMetadata, VirtualFile};

    #[test]
    fn test_upload_record_debug() {
        let record = UploadRecord {
            id: "test-id".to_string(),
            path: std::path::PathBuf::from("/test.txt"),
            size: 1024,
            uploaded_at: chrono::Utc::now(),
            rule_name: Some("test-rule".to_string()),
        };
        let debug = format!("{:?}", record);
        assert!(debug.contains("test-id"));
    }

    #[test]
    fn test_upload_record_clone() {
        let record = UploadRecord {
            id: "test-id".to_string(),
            path: std::path::PathBuf::from("/test.txt"),
            size: 1024,
            uploaded_at: chrono::Utc::now(),
            rule_name: None,
        };
        let cloned = record.clone();
        assert_eq!(record.id, cloned.id);
        assert_eq!(record.size, cloned.size);
    }

    #[test]
    fn test_ftp_spec_registry_new() {
        let registry = FtpSpecRegistry::new();
        assert!(registry.fixtures.is_empty());
        assert_eq!(registry.get_uploads().len(), 0);
    }

    #[test]
    fn test_ftp_spec_registry_default() {
        let registry = FtpSpecRegistry::default();
        assert!(registry.fixtures.is_empty());
    }

    #[test]
    fn test_ftp_spec_registry_protocol() {
        let registry = FtpSpecRegistry::new();
        assert_eq!(registry.protocol(), Protocol::Ftp);
    }

    #[test]
    fn test_ftp_spec_registry_with_fixtures() {
        let fixture = FtpFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            description: Some("A test fixture".to_string()),
            virtual_files: vec![VirtualFileConfig {
                path: std::path::PathBuf::from("/test.txt"),
                content: FileContentConfig::Static {
                    content: "Hello World".to_string(),
                },
                permissions: "644".to_string(),
                owner: "user".to_string(),
                group: "group".to_string(),
            }],
            upload_rules: vec![],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        assert_eq!(registry.fixtures.len(), 1);
        assert!(registry.vfs.get_file(&std::path::PathBuf::from("/test.txt")).is_some());
    }

    #[test]
    fn test_ftp_spec_registry_with_vfs() {
        let vfs = Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/")));
        let registry = FtpSpecRegistry::new().with_vfs(vfs.clone());

        assert!(Arc::ptr_eq(&registry.vfs, &vfs));
    }

    #[test]
    fn test_find_upload_rule_match() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*\.txt$".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        assert!(registry.find_upload_rule("/uploads/test.txt").is_some());
        assert!(registry.find_upload_rule("/uploads/test.pdf").is_none());
    }

    #[test]
    fn test_record_upload() {
        let registry = FtpSpecRegistry::new();
        let path = std::path::PathBuf::from("/upload.txt");

        let id = registry
            .record_upload(path.clone(), 1024, Some("test-rule".to_string()))
            .unwrap();

        assert!(!id.is_empty());
        let uploads = registry.get_uploads();
        assert_eq!(uploads.len(), 1);
        assert_eq!(uploads[0].path, path);
        assert_eq!(uploads[0].size, 1024);
    }

    #[test]
    fn test_get_upload() {
        let registry = FtpSpecRegistry::new();
        let id = registry
            .record_upload(std::path::PathBuf::from("/test.txt"), 512, None)
            .unwrap();

        let upload = registry.get_upload(&id);
        assert!(upload.is_some());
        assert_eq!(upload.unwrap().size, 512);
    }

    #[test]
    fn test_get_upload_not_found() {
        let registry = FtpSpecRegistry::new();
        let upload = registry.get_upload("non-existent-id");
        assert!(upload.is_none());
    }

    #[test]
    fn test_operations() {
        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test Fixture".to_string(),
            description: Some("Test description".to_string()),
            virtual_files: vec![VirtualFileConfig {
                path: std::path::PathBuf::from("/file1.txt"),
                content: FileContentConfig::Static {
                    content: "content".to_string(),
                },
                permissions: "644".to_string(),
                owner: "user".to_string(),
                group: "group".to_string(),
            }],
            upload_rules: vec![],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let ops = registry.operations();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "RETR");
        assert!(ops[0].metadata.contains_key("description"));
    }

    #[test]
    fn test_find_operation() {
        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![VirtualFileConfig {
                path: std::path::PathBuf::from("/test.txt"),
                content: FileContentConfig::Static {
                    content: "test".to_string(),
                },
                permissions: "755".to_string(),
                owner: "root".to_string(),
                group: "admin".to_string(),
            }],
            upload_rules: vec![],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let op = registry.find_operation("RETR", "/test.txt");
        assert!(op.is_some());
        let op = op.unwrap();
        assert_eq!(op.operation_type, "RETR");
        assert_eq!(op.metadata.get("permissions").unwrap(), "755");
    }

    #[test]
    fn test_find_operation_not_found() {
        let registry = FtpSpecRegistry::new();
        let op = registry.find_operation("RETR", "/nonexistent.txt");
        assert!(op.is_none());
    }

    #[test]
    fn test_validate_request_invalid_protocol() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "RETR".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, Some("invalid_protocol".to_string()));
    }

    #[test]
    fn test_validate_request_invalid_operation() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "INVALID".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, Some("unsupported_operation".to_string()));
    }

    #[test]
    fn test_validate_request_valid() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "RETR".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_generate_mock_response_retr_success() {
        let registry = FtpSpecRegistry::new();
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(b"test content".to_vec()),
            FileMetadata {
                size: 12,
                ..Default::default()
            },
        );
        registry.vfs.add_file(std::path::PathBuf::from("/test.txt"), file).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "RETR".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(150));
        assert_eq!(response.body, b"test content");
    }

    #[test]
    fn test_generate_mock_response_retr_not_found() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "RETR".to_string(),
            path: "/nonexistent.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
    }

    #[test]
    fn test_generate_mock_response_stor_success() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "STOR".to_string(),
            path: "/uploads/test.txt".to_string(),
            body: Some(b"file content".to_vec()),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(226));
    }

    #[test]
    fn test_generate_mock_response_stor_rejected() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: false,
            validation: None,
            storage: UploadStorage::Memory,
        };

        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "STOR".to_string(),
            path: "/uploads/test.txt".to_string(),
            body: Some(b"file content".to_vec()),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
        assert_eq!(response.body, b"Upload rejected by rule");
    }

    #[test]
    fn test_generate_mock_response_stor_validation_failed() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: true,
            validation: Some(FileValidation {
                max_size_bytes: Some(10),
                allowed_extensions: None,
                mime_types: None,
            }),
            storage: UploadStorage::Memory,
        };

        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "STOR".to_string(),
            path: "/uploads/test.txt".to_string(),
            body: Some(b"very large file content".to_vec()),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
    }

    #[test]
    fn test_generate_mock_response_stor_no_body() {
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        let fixture = FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "STOR".to_string(),
            path: "/uploads/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
        assert_eq!(response.body, b"No file data provided");
    }

    #[test]
    fn test_generate_mock_response_stor_no_rule() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "STOR".to_string(),
            path: "/uploads/test.txt".to_string(),
            body: Some(b"content".to_vec()),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
        assert_eq!(response.body, b"No upload rule matches this path");
    }

    #[test]
    fn test_generate_mock_response_list() {
        let registry = FtpSpecRegistry::new();
        let file = VirtualFile::new(
            std::path::PathBuf::from("/dir/file.txt"),
            FileContent::Static(b"content".to_vec()),
            FileMetadata {
                size: 7,
                ..Default::default()
            },
        );
        registry.vfs.add_file(std::path::PathBuf::from("/dir/file.txt"), file).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "LIST".to_string(),
            path: "/dir".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(226));
        assert!(response.metadata.get("count").unwrap().parse::<usize>().unwrap() >= 1);
    }

    #[test]
    fn test_generate_mock_response_dele_success() {
        let registry = FtpSpecRegistry::new();
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );
        registry.vfs.add_file(std::path::PathBuf::from("/test.txt"), file).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "DELE".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(250));
    }

    #[test]
    fn test_generate_mock_response_dele_not_found() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "DELE".to_string(),
            path: "/nonexistent.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
    }

    #[test]
    fn test_generate_mock_response_pwd() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "PWD".to_string(),
            path: "/home/user".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(257));
        assert_eq!(response.body, b"\"/home/user\"");
    }

    #[test]
    fn test_generate_mock_response_size_success() {
        let registry = FtpSpecRegistry::new();
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(b"test".to_vec()),
            FileMetadata {
                size: 1024,
                ..Default::default()
            },
        );
        registry.vfs.add_file(std::path::PathBuf::from("/test.txt"), file).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "SIZE".to_string(),
            path: "/test.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(213));
        assert_eq!(response.body, b"1024");
    }

    #[test]
    fn test_generate_mock_response_size_not_found() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "SIZE".to_string(),
            path: "/nonexistent.txt".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(550));
    }

    #[test]
    fn test_generate_mock_response_unsupported_command() {
        let registry = FtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Ftp,
            operation: "MKD".to_string(),
            path: "/newdir".to_string(),
            body: None,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.status, ResponseStatus::FtpStatus(502));
        assert_eq!(response.body, b"Command not implemented");
    }
}
