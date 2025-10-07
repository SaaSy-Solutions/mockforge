//! Clean build artifacts command

use crate::utils::current_dir;
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

pub async fn clean_artifacts(path: Option<&Path>) -> Result<()> {
    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Cleaning build artifacts...".cyan().bold());

    // Change to project directory
    std::env::set_current_dir(&project_dir)
        .with_context(|| format!("Failed to change to directory {}", project_dir.display()))?;

    // Run cargo clean
    let status = Command::new("cargo")
        .arg("clean")
        .status()
        .context("Failed to execute cargo clean")?;

    if !status.success() {
        anyhow::bail!("Cargo clean failed");
    }

    // Also remove any .zip files in the root
    let zip_pattern = project_dir.join("*.zip");
    if let Ok(entries) = glob::glob(zip_pattern.to_str().unwrap_or("")) {
        for entry in entries.flatten() {
            if let Err(e) = std::fs::remove_file(&entry) {
                println!(
                    "{} Failed to remove {}: {}",
                    "⚠".yellow(),
                    entry.display(),
                    e
                );
            } else {
                println!("{} Removed {}", "✓".green(), entry.display());
            }
        }
    }

    println!();
    println!("{}", "Build artifacts cleaned!".green().bold());

    Ok(())
}
