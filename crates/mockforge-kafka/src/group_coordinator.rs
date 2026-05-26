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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedOffset {
    pub offset: i64,
    pub metadata: Option<String>,
}

/// On-disk snapshot of all committed offsets. Just a flat list of
/// `(group_id, topic, partition, offset, metadata)` rows; small,
/// human-readable, easy to diff in a recovery scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OffsetSnapshot {
    /// Schema version — bump if the on-disk shape changes.
    version: u32,
    entries: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotEntry {
    group_id: String,
    topic: String,
    partition: i32,
    offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<String>,
}

/// In-memory coordinator shared across all broker connections for a
/// particular process. Held behind `Arc<tokio::sync::RwLock<_>>` in the
/// broker.
///
/// Offset persistence (#676): when constructed with `new_with_persistence`,
/// every `commit_offset` writes the full offset table back to disk and
/// startup reads any prior snapshot back into memory. Without a path the
/// behaviour is unchanged — offsets live only in process memory.
#[derive(Debug, Default)]
pub struct GroupCoordinator {
    groups: HashMap<String, GroupMembership>,
    /// Committed offsets, keyed by `(group_id, topic, partition_index)`.
    /// Survives member disconnects — that's the point of persistence.
    offsets: HashMap<(String, String, i32), CommittedOffset>,
    /// Optional on-disk snapshot location. When set, `commit_offset`
    /// serialises the offset table to JSON here after every write.
    persistence_path: Option<PathBuf>,
}

impl GroupCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a coordinator that mirrors its offset table to a JSON
    /// snapshot at `path`. If the file already exists it's read into memory
    /// at startup so consumers reconnecting after a restart resume from
    /// their last committed offset instead of falling back to
    /// `auto.offset.reset`.
    ///
    /// The snapshot format is `{ version, entries: [...] }` — small enough
    /// to be human-diffable in a recovery scenario. A corrupt or unreadable
    /// snapshot is logged at WARN and treated as an empty start (the broker
    /// must not refuse to come up because of a bad snapshot — that's
    /// strictly worse than ephemeral offsets).
    pub fn new_with_persistence(path: PathBuf) -> Self {
        let mut coord = Self {
            persistence_path: Some(path.clone()),
            ..Self::default()
        };
        coord.load_snapshot();
        coord
    }

    fn load_snapshot(&mut self) {
        let Some(ref path) = self.persistence_path else {
            return;
        };
        if !path.exists() {
            tracing::debug!(path = %path.display(), "no Kafka offset snapshot to load");
            return;
        }
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "could not read Kafka offset snapshot; starting empty");
                return;
            }
        };
        let snapshot: OffsetSnapshot = match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "Kafka offset snapshot is malformed; starting empty");
                return;
            }
        };
        for entry in snapshot.entries {
            self.offsets.insert(
                (entry.group_id, entry.topic, entry.partition),
                CommittedOffset {
                    offset: entry.offset,
                    metadata: entry.metadata,
                },
            );
        }
        tracing::info!(
            path = %path.display(),
            count = self.offsets.len(),
            "loaded Kafka offset snapshot"
        );
    }

    fn persist_snapshot(&self) {
        let Some(ref path) = self.persistence_path else {
            return;
        };
        let entries: Vec<SnapshotEntry> = self
            .offsets
            .iter()
            .map(|((g, t, p), c)| SnapshotEntry {
                group_id: g.clone(),
                topic: t.clone(),
                partition: *p,
                offset: c.offset,
                metadata: c.metadata.clone(),
            })
            .collect();
        let snapshot = OffsetSnapshot {
            version: 1,
            entries,
        };

        let json = match serde_json::to_string_pretty(&snapshot) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialise Kafka offset snapshot");
                return;
            }
        };

        // Atomic write via tmp-then-rename so a crash mid-write doesn't
        // leave a truncated snapshot file.
        let tmp = path.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp, &json) {
            tracing::warn!(path = %tmp.display(), error = %e, "failed to write Kafka offset snapshot tmp");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            tracing::warn!(
                from = %tmp.display(),
                to = %path.display(),
                error = %e,
                "failed to rename Kafka offset snapshot tmp"
            );
        }
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
    ///
    /// When the coordinator was constructed with `new_with_persistence`,
    /// the full offset table is mirrored to disk after the in-memory
    /// update. The persist call swallows errors (logged at WARN); a
    /// broken snapshot store must not block consumer commits.
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
        if self.persistence_path.is_some() {
            self.persist_snapshot();
        }
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

    // Offset persistence (#676)

    #[test]
    fn persistence_path_none_means_no_disk_io() {
        let mut coord = GroupCoordinator::new();
        // Sanity: no panic, no file created, no env access.
        coord.commit_offset("g", "t", 0, 1, None);
        assert_eq!(coord.fetch_offset("g", "t", 0).unwrap().offset, 1);
    }

    #[test]
    fn commit_writes_snapshot_to_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("offsets.json");
        {
            let mut coord = GroupCoordinator::new_with_persistence(path.clone());
            coord.commit_offset("g1", "topic-a", 0, 42, Some("checkpoint-1".into()));
            coord.commit_offset("g1", "topic-a", 1, 7, None);
        }
        let raw = std::fs::read_to_string(&path).expect("snapshot should be written");
        assert!(raw.contains("\"group_id\": \"g1\""));
        assert!(raw.contains("\"topic\": \"topic-a\""));
        assert!(raw.contains("\"offset\": 42"));
        assert!(raw.contains("\"checkpoint-1\""));
    }

    #[test]
    fn snapshot_round_trips_across_restart() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("offsets.json");

        {
            let mut coord = GroupCoordinator::new_with_persistence(path.clone());
            coord.commit_offset("orders-consumer", "orders", 0, 1024, Some("cp-A".into()));
            coord.commit_offset("orders-consumer", "orders", 1, 2048, None);
            coord.commit_offset("invoices-consumer", "invoices", 0, 99, Some("cp-B".into()));
        }

        // Simulate broker restart: fresh coordinator pointed at the same snapshot.
        let reborn = GroupCoordinator::new_with_persistence(path);
        assert_eq!(reborn.fetch_offset("orders-consumer", "orders", 0).unwrap().offset, 1024);
        assert_eq!(
            reborn.fetch_offset("orders-consumer", "orders", 0).unwrap().metadata.as_deref(),
            Some("cp-A")
        );
        assert_eq!(reborn.fetch_offset("orders-consumer", "orders", 1).unwrap().offset, 2048);
        assert_eq!(reborn.fetch_offset("invoices-consumer", "invoices", 0).unwrap().offset, 99);
    }

    #[test]
    fn malformed_snapshot_starts_empty_rather_than_panicking() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("offsets.json");
        std::fs::write(&path, "this is not json").unwrap();
        // Must not panic; recovery is silent (file is logged at WARN).
        let coord = GroupCoordinator::new_with_persistence(path);
        assert!(coord.fetch_offset("any", "any", 0).is_none());
    }

    #[test]
    fn later_commit_replaces_disk_record() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("offsets.json");
        {
            let mut coord = GroupCoordinator::new_with_persistence(path.clone());
            coord.commit_offset("g", "t", 0, 5, None);
            coord.commit_offset("g", "t", 0, 99, Some("latest".into()));
        }
        let reborn = GroupCoordinator::new_with_persistence(path);
        let fetched = reborn.fetch_offset("g", "t", 0).unwrap();
        assert_eq!(fetched.offset, 99);
        assert_eq!(fetched.metadata.as_deref(), Some("latest"));
    }
}
