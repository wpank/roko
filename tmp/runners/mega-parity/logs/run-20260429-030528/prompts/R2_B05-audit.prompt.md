# AUDIT: Batch R2_B05 — Wire selection into plan run (per-task) and legacy run

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_B05`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task

Wire selection into plan run (per-task) and legacy run

## Runner Context

You are working in runner `mega-parity`, batch R2_B05.
This batch is part of Runner 2: execution-contract — Make CLI execution contracts truthful enough
that demo scenarios and agent sessions can rely on them.

## Problem

### Problem 1: `plan run` per-task dispatch ignores `--model`

In `commands/plan.rs`, the `PlanCmd::Run` handler builds `RunConfig` at plan.rs:283-316.
The `model` field is set at plan.rs:286:
```rust
model: roko_config.agent.default_model.clone(),
```
This always uses the config default. **`cli.model` is never consulted.**

Inside `orchestrate.rs`, `dispatch_agent_with` at line 13945 receives `model_override:
Option<String>`. When the plan runner calls `dispatch_agent_with`, it passes `None` for
`model_override` when there is no per-task model, meaning the CLI flag is silently dropped.

### Problem 2: `roko run` legacy path (`run_once`) uses ad-hoc resolution

`run_once` at run.rs:902 receives `config: &Config` and passes it to `dispatch_agent` at
run.rs:1291. Inside `dispatch_agent`, each backend sub-path independently computes the model:
- routing path (run.rs:1308-1313)
- Claude CLI path (run.rs:1347-1355)
- Anthropic API path (run.rs:1606-1610)
- ollama path (run.rs:1476-1480)

None of these call `resolve_effective_model()`.

## Architecture Contract

- CLI `--model` beats task `model_hint` in all dispatch paths
- `resolve_effective_model()` from `crates/roko-cli/src/model_selection.rs` (created in B02)
- `dispatch_agent_with` receives the resolved model via its `model_override` parameter

## Files to Modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`

---

## Changes to `orchestrate.rs`

### Change 1: Pass cli.model through RunConfig

**Location**: `crates/roko-cli/src/orchestrate.rs`

First, find the `RunConfig` struct definition. Search for:
```
grep -n "pub struct RunConfig" crates/roko-cli/src/runner/types.rs
```

`RunConfig` contains a `model: String` field. Add a `cli_model_override: Option<String>` field:

**Before** (find the struct definition and add the field):
```rust
pub struct RunConfig {
    pub workdir: PathBuf,
    pub plan_dir: PathBuf,
    pub model: String,
    // ... existing fields ...
}
```

**After**:
```rust
pub struct RunConfig {
    pub workdir: PathBuf,
    pub plan_dir: PathBuf,
    pub model: String,
    /// Hard CLI override for model selection. Beats task model_hint.
    /// Set from `--model` flag. None means use normal routing.
    pub cli_model_override: Option<String>,
    // ... existing fields unchanged ...
}
```

**Update the `Default` impl** (if one exists) to add `cli_model_override: None`.

### Change 2: Propagate cli_model_override into RunConfig in commands/plan.rs

**Location**: `crates/roko-cli/src/commands/plan.rs`, `PlanCmd::Run` arm, at plan.rs:283-316
where `RunConfig` is constructed.

**Current code** at plan.rs:283-316 (the `run_config` construction):
```rust
            let run_config = roko_cli::runner::RunConfig {
                workdir: wd.clone(),
                plan_dir: plans_dir.clone(),
                model: roko_config.agent.default_model.clone(),
                timeout_secs: task_timeout_secs,
                // ... other fields ...
            };
```

**Add `cli_model_override`**:
```rust
            let run_config = roko_cli::runner::RunConfig {
                workdir: wd.clone(),
                plan_dir: plans_dir.clone(),
                model: roko_config.agent.default_model.clone(),
                cli_model_override: cli.model.clone(),
                timeout_secs: task_timeout_secs,
                // ... other fields unchanged ...
            };
```

Note: `cli` is the `&Cli` parameter available in `cmd_plan`. `cli.model` is `Option<String>`.

### Change 3: Use `resolve_effective_model()` in `dispatch_agent_with`

**Location**: `dispatch_agent_with` at orchestrate.rs:13945, model selection block at
orchestrate.rs:14020-14061.

The `self.config` is of type `RunConfig`. After Change 1, `self.config.cli_model_override` holds
the CLI flag value.

**Current code** at orchestrate.rs:14020-14061 (three branches — with prompt_override, with
task_def, fallback):
```rust
        let (task_text, mut selected_model) = if let Some(override_prompt) = prompt_override {
            let model = explicit_model_override.clone().unwrap_or_else(|| {
                self.config
                    .agent
                    .model
                    .clone()
                    .unwrap_or_else(|| "claude-sonnet-4-6".into())
            });
            (override_prompt, model)
        } else if let Some(ref td) = task_def {
            let prompt = td.build_prompt(plan_id, &self.workdir);
            let model = explicit_model_override.clone().unwrap_or_else(|| {
                td.effective_model(
                    self.config
                        .agent
                        .model
                        .as_deref()
                        .unwrap_or("claude-sonnet-4-6"),
                    Some(&self.config.agent.tier_models),
                )
            });
            // ... log ...
            (prompt, model)
        } else {
            let text = format!("Plan: {plan_id}\nTask: {task}\n\nImplement...");
            let model = self.config.agent.model.clone().unwrap_or_else(|| "claude-opus-4-6".into());
            (text, model)
        };
```

**Insert before the existing block** (before the `if let Some(override_prompt)` line at
orchestrate.rs:14020):
```rust
        // If the runner-level CLI override is set, it beats everything.
        let cli_runner_override = self.config.cli_model_override.clone();
```

**Then modify all three branches** to check `cli_runner_override` first:

For the `prompt_override` branch, change:
```rust
            let model = explicit_model_override.clone().unwrap_or_else(|| {
                self.config.agent.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".into())
            });
```
to:
```rust
            let model = cli_runner_override.clone()
                .or_else(|| explicit_model_override.clone())
                .unwrap_or_else(|| {
                    self.config.agent.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".into())
                });
```

For the `task_def` branch, change:
```rust
            let model = explicit_model_override.clone().unwrap_or_else(|| {
                td.effective_model(
                    self.config.agent.model.as_deref().unwrap_or("claude-sonnet-4-6"),
                    Some(&self.config.agent.tier_models),
                )
            });
```
to:
```rust
            let model = cli_runner_override.clone()
                .or_else(|| explicit_model_override.clone())
                .unwrap_or_else(|| {
                    td.effective_model(
                        self.config.agent.model.as_deref().unwrap_or("claude-sonnet-4-6"),
                        Some(&self.config.agent.tier_models),
                    )
                });
```

For the fallback branch:
```rust
            let model = cli_runner_override.clone()
                .or_else(|| self.config.agent.model.clone())
                .unwrap_or_else(|| "claude-opus-4-6".into());
```

---

## Changes to `run.rs`

### Change 4: Wire `resolve_effective_model()` into `run_once` (legacy path)

**Location**: `run_once` at run.rs:902. The function receives `config: &Config`.

**Current code** in `dispatch_agent` (called from `run_once` at run.rs:974) inside the `has_routing`
branch at run.rs:1305-1313:
```rust
    if has_routing {
        let tools_csv = claude_tool_allowlist(&config.prompt.role);
        let system_prompt = build_system_prompt(config, prompt_text, &tools_csv);
        let model = config
            .agent
            .model
            .clone()
            .unwrap_or_else(|| routing_config.agent.default_model.clone());
        let resolved = resolve_model(&routing_config, &model);
```

Replace the `model` computation at run.rs:1308-1313 with:
```rust
        let selection = crate::model_selection::resolve_effective_model(
            crate::model_selection::ModelResolutionInput {
                cli_model: config.agent.model.as_deref(),
                task_hint: None,
                role: Some(config.prompt.role.as_str()),
                config: &routing_config,
                cascade_router: None,
            },
        )
        .unwrap_or_else(|_| {
            // Fallback: use routing_config default to avoid crashing legacy path.
            crate::model_selection::EffectiveModelSelection {
                requested_model: config.agent.model.clone(),
                effective_model_key: routing_config.agent.default_model.clone(),
                provider_key: "unknown".to_string(),
                provider_kind: "unknown".to_string(),
                backend_slug: routing_config.agent.default_model.clone(),
                source: crate::model_selection::SelectionSource::ProjectDefault,
                reason: "fallback from legacy run_once path".to_string(),
            }
        });
        selection.print_stderr();
        let model = selection.effective_model_key.clone();
        let resolved = resolve_model(&routing_config, &model);
```

Similarly, update the Claude CLI path at run.rs:1347-1355:
```rust
        let model = config.agent.model.clone().unwrap_or_else(|| {
```
Replace with:
```rust
        let model = config.agent.model.clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| {
```
(This is a minor defensive fix; the real resolution happened in the `has_routing` branch above.
For the non-routing Claude CLI path, `config.agent.model` already holds the CLI value.)

---

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-cli
```

## Acceptance Criteria

- [ ] `roko plan run --model X` uses X for ALL tasks regardless of task `model_hint`
- [ ] Without `--model`, task `model_hint` is still respected (second priority)
- [ ] Legacy `roko run --model X` uses X via `resolve_effective_model()`
- [ ] `RunConfig.cli_model_override: Option<String>` field exists
- [ ] `cli.model` is passed to `RunConfig.cli_model_override` in `PlanCmd::Run`
- [ ] `cargo check -p roko-cli` passes

## Do NOT

- Remove `model_hint` support from tasks (it's second priority — preserved in all branches)
- Change dispatch broadly — only the model resolution computation changes
- Add new fields to tasks.toml schema
- Change `WorkflowEngine` internals

## Evidence

E2E-DOGFOOD-AUDIT Path 5, Path 7

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/orchestrate.rs` (22076 lines — truncated)

```rust
//! Plan-driven orchestration loop: reads plans → builds executor → dispatches
//! agents → runs gates → persists results → advances phases.
//!
//! This is the runtime harness that connects the CLI to the orchestrator's
//! pure state machine. The orchestrator's [`ParallelExecutor`] never does I/O
//! — it returns [`ExecutorAction`]s. This module dispatches those actions to
//! real agents, gates, and git, then feeds results back as events.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow};
use roko_agent::gemini::{Content, GeminiCacheClient, Part};
use roko_agent::mcp::{McpConfig, McpServerConfig};
use roko_agent::perplexity::PerplexitySearchClient;
use roko_agent::provider::is_known_protocol_command;
use roko_agent::safety::provenance::{Custody, CustodyLogger};
use roko_agent::safety::scrub::{ScrubPolicy, scrub_secrets};
use roko_agent::task_runner::{
    AnomalyDetector as RunnerAnomalyDetector, BudgetGuardrail as RunnerBudgetGuardrail,
    ConductorBandit as RunnerConductorBandit, CostTable as RunnerCostTable,
    EventBus as RunnerEventBus, ModelPricing as RunnerModelPricing, TaskRunner, TaskRunnerError,
};
use roko_agent::{
    Agent, AgentInvocationSession, AgentResult, InvocationState, MultiAgentPool, ReuseScope,
    SafetyLayer, WarmReusePolicy, fingerprint_text,
};
use roko_chain::alloy_impl::{AlloyChainClient, AlloyChainWallet};
use roko_chain::{ChainClient, ChainWallet};
use roko_compose::enrichment::{
    ALL_ORDERED, EnrichStep, EnrichmentConfig, EnrichmentPipeline,
    LlmBackend as EnrichmentLlmBackend, LlmClient as EnrichmentLlmClient, PlanInfo, SkipReason,
    StepOutcome, StepSelector, estimate_enrichment,
};
use roko_compose::{
    AttentionBidder, ContextProvider, PadState, Placement, PlanArtifacts, PromptComposer,
    PromptSection, SectionPriority, SectionScorer, TaskContext,
};
use roko_conductor::diagnosis::{
    DiagnosisEngine, DiagnosisResult, ErrorCategory, SuggestedIntervention,
};
use roko_conductor::health::{HealthMonitor, HealthStatus, SystemSnapshot};
use roko_conductor::stuck_detection::{ActivityEntry, MetaCognitionHook, StuckDetector, StuckKind};
use roko_conductor::{CircuitBreakerState, Conductor, ConductorDecision, FailureRecord};
use roko_core::DaimonPolicy;
use roko_core::React;
use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::attestation::{self, SigningKey};
use roko_core::foundation::ShellGateCommand as CoreShellGateCommand;
use roko_core::config::schema::{
    GatesConfig, LearningConfig as RuntimeLearningConfig, RokoConfig, RoleOverride,
};
use roko_core::extension::ExtensionChain;
use roko_core::metric::{ConfigHash, TaskMetric};
use roko_core::obs::health::{AlwaysUpProbe, ProbeRegistry};
use roko_core::obs::{LabelSet, MetricRegistry};
use roko_core::tool::TraceId;
use roko_core::tool::trace::{FailureKind, FailureTrace, TraceStep};
use roko_core::tool::{FormatBandit, ProfileBandit, ToolTraceEvent, TraceSink};
use roko_core::{
    AgentRole, Body, Budget, Compose, ContentHash, Context, Decay, Engram, Kind,
    OperatingFrequency, OperatingFrequencyScheduleContext, PhaseKind, Provenance, Store,
    TaskCategory, TaskComplexityBand, TaskDomain, TaskRequirements, ToolRegistry, Verdict, Verify,
    score_model_for_task,
};
use roko_core::{
    CFactorPolicy, CFactorSource, CFactorSummary, CatalystImpactSummary, CatalystScorer,
    CatalystSignalSource, DiagnosisSeverity, DiagnosisSummary, PredictionPolicy, PredictiveScorer,
};
use roko_daimon::{
    AffectEngine as _, AffectEvent, DaimonState, DispatchParams, SomaticSignal,
    StrategyCoordinates, TaskStrategyObservation,
};
use roko_dreams::{DreamAgentConfig, DreamLoopConfig, DreamRunner};
use roko_fs::FileSubstrate;
use roko_fs::RokoLayout;
use roko_fs::observability::FsObservabilitySinks;
use roko_gate::{
    AcceptanceDecision, AcceptanceEvidence, AcceptanceOutcome, ArtifactStore as GateArtifactStore,
    ClippyGate, CompileGate, FailureClass, GateEvidence, GateFailureAction, GateRatchet,
    ParityLedgerEvidenceRow, ParityLedgerStatus, ParsedReviewVerdict, RecoveryEvidence,
    ReviewVerdictContext, ReviewVerdictEvidence, SearchHit, SearchOracle, ShellGate,
    StructuredOutputEvidence, TestGate, VerdictPublisher,
    adaptive_threshold::AdaptiveThresholds,
    classify_gate_failure, feedback_for_agent,
    gate_pipeline::GatePipeline,
    generated_test_gate::{ArtifactStore as GeneratedArtifactStore, GeneratedTestGate},
    llm_judge_gate::{JudgeOracle, JudgePayload},
    parse_structured_review_verdict,
    payload::{BuildSystem, GatePayload},
    records_from_classification,
    rung_dispatch::{RungExecutionConfig, RungExecutionInputs, run_rung},
    rung_selector::{PlanComplexity, Rung, RungCaps, select_rungs},
    symbol_gate::{SymbolExpectation, SymbolKind, SymbolManifest, Visibility},
};
use roko_learn::anomaly::{Anomaly, AnomalyDetector};
use roko_learn::budget::{BudgetAction, BudgetGuardrail};
use roko_learn::cfactor::{CFactor, detect_cfactor_regression};
use roko_learn::conductor::{
    ConductorAction as RetryConductorAction, ConductorBandit,
    ConductorState as RetryConductorState, ErrorPattern as RetryErrorPattern, HintType,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::costs_log::CostsLog;
use roko_learn::curriculum::{CurriculumMode, CurriculumScheduler};
use roko_learn::efficiency::{
    AgentEfficiencyEvent, FleetCFactor, PromptSectionMeta, compute_fleet_cfactor,
};
use roko_learn::episode_logger::{Episode, EpisodeLogger, GateVerdict, Usage};
use roko_learn::error_pattern_store::{
    ErrorPatternStore, FailurePatternQuery, GateFailureObservation, GateFailureSource,
};
use roko_learn::events::{AgentEvent, EventBus as LearningEventBus};
use roko_learn::hdc_fingerprint::{encode as encode_hdc_fingerprint, fingerprint_episode};
use roko_learn::latency::LatencyRegistry;
use roko_learn::model_experiment::ModelExperimentStore;
use roko_learn::playbook::PlaybookStore;
use roko_learn::prediction::CalibrationTracker;
use roko_learn::prompt_experiment::DEFAULT_STATIC_OVERRIDES_PATH;
use roko_learn::routing_log::{
    RoutingDecisionLog, RoutingDecisionLogStore, RoutingDecisionMeta, RoutingLogger,
};
use roko_learn::runtime_feedback::{
    CompletedRunInput, LearningRuntime, LearningUpdate, read_efficiency_events,
    refresh_cfactor_snapshot,
};
use roko_learn::section_effect::SectionEffectivenessRegistry;
use roko_learn::skill_library::Skill;
use roko_learn::skill_library::{
    SkillExtractionRequest, SkillGateResult, SkillLibrary, SkillQuery,
};
use roko_neuro::tier_progression::{TierProgression, TierProgressionDecision};
use roko_neuro::{
    ContextAssembler, EmotionalProvenance, KnowledgeAdmissionOutcome, KnowledgeAdmissionStore,
    KnowledgeCandidateRecord, KnowledgeEntry, KnowledgeKind, KnowledgeScope, KnowledgeStore,
    KnowledgeTier, NeuroStore,
};
use roko_orchestrator::coordination::{Pheromone, PheromoneKind, PheromoneScope};
use roko_orchestrator::executor::recovery::{RecoveryEngine, WarningSeverity};
use roko_orchestrator::worktree::{
    WorktreeConfig, WorktreeHealth, WorktreeManager, format_branch_name,
};
use roko_orchestrator::{
    CURRENT_SCHEMA_VERSION, DagConfig, EventKind, EventLog, EventLogSnapshot, ExecutorAction,
    ExecutorEvent, ExecutorSnapshot, GateResult, ParallelExecutor,
    PersistedCircuitBreakerFailureRecord, PersistedCircuitBreakerState, PlanRevisionEvidence,
    PlanRevisionRequest, PlanState, PostMergeRunner, ReplanResult, ReplanStrategy, UnifiedTaskDag,
    discover_plans,
};
use roko_runtime::cancel::CancelToken;
use roko_runtime::event_bus::{
    Envelope as RuntimeEventEnvelope, EventBus as RuntimeEventBus, GateVerdictSummary,
    PlanRevisionReason, RokoEvent,
};
use roko_runtime::process::ProcessSupervisor;
use roko_std::StaticToolRegistry;
use roko_std::SumScorer;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken as TokioCancellationToken;
use tracing::{Instrument, info_span, instrument};

use crate::agent_config::{
    synthesize_claude_cli_config, synthesize_known_protocol_config, synthesize_subprocess_config,
};
use crate::agent_spawn::{SpawnAgentSpec, spawn_agent_with_layer};
use crate::chain_registry::{chain_aware_resolver, chain_handler_map};
use crate::config::Config;
use crate::dispatch_helpers::{
    TASK_FAILURE_OUTPUT_TAIL_LINES, apply_section_effectiveness_to_prompt_section,
    build_daimon_context_section, build_relevant_context_layer, build_system_prompt,
    build_system_prompt_with_context_validated, claude_task_tool_allowlist_with,
    claude_tool_allowlist, claude_tool_allowlist_with, code_context_for_task,
    default_task_category, effective_context_window_tokens, extract_task_symbols,
    file_contains_public_api, load_prior_task_outputs, save_task_output, tail_output_lines,
    task_def_to_dag_task, task_def_to_input, task_read_cli_args, truncate_doc_snippet,
    truncate_output, with_task_failure_context,
};
use crate::heartbeat::{
    HeartbeatClock, HeartbeatProbeKind, HeartbeatProbeResult, HeartbeatSnapshot,
    persist_heartbeat_snapshot,
};
use crate::knowledge_helpers::{
    apply_neuro_gate_hints, build_knowledge_routing_advice, build_strategy_fragment_context,
    build_success_knowledge_entry, knowledge_routing_boost, query_anti_knowledge_patterns,
    record_lifecycle_knowledge, render_neuro_chunk,
};
use crate::learning_helpers::{
    TurnLearningFeedback, apply_concluded_experiment_overrides, build_task_playbook,
    install_episode_distillation_hook, learned_error_signature, load_efficiency_cost_signals,
    load_efficiency_signals_sync, load_or_create_playbook_store, load_or_create_skill_library,
    load_recent_signals, playbook_query_context, publish_turn_learning_feedback,
// ... (21676 lines omitted) ...
                distinct_contexts: Vec::new(),

                deprecated: false,
                balance: 1.0,
                frozen: false,
                catalytic_score: 0,
            })
            .unwrap();

        let task: crate::task_parser::TaskDef = toml::from_str(
            r#"
id = "T1"
title = "Inject only matching model-specific strategy fragments"
files = ["crates/roko-cli/src/orchestrate.rs"]
"#,
        )
        .unwrap();

        let mismatch = build_strategy_fragment_context(
            &store,
            AgentRole::Implementer,
            Some(&task),
            "Inject only matching model-specific strategy fragments",
            "gpt-5.4",
        );
        assert!(mismatch.is_none());

        let matched = build_strategy_fragment_context(
            &store,
            AgentRole::Implementer,
            Some(&task),
            "Inject only matching model-specific strategy fragments",
            "claude-sonnet-4-5",
        )
        .expect("matched strategy fragment context");
        assert!(matched.contains("XML tool-call tags"));
    }

    #[test]
    fn cost_anomaly_downgrade() {
        let mut config = Config::default();
        config
            .agent
            .tier_models
            .insert("mechanical".to_string(), "claude-haiku-4-5".to_string());

        let mut detector = AnomalyDetector::new(1_700_000_000_000);
        for cost in [1.0, 1.2, 0.9, 1.1, 1.05, 0.95, 1.15, 1.0] {
            assert!(detect_cost_anomaly_override(&mut detector, cost, &config).is_none());
        }

        assert_eq!(
            detect_cost_anomaly_override(&mut detector, 10.0, &config),
            Some("claude-haiku-4-5".to_string())
        );
    }

    #[test]
    fn atomic_checkpoint_preserves_existing_snapshot_on_tmp_write_failure() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("executor.json");

        let original = ExecutorSnapshot::new(1);
        save_snapshot_atomic(&original, &path).unwrap();
        let original_json = std::fs::read_to_string(&path).unwrap();

        let tmp_path = path.with_extension("json.tmp");
        std::fs::create_dir(&tmp_path).unwrap();

        let replacement = ExecutorSnapshot::new(2);
        let err = save_snapshot_atomic(&replacement, &path).unwrap_err();
        assert!(err.to_string().contains("write snapshot tmp"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original_json);
    }

    #[test]
    fn prioritize_ready_tasks_prefers_high_arousal() {
        let ready = vec!["calm".to_string(), "urgent".to_string()];
        let result = prioritize_ready_tasks(ready, |task_id| match task_id {
            "calm" => -1.0,
            "urgent" => 1.0,
            _ => 0.0,
        });

        assert_eq!(result, vec!["urgent", "calm"]);
    }

    #[test]
    fn prioritize_ready_tasks_keeps_original_order_for_ties() {
        let ready = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = prioritize_ready_tasks(ready, |_| 0.0);

        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn regenerate_plan_preserves_completed_tasks_and_rewrites_dependencies() {
        let old_tasks: TasksFile = toml::from_str(
            r#"
[meta]
plan = "demo"
iteration = 2
total = 2
done = 1
status = "ready"
max_parallel = 2

[[task]]
id = "T1"
title = "Implement plan regeneration"
status = "done"
tier = "focused"
depends_on = []
files = ["src/a.rs"]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"

[[task]]
id = "T2"
title = "Wire dashboard"
status = "ready"
tier = "focused"
depends_on = ["T1"]
files = ["src/b.rs"]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
"#,
        )
        .unwrap();
        let regenerated_tasks: TasksFile = toml::from_str(
            r#"
[meta]
plan = "demo"
iteration = 1
total = 3
done = 0
status = "ready"
max_parallel = 2

[[task]]
id = "N1"
title = "Implement plan regen"
status = "ready"
tier = "focused"
depends_on = []
files = ["src/a.rs"]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"

[[task]]
id = "N2"
title = "Wire dashboard"
status = "ready"
tier = "focused"
depends_on = ["N1"]
files = ["src/b.rs"]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"

[[task]]
id = "N3"
title = "Add metrics"
status = "ready"
tier = "focused"
depends_on = ["N2"]
files = ["src/c.rs"]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
"#,
        )
        .unwrap();
        let completed = vec![old_tasks.tasks[0].clone()];

        let merged = merge_regenerated_plan("demo", &old_tasks, regenerated_tasks, &completed);

        assert_eq!(merged.meta.plan, "demo");
        assert_eq!(merged.meta.iteration, 3);
        assert_eq!(merged.meta.total, 3);
        assert_eq!(merged.meta.done, 1);
        assert_eq!(merged.meta.status, "ready");
        assert_eq!(merged.tasks.len(), 3);
        assert_eq!(merged.tasks[0].id, "T1");
        assert_eq!(merged.tasks[0].status, "done");
        assert_eq!(merged.tasks[1].id, "N2");
        assert_eq!(merged.tasks[1].depends_on, vec!["T1"]);
        assert_eq!(merged.tasks[2].id, "N3");
        assert_eq!(merged.tasks[2].depends_on, vec!["N2"]);
        assert!(merged.tasks.iter().all(|task| task.id != "N1"));
    }
}
```

### `crates/roko-cli/src/run.rs` (3423 lines — truncated)

```rust
//! The universal loop: prompt → compose → agent → gate → persist → policy.
//!
//! This is the body of `roko run <prompt>`. It reads [`Config`], opens a
//! [`FileSubstrate`] under `.roko/`, seeds prompt sections, composes them
//! into a single Prompt signal, invokes the configured agent backend, runs
//! each configured gate on the working directory, and emits an Episode.

use crate::agent_config::{
    synthesize_claude_cli_config, synthesize_known_protocol_config, synthesize_subprocess_config,
};
use crate::agent_spawn::{SpawnAgentSpec, spawn_agent_scoped};
use crate::clean;
use crate::config::{Config, GateConfig, PromptFile};
use crate::knowledge_helpers::{build_strategy_fragment_context, query_anti_knowledge_patterns};
use crate::learning_helpers::{
    load_or_create_playbook_store, load_or_create_skill_library, playbook_query_context,
    render_prior_experience,
};
use crate::episode::EpisodePolicy;
use crate::model_selection::{EffectiveModelSelection, resolve_effective_model};
use crate::output_format;
#[cfg(feature = "legacy-orchestrate")]
use crate::prompting::{PromptBuildOptions, build_role_system_prompt_validated};
use crate::task_helpers::extract_task_symbols;
use crate::state_hub::{StateHub, StateHubSender};
use anyhow::{Context as _, Result, anyhow};
use chrono::Utc;
use roko_agent::provider::is_known_protocol_command;
use roko_agent::translate::{ClaudeTranslator, OllamaTranslator, RenderedTools, Translator};
use roko_agent::{AgentResult, OllamaLlmBackend};
#[cfg(feature = "legacy-orchestrate")]
use roko_compose::{Placement, PromptComposer, PromptSection, SectionPriority, TaskContext};
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::foundation::{
    EventConsumer as WorkflowEventConsumer, ShellGateCommand as CoreShellGateCommand,
};
use roko_core::metric::{ConfigHash, TaskMetric};
use roko_core::tool::ExternalAction;
use roko_core::tool::ToolRegistry;
use roko_core::{
    AgentRole, Body, Budget, Compose, Context, Engram, Kind, Provenance, Store, TaskCategory,
    Verdict, Verify,
};
use roko_fs::FileSubstrate;
use roko_gate::{BuildSystem, ClippyGate, CompileGate, GatePayload, ShellGate, TestGate};
use roko_learn::episode_logger::{Episode, EpisodeLogger, GateVerdict, Usage as EpisodeUsage};
use roko_learn::playbook::Playbook;
use roko_learn::runtime_feedback::{CompletedRunInput, LearningRuntime};
use roko_learn::skill_library::{Skill, SkillQuery};
use roko_orchestrator::{ServiceConfig, ServiceFactory};
use roko_runtime::effect_driver::EffectServices;
use roko_runtime::pipeline_state::WorkflowConfig;
use roko_runtime::workflow_engine::{WorkflowEngine, WorkflowRunConfig, WorkflowRunReport};
use roko_serve::bench::BenchStrategy;
use roko_std::NoOpScorer;
use roko_std::StaticToolRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Summary of a single `run` invocation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunReport {
    /// Content hash of the episode signal emitted at the end.
    pub episode_id: String,
    /// Content hash of the assembled prompt signal.
    pub prompt_id: String,
    /// Content hash of the agent's output signal.
    pub agent_output_id: String,
    /// Whether the agent invocation succeeded (exit code 0, no timeout).
    pub agent_success: bool,
    /// Per-gate verdicts in declaration order: (gate name, passed).
    pub gate_verdicts: Vec<(String, bool)>,
    /// How many signals are now in the substrate.
    pub total_signals: usize,
    /// Final agent output text, if it was a text payload.
    pub output_text: Option<String>,
    /// Token usage reported by the agent dispatch, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<RunUsage>,
}

/// Token usage captured from a single run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunUsage {
    /// Input (prompt) tokens consumed.
    pub input_tokens: u64,
    /// Output (completion) tokens produced.
    pub output_tokens: u64,
}

impl RunReport {
    /// True if the agent succeeded and every configured gate passed.
    #[must_use]
    pub fn overall_success(&self) -> bool {
        self.agent_success && self.gate_verdicts.iter().all(|(_, ok)| *ok)
    }

    /// Return the first gate that failed, if any.
    #[must_use]
    pub(crate) fn first_failed_gate(&self) -> Option<&str> {
        self.gate_verdicts
            .iter()
            .find_map(|(gate, passed)| (!*passed).then_some(gate.as_str()))
    }
}

struct StrategyPromptAugmentation {
    system_prompt: String,
    injected_playbook_ids: Vec<String>,
}

struct ContextEnrichmentOverlay {
    text: String,
    injected_playbook_ids: Vec<String>,
}

struct PlaybookSection {
    text: String,
    injected_playbook_ids: Vec<String>,
}

struct DispatchOutcome {
    agent_result: AgentResult,
    external_actions: Vec<ExternalAction>,
    injected_playbook_ids: Vec<String>,
}

/// Write a RunReport to `.roko/shared/{token}.json` and return the token.
#[cfg(feature = "legacy-orchestrate")]
pub fn write_shared_run(workdir: &std::path::Path, report: &RunReport) -> anyhow::Result<String> {
    let token = roko_core::generate_share_token();
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        id: token.clone(),
        agent: "unknown".to_string(),
        role: "unknown".to_string(),
        prompt: report.prompt_id.clone(),
        success: report.overall_success(),
        gates: report.gate_verdicts.clone(),
        output: report.output_text.clone(),
        cost_usd: None,
        input_tokens: report.usage.map(|usage| usage.input_tokens),
        output_tokens: report.usage.map(|usage| usage.output_tokens),
        model: None,
        duration_s: None,
        episode_id: Some(report.episode_id.clone()),
        transcript: Vec::new(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    write_shared_transcript(workdir, &transcript)
}

pub fn write_shared_workflow_run(
    workdir: &std::path::Path,
    prompt: &str,
    agent: &str,
    role: &str,
    report: &WorkflowRunReport,
) -> anyhow::Result<String> {
    let token = roko_core::generate_share_token();
    let (report_agent, report_role) = workflow_report_agent_role(report);
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        id: token.clone(),
        agent: non_empty(agent)
            .map(ToOwned::to_owned)
            .or(report_agent)
            .unwrap_or_else(|| "workflow".to_string()),
        role: non_empty(role)
            .map(ToOwned::to_owned)
            .or(report_role)
            .unwrap_or_else(|| "workflow".to_string()),
        prompt: prompt.to_string(),
        success: report.success,
        gates: report
            .gates
            .iter()
            .map(|gate| (gate.name.clone(), gate.passed))
            .collect(),
        output: non_empty(&report.output).map(ToOwned::to_owned),
        cost_usd: report.cost,
        // GAP: WorkflowRunReport exposes only a combined `token_usage: u64` total; the
        // workflow engine does not track input vs. output token counts separately. To
        // populate these fields the engine would need to accumulate per-turn TokenUsage
        // breakdowns and surface them on WorkflowRunReport.
        input_tokens: None,
        output_tokens: None,
        model: non_empty(&report.model).map(ToOwned::to_owned),
        duration_s: Some(report.duration_secs),
        episode_id: Some(report.run_id.clone()),
        transcript: report.events.clone(),
        timestamp: report
            .events
            .first()
            .map(|event| event.ts.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    write_shared_transcript(workdir, &transcript)
}
// ... (3023 lines omitted) ...
    #[test]
    fn dashboard_agent_model_is_never_empty_for_run_events() {
        let mut cfg = Config::default();
        cfg.agent.command = "codex".to_string();
        cfg.agent.model = None;

        assert!(!dashboard_agent_model(&cfg).trim().is_empty());

        cfg.agent.model = Some("gpt-5.4".to_string());
        assert_eq!(dashboard_agent_model(&cfg), "gpt-5.4");
    }

    #[test]
    fn engine_flag_express_selects_express_config() {
        let workflow = match "express" {
            "express" => WorkflowConfig::express(),
            "full" => WorkflowConfig::full(),
            _ => WorkflowConfig::standard(),
        };

        assert!(!workflow.has_strategy);
        assert!(!workflow.has_review);
        assert_eq!(workflow.max_iterations, 1);
    }

    #[test]
    fn engine_flag_full_selects_full_config() {
        let workflow = match "full" {
            "express" => WorkflowConfig::express(),
            "full" => WorkflowConfig::full(),
            _ => WorkflowConfig::standard(),
        };

        assert!(workflow.has_strategy);
        assert!(workflow.has_review);
        assert_eq!(workflow.max_iterations, 3);
    }

    #[test]
    fn engine_flag_legacy_and_unknown_select_standard_config() {
        for workflow_template in ["legacy", "v2", "standard", "unknown"] {
            let workflow = match workflow_template {
                "express" => WorkflowConfig::express(),
                "full" => WorkflowConfig::full(),
                _ => WorkflowConfig::standard(),
            };

            assert!(
                !workflow.has_strategy,
                "{workflow_template} should not enable strategy"
            );
            assert!(
                workflow.has_review,
                "{workflow_template} should enable review"
            );
            assert_eq!(workflow.max_iterations, 2);
        }
    }

    #[cfg(feature = "legacy-orchestrate")]
    #[tokio::test]
    async fn minimal_strategy_leaves_system_prompt_unmodified() {
        let tempdir = TempDir::new().expect("tempdir");
        let mut config = Config::default();
        config.prompt.role = "implementer".to_string();

        let base_prompt = build_system_prompt(&config, "Implement the feature.", "Read,Edit");
        let augmented = augment_system_prompt_for_strategy(
            base_prompt.clone(),
            tempdir.path(),
            &config.prompt.role,
            "Implement the feature.",
            "mock-model",
            Some(BenchStrategy::Minimal),
        )
        .await;

        assert_eq!(augmented.system_prompt, base_prompt);
        assert!(augmented.injected_playbook_ids.is_empty());
        assert!(skip_bench_enrichment(Some(BenchStrategy::Minimal)));
        assert!(!skip_bench_enrichment(None));
        assert!(!skip_bench_enrichment(Some(BenchStrategy::ContextEnriched)));
    }

    #[cfg(feature = "legacy-orchestrate")]
    #[tokio::test]
    async fn record_injected_playbook_outcomes_updates_bench_playbooks_only() {
        let tempdir = TempDir::new().expect("tempdir");
        let playbook_root = tempdir.path().join(".roko").join("learn").join("playbooks");
        let store = load_or_create_playbook_store(&playbook_root)
            .await
            .expect("playbook store");
        let playbook = Playbook::new("pb-1", "Audit dependencies");
        store.save(&playbook).await.expect("save playbook");
        let injected_ids = vec!["pb-1".to_string()];

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::ContextEnriched),
            &injected_ids,
            true,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 0);

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::NeuroAugmented),
            &injected_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::Minimal),
            &injected_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        record_injected_playbook_outcomes(tempdir.path(), None, &injected_ids, false).await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);

        let empty_ids: Vec<String> = Vec::new();
        record_injected_playbook_outcomes(
            tempdir.path(),
            Some(BenchStrategy::FullCascade),
            &empty_ids,
            false,
        )
        .await;
        let loaded = store.load("pb-1").await.expect("load").expect("playbook");
        assert_eq!(loaded.success_count, 1);
        assert_eq!(loaded.failure_count, 1);
    }

    #[tokio::test]
    async fn dispatch_agent_uses_exec_agent_for_plain_commands_without_routing() {
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("skipping: ANTHROPIC_API_KEY not set");
            return;
        }
        let tempdir = TempDir::new().expect("tempdir");
        let config = Config::default();
        let prompt = Engram::builder(Kind::Prompt)
            .body(Body::text("plain-exec-ok"))
            .build();

        let result = dispatch_agent(
            tempdir.path(),
            &config,
            &prompt,
            "plain-exec-ok",
            &Context::now(),
            None,
        )
        .await
        .expect("dispatch succeeds");

        assert!(result.agent_result.success);
        assert_eq!(
            result.agent_result.output.body.as_text().unwrap_or(""),
            "plain-exec-ok"
        );
        assert!(result.external_actions.is_empty());
        assert!(result.injected_playbook_ids.is_empty());
    }

    fn init_git_workdir(workdir: &std::path::Path) {
        run_git(workdir, &["init"]);
        run_git(workdir, &["config", "user.email", "test@example.com"]);
        run_git(workdir, &["config", "user.name", "Roko Test"]);
    }

    fn run_git(workdir: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .expect("run git command");

        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
```

---

## Read-Only Context (do not modify)

### `crates/roko-cli/src/model_selection.rs`

```rust
use std::collections::HashMap;
use std::fmt;

use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::config::schema::{ProviderConfig, RokoConfig};
use roko_learn::cascade_router::CascadeRouter;
use thiserror::Error;

use crate::config_helpers::find_role_override;

/// Provenance for the selected model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    /// Explicit `--model` CLI override.
    CliOverride,
    /// Model hint from the task definition.
    TaskModel,
    /// Model override from the role configuration.
    RoleConfig,
    /// Model selected by the cascade router.
    CascadeRouter,
    /// Workspace/project default model.
    ProjectDefault,
    /// Built-in fallback model.
    BuiltInDefault,
}

impl SelectionSource {
    /// Stable human-readable label used in errors and reasons.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CliOverride => "cli override",
            Self::TaskModel => "task model",
            Self::RoleConfig => "role config",
            Self::CascadeRouter => "cascade router",
            Self::ProjectDefault => "project default",
            Self::BuiltInDefault => "built-in default",
        }
    }
}

impl fmt::Display for SelectionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Fully resolved model/provider selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveModelSelection {
    /// The originally requested model string, if one was selected.
    pub requested_model: Option<String>,
    /// The final model key used for resolution.
    pub effective_model_key: String,
    /// Provider registry key.
    pub provider_key: String,
    /// Provider family label.
    pub provider_kind: String,
    /// Concrete backend slug sent to the provider.
    pub backend_slug: String,
    /// Which precedence step produced this selection.
    pub source: SelectionSource,
    /// Human-readable explanation of why this selection won.
    pub reason: String,
}

/// Errors returned by [`resolve_effective_model`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum Error {
    /// The caller provided an empty model string for a required input.
    #[error("{selection_source} received an empty model value")]
    EmptyModel {
        selection_source: SelectionSource,
    },
    /// The selected model points at a provider key that is not configured.
    #[error("{selection_source} selected model '{model}', but provider '{provider_key}' is not configured")]
    MissingProvider {
        /// Which precedence step selected the model.
        selection_source: SelectionSource,
        /// Model that won precedence.
        model: String,
        /// Provider key referenced by the selected model.
        provider_key: String,
    },
    /// The selected model could not be backed by any configured provider.
    #[error("{selection_source} selected unknown model '{model}', and no configured provider matches kind '{provider_kind}'")]
    UnknownModel {
        /// Which precedence step selected the model.
        selection_source: SelectionSource,
        /// Model that won precedence.
        model: String,
        /// Provider kind inferred from the selected model.
        provider_kind: String,
    },
}

#[derive(Debug, Clone)]
struct ModelCandidate {
    source: SelectionSource,
    model: String,
}

/// Resolve the effective model/provider pair using the shared precedence chain.
pub fn resolve_effective_model(
    cli_model: Option<String>,
    task_hint: Option<String>,
    role: Option<String>,
    cascade_router: Option<&CascadeRouter>,
    config: &RokoConfig,
) -> Result<EffectiveModelSelection, Error> {
    let candidate = select_candidate(cli_model, task_hint, role, cascade_router, config)?;
    let source = candidate.source;
    let requested_model = candidate.model;
    let resolved = resolve_model(config, &requested_model);
    let providers = config.effective_providers();
    let (provider_key, provider) = select_provider(source, &requested_model, &resolved, &providers)?;

    let effective_model_key = resolved.model_key;
    let backend_slug = resolved.slug;
    let provider_kind = provider.kind.label().to_string();
    let reason = build_reason(
        source,
        &requested_model,
        &effective_model_key,
        &provider_key,
        &provider_kind,
        &backend_slug,
    );

    Ok(EffectiveModelSelection {
        requested_model: Some(requested_model),
        effective_model_key,
        provider_key,
        provider_kind,
        backend_slug,
        source,
        reason,
    })
}

fn select_candidate(
    cli_model: Option<String>,
    task_hint: Option<String>,
    role: Option<String>,
    cascade_router: Option<&CascadeRouter>,
    config: &RokoConfig,
) -> Result<ModelCandidate, Error> {
        if let Some(model) = required_model(cli_model, SelectionSource::CliOverride)? {
            return Ok(ModelCandidate {
                source: SelectionSource::CliOverride,
                model,
        });
    }

        if let Some(model) = required_model(task_hint, SelectionSource::TaskModel)? {
            return Ok(ModelCandidate {
                source: SelectionSource::TaskModel,
                model,
        });
    }

    if let Some(role_label) = normalized_label(role) {
        if let Some(override_cfg) = find_role_override(config, &role_label) {
            if let Some(model) = override_cfg
                .model
                .as_deref()
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(str::to_owned)
            {
                return Ok(ModelCandidate {
                    source: SelectionSource::RoleConfig,
                    model,
                });
            }
        }
    }

        if let Some(router) = cascade_router {
            // This selector has no richer feature context, so we ask the cascade
            // for its deterministic raw-context choice.
            let model = router.select(Vec::new()).model.slug;
            let model = model.trim();
            if model.is_empty() {
                return Err(Error::EmptyModel {
                    selection_source: SelectionSource::CascadeRouter,
                });
            }
            return Ok(ModelCandidate {
                source: SelectionSource::CascadeRouter,
            model: model.to_string(),
        });
    }

    let default_model = config.agent.default_model.trim();
    if !default_model.is_empty() {
        return Ok(ModelCandidate {
            source: SelectionSource::ProjectDefault,
            model: default_model.to_string(),
        });
    }

    Ok(ModelCandidate {
        source: SelectionSource::BuiltInDefault,
        model: builtin_default_model(),
    })
}

fn required_model(
    input: Option<String>,
    source: SelectionSource,
) -> Result<Option<String>, Error> {
    match input {
        Some(model) => {
            let model = model.trim();
            if model.is_empty() {
                Err(Error::EmptyModel {
                    selection_source: source,
                })
            } else {
                Ok(Some(model.to_string()))
            }
        }
        None => Ok(None),
    }
}

fn normalized_label(input: Option<String>) -> Option<String> {
    input
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_owned)
}

fn builtin_default_model() -> String {
    RokoConfig::default().agent.default_model
}

fn select_provider<'a>(
    source: SelectionSource,
    model: &str,
    resolved: &roko_core::agent::ResolvedModel,
    providers: &'a HashMap<String, ProviderConfig>,
) -> Result<(String, &'a ProviderConfig), Error> {
    if let Some(profile) = resolved.profile.as_ref() {
        let provider_key = profile.provider.trim();
        if provider_key.is_empty() {
            return Err(Error::MissingProvider {
                selection_source: source,
                model: model.to_string(),
                provider_key: profile.provider.clone(),
            });
        }

        let provider = providers.get(provider_key).ok_or_else(|| Error::MissingProvider {
            selection_source: source,
            model: model.to_string(),
            provider_key: provider_key.to_string(),
        })?;

        return Ok((provider_key.to_string(), provider));
    }

    let Some((provider_key, provider)) = provider_for_kind(providers, resolved.provider_kind)
    else {
        return Err(Error::UnknownModel {
            selection_source: source,
            model: model.to_string(),
            provider_kind: resolved.provider_kind.label().to_string(),
        });
    };

    Ok((provider_key, provider))
}

fn provider_for_kind<'a>(
    providers: &'a HashMap<String, ProviderConfig>,
    kind: ProviderKind,
) -> Option<(String, &'a ProviderConfig)> {
    let exact_key = kind.label();
    if let Some(provider) = providers.get(exact_key) {
        if provider.kind == kind {
            return Some((exact_key.to_string(), provider));
        }
    }

    let mut matches = providers
        .iter()
        .filter_map(|(key, provider)| (provider.kind == kind).then_some((key.as_str(), provider)))
        .collect::<Vec<_>>();
    matches.sort_unstable_by(|a, b| a.0.cmp(b.0));
    matches.first().map(|&(key, provider)| (key.to_string(), provider))
}

fn build_reason(
    source: SelectionSource,
    requested_model: &str,
    effective_model_key: &str,
    provider_key: &str,
    provider_kind: &str,
    backend_slug: &str,
) -> String {
    format!(
        "{source} selected `{requested_model}` as `{effective_model_key}` -> `{backend_slug}` via provider `{provider_key}` ({provider_kind})"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use roko_core::config::schema::{ModelProfile, RoleOverride, RokoConfig};
    use roko_learn::cascade_router::CascadeRouter;

    fn role_model(model: &str) -> RoleOverride {
        RoleOverride {
            model: Some(model.to_string()),
            ..Default::default()
        }
    }

    fn explicit_profile(provider: &str, slug: &str) -> ModelProfile {
        ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            ..Default::default()
        }
    }

    fn cascade_router(model: &str) -> CascadeRouter {
        CascadeRouter::new(vec![model.to_string()])
    }

    #[test]
    fn cli_override_wins_over_everything() {
        let mut config = RokoConfig::default();
        config.agent.default_model = "claude-opus-4-6".to_string();
        config.agent.roles.insert("implementer".to_string(), role_model("claude-haiku-4-5"));
        let router = cascade_router("claude-sonnet-4-6");

        let selection = resolve_effective_model(
            Some("claude-haiku-4-5".to_string()),
            Some("claude-sonnet-4-6".to_string()),
            Some("implementer".to_string()),
            Some(&router),
            &config,
        )
        .expect("selection");

        assert_eq!(selection.source, SelectionSource::CliOverride);
        assert_eq!(selection.requested_model.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!(selection.effective_model_key, "claude-haiku-4-5");
        assert_eq!(selection.backend_slug, "claude-haiku-4-5");
        assert_eq!(selection.provider_key, "claude_cli");
        assert_eq!(selection.provider_kind, "claude_cli");
        assert!(selection.reason.contains("cli override"));
    }

    #[test]
    fn task_hint_wins_when_no_cli_override() {
        let mut config = RokoConfig::default();
        config.agent.roles.insert("implementer".to_string(), role_model("claude-opus-4-6"));
        let router = cascade_router("claude-sonnet-4-6");

        let selection = resolve_effective_model(
            None,
            Some("claude-haiku-4-5".to_string()),
            Some("implementer".to_string()),
            Some(&router),
            &config,
        )
        .expect("selection");

        assert_eq!(selection.source, SelectionSource::TaskModel);
        assert_eq!(selection.requested_model.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!(selection.effective_model_key, "claude-haiku-4-5");
        assert_eq!(selection.provider_key, "claude_cli");
        assert!(selection.reason.contains("task model"));
    }

    #[test]
    fn role_default_used_as_fallback() {
        let mut config = RokoConfig::default();
        config.agent.roles.insert("architect".to_string(), role_model("claude-opus-4-6"));

        let selection = resolve_effective_model(
            None,
            None,
            Some("architect".to_string()),
            None,
            &config,
        )
        .expect("selection");

        assert_eq!(selection.source, SelectionSource::RoleConfig);
        assert_eq!(selection.requested_model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(selection.effective_model_key, "claude-opus-4-6");
        assert_eq!(selection.provider_key, "claude_cli");
        assert!(selection.reason.contains("role config"));
    }

    #[test]
    fn cascade_router_is_consulted_when_no_explicit_selection_exists() {
        let config = RokoConfig::default();
        let router = cascade_router("claude-haiku-4-5");

        let selection = resolve_effective_model(None, None, None, Some(&router), &config)
            .expect("selection");

        assert_eq!(selection.source, SelectionSource::CascadeRouter);
        assert_eq!(selection.requested_model.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!(selection.effective_model_key, "claude-haiku-4-5");
        assert_eq!(selection.provider_key, "claude_cli");
        assert!(selection.reason.contains("cascade router"));
    }

    #[test]
    fn config_default_is_used_when_cascade_is_absent() {
        let mut config = RokoConfig::default();
        config.agent.default_model = "claude-opus-4-6".to_string();

        let selection = resolve_effective_model(None, None, None, None, &config).expect("selection");

        assert_eq!(selection.source, SelectionSource::ProjectDefault);
        assert_eq!(selection.requested_model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(selection.effective_model_key, "claude-opus-4-6");
        assert_eq!(selection.provider_key, "claude_cli");
        assert!(selection.reason.contains("project default"));
    }

    #[test]
    fn builtin_fallback_is_used_when_config_has_no_default() {
        let mut config = RokoConfig::default();
        config.agent.default_model.clear();
        let builtin_default = RokoConfig::default().agent.default_model;

        let selection = resolve_effective_model(None, None, None, None, &config).expect("selection");

        assert_eq!(selection.source, SelectionSource::BuiltInDefault);
        assert_eq!(selection.requested_model.as_deref(), Some(builtin_default.as_str()));
        assert_eq!(selection.effective_model_key, builtin_default);
        assert_eq!(selection.provider_key, "claude_cli");
        assert!(selection.reason.contains("built-in default"));
    }

    #[test]
    fn cli_override_with_unavailable_provider_returns_error() {
        let mut config = RokoConfig::default();
        config
            .models
            .insert("custom".to_string(), explicit_profile("openai", "gpt-4o"));

        let err = resolve_effective_model(Some("custom".to_string()), None, None, None, &config)
            .expect_err("selection should fail");

        assert!(err.to_string().contains("provider 'openai'"));
    }

    #[test]
    fn unknown_model_slug_returns_error() {
        let config = RokoConfig::default();

        let err = resolve_effective_model(
            Some("definitely-not-a-model".to_string()),
            None,
            None,
            None,
            &config,
        )
        .expect_err("selection should fail");

        assert!(err.to_string().contains("no configured provider matches kind"));
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
