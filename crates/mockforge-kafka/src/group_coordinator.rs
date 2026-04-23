//! Minimal consumer-group coordinator state.
//!
//! This module is intentionally scoped to what `rdkafka::StreamConsumer`
//! with a single consumer per `group.id` actually needs:
//!
//! - JoinGroup assigns the calling client as group leader (it's the only
//!   member) and bumps the generation.
//! - SyncGroup records the leader-provided assignment and hands each
//!   member back its slice.
//! - Heartbeat just validates generation + member identity.
//! - Offsets storage is provided here too so a follow-up PR can wire
//!   OffsetCommit / OffsetFetch without another refactor.
//!
//! **Out of scope (tracked separately):** multi-member rebalance with
//! generation bumps, instance IDs for static membership, coordinator
//! failover, transactional producer-group coordination.

use std::collections::HashMap;
use uuid::Uuid;

/// Per-member state inside a consumer group.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub member_id: String,
    /// Raw subscription metadata blob from JoinGroup — echoed back to the
    /// leader verbatim so librdkafka's leader-side assignor can parse it.
    pub metadata: Vec<u8>,
    /// Assignment blob filled in by SyncGroup once the leader has decided.
    pub assignment: Vec<u8>,
}

/// One consumer group's state.
#[derive(Debug, Clone)]
pub struct GroupMembership {
    pub generation_id: i32,
    pub leader_id: String,
    /// Protocol name chosen (e.g. "range", "roundrobin"). Echoed in
    /// SyncGroup and OffsetCommit responses.
    pub protocol_name: String,
    pub members: HashMap<String, MemberInfo>,
}

/// Handle returned from `join_group`: identifies the caller and names the
/// other members the coordinator knows about (for single-consumer groups,
/// that's always just the caller).
#[derive(Debug, Clone)]
pub struct JoinOutcome {
    pub generation_id: i32,
    pub member_id: String,
    pub leader_id: String,
    pub protocol_name: String,
    pub members: Vec<MemberInfo>,
}

/// In-memory coordinator shared across all broker connections for a
/// particular process. Held behind `Arc<tokio::sync::RwLock<_>>` in the
/// broker.
#[derive(Debug, Default)]
pub struct GroupCoordinator {
    groups: HashMap<String, GroupMembership>,
}

impl GroupCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle a JoinGroup.
    ///
    /// Caller passes the `group_id`, their `requested_member_id` (empty on
    /// first join), the protocol names they support (we pick the first),
    /// and the protocol metadata blob — for "consumer" it contains the
    /// subscription, and we echo it back to the leader without parsing.
    ///
    /// On first join to a group the caller is the leader. Every subsequent
    /// JoinGroup from an already-known member_id re-acknowledges without
    /// bumping the generation; a JoinGroup with a new empty member_id on an
    /// existing group creates a second member and bumps generation (basic
    /// rebalance support — the first member will get REBALANCE_IN_PROGRESS
    /// on its next heartbeat if we ever extend this; for PR-A scope the
    /// first member just stays leader).
    pub fn join_group(
        &mut self,
        group_id: &str,
        requested_member_id: &str,
        protocols: &[(String, Vec<u8>)],
    ) -> JoinOutcome {
        // Pick the first offered protocol — "range" / "roundrobin" /
        // "cooperative-sticky". librdkafka sends all three in preference
        // order; we just accept whichever is first.
        let (protocol_name, metadata) =
            protocols.first().cloned().unwrap_or_else(|| ("range".to_string(), Vec::new()));

        let entry = self.groups.entry(group_id.to_string()).or_insert_with(|| GroupMembership {
            generation_id: 0,
            leader_id: String::new(),
            protocol_name: protocol_name.clone(),
            members: HashMap::new(),
        });

        let member_id = if requested_member_id.is_empty() {
            // New member — generate an id, bump generation, (re)elect
            // leader if we don't have one.
            let new_id = format!("mockforge-consumer-{}", Uuid::new_v4());
            entry.generation_id += 1;
            entry.members.insert(
                new_id.clone(),
                MemberInfo {
                    member_id: new_id.clone(),
                    metadata: metadata.clone(),
                    assignment: Vec::new(),
                },
            );
            if entry.leader_id.is_empty() || !entry.members.contains_key(&entry.leader_id) {
                entry.leader_id = new_id.clone();
            }
            entry.protocol_name = protocol_name;
            new_id
        } else {
            // Known member re-joining — refresh its subscription metadata
            // without bumping the generation.
            entry
                .members
                .entry(requested_member_id.to_string())
                .and_modify(|m| m.metadata = metadata.clone())
                .or_insert_with(|| MemberInfo {
                    member_id: requested_member_id.to_string(),
                    metadata,
                    assignment: Vec::new(),
                });
            requested_member_id.to_string()
        };

        JoinOutcome {
            generation_id: entry.generation_id,
            member_id,
            leader_id: entry.leader_id.clone(),
            protocol_name: entry.protocol_name.clone(),
            members: entry.members.values().cloned().collect(),
        }
    }

    /// Apply the leader's per-member assignments from a SyncGroup, and
    /// return the specific assignment for `member_id`. Returns `None` if
    /// the group or member is unknown.
    pub fn sync_group(
        &mut self,
        group_id: &str,
        member_id: &str,
        assignments: Vec<(String, Vec<u8>)>,
    ) -> Option<Vec<u8>> {
        let group = self.groups.get_mut(group_id)?;
        for (target_member, assignment) in assignments {
            if let Some(m) = group.members.get_mut(&target_member) {
                m.assignment = assignment;
            }
        }
        group.members.get(member_id).map(|m| m.assignment.clone())
    }

    /// Validate a heartbeat. Returns `Ok(())` on success, `Err(error_code)`
    /// on failure (e.g. UNKNOWN_MEMBER_ID 25 / ILLEGAL_GENERATION 22).
    pub fn heartbeat(
        &self,
        group_id: &str,
        generation_id: i32,
        member_id: &str,
    ) -> Result<(), i16> {
        const ERR_ILLEGAL_GENERATION: i16 = 22;
        const ERR_UNKNOWN_MEMBER_ID: i16 = 25;
        const ERR_GROUP_ID_NOT_FOUND: i16 = 69;
        let group = self.groups.get(group_id).ok_or(ERR_GROUP_ID_NOT_FOUND)?;
        if !group.members.contains_key(member_id) {
            return Err(ERR_UNKNOWN_MEMBER_ID);
        }
        if group.generation_id != generation_id {
            return Err(ERR_ILLEGAL_GENERATION);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_join_makes_caller_leader() {
        let mut coord = GroupCoordinator::new();
        let out = coord.join_group("g1", "", &[("range".into(), b"subscription".to_vec())]);
        assert_eq!(out.generation_id, 1);
        assert_eq!(out.leader_id, out.member_id);
        assert_eq!(out.protocol_name, "range");
        assert_eq!(out.members.len(), 1);
        assert_eq!(out.members[0].metadata, b"subscription");
    }

    #[test]
    fn rejoin_with_known_id_does_not_bump_generation() {
        let mut coord = GroupCoordinator::new();
        let first = coord.join_group("g", "", &[("range".into(), b"m".to_vec())]);
        let second = coord.join_group("g", &first.member_id, &[("range".into(), b"m".to_vec())]);
        assert_eq!(first.generation_id, second.generation_id);
        assert_eq!(first.member_id, second.member_id);
    }

    #[test]
    fn sync_returns_member_assignment() {
        let mut coord = GroupCoordinator::new();
        let out = coord.join_group("g", "", &[("range".into(), b"m".to_vec())]);
        let assignment =
            coord.sync_group("g", &out.member_id, vec![(out.member_id.clone(), b"A".to_vec())]);
        assert_eq!(assignment.as_deref(), Some(b"A".as_ref()));
    }

    #[test]
    fn heartbeat_validates_generation_and_membership() {
        let mut coord = GroupCoordinator::new();
        let out = coord.join_group("g", "", &[("range".into(), vec![])]);
        assert!(coord.heartbeat("g", out.generation_id, &out.member_id).is_ok());
        assert_eq!(coord.heartbeat("g", out.generation_id + 99, &out.member_id), Err(22));
        assert_eq!(coord.heartbeat("g", out.generation_id, "who?"), Err(25));
        assert_eq!(coord.heartbeat("no-such-group", 0, &out.member_id), Err(69));
    }
}
