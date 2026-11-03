//! Structural guard: no audit-log write may carry an email address, username
//! or IdP subject.
//!
//! `audit_logs` rows are immutable and hash-chained (#872) and are retained
//! after account erasure (#1087), so personal identifiers written into
//! `description` or `metadata` can never be removed. Person-related events go
//! through `mockforge_registry_server::audit_entries`, which records ids only.
//! This test scans every `record_audit_event(` / `AuditLog::create(` call in
//! the registry crates and fails if its arguments mention an identifier that
//! looks like personal data, so a new call site cannot quietly reintroduce it.

use std::fs;
use std::path::{Path, PathBuf};

/// Substrings that must not appear in the argument list of an audit write.
/// Matched case-insensitively against the call text with whitespace removed.
const FORBIDDEN: &[&str] = &["email", "username", "name_id", "display_name", "full_name"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Return the text between the call's opening paren and its matching close.
fn call_args(src: &str, open_paren: usize) -> &str {
    let mut depth = 0usize;
    for (i, ch) in src[open_paren..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open_paren + 1..open_paren + i];
                }
            }
            _ => {}
        }
    }
    &src[open_paren + 1..]
}

fn violations_in(path: &Path, src: &str) -> Vec<String> {
    let mut found = Vec::new();
    for needle in ["record_audit_event(", "AuditLog::create("] {
        let mut from = 0;
        while let Some(rel) = src[from..].find(needle) {
            let start = from + rel;
            from = start + needle.len();
            // Skip the definitions themselves (`fn record_audit_event(`).
            let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
            if src[line_start..start].contains("fn ") {
                continue;
            }
            let args = call_args(src, start + needle.len() - 1);
            let compact: String =
                args.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
            for bad in FORBIDDEN {
                if compact.contains(bad) {
                    let line = src[..start].matches('\n').count() + 1;
                    found.push(format!("{}:{line}: audit write mentions `{bad}`", path.display()));
                }
            }
        }
    }
    found
}

#[test]
fn no_audit_write_records_personal_identifiers() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&manifest.join("src"), &mut files);
    rust_files(&manifest.join("../mockforge-registry-core/src"), &mut files);
    assert!(files.len() > 50, "scanned too few files ({}); wrong root?", files.len());

    let mut calls = 0;
    let mut violations = Vec::new();
    for path in &files {
        let src = fs::read_to_string(path).unwrap();
        calls +=
            src.matches("record_audit_event(").count() + src.matches("AuditLog::create(").count();
        violations.extend(violations_in(path, &src));
    }
    assert!(calls > 50, "found only {calls} audit writes; the scan has gone blind");
    assert!(
        violations.is_empty(),
        "audit rows are immutable and outlive erasure; record ids, not identities \
         (see src/audit_entries.rs):\n{}",
        violations.join("\n")
    );
}

#[test]
fn guard_flags_an_email_in_an_audit_call() {
    let src = r#"
        state.store.record_audit_event(
            org_id,
            Some(user.id),
            AuditEventType::MemberAdded,
            format!("Added {}", target.email),
            None,
            None,
            None,
        ).await;
    "#;
    assert_eq!(violations_in(Path::new("fixture.rs"), src).len(), 1);
    let clean = src.replace("target.email", "target.id");
    assert!(violations_in(Path::new("fixture.rs"), &clean).is_empty());
}
