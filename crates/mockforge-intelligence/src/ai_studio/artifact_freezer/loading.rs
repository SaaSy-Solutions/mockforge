//! Decode saved artifacts and match their recorded identity, not a filename substring.
use super::{provenance, FreezeMetadata, FrozenArtifact};
use mockforge_foundation::{Error, Result};
use serde_json::Value;
use std::path::Path;

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
    let bytes = provenance::read_regular(path, MAX_ARTIFACT_BYTES).await?;
    let content: Value = if format == "yaml" {
        serde_yaml::from_slice(&bytes)
            .map_err(|error| Error::config(format!("Invalid frozen YAML: {error}")))?
    } else {
        serde_json::from_slice(&bytes)?
    };
    let stored_provenance;
    let provenance = if content.is_object() {
        content.get("_frozen_metadata")
    } else {
        stored_provenance = provenance::read(path).await?;
        if stored_provenance.get("artifact_type").and_then(Value::as_str).is_none() {
            return Err(Error::config("Frozen provenance is missing its artifact type"));
        }
        Some(&stored_provenance)
    };
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
    if output_hash.is_none() {
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
