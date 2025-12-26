//! Workspace merge operations and conflict resolution

use crate::error::{CollabError, Result};
use crate::history::VersionControl;
use crate::models::{ConflictType, MergeConflict, MergeStatus, WorkspaceMerge};
use chrono::Utc;
use serde_json::Value;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// Merge service for handling workspace merges
pub struct MergeService {
    db: Pool<Sqlite>,
    version_control: VersionControl,
}

impl MergeService {
    /// Create a new merge service
    #[must_use]
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db: db.clone(),
            version_control: VersionControl::new(db),
        }
    }

    /// Find the common ancestor commit between two workspaces
    ///
    /// This uses a simple approach: find the fork point if one exists,
    /// otherwise find the earliest common commit in both histories.
    pub async fn find_common_ancestor(
        &self,
        source_workspace_id: Uuid,
        target_workspace_id: Uuid,
    ) -> Result<Option<Uuid>> {
        // First, check if target is a fork of source
        let fork = sqlx::query!(
            r#"
            SELECT fork_point_commit_id as "fork_point_commit_id: Uuid"
            FROM workspace_forks
            WHERE source_workspace_id = ? AND forked_workspace_id = ?
            "#,
            source_workspace_id,
            target_workspace_id
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(fork) = fork {
            if let Some(commit_id) = fork.fork_point_commit_id {
                return Ok(Some(commit_id));
            }
        }

        // Check if source is a fork of target
        let fork = sqlx::query!(
            r#"
            SELECT fork_point_commit_id as "fork_point_commit_id: Uuid"
            FROM workspace_forks
            WHERE source_workspace_id = ? AND forked_workspace_id = ?
            "#,
            target_workspace_id,
            source_workspace_id
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(fork) = fork {
            if let Some(commit_id) = fork.fork_point_commit_id {
                return Ok(Some(commit_id));
            }
        }

        // Implement sophisticated common ancestor finding by walking commit history
        // This finds the Lowest Common Ancestor (LCA) by walking both commit histories
        let source_commits =
            self.version_control.get_history(source_workspace_id, Some(1000)).await?;
        let target_commits =
            self.version_control.get_history(target_workspace_id, Some(1000)).await?;

        // Build commit ID sets for fast lookup
        let source_commit_ids: std::collections::HashSet<Uuid> =
            source_commits.iter().map(|c| c.id).collect();
        let target_commit_ids: std::collections::HashSet<Uuid> =
            target_commits.iter().map(|c| c.id).collect();

        // Find the first commit that appears in both histories (LCA)
        // Walk from most recent to oldest in source history
        for source_commit in &source_commits {
            if target_commit_ids.contains(&source_commit.id) {
                return Ok(Some(source_commit.id));
            }
        }

        // If no direct match, try walking parent chains
        // Get the latest commits
        if let (Some(source_latest), Some(target_latest)) =
            (source_commits.first(), target_commits.first())
        {
            // Build ancestor sets by walking parent chains
            let source_ancestors = self.build_ancestor_set(source_latest.id).await?;
            let target_ancestors = self.build_ancestor_set(target_latest.id).await?;

            // Find the first common ancestor
            for ancestor in &source_ancestors {
                if target_ancestors.contains(ancestor) {
                    return Ok(Some(*ancestor));
                }
            }
        }

        // No common ancestor found
        Ok(None)
    }

    /// Perform a three-way merge between two workspaces
    ///
    /// Merges changes from `source_workspace` into `target_workspace`.
    /// Returns the merged state and any conflicts.
    pub async fn merge_workspaces(
        &self,
        source_workspace_id: Uuid,
        target_workspace_id: Uuid,
        user_id: Uuid,
    ) -> Result<(Value, Vec<MergeConflict>)> {
        // Get latest commits for both workspaces
        let source_commit =
            self.version_control.get_latest_commit(source_workspace_id).await?.ok_or_else(
                || CollabError::Internal("Source workspace has no commits".to_string()),
            )?;

        let target_commit =
            self.version_control.get_latest_commit(target_workspace_id).await?.ok_or_else(
                || CollabError::Internal("Target workspace has no commits".to_string()),
            )?;

        // Find common ancestor
        let base_commit_id = self
            .find_common_ancestor(source_workspace_id, target_workspace_id)
            .await?
            .ok_or_else(|| {
                CollabError::Internal(
                    "Cannot find common ancestor. Workspaces must be related by fork.".to_string(),
                )
            })?;

        let base_commit = self.version_control.get_commit(base_commit_id).await?;

        // Perform three-way merge
        let (merged_state, conflicts) = self.three_way_merge(
            &base_commit.snapshot,
            &source_commit.snapshot,
            &target_commit.snapshot,
        )?;

        // Create merge record
        let mut merge = WorkspaceMerge::new(
            source_workspace_id,
            target_workspace_id,
            base_commit_id,
            source_commit.id,
            target_commit.id,
        );

        if conflicts.is_empty() {
            merge.status = MergeStatus::Completed;
        } else {
            merge.status = MergeStatus::Conflict;
            merge.conflict_data = Some(serde_json::to_value(&conflicts)?);
        }

        // Save merge record
        let status_str = match merge.status {
            MergeStatus::Pending => "pending",
            MergeStatus::InProgress => "in_progress",
            MergeStatus::Completed => "completed",
            MergeStatus::Conflict => "conflict",
            MergeStatus::Cancelled => "cancelled",
        };
        let conflict_data_str =
            merge.conflict_data.as_ref().map(serde_json::to_string).transpose()?;
        let merged_at_str = merge.merged_at.map(|dt| dt.to_rfc3339());
        let created_at_str = merge.created_at.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO workspace_merges (
                id, source_workspace_id, target_workspace_id,
                base_commit_id, source_commit_id, target_commit_id,
                merge_commit_id, status, conflict_data, merged_by, merged_at, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            merge.id,
            merge.source_workspace_id,
            merge.target_workspace_id,
            merge.base_commit_id,
            merge.source_commit_id,
            merge.target_commit_id,
            merge.merge_commit_id,
            status_str,
            conflict_data_str,
            merge.merged_by,
            merged_at_str,
            created_at_str
        )
        .execute(&self.db)
        .await?;

        Ok((merged_state, conflicts))
    }

    /// Perform a three-way merge on JSON values
    ///
    /// Implements a simple three-way merge algorithm:
    /// - If base == source, use target
    /// - If base == target, use source
    /// - If source == target, use either
    /// - Otherwise, conflict
    fn three_way_merge(
        &self,
        base: &Value,
        source: &Value,
        target: &Value,
    ) -> Result<(Value, Vec<MergeConflict>)> {
        let mut merged = target.clone();
        let mut conflicts = Vec::new();

        self.merge_value("", base, source, target, &mut merged, &mut conflicts)?;

        Ok((merged, conflicts))
    }

    /// Recursively merge JSON values
    fn merge_value(
        &self,
        path: &str,
        base: &Value,
        source: &Value,
        target: &Value,
        merged: &mut Value,
        conflicts: &mut Vec<MergeConflict>,
    ) -> Result<()> {
        match (base, source, target) {
            // No changes: base == source == target
            (b, s, t) if b == s && s == t => {
                // Already correct, do nothing
            }

            // Only target changed: base == source, target differs
            (b, s, t) if b == s && t != b => {
                // Target is already in merged, keep it
            }

            // Only source changed: base == target, source differs
            (b, s, t) if b == t && s != b => {
                *merged = source.clone();
            }

            // Both changed the same way: source == target, both differ from base
            (b, s, t) if s == t && s != b => {
                *merged = source.clone();
            }

            // Handle objects recursively - MUST come before generic conflict handler
            (Value::Object(base_obj), Value::Object(source_obj), Value::Object(target_obj)) => {
                if let Value::Object(merged_obj) = merged {
                    // Get all keys from all three objects
                    let all_keys: std::collections::HashSet<_> =
                        base_obj.keys().chain(source_obj.keys()).chain(target_obj.keys()).collect();

                    for key in all_keys {
                        let base_val = base_obj.get(key);
                        let source_val = source_obj.get(key);
                        let target_val = target_obj.get(key);

                        let new_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        };

                        match (base_val, source_val, target_val) {
                            // Key only in source
                            (None, Some(s), None) => {
                                merged_obj.insert(key.clone(), s.clone());
                            }
                            // Key only in target
                            (None, None, Some(t)) => {
                                merged_obj.insert(key.clone(), t.clone());
                            }
                            // Key in both source and target (but not base) - conflict
                            (None, Some(s), Some(t)) if s != t => {
                                conflicts.push(MergeConflict {
                                    path: new_path.clone(),
                                    base_value: None,
                                    source_value: Some(s.clone()),
                                    target_value: Some(t.clone()),
                                    conflict_type: ConflictType::BothAdded,
                                });
                                // Keep target value
                            }
                            // Key in both, same value
                            (None, Some(s), Some(t)) if s == t => {
                                merged_obj.insert(key.clone(), s.clone());
                            }
                            // Key exists in all three - recurse
                            (Some(b), Some(s), Some(t)) => {
                                if let Some(merged_val) = merged_obj.get_mut(key) {
                                    self.merge_value(&new_path, b, s, t, merged_val, conflicts)?;
                                }
                            }
                            // Key deleted in source
                            (Some(b), None, Some(t)) if b == t => {
                                merged_obj.remove(key);
                            }
                            // Key deleted in target
                            (Some(b), Some(s), None) if b == s => {
                                merged_obj.remove(key);
                            }
                            // Key deleted in source, modified in target - conflict
                            (Some(b), None, Some(_t)) => {
                                conflicts.push(MergeConflict {
                                    path: new_path.clone(),
                                    base_value: Some(b.clone()),
                                    source_value: source_val.cloned(),
                                    target_value: target_val.cloned(),
                                    conflict_type: ConflictType::DeletedModified,
                                });
                            }
                            // Key deleted in target, modified in source - conflict
                            (Some(b), Some(_s), None) => {
                                conflicts.push(MergeConflict {
                                    path: new_path.clone(),
                                    base_value: Some(b.clone()),
                                    source_value: source_val.cloned(),
                                    target_value: target_val.cloned(),
                                    conflict_type: ConflictType::DeletedModified,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Handle arrays - simple approach: use target, mark as conflict if different
            (Value::Array(base_arr), Value::Array(source_arr), Value::Array(target_arr)) => {
                if (base_arr != source_arr || base_arr != target_arr) && source_arr != target_arr {
                    conflicts.push(MergeConflict {
                        path: path.to_string(),
                        base_value: Some(base.clone()),
                        source_value: Some(source.clone()),
                        target_value: Some(target.clone()),
                        conflict_type: ConflictType::Modified,
                    });
                }
            }

            // Conflict: both changed differently (catch-all for non-Object/Array types)
            (b, s, t) if s != t && s != b && t != b => {
                conflicts.push(MergeConflict {
                    path: path.to_string(),
                    base_value: Some(b.clone()),
                    source_value: Some(s.clone()),
                    target_value: Some(t.clone()),
                    conflict_type: ConflictType::Modified,
                });
                // Keep target value for now (user can resolve later)
            }

            _ => {
                // For other types that don't match above patterns
            }
        }

        Ok(())
    }

    /// Complete a merge by creating a merge commit
    pub async fn complete_merge(
        &self,
        merge_id: Uuid,
        user_id: Uuid,
        resolved_state: Value,
        message: String,
    ) -> Result<Uuid> {
        // Get merge record
        let merge = self.get_merge(merge_id).await?;

        if merge.status != MergeStatus::Conflict && merge.status != MergeStatus::Pending {
            return Err(CollabError::InvalidInput(
                "Merge is not in a state that can be completed".to_string(),
            ));
        }

        // Create merge commit
        let merge_commit = self
            .version_control
            .create_commit(
                merge.target_workspace_id,
                user_id,
                message,
                Some(merge.target_commit_id),
                // Version will be incremented by workspace service
                0, // Placeholder, will be set properly
                resolved_state.clone(),
                serde_json::json!({
                    "type": "merge",
                    "source_workspace_id": merge.source_workspace_id,
                    "source_commit_id": merge.source_commit_id,
                }),
            )
            .await?;

        // Update merge record
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE workspace_merges
            SET merge_commit_id = ?, status = ?, merged_by = ?, merged_at = ?
            WHERE id = ?
            "#,
            merge_commit.id,
            MergeStatus::Completed,
            user_id,
            now,
            merge_id
        )
        .execute(&self.db)
        .await?;

        Ok(merge_commit.id)
    }

    /// Get a merge by ID
    pub async fn get_merge(&self, merge_id: Uuid) -> Result<WorkspaceMerge> {
        let merge_id_str = merge_id.to_string();
        let row = sqlx::query!(
            r#"
            SELECT
                id,
                source_workspace_id,
                target_workspace_id,
                base_commit_id,
                source_commit_id,
                target_commit_id,
                merge_commit_id,
                status,
                conflict_data,
                merged_by,
                merged_at,
                created_at
            FROM workspace_merges
            WHERE id = ?
            "#,
            merge_id_str
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::Internal(format!("Merge not found: {merge_id}")))?;

        Ok(WorkspaceMerge {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            source_workspace_id: Uuid::parse_str(&row.source_workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            target_workspace_id: Uuid::parse_str(&row.target_workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            base_commit_id: Uuid::parse_str(&row.base_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            source_commit_id: Uuid::parse_str(&row.source_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            target_commit_id: Uuid::parse_str(&row.target_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {e}")))?,
            merge_commit_id: row.merge_commit_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            status: serde_json::from_str(&row.status)
                .map_err(|e| CollabError::Internal(format!("Invalid status: {e}")))?,
            conflict_data: row.conflict_data.as_ref().and_then(|s| serde_json::from_str(s).ok()),
            merged_by: row.merged_by.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            merged_at: row
                .merged_at
                .as_ref()
                .map(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))
                })
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                .with_timezone(&Utc),
        })
    }

    /// List merges for a workspace
    pub async fn list_merges(&self, workspace_id: Uuid) -> Result<Vec<WorkspaceMerge>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id: Uuid",
                source_workspace_id as "source_workspace_id: Uuid",
                target_workspace_id as "target_workspace_id: Uuid",
                base_commit_id as "base_commit_id: Uuid",
                source_commit_id as "source_commit_id: Uuid",
                target_commit_id as "target_commit_id: Uuid",
                merge_commit_id as "merge_commit_id: Uuid",
                status,
                conflict_data,
                merged_by as "merged_by: Uuid",
                merged_at,
                created_at
            FROM workspace_merges
            WHERE source_workspace_id = ? OR target_workspace_id = ?
            ORDER BY created_at DESC
            "#,
            workspace_id,
            workspace_id
        )
        .fetch_all(&self.db)
        .await?;

        let merges: Result<Vec<WorkspaceMerge>> = rows
            .into_iter()
            .map(|row| {
                let status = match row.status.as_str() {
                    "pending" => MergeStatus::Pending,
                    "in_progress" => MergeStatus::InProgress,
                    "completed" => MergeStatus::Completed,
                    "conflict" => MergeStatus::Conflict,
                    "cancelled" => MergeStatus::Cancelled,
                    other => return Err(CollabError::Internal(format!("Invalid status: {other}"))),
                };
                Ok(WorkspaceMerge {
                    id: row.id,
                    source_workspace_id: row.source_workspace_id,
                    target_workspace_id: row.target_workspace_id,
                    base_commit_id: row.base_commit_id,
                    source_commit_id: row.source_commit_id,
                    target_commit_id: row.target_commit_id,
                    merge_commit_id: row.merge_commit_id,
                    status,
                    conflict_data: row
                        .conflict_data
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok()),
                    merged_by: row.merged_by,
                    merged_at: row
                        .merged_at
                        .as_ref()
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .map_err(|e| {
                                    CollabError::Internal(format!("Invalid timestamp: {e}"))
                                })
                        })
                        .transpose()?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                        .with_timezone(&Utc),
                })
            })
            .collect();
        let merges = merges?;

        Ok(merges)
    }

    /// Build a set of all ancestor commit IDs by walking the parent chain
    async fn build_ancestor_set(&self, commit_id: Uuid) -> Result<std::collections::HashSet<Uuid>> {
        let mut ancestors = std::collections::HashSet::new();
        let mut current_id = Some(commit_id);
        let mut visited = std::collections::HashSet::new();

        // Walk the parent chain up to a reasonable depth (prevent infinite loops)
        let max_depth = 1000;
        let mut depth = 0;

        while let Some(id) = current_id {
            if visited.contains(&id) || depth > max_depth {
                break; // Cycle detected or max depth reached
            }
            visited.insert(id);
            ancestors.insert(id);

            // Get the commit and move to parent
            match self.version_control.get_commit(id).await {
                Ok(commit) => {
                    current_id = commit.parent_id;
                    depth += 1;
                }
                Err(_) => break, // Commit not found, stop walking
            }
        }

        Ok(ancestors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create workspace_forks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_forks (
                id TEXT PRIMARY KEY,
                source_workspace_id TEXT NOT NULL,
                forked_workspace_id TEXT NOT NULL,
                fork_point_commit_id TEXT,
                created_at TEXT NOT NULL,
                created_by TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create workspace_merges table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_merges (
                id TEXT PRIMARY KEY,
                source_workspace_id TEXT NOT NULL,
                target_workspace_id TEXT NOT NULL,
                base_commit_id TEXT NOT NULL,
                source_commit_id TEXT NOT NULL,
                target_commit_id TEXT NOT NULL,
                merge_commit_id TEXT,
                status TEXT NOT NULL,
                conflict_data TEXT,
                merged_by TEXT,
                merged_at TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create commits table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS commits (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                message TEXT NOT NULL,
                parent_id TEXT,
                version INTEGER NOT NULL,
                snapshot TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_merge_service_new() {
        let pool = setup_test_db().await;
        let service = MergeService::new(pool);

        // Just verify service is created
        // We can't test much without real commits
        assert!(true);
    }

    #[test]
    fn test_three_way_merge_no_changes() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({"key": "value"});
        let source = json!({"key": "value"});
        let target = json!({"key": "value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, target);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_source_change() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({"key": "value"});
        let source = json!({"key": "new_value"});
        let target = json!({"key": "value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, source);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_target_change() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({"key": "value"});
        let source = json!({"key": "value"});
        let target = json!({"key": "new_value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, target);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_both_changed_same() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({"key": "value"});
        let source = json!({"key": "new_value"});
        let target = json!({"key": "new_value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, source);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_conflict() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({"key": "value"});
        let source = json!({"key": "source_value"});
        let target = json!({"key": "target_value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, target); // Target is kept on conflict
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].path, "key");
        assert_eq!(conflicts[0].conflict_type, ConflictType::Modified);
    }

    #[test]
    fn test_three_way_merge_object_add_source() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({});
        let source = json!({"new_key": "value"});
        let target = json!({});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged.get("new_key"), Some(&json!("value")));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_object_add_target() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({});
        let source = json!({});
        let target = json!({"new_key": "value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged.get("new_key"), Some(&json!("value")));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_both_added_different() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({});
        let source = json!({"key": "source_value"});
        let target = json!({"key": "target_value"});

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::BothAdded);
    }

    #[test]
    fn test_three_way_merge_nested_objects() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!({
            "parent": {
                "child": "value"
            }
        });
        let source = json!({
            "parent": {
                "child": "new_value"
            }
        });
        let target = json!({
            "parent": {
                "child": "value"
            }
        });

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged["parent"]["child"], json!("new_value"));
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_arrays_no_conflict() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!([1, 2, 3]);
        let source = json!([1, 2, 3]);
        let target = json!([1, 2, 3]);

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, target);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_three_way_merge_arrays_conflict() {
        let pool_fut = setup_test_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = rt.block_on(pool_fut);
        let service = MergeService::new(pool);

        let base = json!([1, 2, 3]);
        let source = json!([1, 2, 4]);
        let target = json!([1, 2, 5]);

        let result = service.three_way_merge(&base, &source, &target);
        assert!(result.is_ok());

        let (merged, conflicts) = result.unwrap();
        assert_eq!(merged, target);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_workspace_merge_new() {
        let source_ws = Uuid::new_v4();
        let target_ws = Uuid::new_v4();
        let base_commit = Uuid::new_v4();
        let source_commit = Uuid::new_v4();
        let target_commit = Uuid::new_v4();

        let merge =
            WorkspaceMerge::new(source_ws, target_ws, base_commit, source_commit, target_commit);

        assert_eq!(merge.source_workspace_id, source_ws);
        assert_eq!(merge.target_workspace_id, target_ws);
        assert_eq!(merge.base_commit_id, base_commit);
        assert_eq!(merge.source_commit_id, source_commit);
        assert_eq!(merge.target_commit_id, target_commit);
        assert_eq!(merge.status, MergeStatus::Pending);
        assert!(merge.merge_commit_id.is_none());
    }

    #[test]
    fn test_merge_conflict_types() {
        assert_eq!(ConflictType::Modified, ConflictType::Modified);
        assert_eq!(ConflictType::BothAdded, ConflictType::BothAdded);
        assert_eq!(ConflictType::DeletedModified, ConflictType::DeletedModified);

        assert_ne!(ConflictType::Modified, ConflictType::BothAdded);
    }

    #[test]
    fn test_merge_status_equality() {
        assert_eq!(MergeStatus::Pending, MergeStatus::Pending);
        assert_eq!(MergeStatus::Conflict, MergeStatus::Conflict);
        assert_eq!(MergeStatus::Completed, MergeStatus::Completed);

        assert_ne!(MergeStatus::Pending, MergeStatus::Completed);
    }
}
