//! Package plugin command

use crate::utils::{
    current_dir, find_manifest, get_plugin_id, get_wasm_output_path, read_manifest,
};
use anyhow::{Context, Result};
use colored::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

pub async fn package_plugin(path: Option<&Path>, output: Option<&Path>) -> Result<PathBuf> {
    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Packaging plugin...".cyan().bold());

    // Find and read manifest
    let manifest_path = find_manifest(&project_dir)?;
    let manifest = read_manifest(&manifest_path)?;
    let plugin_id = get_plugin_id(&manifest)?;

    // Find WASM file (try release first, then debug)
    let wasm_path = find_wasm_file(&project_dir, &plugin_id)?;

    println!("  {} {}", "Plugin ID:".bold(), plugin_id);
    println!("  {} {}", "Manifest:".bold(), manifest_path.display());
    println!("  {} {}", "WASM:".bold(), wasm_path.display());

    // Determine output path
    let output_path = if let Some(out) = output {
        out.to_path_buf()
    } else {
        project_dir.join(format!("{}.zip", plugin_id))
    };

    // Create ZIP archive
    create_plugin_archive(&manifest_path, &wasm_path, &output_path)?;

    // Calculate checksum
    let checksum = calculate_checksum(&output_path)?;

    println!();
    println!("{}", "✓ Plugin packaged successfully!".green().bold());
    println!("  {} {}", "Output:".bold(), output_path.display());
    println!("  {} {}", "SHA-256:".bold(), checksum);
    println!();
    println!("{}", "Install with:".bold());
    println!("  mockforge plugin install {}", output_path.display());

    Ok(output_path)
}

fn find_wasm_file(project_dir: &Path, plugin_id: &str) -> Result<PathBuf> {
    let plugin_lib_name = plugin_id.replace('-', "_");

    // Try release first
    let release_path =
        get_wasm_output_path(project_dir, true)?.join(format!("{}.wasm", plugin_lib_name));
    if release_path.exists() {
        return Ok(release_path);
    }

    // Try debug
    let debug_path =
        get_wasm_output_path(project_dir, false)?.join(format!("{}.wasm", plugin_lib_name));
    if debug_path.exists() {
        return Ok(debug_path);
    }

    anyhow::bail!(
        "WASM file not found. Run 'mockforge-plugin build' first.\nExpected: {}",
        release_path.display()
    )
}

fn create_plugin_archive(manifest_path: &Path, wasm_path: &Path, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create archive at {}", output_path.display()))?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Add manifest
    add_file_to_zip(&mut zip, manifest_path, "plugin.yaml", options)?;

    // Add WASM file
    let wasm_filename = wasm_path
        .file_name()
        .context("Invalid WASM path")?
        .to_str()
        .context("Invalid WASM filename")?;
    add_file_to_zip(&mut zip, wasm_path, wasm_filename, options)?;

    zip.finish().context("Failed to finalize ZIP archive")?;

    Ok(())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    file_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut file =
        File::open(file_path).with_context(|| format!("Failed to open {}", file_path.display()))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    zip.start_file(archive_path, options)
        .with_context(|| format!("Failed to add {} to archive", archive_path))?;

    zip.write_all(&buffer)
        .with_context(|| format!("Failed to write {} to archive", archive_path))?;

    Ok(())
}

fn calculate_checksum(file_path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open {} for checksum", file_path.display()))?;

    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_plugin_project(dir: &Path) -> (PathBuf, PathBuf) {
        let manifest_path = dir.join("plugin.yaml");
        fs::write(&manifest_path, "id: test-plugin\nversion: 1.0.0\nname: Test Plugin").unwrap();

        let wasm_dir = dir.join("target/wasm32-wasi/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        let wasm_path = wasm_dir.join("test_plugin.wasm");
        fs::write(&wasm_path, b"fake wasm content").unwrap();

        (manifest_path, wasm_path)
    }

    #[test]
    fn test_find_wasm_file_release() {
        let temp_dir = TempDir::new().unwrap();
        let wasm_dir = temp_dir.path().join("target/wasm32-wasi/release");
        fs::create_dir_all(&wasm_dir).unwrap();

        let wasm_path = wasm_dir.join("my_plugin.wasm");
        fs::write(&wasm_path, b"test").unwrap();

        let result = find_wasm_file(temp_dir.path(), "my-plugin");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm_path);
    }

    #[test]
    fn test_find_wasm_file_debug_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let wasm_dir = temp_dir.path().join("target/wasm32-wasi/debug");
        fs::create_dir_all(&wasm_dir).unwrap();

        let wasm_path = wasm_dir.join("my_plugin.wasm");
        fs::write(&wasm_path, b"test").unwrap();

        let result = find_wasm_file(temp_dir.path(), "my-plugin");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), wasm_path);
    }

    #[test]
    fn test_find_wasm_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_wasm_file(temp_dir.path(), "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("WASM file not found"));
    }

    #[test]
    fn test_find_wasm_file_prefers_release() {
        let temp_dir = TempDir::new().unwrap();

        let release_dir = temp_dir.path().join("target/wasm32-wasi/release");
        fs::create_dir_all(&release_dir).unwrap();
        let release_path = release_dir.join("test.wasm");
        fs::write(&release_path, b"release").unwrap();

        let debug_dir = temp_dir.path().join("target/wasm32-wasi/debug");
        fs::create_dir_all(&debug_dir).unwrap();
        let debug_path = debug_dir.join("test.wasm");
        fs::write(&debug_path, b"debug").unwrap();

        let result = find_wasm_file(temp_dir.path(), "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), release_path);
    }

    #[test]
    fn test_calculate_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        let result = calculate_checksum(&file_path);
        assert!(result.is_ok());

        let checksum = result.unwrap();
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_calculate_checksum_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"same content").unwrap();

        let checksum1 = calculate_checksum(&file_path).unwrap();
        let checksum2 = calculate_checksum(&file_path).unwrap();

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_different_content() {
        let temp_dir = TempDir::new().unwrap();

        let file1 = temp_dir.path().join("file1.txt");
        fs::write(&file1, b"content1").unwrap();

        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file2, b"content2").unwrap();

        let checksum1 = calculate_checksum(&file1).unwrap();
        let checksum2 = calculate_checksum(&file2).unwrap();

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_calculate_checksum_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let result = calculate_checksum(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_plugin_archive() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_path = temp_dir.path().join("plugin.yaml");
        fs::write(&manifest_path, "id: test\nversion: 1.0.0").unwrap();

        let wasm_path = temp_dir.path().join("plugin.wasm");
        fs::write(&wasm_path, b"fake wasm").unwrap();

        let output_path = temp_dir.path().join("output.zip");

        let result = create_plugin_archive(&manifest_path, &wasm_path, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_create_plugin_archive_invalid_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_path = temp_dir.path().join("nonexistent.yaml");
        let wasm_path = temp_dir.path().join("plugin.wasm");
        fs::write(&wasm_path, b"test").unwrap();

        let output_path = temp_dir.path().join("output.zip");

        let result = create_plugin_archive(&manifest_path, &wasm_path, &output_path);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_package_plugin() {
        let temp_dir = TempDir::new().unwrap();
        create_test_plugin_project(temp_dir.path());

        let result = package_plugin(Some(temp_dir.path()), None).await;
        assert!(result.is_ok());

        let output_path = result.unwrap();
        assert!(output_path.exists());
        assert!(output_path.to_str().unwrap().ends_with("test-plugin.zip"));
    }

    #[tokio::test]
    async fn test_package_plugin_custom_output() {
        let temp_dir = TempDir::new().unwrap();
        create_test_plugin_project(temp_dir.path());

        let custom_output = temp_dir.path().join("custom-name.zip");
        let result = package_plugin(Some(temp_dir.path()), Some(&custom_output)).await;
        assert!(result.is_ok());

        let output_path = result.unwrap();
        assert_eq!(output_path, custom_output);
        assert!(output_path.exists());
    }

    #[tokio::test]
    async fn test_package_plugin_no_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let result = package_plugin(Some(temp_dir.path()), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_package_plugin_no_wasm() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("plugin.yaml");
        fs::write(&manifest_path, "id: test-plugin\nversion: 1.0.0").unwrap();

        let result = package_plugin(Some(temp_dir.path()), None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("WASM file not found"));
    }

    #[test]
    fn test_add_file_to_zip() {
        use std::fs::File;
        use zip::write::{SimpleFileOptions, ZipWriter};
        use zip::CompressionMethod;

        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("source.txt");
        fs::write(&source_file, b"test content").unwrap();

        let zip_path = temp_dir.path().join("test.zip");
        let zip_file = File::create(&zip_path).unwrap();
        let mut zip_writer = ZipWriter::new(zip_file);

        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let result = add_file_to_zip(&mut zip_writer, &source_file, "archived.txt", options);
        assert!(result.is_ok());

        zip_writer.finish().unwrap();
        assert!(zip_path.exists());
    }
}
