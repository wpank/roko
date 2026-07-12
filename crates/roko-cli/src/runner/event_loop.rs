//! Core event loop — drives the executor via `tokio::select!` over agent
//! events, gate completions, executor ticks, periodic flushes, and
//! cancellation.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::state_hub::StateHub;
use anyhow::{Context, Result};
use roko_core::RuntimeEvent;
use roko_core::agent::ModelSpec;
use roko_core::config::GatesConfig;
use roko_core::defaults::DEFAULT_AGENT_TURN_LIMIT;
// TimeoutConfig-derived helpers: agent_dispatch_timeout, plan_total_timeout,
// llm_call_timeout, gate_timeout — see below.
use roko_core::runtime_event::WorkflowOutcome as RuntimeWorkflowOutcome;
use roko_core::{AgentRole, ContentHash, PhaseKind, PlanPhase};
use roko_daimon::{
    AffectEngine as _, AffectEvent, DispatchParams, SomaticSignal, StrategyCoordinates,
    TaskStrategyObservation,
};
use roko_fs::RokoLayout;
use roko_gate::{PlanComplexity, classify_gate_failure, render_failure_classification};
use roko_orchestrator::{
    ExecutorAction, ExecutorConfig, ExecutorEvent, ExecutorSnapshot, GateResult, MergeQueue,
    MergeRequest, OrchestratorSnapshot, ParallelExecutor, PlanRevisionEvidence,
    PlanRevisionRequest, PlanState as OrcPlanState, RecoveryEngine, ReplanStrategy,
    TransitionError, WorktreeConfig, WorktreeManager, format_branch_name,
};
use roko_runtime::event_bus::PlanRevisionReason;
use roko_runtime::{CommitOutcome, HttpEventSink, RunLedger, TaskTerminalOutcome, WorkflowConfig};
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::dispatch::model_routing::tier_to_complexity;
use crate::dispatch::{
    AgentDispatchRequest, DispatchContext, GateFeedback as DispatchGateFeedback, PromptCache,
    PromptDiagnostics, ResolvedAgentRuntime, SharedAgentFactory,
};
use crate::inline::DiffEntry;
use crate::knowledge_helpers::{build_knowledge_routing_advice, neuro_prompt_task_category};
use crate::task_helpers::task_target_crates;
use crate::task_parser::TaskDef;
use roko_learn::playbook::PlaybookStore;
use roko_learn::post_gate_reflection::{PostGateReflectionStore, ReflectionGateOutcome};
use roko_learn::section_outcome::{
    SECTION_OUTCOME_SCHEMA_VERSION, SectionOutcomeRecord, SectionOutcomeStatus, SectionOutcomeStore,
};
use roko_neuro::KnowledgeStore;

use super::agent_events::handle_agent_event;
use super::agent_stream::{AgentHandle, AgentSpawnConfig, AgentTermination, AgentWait};
use super::attempt_ownership::{
    AttemptClaim, AttemptOwner, AttemptOwnership, AttemptPhase, EffectRef,
};
use super::gate_dispatch;
use super::merge::{MergeDispatch, MergeLaunch, PlanMerger, PlanMergerConfig};
use super::output_sink::RunOutputSink;
use super::persist::{self, GateThresholds, PersistPaths};
use super::plan_loader::Plan;
use super::snapshot_writer::{SnapshotPayload, SnapshotWriter};
use super::state::RunState;
use super::task_dag::{DagConfig, DagProgressSummary, TaskDag, task_status_is_terminal};
use super::tui_bridge::TuiBridge;
use super::types::{
    AgentCompletionSummary, AgentDispatchOutcome, AgentEvent, GateCompletion, GateCompletionKind,
    GateEffectRef, GateVerdictSummary, PlanOutcome, PlanRunSummary, PromptAssemblyDiagnostics,
    ResumeMarker, ResumeOutcome, RetryDecision, RunConfig, RunOutcome, RunTotals, RunnerEvent,
    RunnerFailureKind, TaskAttemptOutcome, TaskAttemptRef, TaskAttemptStatus,
    effective_plan_timeout_secs,
};

// ─── RunReport ──────────────────────────────────────────────────────────

/// Summary of a completed run.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub plans: Vec<PlanReport>,
    pub total_tasks: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_cost_usd: f64,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_agent_calls: usize,
    pub duration: Duration,
    /// Per-task failure reasons keyed by "plan_id:task_id".
    pub failure_reasons: HashMap<String, String>,
    /// Per-task cost breakdown.
    pub task_costs: Vec<TaskCostReport>,
}

/// Per-task cost report for the RunLedger.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskCostReport {
    pub plan_id: String,
    pub task_id: String,
    pub model: String,
    pub provider: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub agent_calls: u32,
    pub outcome: String,
}

/// Per-plan report.
#[derive(Debug, Clone)]
pub struct PlanReport {
    pub plan_id: String,
    pub completed: bool,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub gate_results: Vec<GateResult>,
}

impl RunReport {
    pub fn all_succeeded(&self) -> bool {
        self.tasks_failed == 0 && self.plans.iter().all(|p| p.completed)
    }
}

fn duration_secs(duration: Duration) -> u64 {
    duration.as_secs().max(1)
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

/// Resolve agent dispatch wall-clock timeout from `TimeoutConfig` (preferred)
/// or the legacy `RunConfig.timeout_secs` scalar.
pub(crate) fn agent_dispatch_timeout(config: &RunConfig) -> Duration {
    config.roko_config.as_deref().map_or_else(
        || Duration::from_secs(config.timeout_secs),
        |cfg| cfg.timeouts.agent_dispatch(),
    )
}

/// Resolve plan-level wall-clock timeout from `TimeoutConfig` (preferred)
/// or the legacy `RunConfig.plan_timeout_secs` scalar.
pub(crate) fn plan_total_timeout(config: &RunConfig) -> Duration {
    config.roko_config.as_deref().map_or_else(
        || Duration::from_secs(config.plan_timeout_secs),
        |cfg| Duration::from_secs(effective_plan_timeout_secs(cfg)),
    )
}

/// Resolve LLM call timeout from `TimeoutConfig`.
pub(crate) fn llm_call_timeout(config: &RunConfig) -> Duration {
    config.roko_config.as_deref().map_or_else(
        || roko_core::config::TimeoutConfig::default().llm_call(),
        |cfg| cfg.timeouts.llm_call(),
    )
}

/// Resolve gate rung timeout from `TimeoutConfig`:
/// - Compile rung -> `gate_compile()`
/// - Lint rung -> `gate_clippy()`
/// - all other rungs -> `gate_test()`
pub(crate) fn gate_timeout(config: &RunConfig, rung: u32) -> Duration {
    use roko_gate::rung_selector::Rung;
    config.roko_config.as_deref().map_or_else(
        || Duration::from_secs(config.timeout_secs),
        |cfg| {
            if rung == Rung::Compile.as_index() {
                cfg.timeouts.gate_compile()
            } else if rung == Rung::Lint.as_index() {
                cfg.timeouts.gate_clippy()
            } else {
                cfg.timeouts.gate_test()
            }
        },
    )
}

/// Resolve HTTP request timeout from `TimeoutConfig`.
pub(crate) fn http_request_timeout(config: &RunConfig) -> Duration {
    config.roko_config.as_deref().map_or_else(
        || roko_core::config::TimeoutConfig::default().http_request(),
        |cfg| cfg.timeouts.http_request(),
    )
}

/// Resolve health check timeout from `TimeoutConfig`.
pub(crate) fn health_check_timeout(config: &RunConfig) -> Duration {
    config.roko_config.as_deref().map_or_else(
        || roko_core::config::TimeoutConfig::default().health_check(),
        |cfg| cfg.timeouts.health_check(),
    )
}

// ─── RunContext ──────────────────────────────────────────────────────────

const RUNNER_WORKTREE_IDLE_TTL_SECS: u64 = 30 * 60;
static NEXT_GATE_EFFECT: AtomicU64 = AtomicU64::new(1);

fn new_gate_effect(attempt: TaskAttemptRef, kind: GateCompletionKind, rung: u32) -> GateEffectRef {
    GateEffectRef {
        attempt,
        kind,
        rung,
        generation: NEXT_GATE_EFFECT.fetch_add(1, Ordering::Relaxed),
    }
}

enum RoutedAgentEvent {
    Agent {
        attempt: TaskAttemptRef,
        effect: EffectRef,
        agent_id: String,
        event: AgentEvent,
    },
    Fatal {
        attempt: TaskAttemptRef,
        message: String,
    },
}

impl RoutedAgentEvent {
    fn for_attempt(
        attempt: TaskAttemptRef,
        effect: EffectRef,
        agent_id: String,
        event: AgentEvent,
    ) -> Self {
        Self::Agent {
            attempt,
            effect,
            agent_id,
            event,
        }
    }

    fn fatal(attempt: TaskAttemptRef, message: String) -> Self {
        Self::Fatal { attempt, message }
    }
}

async fn forward_agent_events(
    attempt: TaskAttemptRef,
    effect: EffectRef,
    agent_id: String,
    mut raw_rx: mpsc::Receiver<AgentEvent>,
    routed_tx: mpsc::Sender<RoutedAgentEvent>,
) {
    while let Some(event) = raw_rx.recv().await {
        let routed =
            RoutedAgentEvent::for_attempt(attempt.clone(), effect, agent_id.clone(), event);
        if routed_tx.send(routed).await.is_err() {
            break;
        }
    }
}

enum AgentRuntimeResource {
    Dispatching(tokio::sync::OwnedSemaphorePermit),
    Cli {
        handle: AgentHandle,
        forwarder: tokio::task::JoinHandle<()>,
        permit: tokio::sync::OwnedSemaphorePermit,
    },
    Bridge {
        bridge: tokio::task::JoinHandle<()>,
        forwarder: tokio::task::JoinHandle<()>,
        permit: tokio::sync::OwnedSemaphorePermit,
    },
    AwaitingGate,
    Gate {
        effect: GateEffectRef,
        handle: tokio::task::JoinHandle<()>,
    },
}

fn finish_gate_claim(
    ownership: &mut AttemptOwnership<AgentRuntimeResource>,
    claim: &mut Option<(AttemptClaim<AgentRuntimeResource>, EffectRef)>,
    await_next_gate: bool,
) {
    let Some((claim, effect)) = claim.take() else {
        return;
    };
    if await_next_gate {
        ownership
            .transition_claim(claim, AttemptPhase::AwaitingGate, effect)
            .expect("owned gate claim must transition");
    } else {
        ownership
            .complete_claim(claim)
            .expect("owned gate claim must complete");
    }
}

struct AgentSettlement {
    exit_code: Option<i32>,
    errors: Vec<String>,
    unconfirmed: Option<AgentRuntimeResource>,
}

async fn settle_agent_resource(resource: AgentRuntimeResource) -> AgentSettlement {
    match resource {
        AgentRuntimeResource::Cli {
            handle,
            forwarder,
            permit,
        } => match handle.wait().await {
            AgentWait::Confirmed {
                exit_code,
                reader_errors,
                ..
            } => {
                let mut errors = reader_errors;
                if let Err(err) = forwarder.await {
                    errors.push(format!("agent forwarder failed: {err}"));
                }
                drop(permit);
                AgentSettlement {
                    exit_code,
                    errors,
                    unconfirmed: None,
                }
            }
            AgentWait::Unconfirmed { handle, errors } => AgentSettlement {
                exit_code: None,
                errors,
                unconfirmed: Some(AgentRuntimeResource::Cli {
                    handle,
                    forwarder,
                    permit,
                }),
            },
        },
        AgentRuntimeResource::Bridge {
            bridge,
            forwarder,
            permit,
        } => {
            let mut errors = Vec::new();
            if let Err(err) = bridge.await {
                errors.push(format!("agent bridge failed: {err}"));
            }
            if let Err(err) = forwarder.await {
                errors.push(format!("agent forwarder failed: {err}"));
            }
            drop(permit);
            AgentSettlement {
                exit_code: Some(0),
                errors,
                unconfirmed: None,
            }
        }
        other => AgentSettlement {
            exit_code: None,
            errors: vec!["agent terminal event arrived outside the agent phase".to_string()],
            unconfirmed: Some(other),
        },
    }
}

fn agent_terminal_failure(event: &AgentEvent, settlement: &AgentSettlement) -> Option<String> {
    if let AgentEvent::TurnCompleted { is_error: true, .. } = event {
        return Some("agent reported an error result".to_string());
    }
    if let Some(error) = settlement.errors.first() {
        return Some(error.clone());
    }
    match settlement.exit_code {
        Some(0) => None,
        Some(code) => Some(format!("agent exited with status {code}")),
        None => Some("agent exit status was not confirmed".to_string()),
    }
}

#[derive(Clone)]
struct TaskRuntimeState {
    agent_active: bool,
    agent_model: String,
    agent_provider: String,
    agent_output: String,
    session_id: Option<String>,
    agent_pid: Option<u32>,
    agent_turn_completed: bool,
    tokens_in: u64,
    tokens_out: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cost_usd: f64,
    task_agent_calls: u32,
    task_model_hint: Option<String>,
    current_prompt_text: String,
    current_daimon_strategy: Option<StrategyCoordinates>,
    gate_output: String,
    task_started_at: Instant,
    last_dispatch_ms: u64,
    routing_context: Option<roko_learn::model_router::RoutingContext>,
}

impl TaskRuntimeState {
    fn capture(state: &RunState) -> Self {
        Self {
            agent_active: state.agent_active,
            agent_model: state.agent_model.clone(),
            agent_provider: state.agent_provider.clone(),
            agent_output: state.agent_output.clone(),
            session_id: state.session_id.clone(),
            agent_pid: state.agent_pid,
            agent_turn_completed: state.agent_turn_completed,
            tokens_in: state.tokens_in,
            tokens_out: state.tokens_out,
            cache_read_tokens: state.cache_read_tokens,
            cache_write_tokens: state.cache_write_tokens,
            cost_usd: state.cost_usd,
            task_agent_calls: state.task_agent_calls,
            task_model_hint: state.task_model_hint.clone(),
            current_prompt_text: state.current_prompt_text.clone(),
            current_daimon_strategy: state.current_daimon_strategy.clone(),
            gate_output: state.gate_output.clone(),
            task_started_at: state.task_started_at,
            last_dispatch_ms: state.last_dispatch_ms,
            routing_context: state.routing_context.clone(),
        }
    }

    fn restore(&self, state: &mut RunState, plan_id: &str, task_id: &str) {
        state.plan_id = plan_id.to_string();
        state.current_task = task_id.to_string();
        state.agent_active = self.agent_active;
        state.agent_model = self.agent_model.clone();
        state.agent_provider = self.agent_provider.clone();
        state.agent_output = self.agent_output.clone();
        state.session_id = self.session_id.clone();
        state.agent_pid = self.agent_pid;
        state.agent_turn_completed = self.agent_turn_completed;
        state.tokens_in = self.tokens_in;
        state.tokens_out = self.tokens_out;
        state.cache_read_tokens = self.cache_read_tokens;
        state.cache_write_tokens = self.cache_write_tokens;
        state.cost_usd = self.cost_usd;
        state.task_agent_calls = self.task_agent_calls;
        state.task_model_hint = self.task_model_hint.clone();
        state.current_prompt_text = self.current_prompt_text.clone();
        state.current_daimon_strategy = self.current_daimon_strategy.clone();
        state.gate_output = self.gate_output.clone();
        state.task_started_at = self.task_started_at;
        state.last_dispatch_ms = self.last_dispatch_ms;
        state.routing_context = self.routing_context.clone();
    }
}

fn task_key(plan_id: &str, task_id: &str) -> String {
    format!("{plan_id}:{task_id}")
}

fn queue_pending_gate_task(
    pending_gate_tasks: &mut HashMap<String, Vec<String>>,
    plan_id: &str,
    task_id: &str,
) {
    if task_id.is_empty() {
        return;
    }
    let pending = pending_gate_tasks.entry(plan_id.to_string()).or_default();
    if !pending.iter().any(|queued| queued == task_id) {
        pending.push(task_id.to_string());
    }
}

fn remove_pending_gate_task(
    pending_gate_tasks: &mut HashMap<String, Vec<String>>,
    plan_id: &str,
    task_id: &str,
) {
    let Some(pending) = pending_gate_tasks.get_mut(plan_id) else {
        return;
    };
    pending.retain(|queued| queued != task_id);
    if pending.is_empty() {
        pending_gate_tasks.remove(plan_id);
    }
}

fn cleanup_finished_task_gate(
    pending_gate_tasks: &mut HashMap<String, Vec<String>>,
    task_runtime_states: &mut HashMap<String, TaskRuntimeState>,
    executor: &mut ParallelExecutor,
    completion: &GateCompletion,
) {
    if completion.kind != GateCompletionKind::Gate {
        return;
    }
    remove_pending_gate_task(pending_gate_tasks, &completion.plan_id, &completion.task_id);
    if pending_gate_tasks
        .get(&completion.plan_id)
        .is_some_and(|pending| !pending.is_empty())
        && let Some(plan) = executor.plan_state_mut(&completion.plan_id)
    {
        plan.current_phase = PlanPhase::Gating;
    }
    task_runtime_states.remove(&task_key(&completion.plan_id, &completion.task_id));
}

fn restore_task_runtime(
    state: &mut RunState,
    runtimes: &HashMap<String, TaskRuntimeState>,
    plan_id: &str,
    task_id: &str,
) {
    if let Some(runtime) = runtimes.get(&task_key(plan_id, task_id)) {
        runtime.restore(state, plan_id, task_id);
    } else {
        state.plan_id = plan_id.to_string();
        state.current_task = task_id.to_string();
    }
}

/// Shared context for the dispatch loop, replacing 11 loose parameters.
struct RunContext<'a> {
    executor: &'a mut ParallelExecutor,
    task_dag: &'a mut TaskDag,
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    skip_enrichment: &'a HashMap<String, bool>,
    config: &'a RunConfig,
    sink: &'a dyn RunOutputSink,
    tui: &'a TuiBridge,
    state: &'a mut RunState,
    attempt_ownership: &'a mut AttemptOwnership<AgentRuntimeResource>,
    pending_gate_tasks: &'a mut HashMap<String, Vec<String>>,
    agent_tx: &'a mpsc::Sender<RoutedAgentEvent>,
    gate_tx: &'a mpsc::Sender<GateCompletion>,
    fatal_tx: mpsc::Sender<RoutedAgentEvent>,
    paths: &'a PersistPaths,
    merge_queue: &'a MergeQueue,
    worktrees: &'a WorktreeManager,
    gate_thresholds: &'a GateThresholds,
    snapshot_writer: &'a SnapshotWriter,
    prompt_cache: &'a Arc<PromptCache>,
    factory: &'a SharedAgentFactory,
    task_sem: Arc<tokio::sync::Semaphore>,
    gate_sem: Arc<tokio::sync::Semaphore>,
    task_runtime_states: &'a mut HashMap<String, TaskRuntimeState>,
    legacy_gate_attempts: &'a mut HashMap<String, TaskAttemptRef>,
    preflight_attempted: &'a mut HashSet<TaskAttemptRef>,
    /// Prompt section diagnostics per attempt key — populated at dispatch,
    /// consumed on gate completion to build SectionOutcomeRecords.
    section_diagnostics: &'a mut HashMap<String, PromptDiagnostics>,
    /// Playbook IDs per attempt key — populated at dispatch, consumed on gate
    /// terminal to call `PlaybookStore::record_outcome`.
    task_playbook_ids: &'a mut HashMap<String, Vec<String>>,
}

fn default_runner_worktree_manager(workdir: &Path) -> WorktreeManager {
    WorktreeManager::new(WorktreeConfig {
        repo_root: workdir.to_path_buf(),
        base_branch: "HEAD".to_string(),
        worktrees_root: workdir.join(".roko").join("worktrees"),
        max_live: None,
        idle_ttl: Duration::from_secs(RUNNER_WORKTREE_IDLE_TTL_SECS),
    })
}

async fn ensure_plan_workdir(
    worktrees: &WorktreeManager,
    plan_id: &str,
) -> std::result::Result<PathBuf, String> {
    let handle = worktrees
        .ensure_for_plan(plan_id)
        .await
        .map_err(|err| format!("worktree unavailable for plan {plan_id}: {err}"))?;
    worktrees.touch(plan_id);
    Ok(handle.path)
}

fn tracked_plan_workdir(worktrees: &WorktreeManager, plan_id: &str) -> Option<PathBuf> {
    worktrees.get(plan_id).map(|handle| {
        worktrees.touch(plan_id);
        handle.path
    })
}

// Result of dispatching one executor action. Most actions are internal phase
// advances; the runner ledger only records an agent start when a runtime was
// actually launched for a concrete task.
enum ActionDispatchOutcome {
    Noop,
    Handled,
    AgentStarted { plan_id: String, task_id: String },
}

// ─── Main Entry Point ───────────────────────────────────────────────────

/// Run all plans to completion (or cancellation).
pub async fn run(
    plans: Vec<Plan>,
    config: &RunConfig,
    state_hub: &StateHub,
    cancel: CancellationToken,
) -> Result<RunReport> {
    // ── Ensure effective RokoConfig is available ─────────────────────────
    //
    // The CLI bootstrap path (`commands/plan.rs`) already loads config via
    // `RokoBootstrap` / `load_config_unified` and passes it through
    // `RunConfig.roko_config`. This fallback covers secondary callers
    // (tests, integration shims) that construct `RunConfig` without a
    // pre-loaded config: we call the canonical core loader which performs
    // ancestor walk, global merge, and env var overrides — no ad-hoc
    // project-root resolution in the runner.
    let mut config = config.clone();
    if config.roko_config.is_none() {
        let loaded =
            roko_core::config::loader::load_config_unified(&config.workdir).with_context(|| {
                format!(
                    "load roko config for runner workdir {}",
                    config.workdir.display()
                )
            })?;
        config.roko_config = Some(Arc::new(loaded));
    }

    if config.http_event_sink.is_none() {
        config.http_event_sink = HttpEventSink::from_env();
    }

    let max_concurrent_tasks = config.max_concurrent_tasks.max(1);
    let task_timeout_secs = duration_secs(agent_dispatch_timeout(&config));

    let exec_config = ExecutorConfig {
        max_concurrent_plans: plans.len().max(1),
        max_concurrent_tasks,
        max_auto_fix_iterations: config.max_retries,
        task_timeout_secs,
        ..Default::default()
    };
    let paths = PersistPaths::from_workdir(&config.workdir)?;
    let snapshot_writer = SnapshotWriter::new(4);
    persist::cleanup_orphaned_agents(&paths);
    let mut gate_thresholds = persist::load_gate_thresholds(&paths).unwrap_or_default();

    // Ensure knowledge store directory exists for episode ingestion.
    let neuro_dir = config.layout.neuro_dir();
    if let Err(err) = std::fs::create_dir_all(&neuro_dir) {
        warn!(error = %err, "failed to create neuro directory");
    }

    // ── Strict resume validation + JSONL recovery ─────────────────────────
    //
    // Run before any state file is reopened. The validator:
    // 1. Loads `.roko/state/run-state.json` if present.
    // 2. Verifies current task fingerprints against the prior snapshot
    //    unless `--force-resume` is set.
    // 3. Reports drifted completed tasks so the caller can re-queue
    //    them instead of aborting the resume.
    // 4. Truncates `episodes.jsonl`, `events.jsonl`, and
    //    `efficiency.jsonl` after their last validated line (recovers
    //    from partial-append corruption left by a prior crash).
    //
    // On `PlanMissing` / `UnsupportedSchema` the validator still
    // returns Err. We surface the failure and abort the run so the
    // operator can either edit the plan back into a known state or
    // discard the snapshot.
    // Try the unified state snapshot first; fall back to legacy run-state.json.
    let (prior_snapshot, loaded_gate_thresholds) = match persist::load_state_snapshot(&paths) {
        Ok(Some(unified)) => {
            info!(
                timestamp_ms = unified.timestamp_ms,
                "loaded state snapshot -- checksum valid"
            );
            let run_state: Option<persist::RunStateSnapshot> =
                match serde_json::from_str(&unified.run_state_json) {
                    Ok(rs) => Some(rs),
                    Err(err) => {
                        warn!(
                            error = %err,
                            "failed to parse run_state_json from unified snapshot; ignoring"
                        );
                        None
                    }
                };
            let loaded_gt: Option<GateThresholds> =
                match serde_json::from_str(&unified.gate_thresholds_json) {
                    Ok(gt) => Some(gt),
                    Err(err) => {
                        warn!(
                            error = %err,
                            "failed to parse gate_thresholds_json from unified snapshot; using file"
                        );
                        None
                    }
                };
            (run_state, loaded_gt)
        }
        Ok(None) => {
            // No unified snapshot -- try legacy run-state.json.
            match persist::load_run_state(&paths) {
                Ok(Some(snapshot)) => {
                    warn!("no state-snapshot.json found; falling back to legacy run-state.json");
                    (Some(snapshot), None)
                }
                Ok(None) => (None, None),
                Err(err) => {
                    warn!(
                        error = %err,
                        "failed to read prior run-state.json; continuing without seeded resume state"
                    );
                    (None, None)
                }
            }
        }
        Err(err) => {
            warn!(
                error = %err,
                "failed to load state snapshot; falling back to legacy run-state.json"
            );
            match persist::load_run_state(&paths) {
                Ok(snap) => (snap, None),
                Err(err2) => {
                    warn!(
                        error = %err2,
                        "failed to read legacy run-state.json; continuing without seeded resume state"
                    );
                    (None, None)
                }
            }
        }
    };
    // Override gate thresholds if we loaded them from the unified snapshot.
    if let Some(gt) = loaded_gate_thresholds {
        gate_thresholds = gt;
    }
    let resume_report = {
        let mut plan_map: HashMap<String, Vec<TaskDef>> = HashMap::new();
        for plan in &plans {
            plan_map.insert(plan.id.clone(), plan.tasks.tasks.clone());
        }
        if let Some(snapshot) = prior_snapshot.as_ref() {
            apply_revised_tasks_to_plan_map(&mut plan_map, &snapshot.revised_tasks);
        }
        let prior_fingerprints = prior_snapshot
            .as_ref()
            .map(|snapshot| snapshot.fingerprints.as_slice())
            .unwrap_or(&[]);
        match super::resume::prepare_resume_with_force(
            &paths,
            &plan_map,
            prior_fingerprints,
            config.force_resume,
        ) {
            Ok(report) => {
                if report.resumed && !config.force_resume {
                    info!(
                        prior_run_id = ?report.prior_run_id,
                        validated_tasks = report.validated_tasks,
                        "resume validated"
                    );
                }
                for f in &report.recovered_files {
                    use super::resume::JsonlRecoveryReport;
                    match &f.recovery {
                        JsonlRecoveryReport::Clean { .. } => {}
                        JsonlRecoveryReport::TruncatedTrailing {
                            truncated_bytes, ..
                        } => {
                            warn!(file = %f.path, truncated_bytes, "recovered partial JSONL");
                        }
                        JsonlRecoveryReport::DroppedInvalid { dropped_lines, .. } => {
                            warn!(file = %f.path, dropped_lines, "recovered malformed JSONL");
                        }
                    }
                }
                report
            }
            Err(err) => {
                return Err(anyhow::anyhow!("resume validation failed: {err}"));
            }
        }
    };

    // Verify checkpoint integrity when resuming an existing run.
    // A mismatch means the state files were modified outside of a clean
    // atomic write (e.g. partial crash, manual edit, cross-plan leakage).
    // This is non-fatal: we warn and continue so the run is not blocked,
    // but the operator is alerted to potential state inconsistency.
    if prior_snapshot.is_some() {
        let state_dir = paths.executor_json.parent().unwrap_or(&paths.executor_json);
        match persist::verify_checkpoint(state_dir) {
            Ok(true) => {
                debug!("state checkpoint verified — all files consistent");
            }
            Ok(false) => {
                warn!(
                    state_dir = %state_dir.display(),
                    "state checkpoint mismatch: one or more state files changed since last write \
                     (possible cross-plan leakage or crash mid-write)"
                );
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to verify state checkpoint; continuing without verification"
                );
            }
        }
    }

    // Prefer the embedded router snapshot over the file-backed router on resume.
    if let Some(router_json) = resume_report.cascade_router_json.as_deref() {
        if let Some(existing_router) = config.cascade_router.as_ref() {
            let model_slugs = existing_router.model_slugs().to_vec();
            match roko_learn::cascade_router::CascadeRouter::from_snapshot_json(
                router_json,
                model_slugs,
            ) {
                Ok(router) => {
                    info!("restored cascade router from run-state snapshot");
                    config.cascade_router = Some(Arc::new(router));
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "failed to restore cascade router from run-state snapshot; using file-based state"
                    );
                }
            }
        }
    }

    // All mutations to `config` are done; reborrow as shared reference so
    // downstream helpers that expect `&RunConfig` work without extra `&`.
    let config = &config;

    // Per-run task semaphore — limits concurrently dispatched agents.
    let task_sem = Arc::new(tokio::sync::Semaphore::new(
        config.max_concurrent_tasks.max(1),
    ));
    let worktrees = default_runner_worktree_manager(&config.workdir);

    // Per-run gate semaphore — limits how many gate rungs execute concurrently.
    let gate_sem = Arc::new(tokio::sync::Semaphore::new(config.gate_concurrency.max(1)));

    // Build plan ID set for resume validation.
    let plan_ids: Vec<String> = plans.iter().map(|p| p.id.clone()).collect();

    // Only resume if snapshot exists AND its plans match the current run.
    let resume = load_executor(&paths, &exec_config, &plan_ids);
    let mut executor = resume.executor;
    let merge_queue = resume.merge_queue;

    // Index tasks by plan_id/task_id for lookup.
    let mut task_index: HashMap<String, HashMap<String, TaskDef>> = HashMap::new();
    let mut task_dag = TaskDag::new(DagConfig {
        plan_timeout: plan_total_timeout(config),
        ..DagConfig::default()
    });
    let mut total_tasks = 0usize;

    for plan in &plans {
        // add_plan is a no-op if plan already exists (from snapshot).
        let orc_state = OrcPlanState::new(&plan.id);
        executor.add_plan(orc_state);
        task_dag.plan_mut(&plan.id);

        let mut tasks_map = HashMap::new();
        for task in &plan.tasks.tasks {
            tasks_map.insert(task.id.clone(), task.clone());
            total_tasks += 1;
        }
        task_index.insert(plan.id.clone(), tasks_map);
    }

    // Channels.
    let (agent_tx, mut agent_rx) = mpsc::channel::<RoutedAgentEvent>(256);
    // Dynamic gate channel buffer: max_concurrent_tasks * 7 rungs, clamped to [32, 256].
    let gate_buffer = (config.max_concurrent_tasks * 7).max(32).min(256);
    let (gate_tx, mut gate_rx) = mpsc::channel::<GateCompletion>(gate_buffer);
    let sink = config.output_sink.as_ref();

    // -- Warm cargo cache -------------------------------------------------------
    // Run `cargo check --workspace` once before the main loop so that
    // subsequent per-task compile gates are incremental (2-5s vs 30-120s).
    if config.warm_cache {
        sink.warm_cache_started();
        let warm_start = std::time::Instant::now();
        let warm_result = tokio::process::Command::new("cargo")
            .args(["check", "--workspace", "--message-format=short"])
            .current_dir(&config.workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        let warm_ms = warm_start.elapsed().as_millis() as u64;
        match warm_result {
            Ok(status) if status.success() => {
                info!(warm_ms, "cargo cache warmed successfully");
                sink.warm_cache_completed(warm_ms);
            }
            Ok(status) => {
                warn!(
                    warm_ms,
                    exit_code = status.code().unwrap_or(-1),
                    "cargo cache warm failed (non-fatal)"
                );
            }
            Err(e) => {
                warn!(warm_ms, error = %e, "cargo cache warm failed (non-fatal)");
            }
        }
    }

    // Seed playbooks if the store is empty (bootstrap chicken-and-egg).
    seed_playbooks_if_empty(&config.layout).await;

    // Build prompt cache once — reused across all task dispatches.
    // Refreshed when stale (default 5 min) or after gate failures.
    let mut prompt_cache = Arc::new(PromptCache::load(&config.workdir));

    // Shared agent factory — expensive components (semaphores, MCP tools,
    // dispatcher, resolver) created once and reused for every task dispatch.
    let t_factory = Instant::now();
    let factory = SharedAgentFactory::new(
        config.roko_config.clone().unwrap_or_default(),
        config.mcp_config.as_ref(),
        config.cascade_router.clone(),
        Some(Arc::clone(&prompt_cache)),
    )
    .await;
    info!(
        factory_init_ms = t_factory.elapsed().as_millis() as u64,
        "agent factory initialized"
    );

    // State and TUI bridge.
    let tui = TuiBridge::new(state_hub.sender());
    let mut state = RunState::new(total_tasks);
    let mut dream_completion_pending = false;

    // Run ledger — optional enhancement for tracking task starts, completions,
    // and gate outcomes in a typed JSONL file at `.roko/state/run-ledger.jsonl`.
    // Initialization is infallible; we keep it as Option so downstream code
    // gracefully no-ops if we ever make init fallible.
    let mut run_ledger: Option<RunLedger> = {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let prompt_summary = plans
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| "multi-plan".to_string());
        let ledger = RunLedger::new(
            state.run_id(),
            prompt_summary,
            WorkflowConfig::default(),
            now_ms,
        );
        debug!("run ledger initialized for run {}", state.run_id());
        Some(ledger)
    };

    // Compute task fingerprints once at startup so every subsequent
    // `save_snapshot` writes them into `run-state.json` for the strict
    // resume validator to consume on the next run.
    state.task_fingerprints = plans
        .iter()
        .flat_map(|plan| {
            plan.tasks
                .tasks
                .iter()
                .map(move |task| persist::TaskDefFingerprint::from_task(task, &plan.id))
        })
        .collect();

    if matches!(resume.marker.outcome, ResumeOutcome::Resumed) {
        if let Some(snapshot) = prior_snapshot.as_ref() {
            restore_state_from_resume_snapshot(
                &mut state,
                snapshot,
                &task_index,
                &resume_report.drifted_tasks,
            );
        }
    } else {
        seed_completed_tasks_from_plan_status(&mut state, &plans);
        initialize_terminal_plan_phases(&mut executor, &state, &plans);
    }
    if !state.revised_tasks.is_empty() {
        for revision in state.revised_tasks.values() {
            apply_task_revision_to_index(&mut task_index, revision);
        }
        refresh_task_fingerprints_from_index(&mut state, &task_index);
        info!(
            revised_tasks = state.revised_tasks.len(),
            "restored durable task revisions from run-state snapshot"
        );
    }
    seed_task_dag_from_run_state(&mut task_dag, &plans, &state);

    let mut attempt_ownership = AttemptOwnership::<AgentRuntimeResource>::default();
    let mut pending_gate_tasks: HashMap<String, Vec<String>> = HashMap::new();
    let mut task_runtime_states: HashMap<String, TaskRuntimeState> = HashMap::new();
    let mut legacy_gate_attempts: HashMap<String, TaskAttemptRef> = HashMap::new();
    let mut preflight_attempted: HashSet<TaskAttemptRef> = HashSet::new();
    let mut feedback_tasks: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();

    // Track prompt section diagnostics per attempt so gate completions can
    // build SectionOutcomeRecords joining section presence to pass/fail.
    let mut section_diagnostics: HashMap<String, PromptDiagnostics> = HashMap::new();

    // PlaybookStore for recording gate outcomes back to playbook success/failure
    // counters. This closes the feedback loop so adaptive playbook selection can
    // learn which playbooks correlate with task success.
    let playbook_store = PlaybookStore::new(config.layout.playbooks_dir());

    // Playbook IDs injected per task attempt (keyed by attempt key
    // "{plan_id}:{task_id}:{attempt}"). Populated at dispatch time from prompt
    // diagnostics so the gate terminal handler can call record_outcome.
    let mut task_playbook_ids: HashMap<String, Vec<String>> = HashMap::new();

    // skip_enrichment is a plan-level DAG phase control: when true, the plan
    // transitions directly from "started" to "implementing", bypassing the
    // "enriching" executor phase. This is NOT an LLM pre-processing pipeline
    // -- the legacy orchestrate.rs enrichment pipeline (multi-step LLM
    // pre-dispatch enrichment) was not ported to v2 because the v2 prompt
    // builder already handles context assembly via PromptContext sections
    // (workspace_context, cfactor_context, knowledge, playbooks, etc.).
    let skip_enrichment: HashMap<String, bool> = plans
        .iter()
        .map(|p| (p.id.clone(), p.tasks.meta.skip_enrichment))
        .collect();

    let mut tick_interval = interval(Duration::from_millis(100));
    let mut flush_interval = interval(Duration::from_secs(2));
    let plan_timeout_duration = plan_total_timeout(&config);
    let agent_timeout_duration = agent_dispatch_timeout(&config);
    let plan_deadline = tokio::time::Instant::now() + plan_timeout_duration;
    let plan_timeout = tokio::time::sleep_until(plan_deadline);
    tokio::pin!(plan_timeout);

    info!(
        plan_count = plans.len(),
        total_tasks,
        model = %config.model,
        max_concurrent = config.max_concurrent_tasks,
        max_retries = config.max_retries,
        max_gate_rung = config.max_gate_rung,
        max_plan_usd = config.max_plan_usd,
        max_turn_usd = config.max_turn_usd,
        timeout_secs = duration_secs(agent_timeout_duration),
        plan_timeout_secs = duration_secs(plan_timeout_duration),
        clippy_enabled = config.clippy_enabled,
        skip_tests = config.skip_tests,
        output_sink = ?config.output_sink,
        has_mcp_config = config.mcp_config.is_some(),
        has_cascade_router = config.cascade_router.is_some(),
        "starting runner v2 event loop"
    );
    let run_id = state.run_id().to_string();
    emit_runner_event(
        &paths,
        &mut state,
        &tui,
        config,
        RunnerEvent::resume_marker(&run_id, resume.marker.clone()),
    );
    emit_runner_event(
        &paths,
        &mut state,
        &tui,
        config,
        RunnerEvent::run_started(
            &run_id,
            plan_ids.clone(),
            total_tasks,
            matches!(resume.marker.outcome, ResumeOutcome::Resumed),
            config.resume_session.clone(),
        ),
    );

    // ─── Phase 0: Initialize subsystems ─────────────────────────────
    // Extension chain init (no-op with empty chain).
    if let Some(ext_chain) = &config.extension_chain {
        let mut chain = ext_chain.lock().await;
        let errors = chain.init_all().await;
        for (name, err) in &errors {
            warn!(extension = %name, error = %err, "extension init failed");
        }
        if !errors.is_empty() {
            info!(
                failed = errors.len(),
                "extension chain init completed with errors"
            );
        }
    }

    // Register MCP connectors in the connector registry.
    if let Some(registry) = &config.connector_registry {
        if let Some(mcp_path) = &config.mcp_config {
            if let Ok(contents) = std::fs::read_to_string(mcp_path) {
                if let Ok(mcp_json) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(servers) = mcp_json.get("mcpServers").and_then(|s| s.as_object()) {
                        if let Ok(mut reg) = registry.lock() {
                            for name in servers.keys() {
                                reg.register(roko_core::ConnectorInfo {
                                    name: name.clone(),
                                    kind: roko_core::ConnectorKind::Mcp,
                                    health: roko_core::ConnectorHealth {
                                        status: roko_core::ConnectorStatus::Connected,
                                        latency_ms: 0,
                                        last_check: chrono::Utc::now(),
                                    },
                                    created_at: chrono::Utc::now(),
                                    metadata: serde_json::Value::Null,
                                });
                            }
                            info!(count = servers.len(), "registered MCP connectors");
                        }
                    }
                }
            }
        }
    }

    // ── Spawn the learning event subscriber ──────────────────────────
    // Background task that consumes gate/turn events and feeds them into
    // CalibrationPolicy, VerdictScorer, ProviderHealth, CascadeRouter,
    // CostsDb, and the efficiency JSONL log.
    let learning_event_bus = roko_learn::events::EventBus::new(256);
    let learning_subscriber_rx = learning_event_bus.subscribe();
    let subscriber_model_slugs: Vec<String> = config
        .cascade_router
        .as_ref()
        .map(|r| r.model_slugs().to_vec())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| vec![config.model.clone()]);
    let learning_subscriber_handle = {
        use std::sync::Mutex;
        let health = Arc::new(roko_learn::provider_health::ProviderHealthRegistry::new());
        let latency = Arc::new(roko_learn::latency::LatencyRegistry::new());
        let router = Arc::new(roko_learn::cascade_router::CascadeRouter::new(
            subscriber_model_slugs,
        ));
        let anomaly_start_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let anomaly = Arc::new(Mutex::new(roko_learn::anomaly::AnomalyDetector::new(
            anomaly_start_ms,
        )));
        let costs = Arc::new(roko_learn::costs_db::CostsDb::new());
        let efficiency_path = config.layout.learn_dir().join("efficiency.jsonl");
        let router_persist_path = Some(config.layout.learn_dir().join("cascade-router.json"));
        tokio::spawn(roko_learn::event_subscriber::run_learning_subscriber(
            learning_subscriber_rx,
            health,
            latency,
            router,
            anomaly,
            costs,
            efficiency_path,
            router_persist_path,
        ))
    };

    let mut timed_out = false;

    loop {
        // Cancel-safety analysis:
        //   Branch 1 (agent_rx.recv): cancel-safe — mpsc::Receiver::recv drops no data.
        //   Branch 2 (gate_rx.recv):  cancel-safe — mpsc::Receiver::recv drops no data.
        //   Branch 3 (tick_interval): cancel-safe — Interval::tick is restartable.
        //   Branch 4 (flush_interval): cancel-safe — Interval::tick is restartable.
        //   Branch 5 (plan_timeout): cancel-safe — fixed deadline, no state lost.
        //   Branch 6 (cancel.cancelled): cancel-safe — CancellationToken is idempotent.
        tokio::select! {
            // ─── Branch 1: Agent events ─────────────────────────────
            Some(routed) = agent_rx.recv() => {
                let RoutedAgentEvent::Agent {
                    attempt: event_attempt,
                    effect: event_effect,
                    agent_id: event_agent_id,
                    event,
                } = routed else {
                    let RoutedAgentEvent::Fatal { attempt, message } = routed else {
                        unreachable!()
                    };
                    let current = TaskAttemptRef::new(
                        attempt.plan_id.clone(),
                        attempt.task_id.clone(),
                        state.iteration_for(&attempt.plan_id, &attempt.task_id),
                    );
                    if current != attempt || state.task_attempt_is_terminal(&attempt) {
                        warn!(attempt = %attempt.key(), "ignoring stale fatal effect");
                        continue;
                    }
                    restore_task_runtime(
                        &mut state,
                        &task_runtime_states,
                        &attempt.plan_id,
                        &attempt.task_id,
                    );
                    handle_agent_failure(
                        &mut executor, &mut task_dag, &task_index, &mut state, &paths,
                        &tui, sink, config, message,
                    );
                    continue;
                };
                let event_plan_id = event_attempt.plan_id.clone();
                let event_task_id = event_attempt.task_id.clone();
                let is_turn_done = matches!(&event, AgentEvent::TurnCompleted { .. });
                let is_exited = matches!(&event, AgentEvent::Exited { .. });
                let is_terminal = is_turn_done || is_exited;
                if !attempt_ownership.event_is_eligible(
                    &event_attempt,
                    AttemptPhase::Agent,
                    event_effect,
                ) {
                    debug!(attempt = %event_attempt.key(), effect = event_effect.0,
                        "ignoring late agent event without exact ownership");
                    continue;
                }
                let mut settlement = None;
                let mut terminal_failure = None;
                if is_terminal {
                    let mut claim = match attempt_ownership.claim_phase(
                        &event_attempt,
                        AttemptPhase::Agent,
                        event_effect,
                    ) {
                        Ok(claim) => claim,
                        Err(_) => continue,
                    };
                    let resource = claim.replace_resource(AgentRuntimeResource::AwaitingGate);
                    let mut result = settle_agent_resource(resource).await;
                    terminal_failure = agent_terminal_failure(&event, &result);
                    if let Some(resource) = result.unconfirmed.take() {
                        claim.replace_resource(resource);
                        attempt_ownership
                            .transition_claim(claim, AttemptPhase::AgentUnconfirmed, event_effect)
                            .expect("unconfirmed agent ownership must be retained");
                    } else if terminal_failure.is_some() {
                        attempt_ownership
                            .complete_claim(claim)
                            .expect("confirmed failed agent claim must complete");
                    } else {
                        attempt_ownership
                            .transition_claim(claim, AttemptPhase::AwaitingGate, event_effect)
                            .expect("successful agent claim must await gate");
                    }
                    settlement = Some(result);
                }
                restore_task_runtime(
                    &mut state,
                    &task_runtime_states,
                    &event_plan_id,
                    &event_task_id,
                );
                let turn_completed_before_event = state.agent_turn_completed;
                let turn_error = terminal_failure.is_some();

                handle_agent_event(&event, &mut state, &tui, sink);
                append_agent_event(&paths, &event, &state);
                publish_learning_agent_event(&learning_event_bus, &event, &state);
                task_runtime_states.insert(task_key(&event_plan_id, &event_task_id), TaskRuntimeState::capture(&state));

                // ── Forward progress-relevant agent events to RuntimeEvent ──
                if let Some(http_sink) = config.http_event_sink.as_ref() {
                    match &event {
                        AgentEvent::ToolCall { name, .. } => {
                            http_sink.emit(RuntimeEvent::ToolCallStarted {
                                run_id: run_id.clone(),
                                agent_id: format!("{}/{}", state.plan_id, state.current_task),
                                tool: name.clone(),
                                iteration: 0,
                            });
                        }
                        AgentEvent::MessageDelta { text } => {
                            http_sink.emit(RuntimeEvent::AgentOutput {
                                run_id: run_id.clone(),
                                agent_id: format!("{}/{}", state.plan_id, state.current_task),
                                chunk: text.clone(),
                            });
                        }
                        _ => {}
                    }
                }

                // Per-turn budget observation. This event arrives after the
                // provider turn has already completed, so a successful turn
                // should still move on to gating; otherwise we throw away a
                // potentially valid patch after paying for it.
                if is_turn_done {
                    let max_turn = config.max_turn_usd;
                    if let AgentEvent::TurnCompleted { total_cost_usd, .. } = &event
                        && turn_exceeds_budget(*total_cost_usd, max_turn)
                    {
                        warn!(
                            task = %state.current_task,
                            turn_cost = total_cost_usd.unwrap_or_default(),
                            limit = max_turn,
                            "single turn exceeded per-turn budget limit -- continuing to gate completed work"
                        );
                    }
                }

                if is_turn_done {
                    if let AgentEvent::TurnCompleted {
                        session_id,
                        total_cost_usd,
                        num_turns,
                        is_error,
                    } = &event
                    {
                        let agent_id = format!("{}/{}", state.plan_id, state.current_task);
                        let outcome = if turn_error {
                            AgentDispatchOutcome::Failed
                        } else {
                            AgentDispatchOutcome::Completed
                        };
                        let attempt = event_attempt.clone();
                        let run_id = state.run_id().to_string();
                        emit_runner_event(
                            &paths,
                            &mut state,
                            &tui,
                            config,
                            RunnerEvent::agent_completed(
                                &run_id,
                                attempt,
                                &agent_id,
                                outcome,
                                AgentCompletionSummary {
                                    session_id: session_id.clone(),
                                    total_cost_usd: *total_cost_usd,
                                    turns: *num_turns,
                                    exit_code: None,
                                    message: terminal_failure.clone(),
                                },
                            ),
                        );
                    }

                    // Extension: post_inference hook.
                    let task_role = task_index
                        .get(state.plan_id.as_str())
                        .and_then(|tasks| tasks.get(state.current_task.as_str()))
                        .and_then(|t| t.role.as_deref())
                        .unwrap_or("implementer");
                    fire_post_inference_hook(
                        config,
                        &state.plan_id,
                        &state.current_task,
                        &state.agent_model,
                        task_role,
                        !turn_error,
                        state.cost_usd,
                        state.task_elapsed_ms(),
                        &tui,
                    )
                    .await;

                    let plan_id = state.plan_id.clone();
                    if !plan_id.is_empty() {
                        if turn_error {
                            let message = terminal_failure.clone().or_else(|| agent_failure_message(&state.agent_output))
                                .unwrap_or_else(|| "agent reported an error result".to_string());
                            fire_on_error_hook(config, &message, "agent_turn", &tui, &state.plan_id, &state.current_task).await;
                            handle_agent_failure(
                                &mut executor,
                                &mut task_dag,
                                &task_index,
                                &mut state,
                                &paths,
                                &tui,
                                sink,
                                config,
                                message,
                            );
                            task_runtime_states
                                .remove(&task_key(&event_plan_id, &event_task_id));
                        } else {
                            queue_pending_gate_task(
                                &mut pending_gate_tasks,
                                &event_plan_id,
                                &event_task_id,
                            );
                            apply_agent_completion(&mut executor, &plan_id, &tui);
                        }
                        save_snapshot(config, &executor, &paths, &mut state, &merge_queue, &gate_thresholds, &snapshot_writer);
                    }
                }

                if is_exited {
                    let exit_code = settlement.as_ref().and_then(|result| result.exit_code);

                    let plan_id = state.plan_id.clone();
                    if !turn_completed_before_event && !plan_id.is_empty() {
                        let agent_id = format!("{}/{}", state.plan_id, state.current_task);
                        if terminal_failure.is_none() && exit_code == Some(0) {
                            queue_pending_gate_task(
                                &mut pending_gate_tasks,
                                &event_plan_id,
                                &event_task_id,
                            );
                            let attempt = event_attempt.clone();
                            let run_id = state.run_id().to_string();
                            emit_runner_event(
                                &paths,
                                &mut state,
                                &tui,
                                config,
                                RunnerEvent::agent_completed(
                                    &run_id,
                                    attempt,
                                    &agent_id,
                                    AgentDispatchOutcome::Exited,
                                    AgentCompletionSummary {
                                        exit_code,
                                        ..AgentCompletionSummary::default()
                                    },
                                ),
                            );
                            apply_agent_completion(&mut executor, &plan_id, &tui);
                        } else {
                            let message = terminal_failure.clone().unwrap_or_else(|| format!(
                                "agent process exited unsuccessfully: exit_code={}",
                                exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                            ));
                            let attempt = event_attempt.clone();
                            let run_id = state.run_id().to_string();
                            emit_runner_event(
                                &paths,
                                &mut state,
                                &tui,
                                config,
                                RunnerEvent::agent_completed(
                                    &run_id,
                                    attempt,
                                    &agent_id,
                                    AgentDispatchOutcome::Failed,
                                    AgentCompletionSummary {
                                        exit_code,
                                        message: Some(message.clone()),
                                        ..AgentCompletionSummary::default()
                                    },
                                ),
                            );
                            handle_agent_failure(
                                &mut executor,
                                &mut task_dag,
                                &task_index,
                                &mut state,
                                &paths,
                                &tui,
                                sink,
                                config,
                                message,
                            );
                            task_runtime_states
                                .remove(&task_key(&event_plan_id, &event_task_id));
                        }
                    }

                    save_snapshot(config, &executor, &paths, &mut state, &merge_queue, &gate_thresholds, &snapshot_writer);
                }
            }

            // ─── Branch 2: Verify completions ─────────────────────────
            Some(mut completion) = gate_rx.recv() => {
                let mut owned_gate_claim = None;
                let effect_key = gate_effect_key(
                    &completion.plan_id,
                    &completion.task_id,
                    completion.rung,
                    completion.kind,
                );
                let completion_attempt = if completion.kind == GateCompletionKind::Merge {
                    TaskAttemptRef::new(
                        completion.plan_id.clone(),
                        completion.task_id.clone(),
                        state.iteration_for(&completion.plan_id, &completion.task_id),
                    )
                } else if completion.effect.is_none() {
                    let Some(attempt) = take_matching_gate_attempt(
                        &mut legacy_gate_attempts,
                        &effect_key,
                        completion.attempt.as_ref(),
                    ) else {
                        warn!("dropping stale legacy gate completion");
                        continue;
                    };
                    attempt
                } else {
                    let Some(attempt) = completion.attempt.clone() else {
                        warn!("dropping gate completion without exact attempt");
                        continue;
                    };
                    let Some(gate_effect) = completion.effect.clone() else {
                        warn!(attempt = %attempt.key(), "dropping gate completion without exact effect");
                        continue;
                    };
                    if gate_effect.attempt != attempt
                        || attempt.plan_id != completion.plan_id
                        || attempt.task_id != completion.task_id
                        || gate_effect.kind != completion.kind
                        || gate_effect.rung != completion.rung
                    {
                        warn!(attempt = %attempt.key(), "dropping mismatched gate effect");
                        continue;
                    }
                    let Ok(mut claim) = attempt_ownership.claim_phase(
                        &attempt,
                        AttemptPhase::Gate,
                        EffectRef(gate_effect.generation),
                    ) else {
                        warn!(attempt = %attempt.key(), "dropping stale or duplicate gate completion");
                        continue;
                    };
                    let AgentRuntimeResource::Gate { effect, handle } = claim
                        .replace_resource(AgentRuntimeResource::AwaitingGate)
                    else {
                        error!(attempt = %attempt.key(), "gate owner did not retain its join handle");
                        attempt_ownership.complete_claim(claim).ok();
                        state.clear_gate_active(&effect_key);
                        task_dag.clear_running(&attempt.plan_id, &attempt.task_id);
                        cleanup_finished_task_gate(
                            &mut pending_gate_tasks,
                            &mut task_runtime_states,
                            &mut executor,
                            &completion,
                        );
                        continue;
                    };
                    if effect != gate_effect {
                        error!(attempt = %attempt.key(), "gate effect identity mismatch");
                        handle.abort();
                        let _ = handle.await;
                        attempt_ownership.complete_claim(claim).ok();
                        state.clear_gate_active(&effect_key);
                        task_dag.clear_running(&attempt.plan_id, &attempt.task_id);
                        cleanup_finished_task_gate(
                            &mut pending_gate_tasks,
                            &mut task_runtime_states,
                            &mut executor,
                            &completion,
                        );
                        continue;
                    }
                    if let Err(err) = handle.await {
                        error!(attempt = %attempt.key(), %err, "gate producer join failed");
                        attempt_ownership.complete_claim(claim).ok();
                        state.clear_gate_active(&effect_key);
                        task_dag.clear_running(&attempt.plan_id, &attempt.task_id);
                        cleanup_finished_task_gate(
                            &mut pending_gate_tasks,
                            &mut task_runtime_states,
                            &mut executor,
                            &completion,
                        );
                        continue;
                    }
                    owned_gate_claim = Some((claim, EffectRef(gate_effect.generation)));
                    attempt
                };
                if completion.kind != GateCompletionKind::Merge
                    && completion.attempt.as_ref() != Some(&completion_attempt)
                {
                    finish_gate_claim(
                        &mut attempt_ownership,
                        &mut owned_gate_claim,
                        false,
                    );
                    warn!(
                        plan_id = %completion.plan_id,
                        task_id = %completion.task_id,
                        rung = completion.rung,
                        kind = ?completion.kind,
                        reported_attempt = ?completion.attempt,
                        "dropping stale or duplicate gate completion"
                    );
                    continue;
                }
                restore_task_runtime(
                    &mut state,
                    &task_runtime_states,
                    &completion.plan_id,
                    &completion.task_id,
                );
                state.clear_gate_active(&effect_key);
                state.gate_output = completion.output.clone();

                if completion.kind == GateCompletionKind::Preflight {
                    if !completion.passed {
                        finish_gate_claim(
                            &mut attempt_ownership,
                            &mut owned_gate_claim,
                            false,
                        );
                        task_dag.clear_running(
                            &completion_attempt.plan_id,
                            &completion_attempt.task_id,
                        );
                        debug!(
                            attempt = %completion_attempt.key(),
                            duration_ms = completion.duration_ms,
                            "preflight failed; exact attempt is eligible for agent dispatch"
                        );
                        save_snapshot(
                            config, &executor, &paths, &mut state, &merge_queue,
                            &gate_thresholds, &snapshot_writer,
                        );
                        continue;
                    }
                    if let Some(task) = task_index
                        .get(&completion.plan_id)
                        .and_then(|tasks| tasks.get(&completion.task_id))
                    {
                        let role = task.role.as_deref().unwrap_or("implementer");
                        sink.task_started(
                            &completion.plan_id,
                            &completion.task_id,
                            role,
                            &task.title,
                            completion_attempt.attempt,
                        );
                        tui.task_started(
                            &completion.plan_id,
                            &completion.task_id,
                            &task.title,
                            "verifying",
                        );
                        let run_id = state.run_id().to_string();
                        emit_runner_event(
                            &paths,
                            &mut state,
                            &tui,
                            config,
                            RunnerEvent::task_attempt_started(
                                &run_id,
                                completion_attempt.clone(),
                                &task.title,
                            ),
                        );
                    }
                    match advance_preflight_success_to_gate(
                        &mut executor,
                        &completion.plan_id,
                    ) {
                        Ok(_) => {
                            info!(attempt = %completion_attempt.key(),
                                "preflight passes; skipping implementation agent");
                            completion.kind = GateCompletionKind::Gate;
                        }
                        Err(err) => {
                            let message = format!("preflight transition failed: {err}");
                            finish_gate_claim(
                                &mut attempt_ownership,
                                &mut owned_gate_claim,
                                false,
                            );
                            let _ = executor.apply_event(
                                &completion.plan_id,
                                &ExecutorEvent::Fatal(message.clone()),
                            );
                            tui.error(&message);
                            continue;
                        }
                    }
                }

                for v in &completion.verdicts {
                    tui.gate_result(
                        &completion.plan_id,
                        &completion.task_id,
                        &v.gate_name,
                        v.passed,
                    );

                    // Emit gate verdict metric.
                    if let Some(ref metrics) = config.metrics {
                        let verdict_str = if v.passed { "pass" } else { "fail" };
                        let labels = roko_core::obs::metrics::LabelSet::from_pairs(&[
                            (roko_core::obs::schema::LABEL_GATE, &v.gate_name),
                            (roko_core::obs::schema::LABEL_VERDICT, verdict_str),
                        ]);
                        metrics
                            .register_counter(
                                roko_core::obs::schema::ROKO_GATE_VERDICTS_TOTAL,
                                "Verify verdicts, by gate and verdict",
                                labels,
                            )
                            .inc();
                    }
                }

                // Render gate verdicts through the output sink.
                if completion.kind == GateCompletionKind::Gate {
                    for v in &completion.verdicts {
                        sink.gate_result(
                            &completion.plan_id,
                            &completion.task_id,
                            &super::output_sink::GateResultSummary {
                                rung: completion.rung,
                                passed: v.passed,
                                gate_name: v.gate_name.clone(),
                                summary: v.summary.clone(),
                                duration_ms: completion.duration_ms,
                            },
                        );
                    }
                }
                if completion.kind == GateCompletionKind::Gate {
                    if let Some(plan_state) = executor.plan_state_mut(&completion.plan_id) {
                        for verdict in &completion.verdicts {
                            plan_state.gate_results.push(GateResult {
                                gate_name: verdict.gate_name.clone(),
                                rung: completion.rung,
                                passed: verdict.passed,
                                summary: verdict.summary.clone(),
                                duration_ms: completion.duration_ms,
                                test_count: None,
                            });
                        }
                    }
                }
                let run_id = state.run_id().to_string();
                emit_runner_event(
                    &paths,
                    &mut state,
                    &tui,
                    config,
                    RunnerEvent::gate_completed(
                        &run_id,
                        completion_attempt.clone(),
                        &completion,
                    ),
                );
                record_daimon_gate_result(config, &completion);

                // Record gate outcome in the run ledger.
                if let Some(ref mut ledger) = run_ledger {
                    for verdict in &completion.verdicts {
                        ledger.record_gate_run(
                            &verdict.gate_name,
                            verdict.passed,
                            Some(verdict.summary.clone()),
                            completion.duration_ms,
                        );
                    }
                    append_ledger_entry(
                        &paths.run_ledger_jsonl,
                        "gate_outcome",
                        &serde_json::json!({
                            "plan_id": completion.plan_id,
                            "task_id": completion.task_id,
                            "rung": completion.rung,
                            "passed": completion.passed,
                            "duration_ms": completion.duration_ms,
                            "gate_kind": format!("{:?}", completion.kind),
                        }),
                    );
                }

                if completion.kind == GateCompletionKind::Merge {
                    emit_runner_event(
                        &paths,
                        &mut state,
                        &tui,
                        config,
                        RunnerEvent::merge_backend_completed(
                            &run_id,
                            completion_attempt.clone(),
                            &completion,
                            merge_branch_from_task_id(&completion.task_id),
                            conflict_paths_from_merge_output(&completion.output),
                        ),
                    );
                }

                let retry_budget = config
                    .max_retries
                    .min(gate_thresholds.suggested_max_retries(completion.rung));

                update_gate_thresholds(
                    &mut gate_thresholds,
                    completion.rung,
                    completion.passed,
                );
                emit_gate_thresholds_event(&gate_thresholds, &tui);

                // Publish gate result to the learning event bus so the
                // background subscriber can update VerdictHistory and
                // CalibrationPolicy.
                learning_event_bus.publish(
                    roko_learn::events::AgentEvent::GateResult {
                        gate_name: format!("rung-{}", completion.rung),
                        passed: completion.passed,
                        score: if completion.passed { 1.0 } else { 0.0 },
                        duration_ms: completion.duration_ms,
                    },
                );

                // Append gate verdict to signals.jsonl for audit / replay.
                {
                    let verdict_json = serde_json::json!({
                        "kind": "GateVerdict",
                        "plan_id": completion.plan_id,
                        "task_id": completion.task_id,
                        "rung": completion.rung,
                        "passed": completion.passed,
                        "gate_kind": format!("{:?}", completion.kind),
                        "duration_ms": completion.duration_ms,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    let signals_path = config.layout.signals_path();
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&signals_path)
                    {
                        use std::io::Write;
                        let _ = writeln!(f, "{}", verdict_json);
                    }
                }

                // Extension: on_gate hook.
                fire_on_gate_hook(config, &completion, &tui).await;

                if completion.kind == GateCompletionKind::Merge {
                    handle_merge_completion(
                        &completion,
                        &mut executor,
                        &mut state,
                        &paths,
                        &merge_queue,
                        &gate_tx,
                        &config.workdir,
                        gate_timeout(&config, 0),
                        &tui,
                        config,
                        &gate_thresholds,
                        &snapshot_writer,
                    );
                    continue;
                }

                if completion.kind == GateCompletionKind::PlanVerify {
                    finish_gate_claim(
                        &mut attempt_ownership,
                        &mut owned_gate_claim,
                        false,
                    );
                    handle_plan_verify_completion(
                        &completion,
                        &mut executor,
                        &mut state,
                        &paths,
                        &merge_queue,
                        &tui,
                        config,
                        &gate_thresholds,
                        &snapshot_writer,
                    );
                    continue;
                }

                if completion.passed && completion.rung < config.max_gate_rung {
                    state.clear_retry_backoff(&completion.plan_id);
                    info!(
                        plan_id = %completion.plan_id,
                        task_id = %completion.task_id,
                        rung = completion.rung,
                        max_gate_rung = config.max_gate_rung,
                        "gate rung passed — advancing to next configured rung"
                    );
                    finish_gate_claim(
                        &mut attempt_ownership,
                        &mut owned_gate_claim,
                        true,
                    );
                    continue;
                }

                // Final-rung pass, retry, and terminal failure all consume this
                // attempt's exact claim before ledger, DAG, or terminal effects.
                finish_gate_claim(
                    &mut attempt_ownership,
                    &mut owned_gate_claim,
                    false,
                );

                // ── SectionOutcome recording ────────────────────────────
                // Terminal gate result (final rung pass or any fail): join
                // the prompt section diagnostics captured at dispatch time
                // to the gate outcome so adaptive policy can learn which
                // sections correlate with success/failure.
                {
                    let attempt_key = completion_attempt.key();
                    if let Some(diag) = section_diagnostics.remove(&attempt_key) {
                        let status = if completion.passed {
                            SectionOutcomeStatus::Passed
                        } else {
                            SectionOutcomeStatus::Failed
                        };
                        let records = build_section_outcome_records(
                            &completion.plan_id,
                            &completion.task_id,
                            &state.agent_model,
                            &state.agent_provider,
                            status,
                            &diag,
                            &completion.verdicts,
                        );
                        let outcomes_path =
                            persist::section_outcomes_path(&config.workdir);
                        feedback_tasks.spawn(append_section_outcomes(
                            outcomes_path,
                            records,
                        ));
                    }
                }

                // ── Playbook outcome recording ───────────────────────────
                // Record gate pass/fail against every playbook that was
                // injected into this task's prompt. This closes the feedback
                // loop so PlaybookStore can learn which playbooks correlate
                // with task success and adaptive selection improves over time.
                {
                    let attempt_key = completion_attempt.key();
                    if let Some(pb_ids) = task_playbook_ids.remove(&attempt_key) {
                        let store = playbook_store.clone();
                        let gate_passed = completion.passed;
                        feedback_tasks.spawn(async move {
                            for pb_id in &pb_ids {
                                match store.record_outcome(pb_id, gate_passed).await {
                                    Ok(true) => {
                                        debug!(
                                            playbook_id = %pb_id,
                                            passed = gate_passed,
                                            "playbook outcome recorded"
                                        );
                                    }
                                    Ok(false) => {
                                        debug!(
                                            playbook_id = %pb_id,
                                            "playbook not found for outcome recording"
                                        );
                                    }
                                    Err(err) => {
                                        warn!(
                                            playbook_id = %pb_id,
                                            error = %err,
                                            "failed to record playbook outcome"
                                        );
                                    }
                                }
                            }
                        });
                    }
                }

                if completion.passed && completion.task_id.is_empty() {
                    // The post-implementation gate is plan-level: it proves the
                    // current aggregate worktree, not an individual task. Do not
                    // count it as another completed task.
                    state.clear_retry_backoff(&completion.plan_id);
                    match executor.apply_event(&completion.plan_id, &ExecutorEvent::GatePassed) {
                        Ok(phase) => {
                            info!(
                                plan_id = %completion.plan_id,
                                phase = ?phase,
                                "plan gate passed — running plan verify"
                            );
                        }
                        Err(err) => {
                            error!(
                                plan_id = %completion.plan_id,
                                error = %err,
                                "plan gate transition failed"
                            );
                            let _ = executor.apply_event(
                                &completion.plan_id,
                                &ExecutorEvent::Fatal(format!("plan gate transition failed: {err}")),
                            );
                        }
                    }

                    // Queue dream consolidation only when this run actually
                    // spawned agents. Verification-only runs do not create new
                    // agent episodes, and blocking after run.completed on old
                    // episodes makes no-op reruns look stuck.
                    if state.total_agent_calls > 0 {
                        dream_completion_pending = true;
                        debug!("dream consolidation queued after plan gate completion");
                    } else {
                        debug!("dream consolidation skipped after verification-only plan gate");
                    }
                } else if completion.passed {
                    state.clear_retry_backoff(&completion.plan_id);
                    let task_workdir = tracked_plan_workdir(&worktrees, &completion.plan_id);
                    let task_declared_files = task_index
                        .get(completion.plan_id.as_str())
                        .and_then(|tasks| tasks.get(completion.task_id.as_str()))
                        .map(|task| task.files.clone())
                        .unwrap_or_default();
                    let terminalized = terminalize_passed_task(
                        &paths,
                        &mut state,
                        &mut task_dag,
                        &task_index,
                        &mut run_ledger,
                        &tui,
                        sink,
                        config,
                        &completion,
                        &completion_attempt,
                        task_workdir.as_deref(),
                        &task_declared_files,
                    );
                    if matches!(terminalized, TaskTerminalization::AlreadyRecorded) {
                        debug!(
                            plan_id = %completion.plan_id,
                            task_id = %completion.task_id,
                            attempt = completion_attempt.attempt,
                            "duplicate task terminalization ignored"
                        );
                        finish_gate_claim(
                            &mut attempt_ownership,
                            &mut owned_gate_claim,
                            false,
                        );
                        cleanup_finished_task_gate(
                            &mut pending_gate_tasks,
                            &mut task_runtime_states,
                            &mut executor,
                            &completion,
                        );
                        continue;
                    }
                    if let TaskTerminalization::PersistenceFailed { reason } = terminalized {
                        warn!(
                            plan_id = %completion.plan_id,
                            task_id = %completion.task_id,
                            reason = %reason,
                            "task terminalized as failed because durable completion could not be recorded"
                        );
                        let has_runnable = !ready_tasks_for_plan(
                            &task_dag,
                            &executor,
                            &task_index,
                            &state,
                            &completion.plan_id,
                        )
                        .is_empty();
                        if has_runnable {
                            if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                                ps.gate_results.clear();
                                ps.current_phase = PlanPhase::Implementing;
                            }
                        } else if let Err(err) = executor.apply_event(
                            &completion.plan_id,
                            &ExecutorEvent::Fatal(reason.clone()),
                        ) {
                            warn!(
                                plan_id = %completion.plan_id,
                                error = %err,
                                "failed to apply Fatal event -- forcing plan terminal"
                            );
                            state.force_plan_terminal(&completion.plan_id);
                        }
                        finish_gate_claim(
                            &mut attempt_ownership,
                            &mut owned_gate_claim,
                            false,
                        );
                        cleanup_finished_task_gate(
                            &mut pending_gate_tasks,
                            &mut task_runtime_states,
                            &mut executor,
                            &completion,
                        );
                        continue;
                    }
                    let ready =
                        ready_tasks_for_plan(&task_dag, &executor, &task_index, &state, &completion.plan_id);
                    let has_more = !ready.is_empty();

                    if has_more {
                        // More tasks remain — force plan back to Implementing so
                        // the next tick resolves the next ready task.
                        if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                            ps.gate_results.clear();
                            ps.current_phase = PlanPhase::Implementing;
                        }
                        let remaining = task_index.get(completion.plan_id.as_str())
                            .map(|t| t.len().saturating_sub(state.plan_completed_tasks(&completion.plan_id).len() + state.plan_failed_tasks(&completion.plan_id).len())).unwrap_or(0);
                        info!(
                            plan_id = %completion.plan_id,
                            remaining,
                            "task passed — advancing to next task"
                        );
                    } else {
                        let progress = dag_progress_for_plan(
                            &task_dag,
                            &executor,
                            &task_index,
                            &state,
                            &completion.plan_id,
                        );
                        let skipped = task_dag.mark_blocked_tasks_skipped(
                            &completion.plan_id,
                            &progress.blocked_tasks,
                        );
                        if !skipped.is_empty() {
                            debug!(
                                plan_id = %completion.plan_id,
                                skipped = ?skipped,
                                "DAG quiescence propagated blocked tasks"
                            );
                        }
                        if progress.can_make_future_progress() {
                            if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                                ps.gate_results.clear();
                                ps.current_phase = PlanPhase::Implementing;
                            }
                            info!(
                                plan_id = %completion.plan_id,
                                "task passed — waiting on blocked DAG dependencies"
                            );
                        } else if dag_plan_has_failures(&task_dag, &state, &completion.plan_id)
                            || progress.blocked > 0
                        {
                            let reason = dag_quiescence_reason(&completion.plan_id, &progress);
                            warn!(plan_id = %completion.plan_id, reason = %reason, "DAG quiesced with no future progress");
                            if let Err(err) = executor.apply_event(
                                &completion.plan_id,
                                &ExecutorEvent::Fatal(reason.clone()),
                            )
                            {
                                warn!(
                                    plan_id = %completion.plan_id,
                                    error = %err,
                                    "failed to apply Fatal event -- forcing plan terminal"
                                );
                                state.force_plan_terminal(&completion.plan_id);
                            }
                            tui.error(&reason);
                        } else {
                            // All tasks done — run the plan-level verify chain.
                            let _ = executor
                                .apply_event(&completion.plan_id, &ExecutorEvent::GatePassed);
                            info!(plan_id = %completion.plan_id, "all tasks passed — running plan verify");

                            // Queue dream consolidation only when this run actually
                            // spawned agents. Verification-only runs do not create
                            // new agent episodes, and blocking after run.completed
                            // on old episodes makes no-op reruns look stuck.
                            if state.total_agent_calls > 0 {
                                dream_completion_pending = true;
                                debug!("dream consolidation queued after plan completion");
                            } else {
                                debug!(
                                    "dream consolidation skipped after verification-only plan completion"
                                );
                            }
                        }
                    }
                } else {
                    let failure_kind = completion
                        .failure_kind
                        .unwrap_or_else(|| RunnerFailureKind::from_output(&completion.output));
                    let retry_phase_open = executor
                        .plan_state(&completion.plan_id)
                        .map(|ps| ps.current_phase.kind() == PhaseKind::Gating)
                        .unwrap_or(false);
                    let decision_budget = if retry_phase_open { retry_budget } else { 0 };
                    let decision_probe = RetryDecision::for_failure(
                        failure_kind,
                        completion_attempt.attempt,
                        decision_budget,
                        "",
                    );
                    let decision_reason = if decision_probe.should_retry() {
                        "gate failed and retry policy allows auto-fix".to_string()
                    } else if decision_probe.retryable {
                        format!("gate failed and retries exhausted: {}", completion.output)
                    } else {
                        format!(
                            "gate failed with non-retryable {failure_kind:?} failure: {}",
                            completion.output
                        )
                    };
                    let mut decision = RetryDecision::for_failure(
                        failure_kind,
                        completion_attempt.attempt,
                        decision_budget,
                        decision_reason,
                    );
                    let retry_started = if decision.should_retry() {
                        match executor.apply_event(&completion.plan_id, &ExecutorEvent::GateFailed)
                        {
                            Ok(phase) => {
                                let failed_attempt = decision.current_attempt;
                                if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                                    ps.reset_for_retry();
                                    task_dag.clear_running(
                                        &completion.plan_id,
                                        &completion.task_id,
                                    );
                                    let next_attempt = decision.next_attempt.unwrap_or_else(|| {
                                        decision.current_attempt.saturating_add(1)
                                    });
                                    ps.iteration = next_attempt;
                                    state.set_iteration(
                                        &completion.plan_id,
                                        &completion.task_id,
                                        next_attempt,
                                    );
                                }
                                state.set_retry_backoff_from_decision(
                                    &completion.plan_id,
                                    &decision,
                                );
                                let run_id = state.run_id().to_string();
                                emit_runner_event(
                                    &paths,
                                    &mut state,
                                    &tui,
                                    config,
                                    RunnerEvent::retry_decision(
                                        &run_id,
                                        completion_attempt.clone(),
                                        decision.clone(),
                                    ),
                                );
                                tui.phase_transition(&completion.plan_id, "gating", &format!("{phase:?}"));

                                sink.gate_retry(
                                    &completion.plan_id,
                                    &completion.task_id,
                                    decision.next_attempt.unwrap_or_else(|| {
                                        state.iteration_for(&completion.plan_id, &completion.task_id)
                                    }),
                                    decision.cooldown_ms,
                                );

                                info!(
                                    plan_id = %completion.plan_id,
                                    phase = ?phase,
                                    failure_kind = ?failure_kind,
                                    next_attempt = ?decision.next_attempt,
                                    "gate failed — entering auto-fix"
                                );

                                // Enrich every retry prompt with failure context so the
                                // agent understands what went wrong and can adjust.
                                {
                                    let attempt_num = decision.next_attempt.unwrap_or_else(|| {
                                        state.iteration_for(&completion.plan_id, &completion.task_id)
                                    });
                                    let mut replan_context = build_gate_retry_context(
                                        &completion.output,
                                        &state.agent_output,
                                        attempt_num,
                                    );

                                    // Append lessons from past post-gate reflections.
                                    let gate_name = completion
                                        .verdicts
                                        .iter()
                                        .find(|v| !v.passed)
                                        .map(|v| v.gate_name.as_str())
                                        .unwrap_or("unknown");
                                    let lessons = lessons_from_post_gate_reflections(
                                        &config.layout.learn_dir(),
                                        gate_name,
                                        &completion.task_id,
                                    );
                                    if !lessons.is_empty() {
                                        replan_context.push_str(
                                            "\n\n### Lessons from past failures on this gate\n",
                                        );
                                        for lesson in &lessons {
                                            replan_context
                                                .push_str(&format!("- {lesson}\n"));
                                        }
                                        debug!(
                                            gate = %gate_name,
                                            lessons = lessons.len(),
                                            "post-gate reflection lessons added to retry prompt"
                                        );
                                    }

                                    maybe_apply_gate_failure_plan_revision(
                                        config,
                                        &paths,
                                        &mut state,
                                        &mut task_index,
                                        &completion.plan_id,
                                        &completion.task_id,
                                        failed_attempt,
                                        &completion.verdicts,
                                        &completion.output,
                                        &replan_context,
                                    );
                                }

                                // Refresh prompt cache after gate failure — the
                                // agent may have written new episodes / knowledge
                                // that should inform the retry prompt.
                                prompt_cache = Arc::new(PromptCache::load(&config.workdir));
                                debug!("prompt cache refreshed after gate failure");
                                true
                            }
                            Err(e) => {
                                decision = RetryDecision::for_failure(
                                    failure_kind,
                                    completion_attempt.attempt,
                                    0,
                                    format!(
                                        "gate failure retry transition rejected: {e}; {}",
                                        completion.output
                                    ),
                                );
                                warn!(
                                    plan_id = %completion.plan_id,
                                    task_id = %completion.task_id,
                                    err = %e,
                                    "gate failure retry transition rejected; terminalizing attempt"
                                );
                                false
                            }
                        }
                    } else {
                        false
                    };
                    if !retry_started {
                        state.task_failed();
                        tui.task_completed(&completion.plan_id, &completion.task_id, "failed");
                        let reason = decision.reason.clone();
                        state.record_task_failure(&completion.plan_id, &completion.task_id, &reason);
                        // Record task failure in the run ledger.
                        if let Some(ref mut ledger) = run_ledger {
                            let now_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            ledger.record_agent_failed(
                                "implementer",
                                roko_runtime::EffectErrorKind::Unknown,
                                &reason,
                            );
                            append_ledger_entry(
                                &paths.run_ledger_jsonl,
                                "task_failed",
                                &serde_json::json!({
                                    "plan_id": completion.plan_id,
                                    "task_id": completion.task_id,
                                    "passed": false,
                                    "reason": reason,
                                    "duration_ms": completion.duration_ms,
                                    "timestamp_ms": now_ms,
                                }),
                            );
                        }

                        sink.task_failed(&completion.plan_id, &completion.task_id, &reason);
                        let run_id = state.run_id().to_string();
                        emit_runner_event(
                            &paths,
                            &mut state,
                            &tui,
                            config,
                            RunnerEvent::retry_decision(
                                &run_id,
                                completion_attempt.clone(),
                                decision.clone(),
                            ),
                        );
                        let run_id = state.run_id().to_string();
                        let agent_model = state.agent_model.clone();
                        let agent_provider = state.agent_provider.clone();
                        emit_runner_event(
                            &paths,
                            &mut state,
                            &tui,
                            config,
                            RunnerEvent::task_attempt_completed(
                                &run_id,
                                completion_attempt.clone(),
                                decision
                                    .terminal_outcome()
                                    .unwrap_or(TaskAttemptOutcome::Failed),
                                Some(failure_kind),
                                completion.duration_ms,
                                agent_model,
                                agent_provider,
                            ),
                        );
                        record_daimon_task_outcome(
                            config,
                            state.current_daimon_strategy,
                            &completion.plan_id,
                            &completion.task_id,
                            false,
                            &reason,
                        );

                        // Track this task as failed so dependents are skipped.
                        state.mark_task_failed(&completion.plan_id, &completion.task_id);
                        let task_refs = task_refs_for_plan(&task_index, &completion.plan_id);
                        let skipped = task_dag.mark_failed_blocking_downstream(
                            &completion.plan_id,
                            &completion.task_id,
                            &task_refs,
                        );
                        if !skipped.is_empty() {
                            debug!(
                                plan_id = %completion.plan_id,
                                task_id = %completion.task_id,
                                skipped = ?skipped,
                                "gate failure blocked downstream tasks"
                            );
                        }

                        let has_runnable = !ready_tasks_for_plan(
                            &task_dag,
                            &executor,
                            &task_index,
                            &state,
                            &completion.plan_id,
                        )
                        .is_empty();

                        if has_runnable {
                            // Keep the plan in Implementing so the next tick
                            // picks up the next independent task.
                            if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                                ps.gate_results.clear();
                                ps.current_phase = PlanPhase::Implementing;
                            }
                            warn!(
                                plan_id = %completion.plan_id,
                                task_id = %completion.task_id,
                                "task failed — skipping, other tasks remain"
                            );
                            tui.error(&format!(
                                "task {} failed (skipped) — continuing with remaining tasks",
                                completion.task_id
                            ));
                        } else {
                            let progress = dag_progress_for_plan(
                                &task_dag,
                                &executor,
                                &task_index,
                                &state,
                                &completion.plan_id,
                            );
                            let skipped = task_dag.mark_blocked_tasks_skipped(
                                &completion.plan_id,
                                &progress.blocked_tasks,
                            );
                            if !skipped.is_empty() {
                                debug!(
                                    plan_id = %completion.plan_id,
                                    skipped = ?skipped,
                                    "DAG quiescence propagated blocked tasks"
                                );
                            }
                            if progress.can_make_future_progress() {
                                if let Some(ps) = executor.plan_state_mut(&completion.plan_id) {
                                    ps.gate_results.clear();
                                    ps.current_phase = PlanPhase::Implementing;
                                }
                                warn!(
                                    plan_id = %completion.plan_id,
                                    task_id = %completion.task_id,
                                    "task failed — waiting on blocked DAG tasks"
                                );
                            } else {
                                // No more runnable tasks — fail the plan.
                                let quiescence_reason =
                                    dag_quiescence_reason(&completion.plan_id, &progress);
                                let fatal_reason = format!("{reason}; {quiescence_reason}");
                                let _ = executor.apply_event(
                                    &completion.plan_id,
                                    &ExecutorEvent::Fatal(fatal_reason.clone()),
                                );
                                tui.error(&fatal_reason);
                            }
                        }
                    }
                }

                cleanup_finished_task_gate(
                    &mut pending_gate_tasks,
                    &mut task_runtime_states,
                    &mut executor,
                    &completion,
                );

                finish_gate_claim(
                    &mut attempt_ownership,
                    &mut owned_gate_claim,
                    false,
                );
                save_snapshot(config, &executor, &paths, &mut state, &merge_queue, &gate_thresholds, &snapshot_writer);
            }

            // ─── Branch 3: Executor tick ────────────────────────────
            _ = tick_interval.tick() => {
                // Refresh prompt cache if stale (default 5 min).
                if prompt_cache.is_stale() {
                    prompt_cache = Arc::new(PromptCache::load(&config.workdir));
                    debug!("prompt cache refreshed (stale)");
                }
                let actions = executor.tick();
                for action in actions {
                    let t_dispatch = Instant::now();
                    let action_label = match &action {
                        ExecutorAction::SpawnAgent { plan_id, task, .. } => {
                            format!("{plan_id}/{task}")
                        }
                        ExecutorAction::DispatchPlan { plan_id } => {
                            format!("{plan_id}/plan")
                        }
                        ExecutorAction::RunGate { plan_id, rung } => {
                            format!("{plan_id}/gate-{rung}")
                        }
                        _ => "other".to_string(),
                    };
                    let mut ctx = RunContext {
                        executor: &mut executor,
                        task_dag: &mut task_dag,
                        task_index: &task_index,
                        skip_enrichment: &skip_enrichment,
                        config,
                        sink,
                        tui: &tui,
                        state: &mut state,
                        attempt_ownership: &mut attempt_ownership,
                        pending_gate_tasks: &mut pending_gate_tasks,
                        agent_tx: &agent_tx,
                        gate_tx: &gate_tx,
                        fatal_tx: agent_tx.clone(),
                        paths: &paths,
                        merge_queue: &merge_queue,
                        worktrees: &worktrees,
                        gate_thresholds: &gate_thresholds,
                        snapshot_writer: &snapshot_writer,
                        prompt_cache: &prompt_cache,
                        factory: &factory,
                        task_sem: task_sem.clone(),
                        gate_sem: gate_sem.clone(),
                        task_runtime_states: &mut task_runtime_states,
                        legacy_gate_attempts: &mut legacy_gate_attempts,
                        preflight_attempted: &mut preflight_attempted,
                        section_diagnostics: &mut section_diagnostics,
                        task_playbook_ids: &mut task_playbook_ids,
                    };
                    let dispatch_outcome = dispatch_action(&action, &mut ctx).await;
                    let dispatch_ms = t_dispatch.elapsed().as_millis() as u64;
                    if let ActionDispatchOutcome::AgentStarted { plan_id, task_id } = dispatch_outcome {
                        ctx.state.last_dispatch_ms = dispatch_ms;
                        if let Some(runtime) = ctx
                            .task_runtime_states
                            .get_mut(&task_key(&plan_id, &task_id))
                        {
                            runtime.last_dispatch_ms = dispatch_ms;
                        }
                        let action_label = format!("{plan_id}/{task_id}");
                        info!(action = %action_label, dispatch_ms, "agent action dispatched");
                        // Record task start in the run ledger.
                        if let Some(ref mut ledger) = run_ledger {
                            let now_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            ledger.record_phase_transition(
                                roko_runtime::Phase::Pending,
                                roko_runtime::Phase::Implementing,
                                now_ms,
                            );
                            append_ledger_entry(
                                &paths.run_ledger_jsonl,
                                "task_started",
                                &serde_json::json!({
                                    "plan_id": plan_id,
                                    "task_id": task_id,
                                    "timestamp_ms": now_ms,
                                }),
                            );
                        }
                    } else if matches!(&action, ExecutorAction::SpawnAgent { .. }) {
                        debug!(action = %action_label, dispatch_ms, "agent action suppressed or delayed");
                    } else if dispatch_ms > 50 {
                        info!(action = %action_label, dispatch_ms, "action dispatched (slow)");
                    } else {
                        debug!(action = %action_label, dispatch_ms, "action dispatched");
                    }
                }
            }

            // ─── Branch 4: Periodic flush ───────────────────────────
            _ = flush_interval.tick() => {
                save_snapshot(config, &executor, &paths, &mut state, &merge_queue, &gate_thresholds, &snapshot_writer);
                {
                    let pids = attempt_ownership.surviving_agent_metadata().pids;
                    if !pids.is_empty() {
                        let _ = persist::save_agent_pids(&paths, &pids);
                    }
                }
            }

            // ─── Branch 5: Plan timeout ──────────────────────────────
            _ = &mut plan_timeout, if !timed_out => {
                handle_plan_timeout(
                    &executor,
                    &plans,
                    &mut state,
                    &mut attempt_ownership,
                    &paths,
                    &merge_queue,
                    &tui,
                    config,
                    &gate_thresholds,
                    &snapshot_writer,
                )
                .await?;
                timed_out = true;
            }

            // ─── Branch 6: Cancellation ─────────────────────────────
            _ = cancel.cancelled() => {
                warn!("cancellation requested — shutting down");
                stop_all_agents(&mut attempt_ownership, &mut state, Duration::from_secs(3)).await;
                save_snapshot(config, &executor, &paths, &mut state, &merge_queue, &gate_thresholds, &snapshot_writer);
                snapshot_writer.flush();
                shutdown_subsystems(config, &tui).await;
                let event =
                    build_run_completed_event(&executor, &plans, &state, RunOutcome::Cancelled);
                emit_runner_event(&paths, &mut state, &tui, config, event);
                break;
            }
        }

        if !timed_out && tokio::time::Instant::now() >= plan_deadline {
            handle_plan_timeout(
                &executor,
                &plans,
                &mut state,
                &mut attempt_ownership,
                &paths,
                &merge_queue,
                &tui,
                config,
                &gate_thresholds,
                &snapshot_writer,
            )
            .await?;
            timed_out = true;
        }

        if all_plans_terminal(&executor, &plans) {
            save_snapshot(
                config,
                &executor,
                &paths,
                &mut state,
                &merge_queue,
                &gate_thresholds,
                &snapshot_writer,
            );
            let final_report = build_report(&executor, &plans, &state);
            let outcome = if final_report.all_succeeded() {
                RunOutcome::Succeeded
            } else {
                RunOutcome::Failed
            };
            let event = build_run_completed_event(&executor, &plans, &state, outcome);
            emit_runner_event(&paths, &mut state, &tui, config, event);
            let cost_display = format!("{:.4}", final_report.total_cost_usd);
            info!(
                outcome = ?outcome,
                total_tasks = final_report.total_tasks,
                completed = final_report.tasks_completed,
                failed = final_report.tasks_failed,
                cost_usd = %cost_display,
                tokens_in = final_report.total_tokens_in,
                tokens_out = final_report.total_tokens_out,
                agent_calls = final_report.total_agent_calls,
                duration_secs = final_report.duration.as_secs(),
                "run complete — exiting event loop"
            );
            for plan_report in &final_report.plans {
                info!(
                    plan_id = %plan_report.plan_id,
                    completed = plan_report.completed,
                    tasks_done = plan_report.tasks_completed,
                    tasks_total = plan_report.tasks_total,
                    tasks_failed = plan_report.tasks_failed,
                    "plan summary"
                );
            }
            // Persist the run ledger at run completion.
            persist_run_ledger(&run_ledger, &paths.run_ledger_jsonl);
            break;
        }
    }

    // Drain any pending feedback tasks.
    while feedback_tasks.try_join_next().is_some() {}

    // Ensure all pending snapshots land on disk before returning.
    snapshot_writer.flush();

    // Persist the run ledger (final write on the general exit path).
    persist_run_ledger(&run_ledger, &paths.run_ledger_jsonl);

    let report = build_report(&executor, &plans, &state);

    // Shut down the learning subscriber after the event bus is closed so
    // pending turn events are flushed to `.roko/learn/efficiency.jsonl`.
    drop(learning_event_bus);
    if let Err(err) = learning_subscriber_handle.await {
        warn!(error = %err, "learning subscriber task failed during shutdown");
    }

    // Shutdown Phase 0 subsystems and persist learned state.
    shutdown_subsystems(config, &tui).await;

    if dream_completion_pending && !cancel.is_cancelled() {
        run_dream_consolidation_if_enabled(config).await;
    }

    // ── Post-run episode compaction ──────────────────────────────────
    //
    // Compact the episode log using the default retention policy.  This
    // runs after the main loop so it does not contend with the episode
    // sink appending new entries.
    compact_episodes_if_needed(&paths.episodes_jsonl).await;

    Ok(report)
}

fn apply_agent_completion(executor: &mut ParallelExecutor, plan_id: &str, tui: &TuiBridge) {
    let Some(phase_kind) = executor
        .plan_state(plan_id)
        .map(|state| state.current_phase.kind())
    else {
        warn!(plan_id = %plan_id, "agent completed for unknown plan");
        return;
    };

    let event = match phase_kind {
        PhaseKind::Enriching => ExecutorEvent::EnrichmentDone,
        PhaseKind::Implementing => ExecutorEvent::ImplementationDone,
        PhaseKind::AutoFixing => ExecutorEvent::AutoFixDone,
        PhaseKind::RegeneratingVerify => ExecutorEvent::VerifyRegenDone,
        PhaseKind::Reviewing => ExecutorEvent::ReviewApproved,
        PhaseKind::DocRevision => ExecutorEvent::DocRevisionDone,
        _ => {
            info!(
                plan_id = %plan_id,
                phase = ?phase_kind,
                "agent completion ignored for phase"
            );
            return;
        }
    };

    match executor.apply_event(plan_id, &event) {
        Ok(phase) => {
            tui.phase_transition(plan_id, &format!("{phase_kind:?}"), &format!("{phase:?}"));
            info!(plan_id = %plan_id, from = ?phase_kind, phase = ?phase, "agent phase completed");
        }
        Err(e) => {
            warn!(plan_id = %plan_id, err = %e, "transition error after agent completion");
        }
    }
}

fn advance_preflight_success_to_gate(
    executor: &mut ParallelExecutor,
    plan_id: &str,
) -> Result<PlanPhase, TransitionError> {
    let Some(phase_kind) = executor
        .plan_state(plan_id)
        .map(|state| state.current_phase.kind())
    else {
        return Err(TransitionError {
            from: PhaseKind::Queued,
            to: PhaseKind::Gating,
            reason: format!("plan '{plan_id}' not found"),
        });
    };

    match phase_kind {
        PhaseKind::Enriching => {
            executor.apply_event(plan_id, &ExecutorEvent::EnrichmentDone)?;
            executor.apply_event(plan_id, &ExecutorEvent::ImplementationDone)
        }
        PhaseKind::Implementing => {
            executor.apply_event(plan_id, &ExecutorEvent::ImplementationDone)
        }
        PhaseKind::Gating => executor
            .plan_state(plan_id)
            .map(|state| state.current_phase.clone())
            .ok_or_else(|| TransitionError {
                from: PhaseKind::Queued,
                to: PhaseKind::Gating,
                reason: format!("plan '{plan_id}' not found"),
            }),
        other => Err(TransitionError {
            from: other,
            to: PhaseKind::Gating,
            reason: format!("preflight success cannot advance from {other:?}"),
        }),
    }
}

fn take_matching_gate_attempt(
    attempts: &mut HashMap<String, TaskAttemptRef>,
    effect_key: &str,
    reported: Option<&TaskAttemptRef>,
) -> Option<TaskAttemptRef> {
    let reported = reported?;
    if attempts.get(effect_key) != Some(reported) {
        return None;
    }
    attempts.remove(effect_key)
}

fn no_ready_spawn_event(phase_kind: PhaseKind, requested_task: &str) -> Option<ExecutorEvent> {
    match phase_kind {
        PhaseKind::Implementing => Some(ExecutorEvent::ImplementationDone),
        PhaseKind::Complete | PhaseKind::Done | PhaseKind::Failed | PhaseKind::Skipped => None,
        _ => Some(ExecutorEvent::Fatal(format!(
            "agent spawn requested for {requested_task:?} while plan is in {phase_kind:?}, but no runnable task was available"
        ))),
    }
}

fn turn_exceeds_budget(total_cost_usd: Option<f64>, max_turn_usd: f64) -> bool {
    max_turn_usd > 0.0 && total_cost_usd.is_some_and(|cost| cost > max_turn_usd)
}

fn agent_failure_message(agent_output: &str) -> Option<String> {
    agent_output
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && (line.contains("API Error")
                    || line.to_ascii_lowercase().contains("error")
                    || line.to_ascii_lowercase().contains("failed"))
        })
        .map(ToOwned::to_owned)
}

fn build_agent_retry_context(message: &str, agent_output: &str, attempt_num: u32) -> String {
    let agent_excerpt = if agent_output.len() > 3000 {
        &agent_output[..3000]
    } else {
        agent_output
    };
    format!(
        "## IMPORTANT: Your previous agent turn failed\n\n\
         Attempt {attempt_num} ended with an agent/runtime error.\n\n\
         ### Error\n```\n{message}\n```\n\n\
         ### Previous agent output\n```\n{agent_excerpt}\n```"
    )
}

fn handle_agent_failure(
    executor: &mut ParallelExecutor,
    task_dag: &mut TaskDag,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
    state: &mut RunState,
    paths: &PersistPaths,
    tui: &TuiBridge,
    sink: &dyn RunOutputSink,
    config: &RunConfig,
    message: String,
) {
    let plan_id = state.plan_id.clone();
    let task_id = state.current_task.clone();
    if plan_id.is_empty() || task_id.is_empty() {
        tui.error(&message);
        return;
    }

    let failure_text = format!("{message}\n{}", state.agent_output);
    let failure_kind = RunnerFailureKind::from_output(&failure_text);
    let retry_budget = config.max_retries;
    let retry_phase_open = executor
        .plan_state(&plan_id)
        .map(|ps| ps.current_phase.kind() == PhaseKind::Implementing)
        .unwrap_or(false);
    let attempt = state.current_attempt_ref();
    let run_id = state.run_id().to_string();
    let decision_budget = if retry_phase_open { retry_budget } else { 0 };
    let decision_probe =
        RetryDecision::for_failure(failure_kind, attempt.attempt, decision_budget, "");
    let decision_reason = if decision_probe.should_retry() {
        "agent turn failed and retry policy allows another attempt".to_string()
    } else if decision_probe.retryable {
        format!("agent turn failed and retries exhausted: {message}")
    } else {
        format!("agent turn failed with non-retryable {failure_kind:?} failure: {message}")
    };
    let decision = RetryDecision::for_failure(
        failure_kind,
        attempt.attempt,
        decision_budget,
        decision_reason,
    );

    if decision.should_retry() {
        if let Some(ps) = executor.plan_state_mut(&plan_id) {
            ps.reset_for_retry();
            ps.current_phase = PlanPhase::Implementing;
            task_dag.clear_running(&plan_id, &task_id);
            let next_attempt = decision
                .next_attempt
                .unwrap_or_else(|| decision.current_attempt.saturating_add(1));
            ps.iteration = next_attempt;
            state.set_iteration(&plan_id, &task_id, next_attempt);
        }

        state.set_retry_backoff_from_decision(&plan_id, &decision);
        let retry_attempt = decision
            .next_attempt
            .unwrap_or_else(|| state.iteration_for(&plan_id, &task_id));
        state.set_replan_context(
            &plan_id,
            &task_id,
            build_agent_retry_context(&message, &state.agent_output, retry_attempt),
        );
        emit_runner_event(
            paths,
            state,
            tui,
            config,
            RunnerEvent::retry_decision(&run_id, attempt, decision.clone()),
        );
        warn!(
            plan_id = %plan_id,
            task_id = %task_id,
            failure_kind = ?failure_kind,
            cooldown_ms = decision.cooldown_ms,
            "agent turn failed — retrying task after backoff"
        );
        tui.error(&format!(
            "agent turn failed for {task_id}; retrying after {}s",
            decision.cooldown_ms / 1000
        ));
        return;
    }

    state.task_failed();
    let reason = decision.reason.clone();
    state.record_task_failure(&plan_id, &task_id, &reason);
    state.mark_task_failed(&plan_id, &task_id);
    let task_refs = task_refs_for_plan(task_index, &plan_id);
    let skipped = task_dag.mark_failed_blocking_downstream(&plan_id, &task_id, &task_refs);
    if !skipped.is_empty() {
        debug!(
            plan_id = %plan_id,
            task_id = %task_id,
            skipped = ?skipped,
            "agent failure blocked downstream tasks"
        );
    }
    sink.task_failed(&plan_id, &task_id, &reason);
    tui.task_completed(&plan_id, &task_id, "failed");

    append_ledger_entry(
        &paths.run_ledger_jsonl,
        "task_failed",
        &serde_json::json!({
            "plan_id": &plan_id,
            "task_id": &task_id,
            "passed": false,
            "reason": &reason,
            "timestamp_ms": chrono::Utc::now().timestamp_millis().max(0) as u64,
        }),
    );

    emit_runner_event(
        paths,
        state,
        tui,
        config,
        RunnerEvent::retry_decision(&run_id, attempt.clone(), decision.clone()),
    );
    let agent_model = state.agent_model.clone();
    let agent_provider = state.agent_provider.clone();
    emit_runner_event(
        paths,
        state,
        tui,
        config,
        RunnerEvent::task_attempt_completed(
            &run_id,
            attempt,
            decision
                .terminal_outcome()
                .unwrap_or(TaskAttemptOutcome::Failed),
            Some(failure_kind),
            state.task_elapsed_ms(),
            agent_model,
            agent_provider,
        ),
    );
    record_daimon_task_outcome(
        config,
        state.current_daimon_strategy,
        &plan_id,
        &task_id,
        false,
        &reason,
    );

    let has_runnable =
        !ready_tasks_for_plan(task_dag, executor, task_index, state, &plan_id).is_empty();

    if has_runnable {
        if let Some(ps) = executor.plan_state_mut(&plan_id) {
            ps.gate_results.clear();
            ps.current_phase = PlanPhase::Implementing;
        }
        warn!(
            plan_id = %plan_id,
            task_id = %task_id,
            "agent failed task — continuing with remaining independent tasks"
        );
        tui.error(&format!(
            "task {task_id} failed after agent error — continuing with remaining tasks"
        ));
    } else {
        let progress = dag_progress_for_plan(task_dag, executor, task_index, state, &plan_id);
        let skipped = task_dag.mark_blocked_tasks_skipped(&plan_id, &progress.blocked_tasks);
        if !skipped.is_empty() {
            debug!(
                plan_id = %plan_id,
                skipped = ?skipped,
                "DAG quiescence propagated blocked tasks"
            );
        }
        if progress.can_make_future_progress() {
            if let Some(ps) = executor.plan_state_mut(&plan_id) {
                ps.gate_results.clear();
                ps.current_phase = PlanPhase::Implementing;
            }
            warn!(
                plan_id = %plan_id,
                task_id = %task_id,
                "agent failed task — waiting on blocked DAG tasks"
            );
        } else {
            let quiescence_reason = dag_quiescence_reason(&plan_id, &progress);
            let fatal_reason = format!("{reason}; {quiescence_reason}");
            if let Err(err) =
                executor.apply_event(&plan_id, &ExecutorEvent::Fatal(fatal_reason.clone()))
            {
                error!(plan_id = %plan_id, error = %err, "failed to apply Fatal event after agent failure");
                state.force_plan_terminal(&plan_id);
                tui.error(&fatal_reason);
            } else {
                tui.error(&fatal_reason);
            }
        }
    }
}

fn complete_plan_after_successful_verify(
    plan_id: &str,
    executor: &mut ParallelExecutor,
) -> Result<PlanPhase, TransitionError> {
    let mut phase = match executor.plan_state(plan_id) {
        Some(state) if matches!(state.current_phase, PlanPhase::Complete) => {
            return Ok(PlanPhase::Complete);
        }
        Some(state) if state.current_phase.kind() == PhaseKind::Verifying => {
            executor.apply_event(plan_id, &ExecutorEvent::VerifyPassed)?
        }
        Some(state) if state.current_phase.kind() == PhaseKind::Reviewing => {
            state.current_phase.clone()
        }
        Some(state) if state.current_phase.kind() == PhaseKind::DocRevision => {
            state.current_phase.clone()
        }
        Some(state) if state.current_phase.kind() == PhaseKind::Merging => {
            state.current_phase.clone()
        }
        Some(state) => {
            let from = state.current_phase.kind();
            return Err(TransitionError {
                from,
                to: PhaseKind::Complete,
                reason: format!("cannot complete verified plan from unexpected phase {from:?}"),
            });
        }
        None => {
            return Err(TransitionError {
                from: PhaseKind::Queued,
                to: PhaseKind::Complete,
                reason: format!("plan '{plan_id}' not found"),
            });
        }
    };

    if phase.kind() == PhaseKind::Reviewing {
        phase = executor.apply_event(plan_id, &ExecutorEvent::ReviewApproved)?;
    }
    if phase.kind() == PhaseKind::DocRevision {
        phase = executor.apply_event(plan_id, &ExecutorEvent::DocRevisionDone)?;
    }
    if phase.kind() == PhaseKind::Merging {
        phase = executor.apply_event(plan_id, &ExecutorEvent::MergeSucceeded)?;
    }

    Ok(phase)
}

fn complete_verified_plan_success(
    plan_id: &str,
    executor: &mut ParallelExecutor,
    state: &mut RunState,
    paths: &PersistPaths,
    tui: &TuiBridge,
    config: &RunConfig,
) -> Result<PlanPhase, TransitionError> {
    let was_complete = executor
        .plan_state(plan_id)
        .is_some_and(|state| matches!(state.current_phase, PlanPhase::Complete));
    let phase = complete_plan_after_successful_verify(plan_id, executor)?;
    if !was_complete {
        tui.plan_completed(plan_id, true);
        let run_id = state.run_id().to_string();
        emit_runner_event(
            paths,
            state,
            tui,
            config,
            RunnerEvent::plan_completed(&run_id, plan_id, PlanOutcome::Succeeded, None),
        );
    }
    Ok(phase)
}

fn handle_plan_verify_completion(
    completion: &GateCompletion,
    executor: &mut ParallelExecutor,
    state: &mut RunState,
    paths: &PersistPaths,
    merge_queue: &MergeQueue,
    tui: &TuiBridge,
    config: &RunConfig,
    gate_thresholds: &GateThresholds,
    writer: &SnapshotWriter,
) {
    if completion.passed {
        state.clear_retry_backoff(&completion.plan_id);
        match complete_verified_plan_success(
            &completion.plan_id,
            executor,
            state,
            paths,
            tui,
            config,
        ) {
            Ok(phase) => {
                tui.phase_transition(&completion.plan_id, "verifying", &format!("{phase:?}"));
                info!(plan_id = %completion.plan_id, phase = ?phase, "plan verify passed — plan complete");
            }
            Err(e) => {
                warn!(
                    plan_id = %completion.plan_id,
                    err = %e,
                    "transition error while completing plan after verify pass"
                );
                let _ = executor.apply_event(
                    &completion.plan_id,
                    &ExecutorEvent::Fatal(format!("plan verify transition failed: {e}")),
                );
            }
        }
    } else {
        let failure_kind = completion
            .failure_kind
            .unwrap_or_else(|| RunnerFailureKind::from_output(&completion.output));
        let run_id = state.run_id().to_string();
        let attempt = TaskAttemptRef::new(
            completion.plan_id.clone(),
            completion.task_id.clone(),
            state.iteration_for(&completion.plan_id, &completion.task_id),
        );
        let decision = RetryDecision::for_failure(
            failure_kind,
            attempt.attempt,
            attempt.attempt,
            "plan verify failed and verify regeneration is available".to_string(),
        );
        state.set_retry_backoff_from_decision(&completion.plan_id, &decision);
        emit_runner_event(
            paths,
            state,
            tui,
            config,
            RunnerEvent::retry_decision(&run_id, attempt, decision),
        );
        match executor.apply_event(&completion.plan_id, &ExecutorEvent::VerifyFailed) {
            Ok(phase) => {
                tui.phase_transition(&completion.plan_id, "verifying", &format!("{phase:?}"));
                warn!(
                    plan_id = %completion.plan_id,
                    failure_kind = ?failure_kind,
                    phase = ?phase,
                    "plan verify failed"
                );
            }
            Err(e) => {
                let reason = format!("plan verify failed: {}", completion.output);
                warn!(
                    plan_id = %completion.plan_id,
                    err = %e,
                    "transition error after plan verify failure"
                );
                let _ = executor.apply_event(&completion.plan_id, &ExecutorEvent::Fatal(reason));
            }
        }
    }

    save_snapshot(
        config,
        executor,
        paths,
        state,
        merge_queue,
        gate_thresholds,
        writer,
    );
}

fn merge_branch_from_task_id(task_id: &str) -> Option<String> {
    task_id
        .strip_prefix("merge:")
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(ToOwned::to_owned)
}

fn conflict_paths_from_merge_output(output: &str) -> Vec<String> {
    output
        .lines()
        .find_map(|line| {
            line.split_once("conflicted paths:")
                .map(|(_, paths)| paths.to_string())
        })
        .map(|paths| {
            paths
                .split([',', ' ', '\t'])
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn handle_merge_completion(
    completion: &GateCompletion,
    executor: &mut ParallelExecutor,
    state: &mut RunState,
    paths: &PersistPaths,
    merge_queue: &MergeQueue,
    gate_tx: &mpsc::Sender<GateCompletion>,
    workdir: &Path,
    regression_timeout: Duration,
    tui: &TuiBridge,
    config: &RunConfig,
    gate_thresholds: &GateThresholds,
    writer: &SnapshotWriter,
) {
    let run_id = state.run_id().to_string();
    if completion.passed {
        match executor.apply_event(&completion.plan_id, &ExecutorEvent::MergeSucceeded) {
            Ok(phase) => {
                tui.phase_transition(&completion.plan_id, "merging", &format!("{phase:?}"));
                tui.plan_completed(&completion.plan_id, true);
                emit_runner_event(
                    paths,
                    state,
                    tui,
                    config,
                    RunnerEvent::plan_completed(
                        &run_id,
                        &completion.plan_id,
                        PlanOutcome::Succeeded,
                        None,
                    ),
                );
                info!(
                    plan_id = %completion.plan_id,
                    output = %completion.output,
                    "merge finalized and regression passed"
                );
            }
            Err(err) => {
                let reason = format!("executor rejected successful merge: {err}");
                let _ = executor.apply_event(&completion.plan_id, &ExecutorEvent::Fatal(reason));
            }
        }
    } else {
        let reason = format!("merge failed: {}", completion.output);
        match executor.apply_event(&completion.plan_id, &ExecutorEvent::MergeFailed) {
            Ok(phase) => {
                tui.phase_transition(&completion.plan_id, "merging", &format!("{phase:?}"));
                tui.plan_completed(&completion.plan_id, false);
            }
            Err(err) => {
                warn!(
                    plan_id = %completion.plan_id,
                    error = %err,
                    "transition error after merge failure"
                );
                let _ = executor
                    .apply_event(&completion.plan_id, &ExecutorEvent::Fatal(reason.clone()));
            }
        }
        emit_runner_event(
            paths,
            state,
            tui,
            config,
            RunnerEvent::plan_completed(
                &run_id,
                &completion.plan_id,
                PlanOutcome::Failed,
                Some(reason.clone()),
            ),
        );
        tui.error(&reason);
    }

    let merger = PlanMerger::new(
        merge_queue.clone(),
        PlanMergerConfig::new(workdir.to_path_buf(), regression_timeout),
    );
    if let Some(launch) = merger.drain_next() {
        let next_plan_id = launch.plan_id().to_string();
        start_legacy_merge(&merger, launch, gate_tx.clone());
        info!(plan_id = %next_plan_id, "started next queued merge");
    }
    save_snapshot(
        config,
        executor,
        paths,
        state,
        merge_queue,
        gate_thresholds,
        writer,
    );
}

fn start_legacy_merge(
    merger: &PlanMerger,
    launch: MergeLaunch,
    gate_tx: mpsc::Sender<GateCompletion>,
) {
    let producer = merger.prepare(launch, gate_tx);
    let _ = producer.start.send(());
    drop(producer.handle);
}

fn append_agent_event(paths: &PersistPaths, event: &AgentEvent, state: &RunState) {
    let event_type = event.event_type();

    let payload = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "timestamp_ms": chrono::Utc::now().timestamp_millis().max(0) as u64,
        "type": event_type,
        "run_id": state.run_id(),
        "plan_id": state.plan_id.clone(),
        "task_id": state.current_task.clone(),
        "attempt": state.iteration_for(&state.plan_id, &state.current_task),
        "agent_pid": state.agent_pid,
        "event": agent_event_json(event),
    });

    if let Err(err) = persist::append_jsonl(&paths.events_jsonl, &payload) {
        warn!(error = %err, "failed to append runner event");
    }
}

fn publish_learning_agent_event(
    bus: &roko_learn::events::EventBus,
    event: &AgentEvent,
    state: &RunState,
) {
    match event {
        AgentEvent::Started {
            provider, model, ..
        } => {
            bus.publish(roko_learn::events::AgentEvent::TurnStarted {
                task_id: learning_task_id(state),
                model: model.clone(),
                provider: provider.clone(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            });
        }
        AgentEvent::TurnCompleted {
            total_cost_usd,
            num_turns,
            is_error,
            ..
        } => {
            let finish_reason = if *is_error {
                roko_agent::chat_types::FinishReason::Error("agent reported an error".to_string())
            } else {
                roko_agent::chat_types::FinishReason::Stop
            };
            bus.publish(roko_learn::events::AgentEvent::TurnCompleted {
                turn: num_turns.unwrap_or(1),
                usage: roko_agent::Usage {
                    input_tokens: saturating_u32(state.tokens_in),
                    output_tokens: saturating_u32(state.tokens_out),
                    cache_read_tokens: saturating_u32(state.cache_read_tokens),
                    cache_create_tokens: saturating_u32(state.cache_write_tokens),
                    cost_usd: total_cost_usd.unwrap_or(state.cost_usd) as f32,
                    wall_ms: state.task_elapsed_ms(),
                },
                tool_call_count: 0,
                gate_passed: None,
                finish_reason,
            });
        }
        _ => {}
    }
}

fn learning_task_id(state: &RunState) -> String {
    if state.plan_id.is_empty() {
        state.current_task.clone()
    } else {
        let agent_call = state.total_agent_calls.max(1);
        format!("{}:{}:{}", state.plan_id, state.current_task, agent_call)
    }
}

fn saturating_u32(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

fn agent_event_json(event: &AgentEvent) -> serde_json::Value {
    match event {
        AgentEvent::Started {
            agent_id,
            provider,
            model,
            pid,
        } => serde_json::json!({
            "agent_id": agent_id,
            "provider": provider,
            "model": model,
            "pid": pid,
        }),
        AgentEvent::SystemInit { session_id, model } => {
            serde_json::json!({"session_id": session_id, "model": model})
        }
        AgentEvent::MessageDelta { text } => serde_json::json!({"text": text}),
        AgentEvent::ToolCall { id, name } => serde_json::json!({"id": id, "name": name}),
        AgentEvent::ToolOutput { id, output } => serde_json::json!({"id": id, "output": output}),
        AgentEvent::TokenUsage {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
        } => serde_json::json!({
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cache_read_tokens": cache_read_tokens,
            "cache_write_tokens": cache_write_tokens,
        }),
        AgentEvent::TurnCompleted {
            session_id,
            total_cost_usd,
            num_turns,
            is_error,
        } => serde_json::json!({
            "session_id": session_id,
            "total_cost_usd": total_cost_usd,
            "num_turns": num_turns,
            "is_error": is_error,
        }),
        AgentEvent::Error { message } => serde_json::json!({"message": message}),
        AgentEvent::Exited { exit_code } => serde_json::json!({"exit_code": exit_code}),
    }
}

/// Single emit path for runner lifecycle events.
///
/// Owns:
/// - state apply (`RunState::apply_runner_event`)
/// - TUI dashboard publish (`TuiBridge::runner_event`)
/// - durable JSONL append (`persist::append_runner_event`)
/// - **projection broadcast** (`config.projection`)
/// - **feedback fan-out** (`config.feedback_facade`, fire-and-forget)
///
/// Helpers that do not have `&RunConfig` in scope use
/// [`emit_runner_event_facadeless`] which is equivalent to passing
/// `None`/`None` for projection + feedback.
fn emit_runner_event(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    config: &RunConfig,
    event: RunnerEvent,
) {
    emit_runner_event_with_facades(
        paths,
        state,
        tui,
        config.projection.as_ref(),
        config.feedback_facade.as_ref(),
        config.http_event_sink.as_ref(),
        event,
        None,
    );
}

/// Drop-in for emit sites that do not hold a `&RunConfig` (helpers
/// invoked outside `run()`). Skips projection + feedback fan-out; the
/// runner-level emits still cover the lifecycle events these helpers
/// produce because the helpers themselves only emit on their plan's
/// completion which is also republished from `run()`.
fn emit_runner_event_facadeless(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    event: RunnerEvent,
) {
    emit_runner_event_with_facades(paths, state, tui, None, None, None, event, None);
}

/// Internal variant accepting the optional projection + feedback facades.
fn emit_runner_event_with_facades(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    projection: Option<&Arc<super::projection::Projection>>,
    feedback_facade: Option<&Arc<crate::runtime_feedback::FeedbackFacade>>,
    http_event_sink: Option<&HttpEventSink>,
    event: RunnerEvent,
    feedback_tasks: Option<&mut tokio::task::JoinSet<()>>,
) {
    state.apply_runner_event(&event);
    tui.runner_event(&event);
    if let Err(err) = persist::append_runner_event(paths, &event) {
        warn!(
            event_type = event.event_type(),
            error = %err,
            "failed to append runner lifecycle event"
        );
    }

    // ── Mirror to projection facade ─────────────────────────────────────
    if let Some(proj) = projection {
        let raw = super::projection::RawRuntimeEvent::Runner(event.clone());
        match proj.publish(raw) {
            Ok(()) => {}
            Err(super::projection::ProjectionError::NoSubscribers) => {
                // Publishing without live subscribers is normal during smoke
                // runs — the projection facade tracks the dropped-event
                // counter for diagnostics.
            }
        }
    }

    // ── Forward canonical RuntimeEvent over HTTP, if configured ─────────
    if let Some(sink) = http_event_sink {
        let runtime_event = runner_event_to_runtime_event(&event);
        sink.emit(runtime_event.clone());
    }

    // ── Translate to FeedbackEvent and fan out ──────────────────────────
    if let Some(facade) = feedback_facade {
        let usage = TaskUsageSnapshot {
            tokens_in: state.tokens_in,
            tokens_out: state.tokens_out,
            cost_usd: state.cost_usd,
            duration_ms: state.task_elapsed_ms(),
            prompt_text: (!state.current_prompt_text.trim().is_empty())
                .then(|| state.current_prompt_text.clone()),
            agent_output: state.agent_output.clone(),
        };
        if let Some(feedback) = runner_event_to_feedback(&event, &state.routing_context, &usage) {
            if let Some(tasks) = feedback_tasks {
                // Reap completed tasks (non-blocking) to prevent unbounded growth.
                while tasks.try_join_next().is_some() {}

                if tasks.len() >= 32 {
                    debug!(
                        "feedback task backlog full ({} tasks), dropping event",
                        tasks.len()
                    );
                } else {
                    let facade = Arc::clone(facade);
                    tasks.spawn(async move {
                        if let Err(err) = facade.on_event(&feedback).await {
                            warn!(
                                event_type = feedback.label(),
                                %err,
                                "feedback facade returned terminal error",
                            );
                        }
                    });
                }
            } else {
                // Fallback for callers that don't provide a JoinSet.
                let facade = Arc::clone(facade);
                tokio::spawn(async move {
                    if let Err(err) = facade.on_event(&feedback).await {
                        warn!(
                            event_type = feedback.label(),
                            %err,
                            "feedback facade returned terminal error",
                        );
                    }
                });
            }
        }
    }
}

fn runner_event_to_runtime_event(event: &RunnerEvent) -> RuntimeEvent {
    match event {
        RunnerEvent::RunStarted {
            run_id,
            plan_ids,
            total_tasks,
            resumed,
            ..
        } => RuntimeEvent::WorkflowStarted {
            run_id: run_id.clone(),
            template: "runner-v2".to_string(),
            prompt: format!(
                "plans={} total_tasks={} resumed={}",
                plan_ids.join(","),
                total_tasks,
                resumed
            ),
        },
        RunnerEvent::RunCompleted {
            run_id, outcome, ..
        } => RuntimeEvent::WorkflowCompleted {
            run_id: run_id.clone(),
            outcome: runtime_workflow_outcome(*outcome),
        },
        RunnerEvent::AgentDispatchStarted {
            run_id,
            agent_id,
            role,
            requested_model,
            ..
        } => RuntimeEvent::AgentSpawned {
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            role: role.clone(),
            model: requested_model.clone(),
        },
        RunnerEvent::AgentDispatchCompleted {
            run_id,
            agent_id,
            outcome:
                AgentDispatchOutcome::SpawnFailed
                | AgentDispatchOutcome::Failed
                | AgentDispatchOutcome::Exited,
            message,
            ..
        } => RuntimeEvent::AgentFailed {
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            error: message.clone().unwrap_or_else(|| event.message()),
        },
        RunnerEvent::AgentCompleted {
            run_id,
            agent_id,
            outcome,
            total_cost_usd,
            exit_code,
            message,
            ..
        } if agent_completion_succeeded(*outcome, *exit_code) => RuntimeEvent::AgentCompleted {
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            output: message.clone().unwrap_or_else(|| event.message()),
            tokens_used: 0,
            cost_usd: total_cost_usd.unwrap_or(0.0),
        },
        RunnerEvent::AgentCompleted {
            run_id,
            agent_id,
            message,
            ..
        } => RuntimeEvent::AgentFailed {
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            error: message.clone().unwrap_or_else(|| event.message()),
        },
        RunnerEvent::GateDispatchStarted {
            run_id,
            attempt,
            kind,
            rung,
            ..
        } => RuntimeEvent::GateStarted {
            run_id: run_id.clone(),
            gate_name: runtime_gate_name(*kind, attempt),
            rung: (*rung).min(u32::from(u8::MAX)) as u8,
        },
        RunnerEvent::GateCompleted {
            run_id,
            attempt,
            kind,
            passed: true,
            duration_ms,
            ..
        } => RuntimeEvent::GatePassed {
            run_id: run_id.clone(),
            gate_name: runtime_gate_name(*kind, attempt),
            duration_ms: *duration_ms,
        },
        RunnerEvent::GateCompleted {
            run_id,
            attempt,
            kind,
            output,
            duration_ms,
            ..
        } => RuntimeEvent::GateFailed {
            run_id: run_id.clone(),
            gate_name: runtime_gate_name(*kind, attempt),
            output: output.clone(),
            duration_ms: *duration_ms,
        },
        // ── Progress variants ────────────────────────────────────────────
        RunnerEvent::TaskAttemptStarted {
            run_id,
            attempt,
            title,
            ..
        } => RuntimeEvent::TaskStarted {
            run_id: run_id.clone(),
            plan_id: attempt.plan_id.clone(),
            task_id: attempt.task_id.clone(),
            task_title: title.clone(),
            role: String::new(),
        },
        RunnerEvent::TaskAttemptCompleted {
            run_id,
            attempt,
            outcome,
            duration_ms,
            ..
        } => RuntimeEvent::TaskCompleted {
            run_id: run_id.clone(),
            plan_id: attempt.plan_id.clone(),
            task_id: attempt.task_id.clone(),
            passed: matches!(outcome, TaskAttemptOutcome::Passed),
            duration_ms: *duration_ms,
        },
        RunnerEvent::PlanStarted {
            run_id, plan_id, ..
        } => RuntimeEvent::PipelinePhase {
            run_id: run_id.clone(),
            phase: plan_id.clone(),
            status: "started".to_string(),
        },
        RunnerEvent::PlanCompleted {
            run_id,
            plan_id,
            outcome,
            ..
        } => {
            let status = match outcome {
                PlanOutcome::Succeeded => "complete",
                PlanOutcome::Failed => "failed",
                PlanOutcome::Skipped => "skipped",
            };
            RuntimeEvent::PipelinePhase {
                run_id: run_id.clone(),
                phase: plan_id.clone(),
                status: status.to_string(),
            }
        }
        _ => RuntimeEvent::FeedbackRecorded {
            run_id: runner_event_run_id(event).to_string(),
            kind: event.event_type().to_string(),
            summary: event.message(),
        },
    }
}

fn runtime_workflow_outcome(outcome: RunOutcome) -> RuntimeWorkflowOutcome {
    match outcome {
        RunOutcome::Succeeded => RuntimeWorkflowOutcome::Success { commit_hash: None },
        RunOutcome::Failed => RuntimeWorkflowOutcome::Halted {
            reason: "runner failed".to_string(),
        },
        RunOutcome::Cancelled => RuntimeWorkflowOutcome::Cancelled,
    }
}

fn agent_completion_succeeded(outcome: AgentDispatchOutcome, exit_code: Option<i32>) -> bool {
    matches!(outcome, AgentDispatchOutcome::Completed)
        || (matches!(outcome, AgentDispatchOutcome::Exited) && exit_code.unwrap_or(0) == 0)
}

fn runtime_gate_name(kind: GateCompletionKind, attempt: &TaskAttemptRef) -> String {
    let kind = match kind {
        GateCompletionKind::Preflight => "preflight",
        GateCompletionKind::Gate => "gate",
        GateCompletionKind::PlanVerify => "plan_verify",
        GateCompletionKind::Merge => "merge",
    };
    format!(
        "{kind}:{}:{}",
        attempt.plan_id.as_str(),
        attempt.task_id.as_str()
    )
}

fn runner_event_run_id(event: &RunnerEvent) -> &str {
    match event {
        RunnerEvent::ResumeMarker { run_id, .. }
        | RunnerEvent::RunStarted { run_id, .. }
        | RunnerEvent::RunCompleted { run_id, .. }
        | RunnerEvent::PlanStarted { run_id, .. }
        | RunnerEvent::PlanCompleted { run_id, .. }
        | RunnerEvent::TaskAttemptStarted { run_id, .. }
        | RunnerEvent::TaskAttemptCompleted { run_id, .. }
        | RunnerEvent::AgentDispatchStarted { run_id, .. }
        | RunnerEvent::AgentDispatchCompleted { run_id, .. }
        | RunnerEvent::AgentCompleted { run_id, .. }
        | RunnerEvent::GateDispatchStarted { run_id, .. }
        | RunnerEvent::GateCompleted { run_id, .. }
        | RunnerEvent::PromptAssembled { run_id, .. }
        | RunnerEvent::MergeBackendCompleted { run_id, .. }
        | RunnerEvent::RetryDecision { run_id, .. } => run_id,
    }
}

/// Per-task usage snapshot captured just before emitting feedback.
/// Carries the accumulated token / cost / timing data from [`RunState`]
/// so that `runner_event_to_feedback` does not have to zero-fill those
/// fields.
#[derive(Debug, Clone, Default)]
struct TaskUsageSnapshot {
    tokens_in: u64,
    tokens_out: u64,
    cost_usd: f64,
    duration_ms: u64,
    prompt_text: Option<String>,
    agent_output: String,
}

#[derive(Debug, Clone)]
struct DaimonTaskHook {
    strategy: StrategyCoordinates,
    signal: SomaticSignal,
    affect_confidence: f64,
    behavioral_state: roko_core::BehavioralState,
    pleasure: f64,
    arousal: f64,
    dominance: f64,
}

#[derive(Debug, Clone)]
struct DaimonDispatchModulation {
    model: String,
    turn_limit: u32,
    effort: String,
}

fn with_daimon_state<T>(
    config: &RunConfig,
    f: impl FnOnce(&mut roko_daimon::DaimonState) -> T,
) -> Option<T> {
    let daimon_state = config.daimon_state.as_ref()?;
    match daimon_state.lock() {
        Ok(mut guard) => Some(f(&mut guard)),
        Err(err) => {
            warn!(error = %err, "daimon state lock poisoned; skipping affect hook");
            None
        }
    }
}

fn daimon_task_hook(
    config: &RunConfig,
    task_def: &TaskDef,
    attempt_num: u32,
) -> Option<DaimonTaskHook> {
    with_daimon_state(config, |daimon| {
        let affect = daimon.query();
        let observation = TaskStrategyObservation {
            task_tier: task_def.tier.clone(),
            file_count: task_def.files.len(),
            verification_count: task_def.verify.len(),
            dependency_count: task_def.depends_on.len(),
            max_loc: task_def.max_loc.unwrap_or(50),
            familiarity: 0.5,
            confidence: affect.confidence,
            failure_pressure: f64::from(attempt_num.saturating_sub(1).min(5)) / 5.0,
            urgency_pressure: if attempt_num > 1 { 1.0 } else { 0.0 },
        };
        let strategy = daimon.strategy_space().computer().task_coords(&observation);
        let signal = daimon.query_somatic(strategy);
        if signal.should_emit_event() {
            info!(
                task_id = %task_def.id,
                valence = signal.valence,
                intensity = signal.intensity,
                source_episodes = signal.source_episodes.len(),
                "daimon somatic marker fired"
            );
        }
        DaimonTaskHook {
            strategy,
            signal,
            affect_confidence: affect.confidence,
            behavioral_state: affect.behavioral_state,
            pleasure: affect.pad.pleasure,
            arousal: affect.pad.arousal,
            dominance: affect.pad.dominance,
        }
    })
}

fn daimon_policy_for_hook(hook: Option<&DaimonTaskHook>) -> roko_core::DaimonPolicy {
    hook.map(|hook| roko_core::DaimonPolicy::new(hook.affect_confidence, hook.behavioral_state))
        .unwrap_or_default()
}

fn daimon_dispatch_modulation(
    config: &RunConfig,
    hook: &DaimonTaskHook,
    selected_model: &str,
    allow_model_modulation: bool,
) -> Option<DaimonDispatchModulation> {
    with_daimon_state(config, |daimon| {
        let mut params = DispatchParams::new(selected_model.to_string(), DEFAULT_AGENT_TURN_LIMIT);
        params.effort = default_effort_label(config);
        daimon.modulate_with_strategy(&mut params, hook.strategy);
        if !allow_model_modulation {
            params.model = selected_model.to_string();
        }
        DaimonDispatchModulation {
            model: params.model,
            turn_limit: params.turn_limit.max(1),
            effort: params.effort,
        }
    })
}

fn default_effort_label(config: &RunConfig) -> String {
    config
        .roko_config
        .as_ref()
        .map(|config| config.agent.default_effort.trim())
        .filter(|effort| !effort.is_empty())
        .unwrap_or("medium")
        .to_string()
}

fn render_daimon_prompt_context(hook: &DaimonTaskHook) -> Option<String> {
    let pad_magnitude =
        hook.pleasure.abs() + hook.arousal.abs() + hook.dominance.abs() + hook.signal.intensity;
    if pad_magnitude < 0.35 {
        return None;
    }

    let mut content = format!(
        "# Daimon state\nBehavioral state: {:?}\nPAD: pleasure={:.2}, arousal={:.2}, dominance={:.2}",
        hook.behavioral_state, hook.pleasure, hook.arousal, hook.dominance
    );
    if hook.signal.intensity >= 0.15 {
        content.push_str(&format!(
            "\nSomatic hint: valence={:.2}, intensity={:.2}",
            hook.signal.valence, hook.signal.intensity
        ));
        if hook.signal.valence <= -0.2 {
            content
                .push_str("\nInterpretation: slow down, prefer caution, and verify risky moves.");
        } else if hook.signal.valence >= 0.2 {
            content.push_str(
                "\nInterpretation: this strategy region has positive prior outcomes; keep momentum without skipping checks.",
            );
        }
    }
    Some(content)
}

fn record_daimon_gate_result(config: &RunConfig, completion: &GateCompletion) {
    with_daimon_state(config, |daimon| {
        daimon.appraise(AffectEvent::GateResult {
            plan_id: completion.plan_id.clone(),
            task_id: completion.task_id.clone(),
            passed: completion.passed,
            rung: completion.rung,
        });
    });
}

fn record_daimon_task_outcome(
    config: &RunConfig,
    strategy: Option<StrategyCoordinates>,
    plan_id: &str,
    task_id: &str,
    succeeded: bool,
    discriminator: &str,
) {
    with_daimon_state(config, |daimon| {
        daimon.appraise(AffectEvent::TaskOutcome {
            task_id: task_id.to_string(),
            succeeded,
        });
        if let Some(strategy) = strategy {
            daimon.record_somatic_outcome(
                strategy,
                somatic_episode_hash(
                    plan_id,
                    task_id,
                    if succeeded { "success" } else { "failure" },
                    discriminator,
                ),
            );
        }
    });
}

fn somatic_episode_hash(
    plan_id: &str,
    task_id: &str,
    outcome: &str,
    discriminator: &str,
) -> ContentHash {
    ContentHash::of(format!("somatic:{plan_id}:{task_id}:{outcome}:{discriminator}").as_bytes())
}

/// Translate a [`RunnerEvent`] into a [`FeedbackEvent`] when the runner
/// has enough information for one. Returns `None` for variants that do
/// not map to the feedback layer (e.g. `RunStarted`, `ResumeMarker`).
///
/// `routing_ctx` is the dispatch-time routing context stored on
/// [`RunState`] — threaded here so `TaskCompleted` events carry the
/// real feature vector for the CascadeRouter's bandit.
fn runner_event_to_feedback(
    event: &RunnerEvent,
    routing_ctx: &Option<roko_learn::model_router::RoutingContext>,
    usage: &TaskUsageSnapshot,
) -> Option<crate::runtime_feedback::FeedbackEvent> {
    use crate::dispatch::{AgentOutcome, ModelChoiceSource};
    use crate::runtime_feedback::FeedbackEvent;

    match event {
        RunnerEvent::TaskAttemptCompleted {
            attempt,
            outcome,
            model,
            provider,
            ..
        } => {
            if model.trim().is_empty() {
                return None;
            }

            let succeeded = matches!(outcome, TaskAttemptOutcome::Passed);
            let agent_outcome = AgentOutcome {
                task_id: attempt.task_id.clone(),
                plan_id: attempt.plan_id.clone(),
                model: model.clone(),
                provider: provider.clone(),
                output: usage.agent_output.clone(),
                tokens_in: usage.tokens_in,
                tokens_out: usage.tokens_out,
                cost_usd: usage.cost_usd,
                duration_ms: usage.duration_ms,
                exit_code: None,
                is_error: !succeeded,
            };
            Some(FeedbackEvent::TaskCompleted {
                plan_id: attempt.plan_id.clone(),
                task_id: attempt.task_id.clone(),
                outcome: agent_outcome,
                model_source: ModelChoiceSource::Default,
                succeeded,
                routing_context: routing_ctx.clone(),
                prompt_text: usage.prompt_text.clone(),
            })
        }
        RunnerEvent::GateCompleted {
            attempt,
            rung,
            passed,
            duration_ms,
            ..
        } => Some(FeedbackEvent::GateOutcome {
            plan_id: attempt.plan_id.clone(),
            task_id: attempt.task_id.clone(),
            rung: *rung,
            passed: *passed,
            duration_ms: *duration_ms,
        }),
        RunnerEvent::RetryDecision {
            attempt,
            cooldown_ms,
            current_attempt,
            ..
        } => Some(FeedbackEvent::RetryDecision {
            plan_id: attempt.plan_id.clone(),
            task_id: attempt.task_id.clone(),
            attempt: *current_attempt,
            backoff_secs: cooldown_ms / 1000,
        }),
        RunnerEvent::PlanCompleted {
            plan_id, outcome, ..
        } => {
            let succeeded = matches!(outcome, PlanOutcome::Succeeded);
            Some(FeedbackEvent::PlanCompleted {
                plan_id: plan_id.clone(),
                succeeded,
                tasks_completed: 0,
                tasks_failed: 0,
                total_cost_usd: 0.0,
            })
        }
        _ => None,
    }
}

fn build_run_completed_event(
    executor: &ParallelExecutor,
    plans: &[Plan],
    state: &RunState,
    outcome: RunOutcome,
) -> RunnerEvent {
    let report = build_report(executor, plans, state);
    RunnerEvent::run_completed(
        state.run_id(),
        outcome,
        RunTotals {
            total_tasks: report.total_tasks,
            tasks_completed: report.tasks_completed,
            tasks_failed: report.tasks_failed,
            total_agent_calls: report.total_agent_calls,
            total_cost_usd: report.total_cost_usd,
            duration_ms: report.duration.as_millis() as u64,
        },
        report
            .plans
            .into_iter()
            .map(|plan| PlanRunSummary {
                plan_id: plan.plan_id,
                completed: plan.completed,
                tasks_total: plan.tasks_total,
                tasks_completed: plan.tasks_completed,
                tasks_failed: plan.tasks_failed,
            })
            .collect(),
    )
}

// ─── Snapshot Helper ────────────────────────────────────────────────────

fn sorted_task_revisions(state: &RunState) -> Vec<persist::TaskRevision> {
    let mut revisions = state.revised_tasks.values().cloned().collect::<Vec<_>>();
    revisions.sort_by(|left, right| {
        left.plan_id
            .cmp(&right.plan_id)
            .then_with(|| left.task_id.cmp(&right.task_id))
            .then_with(|| left.failure_key.cmp(&right.failure_key))
    });
    revisions
}

fn apply_revised_tasks_to_plan_map(
    plan_map: &mut HashMap<String, Vec<TaskDef>>,
    revisions: &[persist::TaskRevision],
) {
    for revision in revisions {
        let Some(tasks) = plan_map.get_mut(&revision.plan_id) else {
            continue;
        };
        if let Some(task) = tasks.iter_mut().find(|task| task.id == revision.task_id) {
            *task = revision.revised_task.clone();
        }
    }
}

fn apply_task_revision_to_index(
    task_index: &mut HashMap<String, HashMap<String, TaskDef>>,
    revision: &persist::TaskRevision,
) {
    if let Some(tasks) = task_index.get_mut(&revision.plan_id)
        && tasks.contains_key(&revision.task_id)
    {
        tasks.insert(revision.task_id.clone(), revision.revised_task.clone());
    }
}

fn refresh_task_fingerprints_from_index(
    state: &mut RunState,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
) {
    let mut plan_ids = task_index.keys().cloned().collect::<Vec<_>>();
    plan_ids.sort();

    let mut fingerprints = Vec::new();
    for plan_id in plan_ids {
        let Some(tasks) = task_index.get(&plan_id) else {
            continue;
        };
        let mut task_refs = tasks.values().collect::<Vec<_>>();
        task_refs.sort_by(|left, right| {
            left.sequence
                .cmp(&right.sequence)
                .then_with(|| left.id.cmp(&right.id))
        });
        fingerprints.extend(
            task_refs
                .into_iter()
                .map(|task| persist::TaskDefFingerprint::from_task(task, &plan_id)),
        );
    }
    state.task_fingerprints = fingerprints;
}

/// Build a unified [`StateSnapshot`] from all four state groups (executor,
/// orchestrator, run counters, gate thresholds) with a SHA-256 checksum,
/// then enqueue the serialized blob on the async writer. Serialisation
/// (<1ms) happens on the caller's thread; the single atomic disk write
/// runs on the dedicated writer thread.
fn save_snapshot(
    config: &RunConfig,
    executor: &ParallelExecutor,
    paths: &PersistPaths,
    state: &mut RunState,
    merge_queue: &MergeQueue,
    gate_thresholds: &GateThresholds,
    writer: &SnapshotWriter,
) {
    let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
    let snapshot = executor.snapshot(timestamp_ms);
    let orchestrator_snapshot = OrchestratorSnapshot::new(snapshot.clone(), timestamp_ms)
        .with_merge_queue(merge_queue.snapshot());

    let orchestrator_json = match orchestrator_snapshot.to_json() {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "failed to serialize orchestrator snapshot");
            state.snapshot_failed();
            return;
        }
    };
    let executor_json = match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "failed to serialize executor snapshot");
            state.snapshot_failed();
            return;
        }
    };

    let run_state = persist::RunStateSnapshot {
        schema_version: persist::RUN_STATE_SCHEMA_VERSION,
        run_id: state.run_id().to_string(),
        started_at_ms: state.start_epoch_ms,
        timestamp_ms,
        tasks_total: state.tasks_total,
        tasks_completed: state.tasks_completed,
        tasks_failed: state.tasks_failed,
        total_tokens_in: state.total_tokens_in,
        total_tokens_out: state.total_tokens_out,
        total_cost_usd: state.total_cost_usd,
        total_agent_calls: state.total_agent_calls,
        plan_costs: state.plan_costs.clone(),
        completed_tasks: state.completed_tasks.clone(),
        snapshot_fail_streak: state.snapshot_fail_streak,
        fingerprints: state.task_fingerprints.clone(),
        replan_ledger: state.replan_ledger.clone(),
        revised_tasks: sorted_task_revisions(state),
        cascade_router_json: config
            .cascade_router
            .as_ref()
            .map(|router| router.snapshot_json()),
    };
    let run_state_json = match serde_json::to_string_pretty(&run_state) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "failed to serialize run-state snapshot");
            state.snapshot_failed();
            return;
        }
    };
    let gate_thresholds_json = match serde_json::to_string_pretty(gate_thresholds) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "failed to serialize gate thresholds");
            state.snapshot_failed();
            return;
        }
    };

    let unified = roko_runtime::StateSnapshot::new(
        timestamp_ms,
        executor_json,
        orchestrator_json,
        run_state_json,
        gate_thresholds_json,
    );

    let snapshot_json = match serde_json::to_vec_pretty(&unified) {
        Ok(json) => json,
        Err(e) => {
            error!(error = %e, "failed to serialize unified state snapshot");
            state.snapshot_failed();
            return;
        }
    };

    writer.write(SnapshotPayload {
        snapshot_json,
        snapshot_path: paths.state_snapshot_json.clone(),
    });
}

fn restore_state_from_resume_snapshot(
    state: &mut RunState,
    snapshot: &persist::RunStateSnapshot,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
    drifted_tasks: &[super::resume::DriftedTask],
) {
    state.tasks_failed = snapshot.tasks_failed;
    state.total_tokens_in = snapshot.total_tokens_in;
    state.total_tokens_out = snapshot.total_tokens_out;
    state.total_cost_usd = snapshot.total_cost_usd;
    state.total_agent_calls = snapshot.total_agent_calls;
    state.plan_costs = snapshot.plan_costs.clone();
    state.snapshot_fail_streak = snapshot.snapshot_fail_streak;
    state.completed_tasks = snapshot.completed_tasks.clone();
    state.replan_ledger = snapshot.replan_ledger.clone();
    state.revised_tasks = snapshot
        .revised_tasks
        .iter()
        .cloned()
        .map(|revision| {
            (
                format!("{}/{}", revision.plan_id, revision.task_id),
                revision,
            )
        })
        .collect();
    state.completed_tasks.retain(|plan_id, completed| {
        let Some(tasks) = task_index.get(plan_id) else {
            return false;
        };
        completed.retain(|task_id| tasks.contains_key(task_id));
        !completed.is_empty()
    });

    let mut requeued_count = 0usize;
    for drifted in drifted_tasks {
        if let Some(completed) = state.completed_tasks.get_mut(&drifted.plan_id) {
            let before = completed.len();
            completed.retain(|task_id| task_id != &drifted.task_id);
            if completed.len() != before {
                requeued_count += 1;
                warn!(
                    plan = %drifted.plan_id,
                    task = %drifted.task_id,
                    "task definition drifted since snapshot — re-queuing"
                );
                info!(
                    plan = %drifted.plan_id,
                    task = %drifted.task_id,
                    "re-queued (definition changed)"
                );
            }
        }
    }

    if requeued_count > 0 {
        warn!(
            drifted_count = requeued_count,
            "detected drifted tasks — completed ones were re-queued"
        );
    }

    state.tasks_completed = state.completed_tasks.values().map(Vec::len).sum::<usize>();
}

fn seed_completed_tasks_from_plan_status(state: &mut RunState, plans: &[Plan]) {
    for plan in plans {
        for task in &plan.tasks.tasks {
            if task_status_is_terminal(&task.status) {
                state.mark_task_completed(&plan.id, &task.id);
            }
        }
    }

    state.tasks_completed = state.completed_tasks.values().map(Vec::len).sum::<usize>();
    if state.tasks_completed > 0 {
        info!(
            tasks_completed = state.tasks_completed,
            "seeded completed tasks from plan status"
        );
    }
}

fn seed_task_dag_from_run_state(task_dag: &mut TaskDag, plans: &[Plan], state: &RunState) {
    for plan in plans {
        task_dag.plan_mut(&plan.id);
        for task_id in state.plan_completed_tasks(&plan.id) {
            task_dag.mark_complete(&plan.id, task_id);
        }
    }
}

fn initialize_terminal_plan_phases(
    executor: &mut ParallelExecutor,
    state: &RunState,
    plans: &[Plan],
) {
    for plan in plans {
        if plan.tasks.tasks.is_empty() {
            continue;
        }
        let completed = state.plan_completed_tasks(&plan.id);
        let all_tasks_terminal = plan
            .tasks
            .tasks
            .iter()
            .all(|task| completed.contains(&task.id) || task_status_is_terminal(&task.status));

        if all_tasks_terminal
            && let Some(plan_state) = executor.plan_state_mut(&plan.id)
            && !plan_state.is_terminal()
        {
            plan_state.current_phase = PlanPhase::Gating;
            info!(
                plan_id = %plan.id,
                "initialized completed plan at gating phase"
            );
        }
    }
}

// ─── Resume ─────────────────────────────────────────────────────────────

struct ResumeLoad {
    executor: ParallelExecutor,
    merge_queue: MergeQueue,
    marker: ResumeMarker,
}

/// Load a resumable executor snapshot when compatible, otherwise start fresh
/// and emit a structured resume marker explaining the decision.
fn load_executor(paths: &PersistPaths, config: &ExecutorConfig, plan_ids: &[String]) -> ResumeLoad {
    let (snapshot, merge_queue, snapshot_path) = match load_orchestrator_checkpoint(paths) {
        Ok(Some((snapshot, merge_queue))) => (
            snapshot,
            merge_queue,
            paths.orchestrator_json.display().to_string(),
        ),
        Ok(None) => match load_unified_state_checkpoint(paths) {
            Ok(Some((snapshot, merge_queue))) => (
                snapshot,
                merge_queue,
                paths.state_snapshot_json.display().to_string(),
            ),
            Ok(None) => match load_legacy_executor_checkpoint(paths) {
                Ok(Some(snapshot)) => (
                    snapshot,
                    MergeQueue::new(),
                    paths.executor_json.display().to_string(),
                ),
                Ok(None) => {
                    return ResumeLoad {
                        executor: ParallelExecutor::new(config.clone()),
                        merge_queue: MergeQueue::new(),
                        marker: ResumeMarker {
                            outcome: ResumeOutcome::Fresh,
                            snapshot_path: paths.state_snapshot_json.display().to_string(),
                            snapshot_plan_ids: Vec::new(),
                            current_plan_ids: plan_ids.to_vec(),
                            message: Some("no executor snapshot found".to_string()),
                        },
                    };
                }
                Err(e) => {
                    let snapshot_path = paths.executor_json.display().to_string();
                    warn!(err = %e, "failed to load legacy executor snapshot");
                    return fresh_after_snapshot_error(
                        Some((snapshot_path, ResumeOutcome::Corrupt, e)),
                        config,
                        plan_ids,
                    );
                }
            },
            Err(e) => {
                let snapshot_path = paths.state_snapshot_json.display().to_string();
                warn!(err = %e, "failed to load unified state snapshot");
                let first_error = (snapshot_path, ResumeOutcome::Corrupt, e);
                match load_legacy_executor_checkpoint(paths) {
                    Ok(Some(snapshot)) => (
                        snapshot,
                        MergeQueue::new(),
                        paths.executor_json.display().to_string(),
                    ),
                    Ok(None) => {
                        return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                    }
                    Err(e) => {
                        warn!(err = %e, "failed to load legacy executor snapshot");
                        return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                    }
                }
            }
        },
        Err(e) => {
            let snapshot_path = paths.orchestrator_json.display().to_string();
            warn!(err = %e, "failed to load orchestrator snapshot");
            let first_error = (snapshot_path, ResumeOutcome::Corrupt, e);
            match load_unified_state_checkpoint(paths) {
                Ok(Some((snapshot, merge_queue))) => (
                    snapshot,
                    merge_queue,
                    paths.state_snapshot_json.display().to_string(),
                ),
                Ok(None) => match load_legacy_executor_checkpoint(paths) {
                    Ok(Some(snapshot)) => (
                        snapshot,
                        MergeQueue::new(),
                        paths.executor_json.display().to_string(),
                    ),
                    Ok(None) => {
                        return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                    }
                    Err(e) => {
                        warn!(err = %e, "failed to load legacy executor snapshot");
                        return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                    }
                },
                Err(e) => {
                    warn!(err = %e, "failed to load unified state snapshot");
                    match load_legacy_executor_checkpoint(paths) {
                        Ok(Some(snapshot)) => (
                            snapshot,
                            MergeQueue::new(),
                            paths.executor_json.display().to_string(),
                        ),
                        Ok(None) => {
                            return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                        }
                        Err(e) => {
                            warn!(err = %e, "failed to load legacy executor snapshot");
                            return fresh_after_snapshot_error(Some(first_error), config, plan_ids);
                        }
                    }
                }
            }
        }
    };

    // Validate: snapshot must contain at least one of the current plan IDs.
    let snap_plan_ids: Vec<String> = snapshot.plan_states.keys().cloned().collect();
    let has_overlap = plan_ids
        .iter()
        .any(|id| snapshot.plan_states.contains_key(id));

    if snap_plan_ids.is_empty() || !has_overlap {
        info!(
            snapshot_plans = ?snap_plan_ids,
            current_plans = ?plan_ids,
            "stale executor snapshot (no plan overlap) — starting fresh"
        );
        return ResumeLoad {
            executor: ParallelExecutor::new(config.clone()),
            merge_queue: MergeQueue::new(),
            marker: ResumeMarker {
                outcome: ResumeOutcome::IgnoredStale,
                snapshot_path,
                snapshot_plan_ids: snap_plan_ids,
                current_plan_ids: plan_ids.to_vec(),
                message: Some("snapshot has no overlap with current plans".to_string()),
            },
        };
    }

    info!(
        path = %snapshot_path,
        plans = ?snap_plan_ids,
        "resuming from executor snapshot"
    );
    let mut executor = ParallelExecutor::from_snapshot(config.clone(), snapshot.clone());
    let recovery = RecoveryEngine::new().recover_from_executor_snapshot(snapshot);
    let now_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let resume_plan = recovery.resume_plan(now_ms);
    for warning in &resume_plan.warnings {
        warn!(
            plan_id = %warning.plan_id,
            message = %warning.message,
            severity = ?warning.severity,
            "orchestrator recovery warning"
        );
    }
    for plan in &resume_plan.retryable_terminal {
        if executor.requeue_retryable_terminal(&plan.plan_id).is_some() {
            info!(
                plan_id = %plan.plan_id,
                retry_after_ms = ?plan.retry_after_ms,
                "requeued retryable terminal plan after recovery"
            );
        }
    }
    ResumeLoad {
        executor,
        merge_queue,
        marker: ResumeMarker {
            outcome: ResumeOutcome::Resumed,
            snapshot_path,
            snapshot_plan_ids: snap_plan_ids,
            current_plan_ids: plan_ids.to_vec(),
            message: Some("resumed from compatible executor snapshot".to_string()),
        },
    }
}

fn fresh_after_snapshot_error(
    first_error: Option<(String, ResumeOutcome, String)>,
    config: &ExecutorConfig,
    plan_ids: &[String],
) -> ResumeLoad {
    let (snapshot_path, outcome, message) = first_error.unwrap_or_else(|| {
        (
            String::new(),
            ResumeOutcome::ReadFailed,
            "failed to load executor snapshot".to_string(),
        )
    });
    ResumeLoad {
        executor: ParallelExecutor::new(config.clone()),
        merge_queue: MergeQueue::new(),
        marker: ResumeMarker {
            outcome,
            snapshot_path,
            snapshot_plan_ids: Vec::new(),
            current_plan_ids: plan_ids.to_vec(),
            message: Some(message),
        },
    }
}

fn load_orchestrator_checkpoint(
    paths: &PersistPaths,
) -> Result<Option<(ExecutorSnapshot, MergeQueue)>, String> {
    let json = match std::fs::read_to_string(&paths.orchestrator_json) {
        Ok(j) => j,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("failed to read aggregate snapshot: {e}")),
    };
    let snapshot = OrchestratorSnapshot::from_json(&json)
        .map_err(|err| format!("failed to parse aggregate snapshot: {err}"))?;
    let merge_queue = snapshot
        .merge_queue
        .map(MergeQueue::from_snapshot)
        .unwrap_or_else(MergeQueue::new);
    Ok(Some((snapshot.executor, merge_queue)))
}

fn load_unified_state_checkpoint(
    paths: &PersistPaths,
) -> Result<Option<(ExecutorSnapshot, MergeQueue)>, String> {
    let unified = persist::load_state_snapshot(paths)
        .map_err(|err| format!("failed to read unified state snapshot: {err}"))?;
    let Some(unified) = unified else {
        return Ok(None);
    };

    if !unified.orchestrator_json.trim().is_empty() {
        match OrchestratorSnapshot::from_json(&unified.orchestrator_json) {
            Ok(snapshot) => {
                let merge_queue = snapshot
                    .merge_queue
                    .map(MergeQueue::from_snapshot)
                    .unwrap_or_else(MergeQueue::new);
                return Ok(Some((snapshot.executor, merge_queue)));
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to parse orchestrator_json from unified state snapshot; trying executor_json"
                );
            }
        }
    }

    let snapshot = ExecutorSnapshot::from_json(&unified.executor_json).map_err(|err| {
        format!("failed to parse executor_json from unified state snapshot: {err}")
    })?;
    Ok(Some((snapshot, MergeQueue::new())))
}

fn load_legacy_executor_checkpoint(
    paths: &PersistPaths,
) -> Result<Option<ExecutorSnapshot>, String> {
    let json = match std::fs::read_to_string(&paths.executor_json) {
        Ok(json) => json,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("failed to read executor snapshot: {e}")),
    };
    ExecutorSnapshot::from_json(&json)
        .map(Some)
        .map_err(|err| format!("corrupt executor snapshot: {err}"))
}

// ─── Action Dispatcher ──────────────────────────────────────────────────

fn record_skipped_gate_rung(
    ctx: &mut RunContext<'_>,
    plan_id: &str,
    task_id: &str,
    rung: u32,
    gate_name: &str,
    summary: &str,
) {
    if let Some(plan_state) = ctx.executor.plan_state_mut(plan_id) {
        plan_state.gate_results.push(GateResult {
            gate_name: gate_name.to_string(),
            rung,
            passed: true,
            summary: summary.to_string(),
            duration_ms: 0,
            test_count: None,
        });
    }
    ctx.tui.gate_result(plan_id, task_id, gate_name, true);

    if rung >= ctx.config.max_gate_rung {
        if let Err(err) = ctx
            .executor
            .apply_event(plan_id, &ExecutorEvent::GatePassed)
        {
            warn!(plan_id = %plan_id, rung, error = %err, "failed to advance after skipped final gate");
        }
    } else {
        debug!(
            plan_id = %plan_id,
            task_id = %task_id,
            rung,
            max_gate_rung = ctx.config.max_gate_rung,
            "skipped gate rung recorded; advancing to next rung"
        );
    }
}

fn gates_config_for_run(config: &RunConfig) -> GatesConfig {
    let mut gates = config
        .roko_config
        .as_ref()
        .map(|roko_config| roko_config.gates.clone())
        .unwrap_or_default();
    if !gates.has_custom_rungs() {
        gates.clippy_enabled = config.clippy_enabled;
        gates.skip_tests = config.skip_tests;
    }
    gates
}

fn gate_plan_complexity_for_task(task_def: Option<&TaskDef>) -> PlanComplexity {
    match task_def.map(|task| task.tier.as_str()).unwrap_or("focused") {
        "mechanical" | "fast" => PlanComplexity::Trivial,
        "focused" => PlanComplexity::Simple,
        "integrative" => PlanComplexity::Standard,
        "architectural" | "complex" | "premium" => PlanComplexity::Complex,
        _ => PlanComplexity::Simple,
    }
}

fn task_role_is_read_only(task_def: Option<&TaskDef>) -> bool {
    task_def
        .and_then(|task| task.role.as_deref())
        .map_or(false, |role| {
            matches!(role, "researcher" | "strategist" | "quick-reviewer")
        })
}

fn task_should_preflight_verify(task_def: &TaskDef, attempt_num: u32) -> bool {
    let preflight_disabled = std::env::var("ROKO_SKIP_PREFLIGHT").is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    });
    !preflight_disabled
        && attempt_num == 1
        && !task_def.verify.is_empty()
        && !task_role_is_read_only(Some(task_def))
}

async fn dispatch_action(
    action: &ExecutorAction,
    ctx: &mut RunContext<'_>,
) -> ActionDispatchOutcome {
    match action {
        ExecutorAction::DispatchPlan { plan_id } => {
            info!(plan_id = %plan_id, "dispatching plan");
            ctx.tui.plan_started(plan_id);

            if let Err(e) = ctx.executor.apply_event(plan_id, &ExecutorEvent::Start) {
                error!(plan_id = %plan_id, err = %e, "failed to start plan");
                return ActionDispatchOutcome::Noop;
            }
            let run_id = ctx.state.run_id().to_string();
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::plan_started(&run_id, plan_id),
            );

            if ctx
                .skip_enrichment
                .get(plan_id.as_str())
                .copied()
                .unwrap_or(false)
            {
                if let Err(e) = ctx
                    .executor
                    .apply_event(plan_id, &ExecutorEvent::EnrichmentDone)
                {
                    error!(plan_id = %plan_id, err = %e, "failed to skip enrichment");
                }
                ctx.tui
                    .phase_transition(plan_id, "enriching", "implementing");
            }
            ActionDispatchOutcome::Handled
        }

        ExecutorAction::SpawnAgent { plan_id, task, .. } => {
            let phase_kind = ctx
                .executor
                .plan_state(plan_id)
                .map(|state| state.current_phase.kind());
            let is_dag_task_spawn = phase_kind.as_ref().is_some_and(|kind| {
                matches!(
                    kind,
                    PhaseKind::Implementing | PhaseKind::AutoFixing | PhaseKind::RegeneratingVerify
                )
            });

            // Resolve sentinel task names ("next", "fix", etc.) to actual task IDs
            // by walking the plan's DAG and finding the first ready task.
            let resolved_task = if task == "next" || task == "fix" || task == "regen-verify" {
                let completed = ctx.state.plan_completed_tasks(plan_id);
                let completed_plans = completed_plan_ids(ctx.executor, ctx.task_index);
                let plan_tasks = task_refs_for_plan(ctx.task_index, plan_id);
                let next_ready_task = {
                    let task_dag = &*ctx.task_dag;
                    task_dag.next_ready_task(plan_id, &plan_tasks, completed, &completed_plans)
                };
                next_ready_task.map(|task| task.id.clone())
            } else if matches!(task.as_str(), "review" | "doc-revision" | "docs" | "enrich") {
                if ctx.state.current_task.is_empty() {
                    ctx.task_index
                        .get(plan_id.as_str())
                        .and_then(|tasks| tasks.values().min_by_key(|t| t.sequence))
                        .map(|t| t.id.clone())
                } else {
                    Some(ctx.state.current_task.clone())
                }
            } else {
                Some(task.clone())
            };

            let task_id = match resolved_task {
                Some(id) => id,
                None => {
                    let progress = dag_progress_for_plan(
                        ctx.task_dag,
                        ctx.executor,
                        ctx.task_index,
                        ctx.state,
                        plan_id,
                    );
                    let skipped = ctx
                        .task_dag
                        .mark_blocked_tasks_skipped(plan_id, &progress.blocked_tasks);
                    if !skipped.is_empty() {
                        debug!(
                            plan_id = %plan_id,
                            skipped = ?skipped,
                            "DAG quiescence propagated blocked tasks"
                        );
                    }
                    if progress.can_make_future_progress() {
                        debug!(
                            plan_id = %plan_id,
                            requested_task = %task,
                            "no ready task yet — waiting on DAG dependencies"
                        );
                        return ActionDispatchOutcome::Noop;
                    }

                    let Some(phase_kind) = phase_kind else {
                        warn!(plan_id = %plan_id, requested_task = %task, "no ready task for unknown plan");
                        return ActionDispatchOutcome::Noop;
                    };

                    if dag_plan_has_failures(ctx.task_dag, ctx.state, plan_id)
                        || progress.blocked > 0
                    {
                        let reason = dag_quiescence_reason(plan_id, &progress);
                        warn!(
                            plan_id = %plan_id,
                            requested_task = %task,
                            reason = %reason,
                            "no ready task and DAG cannot make future progress"
                        );
                        if let Err(e) = ctx
                            .executor
                            .apply_event(plan_id, &ExecutorEvent::Fatal(reason.clone()))
                        {
                            error!(
                                plan_id = %plan_id,
                                requested_task = %task,
                                phase = ?phase_kind,
                                err = %e,
                                "failed to transition after DAG quiescence"
                            );
                            ctx.state.force_plan_terminal(plan_id);
                        }
                        ctx.tui.error(&reason);
                        return ActionDispatchOutcome::Noop;
                    }

                    let Some(event) = no_ready_spawn_event(phase_kind, &task) else {
                        debug!(
                            plan_id = %plan_id,
                            requested_task = %task,
                            phase = ?phase_kind,
                            "no ready task for terminal plan"
                        );
                        return ActionDispatchOutcome::Noop;
                    };

                    if matches!(event, ExecutorEvent::ImplementationDone) {
                        info!(plan_id = %plan_id, "no more ready tasks — implementation complete");
                    } else {
                        warn!(
                            plan_id = %plan_id,
                            requested_task = %task,
                            phase = ?phase_kind,
                            "agent spawn requested with no runnable task"
                        );
                    }

                    if let Err(e) = ctx.executor.apply_event(plan_id, &event) {
                        error!(
                            plan_id = %plan_id,
                            requested_task = %task,
                            phase = ?phase_kind,
                            err = %e,
                            "failed to transition after no-ready spawn request"
                        );
                        ctx.state.force_plan_terminal(plan_id);
                    }
                    if let ExecutorEvent::Fatal(reason) = event {
                        ctx.tui.error(&reason);
                    }
                    return ActionDispatchOutcome::Noop;
                }
            };

            let active_task_key = task_key(plan_id, &task_id);
            if ctx.attempt_ownership.contains_task(plan_id, &task_id) {
                debug!(
                    plan_id = %plan_id,
                    task = %task_id,
                    "agent already active for this task — suppressing duplicate spawn"
                );
                return ActionDispatchOutcome::Noop;
            }

            if let Some(remaining) = ctx.state.retry_cooldown_remaining(plan_id) {
                debug!(
                    plan_id = %plan_id,
                    task = %task_id,
                    cooldown_ms = remaining.as_millis(),
                    "retry backoff active — delaying spawn"
                );
                return ActionDispatchOutcome::Noop;
            }

            // Per-plan budget check.
            let max_plan_usd = ctx.config.max_plan_usd;
            let plan_spent = ctx.state.plan_cost(plan_id);
            if max_plan_usd > 0.0 && plan_spent >= max_plan_usd {
                warn!(
                    plan_id = %plan_id,
                    spent = plan_spent,
                    limit = max_plan_usd,
                    "plan budget exceeded — aborting"
                );
                ctx.tui.error(&format!(
                    "budget exceeded: ${plan_spent:.2} >= ${max_plan_usd:.2}"
                ));
                if let Err(e) = ctx.executor.apply_event(
                    plan_id,
                    &ExecutorEvent::Fatal(format!(
                        "budget exceeded: ${plan_spent:.2} >= ${max_plan_usd:.2}"
                    )),
                ) {
                    error!(plan_id = %plan_id, error = %e,
                        "failed to apply Fatal event -- forcing plan terminal");
                    ctx.state.force_plan_terminal(plan_id);
                }
                return ActionDispatchOutcome::Noop;
            }

            let task_def = match ctx
                .task_index
                .get(plan_id.as_str())
                .and_then(|m| m.get(task_id.as_str()))
            {
                Some(t) => t,
                None => {
                    error!(plan_id = %plan_id, task = %task_id, "task not found in index");
                    if let Err(e) = ctx.executor.apply_event(
                        plan_id,
                        &ExecutorEvent::Fatal(format!("task {task_id} not found")),
                    ) {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
                    return ActionDispatchOutcome::Noop;
                }
            };

            let task_permit = match ctx.task_sem.clone().try_acquire_owned() {
                Ok(task_permit) => task_permit,
                Err(_) => {
                    debug!(
                        plan_id = %plan_id,
                        task = %task_id,
                        max_concurrent_tasks = ctx.config.max_concurrent_tasks,
                        "agent task permit unavailable — delaying spawn"
                    );
                    return ActionDispatchOutcome::Noop;
                }
            };

            if is_dag_task_spawn {
                let completed = ctx.state.plan_completed_tasks(plan_id);
                let completed_plans = completed_plan_ids(ctx.executor, ctx.task_index);
                let plan_tasks = task_refs_for_plan(ctx.task_index, plan_id);
                let is_ready = ctx
                    .task_dag
                    .ready_tasks(plan_id, &plan_tasks, completed, &completed_plans)
                    .iter()
                    .any(|task| task.id == task_id);
                if !is_ready {
                    debug!(
                        plan_id = %plan_id,
                        task = %task_id,
                        requested_task = %task,
                        "task is not ready in DAG — suppressing spawn"
                    );
                    return ActionDispatchOutcome::Noop;
                }
                if !ctx.task_dag.mark_running(plan_id, &task_id) {
                    debug!(
                        plan_id = %plan_id,
                        task = %task_id,
                        "task already running in DAG — suppressing duplicate spawn"
                    );
                    return ActionDispatchOutcome::Noop;
                }
            }

            info!(plan_id = %plan_id, task = %task_id, "spawning agent");

            let plan_workdir = match ensure_plan_workdir(ctx.worktrees, plan_id).await {
                Ok(path) => path,
                Err(message) => {
                    error!(
                        plan_id = %plan_id,
                        task = %task_id,
                        error = %message,
                        "failed to acquire isolated plan worktree"
                    );
                    if is_dag_task_spawn {
                        ctx.task_dag.clear_running(plan_id, &task_id);
                    }
                    if let Err(e) = ctx
                        .executor
                        .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                    {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
                    ctx.tui.error(&message);
                    return ActionDispatchOutcome::Noop;
                }
            };

            let previous_gate_output = ctx.state.gate_output.clone();
            let attempt_num = ctx
                .executor
                .plan_state(plan_id)
                .map(|state| state.iteration)
                .unwrap_or(1);
            let attempt_ref = TaskAttemptRef::new(plan_id.clone(), task_id.clone(), attempt_num);
            ctx.state.reset_for_task(plan_id, &task_id);
            ctx.state.set_iteration(plan_id, &task_id, attempt_num);
            ctx.task_runtime_states.insert(
                task_key(plan_id, &task_id),
                TaskRuntimeState::capture(ctx.state),
            );
            let role = task_def.role.as_deref().unwrap_or("implementer");

            if task_should_preflight_verify(task_def, attempt_num)
                && ctx.preflight_attempted.insert(attempt_ref.clone())
            {
                let gates_config = gates_config_for_run(ctx.config);
                let has_cargo_toml = std::fs::metadata(plan_workdir.join("Cargo.toml")).is_ok();
                if gates_config.has_custom_rungs() || has_cargo_toml {
                    let pipeline_rung = ctx.config.max_gate_rung;
                    info!(
                        plan_id = %plan_id,
                        task = %task_id,
                        rung = pipeline_rung,
                        "dispatching task verification preflight before agent"
                    );
                    let preflight_effect = new_gate_effect(
                        attempt_ref.clone(),
                        GateCompletionKind::Preflight,
                        pipeline_rung,
                    );
                    ctx.attempt_ownership
                        .insert(
                            attempt_ref.clone(),
                            AttemptOwner::new(AttemptPhase::AwaitingGate, EffectRef(0)),
                            AgentRuntimeResource::AwaitingGate,
                        )
                        .expect("preflight owner must be unique");
                    let mut gate_claim = ctx
                        .attempt_ownership
                        .claim_phase(&attempt_ref, AttemptPhase::AwaitingGate, EffectRef(0))
                        .expect("preflight owner must be claimable");
                    let effect_key = gate_effect_key(
                        plan_id,
                        &task_id,
                        pipeline_rung,
                        GateCompletionKind::Preflight,
                    );
                    if !ctx.state.mark_gate_active(effect_key.clone()) {
                        ctx.attempt_ownership
                            .complete_claim(gate_claim)
                            .expect("suppressed preflight must release owner");
                        return ActionDispatchOutcome::Noop;
                    }
                    let run_id = ctx.state.run_id().to_string();
                    emit_runner_event(
                        ctx.paths,
                        ctx.state,
                        ctx.tui,
                        ctx.config,
                        RunnerEvent::gate_dispatch_started(
                            &run_id,
                            attempt_ref.clone(),
                            GateCompletionKind::Preflight,
                            pipeline_rung,
                        ),
                    );
                    let (gate_handle, start_tx) = gate_dispatch::spawn_gate(
                        preflight_effect.clone(),
                        plan_id.clone(),
                        task_id.clone(),
                        pipeline_rung,
                        plan_workdir.clone(),
                        gates_config,
                        gate_plan_complexity_for_task(Some(task_def)),
                        task_def.verify.clone(),
                        duration_secs(gate_timeout(ctx.config, pipeline_rung)),
                        ctx.gate_tx.clone(),
                        ctx.gate_sem.clone(),
                        task_target_crates(Some(task_def)),
                    );
                    gate_claim.replace_resource(AgentRuntimeResource::Gate {
                        effect: preflight_effect.clone(),
                        handle: gate_handle,
                    });
                    ctx.attempt_ownership
                        .transition_claim(
                            gate_claim,
                            AttemptPhase::Gate,
                            EffectRef(preflight_effect.generation),
                        )
                        .expect("preflight must retain exact owner");
                    if start_tx.send(()).is_err() {
                        ctx.state.clear_gate_active(&effect_key);
                        if let Ok(mut claim) = ctx.attempt_ownership.claim_phase(
                            &attempt_ref,
                            AttemptPhase::Gate,
                            EffectRef(preflight_effect.generation),
                        ) {
                            if let AgentRuntimeResource::Gate { handle, .. } =
                                claim.replace_resource(AgentRuntimeResource::AwaitingGate)
                            {
                                let _ = handle.await;
                            }
                            ctx.attempt_ownership.complete_claim(claim).ok();
                        }
                        let message = format!(
                            "preflight producer failed to start for {}",
                            attempt_ref.key()
                        );
                        let _ = ctx
                            .executor
                            .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()));
                        ctx.tui.error(&message);
                    }
                    return ActionDispatchOutcome::Handled;
                }
            }

            ctx.state.total_agent_calls += 1;
            ctx.state.task_agent_calls += 1;

            let role_enum = parse_dispatch_role(role);
            let task_category = neuro_prompt_task_category(role_enum);

            ctx.sink
                .task_started(plan_id, &task_id, role, &task_def.title, attempt_num);
            let bias_weight = knowledge_bias_weight(ctx.config);
            let knowledge_candidates = candidate_model_slugs(ctx.config);
            let knowledge_store = KnowledgeStore::for_workdir(&ctx.config.workdir);
            let knowledge_advice = build_knowledge_routing_advice(
                &knowledge_store,
                &knowledge_candidates,
                role_enum,
                task_category.label(),
            );
            debug!(
                plan_id = %plan_id,
                task = %task_id,
                role = %role_enum,
                task_category = %task_category.label(),
                hints = knowledge_advice.hints.len(),
                bias_weight = bias_weight,
                "knowledge store consulted for routing"
            );
            let gate_feedback = DispatchGateFeedback::from_raw(&previous_gate_output);
            let daimon_hook = daimon_task_hook(ctx.config, task_def, attempt_num);
            ctx.state.current_daimon_strategy = daimon_hook.as_ref().map(|hook| hook.strategy);
            let routing_context = {
                use roko_learn::model_router::RoutingContext;
                RoutingContext {
                    task_category,
                    complexity: tier_to_complexity(&task_def.tier),
                    iteration: attempt_num.saturating_sub(1),
                    role: role_enum,
                    crate_familiarity: 0.5,
                    has_prior_failure: attempt_num > 1,
                    conductor_load: 0.0,
                    active_agents: u32::try_from(
                        ctx.attempt_ownership
                            .surviving_agent_metadata()
                            .agent_ids
                            .len(),
                    )
                    .unwrap_or(u32::MAX),
                    ready_queue_depth: 0,
                    max_queue_wait_hours: 0.0,
                    daimon_policy: daimon_policy_for_hook(daimon_hook.as_ref()),
                    thinking_level: daimon_hook
                        .as_ref()
                        .map(|_| default_effort_label(ctx.config)),
                    temperament: None,
                    previous_model: None,
                    plan_context_tokens: None,
                    tier_thresholds: daimon_hook
                        .as_ref()
                        .map(|hook| roko_daimon::adjusted_thresholds(&hook.behavioral_state)),
                }
            };
            let dispatch_ctx = DispatchContext {
                plan_id: plan_id.clone(),
                role: role.to_string(),
                workdir: plan_workdir.clone(),
                model_hint: None,
                force_backend: ctx.config.cli_model_override.clone(),
                budget_remaining_usd: if ctx.config.max_plan_usd > 0.0 {
                    (ctx.config.max_plan_usd - ctx.state.plan_cost(plan_id)).max(0.0)
                } else {
                    f64::INFINITY
                },
                attempt: attempt_num.saturating_sub(1),
                gate_feedback,
                routing_context: Some(routing_context),
                dependency_outputs: ctx.state.dependency_outputs(plan_id, &task_def.depends_on),
            };
            ctx.state.task_model_hint = task_def.model_hint.clone();
            ctx.state.routing_context = dispatch_ctx.routing_context.clone();
            let dispatcher = ctx.factory.dispatcher();
            let mut dispatch_plan = match dispatcher.plan(task_def, &dispatch_ctx) {
                Ok(plan) => plan,
                Err(err) => {
                    let message = format!("dispatch planning failed: {err}");
                    error!(plan_id = %plan_id, task = %task_id, error = %message);
                    if is_dag_task_spawn {
                        ctx.task_dag.clear_running(plan_id, &task_id);
                    }
                    ctx.task_runtime_states.remove(&task_key(plan_id, &task_id));
                    if let Err(e) = ctx
                        .executor
                        .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                    {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
                    ctx.tui.error(&message);
                    return ActionDispatchOutcome::Noop;
                }
            };
            let baseline_model = dispatch_plan.model.slug.clone();
            let baseline_score = knowledge_advice.score_for(&baseline_model);
            let mut selected_source = "dispatcher".to_string();
            let allow_learned_model_modulation =
                task_def.model_hint.is_none() && !dispatch_plan.forced;
            if allow_learned_model_modulation {
                if let Some(best_hint) = knowledge_advice
                    .hints
                    .iter()
                    .filter(|hint| hint.model_slug != baseline_model)
                    .max_by(|left, right| {
                        left.score
                            .total_cmp(&right.score)
                            .then_with(|| left.model_slug.cmp(&right.model_slug))
                    })
                {
                    if best_hint.score + bias_weight > baseline_score {
                        debug!(
                            from = %baseline_model,
                            to = %best_hint.model_slug,
                            baseline_score,
                            hint_score = best_hint.score,
                            bias_weight = bias_weight,
                            reason = %best_hint.reason,
                            supporting_entries = best_hint.supporting_entries,
                            "knowledge store nudged model selection"
                        );
                        dispatch_plan.model = ModelSpec::from_slug(best_hint.model_slug.clone());
                        selected_source = "dispatcher+knowledge".to_string();
                    }
                }
            }
            let mut dispatch_turn_limit = DEFAULT_AGENT_TURN_LIMIT;
            let mut dispatch_effort = None;
            let daimon_modulation = daimon_hook.as_ref().and_then(|hook| {
                daimon_dispatch_modulation(
                    ctx.config,
                    hook,
                    &dispatch_plan.model.slug,
                    allow_learned_model_modulation,
                )
            });
            if let Some(modulation) = &daimon_modulation {
                dispatch_turn_limit = modulation.turn_limit;
                dispatch_effort = Some(modulation.effort.clone());
                if modulation.model != dispatch_plan.model.slug {
                    debug!(
                        from = %dispatch_plan.model.slug,
                        to = %modulation.model,
                        effort = %modulation.effort,
                        turn_limit = modulation.turn_limit,
                        "daimon modulated dispatch"
                    );
                    dispatch_plan.model = ModelSpec::from_slug(modulation.model.clone());
                    selected_source = if selected_source == "dispatcher+knowledge" {
                        "dispatcher+knowledge+daimon".to_string()
                    } else {
                        "dispatcher+daimon".to_string()
                    };
                }
            }
            let requested_model = dispatch_plan.model.slug.clone();
            let prompt_diagnostics = dispatch_plan.prompt.diagnostics.clone();
            // Stash section diagnostics keyed by attempt for SectionOutcome
            // recording when the gate completes (pass or fail).
            {
                let attempt_key = format!("{plan_id}:{task_id}:{attempt_num}");
                // Store playbook IDs so gate terminal can call record_outcome.
                if !prompt_diagnostics.playbook_ids.is_empty() {
                    ctx.task_playbook_ids
                        .insert(attempt_key.clone(), prompt_diagnostics.playbook_ids.clone());
                }
                ctx.section_diagnostics
                    .insert(attempt_key, prompt_diagnostics.clone());
            }
            ctx.tui
                .model_selected(plan_id, &task_id, &requested_model, &selected_source);
            let mut system_prompt = dispatch_plan.prompt.system_prompt;
            if let Some(section) = daimon_hook.as_ref().and_then(render_daimon_prompt_context) {
                system_prompt.push_str("\n\n");
                system_prompt.push_str(&section);
            }
            let mut final_prompt = dispatch_plan.prompt.user_prompt;
            info!(
                plan_id = %plan_id,
                task = %task_id,
                model = %requested_model,
                source = %selected_source,
                system_prompt_len = system_prompt.len(),
                user_prompt_len = final_prompt.len(),
                estimated_tokens = prompt_diagnostics.estimated_tokens,
                included_sections = prompt_diagnostics.included_sections.len(),
                dropped_sections = prompt_diagnostics.dropped_sections.len(),
                "dispatch: model selected, prompt assembled"
            );
            debug!(
                plan_id = %plan_id,
                task = %task_id,
                included_sections = ?dispatch_plan.prompt.diagnostics.included_sections,
                dropped_sections = ?dispatch_plan.prompt.diagnostics.dropped_sections,
                knowledge_ids = ?dispatch_plan.prompt.diagnostics.knowledge_ids,
                playbook_ids = ?dispatch_plan.prompt.diagnostics.playbook_ids,
                "dispatch prompt detail"
            );

            // Append replan context before prompt diagnostics so the durable
            // event captures the actual prompt shape sent to the runtime.
            if let Some(replan) = ctx.state.take_replan_context(plan_id, &task_id) {
                final_prompt.push_str(&replan);
            }
            ctx.state.current_prompt_text = format!("{system_prompt}\n\n{final_prompt}");

            // Extension: pre-inference hook.
            let task_role = task_def.role.as_deref().unwrap_or("implementer");
            fire_pre_inference_hook(
                ctx.config,
                plan_id,
                &task_id,
                &requested_model,
                task_role,
                ctx.tui,
            )
            .await;

            let dispatch = match ctx.factory.resolve_runtime(&requested_model) {
                Ok(selection) => selection,
                Err(hint_err) => {
                    // Fall back to default model when model_hint can't be resolved
                    let default_model = &ctx.config.model;
                    warn!(
                        plan_id = %plan_id,
                        task = %task_id,
                        hint = %requested_model,
                        fallback = %default_model,
                        "model_hint resolution failed, falling back to default model"
                    );
                    match ctx.factory.resolve_runtime(default_model) {
                        Ok(selection) => selection,
                        Err(default_err) => {
                            let message = format!(
                                "model resolution failed: hint '{}': {}; default '{}': {}",
                                requested_model, hint_err, default_model, default_err
                            );
                            error!(plan_id = %plan_id, task = %task_id, error = %message);
                            if is_dag_task_spawn {
                                ctx.task_dag.clear_running(plan_id, &task_id);
                            }
                            ctx.task_runtime_states.remove(&task_key(plan_id, &task_id));
                            if let Err(e) = ctx
                                .executor
                                .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                            {
                                error!(plan_id = %plan_id, error = %e,
                                    "failed to apply Fatal event -- forcing plan terminal");
                                ctx.state.force_plan_terminal(plan_id);
                            }
                            ctx.tui.error(&message);
                            return ActionDispatchOutcome::Noop;
                        }
                    }
                }
            };

            let agent_id = format!("{plan_id}/{task_id}");
            let attempt_ref = TaskAttemptRef::new(plan_id.clone(), task_id.clone(), attempt_num);
            let agent_effect = EffectRef(
                (u64::from(attempt_num) << 32) | u64::from(ctx.state.task_agent_calls + 1),
            );
            if ctx
                .attempt_ownership
                .insert(
                    attempt_ref.clone(),
                    AttemptOwner::new(AttemptPhase::Dispatching, agent_effect),
                    AgentRuntimeResource::Dispatching(task_permit),
                )
                .is_err()
            {
                error!(attempt = %attempt_ref.key(), "duplicate dispatch ownership suppressed");
                return ActionDispatchOutcome::Noop;
            }
            let mut dispatch_claim = ctx
                .attempt_ownership
                .claim_phase(&attempt_ref, AttemptPhase::Dispatching, agent_effect)
                .expect("new dispatch ownership must be claimable");
            let run_id = ctx.state.run_id().to_string();
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::task_attempt_started(&run_id, attempt_ref.clone(), &task_def.title),
            );
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::prompt_assembled(
                    &run_id,
                    attempt_ref.clone(),
                    role,
                    &requested_model,
                    system_prompt.len(),
                    final_prompt.len(),
                    PromptAssemblyDiagnostics {
                        included_sections: prompt_diagnostics.included_sections,
                        dropped_sections: prompt_diagnostics.dropped_sections,
                        estimated_tokens: prompt_diagnostics.estimated_tokens,
                        knowledge_ids: prompt_diagnostics.knowledge_ids,
                        playbook_ids: prompt_diagnostics.playbook_ids,
                    },
                ),
            );
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::agent_dispatch_started(
                    &run_id,
                    attempt_ref.clone(),
                    &agent_id,
                    role,
                    &requested_model,
                ),
            );

            match dispatch {
                ResolvedAgentRuntime::Cli {
                    model,
                    cli_provider,
                } => {
                    let model_display = model.clone();
                    let mut spawn_config = AgentSpawnConfig::from_run_config(
                        ctx.config,
                        final_prompt,
                        system_prompt,
                        model,
                        agent_id.clone(),
                    );
                    spawn_config.workdir = plan_workdir.clone();
                    spawn_config.max_turns = dispatch_turn_limit;
                    spawn_config.effort = dispatch_effort.clone();
                    if let Some(provider) = cli_provider {
                        spawn_config = spawn_config.with_cli_provider(provider);
                    }

                    let AgentRuntimeResource::Dispatching(permit) =
                        dispatch_claim.replace_resource(AgentRuntimeResource::AwaitingGate)
                    else {
                        unreachable!("dispatch claim must own permit")
                    };
                    let (raw_agent_tx, raw_agent_rx) = mpsc::channel::<AgentEvent>(64);
                    let forwarder = tokio::spawn(forward_agent_events(
                        attempt_ref.clone(),
                        agent_effect,
                        agent_id.clone(),
                        raw_agent_rx,
                        ctx.agent_tx.clone(),
                    ));

                    match ctx
                        .factory
                        .dispatcher()
                        .spawn_streaming_cli_agent(&spawn_config, raw_agent_tx)
                        .await
                    {
                        Ok(handle) => {
                            ctx.state.agent_active = true;
                            ctx.state.agent_pid = Some(handle.pid);
                            emit_runner_event(
                                ctx.paths,
                                ctx.state,
                                ctx.tui,
                                ctx.config,
                                RunnerEvent::agent_dispatch_completed(
                                    &run_id,
                                    attempt_ref.clone(),
                                    &agent_id,
                                    AgentDispatchOutcome::Spawned,
                                    Some(model_display.clone()),
                                    Some(handle.pid),
                                    None,
                                ),
                            );
                            ctx.tui.agent_spawned(&agent_id, role, &model_display);
                            ctx.tui.task_started(
                                plan_id,
                                &task_id,
                                &task_def.title,
                                "implementing",
                            );
                            dispatch_claim.set_agent(agent_id.clone(), Some(handle.pid));
                            dispatch_claim.replace_resource(AgentRuntimeResource::Cli {
                                handle,
                                forwarder,
                                permit,
                            });
                            ctx.attempt_ownership
                                .transition_claim(dispatch_claim, AttemptPhase::Agent, agent_effect)
                                .expect("CLI dispatch must transition ownership");
                            ctx.task_runtime_states
                                .insert(active_task_key, TaskRuntimeState::capture(ctx.state));
                            register_agent_feed(ctx.config, plan_id, &task_id, &agent_id, ctx.tui);
                            return ActionDispatchOutcome::AgentStarted {
                                plan_id: plan_id.clone(),
                                task_id,
                            };
                        }
                        Err(e) => {
                            forwarder.abort();
                            if let Err(err) = forwarder.await
                                && !err.is_cancelled()
                            {
                                error!(%err, "spawn failure forwarder did not stop cleanly");
                            }
                            drop(permit);
                            error!(err = %e, "failed to spawn agent");
                            let message = format!("agent spawn failed: {e}");
                            if is_dag_task_spawn {
                                ctx.task_dag.clear_running(plan_id, &task_id);
                            }
                            ctx.task_runtime_states.remove(&task_key(plan_id, &task_id));
                            let agent_provider = ctx.state.agent_provider.clone();
                            emit_runner_event(
                                ctx.paths,
                                ctx.state,
                                ctx.tui,
                                ctx.config,
                                RunnerEvent::agent_dispatch_completed(
                                    &run_id,
                                    attempt_ref.clone(),
                                    &agent_id,
                                    AgentDispatchOutcome::SpawnFailed,
                                    Some(model_display.clone()),
                                    None,
                                    Some(message.clone()),
                                ),
                            );
                            emit_runner_event(
                                ctx.paths,
                                ctx.state,
                                ctx.tui,
                                ctx.config,
                                RunnerEvent::task_attempt_completed(
                                    &run_id,
                                    attempt_ref.clone(),
                                    TaskAttemptOutcome::Failed,
                                    Some(RunnerFailureKind::Resource),
                                    0,
                                    model_display,
                                    agent_provider,
                                ),
                            );
                            record_daimon_task_outcome(
                                ctx.config,
                                ctx.state.current_daimon_strategy,
                                plan_id,
                                &task_id,
                                false,
                                &message,
                            );
                            ctx.tui.error(&message);
                            ctx.attempt_ownership
                                .complete_claim(dispatch_claim)
                                .expect("spawn failure must release ownership");
                            if let Err(e2) = ctx.executor.apply_event(
                                plan_id,
                                &ExecutorEvent::Fatal(format!("spawn failed: {e}")),
                            ) {
                                error!(plan_id = %plan_id, error = %e2,
                                    "failed to apply Fatal event -- forcing plan terminal");
                                ctx.state.force_plan_terminal(plan_id);
                            }
                            return ActionDispatchOutcome::Noop;
                        }
                    }
                }
                ResolvedAgentRuntime::Bridge {
                    model,
                    provider_id,
                    roko_config,
                } => {
                    ctx.state.agent_active = true;
                    ctx.state.agent_pid = None;
                    let request = AgentDispatchRequest {
                        model_key: requested_model.clone(),
                        prompt: final_prompt,
                        system_prompt,
                        workdir: plan_workdir.clone(),
                        agent_id: agent_id.clone(),
                        command: None,
                        timeout_ms: Some(duration_millis(agent_dispatch_timeout(ctx.config))),
                        mcp_config: ctx.config.mcp_config.clone(),
                        env: vec![
                            ("CARGO_INCREMENTAL".to_string(), "0".to_string()),
                            ("CARGO_BUILD_JOBS".to_string(), "2".to_string()),
                        ],
                        extra_args: Vec::new(),
                        effort: dispatch_effort.clone(),
                        tools: None,
                        bare_mode: false,
                        dangerously_skip_permissions: ctx.config.dangerously_skip_permissions,
                    };
                    let AgentRuntimeResource::Dispatching(permit) =
                        dispatch_claim.replace_resource(AgentRuntimeResource::AwaitingGate)
                    else {
                        unreachable!("dispatch claim must own permit")
                    };
                    let (raw_agent_tx, raw_agent_rx) = mpsc::channel::<AgentEvent>(64);
                    let forwarder = tokio::spawn(forward_agent_events(
                        attempt_ref.clone(),
                        agent_effect,
                        agent_id.clone(),
                        raw_agent_rx,
                        ctx.agent_tx.clone(),
                    ));
                    let bridge = ctx.factory.spawn_shared_agent_bridge(request, raw_agent_tx);
                    emit_runner_event(
                        ctx.paths,
                        ctx.state,
                        ctx.tui,
                        ctx.config,
                        RunnerEvent::agent_dispatch_completed(
                            &run_id,
                            attempt_ref.clone(),
                            &agent_id,
                            AgentDispatchOutcome::Spawned,
                            Some(model.clone()),
                            None,
                            None,
                        ),
                    );
                    ctx.tui
                        .agent_spawned(&agent_id, role, &format!("{provider_id}:{model}"));
                    ctx.tui
                        .task_started(plan_id, &task_id, &task_def.title, "implementing");
                    dispatch_claim.set_agent(agent_id.clone(), None);
                    dispatch_claim.replace_resource(AgentRuntimeResource::Bridge {
                        bridge,
                        forwarder,
                        permit,
                    });
                    ctx.attempt_ownership
                        .transition_claim(dispatch_claim, AttemptPhase::Agent, agent_effect)
                        .expect("bridge dispatch must transition ownership");
                    ctx.task_runtime_states
                        .insert(active_task_key, TaskRuntimeState::capture(ctx.state));
                    register_agent_feed(ctx.config, plan_id, &task_id, &agent_id, ctx.tui);
                    return ActionDispatchOutcome::AgentStarted {
                        plan_id: plan_id.clone(),
                        task_id,
                    };
                }
            }
        }

        ExecutorAction::RunGate { plan_id, rung } => {
            let task_id = ctx
                .pending_gate_tasks
                .get(plan_id)
                .and_then(|pending| pending.first())
                .cloned()
                .unwrap_or_else(|| ctx.state.current_task.clone());
            restore_task_runtime(ctx.state, ctx.task_runtime_states, plan_id, &task_id);
            let plan_workdir = match tracked_plan_workdir(ctx.worktrees, plan_id) {
                Some(path) => path,
                None => {
                    let message = format!("isolated worktree missing for plan {plan_id}");
                    error!(plan_id = %plan_id, task_id = %task_id, "{}", message);
                    if let Err(e) = ctx
                        .executor
                        .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                    {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
                    ctx.tui.error(&message);
                    return ActionDispatchOutcome::Noop;
                }
            };
            let gates_config = gates_config_for_run(ctx.config);
            let pipeline_rung = ctx.config.max_gate_rung;
            // Default selected rungs are Cargo-oriented; custom rungs own their command semantics.
            let has_cargo_toml = std::fs::metadata(plan_workdir.join("Cargo.toml")).is_ok();
            if !gates_config.has_custom_rungs() && !has_cargo_toml {
                info!(plan_id = %plan_id, rung = pipeline_rung, "skipping default gate pipeline (no Cargo.toml in workspace)");
                record_skipped_gate_rung(
                    ctx,
                    plan_id,
                    &task_id,
                    pipeline_rung,
                    "gate-pipeline:default",
                    "skipped: no Cargo.toml in workspace",
                );
                return ActionDispatchOutcome::Handled;
            }

            let effect_key =
                gate_effect_key(plan_id, &task_id, pipeline_rung, GateCompletionKind::Gate);
            info!(
                plan_id = %plan_id,
                requested_rung = *rung,
                rung = pipeline_rung,
                custom_rungs = gates_config.has_custom_rungs(),
                "dispatching gate pipeline"
            );
            let run_id = ctx.state.run_id().to_string();
            let attempt_ref = TaskAttemptRef::new(
                plan_id.clone(),
                task_id.clone(),
                ctx.state.iteration_for(plan_id, &task_id),
            );
            let gate_effect =
                new_gate_effect(attempt_ref.clone(), GateCompletionKind::Gate, pipeline_rung);
            if !ctx.attempt_ownership.contains(&attempt_ref) {
                ctx.attempt_ownership
                    .insert(
                        attempt_ref.clone(),
                        AttemptOwner::new(AttemptPhase::AwaitingGate, EffectRef(0)),
                        AgentRuntimeResource::AwaitingGate,
                    )
                    .expect("new gate owner must be unique");
            }
            let prior_effect = ctx
                .attempt_ownership
                .current_effect(&attempt_ref)
                .expect("gate owner must expose current effect");
            let mut gate_claim = match ctx.attempt_ownership.claim_phase(
                &attempt_ref,
                AttemptPhase::AwaitingGate,
                prior_effect,
            ) {
                Ok(claim) => claim,
                Err(_) => return ActionDispatchOutcome::Noop,
            };
            if !ctx.state.mark_gate_active(effect_key.clone()) {
                ctx.attempt_ownership
                    .transition_claim(gate_claim, AttemptPhase::AwaitingGate, prior_effect)
                    .expect("duplicate gate suppression must restore ownership");
                debug!(
                    plan_id = %plan_id,
                    task_id = %task_id,
                    rung = pipeline_rung,
                    "gate pipeline already active - suppressing duplicate dispatch"
                );
                return ActionDispatchOutcome::Noop;
            }
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::gate_dispatch_started(
                    &run_id,
                    attempt_ref.clone(),
                    GateCompletionKind::Gate,
                    pipeline_rung,
                ),
            );
            let task_def = ctx
                .task_index
                .get(plan_id.as_str())
                .and_then(|tasks| tasks.get(task_id.as_str()));
            let is_read_only_role = task_role_is_read_only(task_def);

            let (gate_handle, start_tx) = if is_read_only_role {
                // Read-only tasks don't produce artifacts — auto-pass the gate.
                // Running cargo check / structural verify on a researcher task
                // wastes time and fails on files not yet created.
                //
                // IMPORTANT: Send via spawned task, NOT inline. Sending on
                // gate_tx from inside the select loop that reads gate_rx
                // would deadlock if the channel buffer is full.
                info!(
                    plan_id = %plan_id,
                    task_id = %task_id,
                    rung = pipeline_rung,
                    "skipping gate for read-only role"
                );
                let completion = GateCompletion {
                    plan_id: plan_id.clone(),
                    task_id: task_id.clone(),
                    attempt: Some(attempt_ref.clone()),
                    effect: Some(gate_effect.clone()),
                    rung: pipeline_rung,
                    passed: true,
                    output: "skipped: read-only role".to_string(),
                    failure_kind: None,
                    duration_ms: 0,
                    kind: GateCompletionKind::Gate,
                    verdicts: Vec::new(),
                };
                let gate_tx = ctx.gate_tx.clone();
                let fatal_tx = ctx.fatal_tx.clone();
                let plan_id_fatal = plan_id.clone();
                let fatal_attempt = attempt_ref.clone();
                let (start_tx, start_rx) = tokio::sync::oneshot::channel();
                let handle = tokio::spawn(async move {
                    if start_rx.await.is_err() {
                        return;
                    }
                    if let Err(e) = gate_tx.send(completion).await {
                        error!(plan_id = %plan_id_fatal, err = %e,
                            "CRITICAL: failed to send auto-pass gate -- sending fatal");
                        let _ = fatal_tx
                            .send(RoutedAgentEvent::fatal(
                                fatal_attempt,
                                format!("gate channel closed for plan {plan_id_fatal}: {e}"),
                            ))
                            .await;
                    }
                });
                (handle, start_tx)
            } else {
                let verify_steps = task_def.map(|task| task.verify.clone()).unwrap_or_default();
                let complexity = gate_plan_complexity_for_task(task_def);
                let target_crates = task_target_crates(task_def);
                gate_dispatch::spawn_gate(
                    gate_effect.clone(),
                    plan_id.clone(),
                    task_id,
                    pipeline_rung,
                    plan_workdir,
                    gates_config,
                    complexity,
                    verify_steps,
                    duration_secs(gate_timeout(ctx.config, pipeline_rung)),
                    ctx.gate_tx.clone(),
                    ctx.gate_sem.clone(),
                    target_crates,
                )
            };
            gate_claim.replace_resource(AgentRuntimeResource::Gate {
                effect: gate_effect.clone(),
                handle: gate_handle,
            });
            ctx.attempt_ownership
                .transition_claim(
                    gate_claim,
                    AttemptPhase::Gate,
                    EffectRef(gate_effect.generation),
                )
                .expect("gate dispatch must retain exact owner");
            if start_tx.send(()).is_err() {
                ctx.state.clear_gate_active(&effect_key);
                if let Ok(mut failed_claim) = ctx.attempt_ownership.claim_phase(
                    &attempt_ref,
                    AttemptPhase::Gate,
                    EffectRef(gate_effect.generation),
                ) {
                    if let AgentRuntimeResource::Gate { handle, .. } =
                        failed_claim.replace_resource(AgentRuntimeResource::AwaitingGate)
                    {
                        let _ = handle.await;
                    }
                    ctx.attempt_ownership.complete_claim(failed_claim).ok();
                }
                let message = format!("gate producer failed to start for {}", attempt_ref.key());
                error!(attempt = %attempt_ref.key(), %message);
                let _ = ctx
                    .executor
                    .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()));
                ctx.tui.error(&message);
                return ActionDispatchOutcome::Handled;
            }
            ActionDispatchOutcome::Handled
        }

        ExecutorAction::RunVerify { plan_id } => {
            let verify_steps = ctx
                .task_index
                .get(plan_id.as_str())
                .map(|tasks| {
                    let mut tasks: Vec<_> = tasks.values().collect();
                    tasks.sort_by_key(|t| t.sequence);
                    tasks
                        .into_iter()
                        .filter(|task| !task.verify.is_empty())
                        .map(|task| (task.id.clone(), task.verify.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if verify_steps.is_empty() {
                info!(plan_id = %plan_id, "dispatching owned no-step plan verify");
            }

            let plan_workdir = if verify_steps.is_empty() {
                ctx.config.workdir.clone()
            } else {
                match tracked_plan_workdir(ctx.worktrees, plan_id) {
                    Some(path) => path,
                    None => {
                        let message = format!("isolated worktree missing for plan {plan_id}");
                        error!(plan_id = %plan_id, "{}", message);
                        if let Err(e) = ctx
                            .executor
                            .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                        {
                            error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                            ctx.state.force_plan_terminal(plan_id);
                        }
                        ctx.tui.error(&message);
                        return ActionDispatchOutcome::Noop;
                    }
                }
            };

            let effect_key = gate_effect_key(
                plan_id,
                "plan-verify",
                u32::MAX,
                GateCompletionKind::PlanVerify,
            );
            let run_id = ctx.state.run_id().to_string();
            let attempt_ref = TaskAttemptRef::new(
                plan_id.clone(),
                "plan-verify",
                ctx.state.iteration_for(plan_id, "plan-verify"),
            );
            let gate_effect = new_gate_effect(
                attempt_ref.clone(),
                GateCompletionKind::PlanVerify,
                u32::MAX,
            );
            if !ctx.attempt_ownership.contains(&attempt_ref) {
                ctx.attempt_ownership
                    .insert(
                        attempt_ref.clone(),
                        AttemptOwner::new(AttemptPhase::AwaitingGate, EffectRef(0)),
                        AgentRuntimeResource::AwaitingGate,
                    )
                    .expect("plan verify owner must be unique");
            }
            let prior_effect = ctx
                .attempt_ownership
                .current_effect(&attempt_ref)
                .expect("plan verify owner must expose current effect");
            let mut gate_claim = match ctx.attempt_ownership.claim_phase(
                &attempt_ref,
                AttemptPhase::AwaitingGate,
                prior_effect,
            ) {
                Ok(claim) => claim,
                Err(_) => return ActionDispatchOutcome::Noop,
            };
            if !ctx.state.mark_gate_active(effect_key.clone()) {
                ctx.attempt_ownership
                    .transition_claim(gate_claim, AttemptPhase::AwaitingGate, prior_effect)
                    .expect("suppressed plan verify must restore owner");
                return ActionDispatchOutcome::Noop;
            }
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::gate_dispatch_started(
                    &run_id,
                    attempt_ref.clone(),
                    GateCompletionKind::PlanVerify,
                    u32::MAX,
                ),
            );

            info!(
                plan_id = %plan_id,
                task_count = verify_steps.len(),
                "dispatching plan verify"
            );
            let (gate_handle, start_tx) = gate_dispatch::spawn_plan_verify(
                gate_effect.clone(),
                plan_id.clone(),
                plan_workdir,
                verify_steps,
                duration_secs(gate_timeout(ctx.config, 2)),
                ctx.gate_tx.clone(),
                ctx.gate_sem.clone(),
            );
            gate_claim.replace_resource(AgentRuntimeResource::Gate {
                effect: gate_effect.clone(),
                handle: gate_handle,
            });
            ctx.attempt_ownership
                .transition_claim(
                    gate_claim,
                    AttemptPhase::Gate,
                    EffectRef(gate_effect.generation),
                )
                .expect("plan verify must retain exact owner");
            if start_tx.send(()).is_err() {
                ctx.state.clear_gate_active(&effect_key);
                if let Ok(mut claim) = ctx.attempt_ownership.claim_phase(
                    &attempt_ref,
                    AttemptPhase::Gate,
                    EffectRef(gate_effect.generation),
                ) {
                    if let AgentRuntimeResource::Gate { handle, .. } =
                        claim.replace_resource(AgentRuntimeResource::AwaitingGate)
                    {
                        let _ = handle.await;
                    }
                    ctx.attempt_ownership.complete_claim(claim).ok();
                }
                let message = format!("plan verify producer failed to start for {plan_id}");
                let _ = ctx
                    .executor
                    .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()));
                ctx.tui.error(&message);
            }
            ActionDispatchOutcome::Handled
        }

        ExecutorAction::CompletePlan { plan_id } => {
            info!(plan_id = %plan_id, "plan completed");
            ctx.tui.plan_completed(plan_id, true);
            let run_id = ctx.state.run_id().to_string();
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::plan_completed(&run_id, plan_id, PlanOutcome::Succeeded, None),
            );
            save_snapshot(
                ctx.config,
                ctx.executor,
                ctx.paths,
                ctx.state,
                ctx.merge_queue,
                ctx.gate_thresholds,
                ctx.snapshot_writer,
            );
            ActionDispatchOutcome::Handled
        }

        ExecutorAction::FailPlan { plan_id, reason } => {
            warn!(plan_id = %plan_id, reason = %reason, "plan failed");
            ctx.state.tasks_failed += 1;
            ctx.state.roll_into_totals();
            ctx.tui
                .task_completed(plan_id, &ctx.state.current_task, "failed");
            ctx.tui.plan_completed(plan_id, false);
            let run_id = ctx.state.run_id().to_string();
            emit_runner_event(
                ctx.paths,
                ctx.state,
                ctx.tui,
                ctx.config,
                RunnerEvent::plan_completed(
                    &run_id,
                    plan_id,
                    PlanOutcome::Failed,
                    Some(reason.clone()),
                ),
            );
            ActionDispatchOutcome::Handled
        }

        ExecutorAction::MergeBranch { plan_id } => {
            if tracked_plan_workdir(ctx.worktrees, plan_id).is_none() {
                let message = format!("isolated worktree missing for plan {plan_id}");
                error!(plan_id = %plan_id, "{}", message);
                if let Err(e) = ctx
                    .executor
                    .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                {
                    error!(plan_id = %plan_id, error = %e,
                        "failed to apply Fatal event -- forcing plan terminal");
                    ctx.state.force_plan_terminal(plan_id);
                }
                ctx.tui.error(&message);
                return ActionDispatchOutcome::Noop;
            }
            let files_changed = ctx
                .executor
                .plan_state(plan_id)
                .map(|state| state.files_changed.clone())
                .unwrap_or_default();
            let request = MergeRequest::new(
                plan_id.clone(),
                format_branch_name(plan_id),
                files_changed,
                0,
            );
            let merger = PlanMerger::new(
                ctx.merge_queue.clone(),
                PlanMergerConfig::new(ctx.config.workdir.clone(), gate_timeout(ctx.config, 0)),
            );
            match merger.submit(request) {
                MergeDispatch::AlreadyActive { plan_id } => {
                    debug!(plan_id = %plan_id, "duplicate active merge submission suppressed");
                }
                MergeDispatch::Reserved { launch } => {
                    let plan_id = launch.plan_id().to_string();
                    let branch_name = launch.branch_name().to_string();
                    start_legacy_merge(&merger, launch, ctx.gate_tx.clone());
                    info!(
                        plan_id = %plan_id,
                        branch = %branch_name,
                        "reserved merge queue request"
                    );
                    save_snapshot(
                        ctx.config,
                        ctx.executor,
                        ctx.paths,
                        ctx.state,
                        ctx.merge_queue,
                        ctx.gate_thresholds,
                        ctx.snapshot_writer,
                    );
                }
                MergeDispatch::Blocked { plan_id, launch } => {
                    if let Some(launch) = launch {
                        start_legacy_merge(&merger, launch, ctx.gate_tx.clone());
                    }
                    info!(
                        plan_id = %plan_id,
                        blocked_conflicts = ?ctx.merge_queue.blocked_conflicts(),
                        "merge queued but currently blocked by file locks"
                    );
                    save_snapshot(
                        ctx.config,
                        ctx.executor,
                        ctx.paths,
                        ctx.state,
                        ctx.merge_queue,
                        ctx.gate_thresholds,
                        ctx.snapshot_writer,
                    );
                }
            }
            ActionDispatchOutcome::Handled
        }

        _ => {
            info!(action = ?action, "auto-advancing action");
            ActionDispatchOutcome::Handled
        }
    }
}

// ─── Adaptive gate thresholds ────────────────────────────────────────────

/// Update EMA-based adaptive gate thresholds for a given rung.
fn update_gate_thresholds(thresholds: &mut GateThresholds, rung: u32, passed: bool) {
    thresholds.observe(rung, passed);
    // Gate thresholds are now persisted as part of the unified StateSnapshot
    // in save_snapshot(), not via a separate disk write.
}

/// Emit gate thresholds into the TUI push pipeline after updating in memory.
fn emit_gate_thresholds_event(thresholds: &GateThresholds, tui: &TuiBridge) {
    if let Ok(json) = serde_json::to_string(thresholds) {
        tui.gate_thresholds_updated(&json);
    }
}

// ─── Section outcome helpers ─────────────────────────────────────────────

/// Build lightweight `SectionOutcomeRecord` entries from prompt diagnostics
/// and a terminal gate result. Each included/dropped section becomes one
/// record so the downstream bandit can attribute section presence to
/// pass/fail.
fn build_section_outcome_records(
    plan_id: &str,
    task_id: &str,
    model: &str,
    provider: &str,
    status: SectionOutcomeStatus,
    diag: &PromptDiagnostics,
    verdicts: &[GateVerdictSummary],
) -> Vec<SectionOutcomeRecord> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let gate_outcomes = verdicts
        .iter()
        .map(|v| roko_learn::section_outcome::SectionGateOutcome {
            gate_id: v.gate_name.clone(),
            outcome: if v.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            required: true,
        })
        .collect::<Vec<_>>();

    let mut records =
        Vec::with_capacity(diag.included_sections.len() + diag.dropped_sections.len());

    for section_name in &diag.included_sections {
        records.push(SectionOutcomeRecord {
            schema_version: SECTION_OUTCOME_SCHEMA_VERSION,
            timestamp: timestamp.clone(),
            workspace_id: String::new(),
            invocation_id: format!("{plan_id}:{task_id}"),
            task_id: task_id.to_string(),
            task_type: "plan_task".to_string(),
            role_id: String::new(),
            provider: provider.to_string(),
            model: model.to_string(),
            section_id: section_name.clone(),
            section_name: section_name.clone(),
            action_id: format!("prompt_section:{section_name}"),
            section_kind: roko_learn::section_outcome::SectionKind::Prompt,
            included: true,
            estimated_tokens: 0,
            tokens_used: 0,
            token_budget: None,
            source_type: None,
            source_id: None,
            experiment_id: None,
            status,
            gate_outcomes: gate_outcomes.clone(),
            review_verdicts: Vec::new(),
        });
    }

    for section_name in &diag.dropped_sections {
        records.push(SectionOutcomeRecord {
            schema_version: SECTION_OUTCOME_SCHEMA_VERSION,
            timestamp: timestamp.clone(),
            workspace_id: String::new(),
            invocation_id: format!("{plan_id}:{task_id}"),
            task_id: task_id.to_string(),
            task_type: "plan_task".to_string(),
            role_id: String::new(),
            provider: provider.to_string(),
            model: model.to_string(),
            section_id: section_name.clone(),
            section_name: section_name.clone(),
            action_id: format!("prompt_section:{section_name}"),
            section_kind: roko_learn::section_outcome::SectionKind::Prompt,
            included: false,
            estimated_tokens: 0,
            tokens_used: 0,
            token_budget: None,
            source_type: None,
            source_id: None,
            experiment_id: None,
            status,
            gate_outcomes: gate_outcomes.clone(),
            review_verdicts: Vec::new(),
        });
    }

    records
}

/// Append section outcome records to the JSONL store. Failures are logged
/// but do not abort the run.
async fn append_section_outcomes(path: PathBuf, records: Vec<SectionOutcomeRecord>) {
    if records.is_empty() {
        return;
    }
    match SectionOutcomeStore::open_creating(path).await {
        Ok(store) => {
            if let Err(err) = store.append_many(&records).await {
                warn!(err = %err, "failed to append section outcome records");
            }
        }
        Err(err) => warn!(err = %err, "failed to open section outcome store"),
    }
}

fn parse_dispatch_role(role: &str) -> AgentRole {
    match role.trim().to_ascii_lowercase().as_str() {
        "conductor" => AgentRole::Conductor,
        "strategist" => AgentRole::Strategist,
        "implementer" => AgentRole::Implementer,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" | "reviewer" => AgentRole::Auditor,
        "quick-reviewer" | "quick_reviewer" => AgentRole::QuickReviewer,
        "scribe" => AgentRole::Scribe,
        "critic" => AgentRole::Critic,
        "auto-fixer" => AgentRole::AutoFixer,
        "refactorer" => AgentRole::Refactorer,
        "pre-planner" => AgentRole::PrePlanner,
        "doc-verifier" => AgentRole::DocVerifier,
        "integration-tester" => AgentRole::IntegrationTester,
        "merge-resolver" => AgentRole::MergeResolver,
        "terminal-validator" => AgentRole::TerminalValidator,
        "golem-lifecycle-tester" => AgentRole::GolemLifecycleTester,
        "spec-drift-detector" => AgentRole::SpecDriftDetector,
        "regression-detector" => AgentRole::RegressionDetector,
        "performance-sentinel" => AgentRole::PerformanceSentinel,
        "coverage-tracker" => AgentRole::CoverageTracker,
        "plan-lifecycle-manager" | "plan-lifecycle-mgr" => AgentRole::PlanLifecycleManager,
        "cross-system-tester" => AgentRole::CrossSystemTester,
        "error-diagnoser" => AgentRole::ErrorDiagnoser,
        "dep-validator" | "dependency-validator" => AgentRole::DependencyValidator,
        "pattern-extractor" => AgentRole::PatternExtractor,
        "snapshot-comparator" => AgentRole::SnapshotComparator,
        "full-loop-validator" => AgentRole::FullLoopValidator,
        _ => AgentRole::Implementer,
    }
}

fn candidate_model_slugs(config: &RunConfig) -> Vec<String> {
    let mut slugs = if let Some(router) = &config.cascade_router {
        router.model_slugs().to_vec()
    } else if let Some(roko_config) = &config.roko_config {
        roko_config.effective_models().keys().cloned().collect()
    } else {
        Vec::new()
    };
    slugs.sort();
    slugs.dedup();
    slugs
}

fn knowledge_bias_weight(config: &RunConfig) -> f64 {
    config
        .roko_config
        .as_ref()
        .map(|cfg| {
            // Prefer the dedicated knowledge_bias weight; fall back to latency.
            cfg.routing
                .weights
                .default
                .knowledge_bias
                .unwrap_or(cfg.routing.weights.default.latency)
        })
        .unwrap_or(0.2)
        .clamp(0.0, 1.0)
}

// ─── Extension Chain Hooks ───────────────────────────────────────────────

/// Fire pre_inference extension hook (non-blocking try_lock to avoid stalling select).
async fn fire_pre_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    tui: &TuiBridge,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping pre_inference hook");
        return;
    };
    let mut req = roko_core::extension::InferenceRequest {
        plan_id: plan_id.to_string(),
        task: task_id.to_string(),
        role: role.to_string(),
        model: model.to_string(),
        prompt_tokens: 0,
        extra: serde_json::Value::Null,
    };
    let success = chain.run_pre_inference(&mut req).await.is_ok();
    if !success {
        warn!("extension pre_inference hook failed");
    }
    tui.extension_hook(plan_id, task_id, "pre_inference", success);
}

/// Fire post_inference extension hook.
async fn fire_post_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    success: bool,
    cost_usd: f64,
    wall_ms: u64,
    tui: &TuiBridge,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping post_inference hook");
        return;
    };
    let mut resp = roko_core::extension::InferenceResponse {
        plan_id: plan_id.to_string(),
        task: task_id.to_string(),
        role: role.to_string(),
        model: model.to_string(),
        success,
        cost_usd,
        wall_ms,
        extra: serde_json::Value::Null,
    };
    let hook_ok = chain.run_post_inference(&mut resp).await.is_ok();
    if !hook_ok {
        warn!("extension post_inference hook failed");
    }
    tui.extension_hook(plan_id, task_id, "post_inference", hook_ok);
}

/// Fire on_gate extension hook.
async fn fire_on_gate_hook(config: &RunConfig, completion: &GateCompletion, tui: &TuiBridge) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping on_gate hook");
        return;
    };
    for verdict in &completion.verdicts {
        let mut event = roko_core::extension::GateEvent {
            plan_id: completion.plan_id.clone(),
            gate_name: verdict.gate_name.clone(),
            passed: verdict.passed,
            rung: format!("rung-{}", completion.rung),
            duration_ms: completion.duration_ms,
            details: serde_json::Value::Null,
        };
        let hook_ok = chain.run_on_gate(&mut event).await.is_ok();
        if !hook_ok {
            warn!(gate = %verdict.gate_name, "extension on_gate hook failed");
        }
        tui.extension_hook(
            &completion.plan_id,
            &completion.task_id,
            &format!("on_gate:{}", verdict.gate_name),
            hook_ok,
        );
    }
}

/// Fire on_error extension hook.
async fn fire_on_error_hook(
    config: &RunConfig,
    message: &str,
    source: &str,
    tui: &TuiBridge,
    plan_id: &str,
    task_id: &str,
) {
    let Some(ext_chain) = &config.extension_chain else {
        return;
    };
    let Ok(chain) = ext_chain.try_lock() else {
        warn!("extension chain lock contended, skipping on_error hook");
        return;
    };
    let event = roko_core::extension::ErrorEvent {
        error_message: message.to_string(),
        source: source.to_string(),
        extra: serde_json::Value::Null,
    };
    let hook_ok = chain.run_on_error(&event).await.is_ok();
    tui.extension_hook(plan_id, task_id, "on_error", hook_ok);
}

/// Shutdown extension chain + persist cascade router.
async fn shutdown_subsystems(config: &RunConfig, tui: &TuiBridge) {
    // Extension chain shutdown.
    if let Some(ext_chain) = &config.extension_chain {
        let mut chain = ext_chain.lock().await;
        let errors = chain.shutdown_all().await;
        for (name, err) in &errors {
            warn!(extension = %name, error = %err, "extension shutdown failed");
        }
    }

    // Persist cascade router learned state.
    if let Some(router) = &config.cascade_router {
        let router_path = config.layout.cascade_router_path();
        if let Err(err) = router.save(&router_path) {
            warn!(error = %err, "failed to persist cascade router");
        } else {
            info!("cascade router state persisted");
            tui.cascade_router_updated(&router.snapshot_json());
        }
    }
}

/// Compact the episode log if it exceeds the retention threshold.
///
/// Uses the default [`RetentionPolicy`] (200 episodes, 90 days).
/// Errors are logged but never propagated — compaction is best-effort.
async fn compact_episodes_if_needed(episodes_path: &std::path::Path) {
    use roko_learn::episode_logger::{EpisodeLogger, RetentionPolicy};

    // Use metadata probe instead of .exists() to avoid TOCTOU.
    // If the file doesn't exist, there's nothing to compact.
    match std::fs::metadata(episodes_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            warn!(error = %e, "failed to stat episodes file");
            return;
        }
        Ok(_) => {}
    }

    let logger = EpisodeLogger::new(episodes_path.to_path_buf());
    let policy = RetentionPolicy::default();
    let now = chrono::Utc::now();

    match logger.compact(now, &policy).await {
        Ok(stats) if stats.removed > 0 => {
            info!(
                before = stats.before,
                after = stats.after,
                removed = stats.removed,
                bytes_reclaimed = stats.bytes_reclaimed,
                "episode log compacted"
            );
        }
        Ok(_) => {} // nothing to compact
        Err(err) => {
            warn!(error = %err, "episode compaction failed (best-effort)");
        }
    }
}

async fn handle_plan_timeout(
    executor: &ParallelExecutor,
    plans: &[Plan],
    state: &mut RunState,
    attempt_ownership: &mut AttemptOwnership<AgentRuntimeResource>,
    paths: &PersistPaths,
    merge_queue: &MergeQueue,
    tui: &TuiBridge,
    config: &RunConfig,
    gate_thresholds: &GateThresholds,
    writer: &SnapshotWriter,
) -> Result<()> {
    let in_flight = collect_in_flight_attempts(state);
    let timeout_secs = duration_secs(plan_total_timeout(config));
    error!(
        timeout_secs,
        current_plan = %state.plan_id,
        current_task = %state.current_task,
        active_plans = ?executor.active_plans(),
        in_flight_attempts = ?in_flight,
        "plan execution exceeded wall-clock timeout"
    );
    stop_all_agents(attempt_ownership, state, Duration::from_secs(3)).await;
    save_snapshot(
        config,
        executor,
        paths,
        state,
        merge_queue,
        gate_thresholds,
        writer,
    );
    writer.flush();
    shutdown_subsystems(config, tui).await;
    let event = build_run_completed_event(executor, plans, state, RunOutcome::Failed);
    emit_runner_event(paths, state, tui, config, event);
    Err(anyhow::anyhow!(
        "plan execution exceeded wall-clock timeout after {} seconds",
        timeout_secs
    ))
}

fn collect_in_flight_attempts(state: &RunState) -> Vec<String> {
    let mut attempts = state
        .lifecycle
        .task_attempts
        .values()
        .filter(|attempt| {
            !matches!(
                attempt.status,
                TaskAttemptStatus::Passed
                    | TaskAttemptStatus::Failed
                    | TaskAttemptStatus::Exhausted
                    | TaskAttemptStatus::Cancelled
            )
        })
        .map(|attempt| format!("{}:{:?}", attempt.attempt.key(), attempt.status))
        .collect::<Vec<_>>();
    attempts.sort();
    attempts
}

async fn stop_all_agents(
    attempt_ownership: &mut AttemptOwnership<AgentRuntimeResource>,
    state: &mut RunState,
    grace: Duration,
) {
    let mut unconfirmed_pids = HashSet::new();
    for attempt in attempt_ownership.attempts() {
        let Ok(mut claim) = attempt_ownership.claim_cancellation(&attempt) else {
            continue;
        };
        let resource = claim.replace_resource(AgentRuntimeResource::AwaitingGate);
        match resource {
            AgentRuntimeResource::Cli {
                handle,
                forwarder,
                permit,
            } => {
                let pid = handle.pid;
                match handle.kill(grace).await {
                    AgentTermination::Confirmed { .. } => roko_agent::process::unregister_pid(pid),
                    AgentTermination::Failed { errors, .. } => {
                        error!(pid, ?errors, "agent termination could not be confirmed");
                        unconfirmed_pids.insert(pid);
                    }
                }
                forwarder.abort();
                let _ = forwarder.await;
                drop(permit);
            }
            AgentRuntimeResource::Bridge {
                bridge,
                forwarder,
                permit,
            } => {
                bridge.abort();
                forwarder.abort();
                let _ = bridge.await;
                let _ = forwarder.await;
                drop(permit);
            }
            AgentRuntimeResource::Dispatching(permit) => drop(permit),
            AgentRuntimeResource::AwaitingGate => {}
            AgentRuntimeResource::Gate { handle, .. } => {
                handle.abort();
                let _ = handle.await;
            }
        }
        let _ = attempt_ownership.complete_claim(claim);
    }
    if let Some(pid) = state.agent_pid.take() {
        if !unconfirmed_pids.contains(&pid) {
            roko_agent::process::unregister_pid(pid);
        }
    }
    state.agent_active = false;
    state.agent_pid = None;
    state.agent_turn_completed = false;
}

async fn run_dream_consolidation_if_enabled(config: &RunConfig) {
    let Some(roko_config) = config.roko_config.as_ref() else {
        debug!("no roko config -- skipping dream consolidation");
        return;
    };

    if !roko_config.learning.dream_on_completion {
        debug!("dream consolidation after plan completion disabled");
        return;
    }

    debug!("running dream consolidation after plan completion");
    run_dream_consolidation(config).await;
}

async fn run_dream_consolidation(config: &RunConfig) {
    let workdir = config.workdir.clone();
    let timeout = llm_call_timeout(config);
    let dream_config = roko_dreams::DreamLoopConfig {
        auto_dream: true,
        idle_threshold_mins: 0,
        min_episodes_for_dream: 1,
        agent: roko_dreams::DreamAgentConfig {
            command: "claude".to_string(),
            args: Vec::new(),
            model: None,
            bare_mode: true,
            effort: "low".to_string(),
            fallback_model: None,
            timeout_ms: duration_millis(timeout),
            env: Vec::new(),
        },
    };
    let join = tokio::task::spawn_blocking(move || {
        let mut dream_runner = roko_dreams::DreamRunner::new(workdir.clone(), dream_config);
        dream_runner.consolidate_now()
    });
    match tokio::time::timeout(timeout, join).await {
        Ok(Ok(Ok(report))) => info!(
            processed_episodes = report.processed_episodes,
            knowledge_entries = report.knowledge_entries_written,
            playbooks = report.playbooks_created,
            "dream consolidation completed"
        ),
        Ok(Ok(Err(err))) => {
            warn!(error = %err, "dream consolidation failed — plan results unaffected")
        }
        Ok(Err(join_err)) => warn!(error = %join_err, "dream consolidation worker aborted"),
        Err(_) => warn!(
            timeout_secs = duration_secs(timeout),
            "dream consolidation timed out — skipping"
        ),
    }
}

/// Register an agent feed entry after successful spawn.
fn register_agent_feed(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    agent_id: &str,
    tui: &TuiBridge,
) {
    let Some(registry) = &config.feed_registry else {
        return;
    };
    if let Ok(mut reg) = registry.try_lock() {
        reg.register(roko_core::FeedInfo {
            id: String::new(), // Auto-assigned by registry
            name: format!("{plan_id}/{task_id}"),
            agent_id: agent_id.to_string(),
            kind: roko_core::FeedKind::Raw,
            access: roko_core::FeedAccess::Private,
            description: String::new(),
            schema: None,
            created_at: chrono::Utc::now(),
        });
        tui.extension_hook(plan_id, task_id, "feed_registered", true);
    }
}

// ─── Playbook Seeding ────────────────────────────────────────────────────

/// Seed the playbook store with starter templates when empty.
///
/// This solves the chicken-and-egg problem: playbooks are normally only
/// saved on task SUCCESS, but without playbooks the system has no guidance
/// from the start. These seeds give the first few runs structured advice.
async fn seed_playbooks_if_empty(layout: &RokoLayout) {
    use roko_learn::playbook::{Playbook, PlaybookStep, PlaybookStore};

    let pb_dir = layout.playbooks_dir();

    // Quick check: if the directory has any .json files, skip seeding.
    // Use read_dir directly instead of exists() to avoid TOCTOU race.
    match tokio::fs::read_dir(&pb_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    debug!("playbook store already has entries, skipping seed");
                    return;
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Directory doesn't exist yet — will be created by PlaybookStore
        }
        Err(e) => {
            warn!(error = %e, dir = %pb_dir.display(), "failed to read playbook dir");
            return;
        }
    }

    info!("playbook store empty — seeding with starter templates");

    let store = PlaybookStore::new(&pb_dir);

    let seeds: Vec<Playbook> = vec![
        {
            let mut pb = Playbook::new(
                "minimal-edit",
                "Make targeted edits to existing code. Keep diffs under 30 lines. Do not create new files unless explicitly required.",
            );
            pb.name = "Minimal Edit".to_string();
            pb.steps = vec![
                PlaybookStep::new(
                    0,
                    "Search codebase for the relevant function/type",
                    "search",
                    vec!["file_found".into()],
                ),
                PlaybookStep::new(
                    1,
                    "Read the target file to understand context",
                    "read_file",
                    vec!["context_loaded".into()],
                ),
                PlaybookStep::new(
                    2,
                    "Make the minimal edit that satisfies the requirement",
                    "edit_file",
                    vec!["file_modified".into()],
                ),
                PlaybookStep::new(
                    3,
                    "Verify the change compiles",
                    "run_command",
                    vec!["compile_success".into()],
                ),
            ];
            pb
        },
        {
            let mut pb = Playbook::new(
                "test-first",
                "Write or update tests first, then implement. Verify tests pass before finishing.",
            );
            pb.name = "Test First".to_string();
            pb.steps = vec![
                PlaybookStep::new(
                    0,
                    "Identify the test file for the target module",
                    "search",
                    vec!["test_file_found".into()],
                ),
                PlaybookStep::new(
                    1,
                    "Write a failing test that captures the requirement",
                    "edit_file",
                    vec!["test_added".into()],
                ),
                PlaybookStep::new(
                    2,
                    "Implement the code to make the test pass",
                    "edit_file",
                    vec!["implementation_done".into()],
                ),
                PlaybookStep::new(
                    3,
                    "Run the test suite and verify all tests pass",
                    "run_command",
                    vec!["tests_pass".into()],
                ),
            ];
            pb
        },
        {
            let mut pb = Playbook::new(
                "grep-before-write",
                "Search the codebase before writing new code. Check if the function/type already exists.",
            );
            pb.name = "Grep Before Write".to_string();
            pb.steps = vec![
                PlaybookStep::new(
                    0,
                    "Search for existing implementations of the target",
                    "search",
                    vec!["search_complete".into()],
                ),
                PlaybookStep::new(
                    1,
                    "If found, extend or modify rather than duplicate",
                    "read_file",
                    vec!["existing_found".into()],
                ),
                PlaybookStep::new(
                    2,
                    "Implement changes in the existing location",
                    "edit_file",
                    vec!["change_applied".into()],
                ),
                PlaybookStep::new(
                    3,
                    "Verify no duplicate definitions introduced",
                    "search",
                    vec!["no_duplicates".into()],
                ),
            ];
            pb
        },
        {
            let mut pb = Playbook::new(
                "wire-not-build",
                "Connect existing code rather than reimplementing. Check what already exists before creating anything new.",
            );
            pb.name = "Wire Not Build".to_string();
            pb.steps = vec![
                PlaybookStep::new(
                    0,
                    "Search for the target struct/function in the codebase",
                    "search",
                    vec!["target_found".into()],
                ),
                PlaybookStep::new(
                    1,
                    "Trace the call chain to find where it should be wired",
                    "read_file",
                    vec!["call_site_found".into()],
                ),
                PlaybookStep::new(
                    2,
                    "Add the function call or import at the correct call site",
                    "edit_file",
                    vec!["wired".into()],
                ),
                PlaybookStep::new(
                    3,
                    "Verify the feature is accessible via CLI or API",
                    "run_command",
                    vec!["feature_reachable".into()],
                ),
            ];
            pb
        },
        {
            let mut pb = Playbook::new(
                "compile-check-loop",
                "After every edit, run cargo check. Fix errors immediately before proceeding to the next change.",
            );
            pb.name = "Compile Check Loop".to_string();
            pb.steps = vec![
                PlaybookStep::new(
                    0,
                    "Make a single logical change",
                    "edit_file",
                    vec!["change_made".into()],
                ),
                PlaybookStep::new(
                    1,
                    "Run cargo check to verify compilation",
                    "run_command",
                    vec!["compile_success".into()],
                ),
                PlaybookStep::new(
                    2,
                    "If errors, fix them before proceeding",
                    "edit_file",
                    vec!["errors_fixed".into()],
                ),
                PlaybookStep::new(
                    3,
                    "Repeat until all changes are applied and compiling",
                    "run_command",
                    vec!["all_clean".into()],
                ),
            ];
            pb
        },
    ];

    for pb in &seeds {
        if let Err(err) = store.save(pb).await {
            warn!(playbook = %pb.id, error = %err, "failed to seed playbook");
        } else {
            debug!(playbook = %pb.id, "seeded playbook");
        }
    }

    info!(
        count = seeds.len(),
        "playbook store seeded with starter templates"
    );
}

// ─── Run Ledger Helpers ──────────────────────────────────────────────────

/// Append a single typed entry to the run ledger JSONL file.
/// Failures are logged but never propagated — the ledger is best-effort.
fn append_ledger_entry(path: &std::path::Path, kind: &str, data: &serde_json::Value) {
    let entry = serde_json::json!({
        "kind": kind,
        "ts": chrono::Utc::now().to_rfc3339(),
        "data": data,
    });
    let line = match serde_json::to_string(&entry) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "failed to serialize ledger entry");
            return;
        }
    };
    if let Err(e) = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(f, "{}", line)?;
        Ok(())
    })() {
        warn!(error = %e, path = %path.display(), "failed to append to run ledger");
    }
}

/// Persist a final summary entry for the run ledger at run completion.
/// This is a no-op if the ledger was never initialized.
fn persist_run_ledger(ledger: &Option<RunLedger>, path: &std::path::Path) {
    let Some(ledger) = ledger else { return };
    let summary = serde_json::json!({
        "kind": "run_summary",
        "ts": chrono::Utc::now().to_rfc3339(),
        "data": {
            "run_id": ledger.run_id,
            "started_at_ms": ledger.started_at_ms,
            "phase_transitions": ledger.phase_history.len(),
            "agent_outcomes": ledger.agent_outcomes.len(),
            "gate_runs": ledger.gate_runs.len(),
        },
    });
    let line = match serde_json::to_string(&summary) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "failed to serialize run ledger summary");
            return;
        }
    };
    if let Err(e) = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(f, "{}", line)?;
        Ok(())
    })() {
        warn!(error = %e, path = %path.display(), "failed to persist run ledger summary");
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────

/// Collect files modified or created since the last commit with diff stats.
///
/// Uses two git queries:
/// - `git diff --numstat HEAD` — tracked files with unstaged/staged changes
/// - `git status --porcelain` — includes untracked (`??`) files
///
/// The combined list is deduped and capped at 50 entries.
fn git_diff_entries_since_task_start(workdir: &Path) -> Vec<DiffEntry> {
    let mut entries: Vec<DiffEntry> = Vec::new();

    // Modified tracked files.
    if let Ok(output) = std::process::Command::new("git")
        .args(["diff", "--numstat", "HEAD"])
        .current_dir(workdir)
        .output()
    {
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let mut parts = line.splitn(3, '\t');
                let additions = parts
                    .next()
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(0);
                let deletions = parts
                    .next()
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(0);
                if let Some(path) = parts.next().map(str::trim).filter(|path| !path.is_empty()) {
                    entries.push(DiffEntry {
                        path: path.to_string(),
                        additions,
                        deletions,
                        summary: None,
                    });
                }
            }
        }
    }

    // Untracked files are not present in `git diff --numstat HEAD`.
    if let Ok(output) = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workdir)
        .output()
    {
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("?? ") {
                    let path = trimmed.splitn(2, ' ').nth(1).unwrap_or("").trim();
                    if !path.is_empty() {
                        entries.push(DiffEntry {
                            path: path.to_string(),
                            additions: 0,
                            deletions: 0,
                            summary: Some("untracked".to_string()),
                        });
                    }
                }
            }
        }
    }

    // Dedup while preserving order, cap at 50.
    let mut seen = std::collections::HashSet::new();
    entries.retain(|entry| seen.insert(entry.path.clone()));
    entries.truncate(50);
    entries
}

fn normalized_task_git_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed.starts_with('/')
        || trimmed.starts_with("../")
        || trimmed.contains("/../")
    {
        return None;
    }
    Some(trimmed.trim_end_matches('/').to_string())
}

fn task_declared_git_paths(declared_files: &[String]) -> Vec<String> {
    let mut paths = declared_files
        .iter()
        .filter_map(|path| normalized_task_git_path(path))
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn task_path_allowed_by_declared_files(path: &str, declared_files: &[String]) -> bool {
    let Some(path) = normalized_task_git_path(path) else {
        return false;
    };
    task_declared_git_paths(declared_files)
        .iter()
        .any(|declared| path == *declared || path.starts_with(&format!("{declared}/")))
}

fn all_plans_terminal(executor: &ParallelExecutor, plans: &[Plan]) -> bool {
    plans
        .iter()
        .all(|p| executor.plan_state(&p.id).map_or(true, |s| s.is_terminal()))
}

fn task_refs_for_plan<'a>(
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    plan_id: &str,
) -> Vec<&'a TaskDef> {
    task_index
        .get(plan_id)
        .map(|tasks| tasks.values().collect())
        .unwrap_or_default()
}

fn ready_tasks_for_plan<'a>(
    task_dag: &TaskDag,
    executor: &ParallelExecutor,
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    state: &RunState,
    plan_id: &str,
) -> Vec<&'a TaskDef> {
    let task_refs = task_refs_for_plan(task_index, plan_id);
    let completed = state.plan_completed_tasks(plan_id);
    let completed_plans = completed_plan_ids(executor, task_index);
    task_dag.ready_tasks(plan_id, &task_refs, completed, &completed_plans)
}

fn dag_progress_for_plan(
    task_dag: &TaskDag,
    executor: &ParallelExecutor,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
    state: &RunState,
    plan_id: &str,
) -> DagProgressSummary {
    let task_refs = task_refs_for_plan(task_index, plan_id);
    let completed = state.plan_completed_tasks(plan_id);
    let failed = state.plan_failed_tasks(plan_id);
    let completed_plans = completed_plan_ids(executor, task_index);
    let failed_plans = failed_plan_ids(executor, task_index);
    task_dag.progress_summary(
        plan_id,
        &task_refs,
        completed,
        failed,
        &completed_plans,
        &failed_plans,
    )
}

fn completed_plan_ids(
    executor: &ParallelExecutor,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
) -> Vec<String> {
    task_index
        .keys()
        .filter(|plan_id| {
            executor
                .plan_state(plan_id)
                .is_some_and(|state| matches!(state.current_phase, PlanPhase::Complete))
        })
        .cloned()
        .collect()
}

fn failed_plan_ids(
    executor: &ParallelExecutor,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
) -> Vec<String> {
    task_index
        .keys()
        .filter(|plan_id| {
            executor.plan_state(plan_id).is_some_and(|state| {
                state.is_terminal() && !matches!(state.current_phase, PlanPhase::Complete)
            })
        })
        .cloned()
        .collect()
}

fn dag_plan_has_failures(task_dag: &TaskDag, state: &RunState, plan_id: &str) -> bool {
    !state.plan_failed_tasks(plan_id).is_empty()
        || task_dag
            .plan(plan_id)
            .is_some_and(|plan| !plan.failed.is_empty() || !plan.skipped.is_empty())
}

fn dag_quiescence_reason(plan_id: &str, summary: &DagProgressSummary) -> String {
    let blocked = summary.describe_blocked();
    if blocked.is_empty() {
        format!(
            "DAG made no future progress for plan {plan_id}: ready={}, active={}, blocked={}, terminal={}",
            summary.ready, summary.active, summary.blocked, summary.terminal
        )
    } else {
        format!(
            "DAG made no future progress for plan {plan_id}: {blocked} (ready={}, active={}, blocked={}, terminal={})",
            summary.ready, summary.active, summary.blocked, summary.terminal
        )
    }
}

fn gate_effect_key(plan_id: &str, task_id: &str, rung: u32, kind: GateCompletionKind) -> String {
    format!("{kind:?}:{plan_id}:{task_id}:{rung}")
}

/// Build enriched retry context for the agent after a gate failure.
///
/// Uses structured error classification from `roko_gate` to provide the agent
/// with a machine-readable analysis of what went wrong, alongside truncated
/// excerpts of the raw gate output and previous agent output.
fn build_gate_retry_context(
    gate_output: &str,
    prev_agent_output: &str,
    attempt_num: u32,
) -> String {
    let lower_gate_output = gate_output.to_ascii_lowercase();
    let classification_gate = if lower_gate_output.contains("test result: failed")
        || lower_gate_output.contains("assertion failed")
        || lower_gate_output.contains("panicked at")
    {
        "test"
    } else {
        "gate"
    };
    let classification = classify_gate_failure(classification_gate, gate_output);
    let analysis = render_failure_classification(&classification);

    let gate_excerpt = if gate_output.len() > 3000 {
        &gate_output[..3000]
    } else {
        gate_output
    };
    let agent_excerpt = if prev_agent_output.len() > 2000 {
        &prev_agent_output[..2000]
    } else {
        prev_agent_output
    };

    format!(
        "## IMPORTANT: Your previous attempt failed\n\n\
         Attempt {attempt_num} failed.\n\n\
         ### Error analysis\n{analysis}\n\n\
         ### Gate error output\n```\n{gate_excerpt}\n```\n\n\
         ### What you did last time\n```\n{agent_excerpt}\n```"
    )
}

fn maybe_apply_gate_failure_plan_revision(
    config: &RunConfig,
    paths: &PersistPaths,
    state: &mut RunState,
    task_index: &mut HashMap<String, HashMap<String, TaskDef>>,
    plan_id: &str,
    task_id: &str,
    failed_attempt: u32,
    verdicts: &[GateVerdictSummary],
    gate_output: &str,
    replan_context: &str,
) {
    if !gate_failure_replan_enabled(config) {
        return;
    }
    let replan_cap = gate_failure_replan_cap(config);
    if replan_cap == 0 {
        return;
    }
    if state.replan_count_for(plan_id) >= replan_cap {
        debug!(
            plan_id = %plan_id,
            task_id = %task_id,
            replan_cap,
            "gate-failure plan revision cap reached"
        );
        return;
    }

    let Some(task_def) = task_index
        .get(plan_id)
        .and_then(|tasks| tasks.get(task_id))
        .cloned()
    else {
        return;
    };
    let Some(revision) = build_gate_failure_plan_revision(
        config,
        plan_id,
        task_id,
        &task_def,
        failed_attempt,
        verdicts,
        gate_output,
        replan_context,
    ) else {
        return;
    };
    if state.has_seen_replan_failure(&revision.failure_key) {
        debug!(
            plan_id = %plan_id,
            task_id = %task_id,
            failure_key = %revision.failure_key,
            "duplicate gate-failure plan revision skipped"
        );
        return;
    }

    apply_task_revision_to_index(task_index, &revision);
    refresh_task_fingerprints_from_index(state, task_index);
    let request_id = revision.revision_request.request_id.clone();
    let required_next_action = revision.revision_request.disposition.to_string();
    let failure_key = revision.failure_key.clone();
    state.record_task_revision(failure_key.clone(), revision.clone());
    append_ledger_entry(
        &paths.run_ledger_jsonl,
        "plan_revision",
        &serde_json::json!({
            "request_id": request_id,
            "plan_id": plan_id,
            "task_id": task_id,
            "failure_key": failure_key,
            "required_next_action": required_next_action,
            "attempts": failed_attempt,
        }),
    );
    info!(
        plan_id = %plan_id,
        task_id = %task_id,
        request_id = %request_id,
        required_next_action = %required_next_action,
        "gate failure upgraded to durable task revision"
    );
}

fn build_gate_failure_plan_revision(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    task_def: &TaskDef,
    failed_attempt: u32,
    verdicts: &[GateVerdictSummary],
    gate_output: &str,
    replan_context: &str,
) -> Option<persist::TaskRevision> {
    let failing_verdicts = verdicts
        .iter()
        .filter(|verdict| !verdict.passed)
        .cloned()
        .collect::<Vec<_>>();
    if failing_verdicts.is_empty() {
        return None;
    }

    let gate_name = failing_verdicts
        .first()
        .map(|verdict| verdict.gate_name.as_str())
        .unwrap_or("gate");
    let classification = classify_gate_failure(gate_name, gate_output);
    let attempt_limit = gate_failure_replan_attempt_limit(config);
    let needs_revision = matches!(
        classification.recommended_action,
        roko_gate::GateFailureAction::NeedsReplan
    ) || failed_attempt >= attempt_limit;
    let blocked_or_human = matches!(
        classification.recommended_action,
        roko_gate::GateFailureAction::Blocked | roko_gate::GateFailureAction::NeedsHuman
    );
    if !needs_revision || blocked_or_human {
        return None;
    }

    let reason = PlanRevisionReason::GateFailureLimit {
        attempts: failed_attempt,
    };
    let failure_key = gate_failure_revision_failure_key(
        plan_id,
        task_id,
        &reason,
        &failing_verdicts,
        gate_output,
    );
    let evidence = gate_failure_revision_evidence(gate_name, &classification, &failing_verdicts);
    let revision_request =
        PlanRevisionRequest::gate_failure_limit(plan_id, task_id, failed_attempt, evidence);
    let revised_task = revised_task_for_gate_failure(
        config,
        task_def,
        &revision_request,
        &classification,
        replan_context,
    );

    Some(persist::TaskRevision {
        plan_id: plan_id.to_string(),
        task_id: task_id.to_string(),
        failure_key,
        revision_request,
        revised_task,
    })
}

fn gate_failure_revision_failure_key(
    plan_id: &str,
    task_id: &str,
    reason: &PlanRevisionReason,
    failing_verdicts: &[GateVerdictSummary],
    gate_output: &str,
) -> String {
    let gate_excerpt = gate_output.chars().take(4_000).collect::<String>();
    let payload = serde_json::json!({
        "plan_id": plan_id,
        "task_id": task_id,
        "reason": reason,
        "failing_verdicts": failing_verdicts,
        "gate_output": gate_excerpt,
    });
    ContentHash::of(payload.to_string().as_bytes()).to_hex()
}

fn gate_failure_revision_evidence(
    gate_name: &str,
    classification: &roko_gate::GateFailureClassification,
    failing_verdicts: &[GateVerdictSummary],
) -> Vec<PlanRevisionEvidence> {
    let failure_pattern_ids = classification
        .classes
        .iter()
        .map(|class| format!("failure_class:{}", failure_class_label(class)))
        .collect::<Vec<_>>();
    let detail = failing_verdicts
        .iter()
        .map(|verdict| {
            let digest = verdict.error_digest.as_deref().unwrap_or("");
            format!(
                "{}: {}; kind={:?}; digest={}",
                verdict.gate_name, verdict.summary, verdict.failure_kind, digest
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    vec![
        PlanRevisionEvidence::gate(gate_name.to_string())
            .with_classification(Some(failure_class_label(&classification.primary)))
            .with_failure_pattern_ids(failure_pattern_ids)
            .with_blocking_findings(classification.blocking_findings.clone())
            .with_detail((!detail.trim().is_empty()).then_some(detail)),
    ]
}

fn revised_task_for_gate_failure(
    config: &RunConfig,
    task_def: &TaskDef,
    revision_request: &PlanRevisionRequest,
    classification: &roko_gate::GateFailureClassification,
    replan_context: &str,
) -> TaskDef {
    let mut revised_task = task_def.clone();
    let strategy = if classification.replan_candidate {
        ReplanStrategy::Decompose
    } else {
        ReplanStrategy::RetryWithEscalation
    };
    revised_task.status = "ready".to_string();
    revised_task.replan_strategy = Some(strategy);
    if revised_task.model_hint.is_none() || matches!(strategy, ReplanStrategy::RetryWithEscalation)
    {
        revised_task.model_hint = Some(architectural_model_hint(config));
    }
    if matches!(strategy, ReplanStrategy::Decompose) && revised_task.split_into.is_none() {
        revised_task.split_into = Some(vec![
            format!("{}-diagnose", task_def.id),
            format!("{}-fix", task_def.id),
            format!("{}-verify", task_def.id),
        ]);
    }

    let existing_description = revised_task
        .description
        .clone()
        .unwrap_or_else(|| revised_task.title.clone());
    let context_excerpt = replan_context.chars().take(2_000).collect::<String>();
    revised_task.description = Some(format!(
        "{existing_description}\n\n\
         ## Durable Gate-Failure Revision\n\
         Request id: {}\n\
         Required next action: {}\n\
         Reason: gate_failure_limit attempts={}\n\
         Strategy: {}\n\
         Failure classes: {}\n\n\
         Revision context:\n{}",
        revision_request.request_id,
        revision_request.disposition,
        revision_request.attempts,
        strategy,
        classification
            .classes
            .iter()
            .map(failure_class_label)
            .collect::<Vec<_>>()
            .join(", "),
        context_excerpt
    ));
    let acceptance = format!(
        "Address plan revision request {} before rerunning verification.",
        revision_request.request_id
    );
    if !revised_task
        .acceptance
        .iter()
        .any(|item| item == &acceptance)
    {
        revised_task.acceptance.push(acceptance);
    }
    if !revised_task.title.contains("[gate revision]") {
        revised_task.title = format!("{} [gate revision]", revised_task.title);
    }
    revised_task
}

fn gate_failure_replan_enabled(config: &RunConfig) -> bool {
    config
        .roko_config
        .as_deref()
        .map(|cfg| cfg.learning.replan_on_gate_failure)
        .unwrap_or(true)
}

fn gate_failure_replan_cap(config: &RunConfig) -> u32 {
    config
        .roko_config
        .as_deref()
        .map(|cfg| cfg.learning.replan_max_per_plan)
        .unwrap_or(2)
}

fn gate_failure_replan_attempt_limit(config: &RunConfig) -> u32 {
    config
        .roko_config
        .as_deref()
        .map(|cfg| cfg.learning.replan_gate_attempts)
        .unwrap_or(3)
        .max(1)
}

fn architectural_model_hint(config: &RunConfig) -> String {
    config
        .roko_config
        .as_deref()
        .and_then(|cfg| cfg.agent.tier_models.get("architectural"))
        .cloned()
        .unwrap_or_else(|| roko_core::defaults::MODEL_DEEP.to_string())
}

fn failure_class_label(class: &roko_gate::FailureClass) -> String {
    serde_json::to_value(class)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| format!("{class:?}").to_ascii_lowercase())
}

enum TaskTerminalization {
    Passed,
    PersistenceFailed { reason: String },
    AlreadyRecorded,
}

#[allow(clippy::too_many_arguments)]
fn terminalize_passed_task(
    paths: &PersistPaths,
    state: &mut RunState,
    task_dag: &mut TaskDag,
    task_index: &HashMap<String, HashMap<String, TaskDef>>,
    run_ledger: &mut Option<RunLedger>,
    tui: &TuiBridge,
    sink: &dyn RunOutputSink,
    config: &RunConfig,
    completion: &GateCompletion,
    attempt: &TaskAttemptRef,
    task_workdir: Option<&Path>,
    declared_files: &[String],
) -> TaskTerminalization {
    if state.task_attempt_is_terminal(attempt) {
        return TaskTerminalization::AlreadyRecorded;
    }

    let output_diffs = task_workdir
        .map(git_diff_entries_since_task_start)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| task_path_allowed_by_declared_files(&entry.path, declared_files))
        .collect::<Vec<_>>();
    let output_files = output_diffs
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();

    let commit_outcome = match task_workdir {
        Some(workdir) => commit_task_changes(
            workdir,
            &completion.plan_id,
            &completion.task_id,
            declared_files,
        ),
        None => CommitOutcome::Rejected {
            reason: "isolated worktree missing while terminalizing passed task".to_string(),
        },
    };

    let now_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let durability_error = match &commit_outcome {
        CommitOutcome::Created { .. } | CommitOutcome::NoChanges => None,
        CommitOutcome::Rejected { reason } => Some(reason.clone()),
        CommitOutcome::Failed { error } => Some(error.clone()),
    };

    if let Some(reason) = durability_error {
        let reason = format!("task passed gates but durable completion failed: {reason}");
        state.task_failed();
        state.record_task_failure(&completion.plan_id, &completion.task_id, &reason);
        state.mark_task_failed(&completion.plan_id, &completion.task_id);
        let task_refs = task_refs_for_plan(task_index, &completion.plan_id);
        let skipped = task_dag.mark_failed_blocking_downstream(
            &completion.plan_id,
            &completion.task_id,
            &task_refs,
        );
        if !skipped.is_empty() {
            debug!(
                plan_id = %completion.plan_id,
                task_id = %completion.task_id,
                skipped = ?skipped,
                "durability failure blocked downstream tasks"
            );
        }
        sink.task_failed(&completion.plan_id, &completion.task_id, &reason);
        tui.task_completed(&completion.plan_id, &completion.task_id, "failed");
        if let Some(ledger) = run_ledger.as_mut() {
            let inserted = ledger.record_task_terminal(TaskTerminalOutcome {
                plan_id: completion.plan_id.clone(),
                task_id: completion.task_id.clone(),
                attempt: attempt.attempt,
                passed: false,
                reason: Some(reason.clone()),
                output_files,
                commit_outcome,
                duration_ms: completion.duration_ms,
                timestamp_ms: now_ms,
            });
            if inserted {
                append_ledger_entry(
                    &paths.run_ledger_jsonl,
                    "task_failed",
                    &serde_json::json!({
                        "run_id": state.run_id(),
                        "plan_id": completion.plan_id,
                        "task_id": completion.task_id,
                        "attempt": attempt.attempt,
                        "passed": false,
                        "reason": reason,
                        "duration_ms": completion.duration_ms,
                        "timestamp_ms": now_ms,
                        "commit_outcome": ledger.commit.as_ref(),
                    }),
                );
            }
        }
        let run_id = state.run_id().to_string();
        let agent_model = state.agent_model.clone();
        let agent_provider = state.agent_provider.clone();
        emit_runner_event(
            paths,
            state,
            tui,
            config,
            RunnerEvent::task_attempt_completed(
                &run_id,
                attempt.clone(),
                TaskAttemptOutcome::Failed,
                Some(RunnerFailureKind::Permanent),
                completion.duration_ms,
                agent_model,
                agent_provider,
            ),
        );
        record_daimon_task_outcome(
            config,
            state.current_daimon_strategy,
            &completion.plan_id,
            &completion.task_id,
            false,
            &reason,
        );
        return TaskTerminalization::PersistenceFailed { reason };
    }

    state.record_task_outputs(
        &completion.plan_id,
        &completion.task_id,
        output_files.clone(),
    );
    if state.mark_task_completed(&completion.plan_id, &completion.task_id) {
        state.task_completed();
    }

    if let Some(ledger) = run_ledger.as_mut() {
        ledger.record_phase_transition(
            roko_runtime::Phase::Implementing,
            roko_runtime::Phase::Complete,
            now_ms,
        );
        let inserted = ledger.record_task_terminal(TaskTerminalOutcome {
            plan_id: completion.plan_id.clone(),
            task_id: completion.task_id.clone(),
            attempt: attempt.attempt,
            passed: true,
            reason: None,
            output_files,
            commit_outcome,
            duration_ms: completion.duration_ms,
            timestamp_ms: now_ms,
        });
        if inserted {
            append_ledger_entry(
                &paths.run_ledger_jsonl,
                "task_completed",
                &serde_json::json!({
                    "run_id": state.run_id(),
                    "plan_id": completion.plan_id,
                    "task_id": completion.task_id,
                    "attempt": attempt.attempt,
                    "passed": true,
                    "duration_ms": completion.duration_ms,
                    "timestamp_ms": now_ms,
                    "commit_outcome": ledger.commit.as_ref(),
                }),
            );
        }
    }

    let run_id = state.run_id().to_string();
    let agent_model = state.agent_model.clone();
    let agent_provider = state.agent_provider.clone();
    emit_runner_event(
        paths,
        state,
        tui,
        config,
        RunnerEvent::task_attempt_completed(
            &run_id,
            attempt.clone(),
            TaskAttemptOutcome::Passed,
            None,
            completion.duration_ms,
            agent_model,
            agent_provider,
        ),
    );
    record_daimon_task_outcome(
        config,
        state.current_daimon_strategy,
        &completion.plan_id,
        &completion.task_id,
        true,
        &format!("gate-rung-{}", completion.rung),
    );
    tui.task_completed(&completion.plan_id, &completion.task_id, "passed");

    let total_task_ms = state.task_elapsed_ms();
    let dispatch_ms = state.last_dispatch_ms;
    let gate_ms = completion.duration_ms;
    let agent_ms = if state.task_agent_calls == 0 {
        0
    } else {
        total_task_ms.saturating_sub(dispatch_ms + gate_ms)
    };
    sink.diff_block(&completion.plan_id, &completion.task_id, &output_diffs);
    sink.task_completed(
        &completion.plan_id,
        &completion.task_id,
        state.tasks_completed,
        state.tasks_total,
        total_task_ms,
    );
    info!(
        task = %completion.task_id,
        dispatch_ms,
        agent_ms,
        gate_ms,
        "task timing"
    );

    task_dag.mark_complete(&completion.plan_id, &completion.task_id);
    TaskTerminalization::Passed
}

/// Commit working tree changes for a completed task.
///
/// Only acts if there are uncommitted changes in the task's declared files.
/// Silently succeeds if git is not available or the workdir is not a git repo.
/// Uses `--no-verify` to avoid triggering hooks in generated workspaces.
fn commit_task_changes(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
    declared_files: &[String],
) -> CommitOutcome {
    use std::process::Command;

    let paths = task_declared_git_paths(declared_files);
    if paths.is_empty() {
        debug!(
            %plan_id,
            %task_id,
            "task has no declared files -- skipping auto-commit"
        );
        return CommitOutcome::NoChanges;
    }

    // Check if there are changes to commit in this task's declared write set.
    let mut status_cmd = Command::new("git");
    status_cmd
        .args(["status", "--porcelain", "--"])
        .args(&paths)
        .current_dir(workdir);
    let status = match status_cmd.output() {
        Ok(status) => status,
        Err(err) => {
            return CommitOutcome::Failed {
                error: format!("git status failed: {err}"),
            };
        }
    };
    let has_changes = !status.stdout.is_empty();
    if !has_changes {
        debug!(%plan_id, %task_id, "no uncommitted changes to commit");
        return CommitOutcome::NoChanges;
    }

    let msg = format!("[roko] {plan_id}: {task_id} completed");
    let mut add_cmd = Command::new("git");
    add_cmd
        .args(["add", "--"])
        .args(&paths)
        .current_dir(workdir);
    let add = match add_cmd.status() {
        Ok(status) => status,
        Err(err) => {
            return CommitOutcome::Failed {
                error: format!("git add failed: {err}"),
            };
        }
    };
    if !add.success() {
        return CommitOutcome::Failed {
            error: format!("git add exited with status {add}"),
        };
    }
    let mut commit_cmd = Command::new("git");
    commit_cmd
        .args(["commit", "-m", &msg, "--no-verify", "--only", "--"])
        .args(&paths)
        .current_dir(workdir);
    let commit = commit_cmd.status();
    match commit {
        Ok(s) if s.success() => {
            let hash = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(workdir)
                .output()
                .ok()
                .and_then(|output| {
                    output
                        .status
                        .success()
                        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
                })
                .filter(|hash| !hash.is_empty())
                .unwrap_or_else(|| "unknown".to_string());
            info!(%plan_id, %task_id, %hash, "committed task changes to git");
            CommitOutcome::Created { hash }
        }
        Ok(status) => CommitOutcome::Failed {
            error: format!("git commit exited with status {status}"),
        },
        Err(err) => CommitOutcome::Failed {
            error: format!("git commit failed: {err}"),
        },
    }
}

/// Extract lessons from past post-gate reflections for a specific gate.
///
/// Returns up to 3 de-duplicated lessons from failed reflections with confidence
/// above 0.3. If the store file is missing or malformed, returns an empty vec.
fn lessons_from_post_gate_reflections(
    learn_dir: &Path,
    gate_name: &str,
    _task_id: &str,
) -> Vec<String> {
    let path = learn_dir.join("post-gate-reflections.json");
    let store = PostGateReflectionStore::load(&path);

    let mut lessons: Vec<String> = store
        .records
        .iter()
        .filter(|r| r.trigger_gate == gate_name)
        .filter(|r| matches!(r.outcome, ReflectionGateOutcome::Failed))
        .filter(|r| r.confidence > 0.3)
        .filter(|r| !r.proposed_lesson.is_empty())
        .map(|r| r.proposed_lesson.clone())
        .collect();

    lessons.dedup();
    lessons.truncate(3);
    lessons
}

fn build_report(executor: &ParallelExecutor, plans: &[Plan], state: &RunState) -> RunReport {
    let plan_reports: Vec<PlanReport> = plans
        .iter()
        .map(|p| {
            let orc_state = executor.plan_state(&p.id);
            let completed = orc_state
                .map(|s| matches!(s.current_phase, PlanPhase::Complete))
                .unwrap_or(false);
            PlanReport {
                plan_id: p.id.clone(),
                completed,
                tasks_total: p.tasks.tasks.len(),
                tasks_completed: if completed { p.tasks.tasks.len() } else { 0 },
                tasks_failed: if !completed && orc_state.map_or(false, |s| s.is_terminal()) {
                    1
                } else {
                    0
                },
                gate_results: orc_state
                    .map(|state| state.gate_results.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    RunReport {
        plans: plan_reports,
        total_tasks: state.tasks_total,
        tasks_completed: state.tasks_completed,
        tasks_failed: state.tasks_failed,
        total_cost_usd: state.total_cost_usd,
        total_tokens_in: state.total_tokens_in,
        total_tokens_out: state.total_tokens_out,
        total_agent_calls: state.total_agent_calls,
        duration: state.elapsed(),
        failure_reasons: state.failure_reasons.clone(),
        task_costs: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_parser::TasksFile;

    #[test]
    fn successful_plan_verify_finalizes_runner_plan() {
        let mut executor = ParallelExecutor::new(ExecutorConfig::default());
        executor.add_plan(OrcPlanState::new("plan-verify"));

        executor
            .apply_event("plan-verify", &ExecutorEvent::Start)
            .unwrap();
        executor
            .apply_event("plan-verify", &ExecutorEvent::EnrichmentDone)
            .unwrap();
        executor
            .apply_event("plan-verify", &ExecutorEvent::ImplementationDone)
            .unwrap();
        executor
            .apply_event("plan-verify", &ExecutorEvent::GatePassed)
            .unwrap();

        let phase = complete_plan_after_successful_verify("plan-verify", &mut executor).unwrap();

        assert_eq!(phase, PlanPhase::Complete);
        assert!(executor.plan_state("plan-verify").unwrap().is_terminal());
        assert!(
            executor.tick().is_empty(),
            "completed plan must not request review/doc agents or rerun tasks"
        );
    }

    #[test]
    fn successful_preflight_advances_to_gate_exactly_once() {
        let mut executor = ParallelExecutor::new(ExecutorConfig::default());
        executor.add_plan(OrcPlanState::new("preflight"));
        executor
            .apply_event("preflight", &ExecutorEvent::Start)
            .unwrap();

        assert_eq!(
            advance_preflight_success_to_gate(&mut executor, "preflight").unwrap(),
            PlanPhase::Gating
        );
        assert_eq!(
            advance_preflight_success_to_gate(&mut executor, "preflight").unwrap(),
            PlanPhase::Gating
        );
    }

    #[test]
    fn fresh_run_seeds_done_tasks_from_plan_status() {
        let tasks = TasksFile::parse_str(
            r#"
[meta]
plan = "seed-test"
total = 3
status = "ready"

[[task]]
id = "T1"
title = "done dependency"
status = "done"
tier = "focused"
role = "implementer"
depends_on = []

[[task]]
id = "T2"
title = "ready follow-up"
status = "ready"
tier = "focused"
role = "implementer"
depends_on = ["T1"]

[[task]]
id = "T3"
title = "also done"
status = "done"
tier = "focused"
role = "implementer"
depends_on = []
"#,
        )
        .unwrap();
        let plan = Plan {
            id: "seed-test".to_string(),
            dir: std::path::PathBuf::from("plans/seed-test"),
            tasks,
            prd_excerpt: String::new(),
        };
        let mut state = RunState::new(3);

        seed_completed_tasks_from_plan_status(&mut state, &[plan]);

        assert_eq!(state.tasks_completed, 2);
        assert_eq!(state.plan_completed_tasks("seed-test"), ["T1", "T3"]);
    }

    #[test]
    fn completed_plan_initializes_at_gating_phase() {
        let tasks = TasksFile::parse_str(
            r#"
[meta]
plan = "done-plan"
total = 2
done = 2
status = "done"

[[task]]
id = "T1"
title = "one"
status = "done"
tier = "focused"
role = "implementer"
files = ["Cargo.toml"]
verify = [{ phase = "structural", command = "true" }]
depends_on = []

[[task]]
id = "T2"
title = "two"
status = "complete"
tier = "focused"
role = "implementer"
files = ["Cargo.toml"]
verify = [{ phase = "structural", command = "true" }]
depends_on = []
"#,
        )
        .unwrap();
        let plan = Plan {
            id: "done-plan".to_string(),
            dir: std::path::PathBuf::from("plans/done-plan"),
            tasks,
            prd_excerpt: String::new(),
        };
        let mut state = RunState::new(2);
        let mut executor = ParallelExecutor::new(ExecutorConfig::default());
        executor.add_plan(OrcPlanState::new("done-plan"));

        seed_completed_tasks_from_plan_status(&mut state, std::slice::from_ref(&plan));
        initialize_terminal_plan_phases(&mut executor, &state, &[plan]);

        assert!(matches!(
            executor
                .plan_state("done-plan")
                .map(|state| &state.current_phase),
            Some(PlanPhase::Gating)
        ));
    }

    #[test]
    fn no_ready_spawn_only_completes_implementing_phase() {
        assert_eq!(
            no_ready_spawn_event(PhaseKind::Implementing, "next"),
            Some(ExecutorEvent::ImplementationDone)
        );

        assert!(matches!(
            no_ready_spawn_event(PhaseKind::RegeneratingVerify, "regen-verify"),
            Some(ExecutorEvent::Fatal(reason))
                if reason.contains("no runnable task was available")
        ));
        assert!(matches!(
            no_ready_spawn_event(PhaseKind::AutoFixing, "fix"),
            Some(ExecutorEvent::Fatal(reason))
                if reason.contains("no runnable task was available")
        ));
        assert_eq!(no_ready_spawn_event(PhaseKind::Complete, "next"), None);
    }

    #[test]
    fn load_executor_resumes_from_unified_state_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let paths = persist::PersistPaths::from_workdir(dir.path()).unwrap();
        let mut snapshot = ExecutorSnapshot::new(123);
        snapshot
            .plan_states
            .insert("self-dev-ux".to_string(), OrcPlanState::new("self-dev-ux"));
        snapshot.queue_order.push("self-dev-ux".to_string());
        let orchestrator_json = OrchestratorSnapshot::new(snapshot.clone(), 123)
            .to_json()
            .unwrap();
        let unified = roko_runtime::StateSnapshot::new(
            123,
            snapshot.to_json().unwrap(),
            orchestrator_json,
            "{}".to_string(),
            "{}".to_string(),
        );
        persist::save_state_snapshot(&paths, &unified).unwrap();

        let resume = load_executor(
            &paths,
            &ExecutorConfig::default(),
            &["self-dev-ux".to_string()],
        );

        assert_eq!(resume.marker.outcome, ResumeOutcome::Resumed);
        assert_eq!(
            resume.marker.snapshot_path,
            paths.state_snapshot_json.display().to_string()
        );
        assert!(resume.executor.plan_state("self-dev-ux").is_some());
    }

    #[test]
    fn task_path_filter_only_allows_declared_files() {
        let declared = vec![
            "crates/roko-core/src/lib.rs".to_string(),
            "crates/roko-cli/src/runner".to_string(),
        ];

        assert!(task_path_allowed_by_declared_files(
            "crates/roko-core/src/lib.rs",
            &declared
        ));
        assert!(task_path_allowed_by_declared_files(
            "crates/roko-cli/src/runner/event_loop.rs",
            &declared
        ));
        assert!(!task_path_allowed_by_declared_files(
            "crates/roko-core/src/lib.rs.bak",
            &declared
        ));
        assert!(!task_path_allowed_by_declared_files(
            "../outside.rs",
            &declared
        ));
    }

    #[test]
    fn turn_budget_check_is_disabled_when_limit_is_zero() {
        assert!(!turn_exceeds_budget(Some(100.0), 0.0));
        assert!(!turn_exceeds_budget(None, 3.0));
        assert!(!turn_exceeds_budget(Some(3.0), 3.0));
        assert!(turn_exceeds_budget(Some(3.01), 3.0));
    }

    #[test]
    fn verification_only_task_does_not_emit_model_feedback() {
        let event = RunnerEvent::task_attempt_completed(
            "run-1",
            TaskAttemptRef::new("plan".to_string(), "task".to_string(), 1),
            TaskAttemptOutcome::Passed,
            None,
            123,
            "",
            "",
        );

        let feedback = runner_event_to_feedback(&event, &None, &TaskUsageSnapshot::default());

        assert!(
            feedback.is_none(),
            "verification-only tasks have no model to reward in the cascade router"
        );
    }

    #[test]
    fn commit_task_changes_only_commits_declared_files() {
        fn git(workdir: &std::path::Path, args: &[&str]) {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(workdir)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init"]);
        git(
            dir.path(),
            &["config", "user.email", "roko@example.invalid"],
        );
        git(dir.path(), &["config", "user.name", "Roko Test"]);

        std::fs::write(dir.path().join("declared.txt"), "before\n").unwrap();
        std::fs::write(dir.path().join("unrelated.txt"), "before\n").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-m", "initial"]);

        std::fs::write(dir.path().join("declared.txt"), "after\n").unwrap();
        std::fs::write(dir.path().join("unrelated.txt"), "after\n").unwrap();

        commit_task_changes(dir.path(), "plan", "task", &["declared.txt".to_string()]);

        let show = std::process::Command::new("git")
            .args(["show", "--name-only", "--format=", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(show.status.success());
        let committed_files = String::from_utf8_lossy(&show.stdout);
        assert!(committed_files.contains("declared.txt"));
        assert!(!committed_files.contains("unrelated.txt"));

        let status = std::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(status.status.success());
        let status = String::from_utf8_lossy(&status.stdout);
        assert!(status.contains("unrelated.txt"));
        assert!(!status.contains("declared.txt"));
    }

    #[test]
    fn build_gate_retry_context_compile_error_produces_analysis() {
        let gate_output = "error[E0433]: failed to resolve: use of undeclared crate or module `foo`\n\
                           --> src/lib.rs:3:5\n  |\n3 | use foo::bar;\n  |     ^^^ use of undeclared crate or module `foo`";
        let agent_output = "I added `use foo::bar;` to import the module.";
        let result = build_gate_retry_context(gate_output, agent_output, 2);

        assert!(result.contains("## IMPORTANT: Your previous attempt failed"));
        assert!(result.contains("Attempt 2 failed."));
        assert!(result.contains("### Error analysis"));
        // The analysis should contain structured JSON with the classification
        assert!(result.contains("\"primary\""));
        assert!(result.contains("### Gate error output"));
        assert!(result.contains("error[E0433]"));
        assert!(result.contains("### What you did last time"));
        assert!(result.contains("use foo::bar"));
    }

    #[test]
    fn build_gate_retry_context_truncates_long_output() {
        let gate_output = "x".repeat(5000);
        let agent_output = "y".repeat(4000);
        let result = build_gate_retry_context(&gate_output, &agent_output, 1);

        // Gate output truncated to 3000 chars
        let gate_section_start = result.find("### Gate error output").unwrap();
        let agent_section_start = result.find("### What you did last time").unwrap();
        let gate_section = &result[gate_section_start..agent_section_start];
        // Count the 'x' chars in the gate section — should be 3000, not 5000
        let x_count = gate_section.chars().filter(|c| *c == 'x').count();
        assert_eq!(x_count, 3000);

        // Agent output truncated to 2000 chars
        let agent_section = &result[agent_section_start..];
        let agent_block_start = agent_section.find("```\n").unwrap() + "```\n".len();
        let agent_block = &agent_section[agent_block_start..];
        let agent_block_end = agent_block.find("\n```").unwrap();
        let y_count = agent_block[..agent_block_end]
            .chars()
            .filter(|c| *c == 'y')
            .count();
        assert_eq!(y_count, 2000);
    }

    #[test]
    fn build_gate_retry_context_test_output_preserves_test_names() {
        let gate_output = "test result: FAILED. 9 passed; 1 failed; 0 ignored\n\
                           failures:\n    tests::my_important_test\n\
                           thread 'tests::my_important_test' panicked at assertion failed: expected 42, got 0";
        let agent_output = "I implemented the function but forgot to handle the edge case.";
        let result = build_gate_retry_context(gate_output, agent_output, 3);

        assert!(result.contains("Attempt 3 failed."));
        assert!(result.contains("tests::my_important_test"));
        assert!(result.contains("assertion failed"));
        // Classification should identify this as a test failure
        assert!(result.contains("test_expectation_failure"));
    }
}

#[cfg(test)]
mod tests_post_gate_reflection_lessons {
    use super::*;
    use roko_learn::post_gate_reflection::{
        PostGateReflectionRecord, PostGateReflectionStore, ReflectionAdmissionStatus,
        ReflectionGateOutcome,
    };
    use tempfile::TempDir;

    fn make_record(
        gate: &str,
        outcome: ReflectionGateOutcome,
        confidence: f64,
        lesson: &str,
    ) -> PostGateReflectionRecord {
        PostGateReflectionRecord {
            reflection_id: format!("test-{}", lesson.len()),
            plan_id: None,
            task_id: Some("task-1".to_string()),
            episode_id: None,
            trigger_gate: gate.to_string(),
            outcome,
            failure_pattern_ids: vec![],
            pass_evidence: vec![],
            proposed_lesson: lesson.to_string(),
            confidence,
            evidence_count: 1,
            admission_status: ReflectionAdmissionStatus::Candidate,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn missing_store_path_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path().join("nonexistent");
        let result = lessons_from_post_gate_reflections(&learn_dir, "compile", "task-1");
        assert!(result.is_empty());
    }

    #[test]
    fn matching_failed_records_are_included() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path();
        let store = PostGateReflectionStore {
            records: vec![
                make_record(
                    "compile",
                    ReflectionGateOutcome::Failed,
                    0.5,
                    "Fix type mismatch in handler",
                ),
                make_record(
                    "compile",
                    ReflectionGateOutcome::Failed,
                    0.6,
                    "Check import paths",
                ),
            ],
            candidates: vec![],
        };
        store
            .save(&learn_dir.join("post-gate-reflections.json"))
            .unwrap();

        let result = lessons_from_post_gate_reflections(learn_dir, "compile", "task-1");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Fix type mismatch in handler");
        assert_eq!(result[1], "Check import paths");
    }

    #[test]
    fn only_top_3_lessons_returned() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path();
        let store = PostGateReflectionStore {
            records: vec![
                make_record("test", ReflectionGateOutcome::Failed, 0.5, "Lesson A"),
                make_record("test", ReflectionGateOutcome::Failed, 0.6, "Lesson B"),
                make_record("test", ReflectionGateOutcome::Failed, 0.7, "Lesson C"),
                make_record("test", ReflectionGateOutcome::Failed, 0.8, "Lesson D"),
                make_record("test", ReflectionGateOutcome::Failed, 0.9, "Lesson E"),
            ],
            candidates: vec![],
        };
        store
            .save(&learn_dir.join("post-gate-reflections.json"))
            .unwrap();

        let result = lessons_from_post_gate_reflections(learn_dir, "test", "task-1");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn passed_outcomes_excluded() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path();
        let store = PostGateReflectionStore {
            records: vec![
                make_record(
                    "compile",
                    ReflectionGateOutcome::Passed,
                    0.8,
                    "Good approach",
                ),
                make_record("compile", ReflectionGateOutcome::Failed, 0.5, "Fix the bug"),
            ],
            candidates: vec![],
        };
        store
            .save(&learn_dir.join("post-gate-reflections.json"))
            .unwrap();

        let result = lessons_from_post_gate_reflections(learn_dir, "compile", "task-1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Fix the bug");
    }

    #[test]
    fn low_confidence_records_excluded() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path();
        let store = PostGateReflectionStore {
            records: vec![
                make_record(
                    "compile",
                    ReflectionGateOutcome::Failed,
                    0.2,
                    "Low confidence",
                ),
                make_record(
                    "compile",
                    ReflectionGateOutcome::Failed,
                    0.5,
                    "High confidence",
                ),
            ],
            candidates: vec![],
        };
        store
            .save(&learn_dir.join("post-gate-reflections.json"))
            .unwrap();

        let result = lessons_from_post_gate_reflections(learn_dir, "compile", "task-1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "High confidence");
    }

    #[test]
    fn different_gate_records_excluded() {
        let dir = TempDir::new().unwrap();
        let learn_dir = dir.path();
        let store = PostGateReflectionStore {
            records: vec![
                make_record("test", ReflectionGateOutcome::Failed, 0.5, "Test lesson"),
                make_record(
                    "compile",
                    ReflectionGateOutcome::Failed,
                    0.5,
                    "Compile lesson",
                ),
            ],
            candidates: vec![],
        };
        store
            .save(&learn_dir.join("post-gate-reflections.json"))
            .unwrap();

        let result = lessons_from_post_gate_reflections(learn_dir, "compile", "task-1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Compile lesson");
    }

    #[test]
    fn matching_gate_attempt_is_consumed_exactly_once() {
        let attempt = TaskAttemptRef::new("plan", "task", 2);
        let mut attempts = HashMap::from([("effect".to_string(), attempt.clone())]);

        assert_eq!(
            take_matching_gate_attempt(&mut attempts, "effect", Some(&attempt)),
            Some(attempt.clone())
        );
        assert_eq!(
            take_matching_gate_attempt(&mut attempts, "effect", Some(&attempt)),
            None
        );
    }

    #[test]
    fn stale_gate_attempt_cannot_consume_current_attempt() {
        let stale = TaskAttemptRef::new("plan", "task", 1);
        let current = TaskAttemptRef::new("plan", "task", 2);
        let mut attempts = HashMap::from([("effect".to_string(), current.clone())]);

        assert_eq!(
            take_matching_gate_attempt(&mut attempts, "effect", Some(&stale)),
            None
        );
        assert_eq!(attempts.get("effect"), Some(&current));
        assert_eq!(
            take_matching_gate_attempt(&mut attempts, "effect", None),
            None
        );
        assert_eq!(attempts.get("effect"), Some(&current));
    }

    fn turn_completed(is_error: bool) -> AgentEvent {
        AgentEvent::TurnCompleted {
            session_id: None,
            total_cost_usd: None,
            num_turns: Some(1),
            is_error,
        }
    }

    #[test]
    fn agent_terminal_classification_requires_confirmed_clean_success() {
        let clean = AgentSettlement {
            exit_code: Some(0),
            errors: Vec::new(),
            unconfirmed: None,
        };
        assert_eq!(agent_terminal_failure(&turn_completed(false), &clean), None);
        assert_eq!(
            agent_terminal_failure(&turn_completed(true), &clean),
            Some("agent reported an error result".to_string())
        );

        let unknown = AgentSettlement {
            exit_code: None,
            errors: Vec::new(),
            unconfirmed: None,
        };
        assert!(
            agent_terminal_failure(&turn_completed(false), &unknown)
                .unwrap()
                .contains("not confirmed")
        );

        let nonzero = AgentSettlement {
            exit_code: Some(7),
            errors: Vec::new(),
            unconfirmed: None,
        };
        assert!(
            agent_terminal_failure(&turn_completed(false), &nonzero)
                .unwrap()
                .contains("status 7")
        );

        let reader_failure = AgentSettlement {
            exit_code: Some(0),
            errors: vec!["stdout reader failed".to_string()],
            unconfirmed: None,
        };
        assert_eq!(
            agent_terminal_failure(&turn_completed(false), &reader_failure),
            Some("stdout reader failed".to_string())
        );
    }

    #[test]
    fn routed_agent_event_preserves_exact_attempt_and_effect() {
        let attempt = TaskAttemptRef::new("plan", "task", 2);
        let effect = EffectRef(17);
        let routed = RoutedAgentEvent::for_attempt(
            attempt.clone(),
            effect,
            "agent-2".to_string(),
            AgentEvent::Exited { exit_code: Some(0) },
        );

        let RoutedAgentEvent::Agent {
            attempt: routed_attempt,
            effect: routed_effect,
            agent_id,
            ..
        } = routed
        else {
            panic!("expected routed agent event");
        };
        assert_eq!(routed_attempt, attempt);
        assert_eq!(routed_effect, effect);
        assert_eq!(agent_id, "agent-2");
    }

    #[test]
    fn turn_completed_claim_makes_following_exit_ineligible() {
        let attempt = TaskAttemptRef::new("plan", "task", 1);
        let effect = EffectRef(9);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                attempt.clone(),
                AttemptOwner::new(AttemptPhase::Agent, effect),
                (),
            )
            .unwrap();

        let claim = ownership
            .claim_phase(&attempt, AttemptPhase::Agent, effect)
            .unwrap();
        ownership
            .transition_claim(claim, AttemptPhase::AwaitingGate, effect)
            .unwrap();

        assert!(!ownership.event_is_eligible(&attempt, AttemptPhase::Agent, effect));
        assert!(matches!(
            ownership.claim_phase(&attempt, AttemptPhase::Agent, effect),
            Err(_)
        ));
    }

    #[tokio::test]
    async fn bridge_join_failure_is_not_gateable_and_releases_permit() {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let bridge = tokio::spawn(async { panic!("bridge failed") });
        let forwarder = tokio::spawn(async {});
        let settlement = settle_agent_resource(AgentRuntimeResource::Bridge {
            bridge,
            forwarder,
            permit,
        })
        .await;

        assert_eq!(semaphore.available_permits(), 1);
        assert!(settlement.unconfirmed.is_none());
        assert!(
            settlement
                .errors
                .iter()
                .any(|error| error.contains("agent bridge failed"))
        );
        assert!(agent_terminal_failure(&turn_completed(false), &settlement).is_some());
    }

    #[test]
    fn repeated_gate_dispatches_have_distinct_exact_effects() {
        let attempt = TaskAttemptRef::new("plan", "task", 1);
        let first = new_gate_effect(attempt.clone(), GateCompletionKind::Gate, 2);
        let second = new_gate_effect(attempt.clone(), GateCompletionKind::Gate, 2);
        assert_eq!(first.attempt, attempt);
        assert_eq!(first.kind, second.kind);
        assert_eq!(first.rung, second.rung);
        assert_ne!(first.generation, second.generation);
    }

    #[test]
    fn owned_gate_claim_finishes_exactly_once_or_awaits_next_rung() {
        let attempt = TaskAttemptRef::new("plan", "task", 1);
        let effect = EffectRef(77);
        let mut ownership = AttemptOwnership::default();
        ownership
            .insert(
                attempt.clone(),
                AttemptOwner::new(AttemptPhase::Gate, effect),
                AgentRuntimeResource::AwaitingGate,
            )
            .unwrap();
        let claim = ownership
            .claim_phase(&attempt, AttemptPhase::Gate, effect)
            .unwrap();
        let mut claim = Some((claim, effect));
        finish_gate_claim(&mut ownership, &mut claim, true);
        assert!(ownership.event_is_eligible(&attempt, AttemptPhase::AwaitingGate, effect));

        let claim = ownership
            .claim_phase(&attempt, AttemptPhase::AwaitingGate, effect)
            .unwrap();
        let mut claim = Some((claim, effect));
        finish_gate_claim(&mut ownership, &mut claim, false);
        assert!(!ownership.contains(&attempt));
        finish_gate_claim(&mut ownership, &mut claim, false);
    }

    #[test]
    fn finished_gate_cleanup_preserves_sibling_pending_work() {
        let mut pending = HashMap::from([(
            "plan".to_string(),
            vec!["done".to_string(), "sibling".to_string()],
        )]);
        let state = RunState::new(2);
        let runtime = TaskRuntimeState::capture(&state);
        let mut runtimes = HashMap::from([
            (task_key("plan", "done"), runtime.clone()),
            (task_key("plan", "sibling"), runtime),
        ]);
        let mut executor = ParallelExecutor::new(ExecutorConfig::default());
        executor.add_plan(OrcPlanState::new("plan"));
        let completion = GateCompletion {
            kind: GateCompletionKind::Gate,
            attempt: Some(TaskAttemptRef::new("plan", "done", 1)),
            effect: None,
            plan_id: "plan".to_string(),
            task_id: "done".to_string(),
            rung: 1,
            passed: false,
            failure_kind: Some(RunnerFailureKind::Resource),
            verdicts: Vec::new(),
            output: "producer failed".to_string(),
            duration_ms: 0,
        };

        cleanup_finished_task_gate(&mut pending, &mut runtimes, &mut executor, &completion);

        assert_eq!(pending.get("plan"), Some(&vec!["sibling".to_string()]));
        assert!(!runtimes.contains_key(&task_key("plan", "done")));
        assert!(runtimes.contains_key(&task_key("plan", "sibling")));
        assert!(matches!(
            executor
                .plan_state("plan")
                .map(|state| &state.current_phase),
            Some(PlanPhase::Gating)
        ));
    }
}
