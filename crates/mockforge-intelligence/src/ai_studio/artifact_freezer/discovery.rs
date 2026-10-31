//! Discover exact artifact identities, preserving deterministic selection and integrity.
use super::{
    bundles, fs, loading, validate_artifact_type, ArtifactFreezer, FrozenArtifact, Result,
};
use chrono::Utc;
use serde_json::Value;
use std::path::PathBuf;

impl ArtifactFreezer {
    /// Load a frozen artifact by type and identifier
    ///
    /// In deterministic mode, this method searches for frozen artifacts matching
    /// the given type and identifier (e.g., description hash, persona ID).
    pub async fn load_frozen(
        &self,
        artifact_type: &str,
        identifier: Option<&str>,
    ) -> Result<Option<FrozenArtifact>> {
        validate_artifact_type(artifact_type)?;
        let mut entries = match fs::read_dir(&self.base_dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(mockforge_foundation::Error::io_with_context(
                    "read frozen artifacts directory",
                    error.to_string(),
                ))
            }
        };
        let prefix = format!("{artifact_type}_");
        let mut latest_match: Option<FrozenArtifact> = None;
        let mut latest_key = (chrono::DateTime::<Utc>::MIN_UTC, PathBuf::new());
        while let Some(entry) = entries.next_entry().await.map_err(|error| {
            mockforge_foundation::Error::io_with_context("read directory entry", error.to_string())
        })? {
            if !entry.file_name().to_string_lossy().starts_with(&prefix) {
                continue;
            }
            let file_type = entry.file_type().await.map_err(|error| {
                mockforge_foundation::Error::io_with_context(
                    "read frozen file type",
                    error.to_string(),
                )
            })?;
            let path = if file_type.is_file() {
                entry.path()
            } else if file_type.is_dir() {
                let Some(path) = bundles::payload(&entry.path()) else {
                    continue;
                };
                path
            } else {
                continue;
            };
            let Some(frozen) = loading::read(&path, artifact_type, identifier).await? else {
                continue;
            };
            if !self.verify_frozen_artifact(&frozen).await? {
                return Err(mockforge_foundation::Error::config(format!(
                    "Frozen artifact integrity check failed: {}",
                    path.display()
                )));
            }
            let sidecar;
            let provenance = if frozen.content.is_object() {
                frozen.content.get("_frozen_metadata")
            } else {
                sidecar = super::provenance::read(&path).await?;
                Some(&sidecar)
            };
            let frozen_time = provenance
                .and_then(|metadata| metadata.get("frozen_at"))
                .and_then(Value::as_str)
                .and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
                .map(|time| time.with_timezone(&Utc))
                .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC);
            let key = (frozen_time, path);
            if latest_match.is_none() || key > latest_key {
                latest_key = key;
                latest_match = Some(frozen);
            }
        }

        Ok(latest_match)
    }
}
