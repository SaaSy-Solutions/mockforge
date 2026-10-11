//! `GitOps` workflow support for chaos orchestrations
//!
//! Provides integration with `GitOps` tools like Flux and `ArgoCD` for
//! managing chaos orchestrations declaratively.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, path::Path};
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ManifestChange {
    Create(String),
    Update(String),
}

/// `GitOps` repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfig {
    /// Repository URL
    pub repo_url: String,
    /// Branch to watch
    pub branch: String,
    /// Path within repository
    pub path: PathBuf,
    /// Sync interval in seconds
    pub sync_interval_seconds: u64,
    /// Authentication
    pub auth: GitOpsAuth,
    /// Auto-sync enabled
    pub auto_sync: bool,
    /// Prune on sync (delete removed orchestrations)
    pub prune: bool,
}

/// Authentication for Git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GitOpsAuth {
    #[serde(rename = "ssh")]
    /// SSH authentication configuration.
    SSH {
        /// Filesystem path to the SSH private key.
        private_key_path: PathBuf,
    },
    #[serde(rename = "token")]
    /// Token authentication configuration.
    Token {
        /// Access token intended for the Git repository.
        token: String,
    },
    #[serde(rename = "basic")]
    /// Username and password authentication configuration.
    Basic {
        /// Repository login name.
        username: String,
        /// Repository login password.
        password: String,
    },
}

/// `GitOps` sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// UTC start time of the most recent synchronization.
    pub last_sync: DateTime<Utc>,
    /// SHA-256 digest of discovered manifest paths and hashes, rather than a Git commit identifier.
    pub commit_hash: String,
    /// Outcome of the synchronization attempt.
    pub status: SyncState,
    /// Number of newly discovered or changed manifests applied during synchronization.
    pub orchestrations_synced: usize,
    /// Failures encountered while applying or pruning manifests.
    pub errors: Vec<String>,
}

/// Sync state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    /// Manifest discovery and application succeeded.
    Synced,
    /// Desired and observed manifest state differ.
    OutOfSync,
    /// Synchronization is in progress.
    Syncing,
    /// Synchronization encountered an error.
    Failed,
}

/// Orchestration manifest in `GitOps` repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationManifest {
    /// Filesystem path to the orchestration manifest.
    pub file_path: PathBuf,
    /// Parsed manifest configuration.
    pub content: serde_json::Value,
    /// SHA-256 hash of the raw manifest file contents.
    pub hash: String,
    /// File modification time in UTC, falling back to the discovery time when unavailable.
    pub last_modified: DateTime<Utc>,
}

/// `GitOps` manager.
///
/// Reads manifests from the configured local path. Repository URL and authentication
/// settings are retained as configuration; this manager does not fetch remote Git content.
pub struct GitOpsManager {
    config: GitOpsConfig,
    manifests: HashMap<String, OrchestrationManifest>,
    last_sync_status: Option<SyncStatus>,
}

impl GitOpsManager {
    /// Create a new `GitOps` manager
    pub fn new(config: GitOpsConfig) -> Self {
        Self {
            config,
            manifests: HashMap::new(),
            last_sync_status: None,
        }
    }

    /// Sync orchestrations from Git repository
    pub async fn sync(&mut self) -> Result<SyncStatus, String> {
        let start_time = Utc::now();

        let manifests = self.discover_manifests().await?;
        let changes = self.calculate_changes(&manifests)?;

        let mut errors = Vec::new();
        if let Err(e) = self.apply_changes(&manifests, &changes).await {
            errors.push(format!("Failed to apply changes: {}", e));
        }

        // Cleanup (prune) if enabled
        if self.config.prune {
            if let Err(e) = self.prune_removed_orchestrations(&manifests).await {
                errors.push(format!("Failed to prune: {}", e));
            }
        }

        let commit_hash = self.compute_state_hash(&manifests);
        let status = SyncStatus {
            last_sync: start_time,
            commit_hash,
            status: if errors.is_empty() {
                SyncState::Synced
            } else {
                SyncState::Failed
            },
            orchestrations_synced: changes.len(),
            errors,
        };

        self.last_sync_status = Some(status.clone());
        Ok(status)
    }

    /// Discover orchestration manifests in repository
    async fn discover_manifests(&self) -> Result<Vec<OrchestrationManifest>, String> {
        let mut files = Vec::new();
        collect_manifest_files(&self.config.path, &mut files)?;

        let mut manifests = Vec::new();
        for file_path in files {
            let content_raw = fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read manifest {}: {}", file_path.display(), e))?;
            let content = parse_manifest_content(&file_path, &content_raw)?;
            let hash = hash_content(&content_raw);
            let last_modified = fs::metadata(&file_path)
                .and_then(|m| m.modified())
                .map_or_else(|_| Utc::now(), DateTime::<Utc>::from);

            manifests.push(OrchestrationManifest {
                file_path,
                content,
                hash,
                last_modified,
            });
        }

        manifests.sort_by_key(|m| m.file_path.clone());
        Ok(manifests)
    }

    /// Calculate changes between current and desired state
    fn calculate_changes(
        &self,
        manifests: &[OrchestrationManifest],
    ) -> Result<Vec<ManifestChange>, String> {
        let mut changes = Vec::new();
        for manifest in manifests {
            let key = manifest.file_path.to_string_lossy().to_string();
            match self.manifests.get(&key) {
                None => changes.push(ManifestChange::Create(key)),
                Some(existing) if existing.hash != manifest.hash => {
                    changes.push(ManifestChange::Update(key));
                }
                _ => {}
            }
        }
        Ok(changes)
    }

    async fn apply_changes(
        &mut self,
        manifests: &[OrchestrationManifest],
        changes: &[ManifestChange],
    ) -> Result<(), String> {
        let by_path: HashMap<String, OrchestrationManifest> = manifests
            .iter()
            .map(|m| (m.file_path.to_string_lossy().to_string(), m.clone()))
            .collect();

        for change in changes {
            match change {
                ManifestChange::Create(path) | ManifestChange::Update(path) => {
                    let manifest = by_path
                        .get(path)
                        .ok_or_else(|| format!("Manifest not found for change: {}", path))?;
                    self.manifests.insert(path.clone(), manifest.clone());
                }
            }
        }

        Ok(())
    }

    fn compute_state_hash(&self, manifests: &[OrchestrationManifest]) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        for manifest in manifests {
            hasher.update(manifest.file_path.to_string_lossy().as_bytes());
            hasher.update(manifest.hash.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Prune orchestrations that are no longer in Git
    async fn prune_removed_orchestrations(
        &mut self,
        current_manifests: &[OrchestrationManifest],
    ) -> Result<(), String> {
        let current_names: Vec<String> = current_manifests
            .iter()
            .map(|m| m.file_path.to_string_lossy().to_string())
            .collect();

        self.manifests.retain(|name, _| current_names.contains(name));

        Ok(())
    }

    /// Get current sync status
    pub fn get_status(&self) -> Option<&SyncStatus> {
        self.last_sync_status.as_ref()
    }

    /// Check if auto-sync is enabled
    pub fn is_auto_sync_enabled(&self) -> bool {
        self.config.auto_sync
    }

    /// Get sync interval
    pub fn get_sync_interval(&self) -> u64 {
        self.config.sync_interval_seconds
    }

    /// Start auto-sync loop
    pub async fn start_auto_sync(&mut self) -> Result<(), String> {
        if !self.config.auto_sync {
            return Err("Auto-sync is not enabled".to_string());
        }

        loop {
            match self.sync().await {
                Ok(status) => {
                    info!("Sync completed: {:?}", status.status);
                }
                Err(e) => {
                    error!("Sync failed: {}", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(self.config.sync_interval_seconds))
                .await;
        }
    }
}

fn collect_manifest_files(base: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !base.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(base)
        .map_err(|e| format!("Failed to read directory {}: {}", base.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_manifest_files(&path, out)?;
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if matches!(ext, "yaml" | "yml" | "json") {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn parse_manifest_content(path: &Path, content: &str) -> Result<serde_json::Value, String> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("yaml" | "yml") => {
            serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML manifest: {}", e))
        }
        Some("json") => {
            serde_json::from_str(content).map_err(|e| format!("Invalid JSON manifest: {}", e))
        }
        _ => Err(format!("Unsupported manifest file extension: {}", path.display())),
    }
}

fn hash_content(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Flux integration
pub mod flux {
    use super::*;

    /// Flux Kustomization configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FluxKustomization {
        /// Kubernetes API group and version for the resource.
        pub api_version: String,
        /// Kubernetes resource kind.
        pub kind: String,
        /// Resource name and namespace.
        pub metadata: FluxMetadata,
        /// Controller-specific desired state.
        pub spec: FluxSpec,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Kubernetes metadata for a Flux Kustomization.
    pub struct FluxMetadata {
        /// Kubernetes resource name.
        pub name: String,
        /// Kubernetes namespace for the resource or deployment.
        pub namespace: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Desired reconciliation state for a Flux Kustomization.
    pub struct FluxSpec {
        /// Reconciliation interval as a duration string, such as `5m`.
        pub interval: String,
        /// Manifest directory within the source repository.
        pub path: String,
        /// Whether resources removed from the source should be deleted.
        pub prune: bool,
        /// Reference to the Flux source resource.
        pub source_ref: SourceRef,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    /// Reference to a Flux source resource.
    pub struct SourceRef {
        /// Kubernetes resource kind.
        pub kind: String,
        /// Kubernetes resource name.
        pub name: String,
    }

    impl FluxKustomization {
        /// Create a new Flux Kustomization for `MockForge` orchestrations
        pub fn new_for_orchestrations(
            name: String,
            namespace: String,
            git_repo: String,
            path: String,
        ) -> Self {
            Self {
                api_version: "kustomize.toolkit.fluxcd.io/v1".to_string(),
                kind: "Kustomization".to_string(),
                metadata: FluxMetadata { name, namespace },
                spec: FluxSpec {
                    interval: "5m".to_string(),
                    path,
                    prune: true,
                    source_ref: SourceRef {
                        kind: "GitRepository".to_string(),
                        name: git_repo,
                    },
                },
            }
        }

        /// Convert to YAML
        pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
            serde_yaml::to_string(self)
        }
    }
}

/// `ArgoCD` integration
pub mod argocd {
    use super::*;

    /// `ArgoCD` Application configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ArgoApplication {
        /// Kubernetes API group and version for the resource.
        pub api_version: String,
        /// Kubernetes resource kind.
        pub kind: String,
        /// Resource name and namespace.
        pub metadata: ArgoMetadata,
        /// Controller-specific desired state.
        pub spec: ArgoSpec,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Kubernetes metadata for an Argo CD Application.
    pub struct ArgoMetadata {
        /// Kubernetes resource name.
        pub name: String,
        /// Kubernetes namespace for the resource or deployment.
        pub namespace: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Desired state for an Argo CD Application.
    pub struct ArgoSpec {
        /// Argo CD project that owns the application.
        pub project: String,
        /// Repository and manifest location used by the application.
        pub source: ArgoSource,
        /// Kubernetes cluster and namespace receiving the application.
        pub destination: ArgoDestination,
        /// Optional automatic synchronization settings.
        pub sync_policy: Option<SyncPolicy>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    /// Git repository source for an Argo CD Application.
    pub struct ArgoSource {
        /// URL of the Git repository containing manifests.
        pub repo_url: String,
        /// Git revision to reconcile, such as `HEAD`.
        pub target_revision: String,
        /// Manifest directory within the source repository.
        pub path: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Target Kubernetes cluster and namespace for an Argo CD Application.
    pub struct ArgoDestination {
        /// Kubernetes API server address.
        pub server: String,
        /// Kubernetes namespace for the resource or deployment.
        pub namespace: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    /// Argo CD synchronization policy and options.
    pub struct SyncPolicy {
        /// Optional automatic synchronization policy.
        pub automated: Option<AutomatedSync>,
        /// Additional Argo CD synchronization options.
        pub sync_options: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    /// Automatic pruning and drift correction settings for Argo CD.
    pub struct AutomatedSync {
        /// Whether resources removed from the source should be deleted.
        pub prune: bool,
        #[serde(rename = "selfHeal")]
        /// Whether Argo CD automatically corrects drift from the desired state.
        pub self_heal: bool,
    }

    impl ArgoApplication {
        /// Create a new `ArgoCD` Application for `MockForge` orchestrations
        pub fn new_for_orchestrations(
            name: String,
            namespace: String,
            repo_url: String,
            path: String,
            auto_sync: bool,
        ) -> Self {
            Self {
                api_version: "argoproj.io/v1alpha1".to_string(),
                kind: "Application".to_string(),
                metadata: ArgoMetadata {
                    name,
                    namespace: namespace.clone(),
                },
                spec: ArgoSpec {
                    project: "default".to_string(),
                    source: ArgoSource {
                        repo_url,
                        target_revision: "HEAD".to_string(),
                        path,
                    },
                    destination: ArgoDestination {
                        server: "https://kubernetes.default.svc".to_string(),
                        namespace,
                    },
                    sync_policy: if auto_sync {
                        Some(SyncPolicy {
                            automated: Some(AutomatedSync {
                                prune: true,
                                self_heal: true,
                            }),
                            sync_options: vec!["CreateNamespace=true".to_string()],
                        })
                    } else {
                        None
                    },
                },
            }
        }

        /// Convert to YAML
        pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
            serde_yaml::to_string(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitops_manager_creation() {
        let config = GitOpsConfig {
            repo_url: "https://github.com/example/chaos-configs".to_string(),
            branch: "main".to_string(),
            path: PathBuf::from("orchestrations"),
            sync_interval_seconds: 300,
            auth: GitOpsAuth::Token {
                token: "test-token".to_string(),
            },
            auto_sync: true,
            prune: true,
        };

        let manager = GitOpsManager::new(config);
        assert!(manager.is_auto_sync_enabled());
        assert_eq!(manager.get_sync_interval(), 300);
    }

    #[test]
    fn test_flux_kustomization() {
        let kustomization = flux::FluxKustomization::new_for_orchestrations(
            "chaos-orchestrations".to_string(),
            "chaos-testing".to_string(),
            "chaos-repo".to_string(),
            "./orchestrations".to_string(),
        );

        assert_eq!(kustomization.metadata.name, "chaos-orchestrations");
        assert!(kustomization.spec.prune);
    }

    #[test]
    fn test_argocd_application() {
        let app = argocd::ArgoApplication::new_for_orchestrations(
            "chaos-app".to_string(),
            "chaos-testing".to_string(),
            "https://github.com/example/chaos".to_string(),
            "./orchestrations".to_string(),
            true,
        );

        assert_eq!(app.metadata.name, "chaos-app");
        assert!(app.spec.sync_policy.is_some());
    }
}
