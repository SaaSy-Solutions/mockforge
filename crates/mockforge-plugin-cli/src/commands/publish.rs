//! Publish plugin command

use crate::utils::{current_dir, find_manifest, get_plugin_id, get_plugin_version, read_manifest};
use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};

/// Options controlling SBOM attestation during publish. When `key_file`
/// and `sbom_path` are both supplied the command canonicalizes the SBOM,
/// computes a detached Ed25519 signature against
/// `SHA-256(hex_decode(checksum) || canonical(sbom))`, and attaches both
/// the SBOM and the signature to the upload. The registry verifies the
/// signature against the publisher's registered public keys (see
/// `mockforge-plugin key add`).
#[derive(Debug, Default, Clone)]
pub struct SignOptions {
    pub key_file: Option<PathBuf>,
    pub sbom_path: Option<PathBuf>,
}

/// Publish a plugin package to the registry.
pub async fn publish_plugin(
    path: Option<&Path>,
    registry: &str,
    token: Option<&str>,
    dry_run: bool,
    sign: SignOptions,
) -> Result<()> {
    // Determine project directory or package path
    let project_path = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Publishing plugin...".cyan().bold());
    println!();

    // Resolve the package file to upload
    let (plugin_id, plugin_version, package_path) = resolve_package(&project_path)?;

    println!("  {} {}", "Plugin:".bold(), plugin_id);
    println!("  {} {}", "Version:".bold(), plugin_version);
    println!("  {} {}", "Package:".bold(), package_path.display());
    println!("  {} {}", "Registry:".bold(), registry);
    println!();

    // Validate the package before publishing
    println!("{}", "Validating package...".cyan());
    validate_package(&package_path)?;
    println!("{} Package is valid", "  ✓".green());
    println!();

    if dry_run {
        println!(
            "{}",
            "Dry run: skipping actual publish. The following would be published:"
                .yellow()
                .bold()
        );
        println!("  {} {}@{}", "Plugin:".bold(), plugin_id, plugin_version);
        println!("  {} {}", "Package:".bold(), package_path.display());
        println!("  {} {}", "Registry:".bold(), registry);

        let metadata = std::fs::metadata(&package_path).with_context(|| {
            format!("Failed to read package metadata: {}", package_path.display())
        })?;
        println!("  {} {} bytes", "Size:".bold(), metadata.len());

        return Ok(());
    }

    // Check for authentication token
    let auth_token = token.filter(|t| !t.is_empty()).context(
        "Authentication token is required. Provide --token or set MOCKFORGE_REGISTRY_TOKEN",
    )?;

    // Compute SBOM attestation if the publisher asked for it. We do
    // this *before* the upload so a signing failure doesn't waste
    // bandwidth uploading an un-attestable artifact.
    let attestation = build_sbom_attestation(&package_path, &sign)?;

    // Upload the package (with optional SBOM + signature attached as
    // multipart fields the server picks up).
    upload_package(
        &package_path,
        &plugin_id,
        &plugin_version,
        registry,
        auth_token,
        attestation.as_ref(),
    )
    .await?;

    println!();
    println!(
        "{} {}@{} published to {}",
        "✅ Successfully published".green().bold(),
        plugin_id,
        plugin_version,
        registry
    );

    Ok(())
}

/// Resolve the package file to upload.
///
/// If the path is a `.zip` file, use it directly.
/// If the path is a directory, look for a packaged `.zip` file inside it.
fn resolve_package(path: &Path) -> Result<(String, String, PathBuf)> {
    if path.is_file() {
        // Path points to a package file directly — read manifest from the zip
        let (plugin_id, plugin_version) = read_manifest_from_zip(path)?;
        return Ok((plugin_id, plugin_version, path.to_path_buf()));
    }

    // Path is a directory — look for manifest and a packaged zip
    let manifest_path = find_manifest(path)?;
    let manifest = read_manifest(&manifest_path)?;
    let plugin_id = get_plugin_id(&manifest)?;
    let plugin_version = get_plugin_version(&manifest)?;

    let package_path = path.join(format!("{}.zip", plugin_id));
    if !package_path.exists() {
        anyhow::bail!(
            "No package found at {}. Run 'mockforge-plugin package' first.",
            package_path.display()
        );
    }

    Ok((plugin_id, plugin_version, package_path))
}

/// Read plugin id and version from a zip package's embedded plugin.yaml.
fn read_manifest_from_zip(zip_path: &Path) -> Result<(String, String)> {
    let file = std::fs::File::open(zip_path)
        .with_context(|| format!("Failed to open package: {}", zip_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read zip archive: {}", zip_path.display()))?;

    let manifest_name = {
        let mut found = None;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name == "plugin.yaml" || name == "plugin.yml" {
                    found = Some(name);
                    break;
                }
            }
        }
        found.context("Package does not contain a plugin.yaml or plugin.yml")?
    };

    let manifest_content = {
        let mut buf = String::new();
        let mut entry = archive
            .by_name(&manifest_name)
            .context("Failed to read manifest from package")?;
        std::io::Read::read_to_string(&mut entry, &mut buf)
            .context("Failed to read manifest from package")?;
        buf
    };

    let manifest: serde_yaml::Value =
        serde_yaml::from_str(&manifest_content).context("Failed to parse manifest in package")?;

    let plugin_id = get_plugin_id(&manifest)?;
    let plugin_version = get_plugin_version(&manifest)?;

    Ok((plugin_id, plugin_version))
}

/// Validate that the package file looks correct.
fn validate_package(package_path: &Path) -> Result<()> {
    if !package_path.exists() {
        anyhow::bail!("Package file not found: {}", package_path.display());
    }

    let file = std::fs::File::open(package_path)
        .with_context(|| format!("Failed to open package: {}", package_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Invalid zip archive: {}", package_path.display()))?;

    // Check for required files
    let has_manifest =
        archive.by_name("plugin.yaml").is_ok() || archive.by_name("plugin.yml").is_ok();
    if !has_manifest {
        anyhow::bail!("Package is missing plugin.yaml manifest");
    }

    // Check for at least one .wasm file
    let has_wasm = (0..archive.len()).any(|i| {
        archive
            .by_index(i)
            .map(|entry| entry.name().ends_with(".wasm"))
            .unwrap_or(false)
    });
    if !has_wasm {
        anyhow::bail!("Package is missing a .wasm module");
    }

    Ok(())
}

/// Optional attestation block computed locally before upload. The
/// `sbom_canonical_bytes` are the exact bytes the signature covers — we
/// send them verbatim so server-side canonicalization doesn't have to
/// match ours byte-for-byte.
#[derive(Debug)]
struct SbomAttestation {
    sbom_canonical_bytes: Vec<u8>,
    signature_b64: String,
}

/// Build the SBOM attestation payload from the publisher's
/// `SignOptions`. Returns `None` when the publisher didn't ask for
/// signing, `Err` when they did but something's wrong with the inputs
/// (missing file, malformed key, etc.) so the surprise happens before
/// the upload rather than after.
fn build_sbom_attestation(
    package_path: &Path,
    sign: &SignOptions,
) -> Result<Option<SbomAttestation>> {
    use base64::Engine;
    use sha2::{Digest, Sha256};

    let (key_file, sbom_path) = match (&sign.key_file, &sign.sbom_path) {
        (Some(k), Some(s)) => (k, s),
        (None, None) => return Ok(None),
        (None, Some(_)) => anyhow::bail!("--sbom supplied without --key-file"),
        (Some(_), None) => anyhow::bail!("--key-file supplied without --sbom"),
    };

    // Compute the artifact checksum the signature must commit to. The
    // server recomputes this server-side and rejects mismatches, so we
    // need to match exactly.
    let wasm_bytes = std::fs::read(package_path)
        .with_context(|| format!("reading package for signing: {}", package_path.display()))?;
    let checksum_bytes: [u8; 32] = Sha256::digest(&wasm_bytes).into();
    let checksum_hex: String = checksum_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let sbom_canonical = crate::commands::key::read_and_canonicalize_sbom(sbom_path)?;
    let signing = crate::commands::key::load_signing_key(key_file)?;
    let message = crate::commands::key::attestation_message(&checksum_hex, &sbom_canonical)?;
    let sig = ed25519_dalek::Signer::sign(&signing, &message);

    println!("{}  signed SBOM attestation.", "  ✓".green());
    Ok(Some(SbomAttestation {
        sbom_canonical_bytes: sbom_canonical,
        signature_b64: base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()),
    }))
}

/// Upload the package to the registry.
async fn upload_package(
    package_path: &Path,
    plugin_id: &str,
    plugin_version: &str,
    registry: &str,
    token: &str,
    attestation: Option<&SbomAttestation>,
) -> Result<()> {
    let file_bytes = std::fs::read(package_path)
        .with_context(|| format!("Failed to read package: {}", package_path.display()))?;

    let file_name = package_path
        .file_name()
        .context("Invalid package path")?
        .to_str()
        .context("Invalid package filename")?
        .to_string();

    let url = format!("{}/api/v1/plugins", registry.trim_end_matches('/'));

    println!("  {} {}@{} to {}", "Uploading".cyan(), plugin_id, plugin_version, url);

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("application/zip")?;

    let mut form = reqwest::multipart::Form::new()
        .text("name", plugin_id.to_string())
        .text("version", plugin_version.to_string())
        .part("package", part);

    // Attach the attestation as two extra multipart fields. The server
    // treats them as optional and picks them up when present — older
    // registry builds that don't yet understand the fields ignore them.
    if let Some(att) = attestation {
        let sbom_part = reqwest::multipart::Part::bytes(att.sbom_canonical_bytes.clone())
            .file_name("sbom.json".to_string())
            .mime_str("application/json")?;
        form = form.part("sbom", sbom_part).text("sbom_signature", att.signature_b64.clone());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .with_context(|| format!("Failed to connect to registry at {}", url))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| "No response body".to_string());
        anyhow::bail!(
            "Registry returned {} when publishing {}@{}: {}",
            status,
            plugin_id,
            plugin_version,
            body
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use zip::write::{SimpleFileOptions, ZipWriter};
    use zip::CompressionMethod;

    fn create_test_package(dir: &Path, plugin_id: &str, version: &str) -> PathBuf {
        let package_path = dir.join(format!("{}.zip", plugin_id));
        let file = fs::File::create(&package_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        // Add manifest
        let manifest = format!("id: {}\nversion: {}\nname: Test Plugin", plugin_id, version);
        zip.start_file("plugin.yaml", options).unwrap();
        std::io::Write::write_all(&mut zip, manifest.as_bytes()).unwrap();

        // Add a fake wasm file
        zip.start_file("test_plugin.wasm", options).unwrap();
        std::io::Write::write_all(&mut zip, b"fake wasm content").unwrap();

        zip.finish().unwrap();
        package_path
    }

    fn create_test_project(dir: &Path, plugin_id: &str, version: &str) {
        let manifest = format!(
            "id: {}\nversion: {}\nname: Test Plugin\nplugin_type: auth",
            plugin_id, version
        );
        fs::write(dir.join("plugin.yaml"), manifest).unwrap();
    }

    #[test]
    fn test_validate_package_valid() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = create_test_package(temp_dir.path(), "test-plugin", "1.0.0");

        let result = validate_package(&package_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_package_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("nonexistent.zip");

        let result = validate_package(&package_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_validate_package_no_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("bad.zip");
        let file = fs::File::create(&package_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("test.wasm", options).unwrap();
        std::io::Write::write_all(&mut zip, b"fake wasm").unwrap();

        zip.finish().unwrap();

        let result = validate_package(&package_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing plugin.yaml"));
    }

    #[test]
    fn test_validate_package_no_wasm() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("bad.zip");
        let file = fs::File::create(&package_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("plugin.yaml", options).unwrap();
        std::io::Write::write_all(&mut zip, b"id: test\nversion: 1.0.0").unwrap();

        zip.finish().unwrap();

        let result = validate_package(&package_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing a .wasm"));
    }

    #[test]
    fn test_validate_package_not_a_zip() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("not-a-zip.zip");
        fs::write(&package_path, b"this is not a zip file").unwrap();

        let result = validate_package(&package_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid zip"));
    }

    #[test]
    fn test_read_manifest_from_zip_valid() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = create_test_package(temp_dir.path(), "my-plugin", "2.1.0");

        let result = read_manifest_from_zip(&package_path);
        assert!(result.is_ok());
        let (id, version) = result.unwrap();
        assert_eq!(id, "my-plugin");
        assert_eq!(version, "2.1.0");
    }

    #[test]
    fn test_read_manifest_from_zip_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("nonexistent.zip");

        let result = read_manifest_from_zip(&package_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_manifest_from_zip_no_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("no-manifest.zip");
        let file = fs::File::create(&package_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("other.txt", options).unwrap();
        std::io::Write::write_all(&mut zip, b"content").unwrap();

        zip.finish().unwrap();

        let result = read_manifest_from_zip(&package_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not contain"));
    }

    #[test]
    fn test_resolve_package_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "dir-plugin", "1.2.3");
        create_test_package(temp_dir.path(), "dir-plugin", "1.2.3");

        let result = resolve_package(temp_dir.path());
        assert!(result.is_ok());
        let (id, version, path) = result.unwrap();
        assert_eq!(id, "dir-plugin");
        assert_eq!(version, "1.2.3");
        assert!(path.exists());
    }

    #[test]
    fn test_resolve_package_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = create_test_package(temp_dir.path(), "file-plugin", "3.0.0");

        let result = resolve_package(&package_path);
        assert!(result.is_ok());
        let (id, version, path) = result.unwrap();
        assert_eq!(id, "file-plugin");
        assert_eq!(version, "3.0.0");
        assert_eq!(path, package_path);
    }

    #[test]
    fn test_resolve_package_directory_no_zip() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "no-zip-plugin", "1.0.0");

        let result = resolve_package(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Run 'mockforge-plugin package' first"));
    }

    #[test]
    fn test_resolve_package_directory_no_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let result = resolve_package(temp_dir.path());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_publish_plugin_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "dry-run-plugin", "1.0.0");
        create_test_package(temp_dir.path(), "dry-run-plugin", "1.0.0");

        let result = publish_plugin(
            Some(temp_dir.path()),
            "https://registry.mockforge.dev",
            None,
            true,
            SignOptions::default(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_plugin_no_token() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "no-token-plugin", "1.0.0");
        create_test_package(temp_dir.path(), "no-token-plugin", "1.0.0");

        let result = publish_plugin(
            Some(temp_dir.path()),
            "https://registry.mockforge.dev",
            None,
            false,
            SignOptions::default(),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication token is required"));
    }

    #[tokio::test]
    async fn test_publish_plugin_empty_token() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "empty-token-plugin", "1.0.0");
        create_test_package(temp_dir.path(), "empty-token-plugin", "1.0.0");

        let result = publish_plugin(
            Some(temp_dir.path()),
            "https://registry.mockforge.dev",
            Some(""),
            false,
            SignOptions::default(),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication token is required"));
    }

    #[tokio::test]
    async fn test_publish_plugin_sign_requires_both_args() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "half-sign-plugin", "1.0.0");
        let pkg = create_test_package(temp_dir.path(), "half-sign-plugin", "1.0.0");

        // --key-file without --sbom.
        let err = build_sbom_attestation(
            &pkg,
            &SignOptions {
                key_file: Some(PathBuf::from("k.pem")),
                sbom_path: None,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("--key-file supplied without --sbom"));

        // --sbom without --key-file.
        let err = build_sbom_attestation(
            &pkg,
            &SignOptions {
                key_file: None,
                sbom_path: Some(PathBuf::from("s.json")),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("--sbom supplied without --key-file"));

        // Neither flag → no attestation, no error.
        let none = build_sbom_attestation(&pkg, &SignOptions::default()).unwrap();
        assert!(none.is_none());
    }

    /// JCS makes the signature portable across JSON libraries: two
    /// SBOMs that differ only in key order / whitespace must produce
    /// the same canonical bytes and therefore the same signature.
    #[tokio::test]
    async fn test_sbom_canonicalization_is_jcs() {
        use crate::commands::key;
        let tmp = tempfile::tempdir().unwrap();

        // Two SBOMs with identical semantics but different formatting.
        let a = tmp.path().join("a.json");
        let b = tmp.path().join("b.json");
        std::fs::write(&a, br#"{"components":[{"name":"foo","version":"1.0"}]}"#).unwrap();
        // Same content, reordered keys, pretty-printed whitespace.
        std::fs::write(
            &b,
            br#"{
    "components": [
        {
            "version": "1.0",
            "name": "foo"
        }
    ]
}"#,
        )
        .unwrap();

        let bytes_a = key::read_and_canonicalize_sbom(&a).unwrap();
        let bytes_b = key::read_and_canonicalize_sbom(&b).unwrap();
        assert_eq!(bytes_a, bytes_b, "JCS must normalize both inputs to the same bytes");

        // Signatures over both inputs with the same key must also match,
        // proving the property flows through the signer.
        let key_path = tmp.path().join("k.pem");
        key::generate_key(&key_path, false).await.unwrap();
        let signing = key::load_signing_key(&key_path).unwrap();
        let checksum = "aa".repeat(32);
        let msg_a = key::attestation_message(&checksum, &bytes_a).unwrap();
        let msg_b = key::attestation_message(&checksum, &bytes_b).unwrap();
        assert_eq!(msg_a, msg_b);
        let sig_a = ed25519_dalek::Signer::sign(&signing, &msg_a);
        let sig_b = ed25519_dalek::Signer::sign(&signing, &msg_b);
        assert_eq!(sig_a.to_bytes(), sig_b.to_bytes());
    }

    #[tokio::test]
    async fn test_publish_plugin_sign_end_to_end() {
        use crate::commands::key;
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "signed-plugin", "1.0.0");
        let pkg = create_test_package(temp_dir.path(), "signed-plugin", "1.0.0");

        // Generate a keypair, write a tiny SBOM, build the attestation,
        // and confirm the signature round-trips back to the verifying
        // key that just signed it.
        let key_path = temp_dir.path().join("signer.pem");
        key::generate_key(&key_path, false).await.unwrap();

        let sbom_path = temp_dir.path().join("sbom.json");
        std::fs::write(&sbom_path, br#"{"components":[]}"#).unwrap();

        let att = build_sbom_attestation(
            &pkg,
            &SignOptions {
                key_file: Some(key_path.clone()),
                sbom_path: Some(sbom_path.clone()),
            },
        )
        .unwrap()
        .expect("signing asked for");

        // Reconstruct the verifier side and check the signature really
        // covers SHA-256(wasm_checksum || sbom_canonical).
        use base64::Engine;
        use sha2::{Digest, Sha256};
        let signing = key::load_signing_key(&key_path).unwrap();
        let checksum_bytes: [u8; 32] = Sha256::digest(std::fs::read(&pkg).unwrap()).into();
        let checksum_hex: String = checksum_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let msg = key::attestation_message(&checksum_hex, &att.sbom_canonical_bytes).unwrap();
        let sig_bytes =
            base64::engine::general_purpose::STANDARD.decode(&att.signature_b64).unwrap();
        let sig = ed25519_dalek::Signature::from_slice(&sig_bytes).unwrap();
        ed25519_dalek::Verifier::verify(&signing.verifying_key(), &msg, &sig).unwrap();
    }

    #[tokio::test]
    async fn test_publish_plugin_no_package() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path(), "no-package", "1.0.0");

        let result = publish_plugin(
            Some(temp_dir.path()),
            "https://registry.mockforge.dev",
            Some("test-token"),
            false,
            SignOptions::default(),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Run 'mockforge-plugin package' first"));
    }

    #[tokio::test]
    async fn test_publish_plugin_dry_run_from_zip() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = create_test_package(temp_dir.path(), "zip-plugin", "2.0.0");

        let result = publish_plugin(
            Some(&package_path),
            "https://registry.mockforge.dev",
            None,
            true,
            SignOptions::default(),
        )
        .await;
        assert!(result.is_ok());
    }
}
