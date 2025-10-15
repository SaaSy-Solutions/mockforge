use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_ftp::fixtures::{FixtureRegistry, UploadRule};
use mockforge_ftp::storage::MockStorage;
use mockforge_ftp::vfs::{FileContent, FilePermissions, VirtualFileSystem};
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

    c.bench_function("fixture_registry_load", |b| {
        let fixture_config = r#"
        name: "benchmark_fixture"
        description: "Benchmark fixture"
        virtual_files:
          - path: "/test1.txt"
            content:
              type: "static"
              content: "Test content 1"
          - path: "/test2.txt"
            content:
              type: "static"
              content: "Test content 2"
        upload_rules:
          - path_pattern: "/uploads/.*"
            auto_accept: true
            storage:
              type: "memory"
        "#;

        b.iter(|| {
            rt.block_on(async {
                let registry = FixtureRegistry::new();
                black_box(registry.load_fixture_from_yaml(fixture_config).await).unwrap();
            });
        });
    });

    c.bench_function("fixture_upload_validation", |b| {
        let registry = {
            let registry = FixtureRegistry::new();
            let fixture_config = r#"
            name: "upload_fixture"
            description: "Upload fixture"
            upload_rules:
              - path_pattern: "/uploads/.*"
                auto_accept: true
                validation:
                  max_size_bytes: 1048576
                storage:
                  type: "memory"
            "#;
            registry.load_fixture_from_yaml(fixture_config).unwrap();
            registry
        };

        b.iter(|| {
            let rule = UploadRule {
                path_pattern: "/uploads/.*".to_string(),
                auto_accept: true,
                validation: None,
                storage: mockforge_ftp::fixtures::StorageConfig::Memory,
            };
            black_box(registry.validate_upload("/uploads/test.txt", 1024, &rule)).unwrap();
        });
    });
}

fn storage_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("storage_write_small_file", |b| {
        let storage = Arc::new(MockStorage::new());

        b.iter(|| {
            rt.block_on(async {
                let data = b"Hello, World!";
                black_box(storage.write_file("/test.txt", data).await).unwrap();
            });
        });
    });

    c.bench_function("storage_write_large_file", |b| {
        let storage = Arc::new(MockStorage::new());
        let large_data = vec![0u8; 1024 * 1024]; // 1MB

        b.iter(|| {
            rt.block_on(async {
                black_box(storage.write_file("/large.bin", &large_data).await).unwrap();
            });
        });
    });

    c.bench_function("storage_read_file", |b| {
        let storage = Arc::new(rt.block_on(async {
            let storage = MockStorage::new();
            storage.write_file("/read_test.txt", b"Benchmark read content").await.unwrap();
            storage
        }));

        b.iter(|| {
            rt.block_on(async {
                black_box(storage.read_file("/read_test.txt").await).unwrap();
            });
        });
    });
}

criterion_group!(benches, vfs_benchmarks, fixture_benchmarks, storage_benchmarks);
criterion_main!(benches);
