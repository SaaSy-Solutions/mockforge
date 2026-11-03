//! Publish a snapshot and its provenance in one directory rename.
use super::{fs, ArtifactFreezer, FreezeRequest, FrozenArtifact, Result};
use mockforge_foundation::Error;
use std::path::{Path, PathBuf};

pub(super) fn path(snapshot: &Path) -> PathBuf {
    let mut name = snapshot.file_name().unwrap_or_default().to_os_string();
    name.push(".bundle");
    snapshot.with_file_name(name)
}

pub(super) fn payload(directory: &Path) -> Option<PathBuf> {
    let name = directory.file_name()?.to_str()?;
    let extension = if name.ends_with(".json.bundle") {
        "json"
    } else if name.ends_with(".yaml.bundle") {
        "yaml"
    } else {
        return None;
    };
    Some(directory.join(format!("payload.{extension}")))
}

pub(super) async fn publish(
    freezer: &ArtifactFreezer,
    request: &FreezeRequest,
    identifier: &str,
    snapshot: &Path,
) -> Result<FrozenArtifact> {
    fs::create_dir_all(freezer.base_dir())
        .await
        .map_err(|e| Error::io_with_context("create snapshot directory", e.to_string()))?;
    let destination = path(snapshot);
    let temporary = freezer.base_dir().join(format!(".freeze-{}.tmp", uuid::Uuid::new_v4()));
    fs::create_dir(&temporary)
        .await
        .map_err(|e| Error::io_with_context("create snapshot staging directory", e.to_string()))?;
    let mut candidate = request.clone();
    let extension = snapshot.extension().and_then(|s| s.to_str()).unwrap_or("json");
    candidate.path =
        Some(temporary.join(format!("payload.{extension}")).to_string_lossy().into_owned());
    let result = async {
        freezer.freeze(&candidate).await?;
        match fs::rename(&temporary, &destination).await {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(Error::io_with_context(
                    "publish frozen snapshot bundle",
                    error.to_string(),
                ))
            }
        }
        freezer
            .load_snapshot(&request.artifact_type, identifier, &request.format)
            .await?
            .ok_or_else(|| Error::config("Published snapshot bundle disappeared"))
    }
    .await;
    if fs::try_exists(&temporary).await.unwrap_or(false) {
        fs::remove_dir_all(&temporary).await.map_err(|e| {
            Error::io_with_context("remove snapshot staging directory", e.to_string())
        })?;
    }
    result
}
