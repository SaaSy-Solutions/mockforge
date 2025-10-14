use std::sync::Arc;
use anyhow::Result;
use libunftp::Server;
use mockforge_core::config::FtpConfig;
use crate::vfs::VirtualFileSystem;
use crate::spec_registry::FtpSpecRegistry;
use crate::storage::MockForgeStorage;

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
        let server = Server::new(Box::new(move || storage.clone()))
            .greeting("MockForge FTP Server")
            .passive_ports(49152..65535); // Use dynamic port range for passive mode

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
                        self.vfs.add_file(path.to_path_buf(), file)?;
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
