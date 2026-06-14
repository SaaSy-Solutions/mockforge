//! Build script for gRPC code generation
//!
//! This build script automatically discovers and compiles Protocol Buffer (.proto) files
//! in the proto directory, generating Rust code using tonic and prost.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Round 32 (#79 / Srikanth on 0.3.176): Ubuntu 16.04 boxes don't
    // ship `protobuf-compiler`, so `cargo install mockforge-cli`
    // failed with "Could not find `protoc`". Use the vendored binary
    // from `protoc-bin-vendored` when the user hasn't already set
    // `PROTOC` themselves. The crate ships per-target binaries inside
    // its source tarball, so cargo install picks it up automatically.
    let user_protoc = env::var("PROTOC").ok().filter(|s| !s.is_empty());
    if user_protoc.is_none() {
        match protoc_bin_vendored::protoc_bin_path() {
            Ok(p) => {
                // SAFETY: build scripts run single-threaded, so this
                // env mutation is sound. tonic-prost-build reads
                // PROTOC at configure() time later in this main(),
                // so it must be set before then.
                unsafe {
                    env::set_var("PROTOC", &p);
                }
                println!("cargo:rerun-if-env-changed=PROTOC");
                println!("cargo:info=using vendored protoc at {}", p.display());
            }
            Err(e) => {
                println!(
                    "cargo:warning=protoc-bin-vendored returned no binary ({e}); falling back to system PATH"
                );
            }
        }
    }

    // Get proto directory from environment variable or use default
    let proto_dir =
        env::var("MOCKFORGE_PROTO_DIR").unwrap_or_else(|_| format!("{}/proto", manifest_dir));

    let proto_path = Path::new(&proto_dir);

    if !proto_path.exists() {
        println!("cargo:warning=Proto directory does not exist: {}", proto_dir);
        return;
    }

    // Discover all .proto files in the directory and subdirectories
    let proto_files = discover_proto_files(proto_path);

    if proto_files.is_empty() {
        println!("cargo:warning=No .proto files found in directory: {}", proto_dir);
        return;
    }

    println!("cargo:info=Found {} proto files: {:?}", proto_files.len(), proto_files);

    // Configure tonic build
    let descriptor_path = format!("{}/file_descriptor_set.bin", out_dir);
    let config = tonic_prost_build::configure()
        .out_dir(&out_dir)
        .file_descriptor_set_path(&descriptor_path)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[allow(missing_docs)]");

    // Add include paths for all proto files
    let include_paths: Vec<String> = proto_files
        .iter()
        .map(|path_str| {
            Path::new(path_str)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_string_lossy()
                .to_string()
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Compile all proto files
    match config.compile_protos(&proto_files, &include_paths) {
        Ok(_) => {
            println!("cargo:info=Successfully compiled {} proto files", proto_files.len());

            // Tell cargo to rerun if any proto file changes
            for proto_file in &proto_files {
                println!("cargo:rerun-if-changed={}", proto_file);
            }

            // Also watch the proto directory for new files
            println!("cargo:rerun-if-changed={}", proto_dir);
        }
        Err(e) => {
            panic!("Failed to compile proto files: {}", e);
        }
    }
}

/// Recursively discover all .proto files in a directory
fn discover_proto_files(dir: &Path) -> Vec<String> {
    let mut proto_files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively search subdirectories
                proto_files.extend(discover_proto_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
                // Found a .proto file
                proto_files.push(path.to_string_lossy().to_string());
            }
        }
    }

    proto_files
}
