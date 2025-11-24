//! Enhanced error handling with contextual suggestions and better UX
//!
//! This module provides utilities for creating better error messages
//! with actionable suggestions and context-aware help.

use crate::progress::{CliError, ExitCode};
use colored::Colorize;
use std::path::PathBuf;

/// Enhanced error builder with context
pub struct ErrorBuilder {
    message: String,
    exit_code: ExitCode,
    suggestions: Vec<String>,
    context: Vec<String>,
    help_url: Option<String>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new(message: impl Into<String>, exit_code: ExitCode) -> Self {
        Self {
            message: message.into(),
            exit_code,
            suggestions: Vec::new(),
            context: Vec::new(),
            help_url: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Add context information
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context.push(context.into());
        self
    }

    /// Add a help URL
    pub fn with_help_url(mut self, url: impl Into<String>) -> Self {
        self.help_url = Some(url.into());
        self
    }

    /// Build the CLI error
    pub fn build(self) -> CliError {
        let mut full_message = self.message.clone();

        if !self.context.is_empty() {
            full_message.push_str("\n\n");
            full_message.push_str("Context:");
            for ctx in &self.context {
                full_message.push_str(&format!("\n  • {}", ctx));
            }
        }

        let suggestion = if !self.suggestions.is_empty() || self.help_url.is_some() {
            let mut sugg = self.suggestions.join("\n");
            if let Some(url) = &self.help_url {
                if !sugg.is_empty() {
                    sugg.push('\n');
                }
                sugg.push_str(&format!("📚 More help: {}", url));
            }
            Some(sugg)
        } else {
            None
        };

        let mut error = CliError::new(full_message, self.exit_code);
        if let Some(sugg) = suggestion {
            error = error.with_suggestion(sugg);
        }
        error
    }
}

/// Common error patterns with enhanced messages

/// Port already in use error
pub fn port_in_use_error(port: u16, protocol: &str) -> CliError {
    ErrorBuilder::new(
        format!("Port {} is already in use for {}", port, protocol),
        ExitCode::ServerError,
    )
    .with_suggestion(format!(
        "Try using a different port: mockforge serve --{}-port {}",
        protocol.to_lowercase(),
        port + 1
    ))
    .with_suggestion(format!(
        "Or find what's using the port: lsof -i :{} (macOS/Linux) or netstat -ano | findstr :{} (Windows)",
        port, port
    ))
    .with_help_url("https://docs.mockforge.dev/reference/troubleshooting.html#port-already-in-use")
    .build()
}

/// Configuration file not found error
pub fn config_not_found_error(path: &PathBuf) -> CliError {
    ErrorBuilder::new(
        format!("Configuration file not found: {}", path.display()),
        ExitCode::FileNotFound,
    )
    .with_suggestion("Create a configuration file: mockforge init .".to_string())
    .with_suggestion("Or use the wizard: mockforge wizard".to_string())
    .with_suggestion("Or specify a different config: mockforge serve --config <path>".to_string())
    .with_help_url("https://docs.mockforge.dev/getting-started/five-minute-api.html")
    .build()
}

/// Invalid OpenAPI spec error
pub fn invalid_openapi_error(path: &PathBuf, error: &str) -> CliError {
    ErrorBuilder::new(
        format!("Invalid OpenAPI specification: {}", error),
        ExitCode::ConfigurationError,
    )
    .with_context(format!("File: {}", path.display()))
    .with_suggestion("Validate your OpenAPI spec: https://editor.swagger.io/".to_string())
    .with_suggestion("Check the OpenAPI version (3.0.x or 3.1.x required)".to_string())
    .with_help_url("https://docs.mockforge.dev/reference/openapi.html")
    .build()
}

/// Missing required field error
pub fn missing_field_error(field: &str, context: &str) -> CliError {
    ErrorBuilder::new(format!("Missing required field: '{}'", field), ExitCode::ConfigurationError)
        .with_context(context.to_string())
        .with_suggestion(format!("Add '{}' to your configuration", field))
        .with_suggestion("Or provide it via command-line argument".to_string())
        .with_help_url("https://docs.mockforge.dev/config.html")
        .build()
}

/// Network error with retry suggestion
pub fn network_error(operation: &str, error: &str) -> CliError {
    ErrorBuilder::new(
        format!("Network error during {}: {}", operation, error),
        ExitCode::NetworkError,
    )
    .with_suggestion("Check your internet connection".to_string())
    .with_suggestion("Verify the URL/endpoint is correct".to_string())
    .with_suggestion("Check firewall/proxy settings".to_string())
    .with_help_url("https://docs.mockforge.dev/reference/troubleshooting.html#network-errors")
    .build()
}

/// Permission denied error
pub fn permission_denied_error(path: &PathBuf, operation: &str) -> CliError {
    ErrorBuilder::new(
        format!("Permission denied: cannot {} {}", operation, path.display()),
        ExitCode::PermissionDenied,
    )
    .with_suggestion(format!("Check file permissions: chmod +w {}", path.display()))
    .with_suggestion("Or run with appropriate permissions".to_string())
    .with_help_url("https://docs.mockforge.dev/reference/troubleshooting.html#permission-errors")
    .build()
}

/// Display error with enhanced formatting
pub fn display_error(error: CliError) -> ! {
    println!("\n{}", "❌ Error".bright_red().bold());
    println!("{}", "=".repeat(50).bright_red());
    println!("{}", error.message.bright_white());

    if let Some(suggestion) = &error.suggestion {
        println!("\n{}", "💡 Suggestions".bright_yellow().bold());
        for line in suggestion.lines() {
            if line.starts_with("📚") {
                println!("{}", line.bright_cyan());
            } else {
                println!("  • {}", line.bright_white());
            }
        }
    }

    println!();
    error.exit_code.exit();
}

/// Format validation errors with context
pub fn format_validation_errors(errors: &[String]) -> String {
    let mut message = String::from("Configuration validation failed:\n\n");
    for (i, error) in errors.iter().enumerate() {
        message.push_str(&format!("  {}. {}\n", i + 1, error));
    }
    message.push_str("\n💡 Tip: Run 'mockforge config validate' to check your configuration");
    message
}
