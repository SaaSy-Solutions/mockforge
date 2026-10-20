//! Sandbox for the `{{fs.readFile}}` template token.
//!
//! Template tokens come from spec content, and specs are routinely imported
//! from third parties. An unrestricted `fs.readFile` therefore turns any
//! imported spec into an arbitrary local file read (#1015). This module confines
//! the token to one explicit root directory and refuses everything else.
//!
//! Root resolution (first match wins):
//! 1. `MOCKFORGE_TEMPLATE_FS_ROOT` — an explicitly configured fixtures directory.
//! 2. Hosted mocks (`MOCKFORGE_DEPLOYMENT_ID` set) — disabled. The registry must
//!    set `MOCKFORGE_TEMPLATE_FS_ROOT` to a per-tenant fixtures root to enable it.
//! 3. The root registered by the server at startup via [`set_default_root`]
//!    (the CLI registers the directory of the user-supplied spec).
//! 4. Otherwise disabled.
//!
//! Within a root, a path is refused when it contains `..`, percent-encoded or
//! Windows-style separators, drive letters or NUL bytes; when it resolves
//! (after following symlinks) outside the root; when it points into `/proc`,
//! `/sys`, `/dev` or `/etc`; when it names an env-like file (`.env`,
//! `environ`, ...); when it is not a regular file; or when it exceeds the size
//! cap (`MOCKFORGE_TEMPLATE_FS_MAX_BYTES`, default 1 MiB).

use std::path::PathBuf;
use std::sync::RwLock;

mod path_policy;
mod reader;
pub use reader::read_file_in_root;

#[cfg(test)]
mod tests;

/// Env var naming the explicit fixtures root for `fs.readFile`.
pub const ROOT_ENV: &str = "MOCKFORGE_TEMPLATE_FS_ROOT";
/// Env var overriding the per-file size cap in bytes.
pub const MAX_BYTES_ENV: &str = "MOCKFORGE_TEMPLATE_FS_MAX_BYTES";
/// Env var set on every hosted-mock machine by the registry orchestrator.
pub const HOSTED_ENV: &str = "MOCKFORGE_DEPLOYMENT_ID";
/// Default per-file size cap (1 MiB).
pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;

static DEFAULT_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Why `fs.readFile` refused a path.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateFsError {
    /// No root is configured, or the token is disabled on hosted mocks.
    #[error("disabled (no fixtures root configured; set {ROOT_ENV})")]
    Disabled,
    /// The configured root is unusable (missing, not a directory, or `/`).
    #[error("invalid fixtures root")]
    InvalidRoot,
    /// The path is empty.
    #[error("empty path")]
    EmptyPath,
    /// The path uses a form that is never accepted (traversal, encoding, ...).
    #[error("path rejected: {0}")]
    Rejected(&'static str),
    /// The path resolves outside the fixtures root.
    #[error("path is outside the fixtures root")]
    OutsideRoot,
    /// The path names a system or env-like file.
    #[error("path is not readable from templates")]
    Denied,
    /// The path does not exist or is not a regular file.
    #[error("file not found")]
    NotFound,
    /// The file exceeds the size cap.
    #[error("file exceeds {0} byte limit")]
    TooLarge(u64),
    /// The file is not valid UTF-8 or could not be read.
    #[error("read failed")]
    ReadFailed,
}

/// Register the default root (normally the spec's directory).
///
/// `None` clears it. An explicit [`ROOT_ENV`] always takes precedence.
pub fn set_default_root(root: Option<PathBuf>) {
    match DEFAULT_ROOT.write() {
        Ok(mut guard) => *guard = root,
        Err(poisoned) => *poisoned.into_inner() = root,
    }
}

fn default_root() -> Option<PathBuf> {
    match DEFAULT_ROOT.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Pick the active root from the explicit env root, hosted flag and registered
/// default. Pure so the precedence is testable without touching process env.
pub fn resolve_root(
    explicit: Option<PathBuf>,
    hosted: bool,
    registered: Option<PathBuf>,
) -> Option<PathBuf> {
    if explicit.is_some() {
        return explicit;
    }
    if hosted {
        return None;
    }
    registered
}

fn active_root() -> Option<PathBuf> {
    resolve_root(
        non_empty_env(ROOT_ENV).map(PathBuf::from),
        non_empty_env(HOSTED_ENV).is_some(),
        default_root(),
    )
}

fn max_bytes() -> u64 {
    non_empty_env(MAX_BYTES_ENV)
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MAX_BYTES)
}

/// Read a file for `{{fs.readFile}}` using the process-wide sandbox settings.
pub fn read_template_file(raw: &str) -> Result<String, TemplateFsError> {
    let root = active_root().ok_or(TemplateFsError::Disabled)?;
    read_file_in_root(&root, raw, max_bytes())
}
