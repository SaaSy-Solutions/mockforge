//! World State Engine - Central coordinator for unified world state
//!
//! This module provides the core engine that aggregates state from all
//! subsystems and maintains unified state snapshots.

use crate::aggregators::StateAggregator;
use crate::model::{StateLayer, WorldStateSnapshot};
use crate::query::WorldStateQuery;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// World State Engine
///
/// Central coordinator that aggregates state from all MockForge subsystems
/// and provides a unified view of the entire system state.
pub struct WorldStateEngine {
    /// Registered aggregators for each layer
    aggregators: HashMap<StateLayer, Arc<dyn StateAggregator>>,
    /// Historical snapshots (limited to recent ones)
    snapshots: Arc<RwLock<Vec<WorldStateSnapshot>>>,
    /// Maximum number of snapshots to keep
    max_snapshots: usize,
}

impl WorldStateEngine {
    /// Create a new world state engine
    pub fn new() -> Self {
        Self {
            aggregators: HashMap::new(),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots: 100,
        }
    }

    /// Register an aggregator for a layer
    pub fn register_aggregator(&mut self, aggregator: Arc<dyn StateAggregator>) {
        let layer = aggregator.layer();
        self.aggregators.insert(layer, aggregator);
        info!("Registered aggregator for layer: {:?}", layer);
    }

    /// Create a snapshot of the current world state
    pub async fn create_snapshot(&self) -> Result<WorldStateSnapshot> {
        debug!("Creating world state snapshot");

        let mut snapshot = WorldStateSnapshot::new();
        let mut all_nodes = Vec::new();
        let mut all_edges = Vec::new();

        // Aggregate state from all registered aggregators
        for (layer, aggregator) in &self.aggregators {
            match aggregator.aggregate().await {
                Ok((nodes, edges)) => {
                    debug!(
                        "Aggregated {} nodes and {} edges from layer: {:?}",
                        nodes.len(),
                        edges.len(),
                        layer
                    );
                    all_nodes.extend(nodes);
                    all_edges.extend(edges);
                    snapshot.layers.insert(*layer, true);
                }
                Err(e) => {
                    warn!("Failed to aggregate state from layer {:?}: {}", layer, e);
                    snapshot.layers.insert(*layer, false);
                }
            }
        }

        snapshot.nodes = all_nodes;
        snapshot.edges = all_edges;

        // Store snapshot
        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        // Limit snapshot history
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        info!(
            "Created world state snapshot with {} nodes and {} edges",
            snapshot.nodes.len(),
            snapshot.edges.len()
        );

        Ok(snapshot)
    }

    /// Get the current world state snapshot
    pub async fn get_current_snapshot(&self) -> Result<WorldStateSnapshot> {
        self.create_snapshot().await
    }

    /// Get a snapshot by ID
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Option<WorldStateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.iter().find(|s| s.id == snapshot_id).cloned()
    }

    /// Get all available snapshots
    pub async fn get_all_snapshots(&self) -> Vec<WorldStateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.clone()
    }

    /// Query the current world state with filters
    pub async fn query(&self, query: &WorldStateQuery) -> Result<WorldStateSnapshot> {
        let snapshot = self.create_snapshot().await?;

        // Filter nodes
        let filtered_nodes: Vec<_> =
            snapshot.nodes.iter().filter(|node| query.matches_node(node)).cloned().collect();

        // Filter edges
        let filtered_edges: Vec<_> = if query.include_edges {
            snapshot.edges.iter().filter(|edge| query.matches_edge(edge)).cloned().collect()
        } else {
            Vec::new()
        };

        // Create filtered snapshot
        let mut filtered_snapshot = snapshot;
        filtered_snapshot.nodes = filtered_nodes;
        filtered_snapshot.edges = filtered_edges;

        Ok(filtered_snapshot)
    }

    /// Get available layers
    pub fn get_layers(&self) -> Vec<StateLayer> {
        self.aggregators.keys().copied().collect()
    }

    /// Set maximum number of snapshots to keep
    pub fn set_max_snapshots(&mut self, max: usize) {
        self.max_snapshots = max;
    }
}

impl Default for WorldStateEngine {
    fn default() -> Self {
        Self::new()
    }
}
