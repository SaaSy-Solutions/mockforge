use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_ftp::fixtures::FileContentConfig;
use mockforge_ftp::vfs::{FileContent, FileMetadata, VirtualFile, VirtualFileSystem};
use mockforge_ftp::{
    FileValidation, FtpFixture, FtpSpecRegistry, UploadRule, UploadStorage, VirtualFileConfig,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn vfs_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("vfs_add_static_file", |b| {
        b.iter(|| {
            let vfs = VirtualFileSystem::new(std::path::PathBuf::from("/"));
            let file = VirtualFile::new(
                std::path::PathBuf::from("/test.txt"),
                FileContent::Static(b"Hello, World!".to_vec()),
                FileMetadata::default(),
            );
            black_box(vfs.add_file(std::path::PathBuf::from("/test.txt"), file)).unwrap();
        });
    });

    c.bench_function("vfs_add_template_file", |b| {
        b.iter(|| {
            let vfs = VirtualFileSystem::new(std::path::PathBuf::from("/"));
            let template = r#"{"id": "{{uuid}}", "timestamp": "{{now}}"}"#.to_string();
            let file = VirtualFile::new(
                std::path::PathBuf::from("/dynamic.json"),
                FileContent::Template(template),
                FileMetadata::default(),
            );
            black_box(vfs.add_file(std::path::PathBuf::from("/dynamic.json"), file)).unwrap();
        });
    });

    c.bench_function("vfs_list_directory", |b| {
        let vfs = {
            let vfs = VirtualFileSystem::new(std::path::PathBuf::from("/"));
            // Add some test files
            for i in 0..100 {
                let file = VirtualFile::new(
                    std::path::PathBuf::from(&format!("/file{}.txt", i)),
                    FileContent::Static(format!("Content {}", i).into_bytes()),
                    FileMetadata::default(),
                );
                vfs.add_file(std::path::PathBuf::from(&format!("/file{}.txt", i)), file)
                    .unwrap();
            }
            vfs
        };

        b.iter(|| {
            black_box(vfs.list_files(std::path::Path::new("/")));
        });
    });

    c.bench_function("vfs_get_file_content", |b| {
        let vfs = {
            let vfs = VirtualFileSystem::new(std::path::PathBuf::from("/"));
            let file = VirtualFile::new(
                std::path::PathBuf::from("/benchmark.txt"),
                FileContent::Static(b"Benchmark test content".to_vec()),
                FileMetadata::default(),
            );
            vfs.add_file(std::path::PathBuf::from("/benchmark.txt"), file).unwrap();
            vfs
        };

        b.iter(|| {
            let file = black_box(vfs.get_file(std::path::Path::new("/benchmark.txt")).unwrap());
            black_box(file.render_content()).unwrap();
        });
    });
}

fn fixture_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("fixture_registry_creation", |b| {
        b.iter(|| {
            let fixture = FtpFixture {
                identifier: "benchmark_fixture".to_string(),
                name: "benchmark_fixture".to_string(),
                description: Some("Benchmark fixture".to_string()),
                virtual_files: vec![
                    VirtualFileConfig {
                        path: std::path::PathBuf::from("/test1.txt"),
                        content: FileContentConfig::Static {
                            content: "Test content 1".to_string(),
                        },
                        permissions: "644".to_string(),
                        owner: "user".to_string(),
                        group: "group".to_string(),
                    },
                    VirtualFileConfig {
                        path: std::path::PathBuf::from("/test2.txt"),
                        content: FileContentConfig::Static {
                            content: "Test content 2".to_string(),
                        },
                        permissions: "644".to_string(),
                        owner: "user".to_string(),
                        group: "group".to_string(),
                    },
                ],
                upload_rules: vec![UploadRule {
                    path_pattern: "/uploads/.*".to_string(),
                    auto_accept: true,
                    validation: None,
                    storage: UploadStorage::Memory,
                }],
            };

            let registry = FtpSpecRegistry::new().with_fixtures(vec![fixture]).unwrap();
            black_box(registry);
        });
    });

    c.bench_function("fixture_upload_validation", |b| {
        let rule = UploadRule {
            path_pattern: "/uploads/.*".to_string(),
            auto_accept: true,
            validation: Some(FileValidation {
                max_size_bytes: Some(1048576),
                allowed_extensions: None,
                mime_types: None,
            }),
            storage: UploadStorage::Memory,
        };

        b.iter(|| {
            let data = b"Test file content for validation";
            black_box(rule.validate_file(data, "/uploads/test.txt")).unwrap();
        });
    });
}

fn storage_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("vfs_file_operations", |b| {
        let vfs = Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/")));

        b.iter(|| {
            rt.block_on(async {
                // Add a file
                let file = VirtualFile::new(
                    std::path::PathBuf::from("/test.txt"),
                    FileContent::Static(b"Hello, World!".to_vec()),
                    FileMetadata::default(),
                );
                black_box(vfs.add_file(std::path::PathBuf::from("/test.txt"), file)).unwrap();

                // Read the file
                let retrieved = black_box(vfs.get_file(std::path::Path::new("/test.txt")).unwrap());
                let content = black_box(retrieved.render_content()).unwrap();

                // Remove the file
                black_box(vfs.remove_file(std::path::Path::new("/test.txt"))).unwrap();

                content
            });
        });
    });

    c.bench_function("vfs_large_file_operations", |b| {
        let vfs = Arc::new(VirtualFileSystem::new(std::path::PathBuf::from("/")));
        let large_data = vec![0u8; 1024 * 1024]; // 1MB

        b.iter(|| {
            rt.block_on(async {
                // Add a large file
                let file = VirtualFile::new(
                    std::path::PathBuf::from("/large.bin"),
                    FileContent::Static(large_data.clone()),
                    FileMetadata::default(),
                );
                black_box(vfs.add_file(std::path::PathBuf::from("/large.bin"), file)).unwrap();

                // Read the file
                let retrieved =
                    black_box(vfs.get_file(std::path::Path::new("/large.bin")).unwrap());
                let content = black_box(retrieved.render_content()).unwrap();

                // Remove the file
                black_box(vfs.remove_file(std::path::Path::new("/large.bin"))).unwrap();

                content.len()
            });
        });
    });
}

criterion_group!(benches, vfs_benchmarks, fixture_benchmarks, storage_benchmarks);
criterion_main!(benches);
