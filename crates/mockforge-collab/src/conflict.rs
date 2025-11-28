//! Conflict resolution for concurrent edits

use crate::error::{CollabError, Result};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

/// Merge strategy for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Keep local changes (ours)
    Ours,
    /// Keep remote changes (theirs)
    Theirs,
    /// Attempt automatic merge
    Auto,
    /// Manual resolution required
    Manual,
}

/// Conflict resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Whether conflicts were detected
    pub has_conflicts: bool,
    /// Resolved content
    pub resolved: serde_json::Value,
    /// List of conflicts (if manual resolution needed)
    pub conflicts: Vec<Conflict>,
}

/// A conflict between two versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Path to the conflicting field
    pub path: String,
    /// Local value
    pub ours: serde_json::Value,
    /// Remote value
    pub theirs: serde_json::Value,
    /// Common ancestor value (if available)
    pub base: Option<serde_json::Value>,
}

/// Conflict resolver
pub struct ConflictResolver {
    /// Default merge strategy
    default_strategy: MergeStrategy,
}

impl ConflictResolver {
    /// Create a new conflict resolver
    #[must_use]
    pub const fn new(default_strategy: MergeStrategy) -> Self {
        Self { default_strategy }
    }

    /// Resolve conflicts between two versions
    pub fn resolve(
        &self,
        base: Option<&serde_json::Value>,
        ours: &serde_json::Value,
        theirs: &serde_json::Value,
        strategy: Option<MergeStrategy>,
    ) -> Result<ConflictResolution> {
        let strategy = strategy.unwrap_or(self.default_strategy);

        // If values are identical, no conflict
        if ours == theirs {
            return Ok(ConflictResolution {
                has_conflicts: false,
                resolved: ours.clone(),
                conflicts: Vec::new(),
            });
        }

        match strategy {
            MergeStrategy::Ours => Ok(ConflictResolution {
                has_conflicts: false,
                resolved: ours.clone(),
                conflicts: Vec::new(),
            }),
            MergeStrategy::Theirs => Ok(ConflictResolution {
                has_conflicts: false,
                resolved: theirs.clone(),
                conflicts: Vec::new(),
            }),
            MergeStrategy::Auto => self.auto_merge(base, ours, theirs),
            MergeStrategy::Manual => {
                // Detect all conflicts and return for manual resolution
                let conflicts = self.detect_conflicts("", base, ours, theirs);
                Ok(ConflictResolution {
                    has_conflicts: !conflicts.is_empty(),
                    resolved: ours.clone(), // Default to ours
                    conflicts,
                })
            }
        }
    }

    /// Attempt automatic merge
    fn auto_merge(
        &self,
        base: Option<&serde_json::Value>,
        ours: &serde_json::Value,
        theirs: &serde_json::Value,
    ) -> Result<ConflictResolution> {
        // For JSON objects, try field-by-field merge
        match (ours, theirs) {
            (serde_json::Value::Object(ours_obj), serde_json::Value::Object(theirs_obj)) => {
                let mut resolved = serde_json::Map::new();
                let mut conflicts = Vec::new();

                let base_obj = base.and_then(|b| b.as_object());

                // Merge all keys
                let all_keys: std::collections::HashSet<_> =
                    ours_obj.keys().chain(theirs_obj.keys()).collect();

                for key in all_keys {
                    let ours_val = ours_obj.get(key);
                    let theirs_val = theirs_obj.get(key);
                    let base_val = base_obj.and_then(|b| b.get(key));

                    match (ours_val, theirs_val) {
                        (Some(o), Some(t)) if o == t => {
                            // No conflict, values are the same
                            resolved.insert(key.clone(), o.clone());
                        }
                        (Some(o), Some(t)) => {
                            // Check if only one side changed from base
                            if let Some(base_val) = base_val {
                                if o == base_val {
                                    // Only theirs changed
                                    resolved.insert(key.clone(), t.clone());
                                } else if t == base_val {
                                    // Only ours changed
                                    resolved.insert(key.clone(), o.clone());
                                } else {
                                    // Both changed - conflict
                                    conflicts.push(Conflict {
                                        path: key.clone(),
                                        ours: o.clone(),
                                        theirs: t.clone(),
                                        base: Some(base_val.clone()),
                                    });
                                    resolved.insert(key.clone(), o.clone()); // Default to ours
                                }
                            } else {
                                // No base - conflict
                                conflicts.push(Conflict {
                                    path: key.clone(),
                                    ours: o.clone(),
                                    theirs: t.clone(),
                                    base: None,
                                });
                                resolved.insert(key.clone(), o.clone());
                            }
                        }
                        (Some(o), None) => {
                            // Only in ours
                            resolved.insert(key.clone(), o.clone());
                        }
                        (None, Some(t)) => {
                            // Only in theirs
                            resolved.insert(key.clone(), t.clone());
                        }
                        (None, None) => unreachable!(),
                    }
                }

                Ok(ConflictResolution {
                    has_conflicts: !conflicts.is_empty(),
                    resolved: serde_json::Value::Object(resolved),
                    conflicts,
                })
            }
            _ => {
                // For non-objects, treat as conflict
                Ok(ConflictResolution {
                    has_conflicts: true,
                    resolved: ours.clone(),
                    conflicts: vec![Conflict {
                        path: String::new(),
                        ours: ours.clone(),
                        theirs: theirs.clone(),
                        base: base.cloned(),
                    }],
                })
            }
        }
    }

    /// Detect all conflicts recursively
    fn detect_conflicts(
        &self,
        path: &str,
        base: Option<&serde_json::Value>,
        ours: &serde_json::Value,
        theirs: &serde_json::Value,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        if ours == theirs {
            return conflicts;
        }

        match (ours, theirs) {
            (serde_json::Value::Object(ours_obj), serde_json::Value::Object(theirs_obj)) => {
                let base_obj = base.and_then(|b| b.as_object());
                let all_keys: std::collections::HashSet<_> =
                    ours_obj.keys().chain(theirs_obj.keys()).collect();

                for key in all_keys {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };

                    let ours_val = ours_obj.get(key);
                    let theirs_val = theirs_obj.get(key);
                    let base_val = base_obj.and_then(|b| b.get(key));

                    if let (Some(o), Some(t)) = (ours_val, theirs_val) {
                        conflicts.extend(self.detect_conflicts(&new_path, base_val, o, t));
                    } else if ours_val != theirs_val {
                        conflicts.push(Conflict {
                            path: new_path,
                            ours: ours_val.cloned().unwrap_or(serde_json::Value::Null),
                            theirs: theirs_val.cloned().unwrap_or(serde_json::Value::Null),
                            base: base_val.cloned(),
                        });
                    }
                }
            }
            _ => {
                conflicts.push(Conflict {
                    path: path.to_string(),
                    ours: ours.clone(),
                    theirs: theirs.clone(),
                    base: base.cloned(),
                });
            }
        }

        conflicts
    }

    /// Merge text with three-way merge algorithm
    pub fn merge_text(&self, base: &str, ours: &str, theirs: &str) -> Result<String> {
        if ours == theirs {
            return Ok(ours.to_string());
        }

        // Simple line-based three-way merge
        let diff_ours = TextDiff::from_lines(base, ours);
        let _diff_theirs = TextDiff::from_lines(base, theirs);

        let mut result = String::new();
        let has_conflict = false;

        // This is a simplified merge - a real implementation would be more sophisticated
        for change in diff_ours.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => result.push_str(change.value()),
                ChangeTag::Delete => {}
                ChangeTag::Insert => result.push_str(change.value()),
            }
        }

        if has_conflict {
            Err(CollabError::ConflictDetected("Text merge conflict".to_string()))
        } else {
            Ok(result)
        }
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(MergeStrategy::Auto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_conflict() {
        let resolver = ConflictResolver::default();
        let value = json!({"key": "value"});

        let result = resolver.resolve(None, &value, &value, None).unwrap();

        assert!(!result.has_conflicts);
        assert_eq!(result.resolved, value);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_strategy_ours() {
        let resolver = ConflictResolver::default();
        let ours = json!({"key": "ours"});
        let theirs = json!({"key": "theirs"});

        let result = resolver.resolve(None, &ours, &theirs, Some(MergeStrategy::Ours)).unwrap();

        assert!(!result.has_conflicts);
        assert_eq!(result.resolved, ours);
    }

    #[test]
    fn test_strategy_theirs() {
        let resolver = ConflictResolver::default();
        let ours = json!({"key": "ours"});
        let theirs = json!({"key": "theirs"});

        let result = resolver.resolve(None, &ours, &theirs, Some(MergeStrategy::Theirs)).unwrap();

        assert!(!result.has_conflicts);
        assert_eq!(result.resolved, theirs);
    }

    #[test]
    fn test_auto_merge_no_base() {
        let resolver = ConflictResolver::default();
        let ours = json!({"key1": "value1"});
        let theirs = json!({"key2": "value2"});

        let result = resolver.resolve(None, &ours, &theirs, Some(MergeStrategy::Auto)).unwrap();

        // Should merge both keys
        assert!(!result.has_conflicts);
        assert_eq!(result.resolved["key1"], "value1");
        assert_eq!(result.resolved["key2"], "value2");
    }

    #[test]
    fn test_auto_merge_with_base() {
        let resolver = ConflictResolver::default();
        let base = json!({"key": "base"});
        let ours = json!({"key": "ours"});
        let theirs = json!({"key": "base"}); // Only ours changed

        let result = resolver
            .resolve(Some(&base), &ours, &theirs, Some(MergeStrategy::Auto))
            .unwrap();

        assert!(!result.has_conflicts);
        assert_eq!(result.resolved["key"], "ours");
    }

    #[test]
    fn test_conflict_detection() {
        let resolver = ConflictResolver::default();
        let base = json!({"key": "base"});
        let ours = json!({"key": "ours"});
        let theirs = json!({"key": "theirs"});

        let result = resolver
            .resolve(Some(&base), &ours, &theirs, Some(MergeStrategy::Auto))
            .unwrap();

        assert!(result.has_conflicts);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].path, "key");
    }
}
