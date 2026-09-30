//! Concrete verification gates and orchestration primitives for Roko.
//!
//! This crate ships the verification stack described in `docs/v1/04-verification`:
//! concrete gate implementations, the rung selector, the sequential pipeline,
//! adaptive thresholds, runtime dispatch, and the agent feedback filter.
//! Gates verify a signal against ground truth and produce
//! [`Verdict`](roko_core::Verdict)s that flow back into the substrate as
//! signals.
//!
//! # Verify architecture: two-tier system
//!
//! ## Rung-dispatched gates (7 rungs)
//!
//! The canonical 7-rung pipeline is dispatched via [`rung_dispatch`] based on
//! plan complexity. Each rung maps to one or more concrete gates:
//!
//! | Rung | Index | Gates |
//! |------|-------|-------|
//! | `Compile` | 0 | [`CompileGate`] |
//! | `Lint` | 1 | [`ClippyGate`] |
//! | `Test` | 2 | [`TestGate`] |
//! | `Symbol` | 3 | [`SymbolGate`](symbol_gate::SymbolGate) |
//! | `GeneratedTest` | 4 | [`GeneratedTestGate`](generated_test_gate::GeneratedTestGate) + [`VerifyChainGate`](verify_chain_gate::VerifyChainGate) |
//! | `PropertyTest` | 5 | [`PropertyTestGate`](property_test_gate::PropertyTestGate) + [`FactCheckGate`] |
//! | `Integration` | 6 | [`LlmJudgeGate`](llm_judge_gate::LlmJudgeGate) + [`IntegrationGate`](integration_gate::IntegrationGate) |
//!
//! ## Standalone gates (6 gates)
//!
//! These gates are invoked outside the rung pipeline for specific scenarios:
//!
//! - [`DiffGate`] -- diff analysis (post-task review)
//! - [`CodeExecutionGate`] -- sandboxed code execution
//! - [`ShellGate`] -- arbitrary shell command verification
//! - [`BenchmarkRegressionGate`](benchmark_gate::BenchmarkRegressionGate) -- performance benchmarks
//! - [`FormatCheckGate`](format_check_gate::FormatCheckGate) -- code formatting
//! - [`SecurityScanGate`](security_scan_gate::SecurityScanGate) -- security scanning
//!
//! ## Ad-hoc generated checks
//!
//! - [`GateGenerator`] / [`GeneratedCheck`] -- dynamically generated verification checks
//!
//! ## Composition wrappers
//!
//! - [`ParallelGate`] -- run multiple gates in parallel, collect all verdicts
//! - [`VotingGate`] -- majority-vote across inner gates
//! - [`FallbackGate`] -- try gates in order, use first non-error verdict
//!
//! The crate root re-exports the stable verification API so callers can build
//! selectors, pipelines, thresholds, dispatchers, and feedback transforms
//! without reaching into submodules.

// Gate crate: many gate structs with trait-bound literal returns, numeric thresholds,
// and long verification functions. These lints are pervasive and structural.
#![allow(
    clippy::collection_is_never_read,
    clippy::literal_string_with_formatting_args,
    clippy::manual_clamp,
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::missing_fields_in_debug,
    clippy::module_name_repetitions,
    clippy::needless_range_loop,
    clippy::redundant_closure,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::unnecessary_literal_bound,
    clippy::unreadable_literal
)]

pub mod adaptive_threshold;

/// Chaos engineering / fault injection framework for evaluation testing.
///
/// Compiled only when the `chaos` feature is enabled. Activate with:
/// `cargo test -p roko-gate --features chaos`
/// and set `ROKO_CHAOS_MODE=1` to opt in at runtime.
#[cfg(feature = "chaos")]
pub mod chaos;

pub mod acceptance_contract;
pub mod artifact_store;
pub mod attempt_diff;
/// Criterion benchmark regression detection: parse JSON output, compare against baselines.
pub mod benchmark_gate;
mod cancel_safe_command;
pub mod clippy_gate;
pub mod code_exec;
pub mod compile;
/// Structured compile error classification: parse cargo JSON, classify by category.
pub mod compile_errors;
/// Standalone gate combinators: ParallelGate, VotingGate, FallbackGate (GATE-04).
pub mod composition;
pub mod diff_gate;
pub mod env_builder;
pub mod error;
pub mod error_patterns;
pub mod eval_generator;
pub mod fact_check;
pub mod feedback;
/// Forensic causal chain reconstruction from content-addressed artifacts (GATE-07).
pub mod forensic;
pub mod gate_env;
pub mod gate_pipeline;
pub mod gate_service;
pub mod generated;
pub mod generated_test_gate;
/// Graph-compatible Cell wrapper for the production gate pipeline (#250).
pub mod graph_cell;
/// Multi-gate joint anomaly detection via Hotelling's T-squared (GATE-08).
pub mod hotelling;
pub mod integration_gate;
pub mod judge_calibration;
pub mod llm_judge_gate;
pub mod payload;
/// PELT (Pruned Exact Linear Time) offline change point detection (P1-13).
pub mod pelt;
pub mod process_reward;
/// Production gate request types shared between Runner-v2 and Graph (#250).
pub mod production_request;
/// Production gate service trait and implementation (#250/#275).
pub mod production_service;
/// Production gate verdict types shared between Runner-v2 and Graph (#250).
pub mod production_verdict;
pub mod property_test_gate;
pub mod ratchet;
/// Shared gate metadata, status conversion, and alias/rung resolution.
pub mod registry;
pub mod review_verdict;
pub mod rung_dispatch;
pub mod rung_selector;
pub mod shell;
/// Statistical Process Control extensions: CUSUM, EWMA Control Chart, BOCPD (GATE-01).
pub mod spc;
/// Static spec-quality score for task specs: speclint's `sq-2` rules (S07.7).
pub mod spec_quality;
pub mod symbol_gate;
pub mod test_gate;
pub mod verdict_publisher;
pub mod verify_chain_gate;

pub use acceptance_contract::{
    AcceptanceContract, AcceptanceDecision, AcceptanceEvidence, AcceptanceIssue, AcceptanceOutcome,
    GateEvidence, GateRequirement, GateRequirementKind, NoStubEvidence, NoStubRequirement,
    ParityLedgerEvidenceRow, ParityLedgerRequirement, ParityLedgerRequirementRow,
    ParityLedgerStatus, RecoveryEvidence, RecoveryRequirement, RequiredNextAction,
    ReviewVerdictEvidence, ReviewVerdictRequirement, StructuredAgentOutputRequirement,
    StructuredOutputEvidence,
};
pub use adaptive_threshold::{AdaptiveThresholds, RungStats, TOTAL_RUNGS};
pub use artifact_store::ArtifactStore;
pub use benchmark_gate::{BenchmarkComparison, BenchmarkRegressionGate};
pub use clippy_gate::ClippyGate;
pub use code_exec::{
    CodeExecutionBackend, CodeExecutionGate, CodeExecutionOutcome, CodeExecutionPayload,
};
pub use compile::CompileGate;
pub use compile_errors::{
    CompileError, CompileErrorSummary, ErrorCategory, FailureClass, GateFailureAction,
    GateFailureClassification, GateFailureKind, GateFailureRecord, GateRetryPolicy,
    classify_error_code, classify_gate_failure, parse_cargo_json, parse_plain_stderr,
    render_failure_classification, structured_gate_failure, verdict_timed_out,
};
pub use composition::{FallbackGate, ParallelGate, VotingGate};
pub use diff_gate::{DiffAnalysis, DiffGate, DiffPayload, analyze_diff};
pub use env_builder::{GateEnv, GateEnvBuilder, build_for_rung};
pub use error::GateError;
pub use error_patterns::{
    FailurePatternRecord, error_key, extract_error_digest, records_from_classification,
    records_from_parsed_review_verdict,
};
pub use eval_generator::{
    EvalGenerationError, EvalGenerationRequest, EvalGenerator, EvalStrategy, EvalTemplate,
    Evaluation,
};
pub use fact_check::{FactCheckGate, SearchHit, SearchOracle};
pub use feedback::{FeedbackItem, GateFeedback, Severity, feedback_for_agent};
pub use forensic::{
    ArtifactMetadata, CausalChain, ForensicError, ForensicReplayBuilder, TurnRecord,
};
pub use gate_env::{inherit_gate_env, inherit_gate_env_from};
pub use gate_pipeline::{ComposedGatePipeline, GateComposition, GatePipeline};
pub use gate_service::GateService;
pub use generated::{GateGenerator, GeneratedCheck};
pub use hotelling::{HotellingDetector, JointAnomalyResult};
pub use payload::{BuildSystem, GatePayload, TestSelector};
pub use process_reward::{
    AggregateMethod, ProcessRewardModel, ReasoningStep, StepVerdict, TurnSnapshot,
};
pub use ratchet::GateRatchet;
pub use registry::{
    GateKind, GateRegistry, GateSpec, GateStatus, is_deterministic_gate, rung_for_gate_name,
};
pub use review_verdict::{
    ParsedReviewVerdict, ReviewParseSource, ReviewVerdict, ReviewVerdictContext,
    parse_structured_review_verdict,
};
pub use rung_dispatch::{
    GatePipelineBuilder, RungExecutionConfig, RungExecutionInputs, run_canonical_rung,
    run_diff_gate, run_rung,
};
pub use rung_selector::{PlanComplexity, Rung, RungCaps, is_selected, select_rungs};
pub use shell::ShellGate;
pub use spc::{
    BocpdDetector, ChangePoint, ControlStatus, CusumDetector, CusumShift, EwmaControlChart,
    SpcAlert, SpcDetector,
};
pub use test_gate::{TestGate, parse_test_counts};
pub use verdict_publisher::VerdictPublisher;

// ─── Production pipeline types (#250 / #275) ────────────────────────────────
pub use graph_cell::{GatePipelineCell, GatePipelineCellInput, GraphEventProgressSink};
pub use production_request::{GateTaskContextSpec, ProductionGateRequest, VerifyStepSpec};
pub use production_service::{
    DefaultGateService, GatePipelineProgress, NoopProgressSink, ProductionGateRunner,
    ProductionGateService, ProgressSink,
};
pub use production_verdict::{
    EvidenceRef, PipelineOutcome, ProductionGateRungVerdict, ProductionGateVerdictV1, RungState,
    VERDICT_SCHEMA_VERSION,
};
