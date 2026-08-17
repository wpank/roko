//! Structure-preserving maps between Memory, Daimon, and Dreams.

use std::collections::BTreeMap;

use roko_core::{BehavioralState, ContentHash, PadVector};
use roko_daimon::{AffectEngine, AffectEvent, DaimonState};
use roko_dreams::DreamCycleReport;
use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeStore, KnowledgeTier};

use crate::{ComposeError, CrossCutResult};

/// Memory feedback supplied to the Memory -> Daimon transformation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryOutcome {
    /// Plan receiving the gate result.
    pub plan_id: String,
    /// Task receiving the gate result.
    pub task_id: String,
    /// Whether verification passed.
    pub gate_passed: bool,
    /// Verification rung.
    pub rung: u32,
    /// Knowledge entries that influenced the decision.
    pub affected_entry_ids: Vec<String>,
    /// Episode to offer to Dreams.
    pub episode_id: String,
}

/// Stable Daimon snapshot used as a natural-transformation source.
#[derive(Clone, Debug, PartialEq)]
pub struct DaimonAssessment {
    /// Current PAD vector.
    pub pad: PadVector,
    /// Current stable behavioral state.
    pub behavioral_state: BehavioralState,
    /// Affect confidence.
    pub confidence: f64,
    /// Episode represented by this assessment.
    pub episode_id: String,
}

impl DaimonAssessment {
    /// Snapshot the real Daimon engine.
    #[must_use]
    pub fn from_state(state: &DaimonState, episode_id: impl Into<String>) -> Self {
        let affect = state.query();
        Self {
            pad: affect.pad,
            behavioral_state: affect.behavioral_state,
            confidence: affect.confidence,
            episode_id: episode_id.into(),
        }
    }
}

/// Input offered to the delta-speed dream runner.
#[derive(Clone, Debug, PartialEq)]
pub struct DreamConsolidationInput {
    /// Episodes eligible for NREM replay.
    pub episode_ids: Vec<String>,
    /// Replay priority in `[0, 1]`.
    pub priority: f64,
    /// Whether the current affect requires an immediate delta dream.
    pub trigger_delta: bool,
    /// Auditable reason for the priority.
    pub reason: String,
}

/// Dream-derived input to the live Daimon engine.
#[derive(Clone, Debug, PartialEq)]
pub struct DreamAffectInput {
    /// Existing Daimon event representing the dream outcome.
    pub event: AffectEvent,
    /// Whether emotional depotentiation should run after appraisal.
    pub depotentiate: bool,
}

/// Evidence returned by the synchronous half of a gate-failure cascade.
#[derive(Clone, Debug, PartialEq)]
pub struct GateFailureCascade {
    /// Memory outcome that initiated the cascade.
    pub memory_outcome: MemoryOutcome,
    /// PAD after applying eta_MN.
    pub updated_pad: PadVector,
    /// Daimon snapshot offered to Memory and Dreams.
    pub daimon_assessment: DaimonAssessment,
    /// NREM input produced via Daimon -> Memory -> Dreams.
    pub memory_path: DreamConsolidationInput,
    /// NREM input produced directly via Daimon -> Dreams.
    pub direct_path: DreamConsolidationInput,
}

/// Memory -> Daimon: gate outcomes become affect appraisal events.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_MN(outcome: &MemoryOutcome) -> AffectEvent {
    AffectEvent::GateResult {
        plan_id: outcome.plan_id.clone(),
        task_id: outcome.task_id.clone(),
        passed: outcome.gate_passed,
        rung: outcome.rung,
    }
}

/// Daimon -> Memory: persist an affect assessment as a knowledge entry.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_NM(assessment: &DaimonAssessment) -> KnowledgeEntry {
    let priority = consolidation_priority(assessment);
    let trigger_delta = assessment.behavioral_state == BehavioralState::Struggling;
    let content = format!(
        "PAD assessment p={:.3} a={:.3} d={:.3} state={:?}",
        assessment.pad.pleasure,
        assessment.pad.arousal,
        assessment.pad.dominance,
        assessment.behavioral_state
    );
    KnowledgeEntry {
        id: format!("daimon:{}", ContentHash::of(content.as_bytes()).to_hex()),
        kind: KnowledgeKind::Insight,
        source: Some("natural_transform:eta_NM".into()),
        content,
        confidence: priority,
        source_episodes: vec![assessment.episode_id.clone()],
        tags: vec![
            "daimon_assessment".into(),
            format!("dream_trigger:{trigger_delta}"),
        ],
        tier: KnowledgeTier::Working,
        ..KnowledgeEntry::default()
    }
}

/// Memory -> Dreams: knowledge provenance becomes a prioritized replay input.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_MD(entry: &KnowledgeEntry) -> DreamConsolidationInput {
    let trigger_delta = entry.tags.iter().any(|tag| tag == "dream_trigger:true");
    DreamConsolidationInput {
        episode_ids: if entry.source_episodes.is_empty() {
            vec![entry.id.clone()]
        } else {
            entry.source_episodes.clone()
        },
        priority: entry.confidence.clamp(0.0, 1.0),
        trigger_delta,
        reason: "affect-backed memory replay".into(),
    }
}

/// Dreams -> Memory: publish consolidated entries from a completed real cycle.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_DM(report: &DreamCycleReport) -> Vec<KnowledgeEntry> {
    let mut entries = BTreeMap::<String, KnowledgeEntry>::new();
    let candidates = report
        .clusters
        .iter()
        .flat_map(|cluster| {
            cluster
                .knowledge_entries
                .iter()
                .chain(cluster.regression_entries.iter())
        })
        .chain(report.regressions_detected.iter())
        .chain(report.strategy_hypotheses.iter());
    for entry in candidates {
        let mut entry = entry.clone();
        entry
            .source
            .get_or_insert_with(|| "natural_transform:eta_DM".into());
        entries.insert(entry.id.clone(), entry);
    }
    entries.into_values().collect()
}

/// Daimon -> Dreams: trigger consolidation from the current affect snapshot.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_ND(assessment: &DaimonAssessment) -> DreamConsolidationInput {
    DreamConsolidationInput {
        episode_ids: vec![assessment.episode_id.clone()],
        priority: consolidation_priority(assessment),
        trigger_delta: assessment.behavioral_state == BehavioralState::Struggling,
        reason: "direct affect-triggered replay".into(),
    }
}

/// Dreams -> Daimon: convert consolidation output into appraisal + cooling.
#[allow(non_snake_case)]
#[must_use]
pub fn eta_DN(report: &DreamCycleReport) -> DreamAffectInput {
    DreamAffectInput {
        event: AffectEvent::DreamOutcome {
            knowledge_entries: report.knowledge_entries_written,
            playbooks_created: report.playbooks_created,
            regressions_detected: report.regressions_detected.len(),
            strategy_hypotheses: report.strategy_hypotheses.len(),
            episodes_processed: report.processed_episodes,
        },
        depotentiate: report.processed_episodes > 0,
    }
}

/// Fire the synchronous Memory -> Daimon -> Dreams portion of a gate failure.
pub fn run_gate_failure_cascade(
    store: &KnowledgeStore,
    daimon: &mut DaimonState,
    memory_outcome: MemoryOutcome,
) -> CrossCutResult<GateFailureCascade> {
    let refs = memory_outcome
        .affected_entry_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    store
        .score_prediction_utility(&memory_outcome.affected_entry_ids, false, 0.0)
        .map_err(compose_error)?;
    let mut mutable_store = store.clone();
    mutable_store
        .batch_record_usage(
            &refs
                .into_iter()
                .map(|id| (id.to_string(), false))
                .collect::<Vec<_>>(),
        )
        .map_err(compose_error)?;

    let updated_pad = daimon.appraise(eta_MN(&memory_outcome));
    let assessment = DaimonAssessment::from_state(daimon, memory_outcome.episode_id.clone());
    let memory_entry = eta_NM(&assessment);
    let memory_path = eta_MD(&memory_entry);
    let direct_path = eta_ND(&assessment);
    store.ingest(vec![memory_entry]).map_err(compose_error)?;

    debug_assert_eq!(memory_path.episode_ids, direct_path.episode_ids);
    debug_assert_eq!(memory_path.priority, direct_path.priority);
    debug_assert_eq!(memory_path.trigger_delta, direct_path.trigger_delta);
    Ok(GateFailureCascade {
        memory_outcome,
        updated_pad,
        daimon_assessment: assessment,
        memory_path,
        direct_path,
    })
}

fn consolidation_priority(assessment: &DaimonAssessment) -> f64 {
    (0.35
        + assessment.pad.arousal.max(0.0) * 0.25
        + (-assessment.pad.pleasure).max(0.0) * 0.20
        + (1.0 - assessment.confidence.clamp(0.0, 1.0)) * 0.20)
        .clamp(0.0, 1.0)
}

fn compose_error(error: impl std::fmt::Display) -> ComposeError {
    ComposeError::Other(format!("natural transformation: {error}"))
}

/// Marker trait for direction-specific natural transformation adapters.
pub trait NaturalTransformation<Source, Target> {
    /// Apply the structure-preserving map.
    fn transform(source: &Source) -> Target;
}

#[cfg(test)]
mod tests {
    use roko_neuro::KnowledgeKind;

    use super::*;

    #[test]
    fn daimon_memory_dreams_triangle_commutes() {
        let assessment = DaimonAssessment {
            pad: PadVector::new(-0.6, 0.8, -0.5),
            behavioral_state: BehavioralState::Struggling,
            confidence: 0.2,
            episode_id: "failed-episode".into(),
        };

        let through_memory = eta_MD(&eta_NM(&assessment));
        let direct = eta_ND(&assessment);

        assert_eq!(through_memory.episode_ids, direct.episode_ids);
        assert_eq!(through_memory.priority, direct.priority);
        assert_eq!(through_memory.trigger_delta, direct.trigger_delta);
    }

    #[test]
    fn eta_mn_preserves_gate_identity_and_outcome() {
        let outcome = MemoryOutcome {
            plan_id: "p".into(),
            task_id: "t".into(),
            gate_passed: false,
            rung: 2,
            affected_entry_ids: Vec::new(),
            episode_id: "e".into(),
        };
        assert_eq!(
            eta_MN(&outcome),
            AffectEvent::GateResult {
                plan_id: "p".into(),
                task_id: "t".into(),
                passed: false,
                rung: 2,
            }
        );
    }

    #[test]
    fn gate_failure_cascade_weakens_memory_and_produces_commuting_replay_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp.path().join("knowledge.jsonl"));
        store
            .add(KnowledgeEntry {
                id: "context-entry".into(),
                kind: KnowledgeKind::AntiKnowledge,
                content: "failed context".into(),
                confidence: 0.8,
                balance: 1.0,
                ..KnowledgeEntry::default()
            })
            .unwrap();
        let before = store.read_all().unwrap()[0].clone();
        let mut daimon = DaimonState::new();
        let outcome = MemoryOutcome {
            plan_id: "plan".into(),
            task_id: "task".into(),
            gate_passed: false,
            rung: 2,
            affected_entry_ids: vec!["context-entry".into()],
            episode_id: "failed-episode".into(),
        };

        let cascade = run_gate_failure_cascade(&store, &mut daimon, outcome).unwrap();
        let after = store
            .read_all()
            .unwrap()
            .into_iter()
            .find(|entry| entry.id == "context-entry")
            .unwrap();

        assert!(after.balance < before.balance);
        assert!(after.confidence < before.confidence);
        assert_eq!(
            cascade.memory_path.episode_ids,
            cascade.direct_path.episode_ids
        );
        assert_eq!(cascade.memory_path.priority, cascade.direct_path.priority);
        assert_eq!(
            cascade.memory_path.trigger_delta,
            cascade.direct_path.trigger_delta
        );
    }
}
