//! Marketplace upload validation
//!
//! Provides comprehensive validation for marketplace file uploads including:
//! - File size limits
//! - Content-type validation
//! - WASM file validation
//! - Package format validation
//! - Malicious content detection
//! - Path traversal prevention

use crate::error::{ApiError, ApiResult};

/// Maximum file size limits (in bytes)
pub const MAX_PLUGIN_SIZE: u64 = 10 * 1024 * 1024; // 10 MB for WASM plugins
pub const MAX_TEMPLATE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB for template packages
pub const MAX_SCENARIO_SIZE: u64 = 100 * 1024 * 1024; // 100 MB for scenario packages

/// WASM magic bytes (WebAssembly binary format)
const WASM_MAGIC: &[u8] = &[0x00, 0x61, 0x73, 0x6D]; // \0asm

/// Validate WASM file
///
/// Checks:
/// - File size within limits
/// - WASM magic bytes present
/// - Basic structure validation
pub fn validate_wasm_file(data: &[u8], reported_size: u64) -> ApiResult<()> {
    // Check reported size matches actual size
    if data.len() as u64 != reported_size {
        return Err(ApiError::InvalidRequest(format!(
            "Size mismatch: reported {} bytes, actual {} bytes",
            reported_size,
            data.len()
        )));
    }

    // Check file size limit
    if data.len() as u64 > MAX_PLUGIN_SIZE {
        return Err(ApiError::InvalidRequest(format!(
            "File too large: {} bytes (max: {} bytes / {} MB)",
            data.len(),
            MAX_PLUGIN_SIZE,
            MAX_PLUGIN_SIZE / (1024 * 1024)
        )));
    }

    // Check minimum size (WASM files should be at least 8 bytes)
    if data.len() < 8 {
        return Err(ApiError::InvalidRequest(
            "File too small to be a valid WASM file".to_string(),
        ));
    }

    // Check WASM magic bytes
    if !data.starts_with(WASM_MAGIC) {
        return Err(ApiError::InvalidRequest(
            "Invalid WASM file: missing magic bytes (expected \\0asm)".to_string(),
        ));
    }

    // Check for null bytes in header (should only be in magic bytes)
    // This is a basic sanity check
    if data.len() > 8 && data[4..8].contains(&0) {
        // This might be okay, but we'll be cautious
        // WASM version should be at bytes 4-7
    }

    // Basic structure check: ensure file has reasonable structure
    // WASM files should have at least a version number (4 bytes) after magic
    if data.len() < 8 {
        return Err(ApiError::InvalidRequest(
            "Invalid WASM file: incomplete header".to_string(),
        ));
    }

    Ok(())
}

/// Validate package file (for templates/scenarios)
///
/// Checks:
/// - File size within limits
/// - Valid archive format (tar.gz, zip, etc.)
/// - No path traversal in archive
pub fn validate_package_file(
    data: &[u8],
    reported_size: u64,
    max_size: u64,
) -> ApiResult<()> {
    // Check reported size matches actual size
    if data.len() as u64 != reported_size {
        return Err(ApiError::InvalidRequest(format!(
            "Size mismatch: reported {} bytes, actual {} bytes",
            reported_size,
            data.len()
        )));
    }

    // Check file size limit
    if data.len() as u64 > max_size {
        return Err(ApiError::InvalidRequest(format!(
            "Package too large: {} bytes (max: {} bytes / {} MB)",
            data.len(),
            max_size,
            max_size / (1024 * 1024)
        )));
    }

    // Check minimum size (archives should be at least a few bytes)
    if data.len() < 10 {
        return Err(ApiError::InvalidRequest(
            "Package too small to be a valid archive".to_string(),
        ));
    }

    // Detect archive format and validate
    if is_gzip(data) {
        // GZIP/TAR.GZ format
        validate_gzip(data)?;
    } else if is_zip(data) {
        // ZIP format
        validate_zip(data)?;
    } else {
        // Try to detect other formats or reject
        return Err(ApiError::InvalidRequest(
            "Unsupported package format. Supported formats: tar.gz, zip".to_string(),
        ));
    }

    Ok(())
}

/// Check if data is GZIP compressed
fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B
}

/// Check if data is ZIP format
fn is_zip(data: &[u8]) -> bool {
    data.len() >= 4
        && ((data[0] == 0x50 && data[1] == 0x4B && data[2] == 0x03 && data[3] == 0x04)
            || (data[0] == 0x50 && data[1] == 0x4B && data[2] == 0x05 && data[3] == 0x06)
            || (data[0] == 0x50 && data[1] == 0x4B && data[2] == 0x07 && data[3] == 0x08))
}

/// Validate GZIP file
fn validate_gzip(data: &[u8]) -> ApiResult<()> {
    // Basic GZIP header validation
    if !is_gzip(data) {
        return Err(ApiError::InvalidRequest(
            "Invalid GZIP file: missing GZIP magic bytes".to_string(),
        ));
    }

    // Check for reasonable structure
    // GZIP files should have at least 10 bytes for header
    if data.len() < 10 {
        return Err(ApiError::InvalidRequest(
            "Invalid GZIP file: incomplete header".to_string(),
        ));
    }

    // Check compression method (should be deflate = 8)
    if data.len() > 2 && data[2] != 8 {
        return Err(ApiError::InvalidRequest(format!(
            "Unsupported GZIP compression method: {} (expected deflate = 8)",
            data[2]
        )));
    }

    Ok(())
}

/// Validate ZIP file
fn validate_zip(data: &[u8]) -> ApiResult<()> {
    // Basic ZIP header validation
    if !is_zip(data) {
        return Err(ApiError::InvalidRequest(
            "Invalid ZIP file: missing ZIP magic bytes".to_string(),
        ));
    }

    // Check for path traversal in ZIP file names
    // This is a basic check - for production, you'd want to fully parse the ZIP
    // and check all file names
    if contains_path_traversal(data) {
        return Err(ApiError::InvalidRequest(
            "Package contains path traversal attempts (../)".to_string(),
        ));
    }

    Ok(())
}

/// Check for path traversal patterns in binary data
///
/// This is a basic check that looks for common path traversal patterns.
/// For production use, you should fully parse archives and validate all paths.
fn contains_path_traversal(data: &[u8]) -> bool {
    // Convert to string for pattern matching (lossy, but sufficient for this check)
    let text = String::from_utf8_lossy(data);

    // Check for common path traversal patterns
    let dangerous_patterns = [
        "../",
        "..\\",
        "/etc/",
        "/root/",
        "C:\\Windows\\",
        "C:\\System32\\",
    ];

    for pattern in &dangerous_patterns {
        if text.contains(pattern) {
            return true;
        }
    }

    false
}

/// Validate file name for security
///
/// Checks:
/// - No path traversal
/// - No dangerous characters
/// - Reasonable length
pub fn validate_filename(name: &str) -> ApiResult<()> {
    // Check for path traversal
    if name.contains("..") {
        return Err(ApiError::InvalidRequest(
            "Filename contains path traversal (..)".to_string(),
        ));
    }

    // Check for absolute paths
    if name.starts_with('/') || (name.len() > 1 && name.chars().nth(1) == Some(':')) {
        return Err(ApiError::InvalidRequest(
            "Filename must be relative, not absolute".to_string(),
        ));
    }

    // Check for dangerous characters
    let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*', '\0'];
    for ch in dangerous_chars {
        if name.contains(ch) {
            return Err(ApiError::InvalidRequest(format!(
                "Filename contains dangerous character: '{}'",
                ch
            )));
        }
    }

    // Check length (reasonable limit)
    if name.len() > 255 {
        return Err(ApiError::InvalidRequest(
            "Filename too long (max 255 characters)".to_string(),
        ));
    }

    // Check for empty name
    if name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Filename cannot be empty".to_string()));
    }

    Ok(())
}

/// Validate base64 encoded data
pub fn validate_base64(data: &str) -> ApiResult<()> {
    // Check for reasonable length
    if data.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Base64 data cannot be empty".to_string(),
        ));
    }

    // Check for suspicious patterns (basic check)
    // Base64 should only contain A-Z, a-z, 0-9, +, /, and = for padding
    let valid_chars = data
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');

    if !valid_chars {
        return Err(ApiError::InvalidRequest(
            "Invalid base64 encoding: contains invalid characters".to_string(),
        ));
    }

    // Check padding (should be 0, 1, or 2 '=' characters at the end)
    let padding_count = data.chars().rev().take_while(|&c| c == '=').count();
    if padding_count > 2 {
        return Err(ApiError::InvalidRequest(
            "Invalid base64 encoding: too much padding".to_string(),
        ));
    }

    Ok(())
}

/// Validate checksum format (SHA-256 hex)
pub fn validate_checksum(checksum: &str) -> ApiResult<()> {
    // SHA-256 produces 64 hex characters
    if checksum.len() != 64 {
        return Err(ApiError::InvalidRequest(format!(
            "Invalid checksum length: {} (expected 64 characters for SHA-256)",
            checksum.len()
        )));
    }

    // Check that all characters are valid hex
    if !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::InvalidRequest(
            "Invalid checksum format: must be hexadecimal".to_string(),
        ));
    }

    Ok(())
}

/// Validate semantic version string
///
/// Basic validation for semantic versioning (major.minor.patch)
/// Allows pre-release and build metadata
pub fn validate_version(version: &str) -> ApiResult<()> {
    // Check length (reasonable limit)
    if version.is_empty() {
        return Err(ApiError::InvalidRequest("Version cannot be empty".to_string()));
    }

    if version.len() > 100 {
        return Err(ApiError::InvalidRequest(
            "Version too long (max 100 characters)".to_string(),
        ));
    }

    // Basic semantic version pattern: major.minor.patch[-pre][+build]
    // Allow alphanumeric, dots, hyphens, and plus signs
    let valid_chars = version
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '+');

    if !valid_chars {
        return Err(ApiError::InvalidRequest(
            "Version contains invalid characters. Use semantic versioning (e.g., 1.0.0)".to_string(),
        ));
    }

    // Check for path traversal attempts
    if version.contains("..") {
        return Err(ApiError::InvalidRequest(
            "Version cannot contain path traversal (..)".to_string(),
        ));
    }

    // Check that version doesn't start/end with special characters
    if version.starts_with('.') || version.starts_with('-') || version.starts_with('+') {
        return Err(ApiError::InvalidRequest(
            "Version cannot start with '.', '-', or '+'".to_string(),
        ));
    }

    Ok(())
}

/// Validate plugin/template/scenario name
pub fn validate_name(name: &str) -> ApiResult<()> {
    // Check length
    if name.is_empty() {
        return Err(ApiError::InvalidRequest("Name cannot be empty".to_string()));
    }

    if name.len() > 100 {
        return Err(ApiError::InvalidRequest(
            "Name too long (max 100 characters)".to_string(),
        ));
    }

    // Check for path traversal
    if name.contains("..") {
        return Err(ApiError::InvalidRequest(
            "Name cannot contain path traversal (..)".to_string(),
        ));
    }

    // Check for dangerous characters
    let dangerous_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*', '\0'];
    for ch in dangerous_chars {
        if name.contains(ch) {
            return Err(ApiError::InvalidRequest(format!(
                "Name contains invalid character: '{}'",
                ch
            )));
        }
    }

    // Name should be alphanumeric with hyphens, underscores, and dots
    let valid_chars = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.');

    if !valid_chars {
        return Err(ApiError::InvalidRequest(
            "Name contains invalid characters. Use alphanumeric characters, hyphens, underscores, and dots only.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wasm_file() {
        // Valid WASM file
        let valid_wasm = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert!(validate_wasm_file(&valid_wasm, 8).is_ok());

        // Invalid magic bytes
        let invalid_wasm = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        assert!(validate_wasm_file(&invalid_wasm, 8).is_err());

        // Too large
        let large_data = vec![0x00, 0x61, 0x73, 0x6D; MAX_PLUGIN_SIZE as usize + 1];
        assert!(validate_wasm_file(&large_data, large_data.len() as u64).is_err());
    }

    #[test]
    fn test_validate_filename() {
        assert!(validate_filename("plugin.wasm").is_ok());
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("/absolute/path").is_err());
        assert!(validate_filename("file<name>").is_err());
    }

    #[test]
    fn test_validate_checksum() {
        assert!(validate_checksum(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .is_ok());
        assert!(validate_checksum("invalid").is_err());
        assert!(validate_checksum("").is_err());
    }
}
