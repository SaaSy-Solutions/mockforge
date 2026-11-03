//! Token syntax and secret/system path policy, independent of file opening.
use super::TemplateFsError;
use std::path::{Component, Path};

/// Directories that are never readable, even when a root contains them.
const DENIED_PREFIXES: &[&str] = &["/proc", "/sys", "/dev", "/etc"];

fn has_percent_escape(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes
        .windows(3)
        .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit())
}

fn has_drive_prefix(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

pub(super) fn has_windows_path_syntax(raw: &str, windows_host: bool) -> bool {
    raw.contains('\\') || has_drive_prefix(raw) || (windows_host && raw.contains(':'))
}

fn is_env_like(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == ".env"
        || lower.starts_with(".env.")
        || lower.ends_with(".env")
        || lower == "environ"
        || lower == ".envrc"
}

/// `path` is denied when it is under a system directory, or when any component
/// below `root` is env-like. Components of `root` itself are not inspected.
pub(super) fn is_denied(path: &Path, root: &Path) -> bool {
    if DENIED_PREFIXES.iter().any(|p| path.starts_with(p)) {
        return true;
    }
    let below_root = path.strip_prefix(root).unwrap_or(path);
    below_root.components().any(|c| match c {
        Component::Normal(part) => part.to_str().map(is_env_like).unwrap_or(true),
        _ => false,
    })
}

/// Validate token spelling before any filesystem access.
pub(super) fn validate_requested(raw: &str) -> Result<&Path, TemplateFsError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(TemplateFsError::EmptyPath);
    }
    if raw.contains('\0') {
        return Err(TemplateFsError::Rejected("NUL byte"));
    }
    if has_percent_escape(raw) {
        return Err(TemplateFsError::Rejected("percent-encoding"));
    }
    if has_windows_path_syntax(raw, cfg!(windows)) {
        return Err(TemplateFsError::Rejected("Windows-style path"));
    }
    let requested = Path::new(raw);
    if requested.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(TemplateFsError::Rejected("parent-directory traversal"));
    }

    Ok(requested)
}
