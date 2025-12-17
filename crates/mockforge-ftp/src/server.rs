use crate::spec_registry::FtpSpecRegistry;
use crate::storage::MockForgeStorage;
use crate::vfs::VirtualFileSystem;
use anyhow::Result;
use libunftp::ServerBuilder;
use mockforge_core::config::FtpConfig;
use std::sync::Arc;

/// FTP Server implementation for MockForge
#[derive(Debug)]
pub struct FtpServer {
    config: FtpConfig,
    vfs: Arc<VirtualFileSystem>,
    spec_registry: Arc<FtpSpecRegistry>,
}

impl FtpServer {
    pub fn new(config: FtpConfig) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(config.virtual_root.clone()));
        let spec_registry = Arc::new(FtpSpecRegistry::new().with_vfs(vfs.clone()));

        Self {
            config,
            vfs,
            spec_registry,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        println!("Starting FTP server on {}", addr);

        // Create the storage backend
        let storage = MockForgeStorage::new(self.vfs.clone(), self.spec_registry.clone());

        // Create the FTP server with our custom storage
        let server = ServerBuilder::new(Box::new(move || storage.clone()))
            .greeting("MockForge FTP Server")
            .passive_ports(49152..=65534); // Use dynamic port range for passive mode

        println!("FTP server listening on {}", addr);
        let server = server.build()?;
        server.listen(&addr).await?;

        Ok(())
    }

    pub async fn handle_upload(&self, path: &std::path::Path, data: Vec<u8>) -> Result<()> {
        // Handle file upload through fixtures
        let path_str = path.to_string_lossy();

        // Find matching upload rule
        if let Some(rule) = self.spec_registry.find_upload_rule(&path_str) {
            // Validate the upload
            rule.validate_file(&data, &path_str).map_err(|e| anyhow::anyhow!(e))?;

            if rule.auto_accept {
                // Store the file based on rule
                match &rule.storage {
                    crate::fixtures::UploadStorage::Memory => {
                        // Store in VFS
                        let size = data.len() as u64;
                        let file = crate::vfs::VirtualFile::new(
                            path.to_path_buf(),
                            crate::vfs::FileContent::Static(data),
                            crate::vfs::FileMetadata {
                                size,
                                ..Default::default()
                            },
                        );
                        self.vfs.add_file_async(path.to_path_buf(), file).await?;
                    }
                    crate::fixtures::UploadStorage::File { path: storage_path } => {
                        // Write to file system
                        tokio::fs::write(storage_path, &data).await?;
                    }
                    crate::fixtures::UploadStorage::Discard => {
                        // Do nothing
                    }
                }
            }
        }

        Ok(())
    }

    pub fn spec_registry(&self) -> Arc<FtpSpecRegistry> {
        self.spec_registry.clone()
    }

    pub fn vfs(&self) -> Arc<VirtualFileSystem> {
        self.vfs.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{FileValidation, UploadRule, UploadStorage};

    #[test]
    fn test_ftp_server_new() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config.clone());
        assert_eq!(server.config.host, "127.0.0.1");
        assert_eq!(server.config.port, 2121);
    }

    #[test]
    fn test_ftp_server_debug() {
        let config = FtpConfig {
            host: "localhost".to_string(),
            port: 21,
            virtual_root: std::path::PathBuf::from("/tmp"),
            ..Default::default()
        };

        let server = FtpServer::new(config);
        let debug = format!("{:?}", server);
        assert!(debug.contains("FtpServer"));
    }

    #[test]
    fn test_ftp_server_spec_registry() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config);
        let registry = server.spec_registry();
        assert!(registry.fixtures.is_empty());
    }

    #[test]
    fn test_ftp_server_vfs() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/test"),
            ..Default::default()
        };

        let server = FtpServer::new(config);
        let vfs = server.vfs();
        let files = vfs.list_files(&std::path::PathBuf::from("/"));
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_handle_upload_memory_storage() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config);

        // Create a fixture with an upload rule
        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Memory,
        };

        let fixture = crate::fixtures::FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        // Update the spec registry
        let new_registry = FtpSpecRegistry::new()
            .with_vfs(server.vfs.clone())
            .with_fixtures(vec![fixture])
            .unwrap();

        let server = FtpServer {
            config: server.config,
            vfs: server.vfs.clone(),
            spec_registry: Arc::new(new_registry),
        };

        let path = std::path::Path::new("/uploads/test.txt");
        let data = b"test file content".to_vec();

        let result = server.handle_upload(path, data.clone()).await;
        assert!(result.is_ok());

        // Verify file was stored in VFS
        let file = server.vfs.get_file_async(path).await;
        assert!(file.is_some());
    }

    #[tokio::test]
    async fn test_handle_upload_discard_storage() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config);

        let rule = UploadRule {
            path_pattern: r"^/uploads/.*".to_string(),
            auto_accept: true,
            validation: None,
            storage: UploadStorage::Discard,
        };

        let fixture = crate::fixtures::FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let new_registry = FtpSpecRegistry::new()
            .with_vfs(server.vfs.clone())
            .with_fixtures(vec![fixture])
            .unwrap();

        let server = FtpServer {
            config: server.config,
            vfs: server.vfs.clone(),
            spec_registry: Arc::new(new_registry),
        };

        let path = std::path::Path::new("/uploads/test.txt");
        let data = b"test file content".to_vec();

        let result = server.handle_upload(path, data).await;
        assert!(result.is_ok());

        // With discard storage, file should not be in VFS
        let file = server.vfs.get_file_async(path).await;
        assert!(file.is_none());
    }

    #[tokio::test]
    async fn test_handle_upload_validation_failure() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config);

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

        let fixture = crate::fixtures::FtpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            virtual_files: vec![],
            upload_rules: vec![rule],
        };

        let new_registry = FtpSpecRegistry::new()
            .with_vfs(server.vfs.clone())
            .with_fixtures(vec![fixture])
            .unwrap();

        let server = FtpServer {
            config: server.config,
            vfs: server.vfs.clone(),
            spec_registry: Arc::new(new_registry),
        };

        let path = std::path::Path::new("/uploads/test.txt");
        let data = b"this is a very large file that exceeds the limit".to_vec();

        let result = server.handle_upload(path, data).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_upload_no_matching_rule() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            virtual_root: std::path::PathBuf::from("/"),
            ..Default::default()
        };

        let server = FtpServer::new(config);

        let path = std::path::Path::new("/no-rule/test.txt");
        let data = b"test content".to_vec();

        let result = server.handle_upload(path, data).await;
        // Should succeed but do nothing since no rule matches
        assert!(result.is_ok());
    }
}
