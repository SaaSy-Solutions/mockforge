//! Show plugin information command

use crate::utils::{
    current_dir, find_cargo_toml, find_manifest, get_plugin_id, get_plugin_version, read_manifest,
};
use anyhow::Result;
use colored::*;
use std::path::Path;

pub async fn show_plugin_info(path: Option<&Path>) -> Result<()> {
    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Plugin Information".cyan().bold());
    println!("{}", "==================".cyan().bold());
    println!();

    // Read manifest
    let manifest_path = find_manifest(&project_dir)?;
    let manifest = read_manifest(&manifest_path)?;

    // Display basic information
    if let Ok(id) = get_plugin_id(&manifest) {
        println!("{:>15}: {}", "ID".bold(), id);
    }

    if let Ok(version) = get_plugin_version(&manifest) {
        println!("{:>15}: {}", "Version".bold(), version);
    }

    if let Some(name) = manifest.get("name").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Name".bold(), name);
    }

    if let Some(desc) = manifest.get("description").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Description".bold(), desc);
    }

    if let Some(plugin_type) = manifest.get("plugin_type").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Type".bold(), plugin_type);
    }

    // Author information
    if let Some(author) = manifest.get("author") {
        if let Some(name) = author.get("name").and_then(|v| v.as_str()) {
            println!("{:>15}: {}", "Author".bold(), name);
        }
        if let Some(email) = author.get("email").and_then(|v| v.as_str()) {
            println!("{:>15}: {}", "Email".bold(), email);
        }
    }

    // Capabilities
    if let Some(caps) = manifest.get("capabilities") {
        println!();
        println!("{}", "Capabilities:".bold());

        if let Some(network) = caps.get("network").and_then(|v| v.as_bool()) {
            println!("  {:>13}: {}", "Network", if network { "✓".green() } else { "✗".red() });
        }

        if let Some(fs) = caps.get("filesystem").and_then(|v| v.as_bool()) {
            println!("  {:>13}: {}", "Filesystem", if fs { "✓".green() } else { "✗".red() });
        }
    }

    // Resource limits
    if let Some(limits) = manifest.get("resource_limits") {
        println!();
        println!("{}", "Resource Limits:".bold());

        if let Some(mem) = limits.get("max_memory_bytes").and_then(|v| v.as_u64()) {
            let mem_mb = mem as f64 / 1024.0 / 1024.0;
            println!("  {:>13}: {:.1} MB", "Memory", mem_mb);
        }

        if let Some(cpu) = limits.get("max_cpu_time_ms").and_then(|v| v.as_u64()) {
            let cpu_sec = cpu as f64 / 1000.0;
            println!("  {:>13}: {:.1} seconds", "CPU Time", cpu_sec);
        }
    }

    // File locations
    println!();
    println!("{}", "Files:".bold());
    println!("  {:>13}: {}", "Manifest", manifest_path.display());

    if let Ok(cargo_path) = find_cargo_toml(&project_dir) {
        println!("  {:>13}: {}", "Cargo.toml", cargo_path.display());
    }

    // Check for built WASM
    let plugin_id = get_plugin_id(&manifest).unwrap_or_else(|_| "unknown".to_string());
    let plugin_lib = plugin_id.replace('-', "_");

    let release_wasm = project_dir
        .join("target")
        .join("wasm32-wasi")
        .join("release")
        .join(format!("{}.wasm", plugin_lib));

    let debug_wasm = project_dir
        .join("target")
        .join("wasm32-wasi")
        .join("debug")
        .join(format!("{}.wasm", plugin_lib));

    println!();
    println!("{}", "Build Status:".bold());

    if release_wasm.exists() {
        println!("  {:>13}: {} ({})", "Release", "✓".green(), release_wasm.display());
    } else {
        println!("  {:>13}: {}", "Release", "Not built".yellow());
    }

    if debug_wasm.exists() {
        println!("  {:>13}: {} ({})", "Debug", "✓".green(), debug_wasm.display());
    } else {
        println!("  {:>13}: {}", "Debug", "Not built".yellow());
    }

    Ok(())
}
