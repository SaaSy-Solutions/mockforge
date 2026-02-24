//! Behavior Aggregator - Collect behavior trees and rules into world state
//!
//! This aggregator collects behavior trees, rules, and AI modifiers from
//! the intelligent behavior subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Aggregator for behavior state
///
/// Scans the workspace for behavior-related files (rules, policies, scenarios)
/// and represents them as nodes in the world state graph.
pub struct BehaviorAggregator {}

impl BehaviorAggregator {
    /// Create a new behavior aggregator
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StateAggregator for BehaviorAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let behavior_files = collect_matching_files(
            &workspace,
            &["behaviors", "rules", "scenarios", "policies"],
            is_behavior_file,
        );

        let mut behavior_node = StateNode::new(
            "behavior:system".to_string(),
            "Behavior System".to_string(),
            NodeType::Behavior,
            StateLayer::Behavior,
        );
        behavior_node.set_property("workspace".to_string(), json!(workspace.display().to_string()));
        behavior_node.set_property("behavior_count".to_string(), json!(behavior_files.len()));

        nodes.push(behavior_node);

        for file in behavior_files {
            let rel_path = file
                .strip_prefix(&workspace)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file.to_string_lossy().to_string());

            let mut node = StateNode::new(
                format!("behavior:file:{}", rel_path),
                file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "behavior".to_string()),
                NodeType::Behavior,
                StateLayer::Behavior,
            );
            node.set_property("path".to_string(), json!(rel_path));
            if let Ok(metadata) = fs::metadata(&file) {
                node.set_property("size_bytes".to_string(), json!(metadata.len()));
            }
            nodes.push(node);
            edges.push(StateEdge::new(
                "behavior:system".to_string(),
                format!("behavior:file:{}", rel_path),
                "contains".to_string(),
            ));
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Behavior
    }
}

fn collect_matching_files<F>(workspace: &Path, roots: &[&str], predicate: F) -> Vec<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = roots.iter().map(|p| workspace.join(p)).collect();

    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if predicate(&entry_path) {
                out.push(entry_path);
            }
        }
    }

    out
}

fn is_behavior_file(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let behavior_like = name.contains("behavior")
        || name.contains("rule")
        || name.contains("policy")
        || name.contains("scenario");
    behavior_like && matches!(ext.as_str(), "json" | "yaml" | "yml" | "toml")
}
