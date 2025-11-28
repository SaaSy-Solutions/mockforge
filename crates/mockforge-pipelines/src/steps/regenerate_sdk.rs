//! Regenerate SDK step
//!
//! Regenerates client SDKs and mock server code for specified languages when schema changes are detected.

use super::{PipelineStepExecutor, StepContext, StepResult};
use anyhow::{Context, Result};
use mockforge_core::{
    codegen::{CodegenConfig, MockDataStrategy},
    OpenApiSpec,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Regenerate SDK step executor
///
/// This step regenerates client SDKs and mock server code for specified languages when schemas change.
pub struct RegenerateSDKStep {
    /// Output directory for generated SDKs
    output_dir: Option<PathBuf>,
}

impl RegenerateSDKStep {
    /// Create a new regenerate SDK step
    #[must_use]
    pub const fn new(output_dir: Option<PathBuf>) -> Self {
        Self { output_dir }
    }
}

impl Default for RegenerateSDKStep {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait::async_trait]
impl PipelineStepExecutor for RegenerateSDKStep {
    fn step_type(&self) -> &'static str {
        "regenerate_sdk"
    }

    async fn execute(&self, context: StepContext) -> Result<StepResult> {
        info!(
            execution_id = %context.execution_id,
            step_name = %context.step_name,
            "Executing regenerate_sdk step"
        );

        // Extract configuration
        let languages = context
            .config
            .get("languages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing 'languages' in step config"))?
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();

        let spec_path = context
            .config
            .get("spec_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Missing 'spec_path' in step config"))?;

        let output_dir = self
            .output_dir
            .clone()
            .or_else(|| {
                context.config.get("output_dir").and_then(|v| v.as_str()).map(PathBuf::from)
            })
            .unwrap_or_else(|| PathBuf::from("./generated-sdks"));

        let mock_data_strategy = context
            .config
            .get("mock_data_strategy")
            .and_then(|v| v.as_str())
            .map_or(MockDataStrategy::ExamplesOrRandom, |s| match s {
                "random" => MockDataStrategy::Random,
                "examples" => MockDataStrategy::Examples,
                "defaults" => MockDataStrategy::Defaults,
                _ => MockDataStrategy::ExamplesOrRandom,
            });

        debug!(
            execution_id = %context.execution_id,
            languages = ?languages,
            spec_path = %spec_path.display(),
            output_dir = %output_dir.display(),
            "Regenerating SDKs for languages"
        );

        // Load OpenAPI specification
        let spec = OpenApiSpec::from_file(&spec_path)
            .await
            .context("Failed to load OpenAPI specification")?;

        // Validate the spec
        spec.validate().context("OpenAPI specification validation failed")?;

        // Create output directory if it doesn't exist
        tokio::fs::create_dir_all(&output_dir)
            .await
            .context("Failed to create output directory")?;

        let mut generated_files = Vec::new();
        let mut errors = Vec::new();

        // Generate code for each language
        for language in &languages {
            let codegen_config = CodegenConfig {
                mock_data_strategy,
                port: context
                    .config
                    .get("port")
                    .and_then(handlebars::JsonValue::as_u64)
                    .map(|p| p as u16),
                enable_cors: context
                    .config
                    .get("enable_cors")
                    .and_then(handlebars::JsonValue::as_bool)
                    .unwrap_or(true),
                default_delay_ms: context
                    .config
                    .get("default_delay_ms")
                    .and_then(handlebars::JsonValue::as_u64),
            };

            match mockforge_core::codegen::generate_mock_server_code(
                &spec,
                language,
                &codegen_config,
            ) {
                Ok(code) => {
                    // Determine file extension based on language
                    let extension = match language.as_str() {
                        "rs" | "rust" => "rs",
                        "ts" | "typescript" => "ts",
                        "js" | "javascript" => "js",
                        _ => {
                            errors.push(format!("Unsupported language: {language}"));
                            continue;
                        }
                    };

                    let output_file = output_dir.join(format!("mock_server.{extension}"));

                    tokio::fs::write(&output_file, code).await.with_context(|| {
                        format!("Failed to write generated code to {}", output_file.display())
                    })?;

                    generated_files.push(output_file.to_string_lossy().to_string());

                    info!(
                        execution_id = %context.execution_id,
                        language = %language,
                        file = %output_file.display(),
                        "Generated SDK code"
                    );
                }
                Err(e) => {
                    error!(
                        execution_id = %context.execution_id,
                        language = %language,
                        error = %e,
                        "Failed to generate SDK code"
                    );
                    errors.push(format!("Failed to generate {language} SDK: {e}"));
                }
            }
        }

        // Prepare output
        let mut output = HashMap::new();
        output.insert(
            "languages".to_string(),
            Value::Array(languages.iter().map(|l| Value::String(l.clone())).collect()),
        );
        output.insert(
            "spec_path".to_string(),
            Value::String(spec_path.to_string_lossy().to_string()),
        );
        output.insert(
            "output_dir".to_string(),
            Value::String(output_dir.to_string_lossy().to_string()),
        );
        output.insert(
            "generated_files".to_string(),
            Value::Array(generated_files.iter().map(|f| Value::String(f.clone())).collect()),
        );
        output.insert(
            "status".to_string(),
            Value::String(
                if errors.is_empty() {
                    "success"
                } else {
                    "partial_success"
                }
                .to_string(),
            ),
        );

        if errors.is_empty() {
            info!(
                execution_id = %context.execution_id,
                files_generated = generated_files.len(),
                "SDK generation completed successfully"
            );
        } else {
            output.insert(
                "errors".to_string(),
                Value::Array(errors.iter().map(|e| Value::String(e.clone())).collect()),
            );
            warn!(
                execution_id = %context.execution_id,
                errors = ?errors,
                "SDK generation completed with errors"
            );
        }

        Ok(StepResult::success_with_output(output))
    }
}
