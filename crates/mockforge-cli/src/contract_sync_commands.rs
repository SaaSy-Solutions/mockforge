//! Pillars: [Contracts]
//!
//! Contract Sync Commands
//!
//! Commands for validating mocks against Git-hosted OpenAPI specs and syncing them.

use mockforge_core::{
    contract_validation::{BreakingChange, ContractValidator, ValidationError, ValidationResult},
    git_watch::{GitWatchConfig, GitWatchService},
    openapi::OpenApiSpec,
    Error, Result,
};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Handle the contract-sync command
pub async fn handle_contract_sync(
    repository_url: String,
    branch: Option<String>,
    spec_paths: Vec<String>,
    mock_config: Option<PathBuf>,
    auth_token: Option<String>,
    cache_dir: Option<PathBuf>,
    strict: bool,
    output: Option<PathBuf>,
    update: bool,
) -> Result<()> {
    info!("Starting contract sync");

    // Build Git watch configuration
    let git_config = GitWatchConfig {
        repository_url,
        branch: branch.unwrap_or_else(|| "main".to_string()),
        spec_paths: if spec_paths.is_empty() {
            vec![
                "**/*.yaml".to_string(),
                "**/*.json".to_string(),
                "**/openapi*.yaml".to_string(),
                "**/openapi*.json".to_string(),
            ]
        } else {
            spec_paths
        },
        poll_interval_seconds: 60, // Not used for one-time sync
        auth_token: auth_token.clone(),
        cache_dir: cache_dir.unwrap_or_else(|| PathBuf::from("./.mockforge-git-cache")),
        enabled: true,
    };

    // Initialize Git watch service
    let mut git_service = GitWatchService::new(git_config)?;
    git_service.initialize().await?;

    // Get OpenAPI spec files from Git
    let spec_files = git_service.get_spec_files()?;
    if spec_files.is_empty() {
        return Err(Error::generic("No OpenAPI spec files found in repository".to_string()));
    }

    info!("Found {} OpenAPI spec file(s) in Git repository", spec_files.len());

    // Load and validate each spec file
    let mut overall_result = ValidationResult::new();
    let validator = ContractValidator::new().with_strict_mode(strict);

    for spec_file in &spec_files {
        info!("Validating against spec: {}", spec_file.display());

        // Load OpenAPI spec from Git
        let git_spec = load_openapi_spec(spec_file).await?;

        // Load mock configuration if provided
        if let Some(ref mock_config_path) = mock_config {
            let mock_spec = load_openapi_spec(mock_config_path).await?;
            let comparison_result = validator.compare_specs(&mock_spec, &git_spec);
            merge_validation_results(&mut overall_result, comparison_result);
        } else {
            // If no mock config, just validate the Git spec itself
            info!("No mock configuration provided, validating Git spec only");
            overall_result.add_success();
        }
    }

    // Generate and display report
    let report = validator.generate_report(&overall_result);
    println!("{}", report);

    // Save report to file if output path specified
    if let Some(ref output_path) = output {
        tokio::fs::write(output_path, &report).await.map_err(|e| {
            Error::generic(format!("Failed to write report to {}: {}", output_path.display(), e))
        })?;
        info!("Validation report saved to: {}", output_path.display());
    }

    // Update mocks if requested and validation passed
    if update {
        if overall_result.passed {
            info!("Validation passed, updating mocks...");
            update_mocks_from_git_specs(&spec_files, mock_config.as_deref()).await?;
        } else {
            warn!("Validation failed. Use --strict=false to update mocks anyway, or fix validation errors first.");
            if !strict {
                info!("Updating mocks despite validation failures (strict mode disabled)...");
                update_mocks_from_git_specs(&spec_files, mock_config.as_deref()).await?;
            }
        }
    }

    // Exit with appropriate code
    if overall_result.passed {
        info!("Contract sync completed successfully");
        Ok(())
    } else {
        Err(Error::generic(format!(
            "Contract validation failed: {} errors, {} breaking changes",
            overall_result.errors.len(),
            overall_result.breaking_changes.len()
        )))
    }
}

/// Load an OpenAPI spec from a file
async fn load_openapi_spec(path: &Path) -> Result<OpenApiSpec> {
    OpenApiSpec::from_file(path).await.map_err(|e| {
        Error::generic(format!("Failed to load OpenAPI spec from {}: {}", path.display(), e))
    })
}

/// Merge validation results into an overall result
fn merge_validation_results(overall: &mut ValidationResult, other: ValidationResult) {
    overall.total_checks += other.total_checks;
    overall.passed_checks += other.passed_checks;
    overall.failed_checks += other.failed_checks;
    overall.errors.extend(other.errors);
    overall.warnings.extend(other.warnings);
    overall.breaking_changes.extend(other.breaking_changes);
    if !other.passed {
        overall.passed = false;
    }
}

/// Update mock configuration from Git-hosted OpenAPI specs
async fn update_mocks_from_git_specs(
    spec_files: &[PathBuf],
    mock_config_path: Option<&Path>,
) -> Result<()> {
    if spec_files.is_empty() {
        return Err(Error::generic("No spec files to update from".to_string()));
    }

    // Use the first spec file as the primary source
    let primary_spec = &spec_files[0];
    info!("Updating mocks from: {}", primary_spec.display());

    // If mock config path is provided, update it
    if let Some(mock_path) = mock_config_path {
        // Copy the Git spec to the mock config location
        tokio::fs::copy(primary_spec, mock_path).await.map_err(|e| {
            Error::generic(format!("Failed to copy spec to {}: {}", mock_path.display(), e))
        })?;
        info!("Mock configuration updated: {}", mock_path.display());
    } else {
        info!("No mock configuration path specified. Use --mock-config to specify where to update mocks.");
        info!("Git spec available at: {}", primary_spec.display());
    }

    Ok(())
}

/// Validate a single mock against a Git-hosted spec
pub async fn validate_mock_against_git_spec(
    mock_spec_path: &Path,
    git_repo_url: &str,
    git_branch: &str,
    git_spec_path: &str,
    auth_token: Option<&str>,
) -> Result<ValidationResult> {
    // Initialize Git service
    let git_config = GitWatchConfig {
        repository_url: git_repo_url.to_string(),
        branch: git_branch.to_string(),
        spec_paths: vec![git_spec_path.to_string()],
        poll_interval_seconds: 60,
        auth_token: auth_token.map(|s| s.to_string()),
        cache_dir: PathBuf::from("./.mockforge-git-cache"),
        enabled: true,
    };

    let mut git_service = GitWatchService::new(git_config)?;
    git_service.initialize().await?;

    // Get spec file
    let spec_files = git_service.get_spec_files()?;
    if spec_files.is_empty() {
        return Err(Error::generic("No spec file found in Git repository".to_string()));
    }

    let git_spec_file = &spec_files[0];

    // Load both specs
    let mock_spec = load_openapi_spec(mock_spec_path).await?;
    let git_spec = load_openapi_spec(git_spec_file).await?;

    // Compare specs
    let validator = ContractValidator::new();
    let result = validator.compare_specs(&mock_spec, &git_spec);

    Ok(result)
}
