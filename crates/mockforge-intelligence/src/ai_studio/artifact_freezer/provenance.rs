//! Keep non-object payloads unchanged while retaining identity and integrity beside them.
use super::{fs, Result};
use mockforge_foundation::Error;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

pub(super) fn path(payload: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(payload.file_name().unwrap_or_default());
    name.push(".frozen-metadata");
    payload.with_file_name(name)
}

pub(super) async fn write(payload: &Path, metadata: &Value) -> Result<()> {
    let destination = path(payload);
    let temporary = payload.with_file_name(format!(".metadata-{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(metadata)?;
    if bytes.len() > 64 * 1024 {
        return Err(Error::config("Frozen provenance exceeds the 64 KiB limit"));
    }
    fs::write(&temporary, bytes)
        .await
        .map_err(|e| Error::io_with_context("write frozen provenance", e.to_string()))?;
    let result = fs::rename(&temporary, destination).await;
    if result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    result.map_err(|e| Error::io_with_context("publish frozen provenance", e.to_string()))
}

pub(super) async fn read(payload: &Path) -> Result<Value> {
    let bytes = read_regular(&path(payload), 64 * 1024).await?;
    let metadata: Value = serde_json::from_slice(&bytes)?;
    if !metadata.is_object() {
        return Err(Error::config("Frozen provenance must be an object"));
    }
    Ok(metadata)
}

pub(super) async fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .await
        .map_err(|e| Error::io_with_context("inspect frozen file", e.to_string()))?;
    if !metadata.file_type().is_file() {
        return Err(Error::config("Frozen payload and provenance must be regular files"));
    }
    let file = fs::File::open(path)
        .await
        .map_err(|e| Error::io_with_context("open frozen file", e.to_string()))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| Error::io_with_context("read frozen file", e.to_string()))?;
    if bytes.len() as u64 > limit {
        return Err(Error::config("Frozen file exceeds its read limit"));
    }
    Ok(bytes)
}
