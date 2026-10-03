//! The learning wiring of a Graph task dispatcher: the census of S01 §4.8
//! and §5.8 (P0-11).
//!
//! [`GraphTaskDispatcher::wiring_report`] names every learning component
//! S01 §5.8 lists, and those the S02 loops add (the arm set, the withhold
//! arms, the placebo and the router's source-aware credit), and says whether
//! this dispatcher has it. It also says where each loop of the loop registry
//! stands: live, observe-only or retired (S02 SC1). The wiring census
//! (`tests/learning_wiring_census.rs`) builds the production feedback wiring,
//! and fails when a component goes missing or when one it expects to be
//! missing turns up, so the list can only shrink honestly.
//!
//! A component is wired when the production object graph holds it: a sink on
//! the feedback facade, a store path or handle on the feedback context, or,
//! for the S02 components, the loop registry the arm sets are drawn over.
//! Components with no attachment point on the Graph path are reported
//! missing here; whoever wires one reports it.

use roko_learn::loop_audit::{Lifecycle, LoopSpec};
use serde::Serialize;

use super::*;

/// What a census component does with a settled attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WiringKind {
    /// Learns from settled attempts.
    Sink,
    /// Keeps attempt-scoped state or records.
    Store,
    /// Feeds learned state back into dispatch.
    Reader,
}

impl WiringKind {
    /// Wire value, e.g. `sink`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sink => "sink",
            Self::Store => "store",
            Self::Reader => "reader",
        }
    }
}

/// One learning component of the census (`roko.census/1`, S01 §5.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WiringComponent {
    /// Stable id, e.g. `sink.routing`.
    pub id: &'static str,
    /// What the component does.
    pub kind: WiringKind,
    /// Whether the dispatcher has it.
    pub wired: bool,
    /// What provides it, or why it is missing.
    pub detail: &'static str,
}

/// One learning loop of the loop registry, and where it stands (S02 SC1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoopWiring {
    /// The loop's registry id, e.g. `L-know`.
    pub id: String,
    /// `live` (randomized on its layer and audited), `observe_only`
    /// (measured, never randomized) or `retired` (its code is deleted).
    pub state: &'static str,
    /// The task that deleted a retired loop's code.
    pub retired_by: Option<String>,
}

impl LoopWiring {
    /// Where `spec` stands.
    fn of(spec: &LoopSpec) -> Self {
        let (state, retired_by) = match &spec.lifecycle {
            Lifecycle::Active => ("live", None),
            Lifecycle::ObserveOnly => ("observe_only", None),
            Lifecycle::Retired { by } => ("retired", Some(by.clone())),
        };
        Self {
            id: spec.id.as_str().to_string(),
            state,
            retired_by,
        }
    }
}

/// The census of one dispatcher's learning wiring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WiringReport {
    /// Every S01 §5.8 component and the S02 loops' own, wired or not.
    pub components: Vec<WiringComponent>,
    /// The sinks on the feedback facade, in fan-out order.
    pub facade_sinks: Vec<&'static str>,
    /// Every loop of the loop registry the arm sets draw over, in registry
    /// order, and where it stands; empty when no registry loads.
    pub loops: Vec<LoopWiring>,
}

impl WiringReport {
    /// The component `id`, if the census names it.
    #[must_use]
    pub fn component(&self, id: &str) -> Option<&WiringComponent> {
        self.components.iter().find(|component| component.id == id)
    }

    /// Where the loop `id` stands, if the registry names it.
    #[must_use]
    pub fn loop_state(&self, id: &str) -> Option<&'static str> {
        self.loops
            .iter()
            .find(|loop_wiring| loop_wiring.id == id)
            .map(|loop_wiring| loop_wiring.state)
    }

    /// Ids of the components the dispatcher lacks.
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        self.components
            .iter()
            .filter(|component| !component.wired)
            .map(|component| component.id)
            .collect()
    }
}

/// Logged once: a frozen run's attempts skip their learned-state writes.
static FROZEN_WRITES_SKIPPED: std::sync::Once = std::sync::Once::new();

impl GraphTaskDispatcher {
    /// Whether learning is frozen for this run (`[learning] frozen`,
    /// decision 2218): its attempts read learned state and write none, so
    /// they skip the affect appraisal, the T0 reflex hit and credit, the
    /// knowledge-access count, the gate-threshold update and the post-gate
    /// reflection. The first skip says so in the log.
    pub(super) fn learning_frozen(&self) -> bool {
        let frozen = self.config.learning.frozen;
        if frozen {
            FROZEN_WRITES_SKIPPED.call_once(|| {
                tracing::info!(
                    "learning is frozen: attempts read learned state and write none (no affect \
                     appraisal, reflex hit or credit, knowledge access, gate thresholds or \
                     reflection)"
                );
            });
        }
        frozen
    }

    /// The census of this dispatcher's learning wiring (S01 §5.8): every
    /// learning component, and whether the dispatcher has it.
    #[must_use]
    pub fn wiring_report(&self) -> WiringReport {
        let feedback = &self.feedback;
        let facade_sinks: Vec<&'static str> = feedback
            .feedback_facade
            .as_ref()
            .map(|facade| {
                facade
                    .stats()
                    .per_sink
                    .into_iter()
                    .map(|sink| sink.name)
                    .collect()
            })
            .unwrap_or_default();
        let has_sink = |name: &str| facade_sinks.iter().any(|sink| *sink == name);
        // The registry the chains' arm sets are drawn over (S02.P1-14).
        let registry = super::attempt::load_loop_registry(&self.workdir);
        let has_loop = |id: &str| {
            registry
                .as_ref()
                .is_some_and(|loops| loops.get(id).is_some())
        };
        let component = |id, kind, wired, detail| WiringComponent {
            id,
            kind,
            wired,
            detail,
        };
        let components = vec![
            component(
                "sink.episode",
                WiringKind::Sink,
                has_sink("episodes"),
                "`episodes` sink on the feedback facade: one episode per settled attempt",
            ),
            component(
                "sink.routing",
                WiringKind::Sink,
                has_sink("routing"),
                "`routing` sink on the feedback facade: cascade router observations",
            ),
            component(
                "sink.knowledge_ingestion",
                WiringKind::Sink,
                has_sink("verified_knowledge") || has_sink("knowledge"),
                "`verified_knowledge` sink: gate-verified attempts grow durable knowledge",
            ),
            component(
                "sink.playbook_outcome",
                WiringKind::Sink,
                feedback.playbook_dir.is_some(),
                "playbook outcomes recorded after each attempt (`learn/playbooks/`)",
            ),
            component(
                "sink.error_pattern",
                WiringKind::Sink,
                has_sink("error_patterns"),
                "`error_patterns` sink on the feedback facade: failures of the agent's work \
                 feed the error-pattern store prompts read (`learn/error-patterns.json`)",
            ),
            component(
                "sink.section_effect",
                WiringKind::Sink,
                feedback.section_outcomes.is_some(),
                "section bandit outcomes, folded into `learn/section-bandit.json` when the run \
                 ends",
            ),
            component(
                "store.attempt_log",
                WiringKind::Store,
                feedback.runs_dir.is_some(),
                "each run's `attempts.jsonl`: one open line and one verdict per attempt",
            ),
            component(
                "store.prompt_experiment",
                WiringKind::Store,
                feedback.experiment_store_path.is_some(),
                "attempt-scoped prompt treatments from `learn/experiments.json`",
            ),
            component(
                "store.decision_writer",
                WiringKind::Store,
                feedback.runs_dir.is_some(),
                "each run's `decisions.jsonl`: one route decision per attempt that routes",
            ),
            component(
                "store.exposure_writer",
                WiringKind::Store,
                feedback.runs_dir.is_some(),
                "each run's `exposures.jsonl`: one row per item an attempt's prompt retrieved, \
                 and whether it was included",
            ),
            component(
                "store.record_access",
                WiringKind::Store,
                !self.config.learning.frozen,
                "`KnowledgeStore::count_access`: each knowledge entry a planned prompt \
                 included counts an access, unless learning is frozen",
            ),
            component(
                "reader.gate_thresholds",
                WiringKind::Reader,
                feedback.gate_thresholds_path.is_some(),
                "adaptive gate thresholds: updated after each verify run, and they set the \
                 retry budget of tasks that author none",
            ),
            component(
                "store.arm_set",
                WiringKind::Store,
                registry.is_some(),
                "each chain's arm set, drawn at attempt open over the loop registry and \
                 carried by every decision row of its attempts (S02.P1-14)",
            ),
            component(
                "reader.withhold_arms",
                WiringKind::Reader,
                has_loop("L-know") && has_loop("L-play") && has_loop("L-sec"),
                "prompt assembly reads the chain's arms: L-know and L-play withhold their \
                 sections on the default arm, and the section bandit leaves droppable \
                 sections out on L-sec's learned arm",
            ),
            component(
                "store.placebo",
                WiringKind::Store,
                has_loop("L-placebo") && feedback.runs_dir.is_some(),
                "the placebo's decision row for each attempt, in its run's `decisions.jsonl` \
                 (S03 §4.3)",
            ),
            component(
                "sink.router_source_credit",
                WiringKind::Sink,
                has_sink("routing"),
                "the `routing` sink credits the cascade router for its own picks alone \
                 (decision 4111)",
            ),
        ];
        let loops = registry
            .as_ref()
            .map(|registry| registry.loops().iter().map(LoopWiring::of).collect())
            .unwrap_or_default();
        WiringReport {
            components,
            facade_sinks,
            loops,
        }
    }
}
