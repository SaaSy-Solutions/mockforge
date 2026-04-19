//! Publisher SBOM-attestation key management.
//!
//! These commands wrap the `/api/v1/users/me/public-keys` REST surface
//! so publishers don't have to hand-craft `curl` calls. Supports:
//!
//! * `key list` — show the active keys on the current account.
//! * `key add` — register an existing Ed25519 public key (base64 or a
//!   file path).
//! * `key revoke` — soft-revoke a key by id.
//!
//! Key *generation* deliberately lives outside the CLI. The server never
//! holds the private half, so users need to produce it themselves (e.g.
//! `openssl genpkey -algorithm ed25519` or `age-keygen`). Doing it here
//! would imply we manage the private material — we don't want that.

use anyhow::{bail, Context, Result};
use base64::Engine;
use colored::*;
use ed25519_dalek::pkcs8::{spki::der::pem::LineEnding, EncodePrivateKey};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct CreatePublicKeyRequest<'a> {
    algorithm: &'a str,
    #[serde(rename = "publicKeyB64")]
    public_key_b64: &'a str,
    label: &'a str,
}

#[derive(Debug, Deserialize)]
struct PublicKeyResponse {
    id: String,
    algorithm: String,
    #[serde(rename = "publicKeyB64")]
    public_key_b64: String,
    label: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "revokedAt", default)]
    revoked_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListKeysResponse {
    keys: Vec<PublicKeyResponse>,
}

/// `mockforge-plugin key list`
pub async fn list_keys(registry: &str, token: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/users/me/public-keys", registry.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .context("sending list-keys request")?;
    if !resp.status().is_success() {
        bail!("registry returned {}: {}", resp.status(), resp.text().await.unwrap_or_default());
    }
    let body: ListKeysResponse = resp.json().await.context("decoding list-keys response")?;

    if body.keys.is_empty() {
        println!("{}", "No public keys registered on this account.".yellow());
        println!(
            "Add one with: {}",
            "mockforge-plugin key add --label <name> --file <path>".cyan()
        );
        return Ok(());
    }

    println!("{}", "Registered public keys:".bold());
    for key in body.keys {
        let fingerprint = fingerprint_short(&key.public_key_b64);
        println!("  {} {}", "•".cyan(), key.label.bold());
        println!("    id:          {}", key.id);
        println!("    algorithm:   {}", key.algorithm);
        println!("    fingerprint: {}", fingerprint);
        println!("    created:     {}", key.created_at);
        if let Some(rev) = key.revoked_at {
            println!("    revoked:     {} {}", rev, "(inactive)".red());
        }
    }
    Ok(())
}

/// `mockforge-plugin key add --label <x> (--file <path> | --public-key <b64>)`
pub async fn add_key(
    registry: &str,
    token: &str,
    label: &str,
    file: Option<&Path>,
    public_key_b64: Option<&str>,
) -> Result<()> {
    if label.trim().is_empty() {
        bail!("--label must not be empty");
    }

    let key_b64 = match (file, public_key_b64) {
        (Some(p), None) => read_key_file(p)?,
        (None, Some(b)) => b.trim().to_string(),
        (None, None) => {
            bail!("pass either --file <path> or --public-key <base64>");
        }
        (Some(_), Some(_)) => {
            bail!("pass only one of --file / --public-key, not both");
        }
    };

    // Length-check locally so the server's error isn't the first feedback
    // the user sees on an obvious typo.
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(key_b64.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_b64.trim()))
        .context("public key is not valid base64")?;
    if decoded.len() != 32 {
        bail!("ed25519 public key must be 32 bytes; got {}", decoded.len());
    }

    let body = CreatePublicKeyRequest {
        algorithm: "ed25519",
        public_key_b64: key_b64.trim(),
        label,
    };

    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/users/me/public-keys", registry.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context("sending add-key request")?;
    if !resp.status().is_success() {
        bail!("registry returned {}: {}", resp.status(), resp.text().await.unwrap_or_default());
    }
    let created: PublicKeyResponse = resp.json().await.context("decoding add-key response")?;
    println!(
        "{} Registered key {} ({})",
        "✅".green().bold(),
        created.id.cyan(),
        created.label
    );
    println!("   fingerprint: {}", fingerprint_short(&created.public_key_b64));
    Ok(())
}

/// `mockforge-plugin key revoke <id>`
pub async fn revoke_key(registry: &str, token: &str, id: &str) -> Result<()> {
    let uuid = Uuid::parse_str(id).context("key id is not a valid UUID")?;
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/users/me/public-keys/{}", registry.trim_end_matches('/'), uuid);
    let resp = client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .context("sending revoke request")?;
    if !resp.status().is_success() {
        bail!("registry returned {}: {}", resp.status(), resp.text().await.unwrap_or_default());
    }
    println!("{} Revoked key {}", "✅".green().bold(), uuid);
    Ok(())
}

/// `mockforge-plugin key gen --out <path>`
///
/// Generates a fresh Ed25519 keypair and writes the **private** half to
/// `--out` as a PKCS#8 PEM. The public half is printed to stdout as
/// base64 so the user can pipe it straight into `key add --public-key`.
/// We deliberately never transmit the private key anywhere — the whole
/// point of having this locally is that the server never sees it.
///
/// On Unix we set the file mode to 0600 after creating it so other
/// users on a shared machine can't read it. On Windows we rely on the
/// user's home directory being ACL-protected; we document the caveat
/// in the printed output.
pub async fn generate_key(out: &Path, force: bool) -> Result<()> {
    if out.exists() && !force {
        bail!(
            "{} already exists — refusing to overwrite. Pass --force to replace it.",
            out.display()
        );
    }

    // Fill 32 bytes from the OS entropy source. We go through rand's
    // default RNG rather than depending on a specific `rand_core`
    // version — `fill_bytes` has been stable across the rand 0.7 → 0.9
    // drift we see in the workspace. `thread_rng` was renamed to `rng`
    // in rand 0.9, so we keep the deprecated call behind an allow so
    // clippy `-D warnings` still passes on both versions. Once the
    // workspace settles on 0.9+ exclusively this can become `rand::rng()`.
    #[allow(deprecated)]
    let mut thread_rng = rand::thread_rng();
    use rand::RngCore;
    let mut secret = [0u8; 32];
    thread_rng.fill_bytes(&mut secret);
    let signing = SigningKey::from_bytes(&secret);
    let public_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());

    // PKCS#8 PEM so the file loads cleanly into openssl, ssh-keygen, age, etc.
    let pem = signing
        .to_pkcs8_pem(LineEnding::LF)
        .context("encoding private key as PKCS#8 PEM")?;

    // Create parent dirs if the user pointed at a nested path, then
    // write the file with 0600 on Unix.
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent directory {}", parent.display()))?;
        }
    }
    write_private_key_securely(out, pem.as_bytes())
        .with_context(|| format!("writing key to {}", out.display()))?;

    println!("{} Generated Ed25519 keypair.", "✅".green().bold());
    println!("   private key: {}", out.display());
    println!("   public  key: {}", public_b64.bold());
    println!();
    println!(
        "Register it on the registry with:\n    {} {}",
        "mockforge-plugin key add --label <name> --public-key".cyan(),
        public_b64.cyan()
    );
    Ok(())
}

/// Write `bytes` to `path` with the tightest per-platform file
/// permissions we can set without pulling in a Windows-ACL crate:
///
/// * **Unix** — open with `O_CREAT | O_TRUNC | mode(0o600)`, so the
///   file is only user-readable from the moment it exists. No
///   intermediate world-readable state.
/// * **Windows** — write the file with the standard `fs::write`, then
///   shell out to `icacls` to drop DACL inheritance and grant
///   `(OI)(CI)F` *only* to the current user. This is the Windows
///   equivalent of 0600. `icacls` ships in every supported Windows
///   version and avoids a `windows-sys`/`winapi` dependency on the
///   CLI's build graph. If `icacls` isn't on PATH (rare) we emit a
///   clear warning so the operator can tighten the ACL by hand.
fn write_private_key_securely(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    }
    #[cfg(windows)]
    {
        std::fs::write(path, bytes)?;
        tighten_acl_windows(path);
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        std::fs::write(path, bytes)
    }
}

/// On Windows, after writing the file, strip ACL inheritance and grant
/// full control to only the current user. Warnings are non-fatal — the
/// file already exists and is protected by the home-directory ACL in
/// the common case; we just want to belt-and-brace it.
#[cfg(windows)]
fn tighten_acl_windows(path: &Path) {
    match resolve_windows_user() {
        Some(user) => run_icacls(path, &user),
        None => {
            eprintln!(
                "{}",
                "warning: could not determine current Windows user; ACL \
                 tightening skipped. Review the key file's permissions manually."
                    .yellow()
            );
        }
    }
}

/// Exposed as a plain (non-cfg) function so we can unit-test it on any
/// platform. Returns the username `icacls` should grant full-control
/// to, matching the resolution order real Windows shells use:
/// `%USERNAME%` first, then `%USERDOMAIN%\%USERNAME%`. The function is
/// pure-environment-lookup so tests can manipulate `env::set_var`
/// to exercise every branch without needing a Windows host.
#[cfg_attr(not(windows), allow(dead_code))]
fn resolve_windows_user() -> Option<String> {
    let username = std::env::var("USERNAME").ok().filter(|s| !s.trim().is_empty());
    match username {
        Some(u) => {
            // Prefer DOMAIN\USER when USERDOMAIN is set — matches how
            // `whoami` prints on a joined machine and avoids ambiguity
            // when the same username exists on multiple domains.
            if let Ok(domain) = std::env::var("USERDOMAIN") {
                if !domain.trim().is_empty() {
                    return Some(format!("{}\\{}", domain, u));
                }
            }
            Some(u)
        }
        None => None,
    }
}

/// Run `icacls` to replace the file's DACL with a single full-control
/// entry for `user`. Split out so the logic is type-checked on any
/// target (the caller gates the actual invocation behind
/// `cfg(windows)`). `dead_code` is allowed on non-Windows targets
/// because the only call site is inside `tighten_acl_windows`.
#[cfg_attr(not(windows), allow(dead_code))]
fn run_icacls(path: &Path, user: &str) {
    use std::process::Command;
    let status = Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{}:F", user))
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!(
            "{} icacls exited with {}. The key file exists but its ACL \
             may still inherit permissions from the parent directory — \
             review manually.",
            "warning:".yellow(),
            s
        ),
        Err(e) => eprintln!(
            "{} could not run icacls ({}). Tighten the key file's ACL \
             manually so only your account can read it.",
            "warning:".yellow(),
            e
        ),
    }
}

/// Read the PKCS#8 PEM the user produced (either via `key gen` or an
/// external tool like `openssl genpkey -algorithm ed25519`) and return
/// the underlying `SigningKey`. Exposed to the `publish --sign` path so
/// both commands share the same file format.
pub(crate) fn load_signing_key(path: &Path) -> Result<SigningKey> {
    use ed25519_dalek::pkcs8::DecodePrivateKey;
    let pem = std::fs::read_to_string(path)
        .with_context(|| format!("reading private key from {}", path.display()))?;
    let signing = SigningKey::from_pkcs8_pem(&pem).with_context(|| {
        format!(
            "{} is not a PKCS#8 PEM Ed25519 private key (try `openssl genpkey -algorithm ed25519`)",
            path.display()
        )
    })?;
    Ok(signing)
}

/// Canonicalize the SBOM bytes + compute the signed message the
/// registry's attestation verifier expects: `SHA-256(hex_decode(checksum)
/// || sbom_canonical)`. Exposed so both `key sign` (standalone) and
/// `publish --sign` (embedded) share the exact same byte layout.
pub(crate) fn attestation_message(
    artifact_checksum_hex: &str,
    sbom_canonical: &[u8],
) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let checksum = hex::decode(artifact_checksum_hex.trim())
        .with_context(|| format!("artifact checksum is not hex: {}", artifact_checksum_hex))?;
    let mut hasher = Sha256::new();
    hasher.update(&checksum);
    hasher.update(sbom_canonical);
    Ok(hasher.finalize().into())
}

/// Read an SBOM JSON file and canonicalize it into the byte form we'll
/// actually sign. Canonicalization is RFC 8785 JCS (via `serde_jcs`):
/// keys sorted lexicographically, numbers in shortest round-trip form,
/// whitespace stripped, Unicode NFC normalization. That guarantees two
/// publishers producing "the same" SBOM via different JSON libraries
/// (or the same library across serde versions) emit byte-identical
/// inputs to the signer — which is what lets the server verifier
/// accept signatures regardless of which toolchain produced them.
///
/// The server reads the bytes we send verbatim and re-canonicalizes
/// them with the same crate before verifying; so long as both sides
/// agree on "canonical," an SBOM signed here always validates there.
pub(crate) fn read_and_canonicalize_sbom(path: &Path) -> Result<Vec<u8>> {
    let raw =
        std::fs::read(path).with_context(|| format!("reading SBOM from {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_slice(&raw).context("SBOM is not valid JSON")?;
    serde_jcs::to_vec(&value).context("canonicalizing SBOM with RFC 8785 (JCS)")
}

/// `mockforge-plugin key rotate --out <path> --label <name> [--revoke <id>]`
///
/// Atomic, operator-friendly rotation:
///
/// 1. Generate a fresh keypair to `--out` (the old key file is left
///    alone — callers that want to replace it should point `--out` at
///    the same path and pass `--force`).
/// 2. Register the new public key on the registry under `--label`.
/// 3. If `--revoke <id>` was supplied (or `--revoke-previous` was
///    passed), revoke the old key *after* the new one is registered —
///    never before, so the account is never in a window with zero
///    active keys.
///
/// The three-step flow is scriptable today with `gen` + `add` + `revoke`,
/// but doing it in one command keeps the "new first, old last"
/// ordering a single atomic action from the user's perspective.
pub async fn rotate_key(
    registry: &str,
    token: &str,
    out: &Path,
    force: bool,
    label: &str,
    revoke_previous_id: Option<&str>,
) -> Result<()> {
    // Step 1: generate.
    generate_key(out, force).await?;

    // Step 2: pull the public key back out of the file we just wrote
    // and register it. We re-read rather than threading the public key
    // through `generate_key` so the CLI has one canonical path from
    // PKCS#8 to base64.
    let signing = load_signing_key(out)?;
    let public_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());

    println!();
    println!("{} Registering new key…", "→".cyan());
    add_key(registry, token, label, None, Some(&public_b64)).await?;

    // Step 3: revoke the previous key only after the new one is live,
    // so the account is never momentarily keyless.
    if let Some(id) = revoke_previous_id {
        println!();
        println!("{} Revoking previous key {}…", "→".cyan(), id);
        revoke_key(registry, token, id).await?;
    }
    println!();
    println!("{}", "Rotation complete.".green().bold());
    Ok(())
}

/// `mockforge-plugin key sign --key-file <path> --checksum <hex> --sbom <path>`
///
/// Prints the detached base64 Ed25519 signature over
/// `SHA-256(checksum_bytes || sbom_canonical_json)`. Useful as a
/// standalone step in a CI pipeline that produces the signature in one
/// stage and submits it to the registry in a later stage; `publish
/// --sign` is the one-shot wrapper for interactive use.
pub async fn sign_sbom(
    key_file: &Path,
    artifact_checksum_hex: &str,
    sbom_path: &Path,
) -> Result<()> {
    let signing = load_signing_key(key_file)?;
    let sbom_bytes = read_and_canonicalize_sbom(sbom_path)?;
    let msg = attestation_message(artifact_checksum_hex, &sbom_bytes)?;
    let sig = ed25519_dalek::Signer::sign(&signing, &msg);
    let b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
    println!("{}", b64);
    Ok(())
}

/// Convenience wrapper used by the CLI dispatcher; keeps the flag
/// shape documented next to its handler.
pub async fn generate_key_cli(out: Option<PathBuf>, force: bool) -> Result<()> {
    let path = out.unwrap_or_else(|| PathBuf::from("mockforge_publisher_key.pem"));
    generate_key(&path, force).await
}

/// Read a public key from a file and return a base64 string. Accepts:
/// a bare base64 blob, a PEM-wrapped SPKI key (we strip the header/footer
/// and ignore the DER wrapping), or a JWK-style `"x": "<b64>"` field.
/// The goal is "paste whatever your keygen tool produced" rather than
/// forcing a specific format.
fn read_key_file(path: &Path) -> Result<String> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading key file {}", path.display()))?;
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        // Assume JWK.
        let v: serde_json::Value =
            serde_json::from_str(trimmed).context("parsing key file as JWK JSON")?;
        if let Some(x) = v.get("x").and_then(|v| v.as_str()) {
            return Ok(x.to_string());
        }
        bail!("JWK key file has no `x` field");
    }
    if trimmed.starts_with("-----BEGIN") {
        // Strip PEM envelope and whitespace; what's left is base64 SPKI.
        // Ed25519 SPKI is 44 bytes (12-byte header + 32 key) so we
        // base64-decode, slice off the trailing 32, and re-encode. This
        // avoids pulling in a full PEM parser.
        let body: String = trimmed
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<String>()
            .split_whitespace()
            .collect();
        let der = base64::engine::general_purpose::STANDARD
            .decode(&body)
            .context("PEM body is not valid base64")?;
        if der.len() < 32 {
            bail!("PEM-encoded key is too short to contain an ed25519 public key");
        }
        let raw_key = &der[der.len() - 32..];
        return Ok(base64::engine::general_purpose::STANDARD.encode(raw_key));
    }
    // Treat anything else as a raw base64 blob.
    Ok(trimmed.to_string())
}

/// Short fingerprint for UI display — SHA-256 of the decoded public key,
/// hex-encoded, first 16 chars. Not a security-critical identifier; it's
/// just a "is this the key I think it is?" sanity check for humans.
fn fingerprint_short(public_key_b64: &str) -> String {
    use sha2::{Digest, Sha256};
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(public_key_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(public_key_b64))
        .unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    let hex_str: String = digest.iter().take(8).map(|b| format!("{:02x}", b)).collect();
    hex_str
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_key_file_accepts_bare_base64() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap();
        assert_eq!(
            read_key_file(tmp.path()).unwrap(),
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        );
    }

    #[test]
    fn read_key_file_accepts_jwk() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            r#"{"kty":"OKP","crv":"Ed25519","x":"CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC="}"#,
        )
        .unwrap();
        assert_eq!(
            read_key_file(tmp.path()).unwrap(),
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC="
        );
    }

    #[tokio::test]
    async fn generate_key_writes_pkcs8_pem_and_loads_back() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("k.pem");
        generate_key(&path, false).await.unwrap();

        // Written file starts with the PKCS#8 PEM header.
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert!(bytes.starts_with("-----BEGIN PRIVATE KEY-----"));

        // Round-trips via load_signing_key so `publish --sign` can use
        // the same file.
        let signing = load_signing_key(&path).unwrap();
        let msg = b"hi";
        let sig = ed25519_dalek::Signer::sign(&signing, msg);
        ed25519_dalek::Verifier::verify(&signing.verifying_key(), msg, &sig).unwrap();

        // Refuses to overwrite without --force.
        let err = generate_key(&path, false).await.unwrap_err().to_string();
        assert!(err.contains("refusing to overwrite"), "got: {}", err);

        // --force replaces the file with a new key.
        let before = std::fs::read_to_string(&path).unwrap();
        generate_key(&path, true).await.unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_ne!(before, after, "expected a fresh key on overwrite");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn generate_key_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("perm.pem");
        generate_key(&path, false).await.unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        // umask could narrow further; we only require "not world-readable."
        assert_eq!(
            mode & 0o077,
            0,
            "private key should not be group/world readable (got {:o})",
            mode
        );
    }

    /// `resolve_windows_user` is environment-driven so we can
    /// exercise both branches from any platform. We deliberately set
    /// and unset the same two variables the real Windows shell
    /// inherits so the test is reproducible.
    #[test]
    fn resolve_windows_user_prefers_domain_prefix() {
        // Serialize against the rest of the process's env — these
        // variables are ambient on Windows, but the test itself runs
        // inside a single-threaded `#[test]` harness where nothing
        // else touches them. Restore-on-drop would be ideal, but
        // nothing in this test file reads USERDOMAIN/USERNAME.
        std::env::set_var("USERNAME", "alice");
        std::env::remove_var("USERDOMAIN");
        assert_eq!(resolve_windows_user().as_deref(), Some("alice"));

        std::env::set_var("USERDOMAIN", "CORP");
        assert_eq!(resolve_windows_user().as_deref(), Some("CORP\\alice"));

        // Empty USERDOMAIN must not produce a leading `\`.
        std::env::set_var("USERDOMAIN", "  ");
        assert_eq!(resolve_windows_user().as_deref(), Some("alice"));

        // Missing USERNAME → None, even with USERDOMAIN set.
        std::env::remove_var("USERNAME");
        std::env::set_var("USERDOMAIN", "CORP");
        assert!(resolve_windows_user().is_none());

        // Whitespace-only USERNAME is treated as unset — avoids
        // icacls erroring on `CORP\<spaces>:F`.
        std::env::set_var("USERNAME", "   ");
        assert!(resolve_windows_user().is_none());
    }

    /// End-to-end check for the Windows ACL tightening path. Gated
    /// `#[cfg(windows)]` so it only runs on the `windows-latest` leg of
    /// `cross-platform-test` in `.github/workflows/ci.yml` — which is
    /// exactly the coverage the Unix-only `0600` test already has.
    ///
    /// We assert **semantic** properties by reading the DACL back via
    /// PowerShell's `Get-Acl` (which returns a structured object whose
    /// JSON form is locale-independent), not by string-matching on
    /// `icacls` output. That lets us fail loudly if somebody regresses
    /// `tighten_acl_windows` into a no-op, while staying robust to
    /// `icacls`'s localized / drifting stdout:
    ///
    /// 1. `AreAccessRulesProtected == true` — inheritance was actually
    ///    stripped. This is the check that a silent regression (e.g.
    ///    `/inheritance:r` getting dropped from the command line) would
    ///    fail first.
    /// 2. The DACL grants FullControl to the current user. Without
    ///    this, `cargo` wouldn't even be able to re-read its own file.
    /// 3. The DACL contains no allow rule for `BUILTIN\Users`,
    ///    `NT AUTHORITY\Authenticated Users`, or `Everyone` — i.e. no
    ///    broad principal still has a foothold. These SDDL names are
    ///    stable across Windows SKUs and locales, unlike the English
    ///    phrases icacls prints.
    /// 4. The current process can still read the file it just wrote —
    ///    catches the case where we somehow revoked our own access.
    #[cfg(windows)]
    #[tokio::test]
    async fn generate_key_tightens_acl_on_windows() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("win-key.pem");
        generate_key(&path, false).await.unwrap();

        let contents = std::fs::read_to_string(&path).expect("key readable after ACL tighten");
        assert!(
            contents.starts_with("-----BEGIN PRIVATE KEY-----"),
            "unexpected key contents: {}",
            &contents[..contents.len().min(64)]
        );

        // Ask PowerShell for the ACL as JSON. `Get-Acl` returns a
        // .NET object; `Select-Object` projects out just the fields
        // we need so the JSON is bounded in size and shape. We use
        // `-Depth 4` to make sure the nested `Access` array round-
        // trips fully (PowerShell's default depth is 2, which would
        // collapse the FileSystemAccessRule entries to strings).
        let path_str = path.to_str().expect("tempdir path is utf-8");
        let ps_script = format!(
            "(Get-Acl -LiteralPath '{}') | \
             Select-Object @{{Name='AreAccessRulesProtected';Expression={{$_.AreAccessRulesProtected}}}}, \
                           @{{Name='Access';Expression={{$_.Access | ForEach-Object {{ \
                               @{{ IdentityReference = $_.IdentityReference.Value; \
                                   FileSystemRights  = $_.FileSystemRights.ToString(); \
                                   AccessControlType = $_.AccessControlType.ToString(); \
                                   IsInherited       = $_.IsInherited }} \
                           }} }}}} | \
             ConvertTo-Json -Depth 4 -Compress",
            path_str.replace('\'', "''")
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
            .expect("powershell present on windows-latest runners");
        assert!(
            output.status.success(),
            "Get-Acl failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let json_str = String::from_utf8(output.stdout).expect("Get-Acl output is utf-8");
        let acl: serde_json::Value = serde_json::from_str(json_str.trim())
            .unwrap_or_else(|e| panic!("Get-Acl JSON did not parse: {e}; raw={json_str}"));

        // (1) Inheritance actually got stripped.
        assert_eq!(
            acl.get("AreAccessRulesProtected").and_then(|v| v.as_bool()),
            Some(true),
            "expected inheritance stripped; full ACL: {acl}"
        );

        // Normalize Access to an array. If the file ended up with a
        // single ACE, PowerShell serializes it as an object rather
        // than a one-element array.
        let access_rules: Vec<&serde_json::Value> = match acl.get("Access") {
            Some(serde_json::Value::Array(items)) => items.iter().collect(),
            Some(obj @ serde_json::Value::Object(_)) => vec![obj],
            other => panic!("unexpected Access shape: {other:?}; full ACL: {acl}"),
        };
        assert!(!access_rules.is_empty(), "DACL is empty; full ACL: {acl}");

        let rule_identity = |r: &serde_json::Value| -> String {
            r.get("IdentityReference").and_then(|v| v.as_str()).unwrap_or("").to_string()
        };
        let rule_rights = |r: &serde_json::Value| -> String {
            r.get("FileSystemRights").and_then(|v| v.as_str()).unwrap_or("").to_string()
        };
        let rule_is_allow = |r: &serde_json::Value| -> bool {
            r.get("AccessControlType").and_then(|v| v.as_str()) == Some("Allow")
        };

        // (2) Current user got a FullControl allow rule. Fall back
        //     to substring matches on the known identity fragments
        //     because the resolved name may be "COMPUTER\runner",
        //     "CORP\runner", or a bare "runner" depending on the
        //     domain state of the runner.
        let expected_user = resolve_windows_user().expect("USERNAME set on the runner");
        let user_has_full = access_rules.iter().copied().any(|r| {
            rule_is_allow(r)
                && rule_identity(r).eq_ignore_ascii_case(&expected_user)
                && rule_rights(r).contains("FullControl")
        });
        assert!(
            user_has_full,
            "expected FullControl Allow for {expected_user:?}; full ACL: {acl}"
        );

        // (3) No broad principal has a foothold. These are SID-
        //     resolved names that Windows prints in the same ASCII
        //     form on every locale (the SIDs themselves are
        //     well-known: S-1-1-0, S-1-5-11, S-1-5-32-545).
        const FORBIDDEN: &[&str] = &[
            "Everyone",
            "NT AUTHORITY\\Authenticated Users",
            "BUILTIN\\Users",
        ];
        for rule in &access_rules {
            if !rule_is_allow(rule) {
                continue;
            }
            let ident = rule_identity(rule);
            for bad in FORBIDDEN {
                assert!(
                    !ident.eq_ignore_ascii_case(bad),
                    "forbidden principal {bad:?} still has allow rule {rule}; full ACL: {acl}"
                );
            }
        }
    }

    #[test]
    fn fingerprint_is_stable_and_short() {
        let f = fingerprint_short("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        assert_eq!(f.len(), 16);
        // Re-running must produce the same value.
        assert_eq!(f, fingerprint_short("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="));
    }
}
