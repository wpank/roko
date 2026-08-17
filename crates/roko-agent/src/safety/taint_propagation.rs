//! Monotonic trust-origin taint propagation across signal lineage.

use parking_lot::Mutex;
use roko_core::{ContentHash, Signal, Taint};
use std::collections::HashMap;

pub use roko_core::provenance::TrustOriginTaintLevel;

/// Compatibility name used by the E34 trust-origin IFC contract.
pub type TaintLevel = TrustOriginTaintLevel;

/// Why a hash is tracked as tainted, independent of its lattice level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaintReason {
    ExternalSource { detail: String },
    UserInput { detail: String },
    ToolFailure { detail: String },
    Propagated,
    Stale { detail: String },
    Custom { category: String, detail: String },
}

impl TaintReason {
    pub fn external(detail: impl Into<String>) -> Self {
        Self::ExternalSource {
            detail: detail.into(),
        }
    }

    pub fn user_input(detail: impl Into<String>) -> Self {
        Self::UserInput {
            detail: detail.into(),
        }
    }

    pub fn tool_failure(detail: impl Into<String>) -> Self {
        Self::ToolFailure {
            detail: detail.into(),
        }
    }

    pub fn stale(detail: impl Into<String>) -> Self {
        Self::Stale {
            detail: detail.into(),
        }
    }

    pub fn custom(category: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Custom {
            category: category.into(),
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn category(&self) -> &str {
        match self {
            Self::ExternalSource { .. } => "external_source",
            Self::UserInput { .. } => "user_input",
            Self::ToolFailure { .. } => "tool_failure",
            Self::Propagated => "propagated",
            Self::Stale { .. } => "stale",
            Self::Custom { category, .. } => category,
        }
    }
}

#[derive(Clone, Debug)]
struct TaintEntry {
    level: TaintLevel,
    reason: TaintReason,
    derived_from: Vec<ContentHash>,
}

/// Durable-in-memory evidence emitted for each successful propagation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropagationAudit {
    pub hash: ContentHash,
    pub parents: Vec<ContentHash>,
    pub resulting_level: TaintLevel,
}

#[derive(Debug, Default)]
struct TrackerState {
    taints: HashMap<ContentHash, TaintEntry>,
    audit: Vec<PropagationAudit>,
}

/// Thread-safe tracker for taint across a derived-signal DAG.
#[derive(Debug, Default)]
pub struct TaintTracker {
    state: Mutex<TrackerState>,
}

/// Join all input trust-origin levels. Empty input is trusted.
#[must_use]
pub fn propagate_taint(inputs: &[TaintLevel]) -> TaintLevel {
    inputs
        .iter()
        .copied()
        .fold(TaintLevel::Trusted, TaintLevel::join)
}

impl TaintTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a hash without ever lowering an existing lattice level.
    pub fn mark_tainted(&self, hash: ContentHash, reason: TaintReason, level: TaintLevel) {
        let mut state = self.state.lock();
        state
            .taints
            .entry(hash)
            .and_modify(|entry| {
                entry.level = entry.level.join(level);
                entry.reason = reason.clone();
            })
            .or_insert(TaintEntry {
                level,
                reason,
                derived_from: Vec::new(),
            });
    }

    #[must_use]
    pub fn get_level(&self, hash: &ContentHash) -> Option<TaintLevel> {
        self.state.lock().taints.get(hash).map(|entry| entry.level)
    }

    /// Return both boolean taint status and its trust-origin lattice level.
    #[must_use]
    pub fn is_tainted(&self, hash: &ContentHash) -> (bool, Option<TaintLevel>) {
        let level = self.get_level(hash);
        (level.is_some(), level)
    }

    #[must_use]
    pub fn reason(&self, hash: &ContentHash) -> Option<TaintReason> {
        self.state
            .lock()
            .taints
            .get(hash)
            .map(|entry| entry.reason.clone())
    }

    /// Immediate tainted parents recorded for a derived hash.
    #[must_use]
    pub fn derived_from(&self, hash: &ContentHash) -> Vec<ContentHash> {
        self.state
            .lock()
            .taints
            .get(hash)
            .map_or_else(Vec::new, |entry| entry.derived_from.clone())
    }

    /// Propagate the join of all tracked parent levels to `child`.
    pub fn propagate(&self, parents: &[ContentHash], child: ContentHash) -> bool {
        let mut state = self.state.lock();
        let tainted_parents: Vec<(ContentHash, TaintLevel)> = parents
            .iter()
            .filter_map(|hash| state.taints.get(hash).map(|entry| (*hash, entry.level)))
            .collect();
        if tainted_parents.is_empty() {
            return false;
        }

        let inherited = tainted_parents
            .iter()
            .map(|(_, level)| *level)
            .fold(TaintLevel::Trusted, TaintLevel::join);
        let entry = state.taints.entry(child).or_insert(TaintEntry {
            level: TaintLevel::Trusted,
            reason: TaintReason::Propagated,
            derived_from: Vec::new(),
        });
        entry.level = entry.level.join(inherited);
        for (parent, _) in &tainted_parents {
            if !entry.derived_from.contains(parent) {
                entry.derived_from.push(*parent);
            }
        }
        let resulting_level = entry.level;
        state.audit.push(PropagationAudit {
            hash: child,
            parents: parents.to_vec(),
            resulting_level,
        });
        drop(state);

        tracing::info!(
            hash = %child,
            parents = ?parents,
            resulting_level = %resulting_level,
            "taint propagated to derived signal"
        );
        true
    }

    /// Register a signal from its provenance trust decision.
    pub fn observe_signal(&self, signal: &Signal) -> bool {
        let level = signal.effective_taint();
        if !signal.provenance.is_tainted() && level == TaintLevel::Trusted {
            return false;
        }
        let reason = match &signal.provenance.taint {
            Taint::Clean => TaintReason::custom("origin", signal.provenance.author.clone()),
            Taint::UserInput { detail } => TaintReason::user_input(detail.clone()),
            Taint::UnverifiedSource { detail } => TaintReason::external(detail.clone()),
            Taint::ToolFailure { detail } => TaintReason::tool_failure(detail.clone()),
            Taint::StaleData { threshold_ms } => {
                TaintReason::stale(format!("older than {threshold_ms}ms"))
            }
            Taint::Propagated { .. } => TaintReason::Propagated,
            other => TaintReason::custom(
                other.category(),
                other.detail().unwrap_or("tainted provenance"),
            ),
        };
        self.mark_tainted(signal.id, reason, level);
        true
    }

    #[must_use]
    pub fn audit_log(&self) -> Vec<PropagationAudit> {
        self.state.lock().audit.clone()
    }

    pub fn clear(&self) {
        *self.state.lock() = TrackerState::default();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.state.lock().taints.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.state.lock().taints.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Kind, Provenance};

    fn hash(value: &[u8]) -> ContentHash {
        ContentHash::of(value)
    }

    #[test]
    fn explicit_marks_are_monotonic_and_queryable() {
        let tracker = TaintTracker::new();
        let id = hash(b"source");
        tracker.mark_tainted(id, TaintReason::external("api"), TaintLevel::External);
        tracker.mark_tainted(id, TaintReason::user_input("retry"), TaintLevel::Local);

        assert_eq!(tracker.is_tainted(&id), (true, Some(TaintLevel::External)));
        assert_eq!(tracker.reason(&id).unwrap().category(), "user_input");
    }

    #[test]
    fn propagation_joins_parents_tracks_derivation_and_audit() {
        let tracker = TaintTracker::new();
        let local = hash(b"local");
        let hostile = hash(b"hostile");
        let child = hash(b"child");
        tracker.mark_tainted(local, TaintReason::user_input("cli"), TaintLevel::Local);
        tracker.mark_tainted(
            hostile,
            TaintReason::external("unsigned"),
            TaintLevel::Untrusted,
        );

        assert!(tracker.propagate(&[local, hostile], child));
        assert_eq!(tracker.get_level(&child), Some(TaintLevel::Untrusted));
        assert_eq!(tracker.derived_from(&child), vec![local, hostile]);
        assert_eq!(
            tracker.audit_log(),
            vec![PropagationAudit {
                hash: child,
                parents: vec![local, hostile],
                resulting_level: TaintLevel::Untrusted,
            }]
        );
    }

    #[test]
    fn clean_parents_do_not_create_taint_or_audit_evidence() {
        let tracker = TaintTracker::new();
        let child = hash(b"clean child");
        assert!(!tracker.propagate(&[hash(b"clean")], child));
        assert_eq!(tracker.is_tainted(&child), (false, None));
        assert!(tracker.audit_log().is_empty());
    }

    #[test]
    fn signal_observation_uses_effective_trust_origin() {
        let tracker = TaintTracker::new();
        let signal = Signal::builder(Kind::Task)
            .provenance(Provenance::external("webhook"))
            .build();
        assert!(tracker.observe_signal(&signal));
        assert_eq!(tracker.get_level(&signal.id), Some(TaintLevel::External));
    }
}
