use std::collections::HashMap;

/// Manages consumer groups for the Kafka broker
#[derive(Debug)]
pub struct ConsumerGroupManager {
    groups: HashMap<String, ConsumerGroup>,
}

#[derive(Debug)]
pub struct ConsumerGroup {
    pub group_id: String,
    pub members: HashMap<String, GroupMember>,
    pub coordinator: GroupCoordinator,
    pub offsets: HashMap<(String, i32), i64>, // (topic, partition) -> offset
}

#[derive(Debug)]
pub struct GroupMember {
    pub member_id: String,
    pub client_id: String,
    pub assignment: Vec<PartitionAssignment>,
}

#[derive(Debug, Clone)]
pub struct PartitionAssignment {
    pub topic: String,
    pub partitions: Vec<i32>,
}

#[derive(Debug)]
pub struct GroupCoordinator {
    pub coordinator_id: i32,
    pub host: String,
    pub port: i32,
}

impl ConsumerGroupManager {
    /// Create a new consumer group manager
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }
}

impl Default for ConsumerGroupManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsumerGroupManager {
    /// Get a reference to all groups (for internal use)
    pub fn groups(&self) -> &HashMap<String, ConsumerGroup> {
        &self.groups
    }

    /// Join a consumer group
    pub async fn join_group(
        &mut self,
        group_id: &str,
        member_id: &str,
        client_id: &str,
    ) -> Result<JoinGroupResponse> {
        let group = self.groups.entry(group_id.to_string()).or_insert_with(|| ConsumerGroup {
            group_id: group_id.to_string(),
            members: HashMap::new(),
            coordinator: GroupCoordinator {
                coordinator_id: 1,
                host: "localhost".to_string(),
                port: 9092,
            },
            offsets: HashMap::new(),
        });

        group.members.insert(
            member_id.to_string(),
            GroupMember {
                member_id: member_id.to_string(),
                client_id: client_id.to_string(),
                assignment: vec![],
            },
        );

        Ok(JoinGroupResponse {
            generation_id: 1,
            protocol_name: "consumer".to_string(),
            leader: member_id.to_string(),
            member_id: member_id.to_string(),
            members: group.members.keys().cloned().collect(),
        })
    }

    /// Sync group assignment
    pub async fn sync_group(
        &mut self,
        group_id: &str,
        assignments: Vec<PartitionAssignment>,
        topics: &std::collections::HashMap<String, crate::topics::Topic>,
    ) -> Result<()> {
        if let Some(group) = self.groups.get_mut(group_id) {
            // If assignments are provided, use them (leader assignment)
            if !assignments.is_empty() {
                // Distribute assignments to members
                for assignment in assignments {
                    // For simplicity, assign to all members (in real Kafka, leader assigns specific partitions to specific members)
                    for member in group.members.values_mut() {
                        member.assignment.push(assignment.clone());
                    }
                }
            } else {
                // Simple round-robin assignment if no assignments provided
                Self::assign_partitions_round_robin(group, topics);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Group {} does not exist", group_id))
        }
    }

    /// Assign partitions to group members using round-robin strategy
    fn assign_partitions_round_robin(
        group: &mut ConsumerGroup,
        topics: &std::collections::HashMap<String, crate::topics::Topic>,
    ) {
        // Clear existing assignments for rebalance
        for member in group.members.values_mut() {
            member.assignment.clear();
        }

        let mut member_ids: Vec<String> = group.members.keys().cloned().collect();
        member_ids.sort(); // Sort for deterministic assignment

        let mut member_idx = 0;
        for (topic_name, topic) in topics {
            let num_partitions = topic.config.num_partitions as usize;
            for partition_id in 0..num_partitions {
                let member_id = &member_ids[member_idx % member_ids.len()];
                if let Some(member) = group.members.get_mut(member_id.as_str()) {
                    // Find or create assignment for this topic
                    let assignment = member.assignment.iter_mut().find(|a| a.topic == *topic_name);
                    if let Some(assignment) = assignment {
                        assignment.partitions.push(partition_id as i32);
                    } else {
                        member.assignment.push(PartitionAssignment {
                            topic: topic_name.clone(),
                            partitions: vec![partition_id as i32],
                        });
                    }
                }
                member_idx += 1;
            }
        }
    }

    /// Commit consumer offsets
    pub async fn commit_offsets(
        &mut self,
        group_id: &str,
        offsets: HashMap<(String, i32), i64>,
    ) -> Result<()> {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.offsets.extend(offsets);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Group {} does not exist", group_id))
        }
    }

    /// Get committed offsets for a group
    pub fn get_committed_offsets(&self, group_id: &str) -> HashMap<(String, i32), i64> {
        self.groups.get(group_id).map(|g| g.offsets.clone()).unwrap_or_default()
    }

    /// Simulate consumer lag
    pub async fn simulate_lag(
        &mut self,
        group_id: &str,
        topic: &str,
        lag: i64,
        topics: &std::collections::HashMap<String, crate::topics::Topic>,
    ) {
        if let Some(group) = self.groups.get_mut(group_id) {
            // Get actual partition count from topics
            let num_partitions =
                topics.get(topic).map(|t| t.config.num_partitions).unwrap_or(1) as usize;
            // Simulate lag by setting committed offsets behind
            for partition in 0..num_partitions {
                let key = (topic.to_string(), partition as i32);
                let current_offset = group.offsets.get(&key).copied().unwrap_or(0);
                group.offsets.insert(key, current_offset.saturating_sub(lag));
            }
            tracing::info!(
                "Simulated lag of {} messages for group {} on topic {}",
                lag,
                group_id,
                topic
            );
        }
    }

    /// Trigger rebalance for a group
    pub async fn trigger_rebalance(&mut self, group_id: &str) {
        if let Some(group) = self.groups.get_mut(group_id) {
            // Simulate rebalance by clearing assignments and forcing rejoin
            for member in group.members.values_mut() {
                member.assignment.clear();
            }
            tracing::info!("Triggered rebalance for group {}", group_id);
        }
    }

    /// Reset consumer offsets
    pub async fn reset_offsets(
        &mut self,
        group_id: &str,
        topic: &str,
        to: &str,
        topics: &std::collections::HashMap<String, crate::topics::Topic>,
    ) {
        if let Some(group) = self.groups.get_mut(group_id) {
            if let Some(topic_data) = topics.get(topic) {
                let num_partitions = topic_data.config.num_partitions as usize;
                for partition in 0..num_partitions {
                    let key = (topic.to_string(), partition as i32);
                    let target_offset = match to {
                        "earliest" => 0,
                        "latest" => topic_data.partitions[partition].high_watermark,
                        _ => return, // Invalid reset target
                    };
                    group.offsets.insert(key, target_offset);
                }
                tracing::info!("Reset offsets for group {} on topic {} to {}", group_id, topic, to);
            }
        }
    }
}

/// Response for join group request
#[derive(Debug)]
pub struct JoinGroupResponse {
    pub generation_id: i32,
    pub protocol_name: String,
    pub leader: String,
    pub member_id: String,
    pub members: Vec<String>,
}

type Result<T> = std::result::Result<T, anyhow::Error>;
