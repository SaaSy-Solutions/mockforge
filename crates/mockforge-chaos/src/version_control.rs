//! Version control for orchestrations
//!
//! Provides Git-like version control for orchestration configurations with
//! branching, diffing, and history tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::Write;

/// Version control commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub parent_id: Option<String>,
    pub author: String,
    pub email: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub content_hash: String,
    pub metadata: HashMap<String, String>,
}

/// Version control branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub head_commit_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub protected: bool,
}

/// Diff between two versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub from_commit: String,
    pub to_commit: String,
    pub changes: Vec<DiffChange>,
    pub stats: DiffStats,
}

/// Individual change in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffChange {
    pub path: String,
    pub change_type: DiffChangeType,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

/// Type of diff change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiffChangeType {
    Added,
    Modified,
    Deleted,
}

/// Diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
}

/// Version control repository
#[derive(Debug)]
pub struct VersionControlRepository {
    orchestration_id: String,
    storage_path: String,
    branches: HashMap<String, Branch>,
    commits: HashMap<String, Commit>,
    current_branch: String,
}

impl VersionControlRepository {
    /// Create a new repository
    pub fn new(orchestration_id: String, storage_path: String) -> Result<Self, String> {
        // Create storage directory
        fs::create_dir_all(&storage_path).map_err(|e| e.to_string())?;

        let mut repo = Self {
            orchestration_id,
            storage_path: storage_path.clone(),
            branches: HashMap::new(),
            commits: HashMap::new(),
            current_branch: "main".to_string(),
        };

        // Create main branch if it doesn't exist
        if repo.branches.is_empty() {
            let initial_commit = Commit {
                id: Self::generate_commit_id("initial", ""),
                parent_id: None,
                author: "System".to_string(),
                email: "system@mockforge".to_string(),
                message: "Initial commit".to_string(),
                timestamp: Utc::now(),
                content_hash: "".to_string(),
                metadata: HashMap::new(),
            };

            let main_branch = Branch {
                name: "main".to_string(),
                head_commit_id: initial_commit.id.clone(),
                created_at: Utc::now(),
                created_by: "System".to_string(),
                protected: true,
            };

            repo.commits.insert(initial_commit.id.clone(), initial_commit);
            repo.branches.insert("main".to_string(), main_branch);
        }

        // Save repository state
        repo.save()?;

        Ok(repo)
    }

    /// Load repository from disk
    pub fn load(orchestration_id: String, storage_path: String) -> Result<Self, String> {
        let repo_file = Path::new(&storage_path).join("repository.json");

        if !repo_file.exists() {
            return Self::new(orchestration_id, storage_path);
        }

        let content = fs::read_to_string(&repo_file).map_err(|e| e.to_string())?;
        let repo: Self = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        Ok(repo)
    }

    /// Save repository to disk
    fn save(&self) -> Result<(), String> {
        let repo_file = Path::new(&self.storage_path).join("repository.json");
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let mut file = fs::File::create(repo_file).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Create a commit
    pub fn commit(
        &mut self,
        author: String,
        email: String,
        message: String,
        content: &serde_json::Value,
    ) -> Result<Commit, String> {
        let content_hash = Self::hash_content(content);
        let parent_id = self.get_current_head()?;

        let commit = Commit {
            id: Self::generate_commit_id(&author, &message),
            parent_id: Some(parent_id),
            author,
            email,
            message,
            timestamp: Utc::now(),
            content_hash: content_hash.clone(),
            metadata: HashMap::new(),
        };

        // Save content to disk
        let content_file = Path::new(&self.storage_path)
            .join("contents")
            .join(format!("{}.json", content_hash));

        fs::create_dir_all(content_file.parent().unwrap()).map_err(|e| e.to_string())?;
        let content_str = serde_json::to_string_pretty(content).map_err(|e| e.to_string())?;
        let mut file = fs::File::create(content_file).map_err(|e| e.to_string())?;
        file.write_all(content_str.as_bytes()).map_err(|e| e.to_string())?;

        // Update branch head
        if let Some(branch) = self.branches.get_mut(&self.current_branch) {
            branch.head_commit_id = commit.id.clone();
        }

        self.commits.insert(commit.id.clone(), commit.clone());
        self.save()?;

        Ok(commit)
    }

    /// Create a new branch
    pub fn create_branch(&mut self, name: String, from_commit: Option<String>) -> Result<Branch, String> {
        if self.branches.contains_key(&name) {
            return Err(format!("Branch '{}' already exists", name));
        }

        let head_commit_id = from_commit.unwrap_or_else(|| self.get_current_head().unwrap());

        let branch = Branch {
            name: name.clone(),
            head_commit_id,
            created_at: Utc::now(),
            created_by: "user".to_string(),
            protected: false,
        };

        self.branches.insert(name, branch.clone());
        self.save()?;

        Ok(branch)
    }

    /// Switch to a branch
    pub fn checkout(&mut self, branch_name: String) -> Result<(), String> {
        if !self.branches.contains_key(&branch_name) {
            return Err(format!("Branch '{}' does not exist", branch_name));
        }

        self.current_branch = branch_name;
        self.save()?;

        Ok(())
    }

    /// Get diff between two commits
    pub fn diff(&self, from_commit: String, to_commit: String) -> Result<Diff, String> {
        let from_content = self.get_commit_content(&from_commit)?;
        let to_content = self.get_commit_content(&to_commit)?;

        let changes = Self::compute_diff(&from_content, &to_content, "");

        let stats = DiffStats {
            additions: changes.iter().filter(|c| c.change_type == DiffChangeType::Added).count(),
            deletions: changes.iter().filter(|c| c.change_type == DiffChangeType::Deleted).count(),
            modifications: changes.iter().filter(|c| c.change_type == DiffChangeType::Modified).count(),
        };

        Ok(Diff {
            from_commit,
            to_commit,
            changes,
            stats,
        })
    }

    /// Get commit history
    pub fn history(&self, max_count: Option<usize>) -> Result<Vec<Commit>, String> {
        let mut commits = Vec::new();
        let mut current_id = Some(self.get_current_head()?);

        let limit = max_count.unwrap_or(usize::MAX);

        while let Some(id) = current_id {
            if commits.len() >= limit {
                break;
            }

            if let Some(commit) = self.commits.get(&id) {
                commits.push(commit.clone());
                current_id = commit.parent_id.clone();
            } else {
                break;
            }
        }

        Ok(commits)
    }

    /// Get commit content
    pub fn get_commit_content(&self, commit_id: &str) -> Result<serde_json::Value, String> {
        let commit = self.commits.get(commit_id)
            .ok_or_else(|| format!("Commit '{}' not found", commit_id))?;

        let content_file = Path::new(&self.storage_path)
            .join("contents")
            .join(format!("{}.json", commit.content_hash));

        let content = fs::read_to_string(&content_file).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    /// Get current head commit ID
    fn get_current_head(&self) -> Result<String, String> {
        self.branches
            .get(&self.current_branch)
            .map(|b| b.head_commit_id.clone())
            .ok_or_else(|| "Current branch not found".to_string())
    }

    /// Generate commit ID
    fn generate_commit_id(author: &str, message: &str) -> String {
        let data = format!("{}{}{}", author, message, Utc::now().timestamp_millis());
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Hash content
    fn hash_content(content: &serde_json::Value) -> String {
        let content_str = serde_json::to_string(content).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(content_str.as_bytes());
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Compute diff between two JSON values
    fn compute_diff(
        from: &serde_json::Value,
        to: &serde_json::Value,
        path: &str,
    ) -> Vec<DiffChange> {
        let mut changes = Vec::new();

        match (from, to) {
            (serde_json::Value::Object(from_obj), serde_json::Value::Object(to_obj)) => {
                // Check for additions and modifications
                for (key, to_value) in to_obj {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(from_value) = from_obj.get(key) {
                        if from_value != to_value {
                            if from_value.is_object() && to_value.is_object() {
                                changes.extend(Self::compute_diff(from_value, to_value, &new_path));
                            } else {
                                changes.push(DiffChange {
                                    path: new_path,
                                    change_type: DiffChangeType::Modified,
                                    old_value: Some(from_value.clone()),
                                    new_value: Some(to_value.clone()),
                                });
                            }
                        }
                    } else {
                        changes.push(DiffChange {
                            path: new_path,
                            change_type: DiffChangeType::Added,
                            old_value: None,
                            new_value: Some(to_value.clone()),
                        });
                    }
                }

                // Check for deletions
                for (key, from_value) in from_obj {
                    if !to_obj.contains_key(key) {
                        let new_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };

                        changes.push(DiffChange {
                            path: new_path,
                            change_type: DiffChangeType::Deleted,
                            old_value: Some(from_value.clone()),
                            new_value: None,
                        });
                    }
                }
            }
            _ => {
                if from != to {
                    changes.push(DiffChange {
                        path: path.to_string(),
                        change_type: DiffChangeType::Modified,
                        old_value: Some(from.clone()),
                        new_value: Some(to.clone()),
                    });
                }
            }
        }

        changes
    }

    /// Get all branches
    pub fn list_branches(&self) -> Vec<Branch> {
        self.branches.values().cloned().collect()
    }

    /// Get current branch name
    pub fn current_branch(&self) -> &str {
        &self.current_branch
    }
}

impl Serialize for VersionControlRepository {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VersionControlRepository", 5)?;
        state.serialize_field("orchestration_id", &self.orchestration_id)?;
        state.serialize_field("storage_path", &self.storage_path)?;
        state.serialize_field("branches", &self.branches)?;
        state.serialize_field("commits", &self.commits)?;
        state.serialize_field("current_branch", &self.current_branch)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for VersionControlRepository {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RepoData {
            orchestration_id: String,
            storage_path: String,
            branches: HashMap<String, Branch>,
            commits: HashMap<String, Commit>,
            current_branch: String,
        }

        let data = RepoData::deserialize(deserializer)?;

        Ok(VersionControlRepository {
            orchestration_id: data.orchestration_id,
            storage_path: data.storage_path,
            branches: data.branches,
            commits: data.commits,
            current_branch: data.current_branch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_repository_creation() {
        let temp_dir = tempdir().unwrap();
        let repo = VersionControlRepository::new(
            "test-orch".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        ).unwrap();

        assert_eq!(repo.current_branch(), "main");
        assert_eq!(repo.list_branches().len(), 1);
    }

    #[test]
    fn test_commit() {
        let temp_dir = tempdir().unwrap();
        let mut repo = VersionControlRepository::new(
            "test-orch".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        ).unwrap();

        let content = serde_json::json!({
            "name": "Test Orchestration",
            "steps": []
        });

        let commit = repo.commit(
            "Test User".to_string(),
            "test@example.com".to_string(),
            "Initial orchestration".to_string(),
            &content,
        ).unwrap();

        assert_eq!(commit.author, "Test User");
        assert_eq!(repo.history(None).unwrap().len(), 2); // initial + new commit
    }

    #[test]
    fn test_branching() {
        let temp_dir = tempdir().unwrap();
        let mut repo = VersionControlRepository::new(
            "test-orch".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        ).unwrap();

        repo.create_branch("feature-1".to_string(), None).unwrap();
        assert_eq!(repo.list_branches().len(), 2);

        repo.checkout("feature-1".to_string()).unwrap();
        assert_eq!(repo.current_branch(), "feature-1");
    }

    #[test]
    fn test_diff() {
        let temp_dir = tempdir().unwrap();
        let mut repo = VersionControlRepository::new(
            "test-orch".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        ).unwrap();

        let content1 = serde_json::json!({
            "name": "Test Orchestration",
            "steps": []
        });

        let commit1 = repo.commit(
            "User".to_string(),
            "user@example.com".to_string(),
            "First commit".to_string(),
            &content1,
        ).unwrap();

        let content2 = serde_json::json!({
            "name": "Test Orchestration Updated",
            "steps": [{"name": "step1"}]
        });

        let commit2 = repo.commit(
            "User".to_string(),
            "user@example.com".to_string(),
            "Second commit".to_string(),
            &content2,
        ).unwrap();

        let diff = repo.diff(commit1.id, commit2.id).unwrap();
        assert!(diff.stats.modifications > 0 || diff.stats.additions > 0);
    }
}
