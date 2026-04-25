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
//! - Offset persistence: OffsetCommit stores per-(group, topic, partition)
//!   offsets (+ metadata); OffsetFetch reads them back. A new consumer
//!   joining a group with previously committed offsets resumes from the
//!   committed position instead of `auto.offset.reset`.
//!
//! **Out of scope (tracked separately):** multi-member rebalance with
//! generation bumps, instance IDs for static membership, coordinator
//! failover, transactional producer-group coordination.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// How long a member can go without heartbeating before a subsequent
/// JoinGroup will evict them. librdkafka's default `session.timeout.ms`
/// is 45s; we set this shorter because the mock only needs to survive
/// ungraceful disconnects during tests, and a tighter window makes
/// consumer-restart tests complete quickly.
const STALE_MEMBER_TIMEOUT: Duration = Duration::from_secs(10);

/// Per-member state inside a consumer group.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub member_id: String,
    /// Raw subscription metadata blob from JoinGroup — echoed back to the
    /// leader verbatim so librdkafka's leader-side assignor can parse it.
    pub metadata: Vec<u8>,
    /// Assignment blob filled in by SyncGroup once the leader has decided.
    pub assignment: Vec<u8>,
    /// Last time we saw activity from this member (join, sync, heartbeat).
    /// Used to evict members that disconnected without a LeaveGroup so a
    /// new consumer with the same `group.id` can claim the partitions.
    pub last_seen: Instant,
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

/// A committed offset plus the opaque metadata blob the client wrote
/// alongside it. librdkafka uses metadata to round-trip caller state
/// (often empty, sometimes a JSON blob); we just store what we're given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedOffset {
    pub offset: i64,
    pub metadata: Option<String>,
}

/// In-memory coordinator shared across all broker connections for a
/// particular process. Held behind `Arc<tokio::sync::RwLock<_>>` in the
/// broker.
#[derive(Debug, Default)]
pub struct GroupCoordinator {
    groups: HashMap<String, GroupMembership>,
    /// Committed offsets, keyed by `(group_id, topic, partition_index)`.
    /// Survives member disconnects — that's the point of persistence.
    offsets: HashMap<(String, String, i32), CommittedOffset>,
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

        // Evict members that haven't heartbeated recently. Without this,
        // a previous consumer that exited ungracefully holds the leader
        // slot forever, and the next consumer joins as a non-leader with
        // no assignment — so it never fetches and never commits.
        let now = Instant::now();
        entry
            .members
            .retain(|_, m| now.duration_since(m.last_seen) < STALE_MEMBER_TIMEOUT);
        if !entry.leader_id.is_empty() && !entry.members.contains_key(&entry.leader_id) {
            entry.leader_id.clear();
        }

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
                    last_seen: now,
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
                .and_modify(|m| {
                    m.metadata = metadata.clone();
                    m.last_seen = now;
                })
                .or_insert_with(|| MemberInfo {
                    member_id: requested_member_id.to_string(),
                    metadata,
                    assignment: Vec::new(),
                    last_seen: now,
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
        let now = Instant::now();
        let group = self.groups.get_mut(group_id)?;
        for (target_member, assignment) in assignments {
            if let Some(m) = group.members.get_mut(&target_member) {
                m.assignment = assignment;
            }
        }
        if let Some(m) = group.members.get_mut(member_id) {
            m.last_seen = now;
        }
        group.members.get(member_id).map(|m| m.assignment.clone())
    }

    /// Remove a member from the group (LeaveGroup). If the member was
    /// the leader, clears the leader slot so the next JoinGroup elects
    /// a fresh leader. Committed offsets are kept — LeaveGroup doesn't
    /// delete the group's committed state, that's `DeleteGroups`.
    pub fn leave_group(&mut self, group_id: &str, member_id: &str) {
        let Some(group) = self.groups.get_mut(group_id) else {
            return;
        };
        group.members.remove(member_id);
        if group.leader_id == member_id {
            group.leader_id.clear();
        }
    }

    /// Persist a committed offset for `(group_id, topic, partition)`.
    /// Overwrites any previous commit — Kafka's protocol guarantees only
    /// that the *latest* commit wins.
    pub fn commit_offset(
        &mut self,
        group_id: &str,
        topic: &str,
        partition: i32,
        offset: i64,
        metadata: Option<String>,
    ) {
        self.offsets.insert(
            (group_id.to_string(), topic.to_string(), partition),
            CommittedOffset { offset, metadata },
        );
    }

    /// Look up a previously committed offset. Returns `None` when the
    /// group has never committed for that partition — the OffsetFetch
    /// response serializer translates that into `offset = -1`, which
    /// triggers `auto.offset.reset` on the client.
    pub fn fetch_offset(
        &self,
        group_id: &str,
        topic: &str,
        partition: i32,
    ) -> Option<CommittedOffset> {
        self.offsets.get(&(group_id.to_string(), topic.to_string(), partition)).cloned()
    }

    /// Validate a heartbeat. Returns `Ok(())` on success, `Err(error_code)`
    /// on failure (e.g. UNKNOWN_MEMBER_ID 25 / ILLEGAL_GENERATION 22).
    /// Also refreshes the member's `last_seen` on success so regularly
    /// heartbeating members don't get evicted as stale.
    pub fn heartbeat(
        &mut self,
        group_id: &str,
        generation_id: i32,
        member_id: &str,
    ) -> Result<(), i16> {
        const ERR_ILLEGAL_GENERATION: i16 = 22;
        const ERR_UNKNOWN_MEMBER_ID: i16 = 25;
        const ERR_GROUP_ID_NOT_FOUND: i16 = 69;
        let group = self.groups.get_mut(group_id).ok_or(ERR_GROUP_ID_NOT_FOUND)?;
        if group.generation_id != generation_id {
            return Err(ERR_ILLEGAL_GENERATION);
        }
        let member = group.members.get_mut(member_id).ok_or(ERR_UNKNOWN_MEMBER_ID)?;
        member.last_seen = Instant::now();
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

    #[test]
    fn new_join_evicts_stale_leader_and_promotes_fresh_member() {
        let mut coord = GroupCoordinator::new();
        let first = coord.join_group("g", "", &[("range".into(), b"m".to_vec())]);
        assert_eq!(first.leader_id, first.member_id);

        // Backdate the first member's last_seen so the pruning kicks in.
        let stale = Instant::now() - STALE_MEMBER_TIMEOUT - Duration::from_secs(1);
        coord
            .groups
            .get_mut("g")
            .unwrap()
            .members
            .get_mut(&first.member_id)
            .unwrap()
            .last_seen = stale;

        let second = coord.join_group("g", "", &[("range".into(), b"m".to_vec())]);
        assert_ne!(second.member_id, first.member_id);
        assert_eq!(
            second.leader_id, second.member_id,
            "stale leader evicted, new member is leader"
        );
        // Only the fresh member remains in the group.
        let group = coord.groups.get("g").unwrap();
        assert_eq!(group.members.len(), 1);
        assert!(group.members.contains_key(&second.member_id));
    }

    #[test]
    fn commit_then_fetch_round_trips() {
        let mut coord = GroupCoordinator::new();
        coord.commit_offset("g", "t", 0, 42, Some("meta".into()));
        let fetched = coord.fetch_offset("g", "t", 0).expect("committed offset");
        assert_eq!(fetched.offset, 42);
        assert_eq!(fetched.metadata.as_deref(), Some("meta"));
    }

    #[test]
    fn fetch_returns_none_without_prior_commit() {
        let coord = GroupCoordinator::new();
        assert!(coord.fetch_offset("g", "t", 0).is_none());
    }

    #[test]
    fn later_commit_overwrites_earlier_commit() {
        let mut coord = GroupCoordinator::new();
        coord.commit_offset("g", "t", 0, 10, None);
        coord.commit_offset("g", "t", 0, 25, Some("fresh".into()));
        let fetched = coord.fetch_offset("g", "t", 0).unwrap();
        assert_eq!(fetched.offset, 25);
        assert_eq!(fetched.metadata.as_deref(), Some("fresh"));
    }

    #[test]
    fn offsets_are_scoped_per_group_topic_partition() {
        let mut coord = GroupCoordinator::new();
        coord.commit_offset("g1", "t", 0, 1, None);
        coord.commit_offset("g2", "t", 0, 2, None);
        coord.commit_offset("g1", "t", 1, 3, None);
        assert_eq!(coord.fetch_offset("g1", "t", 0).unwrap().offset, 1);
        assert_eq!(coord.fetch_offset("g2", "t", 0).unwrap().offset, 2);
        assert_eq!(coord.fetch_offset("g1", "t", 1).unwrap().offset, 3);
        assert!(coord.fetch_offset("g1", "other", 0).is_none());
    }
}
