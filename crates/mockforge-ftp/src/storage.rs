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
        } else if self.vfs.directory_exists(path) {
            Ok(MockForgeMetadata {
                file: None,
                is_dir: true,
            })
        } else {
            Err(Error::from(ErrorKind::PermanentFileNotAvailable))
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

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, path: P) -> Result<()> {
        let path = path.as_ref();
        if self.vfs.directory_exists(path) {
            return Err(Error::new(
                ErrorKind::PermanentFileNotAvailable,
                "Directory already exists",
            ));
        }
        self.vfs
            .create_directory(path.to_path_buf())
            .map_err(|e| Error::new(ErrorKind::LocalError, e))
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, _user: &U, from: P, to: P) -> Result<()> {
        let from = from.as_ref();
        let to = to.as_ref();

        if let Some(file) = self.vfs.get_file(from) {
            self.vfs.remove_file(from).map_err(|e| Error::new(ErrorKind::LocalError, e))?;
            self.vfs
                .add_file(
                    to.to_path_buf(),
                    VirtualFile::new(to.to_path_buf(), file.content, file.metadata),
                )
                .map_err(|e| Error::new(ErrorKind::LocalError, e))
        } else {
            Err(Error::from(ErrorKind::PermanentFileNotAvailable))
        }
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, path: P) -> Result<()> {
        let path = path.as_ref();
        if !self.vfs.directory_exists(path) {
            return Err(Error::from(ErrorKind::PermanentFileNotAvailable));
        }
        self.vfs
            .remove_directory(path)
            .map_err(|e| Error::new(ErrorKind::PermissionDenied, e))
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, _user: &U, path: P) -> Result<()> {
        let path = path.as_ref();
        if self.vfs.directory_exists(path) {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::PermanentFileNotAvailable))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{
        FileContentConfig, FileValidation, FtpFixture, UploadRule, UploadStorage, VirtualFileConfig,
    };
    use crate::vfs::{FileContent, FileMetadata};

    #[test]
    fn test_mockforge_storage_new() {
        let vfs = Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/")));
        let spec_registry = Arc::new(FtpSpecRegistry::new());
        let storage = MockForgeStorage::new(vfs.clone(), spec_registry.clone());

        let debug = format!("{:?}", storage);
        assert!(debug.contains("MockForgeStorage"));
    }

    #[test]
    fn test_mockforge_storage_clone() {
        let vfs = Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/")));
        let spec_registry = Arc::new(FtpSpecRegistry::new());
        let storage = MockForgeStorage::new(vfs.clone(), spec_registry.clone());

        let _cloned = storage.clone();
        // Just verify it can be cloned
    }

    #[test]
    fn test_mockforge_metadata_len_with_file() {
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(b"test content".to_vec()),
            FileMetadata {
                size: 1024,
                ..Default::default()
            },
        );

        let metadata = MockForgeMetadata {
            file: Some(file),
            is_dir: false,
        };

        assert_eq!(metadata.len(), 1024);
    }

    #[test]
    fn test_mockforge_metadata_len_without_file() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: true,
        };

        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn test_mockforge_metadata_is_dir() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: true,
        };

        assert!(metadata.is_dir());
        assert!(!metadata.is_file());
    }

    #[test]
    fn test_mockforge_metadata_is_file() {
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        let metadata = MockForgeMetadata {
            file: Some(file),
            is_dir: false,
        };

        assert!(metadata.is_file());
        assert!(!metadata.is_dir());
    }

    #[test]
    fn test_mockforge_metadata_is_symlink() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: false,
        };

        assert!(!metadata.is_symlink());
    }

    #[test]
    fn test_mockforge_metadata_modified() {
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        let metadata = MockForgeMetadata {
            file: Some(file),
            is_dir: false,
        };

        let modified = metadata.modified();
        assert!(modified.is_ok());
    }

    #[test]
    fn test_mockforge_metadata_modified_no_file() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: true,
        };

        let modified = metadata.modified();
        assert!(modified.is_ok());
    }

    #[test]
    fn test_mockforge_metadata_gid() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: false,
        };

        assert_eq!(metadata.gid(), 1000);
    }

    #[test]
    fn test_mockforge_metadata_uid() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: false,
        };

        assert_eq!(metadata.uid(), 1000);
    }

    #[test]
    fn test_mockforge_metadata_clone() {
        let file = VirtualFile::new(
            std::path::PathBuf::from("/test.txt"),
            FileContent::Static(vec![]),
            FileMetadata::default(),
        );

        let metadata = MockForgeMetadata {
            file: Some(file),
            is_dir: false,
        };

        let _cloned = metadata.clone();
        // Just verify it can be cloned
    }

    #[test]
    fn test_mockforge_metadata_debug() {
        let metadata = MockForgeMetadata {
            file: None,
            is_dir: true,
        };

        let debug = format!("{:?}", metadata);
        assert!(debug.contains("MockForgeMetadata"));
    }

    // Note: The async methods of StorageBackend trait are difficult to test without
    // a full async runtime and mock user details. The tests above cover the synchronous
    // parts and the metadata implementation which is testable without network I/O.
}
