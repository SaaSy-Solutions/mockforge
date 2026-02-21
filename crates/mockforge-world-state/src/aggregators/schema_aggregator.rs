//! Schema Aggregator - Collect generative schemas into world state
//!
//! This aggregator collects generative schema definitions and entity
//! relationships from the generative schema subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Aggregator for schema state
pub struct SchemaAggregator {
    // Placeholder for schema state access
    // This would typically come from a schema manager
}

impl SchemaAggregator {
    /// Create a new schema aggregator
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StateAggregator for SchemaAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let schema_files = collect_matching_files(
            &workspace,
            &["schemas", "specs", "openapi", "api"],
            is_schema_file,
        );

        let mut schema_node = StateNode::new(
            "schema:system".to_string(),
            "Generative Schemas".to_string(),
            NodeType::Schema,
            StateLayer::Schemas,
        );
        schema_node.set_property("workspace".to_string(), json!(workspace.display().to_string()));
        schema_node.set_property("schema_count".to_string(), json!(schema_files.len()));

        nodes.push(schema_node);

        for file in schema_files {
            let rel_path = file
                .strip_prefix(&workspace)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file.to_string_lossy().to_string());

            let mut node = StateNode::new(
                format!("schema:file:{}", rel_path),
                file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "schema".to_string()),
                NodeType::Schema,
                StateLayer::Schemas,
            );
            node.set_property("path".to_string(), json!(rel_path));
            if let Ok(metadata) = fs::metadata(&file) {
                node.set_property("size_bytes".to_string(), json!(metadata.len()));
            }
            nodes.push(node);
            edges.push(StateEdge::new(
                "schema:system".to_string(),
                format!("schema:file:{}", rel_path),
                "contains".to_string(),
            ));
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Schemas
    }
}

fn collect_matching_files<F>(
    workspace: &Path,
    roots: &[&str],
    predicate: F,
) -> Vec<PathBuf>
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

fn is_schema_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let schema_like = name.contains("schema")
        || name.contains("openapi")
        || name.contains("swagger")
        || name.ends_with(".proto");
    schema_like && matches!(ext.as_str(), "json" | "yaml" | "yml" | "proto")
}
