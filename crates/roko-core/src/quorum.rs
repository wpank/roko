//! P4-22: Quorum sensing for agent group coordination.
//!
//! Implements quorum sensing primitives: agents within a group can sense
//! the collective state (number of active agents, consensus on actions,
//! pheromone levels) and adjust their behavior accordingly.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quorum sensing state for an agent group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumState {
    /// Group identifier.
    pub group_id: String,
    /// Total members in the group.
    pub total_members: usize,
    /// Currently active (recently heartbeated) members.
    pub active_members: usize,
    /// Per-action vote counts.
    pub votes: HashMap<String, ActionVote>,
    /// Pheromone levels for different signals.
    pub pheromones: HashMap<String, f64>,
    /// Whether the group has reached quorum.
    pub has_quorum: bool,
    /// Required fraction for quorum (default 0.5).
    pub quorum_fraction: f64,
}

/// Votes accumulated for a proposed action.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionVote {
    /// Number of votes in favor.
    pub in_favor: u32,
    /// Number of votes against.
    pub against: u32,
    /// Agents who have voted (by ID).
    pub voters: Vec<String>,
}

impl ActionVote {
    /// Total votes cast.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.in_favor + self.against
    }

    /// Approval ratio (fraction of in-favor votes).
    #[must_use]
    pub fn approval_ratio(&self) -> f64 {
        if self.total() == 0 {
            return 0.0;
        }
        self.in_favor as f64 / self.total() as f64
    }

    /// Whether the action has been approved by majority.
    #[must_use]
    pub fn is_approved(&self, threshold: f64) -> bool {
        self.total() > 0 && self.approval_ratio() >= threshold
    }
}

/// Result of a quorum check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumCheck {
    /// Quorum is reached; the group can take coordinated action.
    Reached,
    /// Not enough active members for quorum.
    InsufficientMembers {
        /// Members needed.
        needed: usize,
        /// Members active.
        active: usize,
    },
    /// Group is empty.
    NoMembers,
}

impl Default for QuorumState {
    fn default() -> Self {
        Self {
            group_id: String::new(),
            total_members: 0,
            active_members: 0,
            votes: HashMap::new(),
            pheromones: HashMap::new(),
            has_quorum: false,
            quorum_fraction: 0.5,
        }
    }
}

impl QuorumState {
    /// Create a new quorum state for a group.
    #[must_use]
    pub fn new(group_id: impl Into<String>, total_members: usize) -> Self {
        Self {
            group_id: group_id.into(),
            total_members,
            quorum_fraction: 0.5,
            ..Self::default()
        }
    }

    /// Create with a custom quorum fraction.
    #[must_use]
    pub fn with_quorum_fraction(mut self, fraction: f64) -> Self {
        self.quorum_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    /// Update the active member count and recheck quorum.
    pub fn update_active(&mut self, active_members: usize) {
        self.active_members = active_members;
        self.has_quorum = self.check_quorum() == QuorumCheck::Reached;
    }

    /// Check whether quorum is currently met.
    #[must_use]
    pub fn check_quorum(&self) -> QuorumCheck {
        if self.total_members == 0 {
            return QuorumCheck::NoMembers;
        }
        let needed = ((self.total_members as f64 * self.quorum_fraction).ceil()) as usize;
        if self.active_members >= needed {
            QuorumCheck::Reached
        } else {
            QuorumCheck::InsufficientMembers {
                needed,
                active: self.active_members,
            }
        }
    }

    /// Cast a vote for an action.
    pub fn vote(&mut self, action: &str, agent_id: &str, in_favor: bool) {
        let vote = self.votes.entry(action.to_string()).or_default();
        if vote.voters.contains(&agent_id.to_string()) {
            return; // Already voted.
        }
        vote.voters.push(agent_id.to_string());
        if in_favor {
            vote.in_favor += 1;
        } else {
            vote.against += 1;
        }
    }

    /// Check whether an action has been approved by the group.
    #[must_use]
    pub fn is_action_approved(&self, action: &str, approval_threshold: f64) -> bool {
        self.votes
            .get(action)
            .is_some_and(|v| v.is_approved(approval_threshold))
    }

    /// Emit a pheromone signal.
    pub fn emit_pheromone(&mut self, signal: &str, strength: f64) {
        let current = self.pheromones.entry(signal.to_string()).or_insert(0.0);
        *current = (*current + strength).min(10.0);
    }

    /// Read the current pheromone level for a signal.
    #[must_use]
    pub fn pheromone_level(&self, signal: &str) -> f64 {
        self.pheromones.get(signal).copied().unwrap_or(0.0)
    }

    /// Decay all pheromone levels by a multiplicative factor.
    pub fn decay_pheromones(&mut self, decay_rate: f64) {
        let decay = decay_rate.clamp(0.0, 1.0);
        for level in self.pheromones.values_mut() {
            *level *= decay;
        }
        // Remove negligible pheromones.
        self.pheromones.retain(|_, v| *v > 0.001);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quorum_reached_when_majority_active() {
        let mut state = QuorumState::new("group-1", 5);
        state.update_active(3);
        assert!(state.has_quorum);
        assert_eq!(state.check_quorum(), QuorumCheck::Reached);
    }

    #[test]
    fn quorum_not_reached_when_minority_active() {
        let mut state = QuorumState::new("group-1", 5);
        state.update_active(2);
        assert!(!state.has_quorum);
        assert!(matches!(
            state.check_quorum(),
            QuorumCheck::InsufficientMembers { .. }
        ));
    }

    #[test]
    fn voting_works() {
        let mut state = QuorumState::new("group-1", 3);
        state.vote("deploy", "agent-1", true);
        state.vote("deploy", "agent-2", true);
        state.vote("deploy", "agent-3", false);

        assert!(state.is_action_approved("deploy", 0.5));
        assert!(!state.is_action_approved("deploy", 0.8));
    }

    #[test]
    fn duplicate_votes_ignored() {
        let mut state = QuorumState::new("group-1", 3);
        state.vote("deploy", "agent-1", true);
        state.vote("deploy", "agent-1", false); // Duplicate, ignored.
        let vote = &state.votes["deploy"];
        assert_eq!(vote.total(), 1);
        assert_eq!(vote.in_favor, 1);
    }

    #[test]
    fn pheromone_emit_and_decay() {
        let mut state = QuorumState::new("group-1", 3);
        state.emit_pheromone("danger", 1.0);
        assert!((state.pheromone_level("danger") - 1.0).abs() < 1e-10);

        state.decay_pheromones(0.5);
        assert!((state.pheromone_level("danger") - 0.5).abs() < 1e-10);
    }
}
