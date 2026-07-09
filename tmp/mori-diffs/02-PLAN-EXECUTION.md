# Path 2: Plan Execution -- DAG, Gates, Retries, Merge Queue

## Current State (What's Broken)

The runner v2 event loop (`crates/roko-cli/src/runner/event_loop.rs`) drives plan execution
through a `tokio::select!` loop over agent events, gate completions, executor ticks, and
cancellation. While functional for single-task-at-a-time runs, it has significant architectural
gaps:

### 1. No Real DAG Resolution (Sentinel-Based)

The executor uses sentinel task names (`"next"`, `"fix"`, `"regen-verify"`) instead of proper
dependency tracking. When `SpawnAgent` arrives with `task == "next"`, the event loop walks all
tasks and picks the first one whose `is_ready(completed)` returns true -- a linear scan sorted
by string ID, not topological order:

```rust
// event_loop.rs:405-418 -- current sentinel resolution
let resolved_task = if task == "next" || task == "fix" || task == "regen-verify" {
    let completed = ctx.state.plan_completed_tasks(plan_id);
    let plan_tasks = ctx.task_index.get(plan_id.as_str());
    plan_tasks.and_then(|tasks| {
        let mut all_tasks: Vec<&TaskDef> = tasks.values().collect();
        all_tasks.sort_by(|a, b| a.id.cmp(&b.id));
        all_tasks
            .iter()
            .find(|t| t.is_ready(completed))
            .map(|t| t.id.clone())
    })
} else {
    Some(task.clone())
};
```

Problems: no cycle detection, no topological guarantee, no parallelism within a plan, and
`depends_on_plan` (cross-plan deps) is defined on `TaskDef` but never consulted during
execution.

### 2. No Gate Timeout

`gate_dispatch::spawn_gate` spawns a `tokio::spawn` with no timeout. If `cargo test` hangs,
the gate task blocks forever and the plan stalls:

```rust
// gate_dispatch.rs:22-37 -- no timeout wrapper
tokio::spawn(async move {
    let verdicts = run_rung(&signal, &ctx, rung, &inputs, &config).await;
    // ...no timeout...
});
```

### 3. No Concurrency Limiting for Cargo Processes

Multiple plans can spawn `cargo test` / `cargo clippy` simultaneously. These compete for the
same build artifacts and filesystem locks. No semaphore limits concurrent cargo invocations.

### 4. Budget Check is Post-Hoc

The per-plan budget check happens at `SpawnAgent` dispatch time (line 434), but cost is only
accumulated after the agent completes a turn. An expensive turn can overshoot the budget
because the check runs before the turn, not continuously during it:

```rust
// event_loop.rs:434-448 -- budget checked only at dispatch
let plan_spent = ctx.state.plan_cost(plan_id);
if max_plan_usd > 0.0 && plan_spent >= max_plan_usd {
    // already over budget by the time we check
}
```

### 5. No Failure Classification

Gate failures are binary (passed/failed). The event loop treats all failures identically --
`GateFailed` triggers `AutoFixing` regardless of whether the error is transient (OOM, flaky
test) or permanent (type error, missing import). `roko-gate` has
`GateFailureClassification` / `FailureClass` / `classify_gate_failure` but these are never
consulted at dispatch time.

### 6. No Retry Backoff

Retries are immediate. When a gate fails, the executor transitions to `AutoFixing`, spawns
the agent again, and runs gates again -- all as fast as the tick interval (100ms). No
exponential backoff, no jitter, no cooldown between attempts.

### 7. No Merge Queue

`MergeBranch` auto-advances with no actual merge:

```rust
// event_loop.rs:566-569
ExecutorAction::MergeBranch { plan_id } => {
    info!(plan_id = %plan_id, "auto-advancing merge");
    let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::MergeSucceeded);
}
```

Multiple plans merging simultaneously risk git conflicts. No serialization or conflict
detection.

### 8. No Plan-Level Timeout

`RunConfig::timeout_secs` exists but is only passed to `ExecutorConfig::task_timeout_secs`.
There is no wall-clock timeout for an entire plan run. A plan can loop through retries
indefinitely (up to `max_retries` per task, but across many tasks that's unbounded total
time).


## Design Goals

1. **Proper DAG**: Topological sort with cycle detection, parallel dispatch within plans, cross-plan dependency support
2. **Gate safety**: Timeout per gate, concurrency limit across plans (cargo semaphore)
3. **Proactive budget**: Check budget before AND after each turn, with per-turn cost projection
4. **Failure intelligence**: Classify gate failures to decide retry strategy
5. **Backoff**: Exponential backoff with jitter for transient failures
6. **Merge serialization**: Sequential merge queue with conflict detection
7. **Plan timeout**: Wall-clock hard limit per plan
8. **Minimal disruption**: New types compose with existing `ParallelExecutor` / `PlanState` / `PlanPhase`


## Architecture

### Task State Machine

Each task within a plan has its own lifecycle, independent of the plan-level `PlanPhase`:

```
                +---------+
                | Pending |
                +----+----+
                     |
            deps satisfied
                     |
                +----v----+
                |  Ready  |
                +----+----+
                     |
              agent spawned
                     |
                +----v----+
                | Running |
                +----+----+
                     |
              agent done
                     |
                +----v----+
                | Gating  |
                +----+----+
                    / \
           passed /   \ failed
                /       \
        +------v-+   +--v------+
        | Passed |   | Failed  |
        +--------+   +----+----+
                          |
                   retries left?
                    /        \
                 yes          no
                /              \
        +------v---+    +------v-----+
        | Retrying |    | Exhausted  |
        +------+---+    +------------+
               |
          backoff elapsed
               |
          +----v----+
          |  Ready  | (loops back)
          +---------+
```

### New Types

```rust
// crates/roko-cli/src/runner/dag.rs

use std::collections::{HashMap, HashSet, VecDeque};
use crate::task_parser::TaskDef;

/// Status of a single task within the DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Waiting for dependencies.
    Pending,
    /// All dependencies met, eligible for dispatch.
    Ready,
    /// Agent is currently working on this task.
    Running,
    /// Gate pipeline is verifying this task's output.
    Gating,
    /// Gate passed. Terminal success.
    Passed,
    /// Gate failed, but retries remain.
    Retrying { attempt: u32, backoff_until: Instant },
    /// Gate failed, retries exhausted. Terminal failure.
    Exhausted { attempts: u32, last_error: String },
}

/// Per-task state tracked by the DAG executor.
#[derive(Debug, Clone)]
pub struct TaskState {
    pub task_id: String,
    pub plan_id: String,
    pub status: TaskStatus,
    /// Direct dependency task IDs (within same plan).
    pub depends_on: Vec<String>,
    /// Cross-plan dependency plan IDs.
    pub depends_on_plan: Vec<String>,
    /// Number of attempts so far.
    pub attempts: u32,
    /// Maximum retry attempts (from TaskDef or plan default).
    pub max_retries: u32,
    /// Wall-clock time when this task started its current attempt.
    pub attempt_started_at: Option<Instant>,
    /// Per-task timeout in seconds.
    pub timeout_secs: u64,
    /// Accumulated cost for this task across all attempts.
    pub cost_usd: f64,
    /// Failure classification from the last gate run.
    pub last_failure: Option<GateFailureClassification>,
}

/// Retry policy with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Base delay in milliseconds (default: 5_000).
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (default: 45_000).
    pub max_delay_ms: u64,
    /// Multiplier per attempt (default: 2.0).
    pub multiplier: f64,
    /// Jitter factor 0.0-1.0 (default: 0.2).
    pub jitter: f64,
}

impl RetryPolicy {
    /// Compute delay for attempt N (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.base_delay_ms as f64
            * self.multiplier.powi(attempt as i32);
        let clamped = base.min(self.max_delay_ms as f64);
        let jitter_range = clamped * self.jitter;
        let jittered = clamped + rand::random::<f64>() * jitter_range * 2.0
            - jitter_range;
        Duration::from_millis(jittered.max(0.0) as u64)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delay_ms: 5_000,   // 5s
            max_delay_ms: 45_000,   // 45s
            multiplier: 2.0,
            jitter: 0.2,
        }
    }
}
// Concrete backoff schedule: 5s, 10s, 20s, 40s (capped at 45s)
```

```rust
/// DAG executor for a single plan's tasks.
///
/// Owns the topological ordering and readiness computation. The event loop
/// calls `ready_tasks()` instead of scanning all tasks with `is_ready()`.
#[derive(Debug)]
pub struct DagExecutor {
    /// Plan this DAG belongs to.
    plan_id: String,
    /// All tasks in topological order.
    topo_order: Vec<String>,
    /// Per-task state, keyed by task_id.
    tasks: HashMap<String, TaskState>,
    /// Adjacency list: task_id -> set of tasks that depend on it.
    dependents: HashMap<String, Vec<String>>,
    /// Retry policy for this plan.
    retry_policy: RetryPolicy,
}

impl DagExecutor {
    /// Build a DAG from a list of TaskDefs.
    ///
    /// Returns `Err` if the dependency graph has cycles.
    pub fn from_tasks(
        plan_id: &str,
        tasks: &[TaskDef],
        retry_policy: RetryPolicy,
    ) -> Result<Self, DagError> {
        let topo_order = topological_sort(tasks)?;
        // ...build adjacency, TaskState for each...
    }

    /// Return task IDs that are ready to dispatch right now.
    ///
    /// A task is ready when:
    /// - status == Pending
    /// - all depends_on tasks are Passed
    /// - all depends_on_plan plans are complete
    /// - if Retrying, backoff_until has elapsed
    pub fn ready_tasks(&self, completed_plans: &HashSet<String>) -> Vec<&str> {
        // ...
    }

    /// Mark a task as Running.
    pub fn mark_running(&mut self, task_id: &str) -> Result<(), DagError> { ... }

    /// Mark a task as Gating (agent done, gates starting).
    pub fn mark_gating(&mut self, task_id: &str) -> Result<(), DagError> { ... }

    /// Mark a task as Passed (gate succeeded).
    /// Returns newly-ready task IDs (dependents whose deps are now all met).
    pub fn mark_passed(&mut self, task_id: &str) -> Result<Vec<String>, DagError> { ... }

    /// Mark a task as failed. Consults failure classification to decide
    /// whether to retry or exhaust.
    pub fn mark_failed(
        &mut self,
        task_id: &str,
        classification: &GateFailureClassification,
    ) -> Result<TaskFailureOutcome, DagError> { ... }

    /// Whether all tasks are in a terminal state (Passed or Exhausted).
    pub fn is_complete(&self) -> bool { ... }

    /// Whether any task is Exhausted (plan should fail).
    pub fn has_failures(&self) -> bool { ... }

    /// Snapshot for persistence.
    pub fn snapshot(&self) -> DagSnapshot { ... }

    /// Restore from a persisted snapshot.
    pub fn from_snapshot(snapshot: DagSnapshot, retry_policy: RetryPolicy) -> Self { ... }
}
```

```rust
/// Topological sort with cycle detection via Kahn's algorithm.
fn topological_sort(tasks: &[TaskDef]) -> Result<Vec<String>, DagError> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

    for task in tasks {
        in_degree.entry(&task.id).or_insert(0);
        for dep in &task.depends_on {
            adj.entry(dep.as_str()).or_default().push(&task.id);
            *in_degree.entry(&task.id).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut order = Vec::with_capacity(tasks.len());
    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                let deg = in_degree.get_mut(neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    if order.len() != tasks.len() {
        let remaining: Vec<String> = tasks
            .iter()
            .filter(|t| !order.contains(&t.id))
            .map(|t| t.id.clone())
            .collect();
        return Err(DagError::CycleDetected { tasks: remaining });
    }

    Ok(order)
}

#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("dependency cycle among tasks: {tasks:?}")]
    CycleDetected { tasks: Vec<String> },
    #[error("task {0} not found in DAG")]
    TaskNotFound(String),
    #[error("invalid transition for task {task}: {from:?} -> {to:?}")]
    InvalidTransition {
        task: String,
        from: TaskStatus,
        to: &'static str,
    },
}
```

### Gate Semaphore

```rust
// crates/roko-cli/src/runner/gate_dispatch.rs

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// Gate concurrency limiter. Caps the number of concurrent cargo processes
/// to avoid build directory contention and excessive resource usage.
pub struct GateSemaphore {
    inner: Arc<Semaphore>,
}

impl GateSemaphore {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            inner: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Default: 2 concurrent cargo processes.
    pub fn default_cargo() -> Self {
        Self::new(2)
    }
}

/// Updated gate spawn with timeout and semaphore.
pub async fn spawn_gate(
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_semaphore: Arc<GateSemaphore>,
    gate_timeout: Duration,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        // Acquire semaphore permit before running cargo.
        let _permit = gate_semaphore.inner.acquire().await
            .expect("gate semaphore closed");

        let start = Instant::now();

        // Wrap the entire gate run in a timeout + cancellation.
        let result = tokio::select! {
            result = timeout(gate_timeout, run_rung_inner(&plan_id, &task_id, rung, &workdir)) => {
                match result {
                    Ok(verdicts) => verdicts,
                    Err(_) => {
                        // Timeout: return a synthetic failure verdict.
                        vec![Verdict {
                            gate: format!("rung-{rung}"),
                            passed: false,
                            reason: format!("gate timed out after {}s", gate_timeout.as_secs()),
                            ..Default::default()
                        }]
                    }
                }
            }
            _ = cancel.cancelled() => {
                return; // Drop silently on cancellation.
            }
        };

        // ... send completion through gate_tx ...
    });
}
```

### Merge Queue

```rust
// crates/roko-cli/src/runner/merge_queue.rs

use std::collections::VecDeque;
use tokio::sync::Mutex;

/// Sequential merge queue. Plans wait for their turn to merge, preventing
/// concurrent git operations that would cause conflicts.
#[derive(Debug)]
pub struct MergeQueue {
    /// FIFO queue of plan IDs waiting to merge.
    queue: Mutex<VecDeque<String>>,
    /// Plan currently holding the merge lock.
    active: Mutex<Option<String>>,
}

impl MergeQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            active: Mutex::new(None),
        }
    }

    /// Enqueue a plan for merging. Returns true if this plan is first in line.
    pub async fn enqueue(&self, plan_id: &str) -> bool {
        let mut queue = self.queue.lock().await;
        let mut active = self.active.lock().await;

        if active.is_none() {
            *active = Some(plan_id.to_string());
            return true; // Immediate merge slot.
        }

        queue.push_back(plan_id.to_string());
        false
    }

    /// Release the merge lock after a plan completes merging.
    /// Returns the next plan_id that should merge, if any.
    pub async fn release(&self) -> Option<String> {
        let mut queue = self.queue.lock().await;
        let mut active = self.active.lock().await;

        *active = queue.pop_front();
        active.clone()
    }

    /// Check if a specific plan currently holds the merge lock.
    pub async fn is_active(&self, plan_id: &str) -> bool {
        let active = self.active.lock().await;
        active.as_deref() == Some(plan_id)
    }
}
```

### Module Layout

```
crates/roko-cli/src/runner/
  mod.rs              -- re-exports
  dag.rs              -- NEW: DagExecutor, TaskState, TaskStatus, topological_sort
  event_loop.rs       -- MODIFIED: use DagExecutor instead of sentinel resolution
  gate_dispatch.rs    -- MODIFIED: add GateSemaphore, timeout, CancellationToken
  merge_queue.rs      -- NEW: MergeQueue
  retry.rs            -- NEW: RetryPolicy, failure classification bridge
  state.rs            -- MODIFIED: remove completed_tasks tracking (moved to DagExecutor)
  persist.rs          -- MODIFIED: save DagSnapshot alongside ExecutorSnapshot
  types.rs            -- MODIFIED: add GateFailureInfo to GateCompletion
  agent_events.rs     -- unchanged
  agent_stream.rs     -- unchanged
  plan_loader.rs      -- unchanged
  tui_bridge.rs       -- unchanged
```

### Integration Points

#### 1. Event loop uses DagExecutor instead of sentinels

```rust
// BEFORE (event_loop.rs:402-419):
ExecutorAction::SpawnAgent { plan_id, task, .. } => {
    let resolved_task = if task == "next" || task == "fix" { ... };
}

// AFTER:
ExecutorAction::SpawnAgent { plan_id, task, .. } => {
    let task_id = if task == "next" || task == "fix" || task == "regen-verify" {
        // Ask the DAG for the next ready task.
        let dag = ctx.dags.get(plan_id.as_str()).expect("dag for plan");
        let completed_plans = ctx.state.completed_plan_ids();
        match dag.ready_tasks(&completed_plans).first() {
            Some(id) => id.to_string(),
            None => {
                // No more ready tasks.
                let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::ImplementationDone);
                return;
            }
        }
    } else {
        task.clone()
    };

    // Mark task as Running in the DAG.
    ctx.dags.get_mut(plan_id.as_str()).unwrap().mark_running(&task_id).unwrap();
    // ... rest of spawn logic ...
}
```

#### 2. Gate completion uses failure classification

```rust
// BEFORE (event_loop.rs:243-258):
} else {
    state.task_failed();
    match executor.apply_event(&completion.plan_id, &ExecutorEvent::GateFailed) { ... }
}

// AFTER:
} else {
    // Classify the failure using roko-gate's classifier.
    let classification = classify_gate_failure(&completion.verdicts);
    let dag = ctx.dags.get_mut(completion.plan_id.as_str()).unwrap();
    let outcome = dag.mark_failed(&completion.task_id, &classification);

    match outcome {
        TaskFailureOutcome::Retry { delay } => {
            info!(task = %completion.task_id, delay_ms = delay.as_millis(), "retrying after backoff");
            // Task will become Ready again after backoff_until elapses.
            // The next tick will pick it up.
        }
        TaskFailureOutcome::Exhausted => {
            state.task_failed();
            let _ = executor.apply_event(&completion.plan_id, &ExecutorEvent::GateFailed);
        }
        TaskFailureOutcome::PermanentFailure => {
            // Permanent errors skip retries entirely.
            state.task_failed();
            let _ = executor.apply_event(
                &completion.plan_id,
                &ExecutorEvent::Fatal(format!("permanent failure: {}", classification.summary)),
            );
        }
    }
}
```

#### 3. Gate dispatch gains timeout + semaphore

```rust
// BEFORE (event_loop.rs:536-542):
gate_dispatch::spawn_gate(
    plan_id.clone(),
    ctx.state.current_task.clone(),
    *rung,
    ctx.config.workdir.clone(),
    ctx.gate_tx.clone(),
);

// AFTER:
gate_dispatch::spawn_gate(
    plan_id.clone(),
    ctx.state.current_task.clone(),
    *rung,
    ctx.config.workdir.clone(),
    ctx.gate_tx.clone(),
    ctx.gate_semaphore.clone(),
    Duration::from_secs(ctx.config.gate_timeout_secs),
    ctx.cancel.child_token(),
);
```

#### 4. Budget check is pre- and post-dispatch

```rust
// event_loop.rs -- new pre-dispatch budget guard
fn budget_permits_dispatch(state: &RunState, config: &RunConfig, plan_id: &str) -> bool {
    let plan_spent = state.plan_cost(plan_id);
    let max_plan_usd = config.max_plan_usd;
    if max_plan_usd > 0.0 && plan_spent >= max_plan_usd {
        return false;
    }
    // Global budget check.
    let global_max = config.max_run_usd; // NEW field
    if global_max > 0.0 && state.total_cost_usd >= global_max {
        return false;
    }
    true
}

// Also: on every TurnCompleted, re-check budget and kill agent if over.
```

#### 5. Plan-level wall-clock timeout

```rust
// event_loop.rs -- in the tick branch
_ = tick_interval.tick() => {
    // Check plan timeouts.
    for (plan_id, dag) in &ctx.dags {
        if let Some(started) = dag.plan_started_at() {
            if started.elapsed() > Duration::from_secs(ctx.config.plan_timeout_secs) {
                warn!(plan_id = %plan_id, "plan wall-clock timeout exceeded");
                let _ = ctx.executor.apply_event(
                    plan_id,
                    &ExecutorEvent::Fatal("wall-clock timeout".to_string()),
                );
                // Kill any agent working on this plan.
                if ctx.state.plan_id == *plan_id {
                    if let Some(handle) = ctx.agent_handle.take() {
                        handle.kill(Duration::from_secs(3)).await;
                    }
                }
            }
        }
    }

    // Normal tick...
    let actions = executor.tick();
    // ...
}
```


## Detailed Specification

### DAG Resolution

**Input**: `Vec<TaskDef>` from `tasks.toml`, each with `depends_on: Vec<String>` and
`depends_on_plan: Vec<String>`.

**Construction**:
1. Build adjacency list from `depends_on` fields.
2. Run Kahn's algorithm for topological sort.
3. If `order.len() != tasks.len()`, report `CycleDetected` with the remaining task IDs.
4. Store topological order for deterministic dispatch.

**Readiness**:
- A task is Ready when all `depends_on` tasks are `Passed` AND all `depends_on_plan` plans
  are in `PlanPhase::Complete`.
- Tasks in `Retrying` become Ready again when `Instant::now() >= backoff_until`.
- The event loop calls `dag.ready_tasks()` instead of scanning all tasks.

**Parallelism within a plan**:
- `TaskMeta::max_parallel` (already defined in task_parser.rs) caps concurrent running tasks
  per plan. `ready_tasks()` respects this limit by counting tasks in `Running` status.

### Gate Timeout

- Default: 300 seconds (5 minutes), configurable via `RunConfig::gate_timeout_secs`.
- On timeout, a synthetic `GateCompletion` with `passed: false` is sent, containing
  `"gate timed out after Ns"` as the output.
- The gate failure is classified as `Transient` (retryable).

### Gate Semaphore

- Default: 2 concurrent cargo processes (`GateSemaphore::default_cargo()`).
- Configurable via `roko.toml` `[gates] max_concurrent = N`.
- Semaphore is shared across all plans. Each `spawn_gate` call acquires a permit before
  invoking `run_rung`.
- Permit is held for the duration of the gate execution (including timeout).

### Budget Enforcement

**Pre-dispatch**: Before spawning an agent, check `plan_cost(plan_id) < max_plan_usd`
AND `total_cost_usd < max_run_usd`.

**Per-turn**: After every `TurnCompleted` event, re-check budget. If exceeded, kill the agent
immediately and transition the plan to `Failed` with reason `BudgetExceeded`.

**New fields on RunConfig**:
```rust
/// Maximum USD spend across the entire run (0 = unlimited).
pub max_run_usd: f64,
/// Gate timeout in seconds.
pub gate_timeout_secs: u64,
/// Plan wall-clock timeout in seconds.
pub plan_timeout_secs: u64,
```

### Failure Classification

Bridge between `roko-gate`'s existing `GateFailureClassification` and retry decisions:

```rust
// crates/roko-cli/src/runner/retry.rs

use roko_gate::{FailureClass, GateFailureClassification};

/// Outcome of marking a task as failed.
pub enum TaskFailureOutcome {
    /// Task will be retried after a backoff delay.
    Retry { delay: Duration },
    /// Retries exhausted. Task is terminally failed.
    Exhausted,
    /// Error is permanent (type error, missing dep). No retries.
    PermanentFailure,
}

/// Decide whether to retry based on the gate failure classification.
pub fn should_retry_task(
    classification: &GateFailureClassification,
    attempt: u32,
    max_retries: u32,
) -> TaskFailureOutcome {
    match classification.class {
        FailureClass::Permanent => TaskFailureOutcome::PermanentFailure,
        FailureClass::Transient if attempt < max_retries => {
            let policy = RetryPolicy::default();
            TaskFailureOutcome::Retry {
                delay: policy.delay_for_attempt(attempt),
            }
        }
        _ => TaskFailureOutcome::Exhausted,
    }
}
```

### Exponential Backoff Schedule

| Attempt | Base Delay | With 2x multiplier | Capped at 45s |
|---------|-----------|--------------------:|---------------:|
| 0       | 5,000ms   | 5,000ms            | 5,000ms        |
| 1       | 5,000ms   | 10,000ms           | 10,000ms       |
| 2       | 5,000ms   | 20,000ms           | 20,000ms       |
| 3       | 5,000ms   | 40,000ms           | 40,000ms       |
| 4       | 5,000ms   | 80,000ms           | 45,000ms       |

Plus up to +/-20% jitter.

### Merge Queue

1. When a plan reaches `PlanPhase::Merging`, it calls `merge_queue.enqueue(plan_id)`.
2. If the plan is first in line (`enqueue` returns `true`), proceed with merge immediately.
3. Otherwise, the plan waits. On each tick, check if `merge_queue.is_active(plan_id)`.
4. After merge completes (success or failure), call `merge_queue.release()`.
5. If `release()` returns a next plan_id, send `ExecutorEvent::MergeSlotAvailable` to that plan.

### Plan Wall-Clock Timeout

- Default: 3600 seconds (1 hour), configurable via `RunConfig::plan_timeout_secs`.
- Checked on every tick (100ms interval).
- On timeout, the plan transitions to `Failed` with reason `"wall-clock timeout"`.
- Any running agent for the plan is killed immediately.


## Error Handling

| Error | Classification | Action |
|-------|---------------|--------|
| Compile error (type mismatch, missing import) | `Permanent` | Fail immediately, no retry |
| Compile error (ambiguous, could be ordering) | `Transient` | Retry with backoff |
| Test failure (assertion) | `Transient` | Retry with backoff |
| Clippy warning (new lint) | `Transient` | Retry with backoff |
| Flaky test (pass on re-run) | `Transient` | Retry with backoff |
| OOM / process killed | `Transient` | Retry with backoff |
| Gate timeout | `Transient` | Retry with backoff |
| Budget exceeded | `Permanent` | Fail immediately |
| Agent spawn failure | `Permanent` | Fail immediately |
| Cycle detected in DAG | `Permanent` | Fail at plan load, before execution |
| Merge conflict | `Transient` | Rebase and retry merge |


## Testing Strategy

### Unit Tests

1. **Topological sort**: Test with linear chain, diamond DAG, and cycle detection.
2. **Ready tasks**: Test that tasks become ready only after all deps are `Passed`.
3. **Retry policy**: Verify backoff delays match the schedule table.
4. **Failure classification bridge**: Verify `FailureClass::Permanent` skips retries.
5. **Budget guard**: Verify pre-dispatch and per-turn budget checks.
6. **Merge queue**: Verify FIFO ordering, single-active invariant, and release-advances-next.

### Integration Tests

1. **3-task diamond DAG**: Tasks A, B (depends on A), C (depends on A), D (depends on B, C).
   Verify B and C can run in parallel after A completes. Verify D waits for both.
2. **Gate timeout**: Mock gate that sleeps for 10s with a 1s timeout. Verify synthetic failure
   verdict is generated and task retries.
3. **Budget exceeded mid-run**: Set `max_plan_usd = 0.01`, mock agent that reports $0.02 cost.
   Verify plan fails with `BudgetExceeded`.
4. **Retry exhaustion**: Configure `max_retries = 2` with a permanently-failing gate.
   Verify exactly 3 attempts (1 initial + 2 retries), then `Exhausted`.
5. **Merge queue serialization**: Run 2 plans to completion, verify merges happen sequentially
   (not concurrently) via timestamps or mock git operations.

### Property Tests (if proptest available)

1. Random DAGs with N tasks: verify topological sort always produces valid ordering.
2. Random failure sequences: verify retry count never exceeds `max_retries`.


## Open Questions

1. **Cross-plan task parallelism**: Currently `max_concurrent_tasks` is 1 globally. Should we
   allow N tasks across different plans to run simultaneously? Requires per-plan agent handles
   instead of the current single `agent_handle`.

2. **Partial plan completion**: If task D fails but tasks A, B, C succeeded, should the plan
   report partial success? Currently it's all-or-nothing.

3. **Replan on exhaustion**: Should exhausted tasks trigger `build_gate_failure_plan_revision`
   (the existing replan mechanism in orchestrate.rs), or is that orthogonal?

4. **Merge implementation**: The actual git merge (rebase, cherry-pick, merge commit) is not
   specified here. The merge queue handles serialization, but the merge operation itself is
   TBD (Phase 5 or later).

## Implementation Packet

This work makes runner execution match the plan semantics described in `docs/01-orchestration`.

### Required Context

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/state.rs`
- `crates/roko-cli/src/task_parser.rs`
- `crates/roko-orchestrator/src/executor/action.rs`
- `crates/roko-orchestrator/src/executor/state_machine.rs`
- `crates/roko-orchestrator/src/dag.rs`
- `crates/roko-orchestrator/src/merge_queue.rs`
- `docs/01-orchestration/02-unified-task-dag.md`
- `docs/01-orchestration/03-parallel-executor.md`
- `tmp/unified/03-GRAPH.md`
- `tmp/unified/04-EXECUTION.md`

### Target Files

- [ ] Update `crates/roko-cli/src/runner/event_loop.rs`.
- [ ] Update `crates/roko-cli/src/runner/state.rs`.
- [ ] Add `crates/roko-cli/src/runner/task_dag.rs` if DAG logic does not belong in event loop.
- [ ] Add `crates/roko-cli/src/runner/merge.rs` for merge action dispatch.
- [ ] Add tests under `crates/roko-cli/tests/` or focused module tests.

### Checklist

- [ ] Replace sentinel task resolution with a per-plan ready-task resolver.
- [ ] Track completed task ids per plan using stable task ids, not only counts.
- [ ] Track running task ids per plan so parallel execution does not double-dispatch.
- [ ] Add dependency failure behavior: blocked downstream tasks become skipped or failed with reason.
- [ ] Add a per-plan agent handle map before raising `max_concurrent_tasks` above 1.
- [ ] Add gate timeout using `tokio::time::timeout`.
- [ ] Add gate semaphore so expensive gates do not run unbounded.
- [ ] Add `RunVerify` implementation instead of auto-pass.
- [ ] Wire merge actions through `MergeQueue` instead of immediate `MergeSucceeded`.
- [ ] Persist task-level execution state after every task terminal event.

### Acceptance Criteria

- [ ] A plan with dependencies `A -> B -> C` runs in dependency order.
- [ ] Independent tasks can be marked ready without relying on string sorting.
- [ ] A failed prerequisite prevents dependent task dispatch.
- [ ] A gate timeout transitions to a classified failure.
- [ ] `ExecutorAction::RunVerify` performs real work or returns an explicit unsupported error; it never silently passes.
- [ ] Merge actions call a merge dispatcher or queue; they do not auto-succeed.

## Worker 9 Evidence Checklist (2026-04-26)

Implemented in the current tree:

- [x] `crates/roko-cli/src/runner/state.rs` tracks completed task ids per plan in `RunState::completed_tasks`.
- [x] `runner/event_loop.rs` excludes completed task ids when resolving the next ready task and uses `TaskDefinition::is_ready_with_plan_deps`.
- [x] `runner/gate_dispatch.rs` wraps gate execution in `tokio::time::timeout` and classifies timeouts as `gate-timeout:rung-*` failures.
- [x] `runner/gate_dispatch.rs` uses a static `GATE_SEMAPHORE` to prevent unbounded concurrent gate processes.
- [x] `runner/gate_dispatch.rs::run_verify_steps` executes declared `task.verify` commands through `ShellGate` in rung 0.
- [x] `runner/types.rs` and `runner/event_loop.rs` contain `RunnerFailureKind`, retryability checks, and retry backoff scheduling.
- [x] `crates/roko-orchestrator/src/merge_queue.rs` contains a real `MergeQueue` abstraction with conflict serialization tests.

Not yet implemented or not proven on the active path:

- [ ] `crates/roko-cli/src/runner/task_dag.rs` does not exist; sentinel tasks `"next"`, `"fix"`, and `"regen-verify"` are still resolved inline in `runner/event_loop.rs`.
- [ ] Ready task selection still sorts by task id, so independent-task scheduling is not yet a dedicated DAG policy.
- [ ] Runner execution is still effectively single-agent because `RunContext` has one `agent_handle` and `ExecutorConfig::max_concurrent_tasks` is set to `1`.
- [ ] Downstream dependency failure handling is not represented as explicit skipped/blocked task state.
- [ ] `ExecutorAction::MergeBranch` still auto-applies `ExecutorEvent::MergeSucceeded` in `runner/event_loop.rs`; it does not submit to `MergeQueue`.
- [ ] No current proof shows multi-task real-agent progression, retry loop depth, failed-prerequisite behavior, or resume after interruption.

## 2026-04-27 Deepening Pass - Source-Corrected Execution State

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: this section corrects stale task-DAG and merge claims while preserving the real open execution gaps: active-path DAG authority, single-agent state, dependency-failure propagation, retry proof, and resume proof. The score is not higher because this pass did not run multi-task provider execution.

This section supersedes the "Not yet implemented" list above where source has moved forward.

### Current Source Truth

- [x] `crates/roko-cli/src/runner/task_dag.rs` exists and defines `TaskDag`, `PlanDag`, `SkippedReason`, `DagConfig`, ready-task resolution, running-task tracking, downstream skip propagation, plan timeout skip propagation, and retry backoff helpers.
- [x] `TaskDag` has focused unit coverage for ready-task ordering, running/terminal suppression, downstream skip propagation, skipped reasons, plan timeout, and backoff.
- [x] `crates/roko-cli/src/runner/merge.rs` exists and defines `PlanMerger`, `GitMergeBackend`, `MergeBackend`, `RegressionGate`, conflict evidence, in-place mode handling, and merge regression gate behavior.
- [x] `runner/event_loop.rs` handles `ExecutorAction::MergeBranch` by constructing `PlanMerger` and submitting to the shared `MergeQueue`.
- [x] `runner/gate_dispatch.rs` wraps gate execution in timeouts and uses a semaphore for expensive gate work.
- [x] `runner/event_loop.rs` persists snapshots after key lifecycle transitions and merge queue decisions.
- [x] `runner/event_loop.rs` emits retry decisions and honors retry cooldown before spawning a retry.
- [x] `RunVerify` executes declared plan verify steps through gate dispatch, and only auto-passes when no verify steps are declared.

### Current Execution Gaps

- [ ] Active `SpawnAgent` still resolves sentinel task names inline in `runner/event_loop.rs`; it does not delegate to `TaskDag::next_ready_task`.
- [ ] Active ready selection still sorts by task id, which is deterministic but not a policy object that can express priority, critical path, or resource-aware scheduling.
- [ ] `TaskDag` tracks running tasks and skipped downstream tasks, but active event-loop state still primarily uses one global `agent_handle`.
- [ ] `ExecutorConfig::max_concurrent_tasks` is effectively capped by the runner's single active agent handle.
- [ ] Downstream dependency failure propagation exists in `TaskDag` but is not proven to drive executor events, TUI, persistence, and projection in active `plan run`.
- [ ] Plan-level timeout helpers exist in `TaskDag`, but active plan run wall-clock timeout proof is missing.
- [ ] Merge success and conflict paths are implemented in `PlanMerger`, but clean-clone proof must show real `git merge`, regression success, conflict evidence, and executor failure behavior.
- [ ] Retry classification and backoff events exist, but proof must show transient retry, permanent failure, gate timeout, and max-retry exhaustion on active runner.
- [ ] Proactive budget enforcement still needs per-turn/provider usage integration; a task can overshoot if provider usage arrives only at terminal completion.
- [ ] Resume proof must show task, gate, retry, and merge phases resume without duplicate effects.

### Target Execution Architecture

The active runner should use a strict reducer/effect model:

- [ ] `ExecutionState` owns task DAG status, active effects, retry timers, merge reservations, and plan deadlines.
- [ ] `ExecutionReducer` consumes events and returns effects; it does not spawn providers, run gates, or touch git.
- [ ] `EffectRunner` executes provider dispatch, gate command, verify command, merge, snapshot write, and projection publish effects.
- [ ] Every effect has an idempotency key: `run_id/plan_id/task_id/attempt/effect_kind`.
- [ ] Every effect emits started, completed, failed, and resumed events.
- [ ] Resume reconstructs active effects and decides whether each is completed, safe to retry, or must fail closed.

### Implementation Batches

#### EXE-01: Make TaskDag The Active Resolver

- [ ] Add `TaskDag` to `RunContext` or `RunState` as the only per-plan task scheduling owner.
- [ ] Replace inline sentinel resolution in `ExecutorAction::SpawnAgent` with `TaskDag::next_ready_task`.
- [ ] Mark tasks running before provider dispatch and clear running on every terminal path.
- [ ] Mark completed tasks through `TaskDag::mark_complete`.
- [ ] On terminal failure, call `TaskDag::mark_failed_blocking_downstream` and emit skipped events for downstream tasks.
- [ ] Persist `TaskDag` state or derive it losslessly from `run-state.json` and runner events.

#### EXE-02: Replace Global Agent Handle

- [ ] Replace `Option<AgentHandle>` with `HashMap<EffectId, AgentHandle>` or a typed `AgentProcessRegistry`.
- [ ] Keep provider concurrency controlled by policy, not by an accidental single global handle.
- [ ] Add per-plan and per-provider concurrency limits.
- [ ] Ensure cancellation kills all active handles and persists which effects were cancelled.
- [ ] Do not raise `max_concurrent_tasks` above 1 until handle ownership and persistence are complete.

#### EXE-03: Classify Failures And Retry Deterministically

- [ ] Normalize gate timeout, command failure, provider spawn failure, provider auth failure, rate limit, merge conflict, regression failure, and budget failure into `RunnerFailureKind`.
- [ ] Define retry policy per failure kind: retry, backoff, escalate model, replan, skip downstream, or fail closed.
- [ ] Emit `RunnerEvent::RetryDecision` with attempt, failure kind, backoff, and next action.
- [ ] Ensure retry cooldown survives resume.
- [ ] Prove max retry exhaustion produces one terminal task failure and one downstream skip cascade.

#### EXE-04: Finish Merge Proof

- [ ] Prove branch merge success with a real git branch and regression pass.
- [ ] Prove in-place mode is explicit and still runs regression.
- [ ] Prove merge conflict emits conflicted paths and aborts the merge.
- [ ] Prove merge regression failure leaves executor in a failed state, not `MergeSucceeded`.
- [ ] Persist merge attempt id, branch name, files touched, backend result, conflict evidence, and regression output.

#### EXE-05: Verify And Gate Semantics

- [ ] Keep `RunVerify` auto-pass only when the plan has no declared verify steps.
- [ ] Emit an explicit `verify.skipped.no_steps` event for that case.
- [ ] For declared verify steps, execute through the same gate/command service as task gates.
- [ ] Prove gate timeout, clippy skip, test skip, declared verify pass, and declared verify fail.

#### EXE-06: Budget And Timeout Enforcement

- [ ] Check budget before dispatch.
- [ ] Check projected budget while streaming token usage if provider supports usage deltas.
- [ ] Abort or stop the next turn when turn or plan budget is exceeded.
- [ ] Enforce plan wall-clock deadline through `TaskDag` and emit skipped tasks for remaining work.
- [ ] Persist budget and timeout decisions.

### Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/plan-execution-proof.json`:

```json
{
  "schema": "mori-diffs.plan-execution-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "cases": {
    "linear_dependencies": false,
    "independent_ready_tasks": false,
    "failed_prerequisite_skips_downstream": false,
    "single_agent_limit_documented": false,
    "multi_agent_when_enabled": false,
    "gate_timeout_classified": false,
    "retry_backoff_persisted": false,
    "max_retry_exhaustion": false,
    "plan_verify_pass": false,
    "plan_verify_fail": false,
    "merge_success": false,
    "merge_conflict": false,
    "merge_regression_failure": false,
    "resume_after_task": false,
    "resume_during_gate": false,
    "resume_during_merge": false
  },
  "events": [],
  "queries": [],
  "remaining_gaps": []
}
```

### No-Context Handoff Checklist

- [ ] Open `crates/roko-cli/src/runner/event_loop.rs` and find `ExecutorAction::SpawnAgent`.
- [ ] Replace inline sentinel resolution with `TaskDag`.
- [ ] Open `crates/roko-cli/src/runner/task_dag.rs` and wire running/completed/failed/skipped updates into active events.
- [ ] Open `crates/roko-cli/src/runner/merge.rs` and add real proof for success/conflict/regression failure.
- [ ] Replace the global `Option<AgentHandle>` only after adding effect ids and cancellation persistence.
- [ ] Generate `plan-execution-proof.json`.
- [ ] Update [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), [39-RUNNER-EXECUTION-POLICY-AUDIT.md](39-RUNNER-EXECUTION-POLICY-AUDIT.md), and [README.md](README.md).

### Archive Gate

- [ ] `TaskDag` is the active scheduling owner.
- [ ] Downstream skip behavior is active-path proven.
- [ ] Merge success, conflict, and regression failure are active-path proven.
- [ ] Retry and timeout decisions persist across resume.
- [ ] Multi-agent execution is either implemented or explicitly disabled by policy with no accidental hidden global state.
- [ ] `plan-execution-proof.json` exists and is linked from README.
