//! Package plugin command

use crate::utils::{current_dir, find_manifest, get_plugin_id, get_wasm_output_path, read_manifest};
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
    let release_path = get_wasm_output_path(project_dir, true)?
        .join(format!("{}.wasm", plugin_lib_name));
    if release_path.exists() {
        return Ok(release_path);
    }

    // Try debug
    let debug_path = get_wasm_output_path(project_dir, false)?
        .join(format!("{}.wasm", plugin_lib_name));
    if debug_path.exists() {
        return Ok(debug_path);
    }

    anyhow::bail!(
        "WASM file not found. Run 'mockforge-plugin build' first.\nExpected: {}",
        release_path.display()
    )
}

fn create_plugin_archive(
    manifest_path: &Path,
    wasm_path: &Path,
    output_path: &Path,
) -> Result<()> {
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

    zip.finish()
        .context("Failed to finalize ZIP archive")?;

    Ok(())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    file_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open {}", file_path.display()))?;

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
