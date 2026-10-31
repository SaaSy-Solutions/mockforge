//! Atomically publish complete immutable snapshots without replacing a winner.
use super::{
    fs, loading, validate_artifact_type, ArtifactFreezer, FreezeRequest, FrozenArtifact, Result,
};

impl ArtifactFreezer {
    pub(crate) async fn load_snapshot(
        &self,
        artifact_type: &str,
        identifier: &str,
        format: &str,
    ) -> Result<Option<FrozenArtifact>> {
        let path = self.snapshot_path(artifact_type, identifier, format)?;
        match fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => {
                return Err(mockforge_foundation::Error::config(
                    "Frozen snapshot must be a regular file",
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(mockforge_foundation::Error::io_with_context(
                    "read frozen snapshot",
                    error.to_string(),
                ))
            }
        }
        let frozen =
            loading::read(&path, artifact_type, Some(identifier)).await?.ok_or_else(|| {
                mockforge_foundation::Error::config("Frozen snapshot identity mismatch")
            })?;
        if !self.verify_frozen_artifact(&frozen).await? {
            return Err(mockforge_foundation::Error::config(
                "Frozen snapshot integrity check failed",
            ));
        }
        Ok(Some(frozen))
    }

    fn snapshot_path(
        &self,
        artifact_type: &str,
        identifier: &str,
        format: &str,
    ) -> Result<std::path::PathBuf> {
        validate_artifact_type(artifact_type)?;
        if identifier.len() != 64 || !identifier.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(mockforge_foundation::Error::config("Invalid frozen snapshot identity"));
        }
        let extension = match format {
            "json" => "json",
            "yaml" | "yml" => "yaml",
            _ => {
                return Err(mockforge_foundation::Error::config(
                    "Frozen artifact format must be json or yaml",
                ))
            }
        };
        Ok(self.base_dir.join(format!("{artifact_type}_{identifier}.{extension}")))
    }

    /// Publish one immutable snapshot for an exact generation identity.
    /// A complete temporary file is linked into place without overwriting a
    /// winner from another process. All callers return that same snapshot.
    pub(crate) async fn freeze_once(
        &self,
        request: &FreezeRequest,
        identifier: &str,
    ) -> Result<FrozenArtifact> {
        let path = self.snapshot_path(&request.artifact_type, identifier, &request.format)?;
        let temporary = self.base_dir.join(format!(".freeze-{}.tmp", uuid::Uuid::new_v4()));
        let mut candidate = request.clone();
        candidate.path = Some(temporary.to_string_lossy().into_owned());
        self.freeze(&candidate).await?;
        let published = fs::hard_link(&temporary, &path).await;
        let removed = fs::remove_file(&temporary).await;
        removed.map_err(|error| {
            mockforge_foundation::Error::io_with_context(
                "remove frozen snapshot temporary file",
                error.to_string(),
            )
        })?;
        if let Err(error) = published {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(mockforge_foundation::Error::io_with_context(
                    "publish frozen snapshot",
                    error.to_string(),
                ));
            }
        }
        self.load_snapshot(&request.artifact_type, identifier, &request.format)
            .await?
            .ok_or_else(|| {
                mockforge_foundation::Error::config("Published frozen snapshot disappeared")
            })
    }
}
