//! Learning subsystem configuration.

use serde::{Deserialize, Serialize};

use super::agent::default_true;

// ---- [learning] ----------------------------------------------------------

/// Default number of gate observations between incremental threshold flushes.
pub const DEFAULT_GATE_THRESHOLD_FLUSH_INTERVAL: u64 = 10;

// ---- [learning.dreams] ---------------------------------------------------

/// Configuration for the dreams consolidation subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DreamsConfig {
    /// Automatically trigger dream consolidation after a plan completes.
    ///
    /// The trigger fires only when this (default `true`) and
    /// [`LearningConfig::dream_on_completion`] (default `false`) are both
    /// `true`, so by default no dream runs. Setting this to `false` disables
    /// the automatic trigger; dreams can still be run manually via
    /// `roko knowledge dream run`.
    #[serde(default = "default_true")]
    pub trigger_on_plan_complete: bool,
    /// Maximum number of dream consolidations that ACP sessions run at once
    /// in one process. An ACP turn that finds this many running starts none.
    /// The plan-completion trigger keeps its own limit of one.
    ///
    /// Defaults to `1`. Zero is normalized to one at the runtime boundary
    /// (see [`Self::effective_max_concurrent`]).
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// Let ACP sessions trigger dream consolidation once
    /// [`Self::acp_episode_threshold`] episodes accumulate since the last
    /// dream report.
    ///
    /// Defaults to `false`: each automatic dream costs a model call, so ACP
    /// sessions start none unless this is `true`. It is independent of
    /// [`Self::trigger_on_plan_complete`] and
    /// [`LearningConfig::dream_on_completion`].
    #[serde(default)]
    pub trigger_on_acp_episodes: bool,
    /// Episodes recorded since the last dream report before an ACP session
    /// triggers a dream, when [`Self::trigger_on_acp_episodes`] is `true`.
    ///
    /// Defaults to `10`. Zero is normalized to one at the runtime boundary
    /// (see [`Self::effective_acp_episode_threshold`]), so a dream always
    /// needs at least one new episode.
    #[serde(default = "default_acp_episode_threshold")]
    pub acp_episode_threshold: usize,
}

fn default_max_concurrent() -> usize {
    1
}

const fn default_acp_episode_threshold() -> usize {
    10
}

impl Default for DreamsConfig {
    fn default() -> Self {
        Self {
            trigger_on_plan_complete: true,
            max_concurrent: default_max_concurrent(),
            trigger_on_acp_episodes: false,
            acp_episode_threshold: default_acp_episode_threshold(),
        }
    }
}

impl DreamsConfig {
    /// Return the runtime-safe ACP episode threshold.
    #[must_use]
    pub const fn effective_acp_episode_threshold(&self) -> usize {
        if self.acp_episode_threshold == 0 {
            1
        } else {
            self.acp_episode_threshold
        }
    }

    /// Return the runtime-safe limit on concurrent ACP dream consolidations.
    #[must_use]
    pub const fn effective_max_concurrent(&self) -> usize {
        if self.max_concurrent == 0 {
            1
        } else {
            self.max_concurrent
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
    /// Run dream consolidation after a plan completes.
    ///
    /// Defaults to `false`: each automatic dream costs a model call, so dreams
    /// run on demand, through `roko knowledge dream run`. An explicit `true`
    /// opts in; [`DreamsConfig::trigger_on_plan_complete`] must also be `true`
    /// for the trigger to fire.
    #[serde(default)]
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
    /// Serve plan tasks that author no verify steps from matching T0 reflex
    /// rules (`.roko/learn/reflexes.jsonl`) instead of dispatching a model.
    ///
    /// Off by default: a reflex skips the provider and every gate, so its
    /// output is unverified and its rule earns no gate pass.
    #[serde(default)]
    pub t0_reflexes: bool,
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
    /// P3-26: Knowledge tier progression thresholds.
    #[serde(default)]
    pub knowledge: KnowledgeProgressionConfig,
}

/// P3-26: Configurable thresholds for knowledge tier promotion/demotion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeProgressionConfig {
    /// Confirmations needed for Transient -> Working promotion.
    #[serde(default = "default_transient_confirmations")]
    pub transient_confirmations: u32,
    /// Distinct contexts needed for Working -> Consolidated promotion.
    #[serde(default = "default_working_contexts")]
    pub working_contexts: u32,
    /// Minimum age in days for Consolidated -> Persistent promotion.
    #[serde(default = "default_consolidated_age_days")]
    pub consolidated_age_days: u32,
    /// Minimum balance for demotion consideration.
    #[serde(default = "default_demotion_balance")]
    pub demotion_balance_threshold: f64,
}

const fn default_transient_confirmations() -> u32 {
    2
}
const fn default_working_contexts() -> u32 {
    3
}
const fn default_consolidated_age_days() -> u32 {
    14
}
fn default_demotion_balance() -> f64 {
    0.1
}

impl Default for KnowledgeProgressionConfig {
    fn default() -> Self {
        Self {
            transient_confirmations: default_transient_confirmations(),
            working_contexts: default_working_contexts(),
            consolidated_age_days: default_consolidated_age_days(),
            demotion_balance_threshold: default_demotion_balance(),
        }
    }
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
            dream_on_completion: false,
            dreams: DreamsConfig::default(),
            use_lookahead_router: false,
            lookahead_threshold: default_lookahead_threshold(),
            t0_reflexes: false,
            override_learning_dampening: None,
            gate_threshold_flush_interval: default_gate_threshold_flush_interval(),
            knowledge: KnowledgeProgressionConfig::default(),
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

    #[test]
    fn dream_on_completion_defaults_to_false() {
        assert!(!LearningConfig::default().dream_on_completion);

        let omitted: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert!(!omitted.dream_on_completion);

        // An explicit opt-in is still honoured.
        let opted_in: LearningConfig =
            toml::from_str("dream_on_completion = true").expect("parse explicit opt-in");
        assert!(opted_in.dream_on_completion);
    }

    #[test]
    fn acp_dream_trigger_defaults_to_off() {
        let defaults = DreamsConfig::default();
        assert!(!defaults.trigger_on_acp_episodes);
        assert_eq!(defaults.acp_episode_threshold, 10);

        let omitted: LearningConfig = toml::from_str("").expect("parse empty learning config");
        assert_eq!(omitted.dreams, defaults);

        let opted_in: LearningConfig =
            toml::from_str("[dreams]\ntrigger_on_acp_episodes = true\nacp_episode_threshold = 4")
                .expect("parse ACP dream opt-in");
        assert!(opted_in.dreams.trigger_on_acp_episodes);
        assert_eq!(opted_in.dreams.effective_acp_episode_threshold(), 4);

        let zero: LearningConfig = toml::from_str("[dreams]\nacp_episode_threshold = 0")
            .expect("parse zero ACP threshold");
        assert_eq!(zero.dreams.effective_acp_episode_threshold(), 1);
    }
}
