//! Discover exact artifact identities, preserving deterministic selection and integrity.
use super::{fs, loading, validate_artifact_type, ArtifactFreezer, FrozenArtifact, Result};
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
            let path = entry.path();
            if !entry
                .file_type()
                .await
                .map_err(|error| {
                    mockforge_foundation::Error::io_with_context(
                        "read frozen file type",
                        error.to_string(),
                    )
                })?
                .is_file()
                || !entry.file_name().to_string_lossy().starts_with(&prefix)
            {
                continue;
            }
            let Some(frozen) = loading::read(&path, artifact_type, identifier).await? else {
                continue;
            };
            if !self.verify_frozen_artifact(&frozen).await? {
                return Err(mockforge_foundation::Error::config(format!(
                    "Frozen artifact integrity check failed: {}",
                    path.display()
                )));
            }
            let frozen_time = frozen
                .content
                .get("_frozen_metadata")
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
