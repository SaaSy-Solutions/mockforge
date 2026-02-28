use mockforge_core::config::FtpConfig;
use mockforge_ftp::{FtpServer, FtpSpecRegistry};

#[test]
fn test_ftp_server_creation() {
    let config = FtpConfig::default();
    let server = FtpServer::new(config);
    // Default config should create a server with no fixtures
    assert!(server.spec_registry().fixtures.is_empty());
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
fn test_ftp_server_with_custom_config() {
    let config = FtpConfig {
        port: 2121,
        host: "127.0.0.1".to_string(),
        passive_ports: (30000, 30010),
        ..Default::default()
    };

    let server = FtpServer::new(config);
    assert!(server.spec_registry().fixtures.is_empty());
}

#[test]
fn test_spec_registry_basic() {
    use mockforge_core::protocol_abstraction::{Protocol, SpecRegistry};

    let registry = FtpSpecRegistry::new();

    assert_eq!(registry.protocol(), Protocol::Ftp);
    assert!(registry.operations().is_empty());
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
