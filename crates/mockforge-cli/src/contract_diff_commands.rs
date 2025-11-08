//! Contract Diff Commands
//!
//! CLI commands for AI-powered contract diff analysis, including:
//! - Analyzing requests against contract specs
//! - Comparing contract versions
//! - Generating correction patches
//! - CI/CD integration

use mockforge_core::{
    ai_contract_diff::{
        CapturedRequest, ContractDiffAnalyzer, ContractDiffConfig, ContractDiffResult,
    },
    openapi::OpenApiSpec,
    request_capture::{get_global_capture_manager, init_global_capture_manager},
    Error, Result,
};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Handle the contract-diff analyze command
pub async fn handle_contract_diff_analyze(
    spec_path: PathBuf,
    request_path: Option<PathBuf>,
    capture_id: Option<String>,
    output: Option<PathBuf>,
    config: Option<ContractDiffConfig>,
) -> Result<()> {
    info!("Starting contract diff analysis");

    // Load contract specification
    let spec = OpenApiSpec::from_file(&spec_path).await?;
    info!("Loaded contract spec from: {:?}", spec_path);

    // Get the request to analyze
    let request = if let Some(req_path) = request_path {
        // Load request from file
        let request_json = std::fs::read_to_string(&req_path)?;
        let request: CapturedRequest = serde_json::from_str(&request_json)
            .map_err(|e| Error::generic(format!("Failed to parse request file: {}", e)))?;
        request
    } else if let Some(id) = capture_id {
        // Get request from capture manager
        init_global_capture_manager(1000);
        let manager = get_global_capture_manager()
            .ok_or_else(|| Error::generic("Capture manager not initialized"))?;
        let (request, _) = manager
            .get_capture(&id)
            .await
            .ok_or_else(|| Error::generic(format!("Capture not found: {}", id)))?;
        request
    } else {
        return Err(Error::generic("Either --request-path or --capture-id must be provided"));
    };

    // Create analyzer
    let analyzer_config = config.unwrap_or_else(ContractDiffConfig::default);
    let analyzer = ContractDiffAnalyzer::new(analyzer_config)?;

    // Analyze
    info!("Analyzing request against contract...");
    let result = analyzer.analyze(&request, &spec).await?;

    // Output results
    if let Some(output_path) = output {
        let output_json = serde_json::to_string_pretty(&result)?;
        std::fs::write(&output_path, output_json)?;
        info!("Results written to: {:?}", output_path);
    } else {
        // Print to stdout
        print_analysis_results(&result);
    }

    // Exit with error code if mismatches found
    if !result.matches {
        warn!("Contract mismatches detected!");
        std::process::exit(1);
    }

    info!("Contract analysis completed successfully");
    Ok(())
}

/// Handle the contract-diff compare command
pub async fn handle_contract_diff_compare(
    old_spec_path: PathBuf,
    new_spec_path: PathBuf,
    output: Option<PathBuf>,
) -> Result<()> {
    info!("Comparing contract specifications");

    let old_spec = OpenApiSpec::from_file(&old_spec_path).await?;
    let new_spec = OpenApiSpec::from_file(&new_spec_path).await?;

    info!("Loaded old spec from: {:?}", old_spec_path);
    info!("Loaded new spec from: {:?}", new_spec_path);

    // Use existing contract validator for comparison
    let validator = mockforge_core::contract_validation::ContractValidator::new();
    let result = validator.compare_specs(&old_spec, &new_spec);

    // Output results
    if let Some(output_path) = output {
        let report = validator.generate_report(&result);
        std::fs::write(&output_path, report)?;
        info!("Comparison report written to: {:?}", output_path);
    } else {
        // Print to stdout
        let report = validator.generate_report(&result);
        println!("{}", report);
    }

    // Exit with error code if breaking changes found
    if !result.passed {
        warn!("Breaking changes detected!");
        std::process::exit(1);
    }

    info!("Contract comparison completed successfully");
    Ok(())
}

/// Handle the contract-diff generate-patch command
pub async fn handle_contract_diff_generate_patch(
    spec_path: PathBuf,
    request_path: Option<PathBuf>,
    capture_id: Option<String>,
    output: PathBuf,
    config: Option<ContractDiffConfig>,
) -> Result<()> {
    info!("Generating correction patch");

    // Load contract specification
    let spec = OpenApiSpec::from_file(&spec_path).await?;

    // Get the request
    let request = if let Some(req_path) = request_path {
        let request_json = std::fs::read_to_string(&req_path)?;
        let request: CapturedRequest = serde_json::from_str(&request_json)
            .map_err(|e| Error::generic(format!("Failed to parse request file: {}", e)))?;
        request
    } else if let Some(id) = capture_id {
        init_global_capture_manager(1000);
        let manager = get_global_capture_manager()
            .ok_or_else(|| Error::generic("Capture manager not initialized"))?;
        let (request, _) = manager
            .get_capture(&id)
            .await
            .ok_or_else(|| Error::generic(format!("Capture not found: {}", id)))?;
        request
    } else {
        return Err(Error::generic("Either --request-path or --capture-id must be provided"));
    };

    // Analyze
    let analyzer_config = config.unwrap_or_else(ContractDiffConfig::default);
    let analyzer = ContractDiffAnalyzer::new(analyzer_config)?;
    let result = analyzer.analyze(&request, &spec).await?;

    // Generate patch file
    if result.corrections.is_empty() {
        warn!("No corrections to generate");
        return Ok(());
    }

    let spec_version = if spec.spec.info.version.is_empty() {
        "1.0.0".to_string()
    } else {
        spec.spec.info.version.clone()
    };
    let patch_file = analyzer.generate_patch_file(&result.corrections, &spec_version);

    // Write patch file
    let patch_json = serde_json::to_string_pretty(&patch_file)?;
    std::fs::write(&output, patch_json)?;
    info!("Patch file written to: {:?}", output);
    info!("Generated {} corrections", result.corrections.len());

    Ok(())
}

/// Handle the contract-diff apply-patch command
pub async fn handle_contract_diff_apply_patch(
    spec_path: PathBuf,
    patch_path: PathBuf,
    output: Option<PathBuf>,
) -> Result<()> {
    info!("Applying correction patch to contract");

    // Load contract specification
    let spec = mockforge_core::openapi::OpenApiSpec::from_file(&spec_path).await?;
    let mut spec_json = spec
        .raw_document
        .ok_or_else(|| Error::generic("Spec does not have raw document"))?;

    // Load patch file
    let patch_content = std::fs::read_to_string(&patch_path)?;
    let patch_file: serde_json::Value = serde_json::from_str(&patch_content)
        .map_err(|e| Error::generic(format!("Failed to parse patch file: {}", e)))?;

    // Apply patch operations
    if let Some(operations) = patch_file.get("operations").and_then(|v| v.as_array()) {
        for op in operations {
            apply_patch_operation(&mut spec_json, op)?;
        }
    } else {
        return Err(Error::generic("Invalid patch file format"));
    }

    // Write updated spec
    let output_path = output.unwrap_or(spec_path);
    let updated_json = serde_json::to_string_pretty(&spec_json)?;
    std::fs::write(&output_path, updated_json)?;
    info!("Updated contract spec written to: {:?}", output_path);

    Ok(())
}

/// Apply a single patch operation to the spec
fn apply_patch_operation(spec: &mut serde_json::Value, op: &serde_json::Value) -> Result<()> {
    let op_type = op
        .get("op")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::generic("Missing 'op' field in patch operation"))?;

    let path = op
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::generic("Missing 'path' field in patch operation"))?;

    // Parse JSON Pointer path
    let path_parts: Vec<String> = path
        .trim_start_matches('/')
        .split('/')
        .map(|p| p.replace("~1", "/").replace("~0", "~"))
        .collect();

    match op_type {
        "add" => {
            let value = op
                .get("value")
                .ok_or_else(|| Error::generic("Missing 'value' for add operation"))?;
            add_to_path(spec, &path_parts, value)?;
        }
        "remove" => {
            remove_from_path(spec, &path_parts)?;
        }
        "replace" => {
            let value = op
                .get("value")
                .ok_or_else(|| Error::generic("Missing 'value' for replace operation"))?;
            replace_at_path(spec, &path_parts, value)?;
        }
        _ => {
            return Err(Error::generic(format!("Unsupported patch operation: {}", op_type)));
        }
    }

    Ok(())
}

/// Add value at JSON Pointer path
fn add_to_path(
    spec: &mut serde_json::Value,
    path_parts: &[String],
    value: &serde_json::Value,
) -> Result<()> {
    let mut current = spec;
    for (idx, part) in path_parts.iter().enumerate() {
        if idx == path_parts.len() - 1 {
            // Last part - add here
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.clone(), value.clone());
            } else {
                return Err(Error::generic("Cannot add to non-object"));
            }
        } else {
            // Navigate deeper
            current = current
                .get_mut(part)
                .ok_or_else(|| Error::generic(format!("Path not found: {}", part)))?;
        }
    }
    Ok(())
}

/// Remove value at JSON Pointer path
fn remove_from_path(spec: &mut serde_json::Value, path_parts: &[String]) -> Result<()> {
    let mut current = spec;
    for (idx, part) in path_parts.iter().enumerate() {
        if idx == path_parts.len() - 1 {
            // Last part - remove here
            if let Some(obj) = current.as_object_mut() {
                obj.remove(part);
            } else {
                return Err(Error::generic("Cannot remove from non-object"));
            }
        } else {
            current = current
                .get_mut(part)
                .ok_or_else(|| Error::generic(format!("Path not found: {}", part)))?;
        }
    }
    Ok(())
}

/// Replace value at JSON Pointer path
fn replace_at_path(
    spec: &mut serde_json::Value,
    path_parts: &[String],
    value: &serde_json::Value,
) -> Result<()> {
    let mut current = spec;
    for (idx, part) in path_parts.iter().enumerate() {
        if idx == path_parts.len() - 1 {
            // Last part - replace here
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.clone(), value.clone());
            } else {
                return Err(Error::generic("Cannot replace in non-object"));
            }
        } else {
            current = current
                .get_mut(part)
                .ok_or_else(|| Error::generic(format!("Path not found: {}", part)))?;
        }
    }
    Ok(())
}

/// Print analysis results to stdout
fn print_analysis_results(result: &ContractDiffResult) {
    println!("Contract Diff Analysis Results");
    println!("==============================");
    println!();
    println!(
        "Status: {}",
        if result.matches {
            "✓ MATCHES"
        } else {
            "✗ MISMATCHES"
        }
    );
    println!("Confidence: {:.2}%", result.confidence * 100.0);
    println!("Mismatches: {}", result.mismatches.len());
    println!();

    if !result.mismatches.is_empty() {
        println!("Mismatches:");
        for (idx, mismatch) in result.mismatches.iter().enumerate() {
            println!("  {}. {} - {}", idx + 1, mismatch.path, mismatch.description);
            println!("     Type: {:?}", mismatch.mismatch_type);
            println!("     Severity: {:?}", mismatch.severity);
            println!("     Confidence: {:.2}%", mismatch.confidence * 100.0);
            if let Some(expected) = &mismatch.expected {
                println!("     Expected: {}", expected);
            }
            if let Some(actual) = &mismatch.actual {
                println!("     Actual: {}", actual);
            }
            println!();
        }
    }

    if !result.recommendations.is_empty() {
        println!("Recommendations:");
        for (idx, rec) in result.recommendations.iter().enumerate() {
            println!("  {}. {}", idx + 1, rec.recommendation);
            if let Some(fix) = &rec.suggested_fix {
                println!("     Suggested Fix: {}", fix);
            }
            println!("     Confidence: {:.2}%", rec.confidence * 100.0);
            println!();
        }
    }

    if !result.corrections.is_empty() {
        println!("Corrections Available: {}", result.corrections.len());
        println!("  Use 'contract-diff generate-patch' to create a patch file");
    }
}
