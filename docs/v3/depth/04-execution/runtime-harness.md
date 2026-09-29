# Runtime Harness

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 9.
> Preserves and updates content from v1 `01-orchestration/06-runtime-harness.md`.

---

## Overview

The runtime harness connects the pure Graph engine to effectful subsystems.
It owns the stateful resources -- provider dispatch, prompt cache, learning
stores, process supervisor, worktree manager, and merge queue -- and
injects them into the engine through `CellContext` and `CellResources`.

In the current architecture, `drive_controller()` in `roko-cli` serves as
the runtime harness for plan execution, replacing the v1 `PlanRunner`.

**Source:** `crates/roko-cli/src/commands/plan.rs`,
`crates/roko-execution/src/builder.rs`

---

## CellResources Injection

Cells receive shared service handles through `CellContext`, which carries
a `CellResources` bundle:

```rust
pub struct CellResources {
    pub dispatch_factory: Arc<DispatchFactory>,
    pub prompt_cache: Arc<PromptCacheHandle>,
    pub budget_tracker: Arc<BudgetTracker>,
    pub cancellation: CancellationToken,
    pub event_sink: Arc<dyn TelemetryEventSink>,
    pub workspace_path: PathBuf,
    pub worktree_path: Option<PathBuf>,
}
```

This decouples cells from the full `RuntimeServices` facade. Cells only
see the handles they need, and multiple graphs can share the same service
instances. The decoupling also makes cells testable in isolation -- tests
construct a `CellResources` with mock handles rather than building the
full `RuntimeServices`.

---

## Process Supervision

`ProcessSupervisor` (from `roko-runtime`) tracks spawned agent processes:

```rust
pub struct ProcessSupervisor {
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    cancel: CancellationToken,
}
```

When a plan completes or fails, the supervisor ensures all child processes
are terminated. The `GuaranteedFinallyController` invokes supervisor
shutdown as part of its cleanup guarantee.

### ProcessHandle

Each tracked process carries:

- Process ID (OS PID)
- Plan ID (owning plan)
- Task ID (originating task)
- Start timestamp
- Cancellation token (child of the root token)

### Shutdown sequence

1. Cancel the root cancellation token.
2. Send SIGTERM to all tracked processes.
3. Wait up to 10 seconds for graceful shutdown.
4. Send SIGKILL to any remaining processes.
5. Remove all handles from the tracking map.

This sequence ensures that even runaway agent processes (e.g., stuck in
an infinite tool loop) are cleaned up. The 10-second grace period allows
agents to finish writing partial outputs before forced termination.

---

## Budget Enforcement

The `BudgetTracker` tracks cost in microdollars (1 USD = 1,000,000
microdollars) using atomic operations for thread-safe concurrent access:

```rust
pub struct BudgetTracker {
    tokens_used: AtomicU64,
    cost_microdollars: AtomicU64,
    start_time: Instant,
    limits: BudgetLimits,
    breakdown: Mutex<Vec<NodeCost>>,
}
```

### Reservation lifecycle

Before each Activity node dispatch:

1. **Check limits.** If any limit (tokens, cost, wall-clock deadline)
   would be exceeded, return `BudgetExceeded` without executing.
2. **Reserve.** Atomically add the estimated cost to
   `cost_microdollars`. This prevents concurrent dispatches from
   independently exceeding the budget.
3. **Execute.** The cell runs and reports actual cost.
4. **Settle.** Adjust the reservation to actual cost (release any
   over-reservation or charge additional cost).

Atomic operations ensure that concurrent Activity executions in the same
wave see a consistent budget state without requiring a mutex on the hot
path.

### Persistence

On resume, the schema-v2 cost sidecar restores the exact budget state:

```rust
pub struct BudgetCheckpoint {
    pub tokens_used: u64,
    pub cost_microdollars: u64,
    pub elapsed_ms: u64,
    pub breakdown: Vec<NodeCostCheckpoint>,
}
```

Missing, corrupt, or mismatched cost state fails closed -- the engine
refuses to resume rather than risking over-spend.

---

## Learning Integration

After each agent dispatch, the runtime records learning data through
multiple channels:

### Efficiency events

```rust
pub struct AgentEfficiencyEvent {
    pub plan_id: String,
    pub task_id: String,
    pub role: String,
    pub model: String,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub gate_passed: bool,
}
```

Written to `.roko/learn/efficiency.jsonl` for cost tracking and model
routing feedback.

### Episode logging

Agent turns and gate results are recorded as episodes in
`.roko/episodes.jsonl`. Episodes feed the learning subsystem for model
routing, cascade adaptation, and playbook enrichment.

### Cascade router feedback

The `CascadeRouter` (LinUCB bandit) receives outcome feedback after each
dispatch:

- Gate passed: positive reward to the selected model arm
- Gate failed: negative reward
- Cost: incorporated into the reward function as a penalty

This updates the router's arm statistics for future model selection. Over
time, the router learns which models are effective for which task types,
reducing cost by routing simple tasks to cheaper models.

### Adaptive gate thresholds

`AdaptiveThresholds` tracks per-rung pass rates via exponential moving
average (EMA). The threshold data is persisted to
`.roko/learn/gate-thresholds.json` and informs retry budget decisions:
if a gate consistently fails for a particular kind of task, the system
can adjust model routing or task decomposition.

### Crate familiarity

`CrateFamiliarityTracker` records per-crate success rates. The
familiarity score feeds into the `CascadeRouter`'s context vector for
model selection -- unfamiliar crates get assigned more capable models.

---

## Conductor Integration

A background watcher tails recent signals and periodically runs the
conductor's anomaly detection watchers:

- **Cost overrun detection**: alerts when spending exceeds budget thresholds
- **Context window pressure**: monitors token usage relative to model limits
- **Silence detection**: alerts when agents are not producing output
- **Ghost turn detection**: alerts when agents loop without progress

The watcher runs every 30 seconds, reading the most recent 200 signals.
Alert signals are persisted back to the signal log for the controller to
act on.

---

## Worktree Integration

When a plan is dispatched:

1. `WorktreeManager::ensure_for_plan(plan_id)` creates or reuses a
   worktree.
2. The worktree path becomes the `worktree_path` in `CellResources`.
3. All agent processes execute in the worktree directory.
4. Gates run in the worktree directory.
5. On merge, the worktree branch enters the merge queue.

The worktree provides filesystem isolation so that concurrent plans
cannot conflict on files. Each worktree has its own branch, working
directory, and build artifacts.

---

## Merge Queue Integration

After a plan passes all gates, verification, and review:

1. The controller builds a `MergeRequest` with the plan's changed files.
2. `MergeQueue::enqueue()` adds it to the priority-ordered queue.
3. `MergeQueue::next_mergeable()` returns the highest-priority
   non-conflicting request.
4. The controller performs the git merge in the worktree.
5. On success, `mark_complete()` releases file locks.
6. On failure, `mark_failed()` re-enqueues with reduced priority.
7. Post-merge regression detection runs compile + test on the merged result.

---

## Reporting

After all plans reach terminal states, the controller produces a summary:

- Total plans: completed, failed, skipped
- Total agent dispatches and gate runs
- Total cost (USD) with per-plan and per-model breakdowns
- Critical path duration
- Per-plan status with failure reasons
- C-factor metric (collective intelligence measure)

This summary is displayed in the CLI and logged for post-mortem analysis.
The `roko show costs` command provides detailed cost breakdowns after
execution completes.
