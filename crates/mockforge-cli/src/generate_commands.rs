//! Generate, Schema, and Init commands
//!
//! CLI commands for mock generation, schema management, and project initialization.

use clap::Subcommand;
use mockforge_core::{
    build_file_naming_context, process_generated_file, BarrelGenerator, GeneratedFile, OpenApiSpec,
};
use std::path::PathBuf;

use crate::progress;

/// Schema generation commands
#[derive(Subcommand, Debug)]
pub(crate) enum SchemaCommands {
    /// Generate all JSON Schemas for MockForge configuration files
    ///
    /// Generates schemas for:
    /// - Main config (mockforge.yaml)
    /// - Reality configuration
    /// - Persona configuration
    /// - Blueprint metadata
    ///
    /// Examples:
    ///   mockforge schema generate
    ///   mockforge schema generate --output schemas/
    ///   mockforge schema generate --type config
    Generate {
        /// Output directory or file path
        /// If directory, generates all schemas with standard names
        /// If file, generates only the specified schema type
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Schema type to generate (config, reality, persona, blueprint, all)
        /// If not specified and output is a file, defaults to 'config'
        /// If not specified and output is a directory, generates all schemas
        #[arg(short, long, default_value = "all")]
        r#type: String,
    },

    /// Validate configuration files against JSON Schemas
    ///
    /// Validates MockForge configuration files against their corresponding
    /// JSON Schemas to catch errors early and ensure config correctness.
    ///
    /// Examples:
    ///   mockforge schema validate mockforge.yaml
    ///   mockforge schema validate --file mockforge.yaml --schema-type config
    ///   mockforge schema validate --directory . --schema-dir schemas/
    Validate {
        /// Config file to validate (mutually exclusive with --directory)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Directory containing config files to validate (mutually exclusive with --file)
        #[arg(short, long)]
        directory: Option<PathBuf>,

        /// Schema type to use for validation (config, reality, persona, blueprint)
        /// If not specified, will attempt to auto-detect from file path
        #[arg(long)]
        schema_type: Option<String>,

        /// Directory containing schema files (default: looks for schemas/ in current directory)
        #[arg(long)]
        schema_dir: Option<PathBuf>,

        /// Exit with error code if validation fails (useful for CI)
        #[arg(long)]
        strict: bool,
    },
}

/// Handle JSON Schema generation for MockForge configuration
pub(crate) async fn handle_schema(
    schema_command: Option<SchemaCommands>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::schema::generate_all_schemas;
    use std::fs;
    use std::path::Path;

    // Default to generate all if no subcommand specified
    let command = schema_command.unwrap_or(SchemaCommands::Generate {
        output: None,
        r#type: "all".to_string(),
    });

    match command {
        SchemaCommands::Generate { output, r#type } => {
            let schemas = generate_all_schemas();

            // Determine what to generate
            let types_to_generate: Vec<&str> = if r#type == "all" {
                vec![
                    "mockforge-config",
                    "reality-config",
                    "persona-config",
                    "blueprint-config",
                ]
            } else {
                vec![&r#type]
            };

            if let Some(output_path) = output {
                let output_path = Path::new(&output_path);

                // Check if output is a directory or file
                if output_path.is_dir()
                    || !output_path.exists() && output_path.extension().is_none()
                {
                    // Directory mode: generate all requested schemas
                    fs::create_dir_all(output_path)?;

                    for schema_type in &types_to_generate {
                        if let Some(schema) = schemas.get(*schema_type) {
                            let filename = format!("{}.schema.json", schema_type.replace("-", "_"));
                            let file_path = output_path.join(&filename);
                            let schema_json = serde_json::to_string_pretty(schema)?;
                            fs::write(&file_path, schema_json)?;
                            println!("  \u{2713} Generated: {}", file_path.display());
                        }
                    }

                    println!(
                        "\n\u{2705} Generated {} schema(s) in {}",
                        types_to_generate.len(),
                        output_path.display()
                    );
                    println!("\nTo use in your IDE:");
                    println!("  1. Install a YAML schema extension (e.g., 'YAML' by Red Hat)");
                    println!("  2. Add schema mapping to your VS Code settings.json:");
                    println!("     \"yaml.schemas\": {{");
                    for schema_type in &types_to_generate {
                        let filename = format!("{}.schema.json", schema_type.replace("-", "_"));
                        let schema_path = output_path.join(&filename);
                        let file_pattern = match *schema_type {
                            "mockforge-config" => "mockforge.yaml",
                            "reality-config" => "**/reality*.yaml",
                            "persona-config" => "**/personas/**/*.yaml",
                            "blueprint-config" => "**/blueprint.yaml",
                            _ => "*.yaml",
                        };
                        println!(
                            "       \"{}\": \"{}\",",
                            schema_path.to_string_lossy(),
                            file_pattern
                        );
                    }
                    println!("     }}");
                } else {
                    // File mode: generate single schema
                    let schema_type = if r#type == "all" {
                        "mockforge-config"
                    } else {
                        &r#type
                    };
                    if let Some(schema) = schemas.get(schema_type) {
                        let schema_json = serde_json::to_string_pretty(schema)?;
                        fs::write(output_path, schema_json)?;
                        println!("\u{2705} JSON Schema generated: {}", output_path.display());
                    } else {
                        eprintln!("\u{274c} Unknown schema type: {}", schema_type);
                        eprintln!("Available types: mockforge-config, reality-config, persona-config, blueprint-config");
                        return Err("Invalid schema type".into());
                    }
                }
            } else {
                // No output specified: print to stdout
                if r#type == "all" {
                    println!("Generating all schemas...\n");
                    for schema_type in &types_to_generate {
                        if let Some(schema) = schemas.get(*schema_type) {
                            println!("=== {} ===", schema_type);
                            println!("{}", serde_json::to_string_pretty(schema)?);
                            println!();
                        }
                    }
                } else if let Some(schema) = schemas.get(&r#type) {
                    println!("{}", serde_json::to_string_pretty(schema)?);
                } else {
                    eprintln!("\u{274c} Unknown schema type: {}", r#type);
                    eprintln!("Available types: mockforge-config, reality-config, persona-config, blueprint-config");
                    return Err("Invalid schema type".into());
                }
            }
        }
        SchemaCommands::Validate {
            file,
            directory,
            schema_type,
            schema_dir,
            strict,
        } => {
            use crate::schema::{detect_schema_type, generate_all_schemas, validate_config_file};
            use std::fs;

            let schemas = generate_all_schemas();
            let mut validation_results = Vec::new();
            let mut has_errors = false;

            // Determine schema directory
            let schema_dir_path = schema_dir.or_else(|| {
                let current_dir = std::env::current_dir().ok()?;
                let schemas_dir = current_dir.join("schemas");
                if schemas_dir.exists() {
                    Some(schemas_dir)
                } else {
                    None
                }
            });

            // Collect files to validate
            let files_to_validate: Vec<PathBuf> = if let Some(file_path) = file {
                vec![file_path]
            } else if let Some(dir_path) = directory {
                // Find all YAML/JSON files in directory
                let mut files = Vec::new();
                if let Ok(entries) = fs::read_dir(&dir_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if ext_str == "yaml" || ext_str == "yml" || ext_str == "json" {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
                files
            } else {
                // Default: validate mockforge.yaml in current directory
                let current_dir = std::env::current_dir()?;
                let default_file = current_dir.join("mockforge.yaml");
                if default_file.exists() {
                    vec![default_file]
                } else {
                    eprintln!("\u{274c} No config file specified and mockforge.yaml not found in current directory");
                    eprintln!("   Use --file or --directory to specify files to validate");
                    return Err("No files to validate".into());
                }
            };

            // Validate each file
            for file_path in &files_to_validate {
                // Determine schema type
                let file_schema_type = schema_type.clone().unwrap_or_else(|| {
                    detect_schema_type(file_path).unwrap_or_else(|| "mockforge-config".to_string())
                });

                // Get schema (try from schema_dir first, then use generated)
                let schema = if let Some(ref schema_dir) = schema_dir_path {
                    let schema_file = schema_dir
                        .join(format!("{}.schema.json", file_schema_type.replace("-", "_")));
                    if schema_file.exists() {
                        match fs::read_to_string(&schema_file).and_then(|content| {
                            serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
                                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                            })
                        }) {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!(
                                    "\u{26a0}\u{fe0f}  Failed to load schema from {}: {}",
                                    schema_file.display(),
                                    e
                                );
                                schemas.get(&file_schema_type).cloned().unwrap_or_else(|| {
                                    eprintln!(
                                        "\u{274c} Schema type '{}' not found",
                                        file_schema_type
                                    );
                                    has_errors = true;
                                    serde_json::json!({})
                                })
                            }
                        }
                    } else {
                        schemas.get(&file_schema_type).cloned().unwrap_or_else(|| {
                            eprintln!(
                                "\u{26a0}\u{fe0f}  Schema file not found: {}, using generated schema",
                                schema_file.display()
                            );
                            schemas.get(&file_schema_type).cloned().unwrap_or_else(|| {
                                eprintln!("\u{274c} Schema type '{}' not found", file_schema_type);
                                has_errors = true;
                                serde_json::json!({})
                            })
                        })
                    }
                } else {
                    schemas.get(&file_schema_type).cloned().unwrap_or_else(|| {
                        eprintln!("\u{274c} Schema type '{}' not found", file_schema_type);
                        has_errors = true;
                        serde_json::json!({})
                    })
                };

                // Validate
                match validate_config_file(file_path, &file_schema_type, &schema) {
                    Ok(result) => {
                        validation_results.push(result);
                    }
                    Err(e) => {
                        eprintln!("\u{274c} Failed to validate {}: {}", file_path.display(), e);
                        has_errors = true;
                    }
                }
            }

            // Print results
            println!("\n\u{1f4cb} Validation Results:\n");
            for result in &validation_results {
                if result.valid {
                    println!("  \u{2705} {} (schema: {})", result.file_path, result.schema_type);
                } else {
                    println!("  \u{274c} {} (schema: {})", result.file_path, result.schema_type);
                    for error in &result.errors {
                        println!("     \u{2022} {}", error);
                    }
                    has_errors = true;
                }
            }

            // Summary
            let valid_count = validation_results.iter().filter(|r| r.valid).count();
            let total_count = validation_results.len();

            println!(
                "\n\u{1f4ca} Summary: {} of {} file(s) passed validation",
                valid_count, total_count
            );

            if has_errors {
                if strict {
                    return Err("Validation failed".into());
                } else {
                    eprintln!("\n\u{26a0}\u{fe0f}  Validation completed with errors (use --strict to exit with error code)");
                }
            } else if !validation_results.is_empty() {
                println!("\n\u{2705} All files passed validation!");
            }
        }
    }

    Ok(())
}

/// Handle mock generation from configuration
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_generate(
    config_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
    watch: bool,
    watch_debounce: u64,
    show_progress: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_core::discover_config_file;
    use progress::{CliError, ExitCode, LogLevel, ProgressManager};

    // Initialize progress manager
    let mut progress_mgr = ProgressManager::new(verbose);

    // If watch mode is enabled, set up file watching
    if watch {
        let files_to_watch = if let Some(spec) = &spec_path {
            vec![spec.clone()]
        } else if let Some(config) = &config_path {
            vec![config.clone()]
        } else {
            // Try to discover config file
            match discover_config_file() {
                Ok(path) => vec![path],
                Err(_) => CliError::new(
                    "No configuration file found for watch mode".to_string(),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion(
                    "Provide --config or --spec flag, or create mockforge.toml".to_string(),
                )
                .display_and_exit(),
            }
        };

        progress_mgr.log(LogLevel::Info, "\u{1f504} Starting watch mode...");
        progress_mgr.log(
            LogLevel::Info,
            &format!("\u{1f440} Watching {} file(s) for changes", files_to_watch.len()),
        );

        // Execute initial generation
        if let Err(e) = execute_generation(
            &mut progress_mgr,
            config_path.clone(),
            spec_path.clone(),
            output_path.clone(),
            verbose,
            dry_run,
            show_progress,
        )
        .await
        {
            progress_mgr.log(LogLevel::Error, &format!("Initial generation failed: {}", e));
            return Err(e);
        }

        // Set up watch loop
        let callback = move || {
            let config_path = config_path.clone();
            let spec_path = spec_path.clone();
            let output_path = output_path.clone();
            let verbose = verbose;
            let dry_run = dry_run;
            let progress = show_progress;

            async move {
                let mut progress_mgr = ProgressManager::new(verbose);
                execute_generation(
                    &mut progress_mgr,
                    config_path,
                    spec_path,
                    output_path,
                    verbose,
                    dry_run,
                    progress,
                )
                .await
            }
        };

        progress::watch::watch_files(files_to_watch, callback, watch_debounce).await?;
        return Ok(());
    }

    // Single generation run
    execute_generation(
        &mut progress_mgr,
        config_path,
        spec_path,
        output_path,
        verbose,
        dry_run,
        show_progress,
    )
    .await
}

/// Load and validate a configuration file
async fn load_and_validate_config(
    path: &PathBuf,
    verbose: bool,
    progress_mgr: &mut progress::ProgressManager,
) -> mockforge_core::GenerateConfig {
    use crate::progress::{utils, LogLevel};
    use mockforge_core::load_generate_config_with_fallback;

    if verbose {
        progress_mgr.log(
            LogLevel::Debug,
            &format!("\u{1f4c4} Loading configuration from: {}", path.display()),
        );
    }
    // Validate config file exists
    if let Err(e) = utils::validate_file_path(path) {
        e.display_and_exit();
    }
    load_generate_config_with_fallback(path).await
}

/// Execute the actual generation process with progress tracking
async fn execute_generation(
    progress_mgr: &mut progress::ProgressManager,
    config_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
    show_progress: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_core::{discover_config_file, GenerateConfig};
    use progress::{utils, CliError, ExitCode, LogLevel};
    use std::time::Instant;

    let start_time = Instant::now();

    progress_mgr.log(LogLevel::Info, "\u{1f527} Generating mocks from configuration...");

    // Step 1: Discover or load config file
    let (_config_file, mut config) = if let Some(path) = &config_path {
        let config = load_and_validate_config(path, verbose, progress_mgr).await;
        (Some(path.clone()), config)
    } else {
        match discover_config_file() {
            Ok(path) => {
                let config = load_and_validate_config(&path, verbose, progress_mgr).await;
                (Some(path), config)
            }
            Err(_) => {
                // If no config file found, check if spec_path was provided as CLI argument
                if spec_path.is_none() {
                    progress_mgr.log(
                        LogLevel::Warning,
                        "\u{2139}\u{fe0f}  No configuration file found, using defaults",
                    );
                    CliError::new(
                        "No configuration file found and no spec provided. Please create mockforge.toml, mockforge.yaml, or mockforge.json, or provide --spec flag.".to_string(),
                        ExitCode::ConfigurationError,
                    ).with_suggestion(
                        "Create a configuration file or use --spec to specify an OpenAPI specification".to_string()
                    ).display_and_exit();
                }
                // If spec_path is provided, we can continue without a config file
                progress_mgr.log(
                    LogLevel::Warning,
                    "\u{2139}\u{fe0f}  No configuration file found, using defaults",
                );
                // Use default configuration directly
                (None, GenerateConfig::default())
            }
        }
    };

    // Step 3: Apply CLI argument overrides
    if let Some(spec) = &spec_path {
        config.input.spec = Some(spec.clone());
    }

    if let Some(output) = &output_path {
        config.output.path = output.clone();
    }

    // Step 4: Validate configuration
    // Use require_registry helper (works with references) for better error handling
    let spec = progress::require_registry(&config.input.spec, "spec")?;

    if !spec.exists() {
        CliError::new(
            format!("Specification file not found: {}", spec.display()),
            ExitCode::FileNotFound,
        )
        .with_suggestion("Check the file path and ensure the specification file exists".to_string())
        .display_and_exit();
    }

    // Enhanced validation with detailed error messages
    if verbose {
        progress_mgr.log(LogLevel::Debug, "\u{1f50d} Validating specification...");
    }

    let spec_content = match tokio::fs::read_to_string(spec).await {
        Ok(content) => content,
        Err(e) => CliError::new(
            format!("Failed to read specification file: {}", e),
            ExitCode::FileNotFound,
        )
        .display_and_exit(),
    };

    // Detect format and validate
    let format = match mockforge_core::spec_parser::SpecFormat::detect(&spec_content, Some(spec)) {
        Ok(fmt) => fmt,
        Err(e) => CliError::new(
            format!("Failed to detect specification format: {}", e),
            ExitCode::ConfigurationError,
        )
        .with_suggestion(
            "Ensure your file is a valid OpenAPI, GraphQL, or protobuf specification".to_string(),
        )
        .display_and_exit(),
    };

    if verbose {
        progress_mgr.log(
            LogLevel::Debug,
            &format!("\u{1f4cb} Detected format: {}", format.display_name()),
        );
    }

    // Validate based on format
    match format {
        mockforge_core::spec_parser::SpecFormat::OpenApi20
        | mockforge_core::spec_parser::SpecFormat::OpenApi30
        | mockforge_core::spec_parser::SpecFormat::OpenApi31 => {
            // Optimize parsing: try JSON first, then YAML (avoids double parsing)
            let json_value: serde_json::Value =
                match serde_json::from_str::<serde_json::Value>(&spec_content) {
                    Ok(val) => val,
                    Err(_) => {
                        // Try YAML if JSON parsing fails
                        match serde_yaml::from_str(&spec_content) {
                            Ok(val) => val,
                            Err(e) => CliError::new(
                                format!("Invalid JSON or YAML in OpenAPI spec: {}", e),
                                ExitCode::ConfigurationError,
                            )
                            .display_and_exit(),
                        }
                    }
                };

            let validation =
                mockforge_core::spec_parser::OpenApiValidator::validate(&json_value, format);
            if !validation.is_valid {
                let error_details: Vec<String> = validation
                    .errors
                    .iter()
                    .map(|e| {
                        let mut msg = e.message.clone();
                        if let Some(path) = &e.path {
                            msg.push_str(&format!(" (at {})", path));
                        }
                        if let Some(suggestion) = &e.suggestion {
                            msg.push_str(&format!(". Hint: {}", suggestion));
                        }
                        msg
                    })
                    .collect();

                let error_msg = error_details.join("\n  ");
                CliError::new(
                    format!("Invalid OpenAPI specification:\n  {}", error_msg),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion("Fix the validation errors above and try again".to_string())
                .display_and_exit();
            }

            if !validation.warnings.is_empty() && verbose {
                progress_mgr.log(LogLevel::Warning, "\u{26a0}\u{fe0f}  Validation warnings:");
                for warning in &validation.warnings {
                    progress_mgr.log(LogLevel::Warning, &format!("  - {}", warning));
                }
            }

            if verbose {
                progress_mgr.log(LogLevel::Success, "\u{2705} OpenAPI specification is valid");
            }
        }
        mockforge_core::spec_parser::SpecFormat::GraphQL => {
            let validation = mockforge_core::spec_parser::GraphQLValidator::validate(&spec_content);
            if !validation.is_valid {
                let error_details: Vec<String> = validation
                    .errors
                    .iter()
                    .map(|e| {
                        let mut msg = e.message.clone();
                        if let Some(suggestion) = &e.suggestion {
                            msg.push_str(&format!(". Hint: {}", suggestion));
                        }
                        msg
                    })
                    .collect();

                let error_msg = error_details.join("\n  ");
                CliError::new(
                    format!("Invalid GraphQL schema:\n  {}", error_msg),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion("Fix the validation errors above and try again".to_string())
                .display_and_exit();
            }

            if !validation.warnings.is_empty() && verbose {
                progress_mgr.log(LogLevel::Warning, "\u{26a0}\u{fe0f}  Validation warnings:");
                for warning in &validation.warnings {
                    progress_mgr.log(LogLevel::Warning, &format!("  - {}", warning));
                }
            }

            if verbose {
                progress_mgr.log(LogLevel::Success, "\u{2705} GraphQL schema is valid");
            }
        }
        mockforge_core::spec_parser::SpecFormat::Protobuf => {
            if verbose {
                progress_mgr.log(
                    LogLevel::Info,
                    "\u{1f4cb} Protobuf validation will be performed during parsing",
                );
            }
        }
    }

    // Validate output directory
    if let Err(e) = utils::validate_output_dir(&config.output.path) {
        e.display_and_exit();
    }

    if verbose {
        progress_mgr.log(LogLevel::Debug, &format!("\u{1f4dd} Input spec: {}", spec.display()));
        progress_mgr.log(
            LogLevel::Debug,
            &format!("\u{1f4c2} Output path: {}", config.output.path.display()),
        );
        if let Some(filename) = &config.output.filename {
            progress_mgr.log(LogLevel::Debug, &format!("\u{1f4c4} Output filename: {}", filename));
        }
        if let Some(options) = &config.options {
            progress_mgr
                .log(LogLevel::Debug, &format!("\u{2699}\u{fe0f}  Client: {:?}", options.client));
            progress_mgr
                .log(LogLevel::Debug, &format!("\u{2699}\u{fe0f}  Mode: {:?}", options.mode));
            progress_mgr
                .log(LogLevel::Debug, &format!("\u{2699}\u{fe0f}  Runtime: {:?}", options.runtime));
        }
        if !config.plugins.is_empty() {
            progress_mgr.log(LogLevel::Debug, "\u{1f50c} Plugins:");
            for (name, plugin) in &config.plugins {
                match plugin {
                    mockforge_core::PluginConfig::Simple(pkg) => {
                        progress_mgr.log(LogLevel::Debug, &format!("  - {}: {}", name, pkg));
                    }
                    mockforge_core::PluginConfig::Advanced { package, options } => {
                        progress_mgr.log(
                            LogLevel::Debug,
                            &format!("  - {}: {} (with options)", name, package),
                        );
                        if !options.is_empty() {
                            for (k, v) in options {
                                progress_mgr.log(LogLevel::Debug, &format!("    - {}: {}", k, v));
                            }
                        }
                    }
                }
            }
        }
    }

    if dry_run {
        progress_mgr.log(LogLevel::Success, "\u{2705} Configuration is valid (dry run)");
        return Ok(());
    }

    // Create progress bar for generation steps
    let total_steps = 5u64;
    let progress_bar = if show_progress {
        Some(progress_mgr.create_main_progress(total_steps, "Generating mocks"))
    } else {
        None
    };

    // Step 5: Create output directory
    progress_mgr.log_step(1, total_steps as usize, "Preparing output directory");
    if config.output.clean && config.output.path.exists() {
        if verbose {
            progress_mgr.log(
                LogLevel::Debug,
                &format!("\u{1f9f9} Cleaning output directory: {}", config.output.path.display()),
            );
        }
        tokio::fs::remove_dir_all(&config.output.path).await?;
    }

    tokio::fs::create_dir_all(&config.output.path).await?;
    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 6: Load and process OpenAPI spec
    progress_mgr.log_step(2, total_steps as usize, "Loading OpenAPI specification");
    let spec_content = tokio::fs::read_to_string(spec).await?;
    let spec_size = utils::format_file_size(spec_content.len() as u64);
    progress_mgr.log(
        LogLevel::Info,
        &format!("\u{1f4d6} Loaded OpenAPI specification ({})", spec_size),
    );

    // Parse OpenAPI spec for file naming context
    let parsed_spec =
        OpenApiSpec::from_string(&spec_content, spec.extension().and_then(|e| e.to_str()))
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to parse OpenAPI specification: {}", e).into()
            })?;

    // Build file naming context from OpenAPI spec (for file naming templates)
    let naming_context = if config.output.file_naming_template.is_some() {
        Some(build_file_naming_context(&parsed_spec))
    } else {
        None
    };

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 7: Generate mock server code
    progress_mgr.log_step(3, total_steps as usize, "Generating mock server code");

    // Determine output filename with extension handling
    let base_filename =
        config.output.filename.clone().unwrap_or_else(|| "generated_mock".to_string());

    // Determine extension based on config or default
    let extension = config.output.extension.clone().unwrap_or_else(|| "rs".to_string());

    // Build initial file path
    let mut output_file = config.output.path.join(format!("{}.{}", base_filename, extension));

    // Generate mock server code using the codegen module
    let codegen_config = mockforge_import::codegen::CodegenConfig {
        mock_data_strategy: mockforge_import::codegen::MockDataStrategy::ExamplesOrRandom,
        port: None, // Will use default 3000
        enable_cors: false,
        default_delay_ms: None,
    };

    let raw_mock_code = mockforge_import::codegen::generate_mock_server_code(
        &parsed_spec,
        &extension,
        &codegen_config,
    )
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("Failed to generate mock server code: {}", e).into()
    })?;

    // Create GeneratedFile for processing
    let mut generated_file = GeneratedFile {
        path: output_file
            .strip_prefix(&config.output.path)
            .unwrap_or(&output_file)
            .to_path_buf(),
        content: raw_mock_code,
        extension: extension.clone(),
        exportable: matches!(extension.as_str(), "ts" | "tsx" | "js" | "jsx" | "mjs"),
    };

    // Apply output control options (banner, extension, file naming template with context)
    generated_file =
        process_generated_file(generated_file, &config.output, Some(spec), naming_context.as_ref());

    // Update output_file path after processing
    output_file = config.output.path.join(&generated_file.path);

    // Write the processed file
    tokio::fs::write(&output_file, generated_file.content.clone()).await?;

    // Track generated files for barrel generation
    let all_generated_files = vec![generated_file];

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 8: Generate additional files if needed
    progress_mgr.log_step(4, total_steps as usize, "Generating additional files");

    // Create a basic README
    let readme_content = format!(
        r#"# Generated Mock Server

This mock server was generated by MockForge from the OpenAPI specification:
- Source: {}
- Generated: {}

## Usage

```bash
# Start the mock server
cargo run

# Or use MockForge CLI
mockforge serve --spec {}
```

## Files Generated

- `{}` - Main mock server implementation
- `README.md` - This file
"#,
        spec.display(),
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        spec.display(),
        {
            use crate::progress::get_file_name;
            get_file_name(&output_file).unwrap_or_else(|e| {
                eprintln!("{}", e.message);
                if let Some(suggestion) = e.suggestion {
                    eprintln!("\u{1f4a1} {}", suggestion);
                }
                std::process::exit(e.exit_code as i32);
            })
        }
    );

    let readme_file = config.output.path.join("README.md");
    tokio::fs::write(&readme_file, readme_content).await?;

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 9: Generate barrel files if requested
    if config.output.barrel_type != mockforge_core::BarrelType::None {
        if verbose {
            progress_mgr.log(
                LogLevel::Debug,
                &format!(
                    "\u{1f4e6} Generating barrel files (type: {:?})",
                    config.output.barrel_type
                ),
            );
        }

        match BarrelGenerator::generate_barrel_files(
            &config.output.path,
            &all_generated_files,
            config.output.barrel_type,
        ) {
            Ok(barrel_files) => {
                for (barrel_path, barrel_content) in barrel_files {
                    tokio::fs::write(&barrel_path, barrel_content).await?;
                    if verbose {
                        progress_mgr.log(
                            LogLevel::Debug,
                            &format!("\u{1f4c4} Generated barrel file: {}", barrel_path.display()),
                        );
                    }
                }
            }
            Err(e) => {
                progress_mgr.log(
                    LogLevel::Warning,
                    &format!("\u{26a0}\u{fe0f}  Failed to generate barrel files: {}", e),
                );
            }
        }
    }

    // Step 10: Finalize
    progress_mgr.log_step(5, total_steps as usize, "Finalizing generation");

    let duration = start_time.elapsed();
    let duration_str = utils::format_duration(duration);

    // Count total files (generated + barrel files + README)
    let total_files = all_generated_files.len() + 1; // +1 for README

    progress_mgr.log(
        LogLevel::Success,
        &format!("\u{2705} Mock generation completed in {}", duration_str),
    );
    progress_mgr.log(
        LogLevel::Info,
        &format!("\u{1f4c1} Output directory: {}", config.output.path.display()),
    );
    progress_mgr.log(LogLevel::Info, &format!("\u{1f4c4} Generated files: {} files", total_files));

    if let Some(ref pb) = progress_bar {
        pb.finish();
    }

    Ok(())
}

/// Handle project initialization
pub(crate) async fn handle_init(
    name: String,
    no_examples: bool,
    blueprint: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;

    // If blueprint is provided, use blueprint creation instead
    if let Some(blueprint_id) = blueprint {
        println!("\u{1f680} Creating project from blueprint '{}'...", blueprint_id);

        // Determine project directory
        let project_dir = if name == "." {
            std::env::current_dir()?
        } else {
            PathBuf::from(&name)
        };

        // Use blueprint creation logic
        use crate::blueprint_commands;
        blueprint_commands::create_from_blueprint(
            if name == "." {
                project_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("my-project")
                    .to_string()
            } else {
                name
            },
            blueprint_id,
            Some(project_dir),
            false, // Don't force overwrite by default
        )?;

        return Ok(());
    }

    println!("\u{1f680} Initializing MockForge project...");

    // Determine project directory
    let project_dir = if name == "." {
        std::env::current_dir()?
    } else {
        PathBuf::from(&name)
    };

    // Create project directory if it doesn't exist
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir)?;
        println!("\u{1f4c1} Created directory: {}", project_dir.display());
    }

    // Create config file
    let config_path = project_dir.join("mockforge.yaml");
    if config_path.exists() {
        println!("\u{26a0}\u{fe0f}  Configuration file already exists: {}", config_path.display());
    } else {
        // Conditionally include openapi_spec line only if examples are being created
        let openapi_spec_line = if !no_examples {
            "  openapi_spec: \"./examples/openapi.json\"\n"
        } else {
            ""
        };

        let config_content = format!(
            r#"# MockForge Configuration
# Full configuration reference: https://docs.mockforge.dev/config

# HTTP Server
http:
  port: 3000
  host: "0.0.0.0"
  cors_enabled: true
  request_timeout_secs: 30
  request_validation: "enforce"
  aggregate_validation_errors: true
  validate_responses: false
  response_template_expand: false
  validation_overrides: {{}}
  skip_admin_validation: true
{}
# WebSocket Server
websocket:
  port: 3001
  host: "0.0.0.0"
  connection_timeout_secs: 300

# gRPC Server
grpc:
  port: 50051
  host: "0.0.0.0"

# Admin UI
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"
  api_enabled: true
  auth_required: false
  prometheus_url: "http://localhost:9090"

# Core Features
core:
  latency_enabled: true
  failures_enabled: false
  overrides_enabled: true
  traffic_shaping_enabled: false

# Observability
observability:
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"
  opentelemetry: null
  recorder: null
  chaos: null

# Data Generation
data:
  default_rows: 100
  default_format: "json"
  locale: "en"
  templates: {{}}
  rag:
    enabled: false
    provider: "openai"

# Logging
logging:
  level: "info"
  json_format: false
  max_file_size_mb: 10
  max_files: 5
"#,
            openapi_spec_line
        );
        fs::write(&config_path, config_content)?;
        println!("\u{2705} Created mockforge.yaml");
    }

    // Create examples directory if not skipped
    if !no_examples {
        let examples_dir = project_dir.join("examples");
        fs::create_dir_all(&examples_dir)?;
        println!("\u{1f4c1} Created examples directory");

        // Create example OpenAPI spec
        let openapi_path = examples_dir.join("openapi.json");
        let openapi_content = r#"{
  "openapi": "3.0.0",
  "info": {
    "title": "Example API",
    "version": "1.0.0"
  },
  "paths": {
    "/health": {
      "get": {
        "summary": "Health check",
        "responses": {
          "200": {
            "description": "OK",
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "status": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    "/users": {
      "get": {
        "summary": "List users",
        "responses": {
          "200": {
            "description": "OK",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "type": "object",
                    "properties": {
                      "id": {
                        "type": "integer"
                      },
                      "name": {
                        "type": "string"
                      },
                      "email": {
                        "type": "string"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}"#;
        fs::write(&openapi_path, openapi_content)?;
        println!("\u{2705} Created examples/openapi.json");

        // Create example fixture
        let fixtures_dir = project_dir.join("fixtures");
        fs::create_dir_all(&fixtures_dir)?;
        let fixture_path = fixtures_dir.join("users.json");
        let fixture_content = r#"[
  {
    "id": 1,
    "name": "Alice Johnson",
    "email": "alice@example.com"
  },
  {
    "id": 2,
    "name": "Bob Smith",
    "email": "bob@example.com"
  }
]"#;
        fs::write(&fixture_path, fixture_content)?;
        println!("\u{2705} Created fixtures/users.json");
    }

    println!("\n\u{1f389} MockForge project initialized successfully!");
    println!("\nNext steps:");
    println!("  1. cd {}", if name == "." { "." } else { &name });
    println!("  2. Edit mockforge.yaml to configure your mock servers");
    if !no_examples {
        println!("  3. Review examples/openapi.json for API specifications");
    }
    println!("  4. Run: mockforge serve --config mockforge.yaml");
    println!("\nNote: CLI arguments override config file settings");

    Ok(())
}
