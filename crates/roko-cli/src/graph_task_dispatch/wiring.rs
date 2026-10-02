//! The learning wiring of a Graph task dispatcher: the census of S01 §4.8
//! and §5.8 (P0-11).
//!
//! [`GraphTaskDispatcher::wiring_report`] names every learning component
//! S01 §5.8 lists and says whether this dispatcher has it. The wiring census
//! (`tests/learning_wiring_census.rs`) builds the production feedback wiring,
//! and fails when a component goes missing or when one it expects to be
//! missing turns up, so the list can only shrink honestly.
//!
//! A component is wired when the production object graph holds it: a sink on
//! the feedback facade, or a store path or handle on the feedback context.
//! Components with no attachment point on the Graph path are reported
//! missing here; whoever wires one reports it.

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

/// The census of one dispatcher's learning wiring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WiringReport {
    /// Every S01 §5.8 component, wired or not.
    pub components: Vec<WiringComponent>,
    /// The sinks on the feedback facade, in fan-out order.
    pub facade_sinks: Vec<&'static str>,
}

impl WiringReport {
    /// The component `id`, if the census names it.
    #[must_use]
    pub fn component(&self, id: &str) -> Option<&WiringComponent> {
        self.components.iter().find(|component| component.id == id)
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

impl GraphTaskDispatcher {
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
                false,
                "no writer of per-section prompt effects",
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
                "store.holdout",
                WiringKind::Store,
                feedback.holdout_experiment.is_some(),
                "holdout assignment gating learning updates",
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
                false,
                "`KnowledgeStore::record_access` has no production caller (S01 P0-9)",
            ),
            component(
                "reader.gate_thresholds",
                WiringKind::Reader,
                feedback.gate_thresholds_path.is_some(),
                "adaptive gate thresholds: updated after each verify run, and they set the \
                 retry budget of tasks that author none",
            ),
        ];
        WiringReport {
            components,
            facade_sinks,
        }
    }
}
