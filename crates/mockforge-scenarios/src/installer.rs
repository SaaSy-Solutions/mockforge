//! Scenario installer
//!
//! Handles installation of scenarios from various sources (local, URL, Git, registry)

use crate::error::{Result, ScenarioError};
use crate::package::ScenarioPackage;
use crate::preview::ScenarioPreview;
use crate::schema_alignment::{align_openapi_specs, SchemaAlignmentConfig};
use crate::source::ScenarioSource;
use crate::storage::{InstalledScenario, ScenarioStorage};
// VBR integration is handled at CLI/application level to avoid circular dependencies
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs as async_fs;
use tracing::{info, warn};

/// Installation options
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Force reinstall even if scenario already exists
    pub force: bool,

    /// Skip validation checks
    pub skip_validation: bool,

    /// Expected checksum for verification (URL sources)
    pub expected_checksum: Option<String>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            force: false,
            skip_validation: false,
            expected_checksum: None,
        }
    }
}

/// Scenario installer
pub struct ScenarioInstaller {
    storage: ScenarioStorage,
    client: Client,
    cache_dir: PathBuf,
}

impl ScenarioInstaller {
    /// Create a new scenario installer
    pub fn new() -> Result<Self> {
        let storage = ScenarioStorage::new()?;

        // Create cache directory
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("mockforge")
            .join("scenarios");
        std::fs::create_dir_all(&cache_dir).map_err(|e| {
            ScenarioError::Storage(format!("Failed to create cache directory: {}", e))
        })?;

        // Create HTTP client
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes
            .user_agent(format!("MockForge/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ScenarioError::Network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            storage,
            client,
            cache_dir,
        })
    }

    /// Initialize the installer (loads storage)
    pub async fn init(&mut self) -> Result<()> {
        self.storage.init().await?;
        self.storage.load().await?;
        Ok(())
    }

    /// Preview a scenario from a source string without installing
    ///
    /// Downloads/loads the scenario package and returns preview information
    /// without actually installing it.
    pub async fn preview(&self, source_str: &str) -> Result<ScenarioPreview> {
        let source = ScenarioSource::parse(source_str)?;
        self.preview_from_source(&source).await
    }

    /// Preview a scenario from a specific source
    pub async fn preview_from_source(&self, source: &ScenarioSource) -> Result<ScenarioPreview> {
        info!("Previewing scenario from source: {}", source);

        // Get the scenario package directory (without installing)
        let package_dir = match source {
            ScenarioSource::Local(path) => {
                // Validate local path exists
                if !path.exists() {
                    return Err(ScenarioError::NotFound(format!(
                        "Scenario path not found: {}",
                        path.display()
                    )));
                }
                path.clone()
            }
            ScenarioSource::Url { url, checksum } => {
                self.download_from_url(&url, checksum.as_deref()).await?
            }
            ScenarioSource::Git {
                url,
                reference,
                subdirectory,
            } => self.clone_from_git(&url, reference.as_deref(), subdirectory.as_deref()).await?,
            ScenarioSource::Registry { name, version } => {
                self.download_from_registry(&name, version.as_deref()).await?
            }
        };

        // Load package (without validation to allow preview of invalid packages)
        let package = ScenarioPackage::from_directory(&package_dir)?;

        // Create preview
        ScenarioPreview::from_package(&package)
    }

    /// Install a scenario from a source string
    ///
    /// Automatically detects and handles the source type
    pub async fn install(&mut self, source_str: &str, options: InstallOptions) -> Result<String> {
        let source = ScenarioSource::parse(source_str)?;
        self.install_from_source(&source, options).await
    }

    /// Install a scenario from a specific source
    pub async fn install_from_source(
        &mut self,
        source: &ScenarioSource,
        options: InstallOptions,
    ) -> Result<String> {
        info!("Installing scenario from source: {}", source);

        // Get the scenario package directory
        let package_dir = match source {
            ScenarioSource::Local(path) => {
                // Validate local path exists
                if !path.exists() {
                    return Err(ScenarioError::NotFound(format!(
                        "Scenario path not found: {}",
                        path.display()
                    )));
                }
                path.clone()
            }
            ScenarioSource::Url { url, checksum } => {
                self.download_from_url(&url, checksum.as_deref()).await?
            }
            ScenarioSource::Git {
                url,
                reference,
                subdirectory,
            } => self.clone_from_git(&url, reference.as_deref(), subdirectory.as_deref()).await?,
            ScenarioSource::Registry { name, version } => {
                self.download_from_registry(&name, version.as_deref()).await?
            }
        };

        // Load and validate package
        let package = ScenarioPackage::from_directory(&package_dir)?;

        if !options.skip_validation {
            let validation = package.validate()?;
            if !validation.is_valid {
                return Err(ScenarioError::InvalidManifest(format!(
                    "Package validation failed: {}",
                    validation.errors.join(", ")
                )));
            }

            // Log warnings
            for warning in &validation.warnings {
                warn!("Package validation warning: {}", warning);
            }
        }

        let manifest = &package.manifest;
        let scenario_id = manifest.id();

        // Check if scenario is already installed
        if let Some(existing) = self.storage.get(&manifest.name, &manifest.version) {
            if !options.force {
                return Err(ScenarioError::AlreadyExists(format!(
                    "Scenario {} is already installed at {}",
                    scenario_id,
                    existing.path.display()
                )));
            }

            // Remove existing installation
            info!("Removing existing installation of {}", scenario_id);
            self.uninstall(&manifest.name, &manifest.version).await?;
        }

        // Copy package to storage location
        let install_path = self.storage.scenario_path(&manifest.name, &manifest.version);

        // Create parent directory
        if let Some(parent) = install_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ScenarioError::Storage(format!("Failed to create install directory: {}", e))
            })?;
        }

        // Copy all files from package to install location
        self.copy_package(&package_dir, &install_path)?;

        // Create installed scenario metadata
        let installed = InstalledScenario::new(
            manifest.name.clone(),
            manifest.version.clone(),
            install_path.clone(),
            source.to_string(),
            manifest.clone(),
        );

        // Save metadata
        self.storage.save(installed).await?;

        info!("Scenario installed successfully: {} at {}", scenario_id, install_path.display());
        Ok(scenario_id)
    }

    /// Download scenario from URL
    async fn download_from_url(
        &self,
        url: &str,
        expected_checksum: Option<&str>,
    ) -> Result<PathBuf> {
        info!("Downloading scenario from URL: {}", url);

        // Parse URL to determine file type
        let url_parsed = reqwest::Url::parse(url)
            .map_err(|e| ScenarioError::InvalidSource(format!("Invalid URL '{}': {}", url, e)))?;

        let file_name = url_parsed
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .ok_or_else(|| {
                ScenarioError::InvalidSource("Could not determine file name from URL".to_string())
            })?;

        // Check cache first
        let cache_key = self.generate_cache_key(url);
        let cached_path = self.cache_dir.join(&cache_key);

        if cached_path.exists() {
            info!("Using cached scenario at: {}", cached_path.display());
            return Ok(cached_path);
        }

        // Download file with progress tracking
        let (temp_file, _temp_dir) = self.download_with_progress(url, file_name).await?;

        // Extract or move file based on type
        let scenario_dir = if file_name.ends_with(".zip") {
            self.extract_zip(&temp_file, &cached_path).await?
        } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            self.extract_tar_gz(&temp_file, &cached_path).await?
        } else {
            return Err(ScenarioError::InvalidSource(format!(
                "Unsupported file type: {}. Supported: .zip, .tar.gz",
                file_name
            )));
        };

        // Verify checksum if provided
        if let Some(checksum) = expected_checksum {
            self.verify_checksum(&scenario_dir, checksum)?;
        }

        // Clean up temp file (temp_dir will be dropped automatically)
        let _ = async_fs::remove_file(&temp_file).await;

        info!("Scenario downloaded and extracted to: {}", scenario_dir.display());
        Ok(scenario_dir)
    }

    /// Clone scenario from Git repository
    #[cfg(feature = "git-support")]
    async fn clone_from_git(
        &self,
        url: &str,
        reference: Option<&str>,
        subdirectory: Option<&str>,
    ) -> Result<PathBuf> {
        use git2::{build::RepoBuilder, FetchOptions, Repository};

        info!("Cloning scenario from Git: {}", url);

        // Generate cache key
        let cache_key =
            self.generate_cache_key(&format!("{}{:?}{:?}", url, reference, subdirectory));
        let repo_path = self.cache_dir.join(&cache_key);

        // Check if repository is already cloned
        if repo_path.exists() && Repository::open(&repo_path).is_ok() {
            info!("Using cached repository at: {}", repo_path.display());
        } else {
            // Clone the repository
            let mut fetch_options = FetchOptions::new();
            fetch_options.depth(1); // Shallow clone

            let mut repo_builder = RepoBuilder::new();
            repo_builder.fetch_options(fetch_options);

            if let Some(branch) = reference {
                repo_builder.branch(branch);
            }

            let repo = repo_builder.clone(url, &repo_path).map_err(|e| {
                ScenarioError::Network(format!("Failed to clone repository: {}", e))
            })?;

            // Checkout specific ref if needed
            if let Some(ref_str) = reference {
                if ref_str.len() == 40 && ref_str.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Commit SHA
                    let obj = repo.revparse_single(ref_str).map_err(|e| {
                        ScenarioError::Network(format!("Failed to find commit: {}", e))
                    })?;
                    repo.checkout_tree(&obj, None).map_err(|e| {
                        ScenarioError::Network(format!("Failed to checkout: {}", e))
                    })?;
                    repo.set_head_detached(obj.id()).map_err(|e| {
                        ScenarioError::Network(format!("Failed to set HEAD: {}", e))
                    })?;
                } else if ref_str.starts_with('v') {
                    // Tag
                    let refname = format!("refs/tags/{}", ref_str);
                    let obj = repo.revparse_single(&refname).map_err(|e| {
                        ScenarioError::Network(format!("Failed to find tag: {}", e))
                    })?;
                    repo.checkout_tree(&obj, None).map_err(|e| {
                        ScenarioError::Network(format!("Failed to checkout: {}", e))
                    })?;
                    repo.set_head_detached(obj.id()).map_err(|e| {
                        ScenarioError::Network(format!("Failed to set HEAD: {}", e))
                    })?;
                }
            }
        }

        // If subdirectory is specified, return that path
        let scenario_path = if let Some(subdir) = subdirectory {
            let subdir_path = repo_path.join(subdir);
            if !subdir_path.exists() {
                return Err(ScenarioError::NotFound(format!(
                    "Subdirectory '{}' not found in repository",
                    subdir
                )));
            }
            subdir_path
        } else {
            repo_path
        };

        info!("Scenario cloned to: {}", scenario_path.display());
        Ok(scenario_path)
    }

    #[cfg(not(feature = "git-support"))]
    async fn clone_from_git(
        &self,
        _url: &str,
        _reference: Option<&str>,
        _subdirectory: Option<&str>,
    ) -> Result<PathBuf> {
        Err(ScenarioError::Generic(
            "Git support not enabled. Enable 'git-support' feature.".to_string(),
        ))
    }

    /// Download file with progress bar
    /// Returns (temp_file_path, temp_dir) to keep temp_dir alive
    async fn download_with_progress(
        &self,
        url: &str,
        file_name: &str,
    ) -> Result<(PathBuf, tempfile::TempDir)> {
        let mut response = self.client.get(url).send().await.map_err(|e| {
            ScenarioError::Network(format!("Failed to download from '{}': {}", url, e))
        })?;

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        // Get content length for progress bar
        let total_size = response.content_length();

        // Create progress bar
        let progress_bar = total_size.map(|size| {
            let pb = ProgressBar::new(size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            pb.set_message(format!("Downloading {}", file_name));
            pb
        });

        // Create temporary file
        let temp_dir = tempfile::tempdir().map_err(|e| {
            ScenarioError::Storage(format!("Failed to create temp directory: {}", e))
        })?;
        let temp_file = temp_dir.path().join(file_name);
        let mut file = std::fs::File::create(&temp_file)
            .map_err(|e| ScenarioError::Storage(format!("Failed to create temp file: {}", e)))?;

        // Download chunks and write to file
        let mut downloaded: u64 = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| ScenarioError::Network(format!("Failed to download chunk: {}", e)))?
        {
            file.write_all(&chunk)
                .map_err(|e| ScenarioError::Storage(format!("Failed to write chunk: {}", e)))?;

            downloaded += chunk.len() as u64;

            if let Some(ref pb) = progress_bar {
                pb.set_position(downloaded);
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message(format!("Downloaded {}", file_name));
        }

        file.flush()
            .map_err(|e| ScenarioError::Storage(format!("Failed to flush file: {}", e)))?;
        drop(file);

        // Return both temp_file and temp_dir to keep temp_dir alive
        Ok((temp_file, temp_dir))
    }

    /// Extract a ZIP archive
    async fn extract_zip(&self, zip_path: &PathBuf, dest: &PathBuf) -> Result<PathBuf> {
        info!("Extracting ZIP archive to: {}", dest.display());

        let file = fs::File::open(zip_path)
            .map_err(|e| ScenarioError::Storage(format!("Failed to open ZIP file: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ScenarioError::Storage(format!("Failed to read ZIP archive: {}", e)))?;

        fs::create_dir_all(dest)
            .map_err(|e| ScenarioError::Storage(format!("Failed to create directory: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| ScenarioError::Storage(format!("Failed to read ZIP entry: {}", e)))?;

            let outpath = match file.enclosed_name() {
                Some(path) => dest.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath).map_err(|e| {
                    ScenarioError::Storage(format!("Failed to create directory: {}", e))
                })?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p).map_err(|e| {
                        ScenarioError::Storage(format!("Failed to create parent directory: {}", e))
                    })?;
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| ScenarioError::Storage(format!("Failed to create file: {}", e)))?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| {
                    ScenarioError::Storage(format!("Failed to extract file: {}", e))
                })?;
            }
        }

        Ok(dest.clone())
    }

    /// Extract a tar.gz archive
    async fn extract_tar_gz(&self, tar_path: &PathBuf, dest: &PathBuf) -> Result<PathBuf> {
        info!("Extracting tar.gz archive to: {}", dest.display());

        let file = fs::File::open(tar_path)
            .map_err(|e| ScenarioError::Storage(format!("Failed to open tar.gz file: {}", e)))?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        fs::create_dir_all(dest)
            .map_err(|e| ScenarioError::Storage(format!("Failed to create directory: {}", e)))?;

        archive.unpack(dest).map_err(|e| {
            ScenarioError::Storage(format!("Failed to extract tar.gz archive: {}", e))
        })?;

        Ok(dest.clone())
    }

    /// Verify scenario checksum (SHA-256 of scenario.yaml)
    fn verify_checksum(&self, scenario_dir: &PathBuf, expected_checksum: &str) -> Result<()> {
        use ring::digest::{Context, SHA256};

        info!("Verifying scenario checksum...");

        let manifest_path = scenario_dir.join("scenario.yaml");
        if !manifest_path.exists() {
            return Err(ScenarioError::InvalidManifest(
                "scenario.yaml not found in downloaded package".to_string(),
            ));
        }

        // Calculate SHA-256 hash of the manifest
        let file_contents = fs::read(&manifest_path)
            .map_err(|e| ScenarioError::Storage(format!("Failed to read manifest: {}", e)))?;

        let mut context = Context::new(&SHA256);
        context.update(&file_contents);
        let digest = context.finish();
        let calculated_checksum = hex::encode(digest.as_ref());

        // Compare checksums
        if calculated_checksum != expected_checksum {
            return Err(ScenarioError::ChecksumMismatch {
                expected: expected_checksum.to_string(),
                actual: calculated_checksum,
            });
        }

        info!("Checksum verified successfully");
        Ok(())
    }

    /// Generate a cache key from URL
    fn generate_cache_key(&self, url: &str) -> String {
        use ring::digest::{Context, SHA256};
        let mut context = Context::new(&SHA256);
        context.update(url.as_bytes());
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Download scenario from registry
    async fn download_from_registry(&self, name: &str, version: Option<&str>) -> Result<PathBuf> {
        use crate::registry::ScenarioRegistry;

        info!("Downloading scenario from registry: {}@{}", name, version.unwrap_or("latest"));

        // Create registry client
        let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

        // Get scenario entry
        let entry = registry.get_scenario(name).await?;

        // Determine version to download
        let target_version = version.unwrap_or(&entry.version);

        // Find version entry
        let version_entry = entry
            .versions
            .iter()
            .find(|v| v.version == target_version && !v.yanked)
            .ok_or_else(|| {
                ScenarioError::InvalidVersion(format!(
                    "Version {} not found or yanked for scenario {}",
                    target_version, name
                ))
            })?;

        info!("Downloading version {} ({} bytes)", target_version, version_entry.size);

        // Download package
        let package_data = registry
            .download(&version_entry.download_url, Some(&version_entry.checksum))
            .await?;

        // Save to temporary file
        let temp_dir = tempfile::tempdir().map_err(|e| {
            ScenarioError::Storage(format!("Failed to create temp directory: {}", e))
        })?;

        // Determine file extension from download URL
        let file_name = if version_entry.download_url.ends_with(".zip") {
            format!("{}.zip", name)
        } else if version_entry.download_url.ends_with(".tar.gz")
            || version_entry.download_url.ends_with(".tgz")
        {
            format!("{}.tar.gz", name)
        } else {
            format!("{}.zip", name) // Default to zip
        };

        let temp_file = temp_dir.path().join(&file_name);
        std::fs::write(&temp_file, &package_data).map_err(|e| {
            ScenarioError::Storage(format!("Failed to write downloaded package: {}", e))
        })?;

        // Extract to cache
        let cache_key = self.generate_cache_key(&format!("registry:{}@{}", name, target_version));
        let cached_path = self.cache_dir.join(&cache_key);

        let scenario_dir = if file_name.ends_with(".zip") {
            self.extract_zip(&temp_file, &cached_path).await?
        } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            self.extract_tar_gz(&temp_file, &cached_path).await?
        } else {
            return Err(ScenarioError::InvalidSource(format!(
                "Unsupported package format from registry"
            )));
        };

        info!("Scenario downloaded from registry to: {}", scenario_dir.display());
        Ok(scenario_dir)
    }

    /// Copy package files to installation location
    fn copy_package(&self, source: &PathBuf, dest: &PathBuf) -> Result<()> {
        if source.is_dir() {
            copy_dir::copy_dir(source, dest)
                .map_err(|e| ScenarioError::Storage(format!("Failed to copy package: {}", e)))?;
        } else {
            std::fs::copy(source, dest)
                .map_err(|e| ScenarioError::Storage(format!("Failed to copy package: {}", e)))?;
        }

        Ok(())
    }

    /// Uninstall a scenario
    pub async fn uninstall(&mut self, name: &str, version: &str) -> Result<()> {
        info!("Uninstalling scenario: {}@{}", name, version);

        // Get scenario info
        let scenario = self
            .storage
            .get(name, version)
            .ok_or_else(|| {
                ScenarioError::NotFound(format!("Scenario {}@{} not found", name, version))
            })?
            .clone();

        // Remove scenario directory
        if scenario.path.exists() {
            std::fs::remove_dir_all(&scenario.path).map_err(|e| {
                ScenarioError::Storage(format!("Failed to remove scenario directory: {}", e))
            })?;
        }

        // Remove metadata
        self.storage.remove(name, version).await?;

        info!("Scenario uninstalled: {}@{}", name, version);
        Ok(())
    }

    /// List installed scenarios
    pub fn list_installed(&self) -> Vec<&InstalledScenario> {
        self.storage.list()
    }

    /// Get scenario by name and version
    pub fn get(&self, name: &str, version: &str) -> Option<&InstalledScenario> {
        self.storage.get(name, version)
    }

    /// Get scenario by name (latest version)
    pub fn get_latest(&self, name: &str) -> Option<&InstalledScenario> {
        self.storage.get_latest(name)
    }

    /// Update all installed scenarios to their latest versions
    pub async fn update_all(&mut self) -> Result<Vec<String>> {
        info!("Updating all installed scenarios...");

        // Collect scenario info first to avoid borrow conflicts
        let scenarios_info: Vec<(String, String, String)> = self
            .list_installed()
            .iter()
            .map(|s| (s.name.clone(), s.version.clone(), s.source.clone()))
            .collect();

        let mut updated = Vec::new();

        for (name, current_version, source_str) in scenarios_info {
            info!("Checking for updates: {}@{}", name, current_version);

            // Parse source to determine if it's from registry
            let source = ScenarioSource::parse(&source_str)?;

            match source {
                ScenarioSource::Registry { .. } => {
                    // Try to update from registry
                    match self.update_from_registry(&name, &current_version).await {
                        Ok(new_version) => {
                            if new_version != current_version {
                                updated.push(format!(
                                    "{}@{} -> {}",
                                    name, current_version, new_version
                                ));
                            }
                        }
                        Err(e) => {
                            warn!("Failed to update {}: {}", name, e);
                        }
                    }
                }
                _ => {
                    // For non-registry sources, reinstall from original source
                    info!("Reinstalling {} from original source: {}", name, source_str);
                    let options = InstallOptions {
                        force: true,
                        skip_validation: false,
                        expected_checksum: None,
                    };

                    match self.install(&source_str, options).await {
                        Ok(_) => {
                            updated.push(format!("{}@{}", name, current_version));
                        }
                        Err(e) => {
                            warn!("Failed to update {}: {}", name, e);
                        }
                    }
                }
            }
        }

        info!("Updated {} scenarios", updated.len());
        Ok(updated)
    }

    /// Update a single scenario from registry
    pub async fn update_from_registry(
        &mut self,
        name: &str,
        current_version: &str,
    ) -> Result<String> {
        use crate::registry::ScenarioRegistry;

        let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

        // Get latest version from registry
        let entry = registry.get_scenario(name).await?;

        if entry.version == current_version {
            return Ok(current_version.to_string()); // Already up to date
        }

        // Uninstall current version
        self.uninstall(name, current_version).await?;

        // Install latest version
        let source = format!("registry:{}", name);
        let options = InstallOptions {
            force: false,
            skip_validation: false,
            expected_checksum: None,
        };

        self.install(&source, options).await?;

        Ok(entry.version)
    }

    /// Apply scenario to current workspace
    ///
    /// Copies scenario files (config.yaml, openapi.json, fixtures, etc.) to the current directory
    pub async fn apply_to_workspace(&self, name: &str, version: Option<&str>) -> Result<()> {
        self.apply_to_workspace_with_alignment(name, version, None).await
    }

    /// Apply scenario to current workspace with schema alignment
    ///
    /// Copies scenario files and optionally aligns schemas/routes with existing workspace files.
    pub async fn apply_to_workspace_with_alignment(
        &self,
        name: &str,
        version: Option<&str>,
        alignment_config: Option<SchemaAlignmentConfig>,
    ) -> Result<()> {
        let scenario = if let Some(v) = version {
            self.get(name, v)
        } else {
            self.get_latest(name)
        };

        let scenario = scenario
            .ok_or_else(|| ScenarioError::NotFound(format!("Scenario '{}' not found", name)))?;

        info!("Applying scenario {}@{} to workspace", scenario.name, scenario.version);

        let current_dir = std::env::current_dir().map_err(|e| {
            ScenarioError::Storage(format!("Failed to get current directory: {}", e))
        })?;

        // Copy config.yaml if it exists
        let config_source = scenario.path.join("config.yaml");
        if config_source.exists() {
            let config_dest = current_dir.join("config.yaml");
            std::fs::copy(&config_source, &config_dest).map_err(|e| {
                ScenarioError::Storage(format!("Failed to copy config.yaml: {}", e))
            })?;
            info!("Copied config.yaml to workspace");
        }

        // Handle OpenAPI spec alignment if both exist
        let openapi_source = scenario.path.join("openapi.json");
        let openapi_dest = current_dir.join("openapi.json");
        let openapi_yaml_source = scenario.path.join("openapi.yaml");
        let openapi_yaml_dest = current_dir.join("openapi.yaml");

        if openapi_source.exists() || openapi_yaml_source.exists() {
            let scenario_spec_path = if openapi_source.exists() {
                &openapi_source
            } else {
                &openapi_yaml_source
            };

            let scenario_spec_content =
                std::fs::read_to_string(scenario_spec_path).map_err(|e| ScenarioError::Io(e))?;

            // Try to parse as JSON first, then YAML
            let scenario_spec: Value = if scenario_spec_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "json")
                .unwrap_or(false)
            {
                serde_json::from_str(&scenario_spec_content).map_err(|e| ScenarioError::Serde(e))?
            } else {
                serde_yaml::from_str(&scenario_spec_content).map_err(|e| ScenarioError::Yaml(e))?
            };

            // Check if existing OpenAPI spec exists
            let existing_spec_path = if openapi_dest.exists() {
                Some(&openapi_dest)
            } else if openapi_yaml_dest.exists() {
                Some(&openapi_yaml_dest)
            } else {
                None
            };

            if let (Some(existing_path), Some(ref align_config)) =
                (existing_spec_path, alignment_config)
            {
                // Align schemas
                let existing_spec_content =
                    std::fs::read_to_string(existing_path).map_err(|e| ScenarioError::Io(e))?;

                let existing_spec: Value = if existing_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
                {
                    serde_json::from_str(&existing_spec_content)
                        .map_err(|e| ScenarioError::Serde(e))?
                } else {
                    serde_yaml::from_str(&existing_spec_content)
                        .map_err(|e| ScenarioError::Yaml(e))?
                };

                let alignment_result =
                    align_openapi_specs(&existing_spec, &scenario_spec, align_config)?;

                if alignment_result.success {
                    if let Some(merged_spec) = alignment_result.merged_spec {
                        // Write merged spec
                        let merged_json = serde_json::to_string_pretty(&merged_spec)
                            .map_err(|e| ScenarioError::Serde(e))?;
                        std::fs::write(&openapi_dest, merged_json).map_err(|e| {
                            ScenarioError::Storage(format!(
                                "Failed to write merged openapi.json: {}",
                                e
                            ))
                        })?;
                        info!(
                            "Merged OpenAPI spec with existing (strategy: {:?})",
                            align_config.merge_strategy
                        );

                        // Log warnings
                        for warning in &alignment_result.warnings {
                            warn!("OpenAPI alignment warning: {}", warning);
                        }
                    }
                } else {
                    // Alignment failed (e.g., interactive mode with conflicts)
                    warn!("OpenAPI alignment found {} conflicts", alignment_result.conflicts.len());
                    for conflict in &alignment_result.conflicts {
                        warn!("Conflict: {:?} at {}", conflict.conflict_type, conflict.path);
                    }
                    // Fall back to copying scenario spec
                    std::fs::copy(scenario_spec_path, &openapi_dest).map_err(|e| {
                        ScenarioError::Storage(format!("Failed to copy openapi.json: {}", e))
                    })?;
                    info!("Copied openapi.json to workspace (alignment had conflicts)");
                }
            } else {
                // No existing spec, just copy scenario spec
                let dest_path = if openapi_source.exists() {
                    &openapi_dest
                } else {
                    &openapi_yaml_dest
                };
                std::fs::copy(scenario_spec_path, dest_path).map_err(|e| {
                    ScenarioError::Storage(format!("Failed to copy openapi spec: {}", e))
                })?;
                info!("Copied openapi spec to workspace");
            }
        }

        // Copy fixtures directory if it exists
        let fixtures_source = scenario.path.join("fixtures");
        if fixtures_source.exists() {
            let fixtures_dest = current_dir.join("fixtures");
            if fixtures_dest.exists() {
                // Merge fixtures
                info!("Merging fixtures into existing fixtures directory");
            } else {
                std::fs::create_dir_all(&fixtures_dest).map_err(|e| {
                    ScenarioError::Storage(format!("Failed to create fixtures directory: {}", e))
                })?;
            }
            copy_dir::copy_dir(&fixtures_source, &fixtures_dest)
                .map_err(|e| ScenarioError::Storage(format!("Failed to copy fixtures: {}", e)))?;
            info!("Copied fixtures to workspace");
        }

        // Copy examples directory if it exists
        let examples_source = scenario.path.join("examples");
        if examples_source.exists() {
            let examples_dest = current_dir.join("examples");
            if !examples_dest.exists() {
                std::fs::create_dir_all(&examples_dest).map_err(|e| {
                    ScenarioError::Storage(format!("Failed to create examples directory: {}", e))
                })?;
            }
            copy_dir::copy_dir(&examples_source, &examples_dest)
                .map_err(|e| ScenarioError::Storage(format!("Failed to copy examples: {}", e)))?;
            info!("Copied examples to workspace");
        }

        // Apply VBR entities if defined in scenario
        if let Some(ref vbr_entities) = scenario.manifest.vbr_entities {
            if !vbr_entities.is_empty() {
                info!("Scenario contains {} VBR entity definitions", vbr_entities.len());
                info!("Note: VBR entities need to be applied separately using VBR engine");
                info!("Use 'mockforge vbr' commands or programmatic API to apply entities");
            }
        }

        // Apply MockAI config if defined in scenario
        if let Some(ref mockai_config) = scenario.manifest.mockai_config {
            info!("Scenario contains MockAI configuration");
            info!("Note: MockAI config needs to be merged with existing config.yaml");
            info!("MockAI config will be available in the scenario package");
        }

        info!("Scenario applied successfully to workspace");
        Ok(())
    }

    /// Get VBR entity definitions from a scenario
    ///
    /// Returns the VBR entity definitions if the scenario contains any.
    /// The actual application of entities should be handled at the CLI/application level.
    pub fn get_vbr_entities(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<&[crate::vbr_integration::VbrEntityDefinition]>> {
        let scenario = if let Some(v) = version {
            self.get(name, v)
        } else {
            self.get_latest(name)
        };

        let scenario = scenario
            .ok_or_else(|| ScenarioError::NotFound(format!("Scenario '{}' not found", name)))?;

        Ok(scenario.manifest.vbr_entities.as_deref())
    }

    /// Get MockAI configuration from a scenario
    ///
    /// Returns the MockAI configuration if the scenario contains any.
    /// The actual application of config should be handled at the CLI/application level.
    pub fn get_mockai_config(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<&crate::mockai_integration::MockAIConfigDefinition>> {
        let scenario = if let Some(v) = version {
            self.get(name, v)
        } else {
            self.get_latest(name)
        };

        let scenario = scenario
            .ok_or_else(|| ScenarioError::NotFound(format!("Scenario '{}' not found", name)))?;

        Ok(scenario.manifest.mockai_config.as_ref())
    }
}

// Simple directory copy helper
mod copy_dir {
    use std::fs;
    use std::path::Path;

    pub fn copy_dir<P: AsRef<Path>>(from: P, to: P) -> std::io::Result<()> {
        let from = from.as_ref();
        let to = to.as_ref();

        if !from.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source is not a directory",
            ));
        }

        fs::create_dir_all(to)?;

        for entry in fs::read_dir(from)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let dest = to.join(&file_name);

            if path.is_dir() {
                copy_dir(&path, &dest)?;
            } else {
                fs::copy(&path, &dest)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_installer_creation() {
        let installer = ScenarioInstaller::new().unwrap();
        assert!(installer.list_installed().is_empty());
    }
}
