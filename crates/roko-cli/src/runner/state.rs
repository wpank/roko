//! `RunState` — mutable per-execution state tracking agent output, tokens,
//! and progress across all plans.

use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::time::Instant;

use roko_core::defaults::{
    DEFAULT_RUNNER_RETRY_BACKOFF_MAX_SECS, DEFAULT_RUNNER_RETRY_BACKOFF_MULTIPLIER_FALLBACK,
    DEFAULT_RUNNER_RETRY_BACKOFF_SHIFT_CAP, DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT,
};
use roko_learn::model_router::RoutingContext;

use super::types::{
    AgentDispatchOutcome, PlanLifecycleStatus, RetryAction, RunnerEvent, RunnerFailureKind,
    RunnerLifecycleProjection, RunnerRunStatus, TaskAttemptLifecycle, TaskAttemptOutcome,
    TaskAttemptRef, TaskAttemptStatus, TaskLifecycle, TaskLifecycleStatus,
};

/// Mutable state for the current runner execution.
#[derive(Debug)]
pub struct RunState {
    // ─── Runtime Lifecycle ──────────────────────────────────────────
    /// Typed runtime lifecycle projection updated from runner events.
    pub lifecycle: RunnerLifecycleProjection,

    // ─── Agent ──────────────────────────────────────────────────────
    /// Whether an agent process is currently alive.
    pub agent_active: bool,
    /// Model name reported by the agent's SystemInit.
    pub agent_model: String,
    /// Provider/backend reported by the dispatch layer.
    pub agent_provider: String,
    /// Accumulated text output from the current agent run.
    pub agent_output: String,
    /// Session ID from the last SystemInit or TurnCompleted.
    pub session_id: Option<String>,
    /// PID of the current agent process.
    pub agent_pid: Option<u32>,
    /// Whether the current agent emitted a structured turn-completed event.
    pub agent_turn_completed: bool,

    // ─── Tokens / Cost ──────────────────────────────────────────────
    /// Input tokens consumed this task.
    pub tokens_in: u64,
    /// Output tokens consumed this task.
    pub tokens_out: u64,
    /// Cache read tokens this task (subset of input).
    pub cache_read_tokens: u64,
    /// Cache write tokens this task.
    pub cache_write_tokens: u64,
    /// Estimated cost in USD this task.
    pub cost_usd: f64,
    /// Number of agent spawn attempts for the current task (retries).
    pub task_agent_calls: u32,

    // ─── Current task ───────────────────────────────────────────────
    /// Plan currently being executed.
    pub plan_id: String,
    /// Task currently being executed.
    pub current_task: String,
    /// Explicit `model_hint` for the current task, if the task definition
    /// pinned one. Used to dampen routing feedback for non-router choices.
    pub task_model_hint: Option<String>,
    /// Full prompt text sent for the current task, used by feedback sinks.
    pub current_prompt_text: String,
    /// Strategy-space coordinates used for Daimon somatic outcome recording.
    pub current_daimon_strategy: Option<roko_daimon::StrategyCoordinates>,

    // ─── Progress ───────────────────────────────────────────────────
    /// Number of tasks completed across all plans.
    pub tasks_completed: usize,
    /// Number of tasks that failed.
    pub tasks_failed: usize,
    /// Total tasks across all plans.
    pub tasks_total: usize,
    /// Current gate output (last gate run).
    pub gate_output: String,
    /// Iteration count per task, keyed by `"{plan_id}:{task_id}"`.
    ///
    /// Compatibility mirror for older executor call sites. The canonical
    /// attempt allocation lives in `lifecycle.tasks[*].current_attempt` /
    /// `next_attempt` and this map is only updated from those values.
    pub iterations: HashMap<String, u32>,

    // ─── Totals ─────────────────────────────────────────────────────
    /// Total input tokens across the entire run.
    pub total_tokens_in: u64,
    /// Total output tokens across the entire run.
    pub total_tokens_out: u64,
    /// Total cost in USD across the entire run.
    pub total_cost_usd: f64,
    /// Total agent spawns across the entire run.
    pub total_agent_calls: usize,
    /// Cost accumulated per plan_id (for per-plan budget enforcement).
    pub plan_costs: HashMap<String, f64>,
    /// Current retry backoff deadline per plan.
    pub retry_backoff_until: HashMap<String, Instant>,
    /// Last structured failure kind per plan.
    pub last_failure_kind: HashMap<String, RunnerFailureKind>,
    /// Gate/verify effect keys currently running in background tasks.
    pub active_gate_effects: HashSet<String>,

    // ─── Task DAG ───────────────────────────────────────────────────
    /// Completed task IDs per plan (for DAG dependency resolution).
    pub completed_tasks: HashMap<String, Vec<String>>,
    /// Failed task IDs per plan. Tasks depending on a failed task are
    /// skipped rather than blocking the entire plan.
    pub failed_tasks: HashMap<String, HashSet<String>>,
    /// Files created or modified by each completed task.
    /// Key: `"{plan_id}:{task_id}"`, value: list of file paths.
    pub task_outputs: HashMap<String, Vec<String>>,

    // ─── Health ──────────────────────────────────────────────────────
    /// Consecutive snapshot-save failures. After 3, `snapshot_degraded` is set.
    pub snapshot_fail_streak: u32,
    /// Set after 3 consecutive snapshot failures — crash recovery data may be stale.
    pub snapshot_degraded: bool,

    // ─── Timing ─────────────────────────────────────────────────────
    /// When the run started.
    pub started_at: Instant,
    /// Epoch timestamp (ms since UNIX epoch) when the run started. Used in
    /// snapshots for cross-run comparisons and dashboard display.
    pub start_epoch_ms: u64,
    /// When the current task started (reset per task).
    pub task_started_at: Instant,
    /// How long the last dispatch_action (prompt assembly + spawn) took in ms.
    pub last_dispatch_ms: u64,

    // ─── Replan Context ──────────────────────────────────────────────
    /// Accumulated failure context per plan/task for retry prompt enrichment.
    pub replan_contexts: HashMap<String, String>,
    /// Durable gate-failure replan ledger. Unlike transient retry prompt
    /// context, this is written into snapshots so duplicate revision requests
    /// and per-plan caps survive resume.
    pub replan_ledger: super::persist::ReplanLedgerSnapshot,
    /// Revised task definitions produced by gate-failure plan revisions.
    /// Key: `"{plan_id}/{task_id}"`.
    pub revised_tasks: HashMap<String, super::persist::TaskRevision>,

    // ─── Resume Fingerprints ─────────────────────────────────────────
    /// Forensic fingerprints for every task definition known to this
    /// run. Populated once at startup so `run-state.json` snapshots
    /// always carry the data the strict resume validator needs.
    pub task_fingerprints: Vec<super::persist::TaskDefFingerprint>,

    // ─── Routing ─────────────────────────────────────────────────────
    /// Dispatch-time routing context for the current task. Stored here
    /// so `FeedbackEvent::TaskCompleted` can carry the real feature
    /// vector to the CascadeRouter's bandit.
    pub routing_context: Option<RoutingContext>,

    /// Per-task failure reasons (plan_id:task_id → reason string).
    /// Populated when a task fails so the final summary can show why.
    pub failure_reasons: HashMap<String, String>,
}

impl RunState {
    /// Create a new empty run state.
    pub fn new(total_tasks: usize) -> Self {
        Self {
            lifecycle: RunnerLifecycleProjection::new(total_tasks),
            agent_active: false,
            agent_model: String::new(),
            agent_provider: String::new(),
            agent_output: String::new(),
            session_id: None,
            agent_pid: None,
            agent_turn_completed: false,
            tokens_in: 0,
            tokens_out: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0,
            task_agent_calls: 0,
            plan_id: String::new(),
            current_task: String::new(),
            task_model_hint: None,
            current_prompt_text: String::new(),
            current_daimon_strategy: None,
            tasks_completed: 0,
            tasks_failed: 0,
            tasks_total: total_tasks,
            gate_output: String::new(),
            iterations: HashMap::new(),
            total_tokens_in: 0,
            total_tokens_out: 0,
            total_cost_usd: 0.0,
            total_agent_calls: 0,
            plan_costs: HashMap::new(),
            retry_backoff_until: HashMap::new(),
            last_failure_kind: HashMap::new(),
            active_gate_effects: HashSet::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            task_outputs: HashMap::new(),
            snapshot_fail_streak: 0,
            snapshot_degraded: false,
            started_at: Instant::now(),
            start_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            task_started_at: Instant::now(),
            last_dispatch_ms: 0,
            replan_contexts: HashMap::new(),
            replan_ledger: super::persist::ReplanLedgerSnapshot::default(),
            revised_tasks: HashMap::new(),
            task_fingerprints: Vec::new(),
            routing_context: None,
            failure_reasons: HashMap::new(),
        }
    }

    /// Stable ID for this runner invocation.
    pub fn run_id(&self) -> &str {
        &self.lifecycle.run_id
    }

    /// Current task attempt reference, using at least attempt 1.
    pub fn current_attempt_ref(&self) -> TaskAttemptRef {
        TaskAttemptRef::new(
            self.plan_id.clone(),
            self.current_task.clone(),
            self.iteration_for(&self.plan_id, &self.current_task),
        )
    }

    /// Get the iteration count for a specific plan/task pair.
    pub fn iteration_for(&self, plan_id: &str, task_id: &str) -> u32 {
        let key = format!("{plan_id}:{task_id}");
        self.lifecycle
            .tasks
            .get(&key)
            .map(|task| task.current_attempt.max(1))
            .or_else(|| self.iterations.get(&key).copied())
            .unwrap_or(1)
    }

    /// Set the iteration count for a specific plan/task pair.
    pub fn set_iteration(&mut self, plan_id: &str, task_id: &str, value: u32) {
        let attempt = value.max(1);
        let key = task_key(plan_id, task_id);
        self.ensure_task_lifecycle(plan_id, task_id, 0);
        if let Some(task) = self.lifecycle.tasks.get_mut(&key) {
            task.current_attempt = task.current_attempt.max(attempt);
            task.next_attempt = task
                .next_attempt
                .max(task.current_attempt.saturating_add(1));
            self.iterations.insert(key, task.current_attempt);
        }
    }

    /// Allocate the next attempt for a task monotonically.
    ///
    /// This is the canonical allocator for new code. `set_iteration` remains as
    /// a compatibility shim for executor code that still owns a plan iteration.
    pub fn allocate_next_attempt(&mut self, plan_id: &str, task_id: &str) -> TaskAttemptRef {
        let key = task_key(plan_id, task_id);
        self.ensure_task_lifecycle(plan_id, task_id, 0);
        let attempt = self
            .lifecycle
            .tasks
            .get_mut(&key)
            .map(|task| {
                let attempt = task
                    .next_attempt
                    .max(task.current_attempt.saturating_add(1));
                task.current_attempt = attempt;
                task.next_attempt = attempt.saturating_add(1);
                attempt
            })
            .unwrap_or(1);
        self.iterations.insert(key, attempt);
        TaskAttemptRef::new(plan_id.to_string(), task_id.to_string(), attempt)
    }

    /// Apply a normalized runner event to the in-memory lifecycle projection.
    pub fn apply_runner_event(&mut self, event: &RunnerEvent) {
        self.lifecycle.events_seen = self.lifecycle.events_seen.saturating_add(1);

        match event {
            RunnerEvent::ResumeMarker { marker, .. } => {
                self.lifecycle.resumed =
                    matches!(marker.outcome, super::types::ResumeOutcome::Resumed);
                self.lifecycle.last_resume_marker = Some(marker.clone());
            }
            RunnerEvent::RunStarted {
                total_tasks,
                resumed,
                ..
            } => {
                self.lifecycle.status = RunnerRunStatus::Running;
                self.lifecycle.total_tasks = *total_tasks;
                self.lifecycle.resumed = *resumed;
            }
            RunnerEvent::RunCompleted { outcome, .. } => {
                self.lifecycle.status = match outcome {
                    super::types::RunOutcome::Succeeded => RunnerRunStatus::Completed,
                    super::types::RunOutcome::Failed => RunnerRunStatus::Failed,
                    super::types::RunOutcome::Cancelled => RunnerRunStatus::Cancelled,
                };
            }
            RunnerEvent::PlanStarted { plan_id, .. } => {
                self.lifecycle
                    .plans
                    .insert(plan_id.clone(), PlanLifecycleStatus::Started);
            }
            RunnerEvent::PlanCompleted {
                plan_id, outcome, ..
            } => {
                self.lifecycle
                    .plans
                    .insert(plan_id.clone(), (*outcome).into());
            }
            RunnerEvent::TaskAttemptStarted {
                attempt,
                status,
                timestamp_ms,
                ..
            } => {
                self.register_attempt_start(attempt, *timestamp_ms);
                self.upsert_attempt(attempt, *status, *timestamp_ms, None, None);
            }
            RunnerEvent::AgentDispatchStarted {
                attempt,
                agent_id,
                timestamp_ms,
                ..
            } => {
                self.upsert_attempt(
                    attempt,
                    TaskAttemptStatus::DispatchingAgent,
                    *timestamp_ms,
                    Some(agent_id.clone()),
                    None,
                );
            }
            RunnerEvent::AgentDispatchCompleted {
                attempt,
                outcome,
                agent_id,
                timestamp_ms,
                ..
            } => {
                let status = match outcome {
                    AgentDispatchOutcome::Spawned => TaskAttemptStatus::AgentRunning,
                    AgentDispatchOutcome::SpawnFailed | AgentDispatchOutcome::Failed => {
                        TaskAttemptStatus::Failed
                    }
                    AgentDispatchOutcome::Completed | AgentDispatchOutcome::Exited => {
                        TaskAttemptStatus::AgentCompleted
                    }
                };
                self.upsert_attempt(attempt, status, *timestamp_ms, Some(agent_id.clone()), None);
            }
            RunnerEvent::AgentCompleted {
                attempt,
                outcome,
                timestamp_ms,
                ..
            } => {
                let status = match outcome {
                    AgentDispatchOutcome::Completed | AgentDispatchOutcome::Exited => {
                        TaskAttemptStatus::AgentCompleted
                    }
                    AgentDispatchOutcome::Failed | AgentDispatchOutcome::SpawnFailed => {
                        TaskAttemptStatus::Failed
                    }
                    AgentDispatchOutcome::Spawned => TaskAttemptStatus::AgentRunning,
                };
                self.upsert_attempt(attempt, status, *timestamp_ms, None, None);
            }
            RunnerEvent::GateDispatchStarted {
                attempt,
                timestamp_ms,
                ..
            } => {
                self.upsert_attempt(
                    attempt,
                    TaskAttemptStatus::Gating,
                    *timestamp_ms,
                    None,
                    None,
                );
            }
            RunnerEvent::GateCompleted {
                attempt,
                passed,
                failure_kind,
                timestamp_ms,
                ..
            } => {
                let status = if *passed {
                    TaskAttemptStatus::Passed
                } else {
                    TaskAttemptStatus::GateFailed
                };
                self.upsert_attempt(attempt, status, *timestamp_ms, None, *failure_kind);
            }
            RunnerEvent::PromptAssembled { .. } => {}
            RunnerEvent::MergeBackendCompleted { .. } => {}
            RunnerEvent::RetryDecision {
                attempt,
                action,
                failure_kind,
                next_attempt,
                timestamp_ms,
                ..
            } => {
                let status = match action {
                    RetryAction::RetryAfterBackoff => TaskAttemptStatus::Retrying,
                    RetryAction::Exhausted => TaskAttemptStatus::Exhausted,
                    RetryAction::NotRetryable => TaskAttemptStatus::Failed,
                };
                let key = attempt.key();
                self.upsert_attempt(attempt, status, *timestamp_ms, None, Some(*failure_kind));
                if let Some(attempt_state) = self.lifecycle.task_attempts.get_mut(&key) {
                    attempt_state.retry_action = Some(*action);
                }
                self.apply_retry_to_task(attempt, *action, *failure_kind, *next_attempt);
            }
            RunnerEvent::TaskAttemptCompleted {
                attempt,
                outcome,
                failure_kind,
                timestamp_ms,
                ..
            } => {
                let status = match outcome {
                    TaskAttemptOutcome::Passed => TaskAttemptStatus::Passed,
                    TaskAttemptOutcome::Failed => TaskAttemptStatus::Failed,
                    TaskAttemptOutcome::Exhausted => TaskAttemptStatus::Exhausted,
                    TaskAttemptOutcome::Cancelled => TaskAttemptStatus::Cancelled,
                };
                let key = attempt.key();
                self.upsert_attempt(attempt, status, *timestamp_ms, None, *failure_kind);
                if let Some(attempt_state) = self.lifecycle.task_attempts.get_mut(&key) {
                    attempt_state.completed_at_ms = Some(*timestamp_ms);
                }
                self.apply_attempt_terminal_to_task(attempt, status, *timestamp_ms, *failure_kind);
            }
        }
    }

    fn upsert_attempt(
        &mut self,
        attempt: &TaskAttemptRef,
        status: TaskAttemptStatus,
        timestamp_ms: u64,
        agent_id: Option<String>,
        failure_kind: Option<RunnerFailureKind>,
    ) {
        self.ensure_task_lifecycle(&attempt.plan_id, &attempt.task_id, timestamp_ms);
        self.observe_attempt_number(attempt);
        let key = attempt.key();
        let entry =
            self.lifecycle
                .task_attempts
                .entry(key)
                .or_insert_with(|| TaskAttemptLifecycle {
                    attempt: attempt.clone(),
                    status,
                    started_at_ms: timestamp_ms,
                    completed_at_ms: None,
                    agent_id: None,
                    failure_kind: None,
                    retry_action: None,
                });

        if !entry.status.can_transition_to(status) {
            tracing::warn!(
                plan_id = %attempt.plan_id,
                task_id = %attempt.task_id,
                attempt = attempt.attempt,
                from = ?entry.status,
                to = ?status,
                "illegal task attempt transition ignored"
            );
            return;
        }

        entry.status = status;
        if let Some(agent_id) = agent_id {
            entry.agent_id = Some(agent_id);
        }
        if let Some(failure_kind) = failure_kind {
            entry.failure_kind = Some(failure_kind);
        }
        self.apply_attempt_status_to_task(attempt, status, timestamp_ms, failure_kind);
    }

    fn ensure_task_lifecycle(&mut self, plan_id: &str, task_id: &str, timestamp_ms: u64) {
        let key = task_key(plan_id, task_id);
        self.lifecycle
            .tasks
            .entry(key.clone())
            .or_insert_with(|| TaskLifecycle {
                plan_id: plan_id.to_string(),
                task_id: task_id.to_string(),
                status: TaskLifecycleStatus::Started,
                current_attempt: 1,
                next_attempt: 2,
                started_at_ms: timestamp_ms,
                completed_at_ms: None,
                latest_failure_kind: None,
            });
        self.iterations.entry(key).or_insert(1);
    }

    fn observe_attempt_number(&mut self, attempt: &TaskAttemptRef) {
        let key = attempt.task_key();
        self.ensure_task_lifecycle(&attempt.plan_id, &attempt.task_id, 0);
        if let Some(task) = self.lifecycle.tasks.get_mut(&key) {
            task.current_attempt = task.current_attempt.max(attempt.attempt.max(1));
            task.next_attempt = task
                .next_attempt
                .max(task.current_attempt.saturating_add(1));
            self.iterations.insert(key, task.current_attempt);
        }
    }

    fn register_attempt_start(&mut self, attempt: &TaskAttemptRef, timestamp_ms: u64) {
        self.ensure_task_lifecycle(&attempt.plan_id, &attempt.task_id, timestamp_ms);
        self.observe_attempt_number(attempt);
        self.supersede_retry_attempts_before(attempt, timestamp_ms);
        if let Some(task) = self.lifecycle.tasks.get_mut(&attempt.task_key()) {
            if !task.status.is_terminal() {
                task.status = TaskLifecycleStatus::Running;
                task.completed_at_ms = None;
            }
        }
    }

    fn supersede_retry_attempts_before(&mut self, attempt: &TaskAttemptRef, timestamp_ms: u64) {
        for prior in self.lifecycle.task_attempts.values_mut() {
            if prior.attempt.plan_id == attempt.plan_id
                && prior.attempt.task_id == attempt.task_id
                && prior.attempt.attempt < attempt.attempt
                && prior.status == TaskAttemptStatus::Retrying
            {
                prior.status = TaskAttemptStatus::Superseded;
                prior.completed_at_ms = Some(timestamp_ms);
            }
        }
    }

    fn apply_retry_to_task(
        &mut self,
        attempt: &TaskAttemptRef,
        action: RetryAction,
        failure_kind: RunnerFailureKind,
        next_attempt: Option<u32>,
    ) {
        self.ensure_task_lifecycle(&attempt.plan_id, &attempt.task_id, 0);
        if let Some(task) = self.lifecycle.tasks.get_mut(&attempt.task_key()) {
            task.latest_failure_kind = Some(failure_kind);
            match action {
                RetryAction::RetryAfterBackoff => {
                    task.status = TaskLifecycleStatus::Retrying;
                    task.completed_at_ms = None;
                    task.current_attempt = task.current_attempt.max(attempt.attempt);
                    if let Some(next_attempt) = next_attempt {
                        task.next_attempt = task.next_attempt.max(next_attempt.max(1));
                    }
                    task.next_attempt = task
                        .next_attempt
                        .max(task.current_attempt.saturating_add(1));
                    self.iterations
                        .insert(attempt.task_key(), task.current_attempt.max(1));
                }
                RetryAction::Exhausted => {
                    task.status = TaskLifecycleStatus::Exhausted;
                }
                RetryAction::NotRetryable => {
                    task.status = TaskLifecycleStatus::Failed;
                }
            }
        }
    }

    fn apply_attempt_status_to_task(
        &mut self,
        attempt: &TaskAttemptRef,
        status: TaskAttemptStatus,
        timestamp_ms: u64,
        failure_kind: Option<RunnerFailureKind>,
    ) {
        if status.is_terminal() {
            self.apply_attempt_terminal_to_task(attempt, status, timestamp_ms, failure_kind);
            return;
        }
        if let Some(task) = self.lifecycle.tasks.get_mut(&attempt.task_key()) {
            if let Some(failure_kind) = failure_kind {
                task.latest_failure_kind = Some(failure_kind);
            }
            task.status = match status {
                TaskAttemptStatus::Retrying => TaskLifecycleStatus::Retrying,
                _ => TaskLifecycleStatus::Running,
            };
            task.completed_at_ms = None;
        }
    }

    fn apply_attempt_terminal_to_task(
        &mut self,
        attempt: &TaskAttemptRef,
        status: TaskAttemptStatus,
        timestamp_ms: u64,
        failure_kind: Option<RunnerFailureKind>,
    ) {
        if let Some(task) = self.lifecycle.tasks.get_mut(&attempt.task_key()) {
            if attempt.attempt < task.current_attempt && status == TaskAttemptStatus::Superseded {
                return;
            }
            if let Some(failure_kind) = failure_kind {
                task.latest_failure_kind = Some(failure_kind);
            }
            task.status = match status {
                TaskAttemptStatus::Passed => TaskLifecycleStatus::Passed,
                TaskAttemptStatus::Failed => TaskLifecycleStatus::Failed,
                TaskAttemptStatus::Exhausted => TaskLifecycleStatus::Exhausted,
                TaskAttemptStatus::Cancelled => TaskLifecycleStatus::Cancelled,
                TaskAttemptStatus::Superseded => task.status,
                _ => task.status,
            };
            if task.status.is_terminal() {
                task.completed_at_ms = Some(timestamp_ms);
            }
        }
    }

    /// Reset per-task accumulators for a new task.
    pub fn reset_for_task(&mut self, plan_id: &str, task_id: &str) {
        self.agent_active = false;
        self.agent_model.clear();
        self.agent_provider.clear();
        self.agent_output.clear();
        self.session_id = None;
        self.agent_pid = None;
        self.agent_turn_completed = false;
        self.tokens_in = 0;
        self.tokens_out = 0;
        self.cache_read_tokens = 0;
        self.cache_write_tokens = 0;
        self.cost_usd = 0.0;
        self.task_agent_calls = 0;
        self.plan_id = plan_id.to_string();
        self.current_task = task_id.to_string();
        self.task_model_hint = None;
        self.current_prompt_text.clear();
        self.current_daimon_strategy = None;
        self.gate_output.clear();
        // iteration is per-task in self.iterations, set from executor state
        self.task_started_at = Instant::now();
        self.last_dispatch_ms = 0;
        self.routing_context = None;
    }

    /// Record a completed task, rolling per-task stats into totals.
    pub fn task_completed(&mut self) {
        self.tasks_completed += 1;
        self.roll_into_totals();
    }

    /// Record a failed task, rolling per-task stats into totals.
    pub fn task_failed(&mut self) {
        self.tasks_failed += 1;
        self.roll_into_totals();
    }

    /// Record why a specific task failed, for the final summary.
    pub fn record_task_failure(&mut self, plan_id: &str, task_id: &str, reason: &str) {
        let key = format!("{plan_id}:{task_id}");
        // Keep first 3 lines, up to 500 chars total.
        let lines: String = reason.lines().take(3).collect::<Vec<_>>().join("\n");
        let truncated = if lines.len() > 500 {
            format!("{}...", &lines[..500])
        } else if reason.lines().count() > 3 {
            format!("{lines}\n...")
        } else {
            lines
        };
        self.failure_reasons.entry(key).or_insert(truncated);
    }

    pub fn roll_into_totals(&mut self) {
        self.total_tokens_in += self.tokens_in;
        self.total_tokens_out += self.tokens_out;
        self.total_cost_usd += self.cost_usd;
        // Track per-plan cost for budget enforcement.
        if !self.plan_id.is_empty() {
            *self.plan_costs.entry(self.plan_id.clone()).or_default() += self.cost_usd;
        }
    }

    /// Force a plan into a terminal state when `apply_event(Fatal)` is rejected
    /// by the executor (e.g. because the plan is already in a terminal state or
    /// the state machine rejects the transition). This prevents the run from
    /// hanging forever waiting for a plan that can never advance.
    pub fn force_plan_terminal(&mut self, plan_id: &str) {
        tracing::warn!(plan_id = %plan_id, "force_plan_terminal: marking plan as dead in RunState");
        self.tasks_failed += 1;
        self.failure_reasons
            .entry(format!("{plan_id}:_forced"))
            .or_insert_with(|| "plan forced terminal after apply_event(Fatal) rejection".into());
    }

    /// Cost accumulated for a specific plan.
    pub fn plan_cost(&self, plan_id: &str) -> f64 {
        self.plan_costs.get(plan_id).copied().unwrap_or(0.0)
    }

    /// Whether a plan is still cooling down before retry dispatch.
    pub fn retry_cooldown_remaining(&self, plan_id: &str) -> Option<Duration> {
        let deadline = self.retry_backoff_until.get(plan_id)?;
        deadline.checked_duration_since(Instant::now())
    }

    /// Record retry cooldown and classification after a failed gate.
    pub fn set_retry_backoff(
        &mut self,
        plan_id: &str,
        failure_kind: RunnerFailureKind,
        attempt: u32,
    ) {
        self.last_failure_kind
            .insert(plan_id.to_string(), failure_kind);
        let base = failure_kind.retry_cooldown_secs();
        if base == 0 {
            self.retry_backoff_until.remove(plan_id);
            return;
        }
        let multiplier = 1u64
            .checked_shl(attempt.min(DEFAULT_RUNNER_RETRY_BACKOFF_SHIFT_CAP))
            .unwrap_or(DEFAULT_RUNNER_RETRY_BACKOFF_MULTIPLIER_FALLBACK);
        let delay = Duration::from_secs(
            base.saturating_mul(multiplier)
                .min(DEFAULT_RUNNER_RETRY_BACKOFF_MAX_SECS),
        );
        self.retry_backoff_until
            .insert(plan_id.to_string(), Instant::now() + delay);
    }

    /// Clear retry backoff state for a plan after successful forward progress.
    pub fn clear_retry_backoff(&mut self, plan_id: &str) {
        self.retry_backoff_until.remove(plan_id);
        self.last_failure_kind.remove(plan_id);
    }

    /// Returns true when this gate effect was newly marked active.
    pub fn mark_gate_active(&mut self, key: impl Into<String>) -> bool {
        self.active_gate_effects.insert(key.into())
    }

    /// Clear a finished gate effect key.
    pub fn clear_gate_active(&mut self, key: &str) {
        self.active_gate_effects.remove(key);
    }

    /// Wall time for the current task (since last `reset_for_task`).
    pub fn task_elapsed_ms(&self) -> u64 {
        self.task_started_at.elapsed().as_millis() as u64
    }

    /// Record a successful snapshot save — resets the failure streak.
    pub fn snapshot_succeeded(&mut self) {
        self.snapshot_fail_streak = 0;
    }

    /// Record a failed snapshot save. After consecutive failures past the pivot
    /// threshold, sets degraded flag.
    pub fn snapshot_failed(&mut self) {
        self.snapshot_fail_streak += 1;
        if self.snapshot_fail_streak >= DEFAULT_RUNNER_RETRY_STRATEGY_PIVOT_ATTEMPT
            && !self.snapshot_degraded
        {
            self.snapshot_degraded = true;
            tracing::warn!(
                streak = self.snapshot_fail_streak,
                "snapshot persistence degraded — crash recovery data may be stale"
            );
        }
    }

    /// Mark a task as completed for DAG dependency tracking.
    ///
    /// Returns true only when this call newly recorded a concrete task.
    pub fn mark_task_completed(&mut self, plan_id: &str, task_id: &str) -> bool {
        if task_id.is_empty() {
            return false;
        }
        let completed = self.completed_tasks.entry(plan_id.to_string()).or_default();
        if !completed
            .iter()
            .any(|completed_task| completed_task == task_id)
        {
            completed.push(task_id.to_string());
            true
        } else {
            false
        }
    }

    /// Get completed task IDs for a plan.
    pub fn plan_completed_tasks(&self, plan_id: &str) -> &[String] {
        self.completed_tasks
            .get(plan_id)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    /// Mark a task as permanently failed for DAG tracking.
    pub fn mark_task_failed(&mut self, plan_id: &str, task_id: &str) {
        self.failed_tasks
            .entry(plan_id.to_string())
            .or_default()
            .insert(task_id.to_string());
    }

    /// Get the set of failed task IDs for a plan.
    pub fn plan_failed_tasks(&self, plan_id: &str) -> &HashSet<String> {
        static EMPTY: std::sync::LazyLock<HashSet<String>> = std::sync::LazyLock::new(HashSet::new);
        self.failed_tasks.get(plan_id).unwrap_or(&EMPTY)
    }

    /// Record the files produced by a completed task.
    pub fn record_task_outputs(&mut self, plan_id: &str, task_id: &str, files: Vec<String>) {
        let key = format!("{plan_id}:{task_id}");
        self.task_outputs.insert(key, files);
    }

    /// Return the files recorded for a specific completed task.
    pub fn task_output_files(&self, plan_id: &str, task_id: &str) -> &[String] {
        let key = format!("{plan_id}:{task_id}");
        self.task_outputs
            .get(&key)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    /// Collect output file lists for all tasks in `depends_on`.
    ///
    /// Returns a vec of `(task_id, files)` pairs — empty pairs are
    /// omitted so callers only see tasks that actually produced output.
    pub fn dependency_outputs(
        &self,
        plan_id: &str,
        depends_on: &[String],
    ) -> Vec<(String, Vec<String>)> {
        depends_on
            .iter()
            .filter_map(|task_id| {
                let files = self.task_output_files(plan_id, task_id);
                if files.is_empty() {
                    None
                } else {
                    Some((task_id.clone(), files.to_vec()))
                }
            })
            .collect()
    }

    /// Total elapsed time since the run started.
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Store failure context for a task to enrich the next retry prompt.
    pub fn set_replan_context(&mut self, plan_id: &str, task_id: &str, context: String) {
        self.replan_contexts
            .insert(format!("{plan_id}/{task_id}"), context);
    }

    /// Take (and remove) stored replan context for a task.
    pub fn take_replan_context(&mut self, plan_id: &str, task_id: &str) -> Option<String> {
        self.replan_contexts.remove(&format!("{plan_id}/{task_id}"))
    }

    /// Whether a gate failure has already been upgraded into a durable task
    /// revision.
    pub fn has_seen_replan_failure(&self, failure_key: &str) -> bool {
        self.replan_ledger
            .seen_failure_keys
            .iter()
            .any(|key| key == failure_key)
    }

    /// Number of durable gate-failure replans recorded for `plan_id`.
    pub fn replan_count_for(&self, plan_id: &str) -> u32 {
        self.replan_ledger
            .replans_seen
            .get(plan_id)
            .copied()
            .unwrap_or_default()
    }

    /// Record a durable task revision and update the replan ledger.
    pub fn record_task_revision(
        &mut self,
        failure_key: String,
        revision: super::persist::TaskRevision,
    ) {
        if !self.has_seen_replan_failure(&failure_key) {
            self.replan_ledger
                .seen_failure_keys
                .push(failure_key.clone());
        }
        let plan_count = self
            .replan_ledger
            .replans_seen
            .entry(revision.plan_id.clone())
            .or_insert(0);
        *plan_count = plan_count.saturating_add(1);
        self.replan_ledger
            .revision_requests
            .push(revision.revision_request.clone());
        self.revised_tasks.insert(
            format!("{}/{}", revision.plan_id, revision.task_id),
            revision,
        );
    }
}

fn task_key(plan_id: &str, task_id: &str) -> String {
    format!("{plan_id}:{task_id}")
}

#[cfg(test)]
mod tests {
    use super::RunState;
    use crate::runner::types::{
        AgentCompletionSummary, AgentDispatchOutcome, RetryAction, RunnerEvent, RunnerFailureKind,
        TaskAttemptRef, TaskAttemptStatus, TaskLifecycleStatus,
    };

    #[test]
    fn mark_task_completed_ignores_empty_and_duplicates() {
        let mut state = RunState::new(2);

        assert!(!state.mark_task_completed("plan", ""));
        assert!(state.plan_completed_tasks("plan").is_empty());

        assert!(state.mark_task_completed("plan", "T1"));
        assert!(!state.mark_task_completed("plan", "T1"));
        assert_eq!(state.plan_completed_tasks("plan"), ["T1"]);
    }

    #[test]
    fn attempt_allocation_is_monotonic() {
        let mut state = RunState::new(1);

        state.set_iteration("plan", "T1", 3);
        state.set_iteration("plan", "T1", 1);

        assert_eq!(state.iteration_for("plan", "T1"), 3);
        assert_eq!(state.allocate_next_attempt("plan", "T1").attempt, 4);
        assert_eq!(state.iteration_for("plan", "T1"), 4);
        assert_eq!(
            state.lifecycle.tasks["plan:T1"].next_attempt, 5,
            "next allocation cursor remains ahead of current attempt"
        );
    }

    #[test]
    fn starting_new_attempt_supersedes_stale_retry_attempt() {
        let mut state = RunState::new(1);
        let run_id = state.run_id().to_string();
        let first = TaskAttemptRef::new("plan", "T1", 1);
        let second = TaskAttemptRef::new("plan", "T1", 2);

        state.apply_runner_event(&RunnerEvent::task_attempt_started(
            &run_id,
            first.clone(),
            "task",
        ));
        state.apply_runner_event(&RunnerEvent::agent_completed(
            &run_id,
            first.clone(),
            "plan/T1",
            AgentDispatchOutcome::Failed,
            AgentCompletionSummary {
                message: Some("temporary failure".to_string()),
                ..AgentCompletionSummary::default()
            },
        ));
        state.apply_runner_event(&RunnerEvent::retry_decision(
            &run_id,
            first.clone(),
            RetryAction::RetryAfterBackoff,
            RunnerFailureKind::Transient,
            Some(2),
            10,
            "retry".to_string(),
        ));
        state.apply_runner_event(&RunnerEvent::task_attempt_started(&run_id, second, "task"));

        let first_state = &state.lifecycle.task_attempts[&first.key()];
        assert_eq!(first_state.status, TaskAttemptStatus::Superseded);
        assert!(first_state.completed_at_ms.is_some());
        assert_eq!(
            state.lifecycle.tasks["plan:T1"].status,
            TaskLifecycleStatus::Running
        );
        assert_eq!(state.iteration_for("plan", "T1"), 2);
    }
}
