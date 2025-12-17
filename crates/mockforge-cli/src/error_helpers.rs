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

#[cfg(test)]
mod tests {
    use super::*;

    // ErrorBuilder tests
    #[test]
    fn test_error_builder_new() {
        let builder = ErrorBuilder::new("Test message", ExitCode::GeneralError);
        let error = builder.build();
        assert!(error.message.contains("Test message"));
        assert_eq!(error.exit_code, ExitCode::GeneralError);
    }

    #[test]
    fn test_error_builder_with_suggestion() {
        let error = ErrorBuilder::new("Error", ExitCode::GeneralError)
            .with_suggestion("Try this")
            .build();
        assert!(error.suggestion.is_some());
        assert!(error.suggestion.unwrap().contains("Try this"));
    }

    #[test]
    fn test_error_builder_multiple_suggestions() {
        let error = ErrorBuilder::new("Error", ExitCode::GeneralError)
            .with_suggestion("Suggestion 1")
            .with_suggestion("Suggestion 2")
            .build();
        let suggestion = error.suggestion.unwrap();
        assert!(suggestion.contains("Suggestion 1"));
        assert!(suggestion.contains("Suggestion 2"));
    }

    #[test]
    fn test_error_builder_with_context() {
        let error = ErrorBuilder::new("Error", ExitCode::GeneralError)
            .with_context("Context info")
            .build();
        assert!(error.message.contains("Context"));
        assert!(error.message.contains("Context info"));
    }

    #[test]
    fn test_error_builder_multiple_contexts() {
        let error = ErrorBuilder::new("Error", ExitCode::GeneralError)
            .with_context("Context 1")
            .with_context("Context 2")
            .build();
        assert!(error.message.contains("Context 1"));
        assert!(error.message.contains("Context 2"));
    }

    #[test]
    fn test_error_builder_with_help_url() {
        let error = ErrorBuilder::new("Error", ExitCode::GeneralError)
            .with_help_url("https://docs.example.com")
            .build();
        let suggestion = error.suggestion.unwrap();
        assert!(suggestion.contains("https://docs.example.com"));
    }

    #[test]
    fn test_error_builder_chaining() {
        let error = ErrorBuilder::new("Main error", ExitCode::ConfigurationError)
            .with_suggestion("Suggestion")
            .with_context("File: test.yaml")
            .with_help_url("https://docs.example.com")
            .build();

        assert!(error.message.contains("Main error"));
        assert!(error.message.contains("File: test.yaml"));
        let suggestion = error.suggestion.unwrap();
        assert!(suggestion.contains("Suggestion"));
        assert!(suggestion.contains("https://docs.example.com"));
    }

    // Common error pattern tests
    #[test]
    fn test_port_in_use_error() {
        let error = port_in_use_error(8080, "HTTP");
        assert!(error.message.contains("8080"));
        assert!(error.message.contains("HTTP"));
        assert_eq!(error.exit_code, ExitCode::ServerError);
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_port_in_use_error_different_port() {
        let error = port_in_use_error(3000, "gRPC");
        assert!(error.message.contains("3000"));
        assert!(error.message.contains("gRPC"));
    }

    #[test]
    fn test_config_not_found_error() {
        let path = PathBuf::from("/path/to/config.yaml");
        let error = config_not_found_error(&path);
        assert!(error.message.contains("config.yaml"));
        assert_eq!(error.exit_code, ExitCode::FileNotFound);
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_invalid_openapi_error() {
        let path = PathBuf::from("/path/to/spec.yaml");
        let error = invalid_openapi_error(&path, "Missing paths object");
        assert!(error.message.contains("Invalid OpenAPI"));
        assert!(error.message.contains("Missing paths object"));
        assert!(error.message.contains("spec.yaml"));
        assert_eq!(error.exit_code, ExitCode::ConfigurationError);
    }

    #[test]
    fn test_missing_field_error() {
        let error = missing_field_error("base_url", "HTTP configuration");
        assert!(error.message.contains("base_url"));
        assert!(error.message.contains("HTTP configuration"));
        assert_eq!(error.exit_code, ExitCode::ConfigurationError);
    }

    #[test]
    fn test_network_error() {
        let error = network_error("API fetch", "Connection refused");
        assert!(error.message.contains("API fetch"));
        assert!(error.message.contains("Connection refused"));
        assert_eq!(error.exit_code, ExitCode::NetworkError);
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_permission_denied_error() {
        let path = PathBuf::from("/etc/mockforge/config.yaml");
        let error = permission_denied_error(&path, "write to");
        assert!(error.message.contains("Permission denied"));
        assert!(error.message.contains("write to"));
        assert!(error.message.contains("config.yaml"));
        assert_eq!(error.exit_code, ExitCode::PermissionDenied);
    }

    // format_validation_errors tests
    #[test]
    fn test_format_validation_errors_single() {
        let errors = vec!["Invalid port number".to_string()];
        let formatted = format_validation_errors(&errors);
        assert!(formatted.contains("Configuration validation failed"));
        assert!(formatted.contains("1. Invalid port number"));
        assert!(formatted.contains("mockforge config validate"));
    }

    #[test]
    fn test_format_validation_errors_multiple() {
        let errors = vec![
            "Invalid port".to_string(),
            "Missing spec file".to_string(),
            "Invalid timeout".to_string(),
        ];
        let formatted = format_validation_errors(&errors);
        assert!(formatted.contains("1. Invalid port"));
        assert!(formatted.contains("2. Missing spec file"));
        assert!(formatted.contains("3. Invalid timeout"));
    }

    #[test]
    fn test_format_validation_errors_empty() {
        let errors: Vec<String> = vec![];
        let formatted = format_validation_errors(&errors);
        assert!(formatted.contains("Configuration validation failed"));
        // Should not crash with empty list
    }
}
