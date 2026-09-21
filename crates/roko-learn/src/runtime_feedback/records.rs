//! Durable record types and event vocabulary for the runtime feedback layer.
//!
//! These are the serialized JSONL schemas for efficiency summaries, gate
//! outcomes, retry outcomes, knowledge seeds, and the runner event envelope.

use std::io;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::efficiency::AgentEfficiencyEvent;
use crate::episode_logger::{Episode, LoggerError};
use crate::provider_model_outcome::ProviderModelOutcomeRecord;
use crate::regression::{RegressionReport, RegressionThresholds};
use crate::skill_library::SkillLibraryError;
use roko_core::metric::TaskMetric;

use super::episode_helpers::{
    GateCounts, episode_model, episode_provider, episode_role, episode_run_id, episode_source_id,
    extra_bool, extra_f64, extra_string, extra_string_vec, extra_u64, gate_counts_from_episode,
    non_empty_string, nonzero_u64, prompt_section_count_from_episode, ratio_u64,
    retry_status_from_episode, stable_hash_hex,
};

// ── Schema version ────────────────────────────────────────────────────

/// Current schema version for runtime feedback JSONL records.
pub const RUNTIME_FEEDBACK_SCHEMA_VERSION: u32 = 1;

// ── LearningPaths ─────────────────────────────────────────────────────

use std::path::Path;

/// Well-known paths used by the learning runtime for persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningPaths {
    /// Root directory for runtime-managed learning artifacts.
    pub root: PathBuf,
    /// Append-only episode log.
    pub episodes_jsonl: PathBuf,
    /// Append-only cost log.
    pub costs_jsonl: PathBuf,
    /// JSON map of extracted skills.
    pub skills_json: PathBuf,
    /// Playbook JSON directory.
    pub playbooks_dir: PathBuf,
    /// TOML rules file for playbook rule confidence tracking.
    pub playbook_rules_toml: PathBuf,
    /// Append-only `TaskMetric` JSONL file used for regression checks.
    pub task_metrics_jsonl: PathBuf,
    /// Append-only efficiency events JSONL file.
    pub efficiency_jsonl: PathBuf,
    /// Append-only normalized efficiency summaries JSONL file.
    pub efficiency_summaries_jsonl: PathBuf,
    /// Append-only gate outcome JSONL file.
    pub gate_outcomes_jsonl: PathBuf,
    /// Append-only retry outcome JSONL file.
    pub retry_outcomes_jsonl: PathBuf,
    /// Append-only knowledge seed JSONL file for neuro ingestion.
    pub knowledge_seeds_jsonl: PathBuf,
    /// Persisted latency registry snapshot.
    pub latency_stats_json: PathBuf,
    /// Append-only C-Factor history JSONL file.
    pub cfactor_jsonl: PathBuf,
    /// Cascade router persisted observations JSON.
    pub cascade_router_json: PathBuf,
    /// Prompt experiment store JSON.
    pub experiments_json: PathBuf,
    /// Operator-facing summary of concluded experiment winners.
    pub experiment_winners_json: PathBuf,
    /// Adaptive gate thresholds JSON.
    pub gate_thresholds_json: PathBuf,
    /// Per-subsystem local reward functions JSON.
    pub local_rewards_json: PathBuf,
    /// Learned prompt section effectiveness snapshot.
    pub section_effects_json: PathBuf,
    /// Structured post-gate reflection records and candidates.
    pub post_gate_reflections_json: PathBuf,
    /// Append-only provider/model outcome telemetry for future bandits.
    pub provider_model_outcomes_jsonl: PathBuf,
    /// Write-Ahead Log for crash-safe learning state durability.
    pub wal_jsonl: PathBuf,
}

impl LearningPaths {
    /// Build the default path layout under `root`.
    #[must_use]
    pub fn under(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            episodes_jsonl: root.join("episodes.jsonl"),
            costs_jsonl: root.join("costs.jsonl"),
            skills_json: root.join("skills.json"),
            playbooks_dir: root.join("playbooks"),
            playbook_rules_toml: root.join("playbook-rules.toml"),
            task_metrics_jsonl: root.join("task-metrics.jsonl"),
            efficiency_jsonl: root.join("efficiency.jsonl"),
            efficiency_summaries_jsonl: root.join("efficiency-summaries.jsonl"),
            gate_outcomes_jsonl: root.join("gate-outcomes.jsonl"),
            retry_outcomes_jsonl: root.join("retry-outcomes.jsonl"),
            knowledge_seeds_jsonl: root.join("knowledge-seeds.jsonl"),
            latency_stats_json: root.join("latency-stats.json"),
            cfactor_jsonl: root.join("c-factor.jsonl"),
            cascade_router_json: root.join("cascade-router.json"),
            experiments_json: root.join("experiments.json"),
            experiment_winners_json: root.join("experiment-winners.json"),
            gate_thresholds_json: root.join("gate-thresholds.json"),
            local_rewards_json: root.join("local-rewards.json"),
            section_effects_json: root.join("section-effects.json"),
            post_gate_reflections_json: root.join("post-gate-reflections.json"),
            provider_model_outcomes_jsonl: root.join("provider-model-outcomes.jsonl"),
            wal_jsonl: root.join("wal.jsonl"),
            root,
        }
    }

    /// Build project paths with learning artifacts under `.roko/learn` and
    /// the sole active episode log at `.roko/episodes.jsonl`.
    #[must_use]
    pub fn for_project(workdir: impl AsRef<Path>) -> Self {
        Self::for_roko_dir(workdir.as_ref().join(".roko"))
    }

    /// Build project paths from an already-resolved `.roko` directory.
    #[must_use]
    pub fn for_roko_dir(roko_dir: impl AsRef<Path>) -> Self {
        let roko_dir = roko_dir.as_ref();
        let mut paths = Self::under(roko_dir.join("learn"));
        paths.episodes_jsonl = roko_dir.join("episodes.jsonl");
        paths
    }

    pub(crate) fn for_runtime_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        if root.file_name().and_then(|name| name.to_str()) == Some("learn")
            && root
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some(".roko")
        {
            return Self::for_roko_dir(root.parent().expect("learn root has .roko parent"));
        }
        Self::under(root)
    }
}

// ── Config types ──────────────────────────────────────────────────────

/// Optional knobs for regression detection in [`super::LearningRuntime`].
#[derive(Debug, Clone)]
pub struct RegressionConfig {
    /// Thresholds used by [`crate::regression::detect_regressions`].
    pub thresholds: RegressionThresholds,
    /// Number of latest metrics used as the "current" sample.
    pub current_window: usize,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            thresholds: RegressionThresholds::default(),
            current_window: 20,
        }
    }
}

/// Cadence controls for learning subsystems that should not all react on the
/// same episode boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFrequency {
    /// Cascade router observation cadence.
    pub router_every_n_episodes: u32,
    /// Reserved for orchestrator-managed adaptive gate threshold batching.
    pub gate_thresholds_every_n: u32,
    /// Prompt experiment outcome cadence.
    pub experiments_every_n: u32,
    /// Skill extraction cadence.
    pub skill_mining_every_n: u32,
    /// Pattern miner ingestion cadence.
    pub pattern_discovery_every_n: u32,
    /// Cross-episode consolidation cadence.
    pub distiller_every_n: u32,
}

impl UpdateFrequency {
    pub(crate) fn due(episode_count: u64, every_n: u32) -> bool {
        let cadence = u64::from(every_n.max(1));
        episode_count.is_multiple_of(cadence)
    }

    pub(crate) fn router_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.router_every_n_episodes)
    }

    pub(crate) fn experiments_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.experiments_every_n)
    }

    pub(crate) fn skill_mining_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.skill_mining_every_n)
    }

    pub(crate) fn pattern_discovery_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.pattern_discovery_every_n)
    }

    pub(crate) fn distiller_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.distiller_every_n)
    }

    pub(crate) fn gate_thresholds_due(self, episode_count: u64) -> bool {
        Self::due(episode_count, self.gate_thresholds_every_n)
    }
}

impl Default for UpdateFrequency {
    fn default() -> Self {
        Self {
            router_every_n_episodes: 1,
            gate_thresholds_every_n: 5,
            experiments_every_n: 1,
            skill_mining_every_n: 10,
            pattern_discovery_every_n: 20,
            distiller_every_n: 50,
        }
    }
}

// ── CompletedRunInput ─────────────────────────────────────────────────

/// Input payload for one completed runtime run.
#[derive(Debug, Clone)]
pub struct CompletedRunInput {
    /// Canonical episode for this run.
    pub episode: Episode,
    /// Optional explicit cost record.
    pub cost_record: Option<crate::costs_db::CostRecord>,
    /// Optional provider name when no explicit cost record is supplied.
    pub provider: Option<String>,
    /// Optional playbook id to update outcome counters.
    pub playbook_id: Option<String>,
    /// Optional playbook rule id to update confidence.
    pub playbook_rule_id: Option<String>,
    /// Optional skill id injected into prompt; updates validation counters.
    pub matched_skill_id: Option<String>,
    /// Optional metric for regression history.
    pub task_metric: Option<TaskMetric>,
    /// Optional prompt experiment variant id for A/B outcome recording.
    pub experiment_variant_id: Option<String>,
    /// Optional prompt experiment id paired with `experiment_variant_id`.
    /// When present, outcome recording is scoped to this experiment so
    /// duplicate variant ids in other experiments cannot be updated.
    pub experiment_id: Option<String>,
    /// Optional serialized adaptive gate-threshold JSON snapshot.
    ///
    /// When provided and the `gate_thresholds_every_n` cadence fires,
    /// `LearningRuntime` writes this snapshot to `gate-thresholds.json`
    /// atomically so thresholds survive unexpected shutdowns without
    /// relying on the orchestrator's graceful-shutdown path.
    pub gate_thresholds_snapshot: Option<String>,
}

impl CompletedRunInput {
    /// Construct an input from an episode.
    #[must_use]
    pub const fn from_episode(episode: Episode) -> Self {
        Self {
            episode,
            cost_record: None,
            provider: None,
            playbook_id: None,
            playbook_rule_id: None,
            matched_skill_id: None,
            task_metric: None,
            experiment_variant_id: None,
            experiment_id: None,
            gate_thresholds_snapshot: None,
        }
    }

    /// Attach an explicit cost record.
    #[must_use]
    pub fn with_cost_record(mut self, record: crate::costs_db::CostRecord) -> Self {
        self.cost_record = Some(record);
        self
    }

    /// Attach a task metric to update regression history.
    #[must_use]
    pub fn with_task_metric(mut self, metric: TaskMetric) -> Self {
        self.task_metric = Some(metric);
        self
    }

    /// Attach an experiment-scoped variant assignment.
    #[must_use]
    pub fn with_experiment_assignment(
        mut self,
        experiment_id: impl Into<String>,
        variant_id: impl Into<String>,
    ) -> Self {
        self.experiment_id = Some(experiment_id.into());
        self.experiment_variant_id = Some(variant_id.into());
        self
    }

    /// Attach a serialized adaptive gate-threshold snapshot for incremental
    /// flushing.  When the `gate_thresholds_every_n` cadence fires,
    /// `LearningRuntime::record_completed_run` writes this JSON to the
    /// learn directory so thresholds persist before graceful shutdown.
    #[must_use]
    pub fn with_gate_thresholds(mut self, json: String) -> Self {
        self.gate_thresholds_snapshot = Some(json);
        self
    }
}

// ── ApplyStatus / LearningUpdate ──────────────────────────────────────

/// Status of a specific learning side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApplyStatus {
    /// The subsystem was not updated for this run.
    #[default]
    Skipped,
    /// The subsystem was updated.
    Applied,
}

/// Summary of side effects produced by [`super::LearningRuntime::record_completed_run`].
#[derive(Debug, Clone, Default)]
pub struct LearningUpdate {
    /// Whether the episode was persisted.
    pub episode_logged: ApplyStatus,
    /// Whether a cost record was persisted.
    pub cost_logged: ApplyStatus,
    /// Whether provider health state was updated.
    pub provider_updated: ApplyStatus,
    /// Whether a playbook outcome was updated.
    pub playbook_updated: ApplyStatus,
    /// Whether a playbook rule outcome was updated.
    pub playbook_rule_updated: ApplyStatus,
    /// Newly extracted skill id, if extraction succeeded.
    pub extracted_skill_id: Option<String>,
    /// Whether an existing matched skill outcome was recorded.
    pub matched_skill_updated: ApplyStatus,
    /// Regression report when a task metric was provided and sufficient data exists.
    pub regression_report: Option<RegressionReport>,
    /// Whether pattern mining ingested this episode.
    pub patterns_ingested: bool,
    /// Whether the cascade router was updated with an observation.
    pub router_updated: bool,
    /// Whether a post-gate reflection record was persisted.
    pub reflection_recorded: ApplyStatus,
    /// Whether a reflection-derived playbook candidate was updated.
    pub reflection_candidate_updated: ApplyStatus,
    /// Whether a provider/model outcome record was persisted.
    pub provider_model_outcome_recorded: ApplyStatus,
    /// Whether a normalized efficiency summary was persisted.
    pub efficiency_summary_recorded: ApplyStatus,
    /// Number of gate outcome records persisted.
    pub gate_outcomes_recorded: usize,
    /// Whether a retry outcome record was persisted.
    pub retry_outcome_recorded: ApplyStatus,
    /// Whether a knowledge seed was persisted.
    pub knowledge_seed_recorded: ApplyStatus,
    /// Whether adaptive gate thresholds should be flushed to disk at this cadence point.
    ///
    /// Retained for backward compatibility with callers that handle the
    /// flush themselves (legacy path, now handled by runner-v2).
    pub gate_thresholds_flush_due: bool,
    /// Whether adaptive gate thresholds were incrementally flushed to disk
    /// by `LearningRuntime` during this `record_completed_run` call.
    ///
    /// When `CompletedRunInput::gate_thresholds_snapshot` is provided and the
    /// `gate_thresholds_every_n` cadence fires, the runtime writes the
    /// snapshot to `gate-thresholds.json` and sets this flag to `true`.
    /// Callers that see this flag can skip their own flush for this cycle.
    pub gate_thresholds_flushed: bool,
}

// ── Efficiency records ────────────────────────────────────────────────

/// Granularity represented by an efficiency summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EfficiencyScope {
    /// One provider/model turn.
    Turn,
    /// One task attempt, typically closed by a gate result.
    Task,
    /// One whole runner invocation or plan run.
    Run,
}

impl Default for EfficiencyScope {
    fn default() -> Self {
        Self::Task
    }
}

/// Normalized cost/token/latency summary for query surfaces.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EfficiencySummaryRecord {
    /// JSON schema version.
    pub schema_version: u32,
    /// ISO-8601 timestamp for the observation.
    pub timestamp: String,
    /// Summary granularity.
    pub scope: EfficiencyScope,
    /// Optional runner/session/run identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Stable episode identifier when this summary came from an episode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    /// Plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Task identifier.
    #[serde(default)]
    pub task_id: String,
    /// Agent identifier.
    #[serde(default)]
    pub agent_id: String,
    /// Agent role/profile label.
    #[serde(default)]
    pub role: String,
    /// Provider/backend identifier.
    #[serde(default)]
    pub provider: String,
    /// Model slug.
    #[serde(default)]
    pub model: String,
    /// Retry/turn iteration number.
    #[serde(default)]
    pub iteration: u32,
    /// Input tokens.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens.
    #[serde(default)]
    pub output_tokens: u64,
    /// Reasoning/thinking tokens when available.
    #[serde(default)]
    pub reasoning_tokens: u64,
    /// Cache-read tokens.
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Cache-write tokens.
    #[serde(default)]
    pub cache_write_tokens: u64,
    /// Total input plus output tokens.
    #[serde(default)]
    pub total_tokens: u64,
    /// Observed cost in USD.
    #[serde(default)]
    pub cost_usd: f64,
    /// Estimated cost without cache discount.
    #[serde(default)]
    pub cost_usd_without_cache: f64,
    /// Cache hit rate in `[0.0, 1.0]`.
    #[serde(default)]
    pub cache_hit_rate: f64,
    /// Wall-clock duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Time to first token in milliseconds.
    #[serde(default)]
    pub time_to_first_token_ms: u64,
    /// Number of tools exposed to the agent.
    #[serde(default)]
    pub tools_available: u32,
    /// Number of tools used by the agent.
    #[serde(default)]
    pub tools_used: u32,
    /// Number of tool calls observed.
    #[serde(default)]
    pub tool_calls: u32,
    /// Whether the closing gate passed, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_passed: Option<bool>,
    /// Outcome label such as `success`, `failure`, or provider finish reason.
    #[serde(default)]
    pub outcome: String,
    /// Number of prompt sections represented in this summary.
    #[serde(default)]
    pub prompt_section_count: u32,
    /// Total prompt tokens represented in this summary.
    #[serde(default)]
    pub total_prompt_tokens: u64,
    /// Extra forward-compatible metadata.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
}

impl EfficiencySummaryRecord {
    /// Build a task-level summary from a completed episode.
    #[must_use]
    pub fn from_episode(episode: &Episode) -> Self {
        let provider = episode_provider(episode);
        let model = episode_model(episode);
        let total_tokens = if episode.tokens_used > 0 {
            episode.tokens_used
        } else {
            episode
                .usage
                .input_tokens
                .saturating_add(episode.usage.output_tokens)
        };
        let prompt_section_count = prompt_section_count_from_episode(episode);
        let gate_counts = gate_counts_from_episode(episode);
        let has_only_skipped = gate_counts.is_some_and(GateCounts::has_only_skipped);

        Self {
            schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
            timestamp: episode.completed_at.to_rfc3339(),
            scope: EfficiencyScope::Task,
            run_id: episode_run_id(episode),
            episode_id: Some(episode_source_id(episode).to_string()),
            plan_id: extra_string(episode, "plan_id").unwrap_or_default(),
            task_id: episode.task_id.clone(),
            agent_id: episode.agent_id.clone(),
            role: episode_role(episode),
            provider,
            model,
            iteration: extra_u64(episode, "iteration")
                .or_else(|| extra_u64(episode, "retry_count"))
                .unwrap_or(0)
                .min(u64::from(u32::MAX)) as u32,
            input_tokens: episode.usage.input_tokens,
            output_tokens: episode.usage.output_tokens,
            reasoning_tokens: extra_u64(episode, "reasoning_tokens").unwrap_or(0),
            cache_read_tokens: episode.usage.cache_read_tokens,
            cache_write_tokens: episode.usage.cache_write_tokens,
            total_tokens,
            cost_usd: episode.usage.cost_usd,
            cost_usd_without_cache: episode.usage.cost_usd_without_cache,
            cache_hit_rate: ratio_u64(episode.usage.cache_read_tokens, episode.usage.input_tokens),
            duration_ms: episode.usage.wall_ms,
            time_to_first_token_ms: extra_u64(episode, "time_to_first_token_ms").unwrap_or(0),
            tools_available: extra_u64(episode, "tools_available")
                .unwrap_or(0)
                .min(u64::from(u32::MAX)) as u32,
            tools_used: extra_u64(episode, "tools_used")
                .unwrap_or(episode.external_actions.len() as u64)
                .min(u64::from(u32::MAX)) as u32,
            tool_calls: episode.external_actions.len().min(u32::MAX as usize) as u32,
            gate_passed: if has_only_skipped {
                None
            } else {
                Some(episode.success)
            },
            outcome: if has_only_skipped {
                extra_string(episode, "provider_model_outcome_status")
                    .unwrap_or_else(|| "blocked".to_string())
            } else if episode.success {
                "success".to_string()
            } else {
                episode
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| "failure".to_string())
            },
            prompt_section_count,
            total_prompt_tokens: extra_u64(episode, "total_prompt_tokens").unwrap_or(0),
            metadata: serde_json::json!({
                "source": "episode",
                "kind": episode.kind.clone(),
                "trigger_kind": episode.trigger_kind.clone(),
                "gate_count": episode.gate_verdicts.len(),
                "gate_summary": gate_counts.map(|counts| counts.summary()),
                "gate_pass_rate": gate_counts.map(|counts| counts.pass_rate()),
                "gate_counts": gate_counts.map(|counts| serde_json::json!({
                    "passed": counts.passed,
                    "failed": counts.failed,
                    "skipped": counts.skipped,
                    "executed": counts.executed(),
                    "summary": counts.summary(),
                    "pass_rate": counts.pass_rate(),
                })),
                "gates_passed": gate_counts.map(|counts| counts.passed),
                "gates_failed": gate_counts.map(|counts| counts.failed),
                "gates_skipped": gate_counts.map(|counts| counts.skipped),
                "gates_executed": gate_counts.map(|counts| counts.executed()),
                "provider_model_outcome_status": extra_string(episode, "provider_model_outcome_status"),
            }),
        }
    }

    /// Build a summary from an existing efficiency event.
    #[must_use]
    pub fn from_efficiency_event(event: &AgentEfficiencyEvent, scope: EfficiencyScope) -> Self {
        Self {
            schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
            timestamp: if event.timestamp.trim().is_empty() {
                Utc::now().to_rfc3339()
            } else {
                event.timestamp.clone()
            },
            scope,
            run_id: non_empty_string(event.plan_id.as_str()),
            episode_id: None,
            plan_id: event.plan_id.clone(),
            task_id: event.task_id.clone(),
            agent_id: event.agent_id.clone(),
            role: event.role.clone(),
            provider: event.backend.clone(),
            model: super::routing::latency_model_slug(event).to_string(),
            iteration: event.iteration,
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
            reasoning_tokens: event.reasoning_tokens,
            cache_read_tokens: event.cache_read_tokens,
            cache_write_tokens: event.cache_write_tokens,
            total_tokens: event.total_tokens(),
            cost_usd: event.cost_usd,
            cost_usd_without_cache: event.cost_usd_without_cache,
            cache_hit_rate: event.cache_hit_rate(),
            duration_ms: event.duration_ms.max(event.wall_time_ms),
            time_to_first_token_ms: event.time_to_first_token_ms,
            tools_available: event.tools_available,
            tools_used: event.tools_used,
            tool_calls: event.tool_calls.len().min(u32::MAX as usize) as u32,
            gate_passed: event.gate_passed,
            outcome: if event.outcome.trim().is_empty() {
                if event.gate_passed == Some(true) {
                    "success".to_string()
                } else {
                    "failure".to_string()
                }
            } else {
                event.outcome.clone()
            },
            prompt_section_count: event.prompt_sections.len().min(u32::MAX as usize) as u32,
            total_prompt_tokens: event.total_prompt_tokens,
            metadata: serde_json::json!({
                "source": "efficiency_event",
                "frequency": event.frequency,
                "strategy_attempted": event.strategy_attempted.clone(),
                "gate_errors": event.gate_errors.clone(),
            }),
        }
    }
}

// ── Gate outcome records ──────────────────────────────────────────────

/// One durable gate outcome emitted by the runner or derived from an episode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GateOutcomeRecord {
    /// JSON schema version.
    pub schema_version: u32,
    /// ISO-8601 timestamp for the observation.
    pub timestamp: String,
    /// Optional runner/session/run identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Stable episode identifier when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    /// Plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Task identifier.
    #[serde(default)]
    pub task_id: String,
    /// Gate identifier, such as `compile`, `clippy`, or `test`.
    #[serde(default)]
    pub gate_name: String,
    /// Gate family or effect kind, such as `gate` or `plan_verify`.
    #[serde(default)]
    pub gate_kind: String,
    /// Gate rung number.
    #[serde(default)]
    pub rung: u32,
    /// Whether the gate passed.
    #[serde(default)]
    pub passed: bool,
    /// Optional numeric score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    /// Gate duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Retry attempt/iteration associated with this gate.
    #[serde(default)]
    pub attempt: u32,
    /// Runner-level failure classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<String>,
    /// Short error digest/signature, never raw gate output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_digest: Option<String>,
    /// Provider/backend identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model slug.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Short human-readable summary.
    #[serde(default)]
    pub summary: String,
    /// Extra forward-compatible metadata.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
}

impl GateOutcomeRecord {
    /// Build gate outcome records from an episode's gate verdicts.
    #[must_use]
    pub fn from_episode(episode: &Episode) -> Vec<Self> {
        let provider = non_empty_string(episode_provider(episode).as_str());
        let model = non_empty_string(episode_model(episode).as_str());
        let plan_id = extra_string(episode, "plan_id").unwrap_or_default();
        let run_id = episode_run_id(episode);
        let episode_id = Some(episode_source_id(episode).to_string());
        let attempt = extra_u64(episode, "iteration")
            .or_else(|| extra_u64(episode, "retry_count"))
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32;
        let duration_ms = nonzero_u64(extra_u64(episode, "gate_duration_ms").unwrap_or(0));
        let gate_counts = gate_counts_from_episode(episode);

        episode
            .gate_verdicts
            .iter()
            .enumerate()
            .map(|(idx, verdict)| Self {
                schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
                timestamp: episode.completed_at.to_rfc3339(),
                run_id: run_id.clone(),
                episode_id: episode_id.clone(),
                plan_id: plan_id.clone(),
                task_id: episode.task_id.clone(),
                gate_name: verdict.gate.clone(),
                gate_kind: extra_string(episode, "gate_kind").unwrap_or_else(|| "gate".into()),
                rung: extra_u64(episode, "rung")
                    .unwrap_or(idx as u64)
                    .min(u64::from(u32::MAX)) as u32,
                passed: verdict.passed,
                score: extra_f64(episode, "gate_score").map(|score| score as f32),
                duration_ms,
                attempt,
                failure_kind: extra_string(episode, "failure_kind"),
                error_digest: verdict.signature.clone(),
                provider: provider.clone(),
                model: model.clone(),
                summary: verdict.signature.clone().unwrap_or_default(),
                metadata: serde_json::json!({
                    "source": "episode",
                    "episode_kind": episode.kind.clone(),
                    "gate_counts": gate_counts.map(|counts| serde_json::json!({
                        "passed": counts.passed,
                        "failed": counts.failed,
                        "skipped": counts.skipped,
                        "executed": counts.executed(),
                        "summary": counts.summary(),
                        "pass_rate": counts.pass_rate(),
                    })),
                    "gates_passed": gate_counts.map(|counts| counts.passed),
                    "gates_failed": gate_counts.map(|counts| counts.failed),
                    "gates_skipped": gate_counts.map(|counts| counts.skipped),
                    "gates_executed": gate_counts.map(|counts| counts.executed()),
                    "gate_summary": gate_counts.map(|counts| counts.summary()),
                    "gate_pass_rate": gate_counts.map(|counts| counts.pass_rate()),
                }),
            })
            .collect()
    }
}

// ── Retry outcome records ─────────────────────────────────────────────

/// Retry lifecycle status emitted by the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryOutcomeStatus {
    /// A retry was scheduled but has not started yet.
    Scheduled,
    /// A retry attempt started.
    Started,
    /// A retry eventually passed its terminal gate.
    Succeeded,
    /// Retry budget was exhausted.
    Exhausted,
    /// Retry was skipped because the failure was non-retryable.
    NotRetryable,
    /// Retry was cancelled by operator/runtime shutdown.
    Cancelled,
}

impl Default for RetryOutcomeStatus {
    fn default() -> Self {
        Self::Scheduled
    }
}

/// One retry-policy outcome, append-only and queryable by plan/task.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryOutcomeRecord {
    /// JSON schema version.
    pub schema_version: u32,
    /// ISO-8601 timestamp for the observation.
    pub timestamp: String,
    /// Optional runner/session/run identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Stable episode identifier when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    /// Plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Task identifier.
    #[serde(default)]
    pub task_id: String,
    /// Gate identifier that triggered the retry decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_name: Option<String>,
    /// Attempt number after the decision.
    #[serde(default)]
    pub attempt: u32,
    /// Configured maximum attempts when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u32>,
    /// Retry status.
    pub status: RetryOutcomeStatus,
    /// Whether the triggering failure was retryable.
    #[serde(default)]
    pub retryable: bool,
    /// Runner-level failure classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<String>,
    /// Cooldown before next retry, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<u64>,
    /// Provider/backend identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model slug.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Short reason for the decision.
    #[serde(default)]
    pub reason: String,
    /// Next runner action, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    /// Extra forward-compatible metadata.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
}

impl RetryOutcomeRecord {
    /// Build a retry outcome from episode metadata when the runner supplied retry fields.
    #[must_use]
    pub fn from_episode(episode: &Episode) -> Option<Self> {
        let status = retry_status_from_episode(episode)?;
        let provider = non_empty_string(episode_provider(episode).as_str());
        let model = non_empty_string(episode_model(episode).as_str());

        Some(Self {
            schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
            timestamp: episode.completed_at.to_rfc3339(),
            run_id: episode_run_id(episode),
            episode_id: Some(episode_source_id(episode).to_string()),
            plan_id: extra_string(episode, "plan_id").unwrap_or_default(),
            task_id: episode.task_id.clone(),
            gate_name: extra_string(episode, "gate_name").or_else(|| {
                episode
                    .gate_verdicts
                    .iter()
                    .find(|verdict| !verdict.passed)
                    .map(|verdict| verdict.gate.clone())
            }),
            attempt: extra_u64(episode, "retry_attempt")
                .or_else(|| extra_u64(episode, "iteration"))
                .unwrap_or(0)
                .min(u64::from(u32::MAX)) as u32,
            max_attempts: extra_u64(episode, "max_retries")
                .map(|value| value.min(u64::from(u32::MAX)) as u32),
            status,
            retryable: extra_bool(episode, "retryable").unwrap_or(matches!(
                status,
                RetryOutcomeStatus::Scheduled
                    | RetryOutcomeStatus::Started
                    | RetryOutcomeStatus::Succeeded
            )),
            failure_kind: extra_string(episode, "failure_kind"),
            cooldown_ms: extra_u64(episode, "retry_cooldown_ms"),
            provider,
            model,
            reason: extra_string(episode, "retry_reason")
                .or_else(|| episode.failure_reason.clone())
                .unwrap_or_default(),
            next_action: extra_string(episode, "retry_next_action"),
            metadata: serde_json::json!({
                "source": "episode",
                "episode_success": episode.success,
            }),
        })
    }
}

// ── Knowledge seed records ────────────────────────────────────────────

/// Evidence item supporting a knowledge seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeSeedEvidence {
    /// Evidence source type, such as `episode` or `gate`.
    pub source_type: String,
    /// Stable source identifier.
    pub source_id: String,
    /// Outcome label associated with the evidence.
    pub outcome: String,
    /// Evidence weight in `[0.0, 1.0]`.
    pub weight: f64,
}

/// Lightweight, dependency-free knowledge candidate emitted by runtime feedback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeSeedRecord {
    /// JSON schema version.
    pub schema_version: u32,
    /// Stable deterministic seed id.
    pub seed_id: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Knowledge kind label compatible with `roko-neuro` (`insight`, `warning`, etc.).
    pub kind: String,
    /// Candidate knowledge content.
    pub content: String,
    /// Starting confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Signed retrieval weight seed.
    pub confidence_weight: f64,
    /// Episode ids that support the seed.
    #[serde(default)]
    pub source_episodes: Vec<String>,
    /// Source model when the seed may be model-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_model: Option<String>,
    /// Generality across model families (`1.0` = fully general).
    pub model_generality: f64,
    /// Topic tags for retrieval and admission filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Task identifier.
    #[serde(default)]
    pub task_id: String,
    /// Evidence that caused this seed to be emitted.
    #[serde(default)]
    pub evidence: Vec<KnowledgeSeedEvidence>,
    /// Extra forward-compatible metadata.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub metadata: serde_json::Value,
}

impl KnowledgeSeedRecord {
    /// Build a deterministic knowledge seed from a successful episode.
    #[must_use]
    pub fn from_successful_episode(episode: &Episode) -> Option<Self> {
        if !episode.success
            || gate_counts_from_episode(episode).is_some_and(GateCounts::has_only_skipped)
        {
            return None;
        }

        let source_id = episode_source_id(episode).to_string();
        let plan_id = extra_string(episode, "plan_id").unwrap_or_default();
        let task_category = extra_string(episode, "task_category")
            .unwrap_or_else(|| episode.trigger_kind.clone())
            .trim()
            .to_string();
        let role = episode_role(episode);
        let provider = episode_provider(episode);
        let model = episode_model(episode);
        let gate_names = episode
            .gate_verdicts
            .iter()
            .map(|verdict| verdict.gate.clone())
            .filter(|gate| !gate.trim().is_empty())
            .collect::<Vec<_>>();
        let files = extra_string_vec(episode, "files")
            .or_else(|| extra_string_vec(episode, "files_changed"))
            .unwrap_or_default();
        let gates_label = if gate_names.is_empty() {
            "terminal success".to_string()
        } else {
            gate_names.join(", ")
        };
        let files_label = if files.is_empty() {
            "no file scope recorded".to_string()
        } else {
            files.join(", ")
        };
        let task_label = if episode.task_id.trim().is_empty() {
            "unknown task"
        } else {
            episode.task_id.as_str()
        };
        let content = format!(
            "Successful {task_category} task {task_label} used role {role}, provider {provider}, model {model}, and passed {gates_label}. File scope: {files_label}."
        );
        let confidence = (0.70
            + (gate_names.len().min(5) as f64 * 0.025)
            + if model.is_empty() { 0.0 } else { 0.02 })
        .clamp(0.70, 0.85);
        let kind = if gate_names.len() >= 2 {
            "strategy_fragment"
        } else {
            "insight"
        }
        .to_string();
        let seed_id = format!(
            "ks-{}",
            stable_hash_hex(&[
                kind.as_str(),
                source_id.as_str(),
                episode.task_id.as_str(),
                model.as_str(),
                content.as_str(),
            ])
        );
        let mut tags = extra_string_vec(episode, "task_tags").unwrap_or_default();
        tags.extend([
            "runtime-success".to_string(),
            task_category.clone(),
            role.clone(),
            provider.clone(),
        ]);
        tags.retain(|tag| !tag.trim().is_empty());
        for tag in tags.iter_mut() {
            *tag = tag.trim().to_ascii_lowercase();
        }
        tags.sort();
        tags.dedup();

        Some(Self {
            schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
            seed_id,
            created_at: episode.completed_at.to_rfc3339(),
            kind,
            content,
            confidence,
            confidence_weight: confidence,
            source_episodes: vec![source_id.clone()],
            source_model: non_empty_string(model.as_str()),
            model_generality: if model.is_empty() { 0.75 } else { 0.35 },
            tags,
            plan_id,
            task_id: episode.task_id.clone(),
            evidence: vec![KnowledgeSeedEvidence {
                source_type: "episode".to_string(),
                source_id,
                outcome: "success".to_string(),
                weight: confidence,
            }],
            metadata: serde_json::json!({
                "provider": provider,
                "model": model,
                "role": role,
                "task_category": task_category,
                "gate_names": gate_names,
                "files": files,
                "tokens": {
                    "input": episode.usage.input_tokens,
                    "output": episode.usage.output_tokens,
                    "cache_read": episode.usage.cache_read_tokens,
                    "cache_write": episode.usage.cache_write_tokens,
                },
                "cost_usd": episode.usage.cost_usd,
            }),
        })
    }
}

// ── Runner feedback event envelope ────────────────────────────────────

/// Normalized runner event accepted by the runtime feedback facade.
#[derive(Debug, Clone)]
pub enum RunnerFeedbackEvent {
    /// A completed run should be persisted and fanned out to all derived feedback logs.
    CompletedRun {
        /// Completed-run input.
        input: Box<CompletedRunInput>,
    },
    /// A raw episode should be appended and projected into derived feedback logs.
    Episode {
        /// Episode record.
        episode: Box<Episode>,
    },
    /// A raw efficiency event should be appended and projected into summaries.
    EfficiencyEvent {
        /// Efficiency event.
        event: AgentEfficiencyEvent,
        /// Summary scope for the derived record.
        scope: EfficiencyScope,
    },
    /// A provider/model outcome should be appended directly.
    ProviderModelOutcome {
        /// Provider/model outcome record.
        outcome: ProviderModelOutcomeRecord,
    },
    /// A normalized efficiency summary should be appended directly.
    EfficiencySummary {
        /// Efficiency summary record.
        summary: EfficiencySummaryRecord,
    },
    /// A gate outcome should be appended directly.
    GateOutcome {
        /// Gate outcome record.
        outcome: GateOutcomeRecord,
    },
    /// A retry outcome should be appended directly.
    RetryOutcome {
        /// Retry outcome record.
        outcome: RetryOutcomeRecord,
    },
    /// A knowledge seed should be appended directly.
    KnowledgeSeed {
        /// Knowledge seed record.
        seed: KnowledgeSeedRecord,
    },
}

// ── Generation outcome ────────────────────────────────────────────────

/// Opaque artifact validation payload carried alongside generation outcomes.
pub type ArtifactValidationReport = serde_json::Value;

/// Outcome of a generation operation, distinguishing process from artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOutcome {
    /// Whether the agent process completed without error.
    pub process_success: bool,
    /// Whether the generated artifact passes grounding validation.
    pub artifact_valid: bool,
    /// Validation report (if validation ran).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_report: Option<ArtifactValidationReport>,
}

impl GenerationOutcome {
    /// True only when both process succeeded AND artifact is valid.
    #[must_use]
    pub fn fully_successful(&self) -> bool {
        self.process_success && self.artifact_valid
    }

    /// Status string for display and logging.
    #[must_use]
    pub fn status_label(&self) -> &'static str {
        match (self.process_success, self.artifact_valid) {
            (true, true) => "success",
            (true, false) => "partial_success",
            (false, _) => "failure",
        }
    }
}

// ── RuntimeFeedbackWrite ──────────────────────────────────────────────

/// Counts of append-only records written by a feedback facade call.
#[derive(Debug, Clone, Default)]
pub struct RuntimeFeedbackWrite {
    /// Completed-run update when a completed run was recorded.
    pub learning_update: Option<LearningUpdate>,
    /// Number of raw efficiency events appended.
    pub efficiency_events: usize,
    /// Number of provider/model outcomes appended directly by the facade.
    pub provider_model_outcomes: usize,
    /// Number of efficiency summaries appended.
    pub efficiency_summaries: usize,
    /// Number of gate outcomes appended.
    pub gate_outcomes: usize,
    /// Number of retry outcomes appended.
    pub retry_outcomes: usize,
    /// Number of knowledge seeds appended.
    pub knowledge_seeds: usize,
    /// Whether an episode was appended directly by the facade.
    pub episode_appended: bool,
}

impl RuntimeFeedbackWrite {
    pub(crate) fn merge(&mut self, other: Self) {
        self.efficiency_events += other.efficiency_events;
        self.provider_model_outcomes += other.provider_model_outcomes;
        self.efficiency_summaries += other.efficiency_summaries;
        self.gate_outcomes += other.gate_outcomes;
        self.retry_outcomes += other.retry_outcomes;
        self.knowledge_seeds += other.knowledge_seeds;
        self.episode_appended |= other.episode_appended;
        if self.learning_update.is_none() {
            self.learning_update = other.learning_update;
        }
    }
}

// ── Query / Snapshot ──────────────────────────────────────────────────

/// Query filters for append-only runtime feedback logs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeFeedbackQuery {
    /// Filter by plan id.
    pub plan_id: Option<String>,
    /// Filter by task id.
    pub task_id: Option<String>,
    /// Filter by episode id.
    pub episode_id: Option<String>,
    /// Filter by provider/backend.
    pub provider: Option<String>,
    /// Filter by model slug.
    pub model: Option<String>,
    /// Keep only the latest N records after filtering.
    pub limit: Option<usize>,
}

/// Query result spanning all canonical runtime feedback streams.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuntimeFeedbackSnapshot {
    /// Episode records.
    pub episodes: Vec<Episode>,
    /// Provider/model outcome records.
    pub provider_model_outcomes: Vec<ProviderModelOutcomeRecord>,
    /// Efficiency summary records.
    pub efficiency_summaries: Vec<EfficiencySummaryRecord>,
    /// Gate outcome records.
    pub gate_outcomes: Vec<GateOutcomeRecord>,
    /// Retry outcome records.
    pub retry_outcomes: Vec<RetryOutcomeRecord>,
    /// Knowledge seed records.
    pub knowledge_seeds: Vec<KnowledgeSeedRecord>,
}

// ── Error ─────────────────────────────────────────────────────────────

/// Errors produced by [`super::LearningRuntime`].
#[derive(Debug, Error)]
pub enum LearningRuntimeError {
    /// Filesystem errors.
    #[error("learning runtime io error: {0}")]
    Io(#[from] io::Error),
    /// Episode logger errors.
    #[error("learning runtime episode error: {0}")]
    Episode(#[from] LoggerError),
    /// Skill library errors.
    #[error("learning runtime skill error: {0}")]
    Skill(#[from] SkillLibraryError),
    /// JSON serialization/parsing errors.
    #[error("learning runtime serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Learning subsystem errors.
    #[error("learning subsystem error: {0}")]
    Learn(#[from] crate::error::LearnError),
}
