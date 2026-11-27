//! No key of a known shape is in this repository, beyond the made-up
//! examples named below. The scan reads every tracked text file, and not
//! only the Rust sources: scripts, the UI, the guide, the examples, the
//! workflows. What it looks for is the table of
//! [`mockforge_contracts::keys`], which the exchange log and `range new`
//! use, so that the scan and they cannot drift apart.
//!
//! A match is not proof of a leak: a value can be of a known shape and be
//! made up. Each example that is made up is named here, with why it is
//! there; anything else that matches fails the test and is to be looked at
//! by a person.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mockforge_contracts::keys::{KeyFormat, Part, FORMATS};

/// The made-up examples of a key's shape that the tree holds.
const MADE_UP: &[(&str, &str)] = &[
    ("ghp_0123456789012345678901234567890123456789", "keys.rs: a GitHub token of the right shape, made up"),
    ("ghp_9876543210987654321098765432109876543210", "keys.rs: a GitHub token of the right shape, made up"),
    (
        "sk_live_4f2a9c81e7b3d6a04f2a9c81",
        "keys.rs, and the guide's table of what the data synthesis recognises: a Stripe key made up",
    ),
    // The example access key id of AWS's own documentation, in the vendored crate, and
    // the one a character of which its test changes to show a signature diverging.
    ("AKIAIOSFODNN7EXAMPLE", "vendor/aws-sigv4: the example access key id of AWS's documentation"),
    ("sk_live_01234567890123456789012345", "judge.rs: another Stripe key made up, whose digest does not show a flaw"),
    ("AKIAIOSFODNN7EXAMPLF", "vendor/aws-sigv4: the same example with its last letter changed, in a test"),
    ("xoxb-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx", "keys.rs: a Slack token of the right shape, made up"),
    ("SG.abcdefghijklmnopqrstuv.ABCDEFGHIJKLMNOPQRSTUVWXYZ", "keys.rs: a SendGrid key of the right shape, made up"),
    ("1098765432:AAHdqTcvbXHJKl-MnbxCvbnAsdfghjklpoi", "keys.rs: a Telegram token of the right shape, made up"),
];

/// Directories that hold nothing a person wrote, or repeat what another
/// directory holds.
const SKIP: [&str; 5] = [
    ".git",
    "target",
    "node_modules",
    "book/book",
    "crates/mockforge-ui/ui/dist",
];

fn is_key_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"_.+/=-".contains(&byte)
}

/// The shapes to try for a token by its first character: those whose shape
/// starts with that character, and the two whose shape starts with
/// characters of a set when the token holds what follows them.
fn candidates<'a>(
    index: &'a BTreeMap<u8, Vec<&'static KeyFormat>>,
    token: &[u8],
) -> Vec<&'a KeyFormat> {
    let mut found = index.get(&token[0]).cloned().unwrap_or_default();
    if token.contains(&b':') || token.windows(9).any(|window| window == b".atlasv1.") {
        found.extend(
            FORMATS
                .iter()
                .filter(|format| !matches!(format.parts.first(), Some(Part::Literal(_)))),
        );
    }
    found
}

fn files_under(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let relative =
                path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            if SKIP.contains(&relative.as_str()) {
                continue;
            }
            files_under(root, &path, found);
        } else {
            found.push(path);
        }
    }
}

#[test]
fn no_key_of_a_known_shape_is_in_the_tree() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", ".."].iter().collect();
    let root = root.canonicalize().expect("the root of the repository");
    let mut index: BTreeMap<u8, Vec<&'static KeyFormat>> = BTreeMap::new();
    for format in FORMATS {
        if let Some(Part::Literal(literal)) = format.parts.first() {
            index.entry(literal.as_bytes()[0]).or_default().push(format);
        }
    }

    let mut files = Vec::new();
    files_under(&root, &root, &mut files);
    files.sort();
    assert!(files.len() > 1000, "the walk found {} files", files.len());

    let mut leaks = Vec::new();
    let mut scanned = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        // A file that is not text, or one that is enormous, is not read.
        if bytes.len() > 8 * 1024 * 1024 || bytes.iter().take(8192).any(|byte| *byte == 0) {
            continue;
        }
        scanned += 1;
        let shown = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        let mut start = None;
        for (position, byte) in bytes.iter().enumerate() {
            if is_key_character(*byte) {
                if start.is_none() {
                    start = Some(position);
                }
            } else if let Some(begin) = start.take() {
                check(&bytes[begin..position], &shown, &index, &mut leaks);
            }
        }
        if let Some(begin) = start {
            check(&bytes[begin..], &shown, &index, &mut leaks);
        }
    }
    eprintln!("read {scanned} text files under {}", root.display());
    assert!(
        leaks.is_empty(),
        "a value of a known key's shape is in the tree; each is to be looked at, and named in MADE_UP if it is made up:\n{}",
        leaks.join("\n")
    );
}

/// One run of characters a key is made of.
fn check(
    token: &[u8],
    shown: &str,
    index: &BTreeMap<u8, Vec<&'static KeyFormat>>,
    leaks: &mut Vec<String>,
) {
    if token.len() < 16 || token.starts_with(b"sha256:") {
        return;
    }
    let Ok(text) = std::str::from_utf8(token) else {
        return;
    };
    if MADE_UP.iter().any(|(made_up, _)| *made_up == text) {
        return;
    }
    if let Some(format) = candidates(index, token).into_iter().find(|format| format.matches(text)) {
        leaks.push(format!(
            "{shown}: {} {} ({})",
            text.chars().take(6).collect::<String>(),
            format.provider,
            format.name
        ));
    }
}
