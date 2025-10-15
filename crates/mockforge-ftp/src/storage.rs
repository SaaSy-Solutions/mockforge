use crate::spec_registry::FtpSpecRegistry;
use crate::vfs::{VirtualFile, VirtualFileSystem};
use async_trait::async_trait;
use libunftp::storage::Result;
use libunftp::storage::{Error, ErrorKind, Fileinfo, Metadata, StorageBackend};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Custom storage backend for libunftp that integrates with MockForge's VFS
#[derive(Debug, Clone)]
pub struct MockForgeStorage {
    vfs: Arc<VirtualFileSystem>,
    spec_registry: Arc<FtpSpecRegistry>,
}

impl MockForgeStorage {
    pub fn new(vfs: Arc<VirtualFileSystem>, spec_registry: Arc<FtpSpecRegistry>) -> Self {
        Self { vfs, spec_registry }
    }
}

#[async_trait]
impl<U: libunftp::auth::UserDetail + Send + Sync + 'static> StorageBackend<U> for MockForgeStorage {
    type Metadata = MockForgeMetadata;

    async fn metadata<P: AsRef<Path> + Send + Debug>(
        &self,
        _user: &U,
        path: P,
    ) -> Result<Self::Metadata> {
        let path = path.as_ref();

        if let Some(file) = self.vfs.get_file(path) {
            Ok(MockForgeMetadata {
                file: Some(file),
                is_dir: false,
            })
        } else {
            // Check if it's a directory (for now, assume root is a directory)
            if path == Path::new("/") || path == Path::new("") {
                Ok(MockForgeMetadata {
                    file: None,
                    is_dir: true,
                })
            } else {
                Err(Error::from(ErrorKind::PermanentFileNotAvailable))
            }
        }
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        _user: &U,
        path: P,
    ) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>> {
        let path = path.as_ref();
        let files = self.vfs.list_files(path);

        let mut result = Vec::new();
        for file in files {
            result.push(Fileinfo {
                path: file.path.clone(),
                metadata: MockForgeMetadata {
                    file: Some(file),
                    is_dir: false,
                },
            });
        }

        Ok(result)
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        _user: &U,
        path: P,
        _start_pos: u64,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        let path = path.as_ref();

        if let Some(file) = self.vfs.get_file(path) {
            let content =
                file.render_content().map_err(|e| Error::new(ErrorKind::LocalError, e))?;
            Ok(Box::new(std::io::Cursor::new(content)))
        } else {
            Err(Error::from(ErrorKind::PermanentFileNotAvailable))
        }
    }

    async fn put<
        P: AsRef<Path> + Send + Debug,
        R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static,
    >(
        &self,
        _user: &U,
        bytes: R,
        path: P,
        _start_pos: u64,
    ) -> Result<u64> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Read all data
        use tokio::io::AsyncReadExt;
        let mut data = Vec::new();
        let mut reader = bytes;
        reader
            .read_to_end(&mut data)
            .await
            .map_err(|e| Error::new(ErrorKind::LocalError, e))?;

        // Check upload rules
        if let Some(rule) = self.spec_registry.find_upload_rule(&path_str) {
            // Validate the upload
            rule.validate_file(&data, &path_str)
                .map_err(|e| Error::new(ErrorKind::PermissionDenied, e))?;

            if rule.auto_accept {
                // Store the file
                let file = VirtualFile::new(
                    path.to_path_buf(),
                    crate::vfs::FileContent::Static(data.clone()),
                    crate::vfs::FileMetadata {
                        size: data.len() as u64,
                        ..Default::default()
                    },
                );

                self.vfs
                    .add_file(path.to_path_buf(), file)
                    .map_err(|e| Error::new(ErrorKind::LocalError, e))?;

                // Record the upload
                let rule_name = Some(rule.path_pattern.clone());
                self.spec_registry
                    .record_upload(path.to_path_buf(), data.len() as u64, rule_name)
                    .map_err(|e| Error::new(ErrorKind::LocalError, e))?;

                Ok(data.len() as u64)
            } else {
                Err(Error::new(ErrorKind::PermissionDenied, "Upload rejected by rule"))
            }
        } else {
            Err(Error::new(ErrorKind::PermissionDenied, "No upload rule matches this path"))
        }
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, _user: &U, path: P) -> Result<()> {
        let path = path.as_ref();

        if self.vfs.get_file(path).is_some() {
            self.vfs.remove_file(path).map_err(|e| Error::new(ErrorKind::LocalError, e))?;
            Ok(())
        } else {
            Err(Error::from(ErrorKind::PermanentFileNotAvailable))
        }
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, _path: P) -> Result<()> {
        // For now, directories are not supported in VFS
        Err(Error::new(ErrorKind::PermissionDenied, "Directory creation not supported"))
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(
        &self,
        _user: &U,
        _from: P,
        _to: P,
    ) -> Result<()> {
        // For now, renaming is not supported
        Err(Error::new(ErrorKind::PermissionDenied, "Rename not supported"))
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, _path: P) -> Result<()> {
        // For now, directory removal is not supported
        Err(Error::new(ErrorKind::PermissionDenied, "Directory removal not supported"))
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, _path: P) -> Result<()> {
        // For now, directory changes are not supported
        Err(Error::new(ErrorKind::PermissionDenied, "Directory changes not supported"))
    }
}

/// Metadata implementation for MockForge storage
#[derive(Debug, Clone)]
pub struct MockForgeMetadata {
    file: Option<VirtualFile>,
    is_dir: bool,
}

impl Metadata for MockForgeMetadata {
    fn len(&self) -> u64 {
        if let Some(file) = &self.file {
            file.metadata.size
        } else {
            0
        }
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn is_file(&self) -> bool {
        self.file.is_some()
    }

    fn is_symlink(&self) -> bool {
        false
    }

    fn modified(&self) -> Result<SystemTime> {
        if let Some(file) = &self.file {
            // Convert DateTime to SystemTime (approximate)
            Ok(SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(file.modified_at.timestamp() as u64))
        } else {
            Ok(SystemTime::now())
        }
    }

    fn gid(&self) -> u32 {
        1000 // Default GID
    }

    fn uid(&self) -> u32 {
        1000 // Default UID
    }
}
