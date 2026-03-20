//! Distributed chaos coordination
//!
//! Coordinate chaos orchestrations across distributed systems with leader election,
//! state synchronization, and distributed consensus.

use crate::scenario_orchestrator::OrchestratedScenario;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Node in the distributed system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub address: String,
    pub region: Option<String>,
    pub zone: Option<String>,
    pub capabilities: Vec<String>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: NodeStatus,
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Active,
    Inactive,
    Degraded,
    Failed,
}

/// Leader election state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderState {
    pub leader_id: Option<String>,
    pub term: u64,
    pub elected_at: Option<DateTime<Utc>>,
}

/// Distributed orchestration task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTask {
    pub id: String,
    pub orchestration: OrchestratedScenario,
    pub target_nodes: Vec<String>,
    pub coordination_mode: CoordinationMode,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

/// Coordination mode for distributed execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoordinationMode {
    /// All nodes execute in parallel
    Parallel,
    /// One node at a time
    Sequential,
    /// Leader assigns tasks
    LeaderAssigned,
    /// Nodes coordinate amongst themselves
    PeerToPeer,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Node execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionState {
    pub node_id: String,
    pub task_id: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

/// Execution metrics for a node
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetrics {
    pub steps_completed: usize,
    pub steps_total: usize,
    pub events_generated: usize,
    pub errors: usize,
    pub avg_latency_ms: f64,
}

/// Distributed coordinator
pub struct DistributedCoordinator {
    /// Current node ID
    node_id: String,
    /// Registered nodes
    nodes: Arc<RwLock<HashMap<String, Node>>>,
    /// Leader state
    leader_state: Arc<RwLock<LeaderState>>,
    /// Active tasks
    tasks: Arc<RwLock<HashMap<String, DistributedTask>>>,
    /// Node execution states
    execution_states: Arc<RwLock<HashMap<String, NodeExecutionState>>>,
    /// Control channel
    control_tx: Option<mpsc::Sender<CoordinatorControl>>,
}

/// Coordinator control commands
enum CoordinatorControl {
    RegisterNode(Node),
    UnregisterNode(String),
    SubmitTask(DistributedTask),
    Heartbeat(String),
    TriggerElection,
}

impl DistributedCoordinator {
    /// Create a new distributed coordinator
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            leader_state: Arc::new(RwLock::new(LeaderState {
                leader_id: None,
                term: 0,
                elected_at: None,
            })),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            execution_states: Arc::new(RwLock::new(HashMap::new())),
            control_tx: None,
        }
    }

    /// Start the coordinator
    pub async fn start(&mut self) -> Result<(), String> {
        info!("Starting distributed coordinator for node {}", self.node_id);

        // Create control channel
        let (control_tx, mut control_rx) = mpsc::channel::<CoordinatorControl>(100);
        self.control_tx = Some(control_tx);

        // Clone Arc references for background task
        let node_id = self.node_id.clone();
        let nodes = Arc::clone(&self.nodes);
        let leader_state = Arc::clone(&self.leader_state);
        let tasks = Arc::clone(&self.tasks);
        let execution_states = Arc::clone(&self.execution_states);

        // Spawn coordinator task
        tokio::spawn(async move {
            Self::coordinator_task(
                node_id,
                nodes,
                leader_state,
                tasks,
                execution_states,
                &mut control_rx,
            )
            .await;
        });

        Ok(())
    }

    /// Coordinator background task
    async fn coordinator_task(
        node_id: String,
        nodes: Arc<RwLock<HashMap<String, Node>>>,
        leader_state: Arc<RwLock<LeaderState>>,
        tasks: Arc<RwLock<HashMap<String, DistributedTask>>>,
        _execution_states: Arc<RwLock<HashMap<String, NodeExecutionState>>>,
        control_rx: &mut mpsc::Receiver<CoordinatorControl>,
    ) {
        loop {
            tokio::select! {
                Some(cmd) = control_rx.recv() => {
                    match cmd {
                        CoordinatorControl::RegisterNode(node) => {
                            info!("Registering node: {}", node.id);
                            let mut nodes_guard = nodes.write();
                            nodes_guard.insert(node.id.clone(), node);
                        }
                        CoordinatorControl::UnregisterNode(id) => {
                            info!("Unregistering node: {}", id);
                            let mut nodes_guard = nodes.write();
                            nodes_guard.remove(&id);
                        }
                        CoordinatorControl::SubmitTask(task) => {
                            info!("Submitting task: {}", task.id);
                            let mut tasks_guard = tasks.write();
                            tasks_guard.insert(task.id.clone(), task);
                        }
                        CoordinatorControl::Heartbeat(node_id) => {
                            debug!("Heartbeat from node: {}", node_id);
                            let mut nodes_guard = nodes.write();
                            if let Some(node) = nodes_guard.get_mut(&node_id) {
                                node.last_heartbeat = Utc::now();
                            }
                        }
                        CoordinatorControl::TriggerElection => {
                            info!("Triggering leader election");
                            Self::elect_leader(&node_id, &nodes, &leader_state);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                    // Periodic health checks
                    Self::check_node_health(&nodes);

                    // Check if leader election needed
                    let needs_election = {
                        let state = leader_state.read();
                        state.leader_id.is_none()
                    };

                    if needs_election {
                        Self::elect_leader(&node_id, &nodes, &leader_state);
                    }
                }
            }
        }
    }

    /// Register a node
    pub async fn register_node(&self, node: Node) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(CoordinatorControl::RegisterNode(node))
                .await
                .map_err(|e| format!("Failed to register node: {}", e))?;
            Ok(())
        } else {
            Err("Coordinator not started".to_string())
        }
    }

    /// Unregister a node
    pub async fn unregister_node(&self, node_id: &str) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(CoordinatorControl::UnregisterNode(node_id.to_string()))
                .await
                .map_err(|e| format!("Failed to unregister node: {}", e))?;
            Ok(())
        } else {
            Err("Coordinator not started".to_string())
        }
    }

    /// Submit a distributed task
    pub async fn submit_task(&self, task: DistributedTask) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(CoordinatorControl::SubmitTask(task))
                .await
                .map_err(|e| format!("Failed to submit task: {}", e))?;
            Ok(())
        } else {
            Err("Coordinator not started".to_string())
        }
    }

    /// Send heartbeat
    pub async fn heartbeat(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(CoordinatorControl::Heartbeat(self.node_id.clone()))
                .await
                .map_err(|e| format!("Failed to send heartbeat: {}", e))?;
            Ok(())
        } else {
            Err("Coordinator not started".to_string())
        }
    }

    /// Trigger leader election
    pub async fn trigger_election(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(CoordinatorControl::TriggerElection)
                .await
                .map_err(|e| format!("Failed to trigger election: {}", e))?;
            Ok(())
        } else {
            Err("Coordinator not started".to_string())
        }
    }

    /// Elect leader (simple implementation)
    fn elect_leader(
        _current_node_id: &str,
        nodes: &Arc<RwLock<HashMap<String, Node>>>,
        leader_state: &Arc<RwLock<LeaderState>>,
    ) {
        let nodes_guard = nodes.read();

        // Find active nodes
        let active_nodes: Vec<_> =
            nodes_guard.values().filter(|n| n.status == NodeStatus::Active).collect();

        if active_nodes.is_empty() {
            warn!("No active nodes for leader election");
            return;
        }

        // Simple election: node with lowest ID becomes leader
        let leader = active_nodes.iter().min_by(|a, b| a.id.cmp(&b.id)).unwrap();

        let mut state = leader_state.write();
        state.leader_id = Some(leader.id.clone());
        state.term += 1;
        state.elected_at = Some(Utc::now());

        info!("Leader elected: {} (term {})", leader.id, state.term);
    }

    /// Check node health
    fn check_node_health(nodes: &Arc<RwLock<HashMap<String, Node>>>) {
        let mut nodes_guard = nodes.write();
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(30);

        for node in nodes_guard.values_mut() {
            if node.status == NodeStatus::Active {
                let since_heartbeat = now - node.last_heartbeat;
                if since_heartbeat > timeout {
                    warn!("Node {} missed heartbeat", node.id);
                    node.status = NodeStatus::Degraded;
                }
                if since_heartbeat > timeout * 2 {
                    warn!("Node {} failed (no heartbeat)", node.id);
                    node.status = NodeStatus::Failed;
                }
            }
        }
    }

    /// Get current leader
    pub fn get_leader(&self) -> Option<String> {
        let state = self.leader_state.read();
        state.leader_id.clone()
    }

    /// Check if this node is the leader
    pub fn is_leader(&self) -> bool {
        let state = self.leader_state.read();
        state.leader_id.as_ref() == Some(&self.node_id)
    }

    /// Get all registered nodes
    pub fn get_nodes(&self) -> Vec<Node> {
        let nodes = self.nodes.read();
        nodes.values().cloned().collect()
    }

    /// Get active nodes
    pub fn get_active_nodes(&self) -> Vec<Node> {
        let nodes = self.nodes.read();
        nodes.values().filter(|n| n.status == NodeStatus::Active).cloned().collect()
    }

    /// Get task status
    pub fn get_task(&self, task_id: &str) -> Option<DistributedTask> {
        let tasks = self.tasks.read();
        tasks.get(task_id).cloned()
    }

    /// Get all tasks
    pub fn get_tasks(&self) -> Vec<DistributedTask> {
        let tasks = self.tasks.read();
        tasks.values().cloned().collect()
    }

    /// Get node execution states for a task
    pub fn get_task_execution_states(&self, task_id: &str) -> Vec<NodeExecutionState> {
        let states = self.execution_states.read();
        states
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}:", task_id)))
            .map(|(_, v)| v.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = DistributedCoordinator::new("node-1");
        assert_eq!(coordinator.node_id, "node-1");
        assert!(!coordinator.is_leader());
    }

    #[test]
    fn test_node_status() {
        let node = Node {
            id: "node-1".to_string(),
            address: "127.0.0.1:8080".to_string(),
            region: Some("us-east-1".to_string()),
            zone: Some("us-east-1a".to_string()),
            capabilities: vec!["chaos".to_string()],
            last_heartbeat: Utc::now(),
            status: NodeStatus::Active,
        };

        assert_eq!(node.status, NodeStatus::Active);
    }

    #[tokio::test]
    async fn test_coordinator_start() {
        let mut coordinator = DistributedCoordinator::new("node-1");
        assert!(coordinator.start().await.is_ok());
    }

    #[tokio::test]
    async fn test_register_node() {
        let mut coordinator = DistributedCoordinator::new("node-1");
        coordinator.start().await.unwrap();

        let node = Node {
            id: "node-2".to_string(),
            address: "127.0.0.1:8081".to_string(),
            region: None,
            zone: None,
            capabilities: vec![],
            last_heartbeat: Utc::now(),
            status: NodeStatus::Active,
        };

        // Give the coordinator task time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(coordinator.register_node(node).await.is_ok());

        // Wait for registration to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let nodes = coordinator.get_nodes();
        assert!(nodes.iter().any(|n| n.id == "node-2"));
    }
}
