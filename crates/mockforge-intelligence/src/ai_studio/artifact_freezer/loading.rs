//! Decode saved artifacts and match their recorded identity, not a filename substring.
use super::{FreezeMetadata, FrozenArtifact};
use mockforge_foundation::{Error, Result};
use serde_json::Value;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;

pub(super) async fn read(
    path: &Path,
    artifact_type: &str,
    identifier: Option<&str>,
) -> Result<Option<FrozenArtifact>> {
    let format = match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => "json",
        Some("yaml" | "yml") => "yaml",
        _ => return Ok(None),
    };
    let file = File::open(path)
        .await
        .map_err(|error| Error::io_with_context("open frozen artifact", error.to_string()))?;
    let mut bytes = Vec::new();
    file.take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| Error::io_with_context("read frozen artifact", error.to_string()))?;
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(Error::config("Frozen artifact exceeds the 16 MiB read limit"));
    }
    let content: Value = if format == "yaml" {
        serde_yaml::from_slice(&bytes)
            .map_err(|error| Error::config(format!("Invalid frozen YAML: {error}")))?
    } else {
        serde_json::from_slice(&bytes)?
    };
    let provenance = content.get("_frozen_metadata");
    if provenance
        .and_then(|metadata| metadata.get("artifact_type"))
        .and_then(Value::as_str)
        .is_some_and(|recorded| recorded != artifact_type)
    {
        return Ok(None);
    }
    let metadata: Option<FreezeMetadata> =
        provenance.cloned().map(serde_json::from_value).transpose()?;
    if identifier.is_some_and(|expected| {
        metadata.as_ref().and_then(|metadata| metadata.prompt_hash.as_deref()) != Some(expected)
    }) {
        return Ok(None);
    }
    let output_hash = provenance
        .and_then(|metadata| metadata.get("output_hash"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    if identifier.is_some() && output_hash.is_none() {
        return Err(Error::config("Frozen snapshot is missing its payload integrity hash"));
    }
    Ok(Some(FrozenArtifact {
        artifact_type: artifact_type.into(),
        content,
        format: format.into(),
        path: path.to_string_lossy().into_owned(),
        metadata,
        output_hash,
    }))
}
