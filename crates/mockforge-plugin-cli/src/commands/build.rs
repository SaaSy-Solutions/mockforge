//! Build plugin command

use crate::utils::{check_cargo, check_wasm_target, current_dir, install_wasm_target};
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

pub async fn build_plugin(path: Option<&Path>, release: bool) -> Result<()> {
    // Check prerequisites
    check_cargo().context("Cargo not found")?;

    // Check if wasm32-wasi target is installed
    if !check_wasm_target()? {
        println!(
            "{}",
            "wasm32-wasi target not found. Installing...".yellow()
        );
        install_wasm_target()?;
    }

    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    // Change to project directory
    std::env::set_current_dir(&project_dir)
        .with_context(|| format!("Failed to change to directory {}", project_dir.display()))?;

    // Build the plugin
    println!("{}", "Building plugin WASM module...".cyan().bold());
    println!(
        "  {} {}",
        "Profile:".bold(),
        if release { "release" } else { "debug" }
    );
    println!("  {} wasm32-wasi", "Target:".bold());
    println!();

    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--target").arg("wasm32-wasi");

    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .context("Failed to execute cargo build")?;

    if !status.success() {
        anyhow::bail!("Build failed");
    }

    // Print output location
    let profile = if release { "release" } else { "debug" };
    let output_path = project_dir
        .join("target")
        .join("wasm32-wasi")
        .join(profile);

    println!();
    println!("{}", "Build successful!".green().bold());
    println!("  {} {}", "Output:".bold(), output_path.display());

    Ok(())
}
