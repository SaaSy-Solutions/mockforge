//! Visual layout serialization for state machine graphs
//!
//! Provides structures and conversion utilities for storing and loading visual
//! representations of state machines, compatible with React Flow format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Visual layout for a state machine graph
///
/// Stores the visual representation of a state machine including node positions,
/// edge routing, and visual metadata. This format is compatible with React Flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualLayout {
    /// Visual nodes representing states
    pub nodes: Vec<VisualNode>,

    /// Visual edges representing transitions
    pub edges: Vec<VisualEdge>,

    /// Optional viewport information (zoom, pan)
    pub viewport: Option<Viewport>,
}

/// Visual representation of a state node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualNode {
    /// Unique node identifier (typically matches state name)
    pub id: String,

    /// Node type: "state", "initial", "sub-scenario", "condition"
    #[serde(rename = "type")]
    pub node_type: String,

    /// X position in pixels
    pub position_x: f64,

    /// Y position in pixels
    pub position_y: f64,

    /// Node width in pixels
    #[serde(default = "default_width")]
    pub width: f64,

    /// Node height in pixels
    #[serde(default = "default_height")]
    pub height: f64,

    /// Node label/text
    pub label: String,

    /// Additional visual properties
    #[serde(default)]
    pub style: HashMap<String, serde_json::Value>,

    /// Node data (custom properties)
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

/// Visual representation of a transition edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEdge {
    /// Unique edge identifier
    pub id: String,

    /// Source node ID
    pub source: String,

    /// Target node ID
    pub target: String,

    /// Edge label (condition, probability, etc.)
    #[serde(default)]
    pub label: Option<String>,

    /// Edge type: "transition", "conditional", "default"
    #[serde(rename = "type", default = "default_edge_type")]
    pub edge_type: String,

    /// Whether this edge is animated (for active transitions)
    #[serde(default)]
    pub animated: bool,

    /// Edge style properties
    #[serde(default)]
    pub style: HashMap<String, serde_json::Value>,

    /// Edge data (condition expression, probability, etc.)
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

/// Viewport information for the visual editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// Zoom level (1.0 = 100%)
    pub zoom: f64,

    /// Pan X offset
    pub x: f64,

    /// Pan Y offset
    pub y: f64,
}

impl VisualLayout {
    /// Create a new empty visual layout
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            viewport: None,
        }
    }

    /// Add a visual node
    pub fn add_node(mut self, node: VisualNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add a visual edge
    pub fn add_edge(mut self, edge: VisualEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Set viewport
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// Convert to React Flow format (JSON)
    ///
    /// Returns a JSON object compatible with React Flow's node/edge format.
    pub fn to_react_flow_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(|n| {
                let mut node_data = serde_json::Map::new();
                node_data.insert("label".to_string(), serde_json::Value::String(n.label.clone()));
                for (k, v) in &n.data {
                    node_data.insert(k.clone(), v.clone());
                }
                serde_json::json!({
                    "id": n.id,
                    "type": n.node_type,
                    "position": {
                        "x": n.position_x,
                        "y": n.position_y
                    },
                    "data": node_data,
                    "style": n.style,
                    "width": n.width,
                    "height": n.height
                })
            }).collect::<Vec<_>>(),
            "edges": self.edges.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "source": e.source,
                    "target": e.target,
                    "label": e.label,
                    "type": e.edge_type,
                    "animated": e.animated,
                    "style": e.style,
                    "data": e.data
                })
            }).collect::<Vec<_>>(),
            "viewport": self.viewport.as_ref().map(|v| {
                serde_json::json!({
                    "zoom": v.zoom,
                    "x": v.x,
                    "y": v.y
                })
            })
        })
    }

    /// Create from React Flow format (JSON)
    ///
    /// Parses a React Flow JSON object into a VisualLayout.
    pub fn from_react_flow_json(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let empty_vec: Vec<serde_json::Value> = Vec::new();
        let nodes = value
            .get("nodes")
            .and_then(|n| n.as_array())
            .unwrap_or(&empty_vec)
            .iter()
            .map(|n| {
                let empty_map = serde_json::Map::new();
                let position = n.get("position").and_then(|p| p.as_object()).unwrap_or(&empty_map);
                let data = n.get("data").and_then(|d| d.as_object()).unwrap_or(&empty_map);
                let style = n.get("style").and_then(|s| s.as_object()).unwrap_or(&empty_map);

                VisualNode {
                    id: n.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                    node_type: n
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("state")
                        .to_string(),
                    position_x: position.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    position_y: position.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
                    width: n.get("width").and_then(|w| w.as_f64()).unwrap_or(150.0),
                    height: n.get("height").and_then(|h| h.as_f64()).unwrap_or(40.0),
                    label: data.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                    style: style.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    data: data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                }
            })
            .collect();

        let empty_vec: Vec<serde_json::Value> = Vec::new();
        let edges = value
            .get("edges")
            .and_then(|e| e.as_array())
            .unwrap_or(&empty_vec)
            .iter()
            .map(|e| {
                let empty_map = serde_json::Map::new();
                let style = e.get("style").and_then(|s| s.as_object()).unwrap_or(&empty_map);
                let data = e.get("data").and_then(|d| d.as_object()).unwrap_or(&empty_map);

                VisualEdge {
                    id: e.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                    source: e.get("source").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                    target: e.get("target").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    label: e.get("label").and_then(|l| l.as_str()).map(|s| s.to_string()),
                    edge_type: e
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("default")
                        .to_string(),
                    animated: e.get("animated").and_then(|a| a.as_bool()).unwrap_or(false),
                    style: style.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    data: data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                }
            })
            .collect();

        let viewport = value.get("viewport").and_then(|v| serde_json::from_value(v.clone()).ok());

        Ok(Self {
            nodes,
            edges,
            viewport,
        })
    }
}

impl Default for VisualLayout {
    fn default() -> Self {
        Self::new()
    }
}

fn default_width() -> f64 {
    150.0
}

fn default_height() -> f64 {
    40.0
}

fn default_edge_type() -> String {
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_layout_creation() {
        let layout = VisualLayout::new()
            .add_node(VisualNode {
                id: "state1".to_string(),
                node_type: "state".to_string(),
                position_x: 100.0,
                position_y: 200.0,
                width: 150.0,
                height: 40.0,
                label: "Pending".to_string(),
                style: HashMap::new(),
                data: HashMap::new(),
            })
            .add_edge(VisualEdge {
                id: "edge1".to_string(),
                source: "state1".to_string(),
                target: "state2".to_string(),
                label: Some("condition".to_string()),
                edge_type: "transition".to_string(),
                animated: false,
                style: HashMap::new(),
                data: HashMap::new(),
            });

        assert_eq!(layout.nodes.len(), 1);
        assert_eq!(layout.edges.len(), 1);
    }

    #[test]
    fn test_react_flow_conversion() {
        let layout = VisualLayout::new().add_node(VisualNode {
            id: "state1".to_string(),
            node_type: "state".to_string(),
            position_x: 100.0,
            position_y: 200.0,
            width: 150.0,
            height: 40.0,
            label: "Pending".to_string(),
            style: HashMap::new(),
            data: HashMap::new(),
        });

        let json = layout.to_react_flow_json();
        assert!(json.get("nodes").is_some());
        assert!(json.get("edges").is_some());

        // Test round-trip conversion
        let parsed = VisualLayout::from_react_flow_json(&json).unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].id, "state1");
    }
}
