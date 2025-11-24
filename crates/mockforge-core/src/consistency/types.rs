//! Types for cross-protocol consistency
//!
//! This module defines the core data structures for maintaining unified state
//! across all protocols in MockForge.

use crate::protocol_abstraction::Protocol;
use crate::reality::RealityLevel;
use chrono::{DateTime, Utc};
// ChaosScenario is defined in mockforge-chaos, but we use serde_json::Value to avoid circular dependency
// When used, it should be deserialized from JSON
type ChaosScenario = serde_json::Value;
#[cfg(feature = "data")]
pub use mockforge_data::PersonaGraph;
#[cfg(feature = "data")]
pub use mockforge_data::PersonaProfile;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Unified state for a workspace across all protocols
///
/// This structure maintains the complete state of a workspace, including
/// active persona, scenario, reality level, chaos rules, and cross-protocol
/// entity state. All protocols should reflect this unified state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedState {
    /// Workspace identifier
    pub workspace_id: String,
    /// Active persona profile (if any)
    ///
    /// When set, all protocols should use this persona for data generation
    /// and behavior. Changing the persona should immediately affect all
    /// protocol responses.
    pub active_persona: Option<PersonaProfile>,
    /// Active scenario identifier (if any)
    ///
    /// The scenario defines the current state machine or workflow state.
    /// All protocols should reflect this scenario's state.
    pub active_scenario: Option<String>,
    /// Current reality level (1-5)
    ///
    /// Controls the overall realism level across all protocols.
    pub reality_level: RealityLevel,
    /// Reality continuum blend ratio (0.0 = mock, 1.0 = real)
    ///
    /// Determines how much real vs mock data is used across protocols.
    pub reality_continuum_ratio: f64,
    /// Active chaos scenarios/rules
    ///
    /// List of active chaos engineering rules that should be applied
    /// across all protocols.
    pub active_chaos_rules: Vec<ChaosScenario>,
    /// Cross-protocol entity state (shared entities across protocols)
    ///
    /// Maps entity keys (format: "{entity_type}:{entity_id}") to entity state.
    /// This ensures that entities created in one protocol are visible in all
    /// other protocols.
    pub entity_state: HashMap<String, EntityState>,
    /// Protocol-specific state snapshots
    ///
    /// Each protocol can store its own state (sessions, connections, etc.)
    /// while still being coordinated by the unified state.
    pub protocol_states: HashMap<Protocol, ProtocolState>,
    /// Persona graph for managing entity relationships
    ///
    /// Maintains relationships between personas across different entity types
    /// (user → orders → payments → webhooks → TCP messages), enabling
    /// coherent data generation across endpoints.
    #[cfg(feature = "data")]
    #[serde(skip)]
    pub persona_graph: Option<PersonaGraph>,
    #[cfg(not(feature = "data"))]
    #[serde(skip)]
    #[allow(dead_code)]
    persona_graph: Option<()>,
    /// Timestamp of last state update
    pub last_updated: DateTime<Utc>,
    /// State version for conflict resolution
    ///
    /// Incremented on each state change to enable optimistic locking
    /// and conflict detection.
    pub version: u64,
}

impl UnifiedState {
    /// Create a new unified state for a workspace
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            active_persona: None,
            active_scenario: None,
            reality_level: RealityLevel::ModerateRealism,
            reality_continuum_ratio: 0.0,
            active_chaos_rules: Vec::new(),
            entity_state: HashMap::new(),
            protocol_states: HashMap::new(),
            #[cfg(feature = "data")]
            persona_graph: Some(PersonaGraph::new()),
            #[cfg(not(feature = "data"))]
            persona_graph: None,
            last_updated: Utc::now(),
            version: 1,
        }
    }

    /// Get or create the persona graph for this workspace
    #[cfg(feature = "data")]
    pub fn get_or_create_persona_graph(&mut self) -> &mut PersonaGraph {
        if self.persona_graph.is_none() {
            self.persona_graph = Some(PersonaGraph::new());
        }
        self.persona_graph.as_mut().unwrap()
    }

    /// Get the persona graph (read-only)
    #[cfg(feature = "data")]
    pub fn persona_graph(&self) -> Option<&PersonaGraph> {
        self.persona_graph.as_ref()
    }

    /// Get or create the persona graph for this workspace (stub when feature disabled)
    #[cfg(not(feature = "data"))]
    pub fn get_or_create_persona_graph(&mut self) -> &mut () {
        if self.persona_graph.is_none() {
            self.persona_graph = Some(());
        }
        self.persona_graph.as_mut().unwrap()
    }

    /// Get the persona graph (read-only, stub when feature disabled)
    #[cfg(not(feature = "data"))]
    pub fn persona_graph(&self) -> Option<&()> {
        self.persona_graph.as_ref()
    }

    /// Create entity key from type and ID
    pub fn entity_key(entity_type: &str, entity_id: &str) -> String {
        format!("{}:{}", entity_type, entity_id)
    }

    /// Get entity by type and ID
    pub fn get_entity(&self, entity_type: &str, entity_id: &str) -> Option<&EntityState> {
        let key = Self::entity_key(entity_type, entity_id);
        self.entity_state.get(&key)
    }

    /// Register or update an entity
    pub fn register_entity(&mut self, entity: EntityState) {
        let key = Self::entity_key(&entity.entity_type, &entity.entity_id);
        self.entity_state.insert(key, entity);
        self.last_updated = Utc::now();
        self.version += 1;
    }

    /// Increment version (called on state changes)
    pub fn increment_version(&mut self) {
        self.version += 1;
        self.last_updated = Utc::now();
    }
}

/// Entity state tracked across protocols
///
/// Represents an entity (user, order, device, etc.) that can be accessed
/// from any protocol. When an entity is created via HTTP, it should be
/// immediately available in GraphQL queries, gRPC calls, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// Entity type (e.g., "user", "order", "device")
    pub entity_type: String,
    /// Entity identifier
    pub entity_id: String,
    /// Entity data (JSON)
    ///
    /// The complete entity data as JSON. This is protocol-agnostic and
    /// can be transformed as needed by each protocol adapter.
    pub data: Value,
    /// Protocols that have seen this entity
    ///
    /// Tracks which protocols have accessed or modified this entity,
    /// useful for debugging and understanding entity lifecycle.
    pub seen_in_protocols: Vec<Protocol>,
    /// Timestamp when entity was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when entity was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional persona ID associated with this entity
    ///
    /// If this entity belongs to a persona, this links them together.
    pub persona_id: Option<String>,
}

impl EntityState {
    /// Create a new entity state
    pub fn new(entity_type: String, entity_id: String, data: Value) -> Self {
        let now = Utc::now();
        Self {
            entity_type,
            entity_id,
            data,
            seen_in_protocols: Vec::new(),
            created_at: now,
            updated_at: now,
            persona_id: None,
        }
    }

    /// Mark entity as seen by a protocol
    pub fn mark_seen_by(&mut self, protocol: Protocol) {
        if !self.seen_in_protocols.contains(&protocol) {
            self.seen_in_protocols.push(protocol);
        }
    }

    /// Update entity data
    pub fn update_data(&mut self, data: Value) {
        self.data = data;
        self.updated_at = Utc::now();
    }
}

/// Protocol-specific state snapshot
///
/// Each protocol can maintain its own state (sessions, connections, etc.)
/// while still being coordinated by the unified state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolState {
    /// Protocol type
    pub protocol: Protocol,
    /// Active sessions/connections (if applicable)
    ///
    /// For stateful protocols like WebSocket or gRPC streaming, this tracks
    /// active connections and their associated state.
    pub active_sessions: Vec<SessionInfo>,
    /// Protocol-specific configuration
    ///
    /// JSON value containing protocol-specific configuration that should
    /// be preserved in snapshots.
    pub config: Value,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl ProtocolState {
    /// Create a new protocol state
    pub fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            active_sessions: Vec::new(),
            config: Value::Object(serde_json::Map::new()),
            last_activity: Utc::now(),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Session information for stateful protocols
///
/// Tracks individual sessions/connections for protocols that maintain
/// stateful connections (WebSocket, gRPC streaming, TCP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub session_id: String,
    /// Persona ID associated with this session (if any)
    pub persona_id: Option<String>,
    /// Timestamp when session was created
    pub created_at: DateTime<Utc>,
    /// Session metadata (protocol-specific)
    pub metadata: HashMap<String, Value>,
}

impl SessionInfo {
    /// Create a new session info
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            persona_id: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// State change event for propagation
///
/// Events are broadcast to all protocol adapters when state changes,
/// allowing them to update their internal state accordingly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChangeEvent {
    /// Persona changed for a workspace
    PersonaChanged {
        /// Workspace ID
        workspace_id: String,
        /// New persona profile
        persona: PersonaProfile,
    },
    /// Scenario changed for a workspace
    ScenarioChanged {
        /// Workspace ID
        workspace_id: String,
        /// New scenario ID
        scenario_id: String,
    },
    /// Reality level changed for a workspace
    RealityLevelChanged {
        /// Workspace ID
        workspace_id: String,
        /// New reality level
        level: RealityLevel,
    },
    /// Reality continuum ratio changed
    RealityRatioChanged {
        /// Workspace ID
        workspace_id: String,
        /// New ratio (0.0-1.0)
        ratio: f64,
    },
    /// Entity created
    EntityCreated {
        /// Workspace ID
        workspace_id: String,
        /// Created entity
        entity: EntityState,
    },
    /// Entity updated
    EntityUpdated {
        /// Workspace ID
        workspace_id: String,
        /// Updated entity
        entity: EntityState,
    },
    /// Chaos rule activated
    ChaosRuleActivated {
        /// Workspace ID
        workspace_id: String,
        /// Activated chaos scenario
        rule: ChaosScenario,
    },
    /// Chaos rule deactivated
    ChaosRuleDeactivated {
        /// Workspace ID
        workspace_id: String,
        /// Name of deactivated rule
        rule_name: String,
    },
}

impl StateChangeEvent {
    /// Get the workspace ID from the event
    pub fn workspace_id(&self) -> &str {
        match self {
            StateChangeEvent::PersonaChanged { workspace_id, .. } => workspace_id,
            StateChangeEvent::ScenarioChanged { workspace_id, .. } => workspace_id,
            StateChangeEvent::RealityLevelChanged { workspace_id, .. } => workspace_id,
            StateChangeEvent::RealityRatioChanged { workspace_id, .. } => workspace_id,
            StateChangeEvent::EntityCreated { workspace_id, .. } => workspace_id,
            StateChangeEvent::EntityUpdated { workspace_id, .. } => workspace_id,
            StateChangeEvent::ChaosRuleActivated { workspace_id, .. } => workspace_id,
            StateChangeEvent::ChaosRuleDeactivated { workspace_id, .. } => workspace_id,
        }
    }
}
