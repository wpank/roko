//! Learning subsystem configuration.

use serde::{Deserialize, Serialize};

use super::agent::default_true;

// ---- [learning] ----------------------------------------------------------

/// Default number of gate observations between incremental threshold flushes.
pub const DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL: u64 = 10;

// ---- [learning.dreams] ---------------------------------------------------

/// Configuration for the dreams consolidation subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamsConfig {
    /// Automatically trigger dream consolidation after a plan completes.
    ///
    /// When `true` (the default) the runner spawns a non-blocking dream
    /// consolidation task after each plan that dispatched at least one agent
    /// turn. Setting this to `false` disables the automatic trigger; dreams
    /// can still be run manually via `roko knowledge dream run`.
    #[serde(default = "default_true")]
    pub trigger_on_plan_complete: bool,
    /// Maximum number of concurrent dream consolidation runs.
    ///
    /// Additional triggers are silently dropped while a run is in progress.
    /// Defaults to `1`.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_max_concurrent() -> usize {
    1
}

impl Default for DreamsConfig {
    fn default() -> Self {
        Self {
            trigger_on_plan_complete: true,
            max_concurrent: default_max_concurrent(),
        }
    }
}

/// Learning subsystem configuration.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningConfig {
    /// Auto-refresh playbook rules after successful tasks.
    #[serde(default = "default_true")]
    pub auto_playbook_refresh: bool,
    /// Inject file difficulty profiles into agent context.
    #[serde(default = "default_true")]
    pub knowledge_file_intel: bool,
    /// Inject knowledge store warnings into agent context.
    #[serde(default = "default_true")]
    pub knowledge_warnings: bool,
    /// Enable cross-task wave context propagation.
    #[serde(default = "default_true")]
    pub knowledge_wave_context: bool,
    /// Enable error signature pattern matching.
    #[serde(default = "default_true")]
    pub knowledge_error_patterns: bool,
    /// Min occurrences before promoting learned rules.
    #[serde(default = "default_learning_min_occ")]
    pub learning_min_occurrences: usize,
    /// Max file-intel entries to inject per task.
    #[serde(default = "default_file_intel_max")]
    pub file_intel_max_entries: usize,
    /// Max warning entries to inject per task.
    #[serde(default = "default_warning_max")]
    pub warning_max_entries: usize,
    /// Whether repeated gate failures should trigger a plan revision.
    #[serde(default = "default_true")]
    pub replan_on_gate_failure: bool,
    /// Maximum number of gate-failure-triggered plan revisions per plan.
    #[serde(default = "default_replan_max_per_plan")]
    pub replan_max_per_plan: u32,
    /// Consecutive gate failures required before emitting a plan revision.
    #[serde(default = "default_replan_gate_attempts")]
    pub replan_gate_attempts: u32,
    /// Run dream consolidation after a plan completes.
    ///
    /// Superseded by [`DreamsConfig::trigger_on_plan_complete`]; both must be
    /// `true` for the trigger to fire. Kept for backward compatibility with
    /// existing `roko.toml` files.
    #[serde(default = "default_true")]
    pub dream_on_completion: bool,
    /// Dreams consolidation subsystem configuration.
    #[serde(default)]
    pub dreams: DreamsConfig,
    /// Enable the lookahead router for cost-saving tier downgrades.
    /// When true, the cascade router selection is post-filtered through
    /// `LookaheadRouter::route_with_lookahead()` which may downgrade to a
    /// cheaper model when calibration data indicates sufficient success
    /// probability.
    #[serde(default)]
    pub use_lookahead_router: bool,
    /// Success probability threshold for the lookahead router to accept a
    /// cheaper tier (0.0--1.0). Only used when `use_lookahead_router` is true.
    /// Defaults to 0.7.
    #[serde(default = "default_lookahead_threshold")]
    pub lookahead_threshold: f64,
    /// Dampening factor for manual model override learning (UX34).
    ///
    /// When a user manually overrides the model via `--model` /
    /// `--force-model` / `--force-backend`, the outcome reward is
    /// multiplied by this factor before being fed into the cascade
    /// router's multi-objective bandit. This prevents a single override
    /// from dominating the learned policy. Range: 0.0--1.0.
    /// Defaults to 0.5 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_learning_dampening: Option<f64>,
    /// Number of gate observations between incremental writes of adaptive
    /// thresholds to `.roko/learn/gate-thresholds.json`.
    ///
    /// Values below one are normalized to one at the runtime boundary so an
    /// invalid zero never disables durability or creates an always-due
    /// comparison. Defaults to 10 for compatibility with the original
    /// hardcoded cadence.
    #[serde(default = "default_gate_threshold_flush_interval")]
    pub gate_threshold_flush_interval: u64,
}

fn default_lookahead_threshold() -> f64 {
    0.7
}

const fn default_gate_threshold_flush_interval() -> u64 {
    DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
}

const fn default_learning_min_occ() -> usize {
    2
}

const fn default_file_intel_max() -> usize {
    15
}

const fn default_warning_max() -> usize {
    5
}

const fn default_replan_max_per_plan() -> u32 {
    2
}

const fn default_replan_gate_attempts() -> u32 {
    3
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            auto_playbook_refresh: true,
            knowledge_file_intel: true,
            knowledge_warnings: true,
            knowledge_wave_context: true,
            knowledge_error_patterns: true,
            learning_min_occurrences: default_learning_min_occ(),
            file_intel_max_entries: default_file_intel_max(),
            warning_max_entries: default_warning_max(),
            replan_on_gate_failure: true,
            replan_max_per_plan: default_replan_max_per_plan(),
            replan_gate_attempts: default_replan_gate_attempts(),
            dream_on_completion: default_true(),
            dreams: DreamsConfig::default(),
            use_lookahead_router: false,
            lookahead_threshold: default_lookahead_threshold(),
            override_learning_dampening: None,
            gate_threshold_flush_interval: default_gate_threshold_flush_interval(),
        }
    }
}

impl LearningConfig {
    /// Return the runtime-safe gate-threshold flush cadence.
    #[must_use]
    pub const fn effective_gate_threshold_flush_interval(&self) -> u64 {
        if self.gate_threshold_flush_interval == 0 {
            1
        } else {
            self.gate_threshold_flush_interval
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_threshold_flush_interval_preserves_legacy_default() {
        let config: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert_eq!(
            config.gate_threshold_flush_interval,
            DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
        );
        assert_eq!(
            config.effective_gate_threshold_flush_interval(),
            DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL
        );
    }

    #[test]
    fn gate_threshold_flush_interval_accepts_custom_value_and_normalizes_zero() {
        let custom: LearningConfig =
            toml::from_str("gate_threshold_flush_interval = 3").expect("parse custom interval");
        assert_eq!(custom.effective_gate_threshold_flush_interval(), 3);

        let zero: LearningConfig =
            toml::from_str("gate_threshold_flush_interval = 0").expect("parse zero interval");
        assert_eq!(zero.effective_gate_threshold_flush_interval(), 1);
    }
}
