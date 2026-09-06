//! Learning subsystems for Roko — episodic memory, playbooks, skills, caches,
//! and pattern discovery.
//!
//! These modules consume the signal stream produced by the orchestrator and
//! agents, persist durable records of what worked, and surface reusable
//! knowledge back to the composer/router feedback loop.
//!
//! # Modules
//!
//! - [`episode_logger`] — append-only JSONL record of agent turns
//! - [`playbook`] — reusable patterns extracted from episodes
//! - [`skill_library`] — structured skills agents can invoke
//! - [`context_pack_cache`] — cached composed prompts keyed by task fingerprint
//! - [`pattern_discovery`] — mining episodes for recurring shapes
//! - [`provider_health`] — per-provider circuit breaker for LLM routing
//! - [`latency`] — rolling latency EMAs and percentiles for routing feedback
//! - [`anomaly`] — runaway loop, cost spike, and quality degradation detection
//! - [`pareto`] — cost-quality Pareto frontier computation for models

#![deny(missing_docs)]
// Learning crate: numerics/telemetry-heavy with many trait impls returning literal &str,
// collapsible if chains in analysis code, and long aggregation functions.
#![allow(
    clippy::collapsible_if,
    clippy::collection_is_never_read,
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::float_cmp,
    clippy::implicit_hasher,
    clippy::iter_cloned_collect,
    clippy::many_single_char_names,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_continue,
    clippy::needless_range_loop,
    clippy::redundant_closure,
    clippy::struct_field_names,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines,
    clippy::unnecessary_literal_bound,
    clippy::question_mark,
    clippy::use_self
)]

/// Active inference helpers for tier routing support.
pub mod active_inference;
/// Efficiency trend aggregation helpers for JSONL telemetry.
pub mod aggregate;
pub mod anomaly;
pub mod bandits;
pub mod baseline;
/// Bayesian confidence updating using Beta-Binomial conjugate model (AS-07).
pub mod bayesian_confidence;
/// Budget tracking and enforcement guardrails for routing decisions.
pub mod budget;
/// Bus-backed calibration policy for predict-publish-correct loop (LEARN-09).
pub mod calibration_policy;
/// Extracted submodules for cascade router types, helpers, and persistence.
pub mod cascade;
pub mod cascade_router;
pub mod cfactor;
/// Learned intervention policy for conductor retries and aborts.
pub mod conductor;
pub mod context_pack_cache;
/// Pre-dispatch cost projection for budget estimation.
pub mod cost_projection;
pub mod cost_table;
pub mod costs_db;
pub mod costs_log;
/// Curriculum ordering helpers for task scheduling.
pub mod curriculum;
pub mod efficiency;
pub mod episode_logger;
/// Crate-level error types.
pub mod error;
/// Cheap pre-processing of noisy gate failures into retry-ready diagnoses.
pub mod error_enrichment;
/// Persistent storage for error patterns discovered during plan execution.
pub mod error_pattern_store;
/// Event subscriber that fans runtime events into learning subsystems.
pub mod event_subscriber;
/// Unified learning events emitted by routing, evaluation, and runtime feedback.
pub mod events;
/// Crash-durable prompt-experiment receipt shared across all dispatch surfaces.
pub mod experiment_receipt;
/// Concrete feedback sink for workflow learning telemetry.
pub mod feedback_service;
pub mod hdc_clustering;
/// HDC fingerprint helpers for episode memory.
pub mod hdc_fingerprint;
/// Heuristic, worldview, and research-provenance shells for learning parity.
pub mod heuristics;
/// Append-only hindsight corrections for recent episode outcomes.
pub mod hindsight;
/// Size-based rotation helper for append-only JSONL logs.
/// Rolling latency EMAs and percentiles for routing feedback.
pub mod latency;
pub mod local_reward;
/// Durable direct model-call feedback recorder.
pub mod model_call_feedback;
pub mod model_experiment;
pub mod model_router;
/// Domain-specific Oracle implementations (Chain, Coding, Research) and witness verification.
pub mod oracles;
pub mod pareto;
pub mod pattern_discovery;
pub mod playbook;
pub mod playbook_rules;
/// Structured post-gate reflections and reflection-derived playbook candidates.
pub mod post_gate_reflection;
pub mod prediction;
pub mod prompt_experiment;
pub mod provider_health;
/// Provider/model pass-rate outcome telemetry for future routing bandits.
pub mod provider_model_outcome;
pub mod quality_judge;
/// T0 reflex store — condition-action pairs learned from T2 episode promotions.
pub mod reflex_store;
pub mod regression;
/// Lookahead and calibration shells around the shipped cascade router.
pub mod routing_extras;
/// Append-only routing-decision audit log for explainability and dashboards.
pub mod routing_log;
/// Structured run-metrics persistence for plan runs.
pub mod run_metrics;
pub mod runtime_feedback;
pub mod section_effect;
/// Prompt/context section outcome telemetry for future adaptive policy.
pub mod section_outcome;
/// Shadow testing loop (Loop 12) — runs alternative configs alongside production tasks for A/B comparison.
pub mod shadow;
pub mod skill_library;
pub mod task_metric;
/// Verdict-aware scoring and routing history for gate-verdict re-entry (GATE-05).
pub mod verdict_scorer;
/// Write-Ahead Log for crash-safe learning state persistence.
pub mod wal;

/// Gate gaming detector — flags rising pass rates paired with falling quality.
pub mod gate_gaming;

/// Holdout experiment infrastructure for detecting overfitting in learned routing.
pub mod holdout;

pub use error::LearnError;
pub use feedback_service::{FeedbackService, KnowledgeOutcome};
pub use gate_gaming::{GamingAlert, GamingObservation, GateGamingDetector};
pub use holdout::{HoldoutExperiment, OverfittingAlert, Partition, RollingMetrics};
