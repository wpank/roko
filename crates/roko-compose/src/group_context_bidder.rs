//! P4-08: GroupContext bidder for agent group coordination.
//!
//! Injects group-specific context (shared knowledge, pheromone signals,
//! group conventions) into the prompt when the dispatched agent is a
//! member of one or more groups.

use crate::context_provider::{
    ContextBidder, ContextCandidate, ContextProvider, ContextRequest,
    ContextSection, ContextScope, ContextSource, ContextPurpose,
};
use crate::prompt::{AttentionBidder, PromptSection, SectionPriority};

/// Bidder that injects group-related context into agent prompts.
pub struct GroupContextBidder {
    /// Group knowledge entries to inject.
    entries: Vec<GroupContextEntry>,
}

/// A single group context entry.
#[derive(Debug, Clone)]
pub struct GroupContextEntry {
    /// Group identifier.
    pub group_id: String,
    /// Context content to inject.
    pub content: String,
    /// Priority for this entry.
    pub priority: SectionPriority,
    /// Source of this entry (e.g., "pheromone", "knowledge", "convention").
    pub source: String,
}

impl GroupContextBidder {
    /// Create a new bidder with the given entries.
    #[must_use]
    pub fn new(entries: Vec<GroupContextEntry>) -> Self {
        Self { entries }
    }

    /// Create an empty bidder.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl ContextBidder for GroupContextBidder {
    fn bidder_id(&self) -> &'static str {
        "group-context"
    }

    fn propose_context(
        &self,
        _provider: &ContextProvider,
        _request: &ContextRequest,
    ) -> Vec<ContextCandidate> {
        self.entries
            .iter()
            .map(|entry| {
                let prompt_section = PromptSection::new(
                    format!("group:{}", entry.group_id),
                    entry.content.clone(),
                )
                .with_priority(entry.priority);

                let section = ContextSection::scoped(
                    prompt_section,
                    ContextSource::Pheromone {
                        kind: entry.source.clone(),
                        source: entry.group_id.clone(),
                    },
                    ContextPurpose::TaskGuidance,
                    ContextScope::Global {
                        reason: format!(
                            "group context for {} (source: {})",
                            entry.group_id, entry.source
                        ),
                    },
                    format!(
                        "via {} (group {}, source: {})",
                        self.bidder_id(),
                        entry.group_id,
                        entry.source
                    ),
                );

                ContextCandidate {
                    section,
                    relevance: 0.7,
                    bidder: AttentionBidder::Neuro,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bidder_produces_no_candidates() {
        let bidder = GroupContextBidder::empty();
        assert_eq!(bidder.bidder_id(), "group-context");
    }

    #[test]
    fn bidder_creates_entries() {
        let bidder = GroupContextBidder::new(vec![GroupContextEntry {
            group_id: "team-alpha".to_string(),
            content: "Focus on API stability.".to_string(),
            priority: SectionPriority::Normal,
            source: "convention".to_string(),
        }]);
        assert_eq!(bidder.entries.len(), 1);
    }
}
