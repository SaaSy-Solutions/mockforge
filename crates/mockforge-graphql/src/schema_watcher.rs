//! Schema hot-reloading with file watching
//!
//! Watches GraphQL schema files for changes and automatically reloads them.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Schema file watcher that monitors for changes
pub struct SchemaWatcher {
    /// Path to the schema file
    schema_path: PathBuf,
    /// Current schema SDL
    schema_sdl: Arc<RwLock<String>>,
    /// File watcher
    _watcher: Option<RecommendedWatcher>,
}

impl SchemaWatcher {
    /// Create a new schema watcher
    pub async fn new(
        schema_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Load initial schema
        let initial_sdl = tokio::fs::read_to_string(&schema_path).await?;
        let schema_sdl = Arc::new(RwLock::new(initial_sdl));

        Ok(Self {
            schema_path,
            schema_sdl,
            _watcher: None,
        })
    }

    /// Start watching the schema file for changes
    pub fn start_watching(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let schema_path = self.schema_path.clone();
        let schema_sdl = Arc::clone(&self.schema_sdl);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        info!("Schema file changed, reloading...");
                        let path = schema_path.clone();
                        let sdl = Arc::clone(&schema_sdl);

                        // Spawn async task to reload schema
                        tokio::spawn(async move {
                            match tokio::fs::read_to_string(&path).await {
                                Ok(new_sdl) => {
                                    let mut sdl_lock = sdl.write().await;
                                    *sdl_lock = new_sdl;
                                    info!("✓ Schema reloaded successfully");
                                }
                                Err(e) => {
                                    error!("Failed to reload schema: {}", e);
                                }
                            }
                        });
                    }
                }
                Err(e) => warn!("Watch error: {:?}", e),
            }
        })?;

        watcher.watch(&self.schema_path, RecursiveMode::NonRecursive)?;
        self._watcher = Some(watcher);

        info!("👀 Watching schema file: {:?}", self.schema_path);
        Ok(())
    }

    /// Get the current schema SDL
    pub async fn get_schema(&self) -> String {
        self.schema_sdl.read().await.clone()
    }

    /// Manually reload the schema
    pub async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let new_sdl = tokio::fs::read_to_string(&self.schema_path).await?;
        let mut sdl_lock = self.schema_sdl.write().await;
        *sdl_lock = new_sdl;
        info!("Schema manually reloaded");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_schema_watcher_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "type Query {{ hello: String }}").unwrap();

        let watcher = SchemaWatcher::new(temp_file.path().to_path_buf()).await;
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_get_schema() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let schema_content = "type Query { hello: String }";
        writeln!(temp_file, "{}", schema_content).unwrap();

        let watcher = SchemaWatcher::new(temp_file.path().to_path_buf()).await.unwrap();
        let sdl = watcher.get_schema().await;
        assert!(sdl.contains("type Query"));
    }

    #[tokio::test]
    async fn test_manual_reload() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "type Query {{ hello: String }}").unwrap();

        let watcher = SchemaWatcher::new(temp_file.path().to_path_buf()).await.unwrap();

        // Modify the file
        writeln!(temp_file, "type Query {{ world: String }}").unwrap();

        // Manually reload
        let result = watcher.reload().await;
        assert!(result.is_ok());

        let sdl = watcher.get_schema().await;
        assert!(sdl.contains("world"));
    }
}
