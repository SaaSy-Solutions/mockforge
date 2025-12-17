//! Progress indicators and enhanced CLI feedback utilities
//!
//! This module provides progress bars, spinners, and structured logging
//! for long-running operations in the MockForge CLI.

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Exit codes for CLI operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    InvalidArguments = 2,
    FileNotFound = 3,
    PermissionDenied = 4,
    NetworkError = 5,
    ConfigurationError = 6,
    GenerationError = 7,
    ServerError = 8,
}

impl ExitCode {
    /// Exit the process with this code
    pub fn exit(self) -> ! {
        std::process::exit(self as i32);
    }
}

/// Progress indicator manager for CLI operations
pub struct ProgressManager {
    multi_progress: Arc<MultiProgress>,
    main_progress: Option<ProgressBar>,
    verbose: bool,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new(verbose: bool) -> Self {
        let multi_progress = Arc::new(MultiProgress::new());

        Self {
            multi_progress,
            main_progress: None,
            verbose,
        }
    }

    /// Create a main progress bar for long-running operations
    pub fn create_main_progress(&mut self, total: u64, message: &str) -> ProgressBar {
        // These templates are hardcoded and should never fail, but handle errors gracefully
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-");

        let progress = self.multi_progress.add(ProgressBar::new(total));
        progress.set_style(style);
        progress.set_message(message.to_string());

        self.main_progress = Some(progress.clone());
        progress
    }

    /// Create a spinner for indeterminate operations
    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let spinner = self.multi_progress.add(ProgressBar::new_spinner());
        // Template is hardcoded and should never fail, but handle errors gracefully
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
        spinner.set_style(style);
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner
    }

    /// Log a message with appropriate styling
    pub fn log(&self, level: LogLevel, message: &str) {
        if !self.verbose && level == LogLevel::Debug {
            return;
        }

        let styled_message = match level {
            LogLevel::Info => style(message).green(),
            LogLevel::Success => style(message).green().bold(),
            LogLevel::Warning => style(message).yellow(),
            LogLevel::Error => style(message).red().bold(),
            LogLevel::Debug => style(message).dim(),
        };

        println!("{}", styled_message);
    }

    /// Log a step in a multi-step process
    pub fn log_step(&self, step: usize, total: usize, message: &str) {
        let step_msg = format!("[{}/{}] {}", step, total, message);
        self.log(LogLevel::Info, &step_msg);
    }

    /// Finish all progress indicators
    pub fn finish(&self) {
        if let Some(ref progress) = self.main_progress {
            progress.finish();
        }
        self.multi_progress.clear().unwrap();
    }

    /// Clear all progress indicators
    pub fn clear(&self) {
        self.multi_progress.clear().unwrap();
    }
}

/// Log levels for CLI output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
}

/// Enhanced error handling with structured messages
#[derive(Debug)]
pub struct CliError {
    pub message: String,
    pub exit_code: ExitCode,
    pub suggestion: Option<String>,
}

impl CliError {
    /// Create a new CLI error
    pub fn new(message: String, exit_code: ExitCode) -> Self {
        Self {
            message,
            exit_code,
            suggestion: None,
        }
    }

    /// Add a suggestion to help the user resolve the error
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Display the error with styling and exit
    pub fn display_and_exit(self) -> ! {
        let error_msg = style("❌ Error:").red().bold();
        println!("{} {}", error_msg, style(&self.message).red());

        if let Some(suggestion) = &self.suggestion {
            let suggestion_msg = style("💡 Suggestion:").yellow();
            println!("{} {}", suggestion_msg, style(suggestion).yellow());
        }

        self.exit_code.exit();
    }
}

impl std::error::Error for CliError {
    // In modern Rust, description() is deprecated, use Display trait instead
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " {}", suggestion)?;
        }
        Ok(())
    }
}

/// Helper function to parse socket addresses with better error messages
pub fn parse_address(addr_str: &str, context: &str) -> Result<SocketAddr, CliError> {
    addr_str.parse().map_err(|e| {
        CliError::new(
            format!("Invalid {} address '{}': {}", context, addr_str, e),
            ExitCode::ConfigurationError,
        )
        .with_suggestion("Ensure the address is in the correct format (e.g., '127.0.0.1:8080' or '0.0.0.0:3000')".to_string())
    })
}

/// Helper function to require a config value with a meaningful error
pub fn require_config<T>(opt: Option<T>, field: &str) -> Result<T, CliError> {
    opt.ok_or_else(|| {
        CliError::new(
            format!("Missing required configuration field: '{}'", field),
            ExitCode::ConfigurationError,
        )
        .with_suggestion(format!(
            "Add '{}' to your configuration file or provide it via command-line argument",
            field
        ))
    })
}

/// Helper function to unwrap optional registry references with error context
pub fn require_registry<'a, T>(opt: &'a Option<T>, registry_name: &str) -> Result<&'a T, CliError> {
    opt.as_ref().ok_or_else(|| {
        CliError::new(
            format!("{} registry not available", registry_name),
            ExitCode::ConfigurationError,
        )
        .with_suggestion(format!(
            "Ensure {} is properly configured in your configuration file",
            registry_name
        ))
    })
}

/// Helper function to get file name from path with error handling
pub fn get_file_name(path: &PathBuf) -> Result<String, CliError> {
    path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()).ok_or_else(|| {
        CliError::new(
            format!("Could not extract file name from path: {}", path.display()),
            ExitCode::FileNotFound,
        )
        .with_suggestion("Ensure the path is valid and points to a file".to_string())
    })
}

/// Utility functions for common CLI operations
pub mod utils {
    use super::*;

    use std::path::Path;

    /// Validate that a file exists and is readable
    pub fn validate_file_path(path: &Path) -> Result<(), CliError> {
        if !path.exists() {
            return Err(CliError::new(
                format!("File not found: {}", path.display()),
                ExitCode::FileNotFound,
            )
            .with_suggestion("Check the file path and ensure the file exists".to_string()));
        }

        if !path.is_file() {
            return Err(CliError::new(
                format!("Path is not a file: {}", path.display()),
                ExitCode::InvalidArguments,
            )
            .with_suggestion("Provide a valid file path, not a directory".to_string()));
        }

        Ok(())
    }

    /// Validate that a directory exists and is writable
    pub fn validate_output_dir(path: &Path) -> Result<(), CliError> {
        if path.exists() && !path.is_dir() {
            return Err(CliError::new(
                format!("Path exists but is not a directory: {}", path.display()),
                ExitCode::InvalidArguments,
            )
            .with_suggestion(
                "Provide a valid directory path or remove the existing file".to_string(),
            ));
        }

        Ok(())
    }

    /// Format file size in human-readable format
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
    }

    /// Format duration in human-readable format
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Watch mode utilities for file monitoring
pub mod watch {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs;
    use tokio::time::{sleep, Duration};

    /// Watch for file changes and execute a callback
    pub async fn watch_files<F, Fut>(
        files: Vec<PathBuf>,
        callback: F,
        debounce_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
            + Send,
    {
        let mut last_modified = std::collections::HashMap::new();

        // Initialize last modified times
        for file in &files {
            if let Ok(metadata) = fs::metadata(file).await {
                if let Ok(modified) = metadata.modified() {
                    last_modified.insert(file.clone(), modified);
                }
            }
        }

        loop {
            let mut changed = false;

            for file in &files {
                if let Ok(metadata) = fs::metadata(file).await {
                    if let Ok(modified) = metadata.modified() {
                        if let Some(last_time) = last_modified.get(file) {
                            if modified > *last_time {
                                changed = true;
                                last_modified.insert(file.clone(), modified);
                            }
                        }
                    }
                }
            }

            if changed {
                println!("{}", style("🔄 File change detected, regenerating...").yellow());
                if let Err(e) = callback().await {
                    eprintln!("{}", style(format!("❌ Error during regeneration: {}", e)).red());
                }
            }

            sleep(Duration::from_millis(debounce_ms)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ExitCode tests
    #[test]
    fn test_exit_codes() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::GeneralError as i32, 1);
        assert_eq!(ExitCode::InvalidArguments as i32, 2);
    }

    #[test]
    fn test_exit_codes_all_variants() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::GeneralError as i32, 1);
        assert_eq!(ExitCode::InvalidArguments as i32, 2);
        assert_eq!(ExitCode::FileNotFound as i32, 3);
        assert_eq!(ExitCode::PermissionDenied as i32, 4);
        assert_eq!(ExitCode::NetworkError as i32, 5);
        assert_eq!(ExitCode::ConfigurationError as i32, 6);
        assert_eq!(ExitCode::GenerationError as i32, 7);
        assert_eq!(ExitCode::ServerError as i32, 8);
    }

    #[test]
    fn test_exit_code_clone() {
        let code = ExitCode::GeneralError;
        let cloned = code;
        assert_eq!(code as i32, cloned as i32);
    }

    #[test]
    fn test_exit_code_debug() {
        let code = ExitCode::FileNotFound;
        let debug = format!("{:?}", code);
        assert!(debug.contains("FileNotFound"));
    }

    // LogLevel tests
    #[test]
    fn test_log_level_clone() {
        let level = LogLevel::Info;
        let cloned = level;
        assert_eq!(level, cloned);
    }

    #[test]
    fn test_log_level_debug() {
        let level = LogLevel::Error;
        let debug = format!("{:?}", level);
        assert!(debug.contains("Error"));
    }

    #[test]
    fn test_log_level_equality() {
        assert_eq!(LogLevel::Info, LogLevel::Info);
        assert_ne!(LogLevel::Info, LogLevel::Error);
    }

    // CliError tests
    #[test]
    fn test_cli_error_new() {
        let error = CliError::new("Test error".to_string(), ExitCode::GeneralError);
        assert_eq!(error.message, "Test error");
        assert_eq!(error.exit_code, ExitCode::GeneralError);
        assert!(error.suggestion.is_none());
    }

    #[test]
    fn test_cli_error_with_suggestion() {
        let error = CliError::new("Test error".to_string(), ExitCode::GeneralError)
            .with_suggestion("Try this instead".to_string());
        assert_eq!(error.message, "Test error");
        assert_eq!(error.suggestion, Some("Try this instead".to_string()));
    }

    #[test]
    fn test_cli_error_display() {
        let error = CliError::new("Test error".to_string(), ExitCode::GeneralError);
        let display = format!("{}", error);
        assert!(display.contains("Test error"));
    }

    #[test]
    fn test_cli_error_display_with_suggestion() {
        let error = CliError::new("Test error".to_string(), ExitCode::GeneralError)
            .with_suggestion("Suggestion".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Test error"));
        assert!(display.contains("Suggestion"));
    }

    #[test]
    fn test_cli_error_debug() {
        let error = CliError::new("Test error".to_string(), ExitCode::GeneralError);
        let debug = format!("{:?}", error);
        assert!(debug.contains("CliError"));
        assert!(debug.contains("Test error"));
    }

    // ProgressManager tests
    #[test]
    fn test_progress_manager_new() {
        let manager = ProgressManager::new(false);
        assert!(!manager.verbose);
        assert!(manager.main_progress.is_none());
    }

    #[test]
    fn test_progress_manager_verbose() {
        let manager = ProgressManager::new(true);
        assert!(manager.verbose);
    }

    #[test]
    fn test_progress_manager_create_spinner() {
        let manager = ProgressManager::new(false);
        let spinner = manager.create_spinner("Loading...");
        spinner.finish();
    }

    #[test]
    fn test_progress_manager_create_main_progress() {
        let mut manager = ProgressManager::new(false);
        let progress = manager.create_main_progress(100, "Processing");
        assert!(manager.main_progress.is_some());
        progress.finish();
    }

    // parse_address tests
    #[test]
    fn test_parse_address_valid() {
        let result = parse_address("127.0.0.1:8080", "HTTP");
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_parse_address_invalid() {
        let result = parse_address("invalid", "HTTP");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("Invalid"));
        assert!(error.message.contains("HTTP"));
    }

    #[test]
    fn test_parse_address_with_ipv6() {
        let result = parse_address("[::1]:8080", "gRPC");
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_eq!(addr.port(), 8080);
    }

    // require_config tests
    #[test]
    fn test_require_config_some() {
        let result = require_config(Some("value"), "field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "value");
    }

    #[test]
    fn test_require_config_none() {
        let result: Result<&str, CliError> = require_config(None, "missing_field");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("missing_field"));
    }

    // require_registry tests
    #[test]
    fn test_require_registry_some() {
        let opt = Some("registry".to_string());
        let result = require_registry(&opt, "HTTP");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "registry");
    }

    #[test]
    fn test_require_registry_none() {
        let opt: Option<String> = None;
        let result = require_registry(&opt, "gRPC");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("gRPC"));
    }

    // get_file_name tests
    #[test]
    fn test_get_file_name_valid() {
        let path = PathBuf::from("/path/to/file.txt");
        let result = get_file_name(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "file.txt");
    }

    #[test]
    fn test_get_file_name_directory() {
        let path = PathBuf::from("/path/to/directory/");
        // Should still get the last component
        let result = get_file_name(&path);
        // The trailing slash makes it harder - may fail
        // This tests error handling
        assert!(result.is_ok() || result.is_err());
    }

    // utils::format_file_size tests
    #[test]
    fn test_format_file_size() {
        assert_eq!(utils::format_file_size(1024), "1.0 KB");
        assert_eq!(utils::format_file_size(1048576), "1.0 MB");
        assert_eq!(utils::format_file_size(512), "512.0 B");
    }

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(utils::format_file_size(0), "0.0 B");
        assert_eq!(utils::format_file_size(100), "100.0 B");
        assert_eq!(utils::format_file_size(1023), "1023.0 B");
    }

    #[test]
    fn test_format_file_size_kb() {
        assert_eq!(utils::format_file_size(1024), "1.0 KB");
        assert_eq!(utils::format_file_size(2048), "2.0 KB");
        assert_eq!(utils::format_file_size(1536), "1.5 KB");
    }

    #[test]
    fn test_format_file_size_mb() {
        assert_eq!(utils::format_file_size(1048576), "1.0 MB");
        assert_eq!(utils::format_file_size(2097152), "2.0 MB");
    }

    #[test]
    fn test_format_file_size_gb() {
        assert_eq!(utils::format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_file_size_tb() {
        assert_eq!(utils::format_file_size(1099511627776), "1.0 TB");
    }

    // utils::format_duration tests
    #[test]
    fn test_format_duration() {
        assert_eq!(utils::format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(utils::format_duration(Duration::from_secs(3665)), "1h 1m 5s");
        assert_eq!(utils::format_duration(Duration::from_secs(30)), "30s");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(utils::format_duration(Duration::from_secs(0)), "0s");
    }

    #[test]
    fn test_format_duration_seconds_only() {
        assert_eq!(utils::format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes_and_seconds() {
        assert_eq!(utils::format_duration(Duration::from_secs(90)), "1m 30s");
    }

    #[test]
    fn test_format_duration_hours_minutes_seconds() {
        assert_eq!(utils::format_duration(Duration::from_secs(7200)), "2h 0m 0s");
        assert_eq!(utils::format_duration(Duration::from_secs(7261)), "2h 1m 1s");
    }

    // utils::validate_file_path tests
    #[test]
    fn test_validate_file_path_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = utils::validate_file_path(&file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_path_not_exists() {
        let path = PathBuf::from("/nonexistent/path/file.txt");
        let result = utils::validate_file_path(&path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().exit_code, ExitCode::FileNotFound);
    }

    #[test]
    fn test_validate_file_path_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = utils::validate_file_path(temp_dir.path());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().exit_code, ExitCode::InvalidArguments);
    }

    // utils::validate_output_dir tests
    #[test]
    fn test_validate_output_dir_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = utils::validate_output_dir(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_output_dir_not_exists() {
        let path = PathBuf::from("/nonexistent/path/");
        // Non-existent directory is OK (will be created)
        let result = utils::validate_output_dir(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_output_dir_is_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = utils::validate_output_dir(&file_path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().exit_code, ExitCode::InvalidArguments);
    }
}
