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
        let source_ws_id_str = source_workspace_id.to_string();
        let target_ws_id_str = target_workspace_id.to_string();
        let fork = sqlx::query!(
            r#"
            SELECT fork_point_commit_id
            FROM workspace_forks
            WHERE source_workspace_id = ? AND forked_workspace_id = ?
            "#,
            source_ws_id_str,
            target_ws_id_str
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(fork) = fork {
            if let Some(commit_id_str) = fork.fork_point_commit_id {
                if let Ok(commit_id) = Uuid::parse_str(&commit_id_str) {
                    return Ok(Some(commit_id));
                }
            }
        }

        // Check if source is a fork of target
        let target_ws_id_str2 = target_workspace_id.to_string();
        let source_ws_id_str2 = source_workspace_id.to_string();
        let fork = sqlx::query!(
            r#"
            SELECT fork_point_commit_id
            FROM workspace_forks
            WHERE source_workspace_id = ? AND forked_workspace_id = ?
            "#,
            target_ws_id_str2,
            source_ws_id_str2
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(fork) = fork {
            if let Some(commit_id_str) = fork.fork_point_commit_id {
                if let Ok(commit_id) = Uuid::parse_str(&commit_id_str) {
                    return Ok(Some(commit_id));
                }
            }
        }

        // TODO: Implement more sophisticated common ancestor finding
        // For now, return None if no fork relationship exists
        Ok(None)
    }

    /// Perform a three-way merge between two workspaces
    ///
    /// Merges changes from source_workspace into target_workspace.
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
        let merge_id_str = merge.id.to_string();
        let source_ws_id_str = merge.source_workspace_id.to_string();
        let target_ws_id_str = merge.target_workspace_id.to_string();
        let base_commit_id_str = merge.base_commit_id.to_string();
        let source_commit_id_str = merge.source_commit_id.to_string();
        let target_commit_id_str = merge.target_commit_id.to_string();
        let merge_commit_id_str = merge.merge_commit_id.map(|id| id.to_string());
        let status_str = serde_json::to_string(&merge.status)?;
        let conflict_data_str =
            merge.conflict_data.as_ref().map(|v| serde_json::to_string(v)).transpose()?;
        let merged_by_str = merge.merged_by.map(|id| id.to_string());
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
            merge_id_str,
            source_ws_id_str,
            target_ws_id_str,
            base_commit_id_str,
            source_commit_id_str,
            target_commit_id_str,
            merge_commit_id_str,
            status_str,
            conflict_data_str,
            merged_by_str,
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

            // Conflict: both changed differently
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

            // Handle objects recursively
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
                            format!("{}.{}", path, key)
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
                if base_arr != source_arr || base_arr != target_arr {
                    if source_arr != target_arr {
                        conflicts.push(MergeConflict {
                            path: path.to_string(),
                            base_value: Some(base.clone()),
                            source_value: Some(source.clone()),
                            target_value: Some(target.clone()),
                            conflict_type: ConflictType::Modified,
                        });
                    }
                }
            }

            _ => {
                // For other types, use the simple merge logic above
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
        .ok_or_else(|| CollabError::Internal(format!("Merge not found: {}", merge_id)))?;

        Ok(WorkspaceMerge {
            id: Uuid::parse_str(&row.id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            source_workspace_id: Uuid::parse_str(&row.source_workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            target_workspace_id: Uuid::parse_str(&row.target_workspace_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            base_commit_id: Uuid::parse_str(&row.base_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            source_commit_id: Uuid::parse_str(&row.source_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            target_commit_id: Uuid::parse_str(&row.target_commit_id)
                .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
            merge_commit_id: row.merge_commit_id.and_then(|s| Uuid::parse_str(&s).ok()),
            status: serde_json::from_str(&row.status)
                .map_err(|e| CollabError::Internal(format!("Invalid status: {}", e)))?,
            conflict_data: row.conflict_data.and_then(|s| serde_json::from_str(&s).ok()),
            merged_by: row.merged_by.and_then(|s| Uuid::parse_str(&s).ok()),
            merged_at: row
                .merged_at
                .map(|s| {
                    chrono::DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))
                })
                .transpose()?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc),
        })
    }

    /// List merges for a workspace
    pub async fn list_merges(&self, workspace_id: Uuid) -> Result<Vec<WorkspaceMerge>> {
        let workspace_id_str = workspace_id.to_string();
        let rows = sqlx::query!(
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
            WHERE source_workspace_id = ? OR target_workspace_id = ?
            ORDER BY created_at DESC
            "#,
            workspace_id_str,
            workspace_id_str
        )
        .fetch_all(&self.db)
        .await?;

        let merges: Result<Vec<WorkspaceMerge>> = rows
            .into_iter()
            .map(|row| {
                Ok(WorkspaceMerge {
                    id: Uuid::parse_str(&row.id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    source_workspace_id: Uuid::parse_str(&row.source_workspace_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    target_workspace_id: Uuid::parse_str(&row.target_workspace_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    base_commit_id: Uuid::parse_str(&row.base_commit_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    source_commit_id: Uuid::parse_str(&row.source_commit_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    target_commit_id: Uuid::parse_str(&row.target_commit_id)
                        .map_err(|e| CollabError::Internal(format!("Invalid UUID: {}", e)))?,
                    merge_commit_id: row.merge_commit_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    status: serde_json::from_str(&row.status)
                        .map_err(|e| CollabError::Internal(format!("Invalid status: {}", e)))?,
                    conflict_data: row.conflict_data.and_then(|s| serde_json::from_str(&s).ok()),
                    merged_by: row.merged_by.and_then(|s| Uuid::parse_str(&s).ok()),
                    merged_at: row
                        .merged_at
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .map_err(|e| {
                                    CollabError::Internal(format!("Invalid timestamp: {}", e))
                                })
                        })
                        .transpose()?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                        .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {}", e)))?
                        .with_timezone(&chrono::Utc),
                })
            })
            .collect();
        let merges = merges?;

        Ok(merges)
    }
}
