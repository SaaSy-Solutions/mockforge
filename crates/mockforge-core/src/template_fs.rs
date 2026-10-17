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

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;

/// Env var naming the explicit fixtures root for `fs.readFile`.
pub const ROOT_ENV: &str = "MOCKFORGE_TEMPLATE_FS_ROOT";
/// Env var overriding the per-file size cap in bytes.
pub const MAX_BYTES_ENV: &str = "MOCKFORGE_TEMPLATE_FS_MAX_BYTES";
/// Env var set on every hosted-mock machine by the registry orchestrator.
pub const HOSTED_ENV: &str = "MOCKFORGE_DEPLOYMENT_ID";
/// Default per-file size cap (1 MiB).
pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;

/// Directories that are never readable, even when a root contains them.
const DENIED_PREFIXES: &[&str] = &["/proc", "/sys", "/dev", "/etc"];

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
fn is_denied(path: &Path, root: &Path) -> bool {
    if DENIED_PREFIXES.iter().any(|p| path.starts_with(p)) {
        return true;
    }
    let below_root = path.strip_prefix(root).unwrap_or(path);
    below_root.components().any(|c| match c {
        Component::Normal(part) => part.to_str().map(is_env_like).unwrap_or(true),
        _ => false,
    })
}

/// Read `raw` relative to `root`, enforcing every sandbox rule.
pub fn read_file_in_root(root: &Path, raw: &str, max: u64) -> Result<String, TemplateFsError> {
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
    if raw.contains('\\') || has_drive_prefix(raw) {
        return Err(TemplateFsError::Rejected("Windows-style path"));
    }
    let requested = Path::new(raw);
    if requested.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(TemplateFsError::Rejected("parent-directory traversal"));
    }

    let root = root.canonicalize().map_err(|_| TemplateFsError::InvalidRoot)?;
    if !root.is_dir() || root.parent().is_none() {
        // A missing root or `/` itself would make the sandbox meaningless.
        return Err(TemplateFsError::InvalidRoot);
    }

    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    if is_denied(&joined, &root) {
        return Err(TemplateFsError::Denied);
    }
    let resolved = joined.canonicalize().map_err(|_| {
        if joined.starts_with(&root) {
            TemplateFsError::NotFound
        } else {
            TemplateFsError::OutsideRoot
        }
    })?;
    if !resolved.starts_with(&root) {
        return Err(TemplateFsError::OutsideRoot);
    }
    if is_denied(&resolved, &root) {
        return Err(TemplateFsError::Denied);
    }

    let file = File::open(&resolved).map_err(|_| TemplateFsError::NotFound)?;
    let meta = file.metadata().map_err(|_| TemplateFsError::NotFound)?;
    if !meta.is_file() {
        return Err(TemplateFsError::NotFound);
    }
    if meta.len() > max {
        return Err(TemplateFsError::TooLarge(max));
    }
    // Bound the read as well, in case the file grows after the metadata check.
    let mut buf = Vec::new();
    file.take(max.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|_| TemplateFsError::ReadFailed)?;
    if buf.len() as u64 > max {
        return Err(TemplateFsError::TooLarge(max));
    }
    String::from_utf8(buf).map_err(|_| TemplateFsError::ReadFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path().join("specs");
        fs::create_dir_all(root.join("fixtures")).expect("create fixtures dir");
        fs::write(root.join("fixtures/user.json"), r#"{"id":1}"#).expect("write fixture");
        fs::write(dir.path().join("secret.txt"), "top-secret").expect("write secret");
        (dir, root)
    }

    #[test]
    fn legitimate_relative_file_is_read() {
        let (_dir, root) = fixture();
        let out = read_file_in_root(&root, "fixtures/user.json", DEFAULT_MAX_BYTES);
        assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
        let out = read_file_in_root(&root, "./fixtures/user.json", DEFAULT_MAX_BYTES);
        assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
    }

    #[test]
    fn absolute_path_inside_root_is_read() {
        let (_dir, root) = fixture();
        let abs = root.canonicalize().expect("canon").join("fixtures/user.json");
        let out = read_file_in_root(&root, &abs.to_string_lossy(), DEFAULT_MAX_BYTES);
        assert_eq!(out, Ok(r#"{"id":1}"#.to_string()));
    }

    #[test]
    fn dot_dot_traversal_is_rejected() {
        let (_dir, root) = fixture();
        for p in [
            "../secret.txt",
            "fixtures/../../secret.txt",
            "fixtures/../user.json",
        ] {
            assert_eq!(
                read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
                Err(TemplateFsError::Rejected("parent-directory traversal")),
                "{p}"
            );
        }
    }

    #[test]
    fn absolute_path_outside_root_is_rejected() {
        let (dir, root) = fixture();
        let secret = dir.path().canonicalize().expect("canon").join("secret.txt");
        assert_eq!(
            read_file_in_root(&root, &secret.to_string_lossy(), DEFAULT_MAX_BYTES),
            Err(TemplateFsError::OutsideRoot)
        );
        assert_eq!(
            read_file_in_root(&root, "/nonexistent/file.txt", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::OutsideRoot)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escaping_root_is_rejected() {
        let (dir, root) = fixture();
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), root.join("link.txt"))
            .expect("symlink file");
        std::os::unix::fs::symlink(dir.path(), root.join("up")).expect("symlink dir");
        assert_eq!(
            read_file_in_root(&root, "link.txt", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::OutsideRoot)
        );
        assert_eq!(
            read_file_in_root(&root, "up/secret.txt", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::OutsideRoot)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_inside_root_is_allowed() {
        let (_dir, root) = fixture();
        std::os::unix::fs::symlink(root.join("fixtures/user.json"), root.join("alias.json"))
            .expect("symlink");
        assert_eq!(
            read_file_in_root(&root, "alias.json", DEFAULT_MAX_BYTES),
            Ok(r#"{"id":1}"#.to_string())
        );
    }

    #[test]
    fn system_paths_are_denied() {
        let (_dir, root) = fixture();
        for p in [
            "/etc/passwd",
            "/etc/shadow",
            "/proc/self/environ",
            "/proc/1/cmdline",
            "/sys/x",
        ] {
            assert_eq!(
                read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
                Err(TemplateFsError::Denied),
                "{p}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn system_paths_denied_even_when_root_contains_them() {
        // A misconfigured root of `/proc/self` still cannot leak the environment.
        let root = Path::new("/proc/self");
        assert_eq!(read_file_in_root(root, "environ", 4096), Err(TemplateFsError::Denied));
        // And `/` is never a usable root.
        assert_eq!(
            read_file_in_root(Path::new("/"), "etc/hostname", 4096),
            Err(TemplateFsError::InvalidRoot)
        );
    }

    #[test]
    fn env_like_files_are_denied() {
        let (_dir, root) = fixture();
        for name in [".env", ".env.production", "prod.env", ".envrc", "environ"] {
            fs::write(root.join(name), "SECRET=1").expect("write env file");
            assert_eq!(
                read_file_in_root(&root, name, DEFAULT_MAX_BYTES),
                Err(TemplateFsError::Denied),
                "{name}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_to_env_file_inside_root_is_denied() {
        let (_dir, root) = fixture();
        fs::write(root.join(".env"), "SECRET=1").expect("write env");
        std::os::unix::fs::symlink(root.join(".env"), root.join("innocent.json")).expect("link");
        assert_eq!(
            read_file_in_root(&root, "innocent.json", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Denied)
        );
    }

    #[test]
    fn percent_encoded_variants_are_rejected() {
        let (_dir, root) = fixture();
        for p in [
            "%2e%2e/secret.txt",
            "..%2fsecret.txt",
            "%2Fetc%2Fpasswd",
            "fixtures%2fuser.json",
        ] {
            assert_eq!(
                read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
                Err(TemplateFsError::Rejected("percent-encoding")),
                "{p}"
            );
        }
    }

    #[test]
    fn windows_style_variants_are_rejected() {
        let (_dir, root) = fixture();
        for p in [
            r"..\secret.txt",
            r"fixtures\user.json",
            r"C:\Windows\win.ini",
            "C:/Windows/win.ini",
            r"\\server\share\file",
        ] {
            assert_eq!(
                read_file_in_root(&root, p, DEFAULT_MAX_BYTES),
                Err(TemplateFsError::Rejected("Windows-style path")),
                "{p}"
            );
        }
    }

    #[test]
    fn nul_and_empty_are_rejected() {
        let (_dir, root) = fixture();
        assert_eq!(
            read_file_in_root(&root, "fixtures/user.json\0.png", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::Rejected("NUL byte"))
        );
        assert_eq!(
            read_file_in_root(&root, "  ", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::EmptyPath)
        );
    }

    #[test]
    fn oversized_file_is_rejected() {
        let (_dir, root) = fixture();
        fs::write(root.join("big.txt"), vec![b'a'; 2048]).expect("write big");
        assert_eq!(read_file_in_root(&root, "big.txt", 1024), Err(TemplateFsError::TooLarge(1024)));
        assert_eq!(read_file_in_root(&root, "big.txt", 2048).map(|s| s.len()), Ok(2048));
    }

    #[test]
    fn directories_and_missing_files_are_not_found() {
        let (_dir, root) = fixture();
        assert_eq!(
            read_file_in_root(&root, "fixtures", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::NotFound)
        );
        assert_eq!(
            read_file_in_root(&root, "missing.json", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::NotFound)
        );
    }

    #[test]
    fn missing_root_is_invalid() {
        let (dir, _root) = fixture();
        assert_eq!(
            read_file_in_root(&dir.path().join("nope"), "a.json", DEFAULT_MAX_BYTES),
            Err(TemplateFsError::InvalidRoot)
        );
    }

    #[test]
    fn root_precedence_disables_hosted_unless_explicit() {
        let explicit = Some(PathBuf::from("/srv/fixtures"));
        let registered = Some(PathBuf::from("/home/me/specs"));
        // Explicit root wins everywhere, including hosted mocks.
        assert_eq!(resolve_root(explicit.clone(), true, registered.clone()), explicit);
        assert_eq!(resolve_root(explicit.clone(), false, registered.clone()), explicit);
        // Hosted without an explicit root: disabled, even if a spec dir was registered.
        assert_eq!(resolve_root(None, true, registered.clone()), None);
        // Self-hosted: the spec directory.
        assert_eq!(resolve_root(None, false, registered.clone()), registered);
        // Nothing configured: disabled.
        assert_eq!(resolve_root(None, false, None), None);
    }
}
