//! Cross-cut recommendation arbitration: which cross-cut's recommendation
//! wins when two conflict, by priority layer or a second-price tiebreak.
//!
//! The prompt-budget VCG auction, its learning bidders and their
//! diagnostics lived here until 4218 retired them: no run ever registered a
//! bidder, so prompt composition always ran density-greedy.

#[cfg(feature = "cross-cut-functors")]
use std::sync::Arc;

use roko_core::Signal;
use roko_neuro::KnowledgeTier;
use serde::{Deserialize, Serialize};

#[cfg(feature = "cross-cut-functors")]
use crate::cross_cut::{CrossCutContext, CrossCutFunctor, CrossCutResult};
#[cfg(feature = "cross-cut-functors")]
use crate::daimon_functor::DaimonFunctor;
#[cfg(feature = "cross-cut-functors")]
use crate::dreams_functor::DreamsFunctor;
#[cfg(feature = "cross-cut-functors")]
use crate::memory_functor::MemoryFunctor;

// ─── Cross-cut recommendation arbitration ────────────────────────────────

/// Cross-cuts eligible to bid. Safety is intentionally absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossCutId {
    /// Durable memory.
    Memory,
    /// Affect and behavioral gating.
    Daimon,
    /// Speculative dream output.
    Dreams,
}

/// Decision kinds eligible for conflict resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossCutDecisionKind {
    /// Model or execution routing.
    Route,
    /// Prompt/context composition.
    Compose,
    /// Action gating; never eligible for VCG.
    Act,
    /// Any non-auction decision.
    Other,
}

/// One auditable recommendation emitted by a cross-cut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossCutRecommendation {
    /// Recommending subsystem.
    pub source: CrossCutId,
    /// Stable conflict-group key.
    pub decision_key: String,
    /// Kind of decision.
    pub decision_kind: CrossCutDecisionKind,
    /// Recommended value.
    pub value: String,
    /// Truthful confidence bid in `[0, 1]`.
    pub confidence: f64,
    /// Arbitration level for same-level VCG eligibility.
    pub priority_level: u8,
    /// Daimon safety override marker.
    pub safety_critical: bool,
    /// Memory validation tier, when applicable.
    pub knowledge_tier: Option<KnowledgeTier>,
}

impl CrossCutRecommendation {
    /// Parse the shared recommendation-tag contract from a signal.
    #[must_use]
    pub fn from_signal(signal: &Signal) -> Option<Self> {
        let source = match signal.tag("recommendation_source")? {
            "memory" => CrossCutId::Memory,
            "daimon" => CrossCutId::Daimon,
            "dreams" => CrossCutId::Dreams,
            _ => return None,
        };
        let decision_kind = match signal.tag("decision_kind")? {
            "route" => CrossCutDecisionKind::Route,
            "compose" => CrossCutDecisionKind::Compose,
            "act" => CrossCutDecisionKind::Act,
            _ => CrossCutDecisionKind::Other,
        };
        let confidence = signal
            .tag("recommendation_confidence")?
            .parse::<f64>()
            .ok()?
            .clamp(0.0, 1.0);
        Some(Self {
            source,
            decision_key: signal.tag("decision_key")?.to_string(),
            decision_kind,
            value: signal.tag("recommendation_value")?.to_string(),
            confidence,
            priority_level: signal
                .tag("priority_level")
                .and_then(|value| value.parse().ok())
                .unwrap_or(2),
            safety_critical: signal.tag("safety_critical") == Some("true"),
            knowledge_tier: signal.tag("knowledge_tier").and_then(parse_knowledge_tier),
        })
    }

    /// Whether two recommendations disagree about the same decision.
    #[must_use]
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.decision_key == other.decision_key && self.value != other.value
    }
}

/// How an arbitration completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArbitrationMechanism {
    /// Fixed Daimon/Memory hierarchy.
    Priority,
    /// Second-price VCG tiebreaker.
    Vcg,
}

/// Result of cross-cut recommendation arbitration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CrossCutArbitrationResult {
    /// No eligible conflict was present.
    NoConflict,
    /// One recommendation won.
    Resolved {
        /// Winning subsystem.
        winner: CrossCutId,
        /// Winning recommendation.
        recommendation: CrossCutRecommendation,
        /// Second-highest confidence for VCG; zero for priority resolution.
        attention_cost: f64,
        /// Runner-up in a VCG resolution.
        runner_up: Option<CrossCutId>,
        /// Resolution layer used.
        mechanism: ArbitrationMechanism,
    },
}

/// Apply fixed safety-critical Daimon and consolidated-Memory overrides.
#[must_use]
pub fn resolve_by_priority(
    recommendations: &[CrossCutRecommendation],
) -> Option<CrossCutArbitrationResult> {
    if let Some(daimon) = recommendations
        .iter()
        .find(|rec| rec.source == CrossCutId::Daimon && rec.safety_critical)
    {
        return Some(priority_result(daimon));
    }
    recommendations
        .iter()
        .filter(|rec| {
            rec.source == CrossCutId::Memory
                && matches!(
                    rec.knowledge_tier,
                    Some(KnowledgeTier::Consolidated | KnowledgeTier::Persistent)
                )
        })
        .find(|memory| {
            recommendations
                .iter()
                .any(|dream| dream.source == CrossCutId::Dreams && memory.conflicts_with(dream))
        })
        .map(priority_result)
}

/// Apply a same-level, high-confidence second-price VCG tiebreaker.
#[must_use]
pub fn resolve_by_vcg(recommendations: &[CrossCutRecommendation]) -> CrossCutArbitrationResult {
    let candidates = recommendations
        .iter()
        .filter(|rec| {
            rec.confidence > 0.5
                && matches!(
                    rec.decision_kind,
                    CrossCutDecisionKind::Route | CrossCutDecisionKind::Compose
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut eligible = candidates
        .iter()
        .filter(|candidate| {
            candidates.iter().any(|other| {
                candidate.source != other.source
                    && candidate.priority_level == other.priority_level
                    && candidate.conflicts_with(other)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        right
            .confidence
            .total_cmp(&left.confidence)
            .then_with(|| left.source.cmp(&right.source))
    });
    let Some(winner) = eligible.first().cloned() else {
        return CrossCutArbitrationResult::NoConflict;
    };
    let Some(runner_up) = eligible.iter().skip(1).find(|candidate| {
        candidate.priority_level == winner.priority_level && candidate.conflicts_with(&winner)
    }) else {
        return CrossCutArbitrationResult::NoConflict;
    };
    CrossCutArbitrationResult::Resolved {
        winner: winner.source,
        recommendation: winner,
        attention_cost: runner_up.confidence,
        runner_up: Some(runner_up.source),
        mechanism: ArbitrationMechanism::Vcg,
    }
}

/// Cross-cut pipeline output after safety pre-filtering and arbitration.
#[cfg(feature = "cross-cut-functors")]
pub struct CrossCutArbitration {
    /// Safely enriched signals used to collect bids.
    pub signals: Vec<Signal>,
    /// Conflict-resolution result.
    pub result: CrossCutArbitrationResult,
}

/// Runs the three advisory functors and resolves their conflicting bids.
/// Parked with the functors behind `cross-cut-functors` (9223).
#[cfg(feature = "cross-cut-functors")]
pub struct CrossCutArbitrator {
    /// Durable-memory functor.
    pub memory: Arc<MemoryFunctor>,
    /// Affect functor.
    pub daimon: Arc<DaimonFunctor>,
    /// Delta-speed Dreams functor.
    pub dreams: Arc<DreamsFunctor>,
    safety_filter: Arc<dyn CrossCutFunctor<CrossCutContext>>,
}

#[cfg(feature = "cross-cut-functors")]
impl CrossCutArbitrator {
    /// Safety is mandatory and deliberately not represented as a bidder.
    #[must_use]
    pub fn new(
        memory: Arc<MemoryFunctor>,
        daimon: Arc<DaimonFunctor>,
        dreams: Arc<DreamsFunctor>,
        safety_filter: Arc<dyn CrossCutFunctor<CrossCutContext>>,
    ) -> Self {
        Self {
            memory,
            daimon,
            dreams,
            safety_filter,
        }
    }

    /// Enrich, safety-filter, then collect and arbitrate recommendations.
    pub async fn arbitrate(
        &self,
        mut input: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<CrossCutArbitration> {
        input = self.memory.pre_enrich(input, ctx).await?;
        input = self.daimon.pre_enrich(input, ctx).await?;
        input = self.dreams.pre_enrich(input, ctx).await?;
        let signals = self.safety_filter.pre_enrich(input, ctx).await?;
        let recommendations = signals
            .iter()
            .filter_map(CrossCutRecommendation::from_signal)
            .collect::<Vec<_>>();
        let result = resolve_by_priority(&recommendations)
            .unwrap_or_else(|| resolve_by_vcg(&recommendations));
        Ok(CrossCutArbitration { signals, result })
    }
}

fn priority_result(recommendation: &CrossCutRecommendation) -> CrossCutArbitrationResult {
    CrossCutArbitrationResult::Resolved {
        winner: recommendation.source,
        recommendation: recommendation.clone(),
        attention_cost: 0.0,
        runner_up: None,
        mechanism: ArbitrationMechanism::Priority,
    }
}

fn parse_knowledge_tier(value: &str) -> Option<KnowledgeTier> {
    match value {
        "transient" => Some(KnowledgeTier::Transient),
        "working" => Some(KnowledgeTier::Working),
        "consolidated" => Some(KnowledgeTier::Consolidated),
        "persistent" => Some(KnowledgeTier::Persistent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "cross-cut-functors")]
    use std::sync::{Arc, RwLock};

    #[cfg(feature = "cross-cut-functors")]
    use roko_agent::safety::contract::AgentContract;
    #[cfg(feature = "cross-cut-functors")]
    use roko_core::capabilities::{Capability, CapabilitySet};
    #[cfg(feature = "cross-cut-functors")]
    use roko_core::{Kind, Signal};
    #[cfg(feature = "cross-cut-functors")]
    use roko_daimon::DaimonState;
    #[cfg(feature = "cross-cut-functors")]
    use roko_neuro::KnowledgeStore;

    #[cfg(feature = "cross-cut-functors")]
    use crate::{
        CrossCutContext, DaimonFunctor, DreamsFunctor, LoopStep, MemoryFunctor, SafetyFunctor,
    };

    use super::*;

    fn recommendation(
        source: CrossCutId,
        value: &str,
        confidence: f64,
        priority_level: u8,
    ) -> CrossCutRecommendation {
        CrossCutRecommendation {
            source,
            decision_key: "route:task".into(),
            decision_kind: CrossCutDecisionKind::Route,
            value: value.into(),
            confidence,
            priority_level,
            safety_critical: false,
            knowledge_tier: None,
        }
    }

    #[test]
    fn priority_layer_protects_safety_and_consolidated_memory() {
        let mut daimon = recommendation(CrossCutId::Daimon, "defer", 0.1, 1);
        daimon.decision_kind = CrossCutDecisionKind::Act;
        daimon.safety_critical = true;
        let dreams = recommendation(CrossCutId::Dreams, "execute", 0.99, 1);
        let resolved = resolve_by_priority(&[dreams, daimon]).unwrap();
        assert!(matches!(
            resolved,
            CrossCutArbitrationResult::Resolved {
                winner: CrossCutId::Daimon,
                mechanism: ArbitrationMechanism::Priority,
                ..
            }
        ));

        let mut memory = recommendation(CrossCutId::Memory, "known-model", 0.6, 2);
        memory.knowledge_tier = Some(KnowledgeTier::Consolidated);
        let dreams = recommendation(CrossCutId::Dreams, "speculative-model", 0.9, 2);
        assert!(matches!(
            resolve_by_priority(&[dreams, memory]),
            Some(CrossCutArbitrationResult::Resolved {
                winner: CrossCutId::Memory,
                ..
            })
        ));
    }

    #[test]
    fn cross_cut_vcg_is_same_level_high_confidence_and_second_price() {
        let memory = recommendation(CrossCutId::Memory, "model-a", 0.8, 2);
        let dreams = recommendation(CrossCutId::Dreams, "model-b", 0.7, 2);
        assert!(matches!(
            resolve_by_vcg(&[dreams, memory]),
            CrossCutArbitrationResult::Resolved {
                winner: CrossCutId::Memory,
                attention_cost,
                runner_up: Some(CrossCutId::Dreams),
                mechanism: ArbitrationMechanism::Vcg,
                ..
            } if (attention_cost - 0.7).abs() < f64::EPSILON
        ));

        let memory = recommendation(CrossCutId::Memory, "model-a", 0.8, 1);
        let dreams = recommendation(CrossCutId::Dreams, "model-b", 0.7, 2);
        assert_eq!(
            resolve_by_vcg(&[memory, dreams]),
            CrossCutArbitrationResult::NoConflict
        );
    }

    #[cfg(feature = "cross-cut-functors")]
    #[tokio::test]
    async fn safety_prefilter_removes_forbidden_bid_before_collection() {
        let temp = tempfile::tempdir().unwrap();
        let memory = Arc::new(MemoryFunctor::new(Arc::new(KnowledgeStore::new(
            temp.path().join("knowledge.jsonl"),
        ))));
        let daimon = Arc::new(DaimonFunctor::new(Arc::new(
            RwLock::new(DaimonState::new()),
        )));
        let dreams = Arc::new(DreamsFunctor);
        let safety = Arc::new(SafetyFunctor::new(
            AgentContract {
                role: "test".into(),
                ..AgentContract::default()
            },
            CapabilitySet::from([Capability::ReadFs]),
        ));
        let arbitrator = CrossCutArbitrator::new(memory, daimon, dreams, safety);
        let allowed = Signal::builder(Kind::RouterChoice)
            .tag("recommendation_source", "memory")
            .tag("decision_kind", "route")
            .tag("decision_key", "route:task")
            .tag("recommendation_value", "safe")
            .tag("recommendation_confidence", "0.7")
            .tag("priority_level", "2")
            .tag("requires_capability", "read_fs")
            .build();
        let forbidden = Signal::builder(Kind::RouterChoice)
            .tag("recommendation_source", "dreams")
            .tag("decision_kind", "route")
            .tag("decision_key", "route:task")
            .tag("recommendation_value", "unsafe")
            .tag("recommendation_confidence", "0.99")
            .tag("priority_level", "2")
            .tag("requires_capability", "shell")
            .build();
        let ctx = CrossCutContext {
            step: LoopStep::Verify,
            ..CrossCutContext::default()
        };

        let result = arbitrator
            .arbitrate(vec![allowed, forbidden], &ctx)
            .await
            .unwrap();

        assert_eq!(result.signals.len(), 1);
        assert_eq!(result.result, CrossCutArbitrationResult::NoConflict);
    }
}
