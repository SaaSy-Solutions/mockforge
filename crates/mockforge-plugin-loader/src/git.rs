//! Git repository plugin loading
//!
//! This module provides functionality for cloning plugins from Git repositories
//! with support for:
//! - Version pinning (tags, branches, commits)
//! - Shallow clones for performance
//! - SSH and HTTPS authentication
//! - Repository caching

use super::*;
use std::path::{Path, PathBuf};

#[cfg(feature = "git-support")]
use git2::{build::RepoBuilder, FetchOptions, Repository};

/// Git repository reference (tag, branch, or commit)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    /// A specific tag (e.g., "v1.0.0")
    Tag(String),
    /// A branch name (e.g., "main", "develop")
    Branch(String),
    /// A commit SHA (e.g., "abc123def456")
    Commit(String),
    /// Default branch (usually "main" or "master")
    Default,
}

impl GitRef {
    /// Parse a Git reference from a string
    ///
    /// Examples:
    /// - "v1.0.0" -> Tag("v1.0.0")
    /// - "main" -> Branch("main")
    /// - "abc123" -> Commit("abc123") if it looks like a commit SHA
    pub fn parse(s: &str) -> Self {
        if s.is_empty() {
            return GitRef::Default;
        }

        // Check if it's a commit SHA (40-char hex string)
        if s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            return GitRef::Commit(s.to_string());
        }

        // Check if it starts with 'v' followed by numbers (version tag)
        if s.starts_with('v') && s[1..].chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return GitRef::Tag(s.to_string());
        }

        // Otherwise, treat as branch
        GitRef::Branch(s.to_string())
    }
}

impl std::fmt::Display for GitRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitRef::Tag(tag) => write!(f, "tag:{}", tag),
            GitRef::Branch(branch) => write!(f, "branch:{}", branch),
            GitRef::Commit(commit) => write!(f, "commit:{}", commit),
            GitRef::Default => write!(f, "default"),
        }
    }
}

/// Git plugin source specification
#[derive(Debug, Clone)]
pub struct GitPluginSource {
    /// Repository URL (HTTPS or SSH)
    pub url: String,
    /// Git reference (tag, branch, or commit)
    pub git_ref: GitRef,
    /// Subdirectory within the repo (optional)
    pub subdirectory: Option<String>,
}

impl GitPluginSource {
    /// Parse a Git plugin source from a string
    ///
    /// Formats:
    /// - `https://github.com/user/repo` - Default branch
    /// - `https://github.com/user/repo#v1.0.0` - Specific tag/branch/commit
    /// - `https://github.com/user/repo#v1.0.0:subdir` - With subdirectory
    pub fn parse(input: &str) -> LoaderResult<Self> {
        // Split on '#' for ref specification
        let (url_part, ref_part) = if let Some((url, ref_spec)) = input.split_once('#') {
            (url, Some(ref_spec))
        } else {
            (input, None)
        };

        // Parse ref and subdirectory
        let (git_ref, subdirectory) = if let Some(ref_spec) = ref_part {
            if let Some((ref_str, subdir)) = ref_spec.split_once(':') {
                (GitRef::parse(ref_str), Some(subdir.to_string()))
            } else {
                (GitRef::parse(ref_spec), None)
            }
        } else {
            (GitRef::Default, None)
        };

        Ok(Self {
            url: url_part.to_string(),
            git_ref,
            subdirectory,
        })
    }
}

impl std::fmt::Display for GitPluginSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.url, self.git_ref)?;
        if let Some(ref subdir) = self.subdirectory {
            write!(f, ":{}", subdir)?;
        }
        Ok(())
    }
}

/// Configuration for Git plugin loading
#[derive(Debug, Clone)]
pub struct GitPluginConfig {
    /// Cache directory for cloned repositories
    pub cache_dir: PathBuf,
    /// Use shallow clones (depth=1) for performance
    pub shallow_clone: bool,
    /// Include submodules when cloning
    pub include_submodules: bool,
}

impl Default for GitPluginConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("mockforge")
                .join("git-plugins"),
            shallow_clone: true,
            include_submodules: false,
        }
    }
}

/// Git plugin loader for cloning plugins from Git repositories
#[cfg(feature = "git-support")]
pub struct GitPluginLoader {
    config: GitPluginConfig,
}

#[cfg(feature = "git-support")]
impl GitPluginLoader {
    /// Create a new Git plugin loader
    pub fn new(config: GitPluginConfig) -> LoaderResult<Self> {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| {
            PluginLoaderError::fs(format!(
                "Failed to create cache directory {}: {}",
                config.cache_dir.display(),
                e
            ))
        })?;

        Ok(Self { config })
    }

    /// Clone a plugin from a Git repository
    ///
    /// Returns the path to the cloned plugin directory
    pub async fn clone_from_git(&self, source: &GitPluginSource) -> LoaderResult<PathBuf> {
        tracing::info!("Cloning plugin from Git: {}", source);

        // Generate cache key from repository URL and ref
        let cache_key = self.generate_cache_key(&source.url, &source.git_ref);
        let repo_path = self.config.cache_dir.join(&cache_key);

        // Check if repository is already cloned
        if repo_path.exists() && Repository::open(&repo_path).is_ok() {
            tracing::info!("Using cached repository at: {}", repo_path.display());

            // Update the repository
            self.update_repository(&repo_path, source).await?;
        } else {
            // Clone the repository
            self.clone_repository(&source.url, &repo_path, source).await?;
        }

        // If subdirectory is specified, return that path
        let plugin_path = if let Some(ref subdir) = source.subdirectory {
            let subdir_path = repo_path.join(subdir);
            if !subdir_path.exists() {
                return Err(PluginLoaderError::load(format!(
                    "Subdirectory '{}' not found in repository",
                    subdir
                )));
            }
            subdir_path
        } else {
            repo_path
        };

        tracing::info!("Plugin cloned to: {}", plugin_path.display());
        Ok(plugin_path)
    }

    /// Clone a repository
    async fn clone_repository(
        &self,
        url: &str,
        dest: &Path,
        source: &GitPluginSource,
    ) -> LoaderResult<()> {
        tracing::info!("Cloning repository from: {}", url);

        // Prepare fetch options
        let mut fetch_options = FetchOptions::new();

        // Configure shallow clone if enabled
        if self.config.shallow_clone && matches!(source.git_ref, GitRef::Tag(_) | GitRef::Branch(_))
        {
            fetch_options.depth(1);
        }

        // Build repository
        let mut repo_builder = RepoBuilder::new();
        repo_builder.fetch_options(fetch_options);

        // Set branch if specified
        if let GitRef::Branch(ref branch) = source.git_ref {
            repo_builder.branch(branch);
        }

        // Clone the repository
        let repo = repo_builder
            .clone(url, dest)
            .map_err(|e| PluginLoaderError::load(format!("Failed to clone repository: {}", e)))?;

        // Checkout specific ref if needed
        match &source.git_ref {
            GitRef::Tag(tag) => {
                self.checkout_tag(&repo, tag)?;
            }
            GitRef::Commit(commit) => {
                self.checkout_commit(&repo, commit)?;
            }
            GitRef::Branch(_) | GitRef::Default => {
                // Already on the correct branch from clone
            }
        }

        // Initialize submodules if enabled
        if self.config.include_submodules {
            self.init_submodules(&repo)?;
        }

        tracing::info!("Repository cloned successfully");
        Ok(())
    }

    /// Update an existing repository
    async fn update_repository(&self, repo_path: &Path, source: &GitPluginSource) -> LoaderResult<()> {
        tracing::info!("Updating repository at: {}", repo_path.display());

        let repo = Repository::open(repo_path)
            .map_err(|e| PluginLoaderError::load(format!("Failed to open repository: {}", e)))?;

        // Fetch latest changes
        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| PluginLoaderError::load(format!("Failed to find remote: {}", e)))?;

        let mut fetch_options = FetchOptions::new();
        remote
            .fetch(&[] as &[&str], Some(&mut fetch_options), None)
            .map_err(|e| PluginLoaderError::load(format!("Failed to fetch: {}", e)))?;

        // Checkout the specified ref
        match &source.git_ref {
            GitRef::Tag(tag) => {
                self.checkout_tag(&repo, tag)?;
            }
            GitRef::Branch(branch) => {
                self.checkout_branch(&repo, branch)?;
            }
            GitRef::Commit(commit) => {
                self.checkout_commit(&repo, commit)?;
            }
            GitRef::Default => {
                // Stay on current branch, just pull
                self.pull_current_branch(&repo)?;
            }
        }

        tracing::info!("Repository updated successfully");
        Ok(())
    }

    /// Checkout a specific tag
    fn checkout_tag(&self, repo: &Repository, tag: &str) -> LoaderResult<()> {
        let refname = format!("refs/tags/{}", tag);
        let obj = repo
            .revparse_single(&refname)
            .map_err(|e| PluginLoaderError::load(format!("Failed to find tag '{}': {}", tag, e)))?;

        repo.checkout_tree(&obj, None)
            .map_err(|e| PluginLoaderError::load(format!("Failed to checkout tag: {}", e)))?;

        repo.set_head_detached(obj.id())
            .map_err(|e| PluginLoaderError::load(format!("Failed to set HEAD: {}", e)))?;

        Ok(())
    }

    /// Checkout a specific branch
    fn checkout_branch(&self, repo: &Repository, branch: &str) -> LoaderResult<()> {
        let refname = format!("refs/remotes/origin/{}", branch);
        let obj = repo
            .revparse_single(&refname)
            .map_err(|e| {
                PluginLoaderError::load(format!("Failed to find branch '{}': {}", branch, e))
            })?;

        repo.checkout_tree(&obj, None)
            .map_err(|e| PluginLoaderError::load(format!("Failed to checkout branch: {}", e)))?;

        // Create local branch if it doesn't exist
        let branch_refname = format!("refs/heads/{}", branch);
        let _ = repo.reference(&branch_refname, obj.id(), true, "checkout branch");

        repo.set_head(&branch_refname)
            .map_err(|e| PluginLoaderError::load(format!("Failed to set HEAD: {}", e)))?;

        Ok(())
    }

    /// Checkout a specific commit
    fn checkout_commit(&self, repo: &Repository, commit: &str) -> LoaderResult<()> {
        let obj = repo
            .revparse_single(commit)
            .map_err(|e| {
                PluginLoaderError::load(format!("Failed to find commit '{}': {}", commit, e))
            })?;

        repo.checkout_tree(&obj, None)
            .map_err(|e| PluginLoaderError::load(format!("Failed to checkout commit: {}", e)))?;

        repo.set_head_detached(obj.id())
            .map_err(|e| PluginLoaderError::load(format!("Failed to set HEAD: {}", e)))?;

        Ok(())
    }

    /// Pull the current branch
    fn pull_current_branch(&self, repo: &Repository) -> LoaderResult<()> {
        // Get current branch
        let head = repo
            .head()
            .map_err(|e| PluginLoaderError::load(format!("Failed to get HEAD: {}", e)))?;

        if !head.is_branch() {
            // Detached HEAD, nothing to pull
            return Ok(());
        }

        let branch = head
            .shorthand()
            .ok_or_else(|| PluginLoaderError::load("Failed to get branch name"))?;

        // Fetch and merge
        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| PluginLoaderError::load(format!("Failed to find remote: {}", e)))?;

        let mut fetch_options = FetchOptions::new();
        remote
            .fetch(&[branch], Some(&mut fetch_options), None)
            .map_err(|e| PluginLoaderError::load(format!("Failed to fetch: {}", e)))?;

        // Fast-forward merge
        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| PluginLoaderError::load(format!("Failed to find FETCH_HEAD: {}", e)))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| PluginLoaderError::load(format!("Failed to get commit: {}", e)))?;

        // Perform fast-forward
        let (analysis, _) = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| PluginLoaderError::load(format!("Failed to analyze merge: {}", e)))?;

        if analysis.is_fast_forward() {
            let mut reference = repo
                .find_reference(&format!("refs/heads/{}", branch))
                .map_err(|e| PluginLoaderError::load(format!("Failed to find reference: {}", e)))?;

            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(|e| PluginLoaderError::load(format!("Failed to fast-forward: {}", e)))?;

            repo.set_head(&format!("refs/heads/{}", branch))
                .map_err(|e| PluginLoaderError::load(format!("Failed to set HEAD: {}", e)))?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| PluginLoaderError::load(format!("Failed to checkout HEAD: {}", e)))?;
        }

        Ok(())
    }

    /// Initialize submodules
    fn init_submodules(&self, repo: &Repository) -> LoaderResult<()> {
        repo.submodules()
            .map_err(|e| PluginLoaderError::load(format!("Failed to get submodules: {}", e)))?
            .iter_mut()
            .try_for_each(|submodule| {
                submodule
                    .update(true, None)
                    .map_err(|e| PluginLoaderError::load(format!("Failed to update submodule: {}", e)))
            })?;

        Ok(())
    }

    /// Generate a cache key from repository URL and ref
    fn generate_cache_key(&self, url: &str, git_ref: &GitRef) -> String {
        use ring::digest::{Context, SHA256};

        let combined = format!("{}#{}", url, git_ref);
        let mut context = Context::new(&SHA256);
        context.update(combined.as_bytes());
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Clear the Git repository cache
    pub async fn clear_cache(&self) -> LoaderResult<()> {
        if self.config.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.config.cache_dir)
                .await
                .map_err(|e| {
                    PluginLoaderError::fs(format!("Failed to clear cache directory: {}", e))
                })?;
            tokio::fs::create_dir_all(&self.config.cache_dir)
                .await
                .map_err(|e| {
                    PluginLoaderError::fs(format!("Failed to recreate cache directory: {}", e))
                })?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "git-support"))]
pub struct GitPluginLoader;

#[cfg(not(feature = "git-support"))]
impl GitPluginLoader {
    pub fn new(_config: GitPluginConfig) -> LoaderResult<Self> {
        Err(PluginLoaderError::load(
            "Git support not enabled. Recompile with 'git-support' feature",
        ))
    }

    pub async fn clone_from_git(&self, _source: &GitPluginSource) -> LoaderResult<PathBuf> {
        Err(PluginLoaderError::load(
            "Git support not enabled. Recompile with 'git-support' feature",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_ref_parse() {
        assert_eq!(GitRef::parse("v1.0.0"), GitRef::Tag("v1.0.0".to_string()));
        assert_eq!(GitRef::parse("main"), GitRef::Branch("main".to_string()));
        assert_eq!(
            GitRef::parse("abc123def456789012345678901234567890abcd"),
            GitRef::Commit("abc123def456789012345678901234567890abcd".to_string())
        );
        assert_eq!(GitRef::parse(""), GitRef::Default);
    }

    #[test]
    fn test_git_plugin_source_parse() {
        // URL only
        let source = GitPluginSource::parse("https://github.com/user/repo").unwrap();
        assert_eq!(source.url, "https://github.com/user/repo");
        assert_eq!(source.git_ref, GitRef::Default);
        assert_eq!(source.subdirectory, None);

        // URL with tag
        let source = GitPluginSource::parse("https://github.com/user/repo#v1.0.0").unwrap();
        assert_eq!(source.url, "https://github.com/user/repo");
        assert_eq!(source.git_ref, GitRef::Tag("v1.0.0".to_string()));
        assert_eq!(source.subdirectory, None);

        // URL with branch and subdirectory
        let source = GitPluginSource::parse("https://github.com/user/repo#main:plugins/auth").unwrap();
        assert_eq!(source.url, "https://github.com/user/repo");
        assert_eq!(source.git_ref, GitRef::Branch("main".to_string()));
        assert_eq!(source.subdirectory, Some("plugins/auth".to_string()));
    }

    #[test]
    fn test_git_ref_display() {
        assert_eq!(GitRef::Tag("v1.0.0".to_string()).to_string(), "tag:v1.0.0");
        assert_eq!(GitRef::Branch("main".to_string()).to_string(), "branch:main");
        assert_eq!(GitRef::Commit("abc123".to_string()).to_string(), "commit:abc123");
        assert_eq!(GitRef::Default.to_string(), "default");
    }

    #[test]
    fn test_git_plugin_source_display() {
        let source = GitPluginSource {
            url: "https://github.com/user/repo".to_string(),
            git_ref: GitRef::Tag("v1.0.0".to_string()),
            subdirectory: Some("plugins".to_string()),
        };
        assert_eq!(source.to_string(), "https://github.com/user/repo#tag:v1.0.0:plugins");
    }
}
