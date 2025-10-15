use std::collections::HashMap;
use std::sync::Arc;
use mockforge_core::Result;
use mockforge_core::protocol_abstraction::{SpecRegistry, SpecOperation, ValidationResult, ValidationError, Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus};
use crate::vfs::VirtualFileSystem;
use crate::fixtures::{FtpFixture, UploadRule};

/// FTP Spec Registry for MockForge
#[derive(Debug, Clone)]
pub struct FtpSpecRegistry {
    pub fixtures: Vec<FtpFixture>,
    pub vfs: Arc<VirtualFileSystem>,
}

impl FtpSpecRegistry {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            vfs: Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/"))),
        }
    }

    pub fn with_fixtures(mut self, fixtures: Vec<FtpFixture>) -> Self {
        self.fixtures = fixtures;
        self
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
                        ("description".to_string(), fixture.description.clone().unwrap_or_default()),
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
        let valid_operations = ["RETR", "STOR", "LIST", "DELE", "MKD", "RMD", "CWD", "PWD", "SIZE", "MDTM"];
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
                    let content = file.render_content().map_err(|e| mockforge_core::Error::from(e.to_string()))?;
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
                let listing = files.iter()
                    .map(|file| {
                        format!("-rw-r--r-- 1 {} {} {} {} {} {}",
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
                    self.vfs.remove_file(path).map_err(|e| mockforge_core::Error::from(e.to_string()))?;
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
