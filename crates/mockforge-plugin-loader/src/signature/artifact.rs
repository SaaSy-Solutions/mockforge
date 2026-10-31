//! Canonical signed artifact payload and safe file traversal.
use super::{ARTIFACT_PAYLOAD_DOMAIN, SIGNATURE_FILE};
use crate::{LoaderResult, PluginLoaderError};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Build the canonical bytes a v2 plugin signature covers.
///
/// Every regular file under `plugin_dir` is included except the
/// top-level `plugin.sig`, so the manifest, the WASM module and any
/// bundled assets are all bound to the signature. Files are ordered
/// by their `/`-separated relative path. The payload is:
///
/// ```text
/// "mockforge-plugin-signature-v2\0" || u64be(file_count)
///   || for each file: u64be(len(path)) || path || u64be(len(content)) || sha256(content)
/// ```
///
/// Length prefixes keep path/content boundaries unambiguous. Symlinks
/// and non-UTF-8 paths are rejected rather than skipped, so nothing in
/// the directory escapes the signature.
pub fn compute_artifact_payload(plugin_dir: &Path) -> LoaderResult<Vec<u8>> {
    if !plugin_dir.is_dir() {
        return Err(PluginLoaderError::security(format!(
            "Plugin path is not a directory: {}",
            plugin_dir.display()
        )));
    }

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_files(plugin_dir, plugin_dir, &mut files)?;
    files.retain(|(rel, _)| rel != SIGNATURE_FILE);
    if files.is_empty() {
        return Err(PluginLoaderError::security("Plugin directory contains no files to verify"));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut payload = Vec::with_capacity(ARTIFACT_PAYLOAD_DOMAIN.len() + files.len() * 96);
    payload.extend_from_slice(ARTIFACT_PAYLOAD_DOMAIN);
    payload.extend_from_slice(&(files.len() as u64).to_be_bytes());
    for (rel, path) in &files {
        let content = fs::read(path).map_err(|e| {
            PluginLoaderError::security(format!("Failed to read {}: {}", path.display(), e))
        })?;
        let digest = ring::digest::digest(&ring::digest::SHA256, &content);
        payload.extend_from_slice(&(rel.len() as u64).to_be_bytes());
        payload.extend_from_slice(rel.as_bytes());
        payload.extend_from_slice(&(content.len() as u64).to_be_bytes());
        payload.extend_from_slice(digest.as_ref());
    }
    Ok(payload)
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> LoaderResult<()> {
    let entries = fs::read_dir(dir).map_err(|e| {
        PluginLoaderError::security(format!("Cannot read {}: {}", dir.display(), e))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| {
            PluginLoaderError::security(format!("Cannot read {}: {}", dir.display(), e))
        })?;
        let path = entry.path();
        let file_type = fs::symlink_metadata(&path)
            .map_err(|e| {
                PluginLoaderError::security(format!("Cannot stat {}: {}", path.display(), e))
            })?
            .file_type();
        if file_type.is_symlink() {
            return Err(PluginLoaderError::security(format!(
                "Signed plugins may not contain symlinks: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            collect_files(root, &path, out)?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| PluginLoaderError::security("Plugin file outside plugin directory"))?;
            let mut parts = Vec::new();
            for component in rel.components() {
                let part = component.as_os_str().to_str().ok_or_else(|| {
                    PluginLoaderError::security(format!(
                        "Plugin file name is not valid UTF-8: {}",
                        path.display()
                    ))
                })?;
                parts.push(part);
            }
            out.push((parts.join("/"), path));
        } else {
            return Err(PluginLoaderError::security(format!(
                "Unsupported file type in plugin directory: {}",
                path.display()
            )));
        }
    }
    Ok(())
}
