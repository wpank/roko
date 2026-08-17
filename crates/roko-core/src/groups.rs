//! Persistent group identity, membership, coordination, and Bus-event contracts.
//!
//! A group is the domain-level representation of a kernel Space: it owns a
//! membership boundary and names the Bus/Store partitions used by cooperating
//! agents. Storage and transport implementations live in higher-layer crates.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stable group identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupId(pub String);

impl GroupId {
    /// Construct a group identifier from its persisted string form.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Root Bus room for this group's Space partition.
    #[must_use]
    pub fn room(&self) -> String {
        format!("group:{}", self.0)
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Stable invitation identifier.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvitationId(pub String);

impl InvitationId {
    /// Construct an invitation identifier from its persisted string form.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InvitationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Persistent group identity and policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub owner: String,
    #[serde(default)]
    pub members: Vec<GroupMember>,
    pub coordination: CoordinationMode,
    #[serde(default)]
    pub config: GroupConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Group {
    /// Find a member by stable agent identifier.
    #[must_use]
    pub fn member(&self, agent_id: &str) -> Option<&GroupMember> {
        self.members
            .iter()
            .find(|member| member.agent_id == agent_id)
    }

    /// Whether an agent can read this group's Space partitions.
    #[must_use]
    pub fn can_read(&self, agent_id: &str) -> bool {
        self.member(agent_id)
            .is_some_and(|member| member.permissions.read)
    }

    /// Whether an agent can write this group's Space partitions.
    #[must_use]
    pub fn can_write(&self, agent_id: &str) -> bool {
        self.member(agent_id)
            .is_some_and(|member| member.permissions.write)
    }
}

/// Agent membership within a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMember {
    pub agent_id: String,
    pub owner: String,
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub joined_at: DateTime<Utc>,
}

/// Authority level assigned to a group member.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Leader,
    #[default]
    Member,
    Observer,
}

/// Space-partition permissions for a group member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemberPermissions {
    /// Full participation permissions.
    pub const FULL: Self = Self {
        read: true,
        write: true,
        execute: true,
    };

    /// Read-only observer permissions.
    pub const READ_ONLY: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
}

impl Default for MemberPermissions {
    fn default() -> Self {
        Self::FULL
    }
}

/// Policy knobs attached to a group Space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupConfig {
    #[serde(default)]
    pub max_members: Option<usize>,
    #[serde(default)]
    pub auto_accept: bool,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub knowledge_policy: KnowledgePolicy,
    #[serde(default = "default_pheromone_decay_rate")]
    pub pheromone_decay_rate: f64,
}

const fn default_pheromone_decay_rate() -> f64 {
    1.0
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            max_members: None,
            auto_accept: false,
            public: false,
            knowledge_policy: KnowledgePolicy::default(),
            pheromone_decay_rate: default_pheromone_decay_rate(),
        }
    }
}

/// How agents coordinate within the group Space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationMode {
    #[default]
    Stigmergic,
    Pipeline,
    Broadcast,
    LeaderFollower,
}

impl FromStr for CoordinationMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "stigmergic" => Ok(Self::Stigmergic),
            "pipeline" => Ok(Self::Pipeline),
            "broadcast" => Ok(Self::Broadcast),
            "leader_follower" => Ok(Self::LeaderFollower),
            _ => Err(format!("unknown coordination mode '{value}'")),
        }
    }
}

/// Write policy for the group's shared knowledge partition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgePolicy {
    #[default]
    Open,
    WriteLeader,
    Curated,
}

impl FromStr for KnowledgePolicy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "open" => Ok(Self::Open),
            "write_leader" => Ok(Self::WriteLeader),
            "curated" => Ok(Self::Curated),
            _ => Err(format!("unknown knowledge policy '{value}'")),
        }
    }
}

/// Invitation awaiting an agent owner's decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub agent_id: String,
    pub invited_by: String,
    pub agent_owner: String,
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub status: InvitationStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Invitation lifecycle state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    Expired,
}

/// Request to invite an agent into a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRequest {
    pub agent_id: String,
    #[serde(default)]
    pub role: MemberRole,
    #[serde(default)]
    pub permissions: MemberPermissions,
}

/// Result of inviting or deciding an invitation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteResponse {
    pub status: InvitationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invitation_id: Option<InvitationId>,
    pub agent_id: String,
    pub group_id: GroupId,
}

/// Leader-follower assignment configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderConfig {
    pub leader_agent: String,
    pub assignment_strategy: AssignmentStrategy,
    pub max_concurrent_tasks: usize,
}

/// Built-in leader assignment strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStrategy {
    #[default]
    RoundRobin,
    CapabilityMatch,
    LoadBalanced,
    CascadeRouter,
}

impl FromStr for AssignmentStrategy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace('-', "_")
            .replace("rule_router:", "");
        match normalized.as_str() {
            "round_robin" => Ok(Self::RoundRobin),
            "capability_match" => Ok(Self::CapabilityMatch),
            "load_balanced" => Ok(Self::LoadBalanced),
            "cascade_router" => Ok(Self::CascadeRouter),
            _ => Err(format!("unknown assignment strategy '{value}'")),
        }
    }
}

/// Task assignment published to the coordination room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: String,
    pub assigned_to: String,
    pub assigned_by: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
}

/// Task completion published to the coordination room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCompletion {
    pub task_id: String,
    pub completed_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_id: Option<String>,
    pub duration_secs: u64,
}

/// Pheromone deposited in a group's shared Store partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupPheromone {
    pub group_id: GroupId,
    pub depositor: String,
    pub signal_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_hint: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub deposited_at: DateTime<Utc>,
}

/// Request to deposit a group pheromone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PheromoneDeposit {
    pub group_id: GroupId,
    pub signal_type: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Filter for querying a pheromone field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PheromoneQuery {
    pub group_id: GroupId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_balance: Option<f64>,
}

/// Summary of the active pheromone field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PheromoneFieldSummary {
    pub group_id: GroupId,
    pub count: usize,
    pub types: Vec<String>,
}

/// Bid weights used when group context competes for prompt space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupContextBidder {
    pub group_id: GroupId,
    pub pheromone_weight: f64,
    pub knowledge_weight: f64,
    pub coordination_weight: f64,
}

impl GroupContextBidder {
    /// Score normalized group signals for attention allocation.
    #[must_use]
    pub fn bid_value(
        &self,
        pheromone_intensity: f64,
        knowledge_recency: f64,
        coordination_urgency: f64,
    ) -> f64 {
        let weighted = self.pheromone_weight.max(0.0) * pheromone_intensity.clamp(0.0, 1.0)
            + self.knowledge_weight.max(0.0) * knowledge_recency.clamp(0.0, 1.0)
            + self.coordination_weight.max(0.0) * coordination_urgency.clamp(0.0, 1.0);
        if weighted.is_finite() { weighted } else { 0.0 }
    }
}

/// Typed event published to a group's Bus partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GroupEvent {
    Created {
        group_id: GroupId,
        name: String,
        owner: String,
    },
    Updated {
        group_id: GroupId,
        changes: serde_json::Value,
    },
    Deleted {
        group_id: GroupId,
        owner: String,
    },
    MemberInvited {
        agent_id: String,
        invited_by: String,
        role: MemberRole,
    },
    MemberJoined {
        agent_id: String,
        owner: String,
        role: MemberRole,
    },
    MemberLeft {
        agent_id: String,
        reason: String,
    },
    MemberUpdated {
        agent_id: String,
        changes: serde_json::Value,
    },
    Message {
        from: String,
        content: String,
        tags: Vec<String>,
    },
    ClusterStarted {
        cluster_id: String,
        pipeline: serde_json::Value,
        agents: Vec<String>,
    },
    ClusterCompleted {
        cluster_id: String,
        outcome: String,
        duration_secs: u64,
    },
    KnowledgePublished {
        entry_id: String,
        author: String,
        topic: String,
    },
    KnowledgeValidated {
        entry_id: String,
        validator: String,
    },
    PheromoneDeposited {
        depositor: String,
        signal_type: String,
        intensity: f64,
    },
    PheromoneDecayed {
        count_removed: usize,
        threshold: f64,
    },
    TaskAssigned {
        task_id: String,
        assigned_to: String,
        assigned_by: String,
    },
    TaskCompleted {
        task_id: String,
        completed_by: String,
        result: serde_json::Value,
    },
}

impl GroupEvent {
    /// Resolve the Bus room for this event.
    #[must_use]
    pub fn room(&self, group_id: &GroupId) -> String {
        match self {
            Self::Created { .. } | Self::Deleted { .. } => "system".to_string(),
            Self::KnowledgePublished { .. } | Self::KnowledgeValidated { .. } => {
                format!("group:{group_id}:knowledge")
            }
            Self::PheromoneDeposited { .. } | Self::PheromoneDecayed { .. } => {
                format!("group:{group_id}:pheromones")
            }
            Self::TaskAssigned { .. } | Self::TaskCompleted { .. } => {
                format!("group:{group_id}:coordination")
            }
            _ => group_id.room(),
        }
    }

    /// Stable event type used in Bus metadata and API responses.
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::Created { .. } => "group.created",
            Self::Updated { .. } => "group.updated",
            Self::Deleted { .. } => "group.deleted",
            Self::MemberInvited { .. } => "group.member_invited",
            Self::MemberJoined { .. } => "group.member_joined",
            Self::MemberLeft { .. } => "group.member_left",
            Self::MemberUpdated { .. } => "group.member_updated",
            Self::Message { .. } => "group.message",
            Self::ClusterStarted { .. } => "group.cluster_started",
            Self::ClusterCompleted { .. } => "group.cluster_completed",
            Self::KnowledgePublished { .. } => "group.knowledge_published",
            Self::KnowledgeValidated { .. } => "group.knowledge_validated",
            Self::PheromoneDeposited { .. } => "group.pheromone_deposited",
            Self::PheromoneDecayed { .. } => "group.pheromone_decayed",
            Self::TaskAssigned { .. } => "group.task_assigned",
            Self::TaskCompleted { .. } => "group.task_completed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_use_stable_snake_case_wire_names() {
        assert_eq!(
            serde_json::to_string(&CoordinationMode::LeaderFollower).unwrap(),
            "\"leader_follower\""
        );
        assert_eq!(
            serde_json::to_string(&KnowledgePolicy::WriteLeader).unwrap(),
            "\"write_leader\""
        );
        assert_eq!(
            CoordinationMode::from_str("leader-follower").unwrap(),
            CoordinationMode::LeaderFollower
        );
    }

    #[test]
    fn group_event_routes_to_its_space_partition() {
        let group_id = GroupId::new("g-1");
        let knowledge = GroupEvent::KnowledgeValidated {
            entry_id: "k".into(),
            validator: "a".into(),
        };
        let pheromone = GroupEvent::PheromoneDecayed {
            count_removed: 2,
            threshold: 0.1,
        };
        let member = GroupEvent::MemberLeft {
            agent_id: "a".into(),
            reason: "done".into(),
        };
        assert_eq!(knowledge.room(&group_id), "group:g-1:knowledge");
        assert_eq!(pheromone.room(&group_id), "group:g-1:pheromones");
        assert_eq!(member.room(&group_id), "group:g-1");
    }

    #[test]
    fn bidder_clamps_adversarial_inputs_and_rejects_non_finite_output() {
        let bidder = GroupContextBidder {
            group_id: GroupId::new("g"),
            pheromone_weight: 2.0,
            knowledge_weight: -10.0,
            coordination_weight: f64::INFINITY,
        };
        assert_eq!(bidder.bid_value(2.0, 1.0, 1.0), 0.0);
        let bidder = GroupContextBidder {
            coordination_weight: 3.0,
            ..bidder
        };
        assert_eq!(bidder.bid_value(2.0, 1.0, 0.5), 3.5);
    }
}
