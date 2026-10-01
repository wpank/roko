//! Host adapter that makes converted Graph plan tasks execute real agents.
//!
//! ## Streaming dispatch (#274)
//!
//! [`GraphTaskDispatcher`] now implements [`StreamingTaskDispatcher`] in
//! addition to the basic [`TaskDispatcher`] trait. The streaming path adds:
//!
//! - Live text/tool/usage/progress events forwarded through a bounded channel.
//! - Attempt start/terminal receipts through an injected [`ProviderAttemptRecorder`].
//! - `reconcile_attempt` for crash-safe resume (reuse committed, allocate new,
//!   or fail ambiguous).
//! - Lease validation: the workdir must match the acquired lease path.
//!
//! The existing `TaskDispatcher::dispatch` implementation is unchanged and
//! remains the production plan route until #256 atomically activates the
//! streaming path after lease acquisition.

use std::collections::{HashMap, hash_map::Entry};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use roko_agent::safety::contract::{AgentContract, ContractLoadMode};
use roko_core::config::schema::RokoConfig;
use roko_core::error::{Result, RokoError};
use roko_core::{Body, Context, Kind, Signal, Verify};
use roko_gate::GatePayload;
use roko_gate::ShellGate;
use roko_gate::TurnSnapshot;
use roko_gate::eval_generator::EvalGenerator;
use roko_gate::rung_for_gate_name;
use roko_graph::cell::CellContext;
use roko_graph::cells::task_executor::TaskGateVerdict;
use roko_graph::cells::{
    AttemptReconciliation, GraphTaskEvent, ProviderAttemptRecorder, StreamingTaskDispatcher,
    TaskAttempt, TaskDispatchOutcome, TaskDispatchOutcomeKind, TaskDispatcher, TaskExecutionSpec,
    TaskLease,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::oracles::coding::{BuildRecord, CodingOracle, TestRecord};
use roko_learn::reflex_store::{ReflexObservation, ReflexStore};
use roko_learn::shadow::ShadowRunner;
use roko_learn::telemetry::{AttemptKeyed, AttemptOutcome};

use crate::dispatch::{
    AgentDispatchRequest, DispatchContext, GateFeedback, ModelChoiceSource, SharedAgentFactory,
};
use crate::graph_checkpoint::GraphCostLedgerCheckpoint;
use crate::runner::persist::GateThresholds;
use crate::runner::tui_bridge::TuiBridge;
use crate::runtime_feedback::{FeedbackEvent, FeedbackFacade};
use crate::task_parser::TaskDef;

mod attempt;
mod attempt_workspace;
pub(crate) mod baseline_verify;
mod budget;
mod diff_snapshot;
mod failover;
mod feedback;
mod gate_learning;
mod helper_calls;
mod inert_settings;
mod ladder;
mod prompt_experiment;
mod red_flags;
mod reflex_credit;
mod retry_budget;
mod retry_feedback;
mod routing_context;
mod served_model;
mod sibling_settle;
mod streaming;
mod supervision;
mod tui_forward;
mod turn_policy;
mod verification;
mod watchdog;
mod wiring;

pub use budget::{GraphPlanBudgetPolicy, GraphPlanBudgetSnapshot};
pub use feedback::GraphFeedbackContext;
pub use inert_settings::{InertGraphSetting, graph_engine_inert_settings};
pub(crate) use retry_budget::TaskRetryBudgets;
pub use streaming::streaming_event_channel_capacity;
pub use supervision::{ConductorStop, ConductorTicker, SUPERVISION_INTERVAL};
pub use wiring::{WiringComponent, WiringKind, WiringReport};

use attempt::{AttemptBook, SettledAttempt, Settlement, first_token_seen};
use budget::{
    GraphDailyBudget, GraphPlanBudgetLedger, GraphTaskSpendLedger, effective_routing_budget,
    task_budget_ceiling_usd,
};
use helper_calls::{HelperAgent, HelperCalls};
use inert_settings::warn_inert_graph_settings_once;
use routing_context::{
    CheapFactoryAgent, arbitrate_cross_cut_routing_bias, assign_retrieval_strategy_arm,
    build_routing_context, dream_routing_bias, effective_agent_contract, select_cheap_model_key,
    upstream_outputs,
};
use supervision::SupervisedAttempt;
use tui_forward::forward_live_event_to_tui;
use turn_policy::{
    TurnCapRetry, base_attempt_timeout_ms_with, is_express_task, provider_failure_outcome,
    provider_failure_reason, raised_attempt_timeout_ms, raised_turn_cap, task_turn_limit_with,
    timeout_resume_note, turn_cap_resume_note, verify_failure_reason,
};
use watchdog::{AttemptProgress, StallWatch, WatchedAttempt};

#[cfg(test)]
use turn_policy::task_turn_limit;
#[cfg(test)]
use verification::published_gate_output;

#[cfg(test)]
mod gate_output_accept;

/// How often to publish a [`TuiBridge::agent_heartbeat`] while waiting for a
/// provider dispatch to complete.  5 seconds lets the dashboard show elapsed
/// time at a human-readable granularity without generating excessive events.
const AGENT_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Which live events the dispatcher forwards to the TUI while an agent runs.
///
/// Configured via [`GraphTaskDispatcher::with_live_agent_output`]. The
/// default is `None` (no live output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveAgentOutput {
    /// Forward tool steps only (name + target, no raw text/reasoning).
    ///
    /// Safe to display without additional scrubbing: only the tool ID, tool
    /// name, and the projected/scrubbed target string are forwarded.
    ToolSteps,
    /// Forward tool steps **and** unscreened text/reasoning/tool-call deltas.
    ///
    /// Should only be used for trusted local consumers (e.g. the developer's
    /// TUI), since unscreened content may include sensitive output.
    Trusted,
}

/// Real runner/provider adapter injected into `TaskExecutorCell` factories.
pub struct GraphTaskDispatcher {
    factory: Arc<SharedAgentFactory>,
    config: Arc<RokoConfig>,
    workdir: PathBuf,
    budget_policy: GraphPlanBudgetPolicy,
    budget_ledger: GraphPlanBudgetLedger,
    /// CLI model override (from `--model`). When set, this replaces the
    /// config default and any per-task `model_hint` in dispatch.
    cli_model_override: Option<String>,
    /// Whether to skip agent permission prompts (from `--dangerously-skip-permissions`).
    dangerously_skip_permissions: bool,
    /// Learning/feedback subsystems wired into the Graph engine.
    feedback: GraphFeedbackContext,
    /// Optional per-task worktree isolation provider. When `Some`, each task
    /// dispatch acquires an isolated git worktree via this provider, runs the
    /// agent and verify steps inside it, and on success accepts the attempt
    /// onto its plan branch (see [`Self::accept_attempt`]). When `None` (the
    /// default), all tasks share `self.workdir`.
    workspace_provider: Option<Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>>,
    /// Disk-headroom admission of attempts (reg-7cf6f9); see
    /// [`Self::with_disk_admission`].
    disk_admission: Option<crate::graph_execution::disk_admission::DiskAdmission>,
    /// Checkout generation per task (`"{plan_id}/{task_id}"`): see
    /// [`Self::worktree_generation`].
    worktree_generations: parking_lot::Mutex<HashMap<String, u32>>,
    /// Optional TUI bridge for forwarding live agent output events to the
    /// dashboard. When set, completed dispatch events (text deltas, tool
    /// calls, tool outputs) are published through the StateHub so the TUI
    /// can render agent activity in real time.
    tui_bridge: Option<TuiBridge>,
    /// Which live events to forward to the TUI while the agent runs.
    ///
    /// Defaults to `None` (no live output). Set via
    /// [`Self::with_live_agent_output`]. Requires `tui_bridge` to be set.
    live_agent_output: Option<LiveAgentOutput>,
    /// Per-task gate failure context carried across retries.
    ///
    /// When a task's verify steps fail, the structured gate output is left
    /// here for the task's next attempt, which injects it into the
    /// `DispatchContext` so the agent prompt includes the previous errors.
    /// Plans with a Graph checkpoint also keep it on disk
    /// ([`Self::attach_retry_feedback`]), so a resumed run gets it too.
    gate_retry_context: retry_feedback::RetryFeedbackBook,
    /// Aggregate input tokens accumulated across all dispatches in this run.
    agg_tokens_in: AtomicU64,
    /// Aggregate output tokens accumulated across all dispatches in this run.
    agg_tokens_out: AtomicU64,
    /// Aggregate number of agent dispatch calls in this run.
    agg_dispatch_count: AtomicU64,
    /// Run-scoped cache of static prompt context that does not change per task.
    ///
    /// Contains `(workspace_map, workspace_context, cfactor_context)`.
    /// Computed at most once per plan run on the first dispatch call, then
    /// cloned into every `DispatchContext` to avoid repeated blocking I/O
    /// (filesystem reads + `git` subprocess spawns) on the Tokio reactor.
    static_prompt_cache: std::sync::OnceLock<(String, String, String)>,
    /// Turn caps and timeouts learned per tier from the workspace's settled
    /// attempts, read on the first dispatch (gap-5a6e01).
    learned_tier_limits: std::sync::OnceLock<roko_learn::tier_limits::LearnedTierLimits>,
    /// T0 reflex store. When set and `[learning] t0_reflexes` is on, each
    /// dispatch of a task that no verify step checks (neither its own nor a
    /// workspace rung) looks for a matching reflex rule before invoking the
    /// LLM. A match bypasses the agent call entirely and returns the rule's
    /// cached output (zero-cost repeated decisions), stamped unverified. No
    /// gate runs, so the rule earns no gate pass.
    reflex_store: Option<ReflexStore>,
    /// Per-task spend across attempts, enforcing `budget.max_task_usd` and
    /// `budget.max_task_retry_usd`.
    task_spend: GraphTaskSpendLedger,
    /// Today's spend before this process, for `budget.max_daily_usd`.
    daily_budget: GraphDailyBudget,
    /// Set once the plan run began to stop ([`Self::begin_stop`]).
    stopping: std::sync::atomic::AtomicBool,
    /// `[meta] skip_enrichment` per plan id, read once from the plan's
    /// `tasks.toml`.
    skip_enrichment_plans: parking_lot::Mutex<HashMap<String, bool>>,
    /// Whether each plan's tasks run the workspace's `[[gates.rungs]]`
    /// (`[meta] workspace_rungs`), per plan id, read once from the plan's
    /// `tasks.toml`.
    workspace_rung_plans: parking_lot::Mutex<HashMap<String, bool>>,
    /// Tasks (`"{plan_id}/{task_id}"`) whose last attempt stopped at its turn
    /// cap; the next attempt raises the cap and resumes the partial work.
    turn_cap_retries: parking_lot::Mutex<HashMap<String, TurnCapRetry>>,
    /// Tasks (`"{plan_id}/{task_id}"`) whose last attempt ran out of time,
    /// with that attempt's timeout in ms; the next attempt gets more time
    /// and resumes the partial work.
    timeout_retries: parking_lot::Mutex<HashMap<String, u64>>,
    /// Dispatch attempts this process started per task
    /// (`"{plan_id}/{task_id}"`); [`Self::attempt_in_run`] counts the Graph
    /// engine's retries from it.
    task_attempts: parking_lot::Mutex<HashMap<String, u32>>,
    /// Durable attempt identity per run: ordinals, and the run's
    /// `attempts.jsonl` writer (see [`Self::open_attempt`]).
    attempts: AttemptBook,

    /// RAG-10/11: Per-task retrieval context retained from prompt assembly until
    /// gate settlement.
    ///
    /// Keyed by `"{plan_id}/{task_id}"`.  Value is
    /// `(strategy, query, results_count, latency_ms)`.
    /// Set immediately after `plan()` returns so that both the pre-gate record and
    /// the gate-settled record carry the same metadata.
    retrieval_ctx: parking_lot::Mutex<HashMap<String, (String, String, usize, u64)>>,
    /// Attempts running now, so a verify step that fails while siblings edit
    /// the same working tree can wait for them to settle.
    in_flight: sibling_settle::InFlightTasks,
    /// The tree each task started from, which the pre-verify screen
    /// (`red_flags`) diffs its attempts against.
    diff_bases: diff_snapshot::DiffBases,
    /// Failed test steps run again on the plan run's start commit, to tell
    /// pre-existing failures from new ones (gap-161be1).
    baselines: baseline_verify::Baselines,
    /// Verify runs not yet written to `gate-thresholds.json`: written every
    /// `[learning] gate_threshold_flush_interval` observations, and when the
    /// dispatcher is dropped (reg-c7ecf6).
    gate_threshold_writes: gate_learning::GateThresholdWrites,
    /// The run's conductor, which supervises running attempts (see
    /// [`Self::with_conductor`]).
    conductor: Option<supervision::GraphConductor>,
    /// Plans whose verified tasks wait for a person's approval before they
    /// are accepted (gap-0d64d5, [`Self::hold_for_approval`]).
    approval_plans: parking_lot::Mutex<std::collections::HashSet<String>>,
    /// How long a streaming provider call may report nothing from its start
    /// before the stall watchdog counts its silence
    /// ([`watchdog::FIRST_OUTPUT_GRACE`]; tests shorten it).
    first_output_grace: std::time::Duration,
    /// The plans of this run that are running now, by id, with the areas
    /// their tasks write ([`Self::plan_started`]).
    running_plans: parking_lot::Mutex<std::collections::BTreeMap<String, Vec<String>>>,
}

impl GraphTaskDispatcher {
    /// Construct a dispatcher sharing the plan run's provider runtime.
    #[must_use]
    pub fn new(
        factory: Arc<SharedAgentFactory>,
        config: Arc<RokoConfig>,
        workdir: PathBuf,
    ) -> Self {
        warn_inert_graph_settings_once(&config);
        let flush_interval = config.learning.effective_gate_threshold_flush_interval();
        Self {
            factory,
            config,
            workdir,
            budget_policy: GraphPlanBudgetPolicy::unlimited(),
            budget_ledger: GraphPlanBudgetLedger::default(),
            cli_model_override: None,
            dangerously_skip_permissions: false,
            feedback: GraphFeedbackContext::default(),
            workspace_provider: None,
            disk_admission: None,
            worktree_generations: parking_lot::Mutex::new(HashMap::new()),
            tui_bridge: None,
            live_agent_output: None,
            gate_retry_context: retry_feedback::RetryFeedbackBook::default(),
            agg_tokens_in: AtomicU64::new(0),
            agg_tokens_out: AtomicU64::new(0),
            agg_dispatch_count: AtomicU64::new(0),
            static_prompt_cache: std::sync::OnceLock::new(),
            learned_tier_limits: std::sync::OnceLock::new(),
            reflex_store: None,
            retrieval_ctx: parking_lot::Mutex::new(HashMap::new()),
            task_spend: GraphTaskSpendLedger::default(),
            daily_budget: GraphDailyBudget::default(),
            stopping: std::sync::atomic::AtomicBool::new(false),
            skip_enrichment_plans: parking_lot::Mutex::new(HashMap::new()),
            workspace_rung_plans: parking_lot::Mutex::new(HashMap::new()),
            turn_cap_retries: parking_lot::Mutex::new(HashMap::new()),
            timeout_retries: parking_lot::Mutex::new(HashMap::new()),
            task_attempts: parking_lot::Mutex::new(HashMap::new()),
            attempts: AttemptBook::default(),
            in_flight: sibling_settle::InFlightTasks::default(),
            diff_bases: diff_snapshot::DiffBases::default(),
            baselines: baseline_verify::Baselines::default(),
            gate_threshold_writes: gate_learning::GateThresholdWrites::new(flush_interval),
            conductor: None,
            approval_plans: parking_lot::Mutex::default(),
            first_output_grace: watchdog::FIRST_OUTPUT_GRACE,
            running_plans: parking_lot::Mutex::default(),
        }
    }

    /// Set the CLI model override (from `--model`).
    ///
    /// When set, this bypasses adaptive routing and forces all graph task
    /// dispatches to use the specified model slug.
    #[must_use]
    pub fn with_cli_model_override(mut self, model: Option<String>) -> Self {
        self.cli_model_override = model;
        self
    }

    /// Enable or disable permission skipping (from `--dangerously-skip-permissions`).
    #[must_use]
    pub fn with_dangerously_skip_permissions(mut self, skip: bool) -> Self {
        self.dangerously_skip_permissions = skip;
        self
    }

    /// Attach the learning/feedback subsystem context.
    #[must_use]
    pub fn with_feedback(mut self, feedback: GraphFeedbackContext) -> Self {
        self.feedback = feedback;
        self
    }

    /// Enable per-task worktree isolation via the given workspace provider.
    ///
    /// When set, each `dispatch` call will:
    /// 1. Acquire an isolated worktree for the task attempt.
    /// 2. Run the agent and verify steps inside the worktree.
    /// 3. On success, accept the attempt onto its plan branch and keep the
    ///    worktree for review (`RetainForReview`); on failure, keep it for
    ///    post-mortem (`RetainForFailure`). A task whose `plan.gate` follows
    ///    hands the worktree on to it instead.
    ///
    /// This is opt-in via `--worktree-per-task` and defaults to `None` (shared workdir).
    #[must_use]
    pub fn with_workspace_provider(
        mut self,
        provider: Arc<dyn roko_graph::workspace::ExecutionWorkspaceProvider>,
    ) -> Self {
        self.workspace_provider = Some(provider);
        self
    }

    /// Reserve each attempt's disk headroom before it starts, waiting under
    /// disk pressure until running attempts end (reg-7cf6f9). `plan run`
    /// sets it with `--worktree-per-task`.
    #[must_use]
    pub fn with_disk_admission(
        mut self,
        admission: crate::graph_execution::disk_admission::DiskAdmission,
    ) -> Self {
        self.disk_admission = Some(admission);
        self
    }

    /// Attach a TUI bridge for forwarding live agent output events.
    ///
    /// When set, agent dispatch events (text deltas, tool calls, tool
    /// outputs, spawned/completed lifecycle) are published through the
    /// StateHub so the TUI dashboard can render agent activity during
    /// Graph plan execution.
    #[must_use]
    pub fn with_tui_bridge(mut self, bridge: TuiBridge) -> Self {
        self.tui_bridge = Some(bridge);
        self
    }

    /// Configure which live events are forwarded to the TUI while an agent runs.
    ///
    /// Requires [`Self::with_tui_bridge`] to also be configured. When set,
    /// each task dispatch creates a bounded channel, attaches it to the
    /// [`AgentDispatchRequest`] so the immune boundary can push events, and
    /// spawns a forwarder task that publishes each event to the TUI bridge
    /// before the screened transcript arrives via
    /// [`Self::forward_dispatch_events_to_tui`].
    ///
    /// Default setting: [`LiveAgentOutput::ToolSteps`] when called with `None`
    /// is not applicable — call this method with the desired variant.
    #[must_use]
    pub fn with_live_agent_output(mut self, setting: LiveAgentOutput) -> Self {
        self.live_agent_output = Some(setting);
        self
    }

    /// Attach the T0 reflex store for pre-dispatch reflex checks.
    ///
    /// With `[learning] t0_reflexes` on (off by default), each `dispatch` of
    /// a task that no verify step checks (neither its own nor a workspace
    /// rung) looks for a rule that matches the task's role, file extensions,
    /// and title before invoking the LLM. A match bypasses the agent call and
    /// returns the rule's cached output (`action.args`), stamped unverified.
    /// No gate runs, so the rule is not credited with a gate pass.
    #[must_use]
    pub fn with_reflex_store(mut self, store: ReflexStore) -> Self {
        self.reflex_store = Some(store);
        self
    }

    /// Apply a per-plan cost ceiling to subsequent task dispatches.
    #[must_use]
    pub fn with_plan_budget(
        mut self,
        ceiling_usd: f64,
        max_turn_usd: f64,
        continue_on_exhaustion: bool,
    ) -> Self {
        self.budget_policy =
            GraphPlanBudgetPolicy::from_limits(ceiling_usd, max_turn_usd, continue_on_exhaustion);
        self
    }

    /// Restore and attach the durable actual-provider-cost state for a plan.
    pub fn attach_plan_budget_checkpoint(
        &self,
        plan_id: &str,
        checkpoint: GraphCostLedgerCheckpoint,
    ) -> Result<()> {
        self.budget_ledger.attach_checkpoint(plan_id, checkpoint)
    }

    /// Hold each verified task of `plan_id` for a person's approval before
    /// it is accepted onto the plan branch (gap-0d64d5, `[meta] approval =
    /// "per_task"`). See [`Self::await_review`].
    pub fn hold_for_approval(&self, plan_id: &str) {
        self.approval_plans.lock().insert(plan_id.to_string());
    }

    /// Whether `plan_id`'s verified tasks wait for approval.
    fn holds_for_approval(&self, plan_id: &str) -> bool {
        self.approval_plans.lock().contains(plan_id)
    }

    /// Note that `plan_id`, whose tasks write `areas`, now runs in this
    /// dispatcher's working tree. Agents of the run's other plans are told
    /// of it, so they keep their builds to their own crates (gap-c09fc7).
    pub fn plan_started(&self, plan_id: &str, areas: Vec<String>) {
        self.running_plans.lock().insert(plan_id.to_string(), areas);
    }

    /// Note that `plan_id` no longer runs.
    pub fn plan_finished(&self, plan_id: &str) {
        self.running_plans.lock().remove(plan_id);
    }

    /// The running plans other than `plan_id`, with the areas they write.
    fn concurrent_plans(&self, plan_id: &str) -> Vec<(String, Vec<String>)> {
        self.running_plans
            .lock()
            .iter()
            .filter(|(id, _)| id.as_str() != plan_id)
            .map(|(id, areas)| (id.clone(), areas.clone()))
            .collect()
    }

    /// Keep `plan_id`'s pending retry feedback in `path`, beside its Graph
    /// checkpoint of run `run_id`, restoring what an earlier process of that
    /// run left for its tasks' next attempts.
    pub fn attach_retry_feedback(&self, plan_id: &str, path: PathBuf, run_id: &str) {
        let restored = self
            .gate_retry_context
            .attach(plan_id, path.clone(), run_id);
        if !restored.is_empty() {
            tracing::info!(
                plan_id,
                tasks = %restored.join(", "),
                path = %path.display(),
                "restored gate feedback for the next attempts of resumed tasks"
            );
        }
        // The tasks' spend and turn-cap retries the earlier process kept
        // (gap-34b2ed).
        for (task_id, micro_usd) in self.gate_retry_context.kept_task_spend(plan_id) {
            let key = format!("{plan_id}/{task_id}");
            self.task_spend.restore(&key, micro_usd);
        }
        for (task_id, retry) in self.gate_retry_context.kept_turn_caps(plan_id) {
            let key = format!("{plan_id}/{task_id}");
            self.turn_cap_retries.lock().insert(key, retry);
        }
    }

    /// Retry budgets of the tasks of the plan in `plan_dir`: authored ones as
    /// written, the rest set by `[gates]` and the adaptive gate thresholds
    /// this dispatcher's verify runs record (see [`TaskRetryBudgets`]). The
    /// verify runs held for the flush interval are written first, so the
    /// budgets see every one so far.
    pub(crate) fn task_retry_budgets(&self, plan_dir: &Path) -> TaskRetryBudgets {
        self.gate_threshold_writes.flush();
        let tasks_toml = [plan_dir.to_path_buf(), self.workdir.join(plan_dir)]
            .into_iter()
            .map(|dir| dir.join("tasks.toml"))
            .find(|path| path.is_file())
            .unwrap_or_else(|| plan_dir.join("tasks.toml"));
        TaskRetryBudgets::load(
            self.feedback.gate_thresholds_path.as_deref(),
            &self.config.gates,
            &tasks_toml,
        )
        .with_ladder_min_retries(self.ladder_min_retries())
    }

    /// This process's index of the attempt of `task_key` that
    /// [`Self::open_attempt`] opened last: attempt `k` is the Graph engine's
    /// retry `k`.
    fn attempt_in_run(&self, task_key: &str) -> u32 {
        self.task_attempts
            .lock()
            .get(task_key)
            .map_or(0, |started| started.saturating_sub(1))
    }

    /// Attempt number and pending gate feedback of the dispatch of `task_id`
    /// that [`Self::open_attempt`] just opened.
    fn next_retry_attempt(&self, plan_id: &str, task_id: &str) -> retry_feedback::NextAttempt {
        let attempt_in_run = self.attempt_in_run(&format!("{plan_id}/{task_id}"));
        self.gate_retry_context
            .next_attempt(plan_id, task_id, attempt_in_run)
    }

    /// Return the current cost state for `plan_id`.
    #[must_use]
    pub fn plan_budget_snapshot(&self, plan_id: &str) -> GraphPlanBudgetSnapshot {
        self.budget_ledger.snapshot(plan_id, self.budget_policy)
    }

    /// Why no further task of `plan_id` may be dispatched in this run, when
    /// that is so: its settled spend reached the plan ceiling, or today's
    /// reached `budget.max_daily_usd` (and no explicit override lets it
    /// continue), or its cost ledger cannot be persisted. In-flight
    /// reservations alone never stop a plan.
    #[must_use]
    pub fn plan_dispatch_stop(&self, plan_id: &str) -> Option<String> {
        self.budget_ledger
            .dispatch_stop(plan_id, self.budget_policy)
            .or_else(|| self.daily_dispatch_stop())
    }

    /// The plan run began to stop (an interrupt), and is signalling its
    /// agents. From now on a provider call that ends without a successful
    /// result, an agent that exits on that SIGTERM included, settles as
    /// cancelled rather than as a provider failure, and fails with
    /// [`RokoError::Cancelled`], which the task executor does not retry
    /// (bug-28b604).
    pub fn begin_stop(&self) {
        self.stopping.store(true, Ordering::Release);
    }

    /// The cancellation a call of `plan_id/task_id` that ended with `cause`
    /// becomes once its run began to stop ([`Self::begin_stop`]).
    fn stopped_call(&self, plan_id: &str, task_id: &str, cause: &str) -> Option<RokoError> {
        self.stopping.load(Ordering::Acquire).then(|| {
            RokoError::cancelled(format!(
                "agent for {plan_id}/{task_id} ended while its plan run was stopping: {cause}"
            ))
        })
    }

    /// Return aggregate token and dispatch counts accumulated across all
    /// task dispatches in this run. Used by the run-metrics persistence
    /// path (backlog #169) to populate `RunMetricsRecord` with real values
    /// instead of zeros.
    #[must_use]
    pub fn run_aggregate_stats(&self) -> (u64, u64, u64) {
        (
            self.agg_tokens_in.load(Ordering::Relaxed),
            self.agg_tokens_out.load(Ordering::Relaxed),
            self.agg_dispatch_count.load(Ordering::Relaxed),
        )
    }

    /// Return a helper agent wired to the model chosen by
    /// [`select_cheap_model_key`], or `None` when no model is dispatchable.
    /// Used for best-effort error enrichment, quality judgment and gate
    /// reflection calls, which count toward the attempt being verified
    /// ([`HelperAgent`]).
    fn cheap_agent(&self) -> Option<HelperAgent> {
        let model_key = select_cheap_model_key(&self.config)?;
        let target = crate::dispatch_v2::ProviderDispatchResolver::new(Arc::clone(&self.config))
            .resolve(&model_key);
        let agent = CheapFactoryAgent {
            factory: Arc::clone(&self.factory),
            model_key,
            workdir: self.workdir.clone(),
            timeout_ms: self
                .config
                .timeouts
                .llm_call_secs
                .max(1)
                .saturating_mul(1_000),
        };
        Some(HelperAgent::new(agent, target))
    }

    /// The `[meta]` of `spec`'s plan, from `<plan_dir>/tasks.toml`; `None`
    /// when the file is missing or unreadable.
    fn read_plan_meta(&self, spec: &TaskExecutionSpec) -> Option<crate::task_parser::TaskMeta> {
        let plan_dir = Path::new(&spec.plan_dir);
        [plan_dir.to_path_buf(), self.workdir.join(plan_dir)]
            .into_iter()
            .filter(|_| !spec.plan_dir.trim().is_empty())
            .map(|dir| dir.join("tasks.toml"))
            .find(|path| path.is_file())
            .and_then(|path| crate::task_parser::TasksFile::parse(&path).ok())
            .map(|tasks| tasks.meta)
    }

    /// Whether the plan's `[meta] skip_enrichment` is set, read once per plan
    /// from `<plan_dir>/tasks.toml`. An unreadable file counts as `false`.
    fn plan_skips_enrichment(&self, spec: &TaskExecutionSpec) -> bool {
        let mut plans = self.skip_enrichment_plans.lock();
        if let Some(skip) = plans.get(&spec.plan_id) {
            return *skip;
        }
        let skip = self
            .read_plan_meta(spec)
            .is_some_and(|meta| meta.skip_enrichment);
        if skip {
            tracing::info!(
                plan_id = %spec.plan_id,
                "plan sets skip_enrichment: dispatching tasks as authored \
                 (no eval artifacts, no dream/cross-cut routing advice)"
            );
        }
        plans.insert(spec.plan_id.clone(), skip);
        skip
    }

    /// Per-task spend admission against [`task_budget_ceiling_usd`], mirroring
    /// the plan ceiling: an explicit `--budget` override only warns, and
    /// `--no-budget` disables the check.
    fn admit_task_budget(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        task_spend_key: &str,
    ) -> Result<()> {
        let policy = self.budget_policy;
        if policy.continue_on_exhaustion && policy.ceiling_micro_usd.is_none() {
            return Ok(());
        }
        let ceiling_usd = task_budget_ceiling_usd(&self.config.budget, task);
        let Err(error) = self.task_spend.admit(task_spend_key, ceiling_usd) else {
            return Ok(());
        };
        if policy.continue_on_exhaustion {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                ceiling_usd,
                %error,
                "per-task budget exhausted; continuing under the explicit --budget override"
            );
            return Ok(());
        }
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            ceiling_usd,
            %error,
            "per-task budget exhausted (budget.max_task_usd x tier multiplier, \
             budget.max_task_retry_usd); refusing another attempt"
        );
        Err(error)
    }

    /// The Graph checkpoint run of `plan_id`, once its retry feedback is
    /// attached ([`Self::attach_retry_feedback`]). Its attempt keys carry it.
    #[must_use]
    pub fn plan_run_id(&self, plan_id: &str) -> Option<String> {
        self.gate_retry_context.run_id(plan_id)
    }

    /// Plan a dispatch. When prompt assembly fails with experiment
    /// treatments (say, two running experiments on one section), plan again
    /// without them: a broken experiment must not stop the task.
    fn plan_dispatch(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch_ctx: &mut DispatchContext,
    ) -> Result<crate::dispatch::RunnerDispatchPlan> {
        // The prompt shows every check that will judge the task: its own
        // verify steps, then the workspace rungs it faces.
        let task = &self.prompt_task(spec, task);
        match self.factory.dispatcher().plan(task, dispatch_ctx) {
            Err(error) if dispatch_ctx.prompt_experiment.is_some() => {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "prompt assembly with experiment treatments failed; dispatching without them"
                );
                dispatch_ctx.prompt_experiment = None;
                self.factory.dispatcher().plan(task, dispatch_ctx)
            }
            planned => planned,
        }
        .map_err(|error| RokoError::Planning(error.to_string()))
    }
}

#[async_trait::async_trait]
impl TaskDispatcher for GraphTaskDispatcher {
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>> {
        self.admit_daily_budget(spec).await?;
        // A task that starts while another call holds the plan's remaining
        // budget waits for it to settle (bug-0bc2b4).
        let budget_reservation = self
            .budget_ledger
            .reserve_waiting(&spec.plan_id, self.budget_policy, || ctx.is_cancelled())
            .await?;

        let task: TaskDef = serde_json::from_str(&spec.task_def_json).map_err(|error| {
            RokoError::Planning(format!(
                "decode task definition for `{}`: {error}",
                spec.title
            ))
        })?;
        let task_spend_key = format!("{}/{}", spec.plan_id, task.id);
        self.admit_task_budget(spec, &task, &task_spend_key)?;

        // ── Role-enabled check ──────────────────────────────────────────
        //
        // When a role is disabled via `[agent.roles.<role>] enabled = false`,
        // the task fails without a dispatch (bug-a843d4). Completing it
        // would pass work that was never done, its verify steps unrun. The
        // rejection is not retried, and a resume runs the task again.
        if let Some(role_label) = task.role.as_deref() {
            if !crate::config_helpers::is_role_enabled(&self.config, role_label) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    role = role_label,
                    "role is disabled in config; failing task"
                );
                return Err(RokoError::Rejected(format!(
                    "task `{}` was not run: its role `{role_label}` is disabled in config \
                     ([agent.roles.{role_label}] enabled = false); enable the role and \
                     resume the plan",
                    task.id
                )));
            }
        }

        // ── T0 reflex check ─────────────────────────────────────────────
        //
        // With `[learning] t0_reflexes` on (off by default), check the reflex
        // store for a matching deterministic rule before invoking the LLM. A
        // rule fires when every populated field of its `ReflexCondition`
        // matches the task's observable attributes.  The observation is built
        // from the task's role (→ `message_type`), title (→ `context`), and the
        // unique file extensions present in `task.files` (→ `file_exts`).
        // When a rule matches:
        //
        //   1. The agent call is skipped entirely.
        //   2. The rule's `action.args` field is used as the cached output text.
        //   3. The output is stamped `Unverified`. No gate has run, so the rule
        //      earns no gate pass.
        //
        // This implements the "zero-cost repeated decisions" pattern: tasks
        // that succeed repeatedly with the same structural signature can be
        // served from the T0 store without an LLM round-trip.
        //
        // Reflexes skip the provider *and* the verify steps, so they only
        // serve tasks that no verify step checks: a task with its own steps,
        // or one the workspace rungs check, must earn its pass from them.
        if let Some(reflex_store) = self.reflex_store.as_ref().filter(|_| {
            self.config.learning.t0_reflexes && self.verify_steps(spec, &task).is_empty()
        }) {
            let file_exts: Vec<String> = task
                .files
                .iter()
                .filter_map(|f| {
                    std::path::Path::new(f)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| format!(".{ext}"))
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let observation = ReflexObservation {
                tool: None,
                args: None,
                context: Some(task.title.clone()),
                message_type: task.role.clone(),
                file_exts,
            };

            if let Some(reflex_match) = reflex_store.match_observation_with_id(&observation) {
                let rule_id = reflex_match.rule_id;
                let cached_output = reflex_match.action.args.clone();
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %rule_id,
                    "T0 reflex matched: skipping LLM dispatch, using cached output"
                );
                // Settle the budget reservation at zero cost (no LLM call).
                budget_reservation.settle(0.0)?;
                // The rule serves this attempt, and only the attempt's settled
                // record credits or demotes it (`reflex_credit`). Unverified,
                // it teaches the rule nothing.
                let mut attempt = self.open_attempt(spec, &task, ctx);
                attempt.served_by_reflex(rule_id);
                let settlement = Settlement::verified(&Ok(TaskGateVerdict::Unverified));
                let settled = attempt.settle(settlement, "", None);
                self.publish_settlement(spec, &task, &settled).await;
                let output_signal = Signal::builder(Kind::AgentOutput)
                    .body(Body::text(cached_output))
                    .build();
                let mut outputs = vec![output_signal];
                TaskGateVerdict::Unverified.stamp(&mut outputs);
                return Ok(outputs);
            }
        }

        // A plan with `[meta] skip_enrichment = true` is dispatched as
        // authored: no eval artifacts and no dream/cross-cut routing advice.
        let skip_enrichment = self.plan_skips_enrichment(spec);

        // ── P0-02: EvalGenerator pre-dispatch ───────────────────────────
        //
        // For standard-tier and above tasks, write the evaluations that pass
        // `generate_checked` (each holds a `#[test]` that can fail) to
        // `.roko/generated-tests/`, not the repo root, before the agent
        // starts. Opt-in via `gates.write_eval_artifacts`: nothing in `plan
        // run` executes them, and the built-in template needs an assertion
        // body that Graph tasks do not author, so none is written today.
        if self.feedback.eval_generation_enabled
            && self.config.gates.write_eval_artifacts
            && !skip_enrichment
        {
            let is_standard_or_above = task.tier_class() != roko_core::task::TaskTier::Mechanical;
            if is_standard_or_above {
                let target_crates = crate::task_helpers::task_target_crates(Some(&task));
                let primary_crate = target_crates
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "roko-cli".to_string());
                let generator = EvalGenerator::new();
                let (evals, rejected) =
                    generator.generate_checked_all(&task.title, &primary_crate, &task.files, None);
                for error in &rejected {
                    tracing::debug!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "P0-02: eval template rejected (non-fatal)"
                    );
                }
                if !evals.is_empty() {
                    let gen_dir = self.workdir.join(".roko").join("generated-tests");
                    if let Err(err) = std::fs::create_dir_all(&gen_dir) {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P0-02: failed to create generated-tests dir (non-fatal)"
                        );
                    } else {
                        for eval in &evals {
                            let file_name = format!("{}.rs", eval.name);
                            let file_path = gen_dir.join(&file_name);
                            if let Err(err) = std::fs::write(&file_path, &eval.test_source) {
                                tracing::warn!(
                                    plan_id = %spec.plan_id,
                                    task_id = %task.id,
                                    file = %file_name,
                                    error = %err,
                                    "P0-02: failed to write generated eval (non-fatal)"
                                );
                            }
                        }
                        tracing::debug!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            eval_count = evals.len(),
                            "P0-02: generated eval artifacts before dispatch"
                        );
                    }
                }
            }
        }

        // ── P2-01: ShadowRunner decision recording ──────────────────────
        //
        // Record whether this task would be shadowed. Infrastructure-only:
        // we record the decision but do not actually spawn a shadow task.
        if let Some(shadow) = &self.feedback.shadow_runner {
            let should = shadow.should_shadow();
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                should_shadow = should,
                shadow_model = %shadow.config.model_slug,
                "P2-01: shadow decision recorded (infrastructure-only)"
            );
        }

        // ── Disk headroom (reg-7cf6f9) ───────────────────────────────────
        //
        // Reserve the space the attempt's worktree is expected to grow by,
        // waiting under disk pressure until running attempts end. The
        // reservation ends with this attempt.
        let _disk_reservation = match &self.disk_admission {
            Some(admission) => Some(admission.admit().await),
            None => None,
        };

        // ── Worktree isolation: acquire ─────────────────────────────────
        //
        // When a workspace provider is configured, acquire an isolated
        // worktree for this task attempt. The agent and verify steps will
        // run inside it instead of the shared repository root.
        let attempt_id = roko_graph::workspace::WorkspaceAttemptId {
            plan_id: spec.plan_id.clone(),
            task_id: task.id.clone(),
            // The task's checkout, not one attempt's: the provider's acquire
            // is idempotent, so every retry reuses it and resumes the work its
            // predecessor left, until the plan branch refuses that work (see
            // `worktree_generation`). The attempt itself is named by its key
            // (`open_attempt`).
            attempt: self.worktree_generation(&task_spend_key),
        };
        let lease = if let Some(provider) = &self.workspace_provider {
            let lease = provider
                .acquire(&attempt_id)
                .await
                .map_err(|e| RokoError::Agent {
                    backend: "worktree-isolation".to_string(),
                    message: format!("failed to acquire worktree for {attempt_id}: {e}"),
                })?;
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                worktree = %lease.path.display(),
                "acquired isolated worktree for task"
            );
            Some(lease)
        } else {
            None
        };
        // Effective working directory: worktree path if isolated, else shared workdir.
        let effective_workdir = lease
            .as_ref()
            .map_or_else(|| self.workdir.clone(), |l| l.path.clone());
        // A git process killed mid-command (an earlier attempt's agent, a
        // crashed run) leaves `index.lock` behind, and every index-writing git
        // command here then fails: clear a stale one before the agent starts
        // (bug-109b5a). The shared checkout is the user's, whose own git may
        // hold the lock for minutes (a commit waiting on its editor), so a lock
        // there must be much older than in a roko-owned worktree.
        let stale_index_lock_after = if lease.is_some() {
            std::time::Duration::from_secs(roko_core::defaults::DEFAULT_STALE_LOCK_SECS)
        } else {
            std::time::Duration::from_mins(10)
        };
        crate::orchestrator::worktree::clear_stale_index_lock(
            &effective_workdir,
            stale_index_lock_after,
        );
        // Until this attempt ends, a sibling's failed verify step in the same
        // working tree may wait for it to settle. It starts editing once no
        // sibling runs a verify step that reads its files (gap-1920ba).
        let _in_flight = self
            .in_flight
            .register_when_unread(
                &task_spend_key,
                &effective_workdir,
                &task.files,
                std::time::Duration::from_secs(self.config.gates.sibling_settle_secs),
            )
            .await;

        let role = task.role.as_deref().unwrap_or("implementer");

        // ── P3-AGT-2: Express mode check ────────────────────────────────
        //
        // When `conductor.express_mode = true` and the task tier is
        // "mechanical" or "trivial", bypass the full routing pipeline:
        //   - Route to `routing.fast_task_model` (cheapest available model).
        //   - Cap the agent turn limit at EXPRESS_MAX_TURNS (5).
        //   - Eval-generation is already skipped by the tier check above.
        // A CLI `--model` override (`cli_model_override`) takes precedence over
        // express routing so manual experiments are not silently replaced.
        let express_active = is_express_task(&self.config, &task);
        let mut max_turns = task_turn_limit_with(
            &self.config,
            Some(self.learned_tier_limits()),
            &task,
            express_active,
        );
        // The last attempt stopped at its turn cap with partial work on disk:
        // raise the cap and tell the agent to resume, never rerun the same cap.
        let turn_cap_resume = self.take_turn_cap_retry(&spec.plan_id, &task.id);
        if let Some(previous) = turn_cap_resume {
            max_turns = max_turns.max(raised_turn_cap(previous.cap));
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                previous_cap = previous.cap,
                max_turns,
                "previous attempt hit its turn cap; resuming with a raised cap"
            );
        }
        if express_active {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                tier = %task.tier,
                fast_model = %self.config.routing.fast_task_model,
                max_turns,
                "P3-AGT-2: express mode active — routing to fast model with reduced turn limit"
            );
        }

        // ── W10: Enrichment pipeline ─────────────────────────────────────
        let mut routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);
        // Clone before the move into DispatchContext so emit_feedback can pass
        // the real dispatch-time context to the routing observation sink.
        // This ensures force_backend override outcomes are recorded with the
        // correct task category, complexity, and role rather than fallback defaults.
        let mut routing_ctx_for_feedback = routing_ctx.clone();

        // Load persisted dream routing advice once; both the cross-cut
        // arbitration and the P1-18 dream bias read it. Plans that skip
        // enrichment get neither.
        let routing_bias = if skip_enrichment {
            None
        } else {
            let dream_advice = roko_dreams::load_dream_routing_advice(&self.workdir).ok();
            // P1-16: Run cross-cut arbitration to detect safety-critical
            // overrides before applying dream routing advice.
            let task_category = task.domain.as_ref().map_or("implementation", |d| d.label());
            let arbitration_bias = arbitrate_cross_cut_routing_bias(
                &self.feedback,
                dream_advice.as_ref(),
                task_category,
            );

            // P1-18: Convert the dream advice to a RoutingBias so the cascade
            // router accounts for dream-observed model performance when
            // picking a provider for this task. Arbitration safety overrides
            // take priority over dream advice.
            arbitration_bias
                .or_else(|| dream_routing_bias(dream_advice.as_ref(), task_category, &routing_ctx))
        };

        // ── Gate retry context lookup ──────────────────────────────────
        //
        // If this task was previously dispatched and failed verification,
        // the gate_retry_context holds the structured errors. Injecting them
        // into the DispatchContext causes the prompt assembler to include a
        // "Previous attempt feedback" section with the actual
        // compile/test/clippy errors so the agent can fix them. The attempt
        // number counts every earlier dispatch, as the engine's retries do.
        let retry_key = format!("{}/{}", spec.plan_id, task.id);
        // The attempt opens before prompt assembly (S01 §4.2): if the process
        // dies from here on, the attempt keeps its key and counts as
        // abandoned.
        let mut attempt = self.open_attempt(spec, &task, ctx);
        // The tree the task starts from, before its agent runs, for the
        // pre-verify screen's diff (`red_flags`).
        self.record_diff_base(
            &attempt.key.attempt_key(),
            &effective_workdir,
            lease.as_ref().map(|lease| lease.base_revision.as_str()),
        )
        .await;
        let retry_feedback::NextAttempt {
            attempt: attempt_number,
            feedback: prior_gate_feedback,
        } = self.next_retry_attempt(&spec.plan_id, &task.id);

        if prior_gate_feedback.is_some() {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                attempt = attempt_number,
                has_gate_feedback = prior_gate_feedback.is_some(),
                "graph dispatch: injecting gate feedback from previous attempt"
            );
        }

        // gap-b62e95: the router and its observations know a retry from a
        // first attempt. (The dream bias above keeps the first attempt's band.)
        routing_context::mark_attempt(&mut routing_ctx, &task, attempt_number);
        routing_context::mark_attempt(&mut routing_ctx_for_feedback, &task, attempt_number);

        let (cached_workspace_map, cached_workspace_context, cached_cfactor_context) =
            self.static_prompt_cache.get_or_init(|| {
                let ws_map =
                    crate::dispatch::prompt_builder::generate_workspace_map_pub(&self.workdir);
                let ws_ctx =
                    crate::dispatch::prompt_builder::generate_workspace_context_pub(&self.workdir);
                let cf_ctx =
                    crate::dispatch::prompt_builder::generate_cfactor_context_pub(&self.workdir);
                tracing::debug!(
                    ws_map_bytes = ws_map.len(),
                    ws_ctx_bytes = ws_ctx.len(),
                    cf_ctx_bytes = cf_ctx.len(),
                    "static_prompt_cache: computed once for this run"
                );
                (ws_map, ws_ctx, cf_ctx)
            });
        // Express mode sets force_backend to the fast model unless the operator
        // has already supplied a --model override (cli_model_override takes
        // priority so manual experiments are not silently replaced).
        let express_force_backend = if express_active && self.cli_model_override.is_none() {
            Some(self.config.routing.fast_task_model.clone())
        } else {
            None
        };
        // Durable, attempt-scoped prompt treatments from the root workspace's
        // experiment store; settled with the attempt's outcome in
        // `emit_feedback`.
        let prompt_experiment = self
            .feedback
            .experiment_store_path
            .as_deref()
            .and_then(|store| prompt_experiment::context(store, &attempt.key));
        let ladder_step = self.ladder_step(spec, &task);
        let mut dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: effective_workdir.clone(),
            // Task-authored model_hint flows through to RoutingInputs where it
            // beats the cascade router but loses to force_backend.  When the
            // task has no hint we leave this None so the cascade router can
            // make its own decision rather than short-circuiting to the config
            // default.
            model_hint: task.model_hint.clone(),
            force_backend: self.cli_model_override.clone().or(express_force_backend),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: attempt_number,
            ladder_step,
            prompt_experiment: prompt_experiment.clone(),
            gate_feedback: prior_gate_feedback,
            routing_context: Some(routing_ctx),
            routing_bias,
            dependency_outputs: upstream_outputs(&input),
            error_patterns_context: self.factory.format_error_patterns_for_prompt(5),
            cached_workspace_map: cached_workspace_map.clone(),
            cached_workspace_context: cached_workspace_context.clone(),
            cached_cfactor_context: cached_cfactor_context.clone(),
            concurrent_plans: self.concurrent_plans(&spec.plan_id),
        };
        let prompt_assembly_started = std::time::Instant::now();
        let dispatch_plan = match self.plan_dispatch(spec, &task, &mut dispatch_ctx) {
            Ok(dispatch_plan) => dispatch_plan,
            Err(error) => return Err(self.fail_attempt(spec, &task, attempt, None, error).await),
        };
        let prompt_assembly_latency_ms = prompt_assembly_started.elapsed().as_millis() as u64;
        attempt.prompt_assembled();
        self.record_attempt_ladder(&mut attempt, spec, &task, &dispatch_plan, ladder_step);

        // ── RAG-10/11: Retrieval outcome telemetry (pre-gate) ────────────
        //
        // Immediately after prompt assembly we know:
        //   - which strategy was used (RAG-11 experiment assignment or default)
        //   - how many knowledge entries were retrieved (diagnostics.knowledge_ids)
        //   - the query text (task title + description)
        //   - prompt assembly latency (covers neuro knowledge retrieval)
        //
        // We record a pre-gate record now and a settled record after verify.
        {
            let results_count = dispatch_plan.prompt.diagnostics.knowledge_ids.len();
            let query = format!(
                "{} {}",
                task.title,
                task.description.as_deref().unwrap_or("")
            )
            .trim()
            .to_string();

            // RAG-11: assign retrieval strategy via experiment store, or fall
            // back to the default "keyword" arm (which is what the current
            // `collect_neuro_knowledge_cached` always runs). The store read is
            // blocking file I/O, so it runs off the reactor.
            let strategy = if let Some(exp_path) = self.feedback.experiment_store_path.clone() {
                tokio::task::spawn_blocking(move || assign_retrieval_strategy_arm(&exp_path))
                    .await
                    .unwrap_or_else(|_| roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string())
            } else {
                roko_learn::retrieval_outcome::STRATEGY_KEYWORD.to_string()
            };

            // Stash for gate-settlement below.
            self.retrieval_ctx.lock().insert(
                retry_key.clone(),
                (
                    strategy.clone(),
                    query.clone(),
                    results_count,
                    prompt_assembly_latency_ms,
                ),
            );

            // Write the pre-gate record (best-effort, non-blocking).
            if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                let record = roko_learn::retrieval_outcome::RetrievalOutcomeRecord::pre_gate(
                    &spec.plan_id,
                    &task.id,
                    &query,
                    &strategy,
                    results_count,
                )
                .with_latency_ms(prompt_assembly_latency_ms);
                crate::background_writes::spawn(&path.clone(), async move {
                    if let Err(error) =
                        roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                            .without_fsync()
                            .append(&record)
                            .await
                    {
                        tracing::warn!(
                            %error,
                            "RAG-10: pre-gate retrieval outcome write failed (best-effort)"
                        );
                    }
                });
            }
        }

        let contract = effective_agent_contract(role, &task, &self.config);
        let base_timeout_ms =
            base_attempt_timeout_ms_with(&self.config, Some(self.learned_tier_limits()), spec);
        // The last attempt ran out of time with partial work on disk: give
        // this one half again as long (bounded) and tell it to resume, never
        // rerun the budget that already ran out.
        let timeout_resume = self.timeout_retries.lock().remove(&task_spend_key);
        let timeout_ms = timeout_resume.map_or(base_timeout_ms, |previous_ms| {
            let raised = raised_attempt_timeout_ms(previous_ms, base_timeout_ms);
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                previous_timeout_ms = previous_ms,
                timeout_ms = raised,
                "previous attempt timed out; resuming with an escalated timeout"
            );
            raised
        });
        let mut prompt = dispatch_plan.prompt.user_prompt.clone();
        if let Some(previous) = turn_cap_resume {
            prompt.push_str(&turn_cap_resume_note(previous, max_turns));
        }
        if let Some(previous_ms) = timeout_resume {
            prompt.push_str(&timeout_resume_note(previous_ms, timeout_ms));
        }
        let mut request = AgentDispatchRequest {
            model_key: self.dispatch_model_key(&dispatch_plan, &task),
            prompt,
            system_prompt: dispatch_plan.prompt.system_prompt.clone(),
            workdir: effective_workdir.clone(),
            // Immune state (tool controls, evidence, the quarantine vault)
            // belongs to the workspace, not the attempt checkout, so it
            // survives checkout cleanup and the safety routes see it.
            immune_root: Some(self.workdir.clone()),
            agent_id: format!(
                "{}/{}",
                spec.plan_id,
                ctx.cell_id.as_deref().unwrap_or(&task.id)
            ),
            command: None,
            timeout_ms: Some(timeout_ms),
            mcp_config: self.config.agent.mcp_config.clone(),
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: Some(self.config.agent.default_effort.clone()),
            tools: None,
            agent_contract: Some(contract),
            bare_mode: self.config.agent.bare_mode,
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            // Tier turn cap from `[pipeline.<tier>] max_turns`, lowered to
            // EXPRESS_MAX_TURNS for express tasks and raised after a turn-cap
            // stop. Never unbounded.
            max_turns: Some(max_turns),
            live_output: None,
            attempt_key: Some(attempt.key.attempt_key()),
        };

        // Bind the prompt treatments to the exact final prompt before launch;
        // an attempt that ends before `emit_feedback` abandons them on drop.
        let _launched_treatments = prompt_experiment::LaunchedTreatments::bind(
            prompt_experiment,
            &dispatch_plan.prompt.diagnostics.experiment_assignments,
            &request.system_prompt,
            &request.prompt,
        )
        .await;

        // ── T04: Pre-dispatch agent_spawned ─────────────────────────────
        //
        // Publish `agent_spawned` immediately so the TUI shows the agent as
        // active before the (potentially multi-minute) provider call starts.
        // `forward_dispatch_events_to_tui` no longer emits it.
        let pre_dispatch_agent_id = format!(
            "{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or(&task.id)
        );
        if let Some(tui) = &self.tui_bridge {
            // Derive a provider label from the planned backend so the dashboard
            // can display it before the actual dispatch resolves a provider.
            let planned_provider: String =
                roko_core::ProviderKind::from(dispatch_plan.model.backend)
                    .label()
                    .to_string();
            tui.agent_spawned(
                &pre_dispatch_agent_id,
                &spec.plan_id,
                &task.id,
                0,
                task.role.as_deref().unwrap_or("implementer"),
                &dispatch_plan.model.slug,
                &planned_provider,
            );
        }

        // ── Live output tap, stall watchdog and conductor ──────────────
        //
        // The agent's live output feeds the TUI, when a bridge and a
        // live-output setting are configured, the attempt's stall watchdog
        // (`[conductor] silence_timeout_secs`, `task_stall_secs`) and the
        // run's conductor. `forward_dispatch_events_to_tui` still publishes
        // the screened transcript after `run_bridge_with_failover` returns
        // (§4).
        let watched_key = attempt.key.attempt_key();
        let watched = WatchedAttempt {
            agent_id: &pre_dispatch_agent_id,
            plan_id: &spec.plan_id,
            task_id: &task.id,
            attempt_key: &watched_key,
            stop: ctx.cancel_flag.as_deref(),
        };
        let stall_watch = self.stall_watch();
        // The attempt's progress, and so the usage its call streams, is
        // tracked even with both stall thresholds off: a call that is stopped
        // or cancelled then settles what it streamed (bug-3a3b0f).
        let progress = stall_watch
            .as_ref()
            .map_or_else(AttemptProgress::default, StallWatch::progress);
        let supervised = self.supervise_attempt(&watched);
        request.live_output = self.live_output_tap(
            &watched,
            Some(progress.clone()),
            supervised.as_ref().map(SupervisedAttempt::feed),
        );

        attempt.dispatch_started();
        let started_at = Instant::now();
        // The planned model is a preference: an unusable or out-of-usage
        // provider fails over; `dispatch.target` names the model that ran.
        // While it runs, heartbeats keep the TUI's elapsed-time counter live.
        // The stall watchdog cancels an attempt that goes silent and the
        // conductor one it restarts; either then fails and retries under
        // `max_retries`, settled as what ended it (bug-4c553b).
        let watched_result = self
            .run_watched(
                self.run_bridge_with_failover(
                    spec,
                    &task.id,
                    attempt.key.attempt_key(),
                    request,
                    Some(&progress),
                ),
                &progress,
                stall_watch,
                supervised.as_ref(),
                &watched,
            )
            .await;
        let ended_by = watched_result.as_ref().err().cloned();
        let dispatch_result =
            watched_result.unwrap_or_else(|interrupted| Err(interrupted.error(&watched)));
        attempt.dispatch_ended();
        if let Some(supervised) = supervised {
            supervised
                .end(matches!(&dispatch_result, Ok((dispatch, _)) if dispatch.result.success));
        }
        let (mut dispatch, failover) = match dispatch_result {
            Ok(dispatched) => dispatched,
            Err(error) => {
                // A call that failed once the run began to stop, such as an
                // agent that exited on the run's own SIGTERM, is a
                // cancellation (bug-28b604).
                let error = match error {
                    RokoError::Cancelled(_) => error,
                    error => self
                        .stopped_call(&spec.plan_id, &task.id, &error.to_string())
                        .unwrap_or(error),
                };
                // Best-effort release on dispatch failure when worktree isolation is active.
                if let Some((provider, lease)) =
                    self.workspace_provider.as_ref().zip(lease.as_ref())
                {
                    let provider = Arc::clone(provider);
                    let lease = lease.clone();
                    tokio::spawn(async move {
                        let _ = provider
                            .release(
                                &lease,
                                roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                            )
                            .await;
                    });
                }
                // T04: Publish agent_completed on the error path so the dashboard
                // never leaves an agent stuck in the "running" state.
                if let Some(tui) = &self.tui_bridge {
                    tui.agent_completed(&pre_dispatch_agent_id, &spec.plan_id, &task.id, 0);
                }
                // A call the stall watchdog, the conductor or a stopping plan
                // run cancelled is accounted like any failed call, with the
                // usage it streamed (bug-aa2044, bug-2b1ddc).
                if let Some(interrupted) = progress.interrupted_call() {
                    let wall_duration = started_at.elapsed();
                    let (dispatch, failover) = interrupted.into_dispatch(
                        &error.to_string(),
                        u64::try_from(wall_duration.as_millis()).unwrap_or(u64::MAX),
                    );
                    let cost_usd = f64::from(dispatch.result.usage.cost_usd);
                    self.record_task_spend(&spec.plan_id, &task.id, &dispatch.result.usage);
                    if let Err(budget_error) = budget_reservation.settle(cost_usd) {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            %budget_error,
                            "could not settle a cancelled call's spend"
                        );
                    }
                    attempt.record_failover(failover);
                    let settlement = watchdog::failed_call_settlement(
                        ended_by.as_ref(),
                        &error,
                        Some(&progress),
                    );
                    let settled =
                        attempt.settle(settlement, &dispatch_plan.model.slug, Some(&dispatch));
                    self.emit_feedback(
                        spec,
                        &task,
                        &settled,
                        &dispatch,
                        wall_duration,
                        &dispatch_plan,
                        Some(routing_ctx_for_feedback),
                    )
                    .await;
                    return Err(error);
                }
                // No provider result reached the sinks that predate S01, so
                // they still see nothing; the attempt's verdict is recorded.
                let settlement =
                    watchdog::failed_call_settlement(ended_by.as_ref(), &error, Some(&progress));
                let settled = attempt.settle(settlement, &dispatch_plan.model.slug, None);
                self.publish_settlement(spec, &task, &settled).await;
                return Err(error);
            }
        };
        // The attempt's records name the planned model beside the one that
        // ran (bug-35379d).
        attempt.record_failover(failover);
        let wall_duration = started_at.elapsed();
        // The model the provider reported serving (bug-31438d). A
        // substitution is priced by the model that served, and fails a
        // `--model` pin once the call is accounted and recorded.
        let pinned_model_substituted = self.check_served_model(spec, &task.id, &mut dispatch);

        // Account for every completed provider call, including unsuccessful
        // results: callers may still have incurred the reported cost.
        self.record_task_spend(&spec.plan_id, &task.id, &dispatch.result.usage);
        if let Err(error) = budget_reservation.settle(f64::from(dispatch.result.usage.cost_usd)) {
            let routed = Some((dispatch_plan.model.slug.as_str(), &dispatch));
            return Err(self.fail_attempt(spec, &task, attempt, routed, error).await);
        }

        // ── TUI streaming output ─────────────────────────────────────────
        //
        // Forward provider dispatch events (text deltas, tool calls, tool
        // outputs) to the TUI bridge so the dashboard shows what the agent
        // produced. This runs for both successful and failed dispatches.
        self.forward_dispatch_events_to_tui(spec, &task, &dispatch, ctx);

        if let Some(error) = pinned_model_substituted {
            let settlement =
                Settlement::provider_failure(&error.to_string(), first_token_seen(&dispatch));
            let settled = attempt.settle(settlement, &dispatch_plan.model.slug, Some(&dispatch));
            self.emit_feedback(
                spec,
                &task,
                &settled,
                &dispatch,
                wall_duration,
                &dispatch_plan,
                Some(routing_ctx_for_feedback),
            )
            .await;
            if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                let _ = provider
                    .release(
                        lease,
                        roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                    )
                    .await;
            }
            return Err(error);
        }

        if !dispatch.result.success {
            let message = dispatch
                .result
                .output
                .body
                .as_text()
                .unwrap_or("provider returned an unsuccessful result")
                .to_string();
            // An agent that exited on its run's SIGTERM was cancelled, not a
            // failed provider (bug-28b604).
            let stopped = self.stopped_call(&spec.plan_id, &task.id, &message);
            // A failed provider call is settled now; a successful one is
            // settled after its verify steps so learning sees the verified
            // outcome.
            let settlement = match &stopped {
                Some(cancelled) => Settlement::provider_call_error(cancelled),
                None => Settlement::provider_failure(&message, first_token_seen(&dispatch)),
            };
            let settled = attempt.settle(settlement, &dispatch_plan.model.slug, Some(&dispatch));
            self.emit_feedback(
                spec,
                &task,
                &settled,
                &dispatch,
                wall_duration,
                &dispatch_plan,
                Some(routing_ctx_for_feedback),
            )
            .await;
            // Release worktree with RetainForFailure policy for post-mortem.
            if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
                let _ = provider
                    .release(
                        lease,
                        roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                    )
                    .await;
            }
            if let Some(cancelled) = stopped {
                return Err(cancelled);
            }

            // Detect billing/credit errors and log a clear warning so
            // operators see the root cause. The in-memory health registry
            // has already been updated by the dispatch layer (dispatch_v2),
            // which marks the provider as Open-circuit with a 24-hour
            // cooldown, so the next retry will route to a different provider
            // through the cascade router.
            let message_lower = message.to_ascii_lowercase();
            if roko_agent::provider::error_classify::is_billing_message(&message_lower) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    provider = %dispatch.target.provider_id,
                    "provider {} returned billing error: {}. \
                     Marking as unavailable for this run.",
                    dispatch.target.provider_id,
                    message
                );
            }

            if let Some(hit) = roko_agent::provider::error_classify::detect_turn_cap(&message) {
                self.keep_turn_cap_retry(
                    &spec.plan_id,
                    &task.id,
                    TurnCapRetry {
                        cap: max_turns,
                        num_turns: hit.num_turns,
                    },
                );
                return Err(RokoError::TurnLimitReached {
                    backend: dispatch.target.provider_id,
                    limit: max_turns,
                    num_turns: hit.num_turns.unwrap_or(max_turns),
                });
            }
            if roko_agent::provider::error_classify::detect_attempt_timeout(&message) {
                self.timeout_retries
                    .lock()
                    .insert(task_spend_key.clone(), timeout_ms);
            }
            return Err(RokoError::Agent {
                backend: dispatch.target.provider_id,
                message,
            });
        }

        // ── Verify steps (gate execution) ──────────────────────────────
        //
        // Authored [[task.verify]] steps gate the task in the effective
        // workdir (worktree if isolated). A failure fails this attempt so the
        // Graph engine can retry or abort; it is never force-accepted.
        let attempt_key = attempt.key.attempt_key();
        let helper_calls = HelperCalls::default();
        let verification = helper_calls
            .scope(self.settle_task_verification(
                spec,
                &task,
                &dispatch,
                &effective_workdir,
                &retry_key,
                attempt_number,
                &attempt_key,
                None,
            ))
            .await;
        // The helper model calls verification made count toward this
        // attempt, the background ones included (bug-62e3f4).
        attempt.record_helper_calls(
            self.settle_helper_calls(spec, &task, &attempt_key, &helper_calls)
                .await,
        );

        // ── Learning/feedback pipeline ───────────────────────────────────
        //
        // Settled after the gate so episodes, routing, playbooks, affect, and
        // experiments learn from the verified outcome rather than from the
        // provider dispatch result.
        let settlement = Settlement::verified(&verification);
        let settled = attempt.settle(settlement, &dispatch_plan.model.slug, Some(&dispatch));
        self.emit_feedback(
            spec,
            &task,
            &settled,
            &dispatch,
            wall_duration,
            &dispatch_plan,
            Some(routing_ctx_for_feedback),
        )
        .await;

        let verdict = match verification {
            Ok(verdict) => verdict,
            Err(error) => {
                // Release worktree with RetainForFailure for post-mortem.
                if let Some((provider, lease)) =
                    self.workspace_provider.as_ref().zip(lease.as_ref())
                {
                    let _ = provider
                        .release(
                            lease,
                            roko_graph::workspace::WorkspaceReleasePolicy::RetainForFailure,
                        )
                        .await;
                }
                return Err(error);
            }
        };

        // ── Worktree isolation: hand on, or accept on success ───────────
        //
        // When a later cell of the task judges this checkout (the rich
        // topology's `plan.gate`), it must outlive the dispatch: the lease
        // travels on the output, and that cell accepts or keeps it.
        // Otherwise the attempt is accepted onto its plan branch now, as its
        // settled verdict allows (gap-3b5361).
        let mut handed_on = None;
        let mut accepted = None;
        if let Some((provider, lease)) = self.workspace_provider.as_ref().zip(lease.as_ref()) {
            if spec.keep_workspace {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    worktree = %lease.path.display(),
                    "handing the attempt's worktree on to the task's gate"
                );
                handed_on = Some(lease.clone());
            } else {
                accepted = self
                    .accept_attempt(
                        spec,
                        &task,
                        &settled,
                        verdict,
                        provider.as_ref(),
                        lease,
                        ctx,
                    )
                    .await?;
            }
        }

        let mut output = dispatch.result.output;
        if output.body.as_text().is_err() {
            output = Signal::builder(Kind::AgentOutput)
                .body(Body::text(format!(
                    "provider `{}` completed task `{}`",
                    dispatch.target.provider_id, spec.title
                )))
                .build();
        }
        let mut outputs = vec![output];
        // The output names the exact attempt and checkout that produced it,
        // so the task's later cells act on them (bug-50caf2).
        let key = settled.key();
        TaskAttempt {
            plan_id: spec.plan_id.clone(),
            task_id: task.id.clone(),
            run_id: Some(key.run_id),
            attempt_key: Some(settled.attempt_key().to_string()),
            attempt: key.attempt,
            workspace: lease.as_ref().map(|lease| lease.path.clone()),
            lease: handed_on,
            accepted,
        }
        .stamp(&mut outputs);
        verdict.stamp(&mut outputs);
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn graph_task_cell_reaches_real_provider_runtime() {
        let temp = tempdir().expect("tempdir");
        let script = temp.path().join("fake-claude.sh");
        std::fs::write(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"graph-live-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-1","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":11,"output_tokens":22}}'
"#,
        )
        .expect("write provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make script executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "graph-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "graph-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "graph-model".to_string(),
            ModelProfile {
                provider: "graph-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_plan_budget(0.50, 0.25, false),
        );

        let task = TaskDef {
            id: "T01".to_string(),
            title: "Execute a real graph task".to_string(),
            description: Some("Return live output".to_string()),
            role: Some("implementer".to_string()),
            status: "ready".to_string(),
            tier: "focused".to_string(),
            frequency: None,
            model_hint: Some("graph-model".to_string()),
            replan_strategy: None,
            max_loc: None,
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            accept: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            hints: Default::default(),
        };
        let config = toml::Value::Table(toml::map::Map::from_iter([
            ("plan_id".to_string(), toml::Value::String("p1".to_string())),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            (
                "timeout_secs".to_string(),
                toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
            ),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        let cell = roko_graph::cells::TaskExecutorCell::live(config, dispatcher.clone());
        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01".to_string()),
            )
            .await
            .expect("live graph task dispatch");

        assert_eq!(
            output[0].body.as_text().expect("provider text"),
            "graph-live-output"
        );
        assert!(
            !output[0]
                .body
                .as_text()
                .expect("provider text")
                .contains("dry-run")
        );

        let budget = dispatcher.plan_budget_snapshot("p1");
        assert!((budget.spent_usd - 0.25).abs() < 0.000_001);
        assert!(!budget.exhausted);
        assert!(!budget.dispatch_blocked);

        std::fs::write(
            &script,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"paid-provider-failure"}}'
printf '%s\n' '{"type":"result","session_id":"sess-2","model":"claude-sonnet-4-6","total_cost_usd":0.25,"usage":{"input_tokens":7,"output_tokens":3},"is_error":true}'
exit 1
"#,
        )
        .expect("replace provider script with paid failure");

        let paid_failure = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01-paid-failure".to_string()),
            )
            .await
            .expect_err("unsuccessful paid provider result must fail the task");
        assert!(matches!(paid_failure, RokoError::Agent { .. }));

        let budget = dispatcher.plan_budget_snapshot("p1");
        assert!((budget.spent_usd - 0.50).abs() < 0.000_001);
        assert!(budget.exhausted);
        assert!(budget.dispatch_blocked);

        let blocked = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T01-retry".to_string()),
            )
            .await
            .expect_err("later dispatch must fail closed after plan budget exhaustion");
        assert!(matches!(blocked, RokoError::BudgetExceeded { .. }));
    }

    /// gap-5d3b82 (proof case 1): an agent that exits before its first event
    /// fails each attempt at once rather than at the task's timeout. The
    /// task is retried `max_retries` times and then fails, naming the exit;
    /// each attempt leaves a failed verdict; and its agent slot is freed for
    /// the next task.
    #[tokio::test]
    async fn agent_exit_before_first_event_fails_the_attempt_promptly() {
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let crash = temp.path().join("crash-on-start");
        let script = temp.path().join("fake-claude.sh");
        std::fs::write(
            &script,
            format!(
                r#"#!/bin/sh
cat >/dev/null
if [ -e '{crash}' ]; then
  echo crashed >> '{calls}'
  echo 'claude: error: unknown option --bogus' >&2
  exit 1
fi
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}}'
"#,
                crash = crash.display(),
                calls = calls.display()
            ),
        )
        .expect("write provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make script executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "graph-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "graph-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "graph-model".to_string(),
            ModelProfile {
                provider: "graph-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        // The attempt log only: no episodes, whose background distillation
        // would call the fake CLI too.
        let feedback = GraphFeedbackContext {
            runs_dir: Some(temp.path().join(".roko/runs")),
            ..GraphFeedbackContext::default()
        };
        let dispatcher = Arc::new(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_feedback(feedback),
        );
        // `[conductor] max_agents = 1`: the next task runs only once the
        // failed one freed its slot.
        let slots: Arc<dyn TaskDispatcher> =
            Arc::new(crate::graph_execution::agent_slots::AgentSlotDispatcher::new(dispatcher, 1));
        let cell = |id: &str, max_retries: u32| {
            let task = TaskDef {
                id: id.to_string(),
                title: format!("Task {id}"),
                model_hint: Some("graph-model".to_string()),
                timeout_secs: FIXTURE_HANG_GUARD_SECS,
                max_retries,
                ..make_task_def("focused")
            };
            let config = toml::Value::Table(toml::map::Map::from_iter([
                ("plan_id".to_string(), toml::Value::String("p1".to_string())),
                ("title".to_string(), toml::Value::String(task.title.clone())),
                (
                    "timeout_secs".to_string(),
                    toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
                ),
                (
                    "max_retries".to_string(),
                    toml::Value::Integer(i64::from(max_retries)),
                ),
                (
                    "task_def_json".to_string(),
                    toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
                ),
            ]));
            roko_graph::cells::TaskExecutorCell::live(config, Arc::clone(&slots))
        };
        let ctx = |id: &str| {
            CellContext::new()
                .with_cell_id(id.to_string())
                .with_run_id("early-exit".to_string())
        };

        std::fs::write(&crash, "").expect("arm the crash");
        let started = Instant::now();
        let error = cell("T01", 1)
            .execute(Vec::new(), &ctx("T01"))
            .await
            .expect_err("an agent that exits on start fails its task");
        let elapsed = started.elapsed();
        assert!(error.to_string().contains("exit 1"), "{error}");
        assert!(error.to_string().contains("unknown option"), "{error}");
        assert!(
            elapsed < std::time::Duration::from_secs(FIXTURE_HANG_GUARD_SECS / 2),
            "both attempts failed after {elapsed:?}, not at the task's timeout"
        );
        let crashed = std::fs::read_to_string(&calls).unwrap_or_default();
        assert_eq!(
            crashed.lines().count(),
            2,
            "max_retries = 1 runs the agent twice"
        );
        let verdicts = jsonl_rows_where(
            &temp.path().join(".roko/runs/early-exit/attempts.jsonl"),
            2,
            |row| row["schema_version"] == "roko.verdict/1" && row["task_id"] == "T01",
        )
        .await;
        assert!(
            verdicts
                .iter()
                .all(|verdict| verdict["outcome"] != "passed"),
            "{verdicts:?}"
        );

        std::fs::remove_file(&crash).expect("disarm the crash");
        let next = cell("T02", 0);
        tokio::time::timeout(
            std::time::Duration::from_secs(FIXTURE_HANG_GUARD_SECS),
            next.execute(Vec::new(), &ctx("T02")),
        )
        .await
        .expect("the failed task freed its agent slot")
        .expect("the next task runs");
    }

    pub(super) const VERIFY_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"verify-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-v1","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// Time limit, in seconds, of a fixture's attempts and verify steps,
    /// which finish at once. It only guards against a hang: a loaded machine
    /// can hold up a fake provider or a step for seconds (bug-779ae7).
    pub(super) const FIXTURE_HANG_GUARD_SECS: u64 = 120;

    /// Time limit, in ms, of the fixtures' providers, the fake CLIs and the
    /// mock servers. A loaded machine can hold up a fake CLI's start or a
    /// mock's answer for seconds (bug-f1f814), so this is the hang guard too.
    /// A test of time limits sets its attempt's own limit, which is shorter
    /// and stops the provider first.
    pub(super) const FIXTURE_PROVIDER_TIMEOUT_MS: u64 = FIXTURE_HANG_GUARD_SECS * 1_000;

    pub(super) fn verify_step(phase: &str, command: &str) -> crate::task_parser::VerifyStep {
        crate::task_parser::VerifyStep {
            phase: phase.to_string(),
            command: command.to_string(),
            fail_msg: None,
            timeout_ms: FIXTURE_HANG_GUARD_SECS * 1_000,
            scope: Vec::new(),
        }
    }

    pub(super) fn no_auto_fix(config: &mut RokoConfig) {
        config.gates.cargo_fix_enabled = false;
    }

    /// Like [`make_streaming_dispatcher`], with a config tweak and feedback.
    pub(super) async fn make_test_dispatcher(
        temp: &tempfile::TempDir,
        script_content: &str,
        configure: impl FnOnce(&mut RokoConfig),
        feedback: GraphFeedbackContext,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        make_test_dispatcher_with(temp, script_content, configure, feedback, |dispatcher| {
            dispatcher
        })
        .await
    }

    /// Like [`make_test_dispatcher`], finishing the dispatcher with `finish`.
    pub(super) async fn make_test_dispatcher_with(
        temp: &tempfile::TempDir,
        script_content: &str,
        configure: impl FnOnce(&mut RokoConfig),
        feedback: GraphFeedbackContext,
        finish: impl FnOnce(GraphTaskDispatcher) -> GraphTaskDispatcher,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        let script = temp.path().join("fake-claude-stream.sh");
        std::fs::write(&script, script_content).expect("write stream provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "stream-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "stream-cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(script.display().to_string()),
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "stream-model".to_string(),
            ModelProfile {
                provider: "stream-cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        configure(&mut config);
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(finish(
            GraphTaskDispatcher::new(factory, Arc::clone(&config), temp.path().to_path_buf())
                .with_plan_budget(1.00, 0.50, false)
                .with_feedback(feedback),
        ));

        let task = TaskDef {
            id: "T-STREAM".to_string(),
            title: "Streaming graph task".to_string(),
            description: Some("Test streaming dispatch".to_string()),
            role: Some("implementer".to_string()),
            status: "ready".to_string(),
            tier: "focused".to_string(),
            frequency: None,
            model_hint: Some("stream-model".to_string()),
            replan_strategy: None,
            max_loc: None,
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            accept: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            hints: Default::default(),
        };

        (dispatcher, task)
    }

    pub(super) fn make_spec(task: &TaskDef) -> TaskExecutionSpec {
        TaskExecutionSpec {
            plan_id: "stream-plan".to_string(),
            plan_dir: "/tmp/plans/stream-plan".to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            role: task.role.clone(),
            tier: task.tier.clone(),
            model_hint: task.model_hint.clone(),
            files: task.files.clone(),
            timeout_secs: task.timeout_secs,
            max_retries: task.max_retries,
            task_def_json: serde_json::to_string(task).expect("serialize task"),
            keep_workspace: false,
        }
    }

    pub(super) fn make_task_def(tier: &str) -> TaskDef {
        TaskDef {
            id: "T-EXP".to_string(),
            title: "express test task".to_string(),
            description: None,
            role: Some("implementer".to_string()),
            status: "pending".to_string(),
            tier: tier.to_string(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: Some(20),
            files: Vec::new(),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify: Vec::new(),
            timeout_secs: 0,
            max_retries: 0,
            acceptance: Vec::new(),
            acceptance_contract: None,
            accept: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            hints: Default::default(),
        }
    }

    // ─── Dispatch overhead and config wiring ─────────────────────────────────

    /// A dispatcher over `config` whose factory owns no live provider.
    pub(super) async fn make_bare_dispatcher(
        config: RokoConfig,
        workdir: &Path,
    ) -> GraphTaskDispatcher {
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        GraphTaskDispatcher::new(factory, config, workdir.to_path_buf())
    }

    pub(super) fn cli_provider(command: &str) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            api_key_env: None,
            command: Some(command.to_string()),
            args: None,
            timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        }
    }

    pub(super) fn model(provider: &str, slug: &str, prices: Option<(f64, f64)>) -> ModelProfile {
        ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            supports_tools: true,
            cost_input_per_m: prices.map(|(input, _)| input),
            cost_output_per_m: prices.map(|(_, output)| output),
            ..ModelProfile::default()
        }
    }

    /// Batch-path fixture: a fake Claude CLI reporting `cost_usd` per call.
    pub(super) async fn make_batch_dispatcher(
        temp: &tempfile::TempDir,
        cost_usd: f64,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        let script = format!(
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' "$*" >> "$(dirname -- "$0")/provider-args"
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"batch-output"}}}}'
printf '%s\n' '{{"type":"result","session_id":"sess-b","model":"claude-sonnet-4-6","total_cost_usd":{cost_usd},"usage":{{"input_tokens":5,"output_tokens":10}}}}'
"#
        );
        make_scripted_batch_dispatcher(temp, &script, configure).await
    }

    /// [`make_batch_dispatcher`] with a caller-supplied fake Claude CLI.
    pub(super) async fn make_scripted_batch_dispatcher(
        temp: &tempfile::TempDir,
        script_body: &str,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (GraphTaskDispatcher, TaskDef) {
        let script = temp.path().join("fake-claude-batch.sh");
        std::fs::write(&script, script_body).expect("write batch provider script");
        let mut permissions = std::fs::metadata(&script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("make executable");

        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "batch-model".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "batch-cli".to_string(),
            cli_provider(&script.display().to_string()),
        );
        config.models.insert(
            "batch-model".to_string(),
            model("batch-cli", "claude-sonnet-4-6", None),
        );
        configure(&mut config);

        let mut task = make_task_def("focused");
        task.title = "Wire the batch fixture".to_string();
        task.model_hint = Some("batch-model".to_string());
        task.timeout_secs = FIXTURE_HANG_GUARD_SECS;
        (make_bare_dispatcher(config, temp.path()).await, task)
    }

    pub(super) fn batch_ctx() -> CellContext {
        CellContext::new().with_cell_id("T-EXP".to_string())
    }

    /// bug-a843d4: a task whose role is disabled fails without a dispatch,
    /// instead of completing with its verify steps unrun.
    #[tokio::test]
    async fn disabled_role_task_fails_without_dispatch() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_batch_dispatcher(&temp, 0.01, |config| {
            let disabled = roko_core::config::schema::RoleOverride {
                enabled: false,
                ..Default::default()
            };
            config
                .agent
                .roles
                .insert("implementer".to_string(), disabled);
        })
        .await;
        task.verify = vec![verify_step("structural", "true")];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect_err("a disabled role fails its task");

        assert!(matches!(error, RokoError::Rejected(_)), "{error}");
        assert!(
            error.to_string().contains("role `implementer` is disabled"),
            "{error}"
        );
        assert!(
            !temp.path().join("provider-args").exists(),
            "the provider ran"
        );
    }

    /// gap-c09fc7: a plan's agents hear of the run's other running plans, not
    /// of their own plan or of one that finished.
    #[tokio::test]
    async fn concurrent_plans_name_the_other_running_plans() {
        let temp = tempdir().expect("tempdir");
        let dispatcher = make_bare_dispatcher(RokoConfig::default(), temp.path()).await;
        dispatcher.plan_started("api", vec!["crates/roko-serve".to_string()]);
        dispatcher.plan_started("web", vec!["web/src".to_string()]);
        dispatcher.plan_started("docs", Vec::new());
        dispatcher.plan_finished("docs");

        assert_eq!(
            dispatcher.concurrent_plans("api"),
            [("web".to_string(), vec!["web/src".to_string()])]
        );
    }

    /// bug-017c2d: no placeholder test is written, and nothing reaches the
    /// repo root. With `write_eval_artifacts` on, only checked evaluations
    /// are written; the built-in template needs a property body that the
    /// fixture task does not author, so none is.
    #[tokio::test]
    async fn eval_artifacts_never_hold_placeholder_tests_or_reach_the_repo_root() {
        for write_eval_artifacts in [false, true] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
                config.gates.write_eval_artifacts = write_eval_artifacts;
            })
            .await;
            let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
                eval_generation_enabled: true,
                ..GraphFeedbackContext::default()
            });
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
                .await
                .expect("dispatch");

            assert!(!temp.path().join("generated-tests").exists());
            let written = std::fs::read_dir(temp.path().join(".roko/generated-tests"))
                .map(|entries| entries.count())
                .unwrap_or(0);
            assert_eq!(
                written, 0,
                "write_eval_artifacts={write_eval_artifacts} wrote {written} artifacts"
            );
        }
    }

    #[tokio::test]
    async fn skip_enrichment_plan_meta_is_read_and_suppresses_eval_artifacts() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            config.gates.write_eval_artifacts = true;
        })
        .await;
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            eval_generation_enabled: true,
            ..GraphFeedbackContext::default()
        });
        let plan_dir = temp.path().join("plans/authored");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"authored\"\nskip_enrichment = true\n\n\
             [[task]]\nid = \"T-EXP\"\ntitle = \"Wire the batch fixture\"\n",
        )
        .expect("write tasks.toml");
        let mut spec = make_spec(&task);
        spec.plan_id = "authored".to_string();
        spec.plan_dir = plan_dir.display().to_string();

        assert!(dispatcher.plan_skips_enrichment(&spec));
        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("dispatch");
        assert!(!temp.path().join(".roko/generated-tests").exists());

        let mut unflagged = make_spec(&task);
        unflagged.plan_id = "unflagged".to_string();
        unflagged.plan_dir = temp.path().join("plans/missing").display().to_string();
        assert!(!dispatcher.plan_skips_enrichment(&unflagged));
    }

    /// A T0 reflex rule that matches a task is never credited with a gate pass
    /// when it fires: no gate has run. With `[learning] t0_reflexes` off (the
    /// default) no rule is consulted. With it on, a task whose verify step
    /// fails is still dispatched and gated, and a task without verify steps
    /// gets the rule's output, stamped unverified.
    #[tokio::test]
    async fn reflex_match_records_no_gate_pass_before_verify() {
        use roko_learn::reflex_store::{PromotionCandidate, ReflexAction, ReflexCondition};

        assert!(!RokoConfig::default().learning.t0_reflexes);
        let store_dir = tempdir().expect("tempdir");
        let reflexes = ReflexStore::open(store_dir.path().join("reflexes.jsonl"));
        // A wildcard rule matches every task; promotion credits it with three
        // gate passes out of three hits.
        let promoted = reflexes.try_promote(
            &PromotionCandidate {
                episode_id: "episode-reflex".to_string(),
                condition: ReflexCondition::default(),
                action: ReflexAction {
                    tool: "respond".to_string(),
                    args: "cached reflex output".to_string(),
                },
            },
            3,
        );
        assert!(promoted);
        let counts = || {
            let rule = reflexes.snapshot().pop().expect("the promoted rule");
            (rule.hit_count, rule.success_count)
        };

        for t0_reflexes in [false, true] {
            let temp = tempdir().expect("tempdir");
            let (dispatcher, mut task) = make_batch_dispatcher(&temp, 0.01, |config| {
                no_auto_fix(config);
                config.learning.t0_reflexes = t0_reflexes;
            })
            .await;
            let dispatcher = dispatcher.with_reflex_store(reflexes.clone());

            // A matching rule whose task then fails its verify step.
            task.verify = vec![verify_step("structural", "exit 1")];
            let error = dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
                .await
                .expect_err("the failing verify step fails the attempt");
            assert!(matches!(error, RokoError::Verify { .. }), "{error}");
            assert_eq!(counts(), (3, 3), "t0_reflexes = {t0_reflexes}");

            // The same task without verify steps.
            task.verify.clear();
            let outputs = dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
                .await
                .expect("dispatch without verify steps");
            assert_eq!(
                TaskGateVerdict::from_signals(&outputs),
                Some(TaskGateVerdict::Unverified)
            );
            let text = outputs[0].body.as_text().expect("output text");
            if t0_reflexes {
                assert_eq!(text, "cached reflex output");
                assert_eq!(counts(), (4, 3), "the rule fired but earned no gate pass");
            } else {
                assert_eq!(text, "batch-output");
                assert_eq!(counts(), (3, 3), "no rule is consulted by default");
            }
        }
    }

    /// Feedback sink keeping `(iteration, has_prior_failure)` of each settled
    /// attempt's routing context, in order.
    #[derive(Debug, Default)]
    pub(super) struct RoutingContextLog(parking_lot::Mutex<Vec<(u32, bool)>>);

    impl RoutingContextLog {
        /// Feedback that records into `log`.
        pub(super) fn feedback(log: &Arc<Self>) -> GraphFeedbackContext {
            GraphFeedbackContext {
                feedback_facade: Some(Arc::new(
                    crate::runtime_feedback::FeedbackFacade::new().with_sink(log.clone()),
                )),
                ..GraphFeedbackContext::default()
            }
        }

        /// What was recorded so far.
        pub(super) fn marks(&self) -> Vec<(u32, bool)> {
            self.0.lock().clone()
        }
    }

    #[async_trait::async_trait]
    impl crate::runtime_feedback::FeedbackSink for RoutingContextLog {
        fn name(&self) -> &'static str {
            "routing-contexts"
        }

        async fn on_event(&self, event: &FeedbackEvent) -> anyhow::Result<()> {
            if let FeedbackEvent::TaskCompleted {
                routing_context: Some(routing),
                ..
            } = event
            {
                self.0
                    .lock()
                    .push((routing.iteration, routing.has_prior_failure));
            }
            Ok(())
        }
    }

    /// gap-b62e95: the retry of a task whose verify step failed routes, and
    /// is recorded, as a retry after a failure; its first attempt is not.
    #[tokio::test]
    async fn routing_context_marks_retry_after_failure() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_batch_dispatcher(&temp, 0.01, no_auto_fix).await;
        let contexts = Arc::new(RoutingContextLog::default());
        let dispatcher = dispatcher.with_feedback(RoutingContextLog::feedback(&contexts));
        // The verify step fails once, then passes.
        task.verify = vec![verify_step(
            "structural",
            "test -f retried || { touch retried; exit 1; }",
        )];
        let spec = make_spec(&task);
        let error = dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect_err("the first attempt fails its verify step");
        assert!(matches!(error, RokoError::Verify { .. }), "{error}");
        dispatcher
            .dispatch(&spec, Vec::new(), &batch_ctx())
            .await
            .expect("the retry passes");

        assert_eq!(contexts.marks(), [(0, false), (1, true)]);
    }

    /// Every record file a Graph attempt writes, under `workdir/.roko`.
    pub(super) fn recording_feedback(workdir: &Path) -> GraphFeedbackContext {
        let roko = workdir.join(".roko");
        let facade = crate::runtime_feedback::FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::EpisodeSink::at(roko.join("episodes.jsonl")),
        ));
        GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            efficiency_path: Some(roko.join("learn/efficiency.jsonl")),
            costs_path: Some(roko.join("learn/costs.jsonl")),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        }
    }

    /// The rows of the JSONL file at `path` that `keep` accepts, once at
    /// least `expected` of them landed from the background writers.
    pub(super) async fn jsonl_rows_where(
        path: &Path,
        expected: usize,
        keep: impl Fn(&serde_json::Value) -> bool,
    ) -> Vec<serde_json::Value> {
        crate::background_writes::settled(path.parent().unwrap_or(path)).await;
        for _ in 0..600 {
            let rows: Vec<serde_json::Value> = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .filter(|row| keep(row))
                .collect();
            if rows.len() >= expected {
                return rows;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("{expected} rows were not written to {}", path.display());
    }

    /// Serve canned OpenAI-compatible chat responses, one per connection,
    /// capturing each request body.
    pub(super) fn spawn_openai_mock(
        responses: Vec<serde_json::Value>,
    ) -> (String, Arc<parking_lot::Mutex<Vec<serde_json::Value>>>) {
        use std::io::Write;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}/v1", listener.local_addr().expect("mock addr"));
        let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let requests = Arc::clone(&captured);
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let Some(request) = read_mock_request(&mut stream) else {
                    return;
                };
                // A request that asks to stream gets the same answer as SSE
                // chunks: an attempt streams whenever its progress is tracked
                // (bug-3a3b0f), with or without the stall watchdog.
                let streamed = request["stream"] == serde_json::Value::Bool(true);
                requests.lock().push(request);
                let wire = if streamed {
                    let mut body = String::new();
                    for chunk in openai_stream_chunks(&response) {
                        body.push_str(&format!("data: {chunk}\n\n"));
                    }
                    body.push_str("data: [DONE]\n\n");
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}"
                    )
                } else {
                    let body = response.to_string();
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                let _ = stream.write_all(wire.as_bytes());
            }
        });
        (base_url, captured)
    }

    /// A chat-completion `response` as the SSE chunks a streaming provider
    /// sends for it: the message's content and tool calls as one delta, then
    /// its finish reason, then its usage.
    fn openai_stream_chunks(response: &serde_json::Value) -> Vec<serde_json::Value> {
        let choice = &response["choices"][0];
        let message = &choice["message"];
        let mut delta = serde_json::json!({ "role": "assistant" });
        let calls = message["tool_calls"]
            .as_array()
            .filter(|calls| !calls.is_empty());
        if let Some(content) = message["content"]
            .as_str()
            .filter(|content| !content.is_empty())
        {
            delta["content"] = serde_json::json!(content);
        }
        if let Some(calls) = calls {
            let calls: Vec<serde_json::Value> = calls
                .iter()
                .enumerate()
                .map(|(index, call)| {
                    serde_json::json!({
                        "index": index,
                        "id": call["id"],
                        "function": call["function"],
                    })
                })
                .collect();
            delta["tool_calls"] = serde_json::Value::Array(calls);
        }
        let chunk = |choices: serde_json::Value| {
            let mut chunk = serde_json::json!({ "id": response["id"], "choices": choices });
            if let Some(model) = response.get("model") {
                chunk["model"] = model.clone();
            }
            chunk
        };
        let mut chunks = vec![
            chunk(serde_json::json!([{ "index": 0, "delta": delta }])),
            chunk(serde_json::json!([{
                "index": 0,
                "delta": {},
                "finish_reason": choice["finish_reason"]
            }])),
        ];
        if let Some(usage) = response.get("usage") {
            let mut last = chunk(serde_json::json!([]));
            last["usage"] = usage.clone();
            chunks.push(last);
        }
        chunks
    }

    /// [`spawn_openai_mock`] answering over SSE: each request gets the next
    /// of `streams`, one `data:` line per chunk and then `[DONE]`.
    pub(super) fn spawn_openai_stream_mock(
        streams: Vec<Vec<serde_json::Value>>,
    ) -> (String, Arc<parking_lot::Mutex<Vec<serde_json::Value>>>) {
        use std::io::Write;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let base_url = format!("http://{}/v1", listener.local_addr().expect("mock addr"));
        let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let requests = Arc::clone(&captured);
        std::thread::spawn(move || {
            for chunks in streams {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let Some(request) = read_mock_request(&mut stream) else {
                    return;
                };
                requests.lock().push(request);
                let mut body = String::new();
                for chunk in chunks {
                    body.push_str(&format!("data: {chunk}\n\n"));
                }
                body.push_str("data: [DONE]\n\n");
                let wire = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}"
                );
                let _ = stream.write_all(wire.as_bytes());
            }
        });
        (base_url, captured)
    }

    /// The JSON body of the HTTP request on `stream`; `None` when the
    /// client hung up before its headers.
    fn read_mock_request(stream: &mut std::net::TcpStream) -> Option<serde_json::Value> {
        use std::io::Read;

        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 8192];
        let body_start = loop {
            let n = stream.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let headers = String::from_utf8_lossy(&buf[..body_start]).to_ascii_lowercase();
        let length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        while buf.len() < body_start + length {
            let n = stream.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let end = buf.len().min(body_start + length);
        Some(serde_json::from_slice(&buf[body_start..end]).unwrap_or(serde_json::Value::Null))
    }

    pub(super) fn tool_call_turn(
        id: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": format!("chatcmpl-{id}"),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": id,
                        "type": "function",
                        "function": { "name": name, "arguments": arguments.to_string() }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        })
    }

    pub(super) fn final_turn(text: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-final",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": text },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15 }
        })
    }

    /// Attempt time limits, in seconds, for a test whose scripted provider
    /// must reach some point before its attempt runs out of time. The test
    /// tries the next limit when the provider did not get there, as on a
    /// loaded machine, instead of failing on a clock (bug-779ae7).
    pub(super) const TIMEOUT_SECS_UNDER_LOAD: [u64; 5] = [1, 2, 4, 8, 16];

    /// A fake Claude CLI that streams one API message, then works past its
    /// timeout without reaching its `result` event. It records each prompt
    /// as `prompt-<call>` once it has read all of it.
    pub(super) const STREAMS_THEN_TIMES_OUT_PROVIDER: &str = r#"#!/bin/sh
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/calls"
cat > "$dir/prompt-$n.part" && mv "$dir/prompt-$n.part" "$dir/prompt-$n"
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"cargo build"}}],"usage":{"input_tokens":1000,"output_tokens":200}},"parent_tool_use_id":null}'
sleep 30
"#;

    /// find-f489db: a Graph run attaches its tool observability to the agent
    /// factory. A tool call an API model makes then leaves a scrubbed admit
    /// and result pair in `.roko/tool_audit.jsonl` that names the attempt's
    /// run and task, a closed trace under `.roko/traces/` and a metrics
    /// record.
    #[tokio::test]
    async fn graph_run_writes_tool_audit_admit_and_result() {
        // A GitHub token, which the scrubber's built-in patterns catch.
        const SECRET: &str = "ghp_f489dbAuditCanary0123456789abcdefghi";
        assert_eq!(SECRET.len(), 40, "ghp_ and 36 characters");
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().to_path_buf();
        std::fs::write(workdir.join("notes.txt"), format!("notes {SECRET}\n")).expect("seed notes");
        let (base_url, _requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "call-read",
                "read_file",
                serde_json::json!({ "path": "notes.txt" }),
            ),
            final_turn("read the notes"),
        ]);
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "api-model".to_string();
        config.agent.bare_mode = false;
        // `PATH` is always set, standing in for an API key.
        config.providers.insert(
            "mock_api".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some(base_url),
                api_key_env: Some("PATH".to_string()),
                command: None,
                args: None,
                timeout_ms: Some(15_000),
                ttft_timeout_ms: Some(15_000),
                connect_timeout_ms: Some(5_000),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "api-model".to_string(),
            ModelProfile {
                provider: "mock_api".to_string(),
                slug: "api-model-1".to_string(),
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                supports_tools: true,
                tool_format: "openai_json".to_string(),
                ..ModelProfile::default()
            },
        );
        // The mock answers without SSE: keep the stall watchdog, which would
        // stream over live output, off.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        let config = Arc::new(config);
        let factory = SharedAgentFactory::new(Arc::clone(&config), None, None, None).await;
        let factory = Arc::new(
            crate::graph_execution::plan_runner::attach_tool_observability(factory, &workdir).await,
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            workdir.clone(),
        ));
        let task = TaskDef {
            id: "T01".to_string(),
            title: "Read the notes".to_string(),
            model_hint: Some("api-model".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let cell_config = toml::Value::Table(toml::map::Map::from_iter([
            (
                "plan_id".to_string(),
                toml::Value::String("p-audit".to_string()),
            ),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            (
                "timeout_secs".to_string(),
                toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
            ),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        let cell = roko_graph::cells::TaskExecutorCell::live(cell_config, dispatcher);
        cell.execute(
            Vec::new(),
            &CellContext::new().with_cell_id("T01".to_string()),
        )
        .await
        .expect("the task completes");

        let roko_dir = workdir.join(".roko");
        let audit =
            std::fs::read_to_string(roko_dir.join("tool_audit.jsonl")).expect("tool audit log");
        assert!(!audit.contains(SECRET), "{audit}");
        let lines: Vec<serde_json::Value> = audit
            .lines()
            .map(|line| serde_json::from_str(line).expect("audit line is JSON"))
            .collect();
        let kinds: Vec<&str> = lines
            .iter()
            .map(|line| line["kind"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(kinds, ["admit", "result"], "{audit}");
        for line in &lines {
            assert_eq!(line["call_id"], "call-read", "{line}");
            assert_eq!(line["call_name"], "read_file", "{line}");
            let correlation = &line["correlation"];
            assert_eq!(correlation["task_id"], "T01", "{line}");
            let run_id = correlation["run_id"].as_str().unwrap_or_default();
            assert!(!run_id.is_empty(), "{line}");
            assert_eq!(
                correlation["attempt_id"],
                format!("{run_id}:p-audit:T01:1"),
                "{line}"
            );
        }

        // The call's trace is closed with its handler time and outcome, and
        // its metrics sample is keyed on the model.
        let traces: Vec<String> = std::fs::read_dir(roko_dir.join("traces"))
            .expect("trace directory")
            .flatten()
            .flat_map(|day| std::fs::read_dir(day.path()).expect("trace day").flatten())
            .map(|file| std::fs::read_to_string(file.path()).expect("trace file"))
            .collect();
        assert_eq!(traces.len(), 1, "{traces:#?}");
        assert!(
            traces[0].contains("handler_finished") && traces[0].contains("\"outcome\""),
            "{traces:#?}"
        );
        let metrics = std::fs::read_to_string(roko_dir.join("metrics").join("tool_metrics.jsonl"))
            .expect("tool metrics");
        assert_eq!(metrics.lines().count(), 1, "{metrics}");
        assert!(
            metrics.contains("read_file") && metrics.contains("api-model-1"),
            "{metrics}"
        );
    }

    /// bug-28b604: once the plan run began to stop, an agent that exits on
    /// its SIGTERM within the drain settles as cancelled, not as a provider
    /// failure, and fails with a cancellation, which the task executor does
    /// not retry.
    #[tokio::test]
    async fn a_sigterm_exit_during_the_drain_settles_as_cancelled() {
        let temp = tempdir().expect("tempdir");
        let script = "#!/bin/sh\ncat >/dev/null\nkill -TERM $$\n";
        let (dispatcher, task) = make_scripted_batch_dispatcher(&temp, script, |_| {}).await;
        let runs = temp.path().join(".roko/runs");
        let dispatcher = dispatcher.with_feedback(GraphFeedbackContext {
            runs_dir: Some(runs.clone()),
            ..GraphFeedbackContext::default()
        });
        dispatcher.begin_stop();
        let ctx = CellContext::new().with_run_id("stopping-run".to_string());
        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the agent exited on SIGTERM");
        assert!(matches!(error, RokoError::Cancelled(_)), "got {error:?}");
        // Closing the run's writer flushes its lines.
        drop(dispatcher);

        let attempts = runs.join("stopping-run").join("attempts.jsonl");
        let is_verdict = |row: &serde_json::Value| row["schema_version"] == "roko.verdict/1";
        let verdicts = jsonl_rows_where(&attempts, 1, is_verdict).await;
        assert_eq!(verdicts[0]["outcome"], "cancelled", "{}", verdicts[0]);
    }
}
