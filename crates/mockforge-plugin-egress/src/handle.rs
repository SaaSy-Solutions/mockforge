//! Cheaply-cloneable, hot-swappable handle around a [`HostPolicy`].
//!
//! Built on `Arc<std::sync::RwLock<Arc<HostPolicy>>>`. The double
//! `Arc` lets per-connection tasks `read()` the inner `Arc<Policy>`
//! under a brief read lock and then drop the lock — checks against
//! the policy don't hold the lock across await points. A reload
//! takes the write lock just long enough to replace the inner
//! `Arc`. Readers using the old version finish naturally as their
//! `Arc<HostPolicy>` clones drop.
//!
//! Why `std::sync::RwLock` rather than `tokio::sync::RwLock`: the
//! lock is held for nanoseconds (clone an Arc, drop the guard).
//! Holding it across an `await` would be wrong; the API
//! deliberately doesn't expose that. Tokio's RwLock would force
//! every check to be `.await` for no benefit.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::policy::{HostPolicy, PolicyDecision, PolicyError};

/// Hot-swappable policy handle. Cloning is `Arc::clone` —
/// per-connection tasks each hold one.
#[derive(Clone)]
pub struct PolicyHandle {
    inner: Arc<RwLock<Arc<HostPolicy>>>,
}

impl PolicyHandle {
    /// Wrap an existing `HostPolicy`.
    pub fn new(policy: HostPolicy) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Arc::new(policy))),
        }
    }

    /// Take a cheap snapshot of the current policy. The returned
    /// `Arc<HostPolicy>` survives a concurrent `replace`; the
    /// caller's view of the policy is consistent for the rest of
    /// the request.
    pub fn snapshot(&self) -> Arc<HostPolicy> {
        // `expect` rather than `unwrap` so a poisoned lock surfaces
        // a clear message — poisoning would mean a previous holder
        // panicked while writing, which is a bug worth seeing.
        Arc::clone(
            &self
                .inner
                .read()
                .expect("PolicyHandle read lock poisoned — earlier write panicked"),
        )
    }

    /// One-shot check that combines `snapshot` and
    /// `HostPolicy::check`. Convenient for the proxy hot path.
    pub fn check(&self, host: &str) -> PolicyDecision {
        self.snapshot().check(host)
    }

    /// Atomically replace the inner policy.
    pub fn replace(&self, policy: HostPolicy) {
        let mut guard = self
            .inner
            .write()
            .expect("PolicyHandle write lock poisoned — previous holder panicked");
        *guard = Arc::new(policy);
    }
}

/// Read an allowlist file (one pattern per line, `#` comments) and
/// build a fresh [`HostPolicy`]. Used by the SIGHUP handler in
/// `main.rs`.
pub fn load_policy_from_file(path: &Path) -> Result<HostPolicy, ReloadError> {
    let contents = std::fs::read_to_string(path).map_err(|err| ReloadError::Io {
        path: path.to_path_buf(),
        err: err.to_string(),
    })?;
    let patterns: Vec<String> = contents
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| s.to_string())
        .collect();
    HostPolicy::from_patterns(&patterns).map_err(ReloadError::Compile)
}

/// Errors building a fresh policy from disk during reload.
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// Couldn't read the allowlist file.
    #[error("io error reading {}: {err}", path.display())]
    Io {
        /// Path that failed to open.
        path: PathBuf,
        /// Underlying io::Error message.
        err: String,
    },
    /// Patterns didn't compile (e.g. mid-string wildcard).
    #[error("policy compile error: {0}")]
    Compile(#[from] PolicyError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn handle_snapshot_returns_current_policy() {
        let policy = HostPolicy::from_patterns(&["api.stripe.com".to_string()]).unwrap();
        let handle = PolicyHandle::new(policy);
        let snap = handle.snapshot();
        assert_eq!(snap.check("api.stripe.com"), PolicyDecision::Allowed);
    }

    #[test]
    fn handle_replace_swaps_policy() {
        let initial = HostPolicy::from_patterns(&["a.example.com".to_string()]).unwrap();
        let handle = PolicyHandle::new(initial);

        // Old policy allows a.example.com but not b.example.com.
        assert_eq!(handle.check("a.example.com"), PolicyDecision::Allowed);
        assert!(matches!(handle.check("b.example.com"), PolicyDecision::Denied(_)));

        // Hot-swap.
        let updated = HostPolicy::from_patterns(&["b.example.com".to_string()]).unwrap();
        handle.replace(updated);

        assert!(matches!(handle.check("a.example.com"), PolicyDecision::Denied(_)));
        assert_eq!(handle.check("b.example.com"), PolicyDecision::Allowed);
    }

    #[test]
    fn handle_clone_shares_state() {
        let policy = HostPolicy::from_patterns(&["api.example.com".to_string()]).unwrap();
        let handle = PolicyHandle::new(policy);
        let cloned = handle.clone();

        // Replace via the clone — original sees the change.
        cloned.replace(HostPolicy::from_patterns(&["new.example.com".to_string()]).unwrap());
        assert_eq!(handle.check("new.example.com"), PolicyDecision::Allowed);
    }

    #[test]
    fn pre_reload_snapshot_survives_concurrent_replace() {
        let initial = HostPolicy::from_patterns(&["before.example.com".to_string()]).unwrap();
        let handle = PolicyHandle::new(initial);

        // Take a snapshot, then replace, then keep using the old
        // snapshot. This simulates a long-lived request that
        // started under an old policy — its decisions stay stable
        // for the request's lifetime.
        let snap = handle.snapshot();
        handle.replace(HostPolicy::from_patterns(&["after.example.com".to_string()]).unwrap());

        // Old snapshot still allows the old hostname.
        assert_eq!(snap.check("before.example.com"), PolicyDecision::Allowed);
        // New handle reads see the swap.
        assert_eq!(handle.check("after.example.com"), PolicyDecision::Allowed);
        assert!(matches!(handle.check("before.example.com"), PolicyDecision::Denied(_)));
    }

    #[test]
    fn load_policy_from_file_strips_comments_and_blank_lines() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "# header comment").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "api.stripe.com").unwrap();
        writeln!(file, "  *.stripe.com  ").unwrap();
        writeln!(file, "# another comment").unwrap();

        let policy = load_policy_from_file(file.path()).unwrap();
        assert_eq!(policy.check("api.stripe.com"), PolicyDecision::Allowed);
        assert_eq!(policy.check("events.stripe.com"), PolicyDecision::Allowed);
    }

    #[test]
    fn load_policy_from_file_surfaces_compile_errors() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "foo.*.com").unwrap(); // mid-string wildcard
        let result = load_policy_from_file(file.path());
        assert!(matches!(result, Err(ReloadError::Compile(_))));
    }

    #[test]
    fn load_policy_from_file_surfaces_missing_file_errors() {
        let result = load_policy_from_file(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(matches!(result, Err(ReloadError::Io { .. })));
    }
}
