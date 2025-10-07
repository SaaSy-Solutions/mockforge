//! Run plugin tests command

use crate::utils::{check_cargo, current_dir};
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

pub async fn run_tests(path: Option<&Path>, test_pattern: Option<&str>) -> Result<()> {
    // Check prerequisites
    check_cargo().context("Cargo not found")?;

    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    // Change to project directory
    std::env::set_current_dir(&project_dir)
        .with_context(|| format!("Failed to change to directory {}", project_dir.display()))?;

    println!("{}", "Running plugin tests...".cyan().bold());
    if let Some(pattern) = test_pattern {
        println!("  {} {}", "Pattern:".bold(), pattern);
    }
    println!();

    // Run tests
    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if let Some(pattern) = test_pattern {
        cmd.arg(pattern);
    }

    // Always show test output
    cmd.arg("--").arg("--nocapture");

    let status = cmd.status().context("Failed to execute cargo test")?;

    if !status.success() {
        anyhow::bail!("Tests failed");
    }

    println!();
    println!("{}", "All tests passed!".green().bold());

    Ok(())
}
