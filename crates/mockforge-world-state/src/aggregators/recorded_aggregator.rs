//! Recorded Aggregator - Collect recorded data and replay state into world state
//!
//! This aggregator collects recorded requests/responses, fixtures, and
//! replay state from the record/replay subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Aggregator for recorded data state
///
/// Scans the workspace for recorded data files (fixtures, recordings, captures)
/// and represents them as nodes in the world state graph.
pub struct RecordedAggregator {}

impl RecordedAggregator {
    /// Create a new recorded aggregator
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StateAggregator for RecordedAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let recorded_files = collect_matching_files(
            &workspace,
            &["fixtures", "recordings", "captures", "replays"],
            is_recorded_file,
        );

        let mut recorded_node = StateNode::new(
            "recorded:system".to_string(),
            "Recorded Data".to_string(),
            NodeType::Recorded,
            StateLayer::Recorded,
        );
        recorded_node.set_property("workspace".to_string(), json!(workspace.display().to_string()));
        recorded_node.set_property("recorded_count".to_string(), json!(recorded_files.len()));

        nodes.push(recorded_node);

        for file in recorded_files {
            let rel_path = file
                .strip_prefix(&workspace)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file.to_string_lossy().to_string());

            let mut node = StateNode::new(
                format!("recorded:file:{}", rel_path),
                file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "recorded".to_string()),
                NodeType::Recorded,
                StateLayer::Recorded,
            );
            node.set_property("path".to_string(), json!(rel_path));
            if let Ok(metadata) = fs::metadata(&file) {
                node.set_property("size_bytes".to_string(), json!(metadata.len()));
            }
            nodes.push(node);
            edges.push(StateEdge::new(
                "recorded:system".to_string(),
                format!("recorded:file:{}", rel_path),
                "contains".to_string(),
            ));
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Recorded
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

fn is_recorded_file(path: &Path) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let recorded_like = name.contains("fixture")
        || name.contains("record")
        || name.contains("capture")
        || name.contains("replay");
    recorded_like && matches!(ext.as_str(), "json" | "yaml" | "yml" | "har")
}
