use super::*;
use tokio::io::AsyncWriteExt;

impl QdrantStorage {
    pub(super) async fn download_snapshot(&self, path: &str) -> Result<()> {
        self.initialize().await?;
        let snapshot =
            Self::result(self.request(Method::POST, &["snapshots"]).query(&[("wait", true)]))
                .await?;
        let name = snapshot["name"]
            .as_str()
            .ok_or_else(|| Error::generic("Qdrant snapshot name missing"))?;
        let destination = std::path::Path::new(path);
        let temporary = destination.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = async {
            let expected_size = snapshot["size"]
                .as_u64()
                .ok_or_else(|| Error::generic("Qdrant snapshot size missing"))?;
            let mut size = 0_u64;
            let mut checksum = Sha256::new();
            let mut response = Self::send(self.request(Method::GET, &["snapshots", name])).await?;
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await?;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| Error::generic("Qdrant snapshot download interrupted"))?
            {
                size = size.saturating_add(chunk.len() as u64);
                checksum.update(&chunk);
                file.write_all(&chunk).await?;
            }
            if size != expected_size {
                return Err(Error::generic("Qdrant snapshot download size mismatch"));
            }
            if let Some(expected) = snapshot["checksum"].as_str() {
                if hex::encode(checksum.finalize()) != expected {
                    return Err(Error::generic("Qdrant snapshot checksum mismatch"));
                }
            }
            file.sync_all().await?;
            drop(file);
            tokio::fs::rename(&temporary, destination).await?;
            Ok::<_, Error>(())
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&temporary).await;
        }
        if Self::send(self.request(Method::DELETE, &["snapshots", name]).query(&[("wait", true)]))
            .await
            .is_err()
        {
            tracing::warn!("Could not remove temporary Qdrant server snapshot after backup");
        }
        result
    }

    pub(super) async fn upload_snapshot(&self, path: &str) -> Result<()> {
        self.initialize().await?;
        let form = reqwest::multipart::Form::new().file("snapshot", path).await?;
        Self::result(
            self.request(Method::POST, &["snapshots", "upload"])
                .query(&[("wait", "true"), ("priority", "snapshot")])
                .multipart(form),
        )
        .await?;
        let info = Self::result(self.request(Method::GET, &[])).await?;
        let vectors = &info["config"]["params"]["vectors"];
        if vectors["size"].as_u64() != Some(self.dimensions as u64)
            || vectors["distance"].as_str() != Some(self.distance.as_str())
        {
            return Err(Error::config(
                "Restored Qdrant snapshot has incompatible vector configuration",
            ));
        }
        Ok(())
    }
}
