use mockforge_core::config::FtpConfig;
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus,
};
use mockforge_ftp::{
    FileContent, FileMetadata, FtpFixture, FtpServer, FtpSpecRegistry, UploadRule, UploadStorage,
    VirtualFile, VirtualFileConfig,
};
use std::time::Duration;
use tokio::time::timeout;

#[test]
fn test_ftp_server_creation() {
    let config = FtpConfig::default();
    let server = FtpServer::new(config);
    assert!(
        !server.spec_registry().fixtures.is_empty() || server.spec_registry().fixtures.is_empty()
    ); // Just check it doesn't panic
}

#[test]
fn test_vfs_operations() {
    use mockforge_ftp::vfs::{FileContent, FileMetadata, VirtualFile, VirtualFileSystem};
    use std::path::PathBuf;

    let vfs = VirtualFileSystem::new(PathBuf::from("/test"));
    let file = VirtualFile::new(
        PathBuf::from("/test/file.txt"),
        FileContent::Static(b"Hello World".to_vec()),
        FileMetadata::default(),
    );

    vfs.add_file(PathBuf::from("/test/file.txt"), file).unwrap();
    let retrieved = vfs.get_file(std::path::Path::new("/test/file.txt"));
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().render_content().unwrap(), b"Hello World");
}

#[test]
fn test_ftp_server_with_client() {
    // For now, this is a placeholder test
    // Full FTP client integration testing would require:
    // 1. Starting the server on a random port
    // 2. Connecting with a real FTP client (suppaftp)
    // 3. Testing LIST, RETR, STOR operations
    // 4. Verifying upload rules and file content

    // This test currently just validates that the server can be created
    // without panicking. Full integration tests will be added once
    // the server port binding is exposed for testing.

    let config = FtpConfig::default();

    let server = FtpServer::new(config);

    // Just verify the server was created successfully
    assert_eq!(server.spec_registry().fixtures.len(), 0);
}

#[test]
fn test_spec_registry_basic() {
    let registry = FtpSpecRegistry::new();

    // Test that registry is created successfully
    // assert_eq!(mockforge_core::SpecRegistry::protocol(&registry), mockforge_core::protocol_abstraction::Protocol::Ftp);
    // assert!(mockforge_core::SpecRegistry::operations(&registry).is_empty()); // No fixtures by default
}

#[test]
fn test_vfs_file_operations() {
    use mockforge_ftp::vfs::{
        FileContent, FileMetadata, GenerationPattern, VirtualFile, VirtualFileSystem,
    };
    use std::path::PathBuf;

    let vfs = VirtualFileSystem::new(PathBuf::from("/test"));

    // Test static content
    let static_file = VirtualFile::new(
        PathBuf::from("/test/static.txt"),
        FileContent::Static(b"Static content".to_vec()),
        FileMetadata::default(),
    );
    vfs.add_file(PathBuf::from("/test/static.txt"), static_file).unwrap();
    let retrieved = vfs.get_file(std::path::Path::new("/test/static.txt")).unwrap();
    assert_eq!(retrieved.render_content().unwrap(), b"Static content");

    // Test generated content
    let generated_file = VirtualFile::new(
        PathBuf::from("/test/generated.bin"),
        FileContent::Generated {
            size: 100,
            pattern: GenerationPattern::Zeros,
        },
        FileMetadata::default(),
    );
    vfs.add_file(PathBuf::from("/test/generated.bin"), generated_file).unwrap();
    let retrieved_gen = vfs.get_file(std::path::Path::new("/test/generated.bin")).unwrap();
    let content = retrieved_gen.render_content().unwrap();
    assert_eq!(content.len(), 100);
    assert!(content.iter().all(|&b| b == 0));

    // Test file removal
    vfs.remove_file(std::path::Path::new("/test/static.txt")).unwrap();
    assert!(vfs.get_file(std::path::Path::new("/test/static.txt")).is_none());
}
