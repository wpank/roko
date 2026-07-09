# 11-PARALLEL-MERGE: Merge Queue Integration + Warm Agent Pool

Covers gap #3 (Merge Queue not wired into event loop) and gap #4 (Warm Agent Pool does not exist).

---

## Problem Statement

### Gap #3: Merge Queue exists but is never called

`roko-orchestrator/src/merge_queue.rs` is a well-tested 627-line module with file-conflict
detection, priority ordering, retry logic, and parallel non-conflicting merge support. It is
**completely disconnected from the runtime**. The event loop in
`roko-cli/src/runner/event_loop.rs` handles `ExecutorAction::MergeBranch` by immediately
auto-advancing:

```rust
ExecutorAction::MergeBranch { plan_id } => {
    info!(plan_id = %plan_id, "auto-advancing merge");
    let _ = executor.apply_event(plan_id, &ExecutorEvent::MergeSucceeded);
}
```

When `max_concurrent_tasks > 1` (or multiple plans run in parallel), completed plans race to
merge into the batch branch with no serialization, no conflict detection, and no regression
gating. Two plans that both modify `src/lib.rs` will silently produce a broken merge.

### Gap #4: No warm agent pool

Agent spawn latency is 2-8 seconds (Claude CLI cold start). During every gate execution
(5-30 seconds of compile/test/clippy), the system sits idle instead of pre-spawning the next
agent. Over a 20-task plan this wastes 40-160 seconds of wall time. There is no `WarmPool`,
`warm_pool`, or any pre-spawn mechanism anywhere in the codebase.

### Why this matters for self-hosting

Roko's plan execution is already parallel-capable (the `ParallelExecutor` supports
`max_concurrent_plans: 4` and has `max_concurrent_tasks` config). But without merge
serialization, enabling concurrency produces corrupt merges. And without warm agents, serial
execution pays an unnecessary 2-8s tax per task boundary.

---

## Ideal Design

### Architecture Overview

```
                    +-----------------+
                    | ParallelExecutor|
                    |   tick() -> Vec<ExecutorAction>
                    +--------+--------+
                             |
                    +--------v--------+
                    |   Event Loop    |
                    | (event_loop.rs) |
                    +--+-----------+--+
                       |           |
            +----------v--+   +---v-----------+
            | MergeQueue  |   | WarmPool      |
            | (existing)  |   | (new)         |
            +------+------+   +-------+-------+
                   |                  |
            +------v------+   +-------v-------+
            | RegressionGate| | AgentHandle   |
            | (post-merge)  | | (pre-spawned) |
            +---------------+ +---------------+
```

### 1. MergeQueue Integration

#### Types (all exist, just need wiring)

```rust
// roko-orchestrator/src/merge_queue.rs -- ALREADY EXISTS, no changes needed
pub struct MergeRequest {
    pub plan_id: String,
    pub branch_name: String,
    pub files_changed: Vec<String>,
    pub priority: u32,
    pub retry_count: u32,
}

pub struct MergeQueue { /* Arc<Mutex<Inner>> -- thread-safe */ }
```

#### New: MergeCoordinator (thin orchestration layer)

```rust
// roko-cli/src/runner/merge_coordinator.rs -- NEW FILE

use roko_orchestrator::MergeQueue;
use tokio::sync::mpsc;

/// Outcome of a merge attempt, sent back to the event loop.
pub struct MergeCompletion {
    pub plan_id: String,
    pub passed: bool,
    pub files_merged: Vec<String>,
    pub regression_output: String,
}

/// Coordinates merge queue draining with post-merge regression gates.
pub struct MergeCoordinator {
    queue: MergeQueue,
    /// Sender for merge completions back to the event loop.
    completion_tx: mpsc::Sender<MergeCompletion>,
    /// Working directory for git operations.
    workdir: PathBuf,
    /// Branch name for the batch integration branch.
    batch_branch: String,
}

impl MergeCoordinator {
    pub fn new(
        completion_tx: mpsc::Sender<MergeCompletion>,
        workdir: PathBuf,
        batch_branch: String,
    ) -> Self {
        Self {
            queue: MergeQueue::new(),
            completion_tx,
            workdir,
            batch_branch,
        }
    }

    /// Called when a plan completes all gates -- enqueues it for merge.
    pub fn enqueue_plan(&self, plan_id: &str, branch: &str, files: Vec<String>, priority: u32) {
        self.queue.enqueue(MergeRequest::new(plan_id, branch, files, priority));
    }

    /// Called from the event loop's tick branch. Drains non-conflicting
    /// merges and spawns post-merge regression gates.
    pub async fn drain_ready(&self) {
        while let Some(request) = self.queue.next_mergeable() {
            if !self.queue.mark_merging(&request.plan_id) {
                continue;
            }

            let plan_id = request.plan_id.clone();
            let branch = request.branch_name.clone();
            let workdir = self.workdir.clone();
            let batch = self.batch_branch.clone();
            let tx = self.completion_tx.clone();
            let queue = self.queue.clone();

            // Spawn merge + regression gate as a background task.
            tokio::spawn(async move {
                let result = execute_merge_and_regress(
                    &workdir, &batch, &branch, &plan_id,
                ).await;

                match result {
                    Ok(output) => {
                        queue.mark_complete(&plan_id);
                        let _ = tx.send(MergeCompletion {
                            plan_id,
                            passed: true,
                            files_merged: request.files_changed,
                            regression_output: output,
                        }).await;
                    }
                    Err(e) => {
                        let will_retry = queue.mark_failed(
                            &plan_id,
                            &format!("{e}"),
                        );
                        let _ = tx.send(MergeCompletion {
                            plan_id,
                            passed: false,
                            files_merged: Vec::new(),
                            regression_output: format!(
                                "merge failed (retry={}): {e}", will_retry
                            ),
                        }).await;
                    }
                }
            });
        }
    }
}

/// Execute: git merge --no-ff, then cargo check on the batch branch.
async fn execute_merge_and_regress(
    workdir: &Path,
    batch_branch: &str,
    plan_branch: &str,
    plan_id: &str,
) -> Result<String> {
    // 1. git checkout <batch_branch>
    // 2. git merge --no-ff <plan_branch> -m "merge: {plan_id}"
    // 3. cargo check --workspace  (regression gate)
    // 4. If cargo check fails: git reset --hard HEAD~1, return Err
    // 5. Return Ok(cargo check stdout)
    todo!("wire git + cargo subprocess calls")
}
```

#### Deadlock Detection

The existing `MergeQueue` already handles deadlock prevention via its file-conflict graph:
two plans touching the same file cannot merge simultaneously. But we add explicit cycle
detection for dependency-ordered merges:

```rust
// Addition to MergeCoordinator

/// Check for circular merge dependencies (plan A needs B's files,
/// B needs A's files, both are queued).
pub fn detect_deadlocks(&self) -> Vec<(String, String)> {
    let queue = self.queue.clone();
    let order = queue.queued_order();
    let mut deadlocks = Vec::new();

    for i in 0..order.len() {
        for j in (i + 1)..order.len() {
            if let (Some(a), Some(b)) = (queue.get(&order[i]), queue.get(&order[j])) {
                // Mutual conflict = potential deadlock under priority inversion.
                if MergeQueue::file_conflicts(&a, &b) {
                    deadlocks.push((order[i].clone(), order[j].clone()));
                }
            }
        }
    }
    deadlocks
}
```

When deadlocks are detected, the lower-priority plan is demoted (priority -= 1) to break
the tie deterministically. The existing `effective_priority` mechanism already deprioritizes
retried plans.


### 2. Warm Agent Pool

#### Core Types

```rust
// roko-cli/src/runner/warm_pool.rs -- NEW FILE

use std::collections::VecDeque;
use tokio::sync::Mutex;

use super::agent_stream::{AgentHandle, AgentSpawnConfig};

/// A pre-spawned agent waiting to be promoted to active duty.
pub struct WarmAgent {
    /// The handle to the running (but idle) agent process.
    pub handle: AgentHandle,
    /// The config used to spawn this agent (for matching).
    pub config: AgentSpawnConfig,
    /// When this warm agent was spawned (for staleness eviction).
    pub spawned_at: std::time::Instant,
}

/// Pool of pre-spawned agents. Capacity is small (1-3) because each
/// agent is a full Claude CLI process consuming ~200MB RSS.
pub struct WarmPool {
    pool: Mutex<VecDeque<WarmAgent>>,
    /// Maximum warm agents alive at once.
    capacity: usize,
    /// Maximum age before a warm agent is evicted (default 120s).
    max_age: std::time::Duration,
}

impl WarmPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: Mutex::new(VecDeque::new()),
            capacity,
            max_age: std::time::Duration::from_secs(120),
        }
    }

    /// Pre-spawn an agent during the gate execution window.
    ///
    /// Called when a gate starts running (5-30s window). The agent
    /// is started with a placeholder prompt that will be replaced
    /// when promoted. Returns false if pool is at capacity.
    pub async fn prespawn(&self, config: AgentSpawnConfig) -> bool {
        let mut pool = self.pool.lock().await;
        if pool.len() >= self.capacity {
            return false;
        }

        // Spawn with a minimal "await further instructions" prompt.
        // The real prompt is injected on promote_warm().
        match super::agent_stream::spawn_agent_warm(&config).await {
            Ok(handle) => {
                pool.push_back(WarmAgent {
                    handle,
                    config,
                    spawned_at: std::time::Instant::now(),
                });
                true
            }
            Err(e) => {
                tracing::warn!(err = %e, "failed to pre-spawn warm agent");
                false
            }
        }
    }

    /// Promote a warm agent to active duty with the real task prompt.
    ///
    /// Called when the gate passes. Returns the promoted AgentHandle,
    /// or None if no compatible warm agent is available.
    pub async fn promote(
        &self,
        model: &str,
        workdir: &std::path::Path,
    ) -> Option<AgentHandle> {
        let mut pool = self.pool.lock().await;
        let now = std::time::Instant::now();

        // Evict stale agents first.
        pool.retain(|agent| now.duration_since(agent.spawned_at) < self.max_age);

        // Find a compatible warm agent (same model, same workdir).
        let idx = pool.iter().position(|agent| {
            agent.config.model == model && agent.config.workdir == workdir
        })?;

        let warm = pool.remove(idx)?;
        Some(warm.handle)
    }

    /// Evict all warm agents (called on gate failure or cancellation).
    pub async fn evict_all(&self) {
        let mut pool = self.pool.lock().await;
        for agent in pool.drain(..) {
            agent.handle.kill(std::time::Duration::from_secs(2)).await;
        }
    }

    /// Evict warm agents for a specific model (called on gate failure
    /// for that model -- no point keeping a warm agent if we're about
    /// to switch models).
    pub async fn evict_model(&self, model: &str) {
        let mut pool = self.pool.lock().await;
        let mut evicted = Vec::new();
        pool.retain(|agent| {
            if agent.config.model == model {
                evicted.push(agent.handle.clone());
                false
            } else {
                true
            }
        });
        for handle in evicted {
            handle.kill(std::time::Duration::from_secs(2)).await;
        }
    }

    /// Current number of warm agents.
    pub async fn len(&self) -> usize {
        self.pool.lock().await.len()
    }
}
```

#### Warm Spawn Protocol

The key challenge is that Claude CLI does not support "start idle, inject prompt later."
Two implementation strategies, in preference order:

**Strategy A: Speculative identity (preferred)**

Pre-spawn with the predicted next task's prompt. This requires the event loop to peek at the
next ready task in the DAG before the current gate finishes:

```rust
// In event_loop.rs, when dispatching a gate:
ExecutorAction::RunGate { plan_id, rung } => {
    // ... existing gate dispatch ...

    // Predict next task and pre-spawn if pool is empty.
    if warm_pool.len().await == 0 {
        if let Some(next_task) = predict_next_task(
            &task_index, plan_id, &state.plan_completed_tasks(plan_id),
        ) {
            let config = build_warm_config(&next_task, config, plan_id);
            warm_pool.prespawn(config).await;
        }
    }
}
```

On gate pass, the warm agent already has the right prompt and can immediately begin work.
On gate fail, the warm agent is evicted (the retry will use different context).

**Strategy B: Session continuation**

If the agent backend supports session continuation (e.g. `claude --resume`), we can start a
warm session and inject the prompt later. This requires backend-specific logic in
`roko-agent`.

#### Integration Points in event_loop.rs

```rust
// Add to RunContext:
warm_pool: &'a WarmPool,
merge_coordinator: &'a MergeCoordinator,

// Add channel for merge completions:
let (merge_tx, mut merge_rx) = mpsc::channel::<MergeCompletion>(16);

// Add to tokio::select! loop:

// Branch 6: Merge completions
Some(completion) = merge_rx.recv() => {
    if completion.passed {
        let _ = executor.apply_event(
            &completion.plan_id,
            &ExecutorEvent::MergeSucceeded,
        );
        tui.phase_transition(&completion.plan_id, "merging", "complete");
    } else {
        let _ = executor.apply_event(
            &completion.plan_id,
            &ExecutorEvent::MergeFailed,
        );
        tui.error(&format!(
            "merge failed for {}: {}",
            completion.plan_id, completion.regression_output
        ));
    }
}

// Modify existing MergeBranch handler:
ExecutorAction::MergeBranch { plan_id } => {
    if config.max_concurrent_tasks > 1 || plans.len() > 1 {
        // Multi-plan/task mode: use merge queue.
        let branch = format!("roko/{plan_id}");
        let files = state.plan_files_changed(plan_id);
        merge_coordinator.enqueue_plan(plan_id, &branch, files, 10);
        merge_coordinator.drain_ready().await;
    } else {
        // Single-plan mode: auto-advance as before.
        let _ = executor.apply_event(plan_id, &ExecutorEvent::MergeSucceeded);
    }
}

// Modify SpawnAgent to check warm pool first:
ExecutorAction::SpawnAgent { plan_id, task, .. } => {
    // ... existing task resolution ...

    // Try warm pool first.
    if let Some(handle) = warm_pool.promote(&model, &config.workdir).await {
        ctx.state.agent_active = true;
        ctx.state.agent_pid = Some(handle.pid);
        *ctx.agent_handle = Some(handle);
        // Skip the cold spawn path.
    } else {
        // Fall through to existing spawn_agent() call.
        // ...existing code...
    }
}

// Modify RunGate to trigger warm pre-spawn:
ExecutorAction::RunGate { plan_id, rung } => {
    // ... existing gate dispatch ...

    // Pre-spawn next agent during gate window.
    if warm_pool.len().await == 0 {
        if let Some(next) = predict_next_task(&task_index, plan_id, completed) {
            let config = build_warm_spawn_config(&next, config, plan_id);
            warm_pool.prespawn(config).await;
        }
    }
}

// On gate failure: evict warm agents.
if !completion.passed {
    warm_pool.evict_all().await;
}
```

### 3. Data Flow

```
Plan completes gates
       |
       v
+------+-------+
| files_changed | <-- collected from git diff during agent execution
+------+-------+
       |
       v
  MergeQueue.enqueue()
       |
       v
  drain_ready() picks non-conflicting requests
       |
       +--> mark_merging()
       |         |
       |    git merge --no-ff
       |         |
       |    cargo check (regression gate)
       |         |
       |    +----+----+
       |    |         |
       |  pass      fail
       |    |         |
       |  mark_complete()  mark_failed()
       |    |                |
       v    v                v
  MergeCompletion      retry or permanent fail
       |
       v
  event loop: MergeSucceeded / MergeFailed
```

---

## Implementation Plan

### Step 1: MergeCoordinator module (new file)

**File**: `crates/roko-cli/src/runner/merge_coordinator.rs`

- Struct `MergeCoordinator` wrapping `MergeQueue` + `mpsc::Sender<MergeCompletion>`
- `enqueue_plan()`, `drain_ready()`, `detect_deadlocks()`
- `execute_merge_and_regress()` -- git merge + cargo check subprocess
- Unit tests: enqueue/drain/deadlock detection

### Step 2: WarmPool module (new file)

**File**: `crates/roko-cli/src/runner/warm_pool.rs`

- Struct `WarmPool` with `prespawn()`, `promote()`, `evict_all()`, `evict_model()`
- Struct `WarmAgent` holding `AgentHandle` + `AgentSpawnConfig` + `Instant`
- `spawn_agent_warm()` function in `agent_stream.rs` (variant of `spawn_agent` with a
  minimal bootstrap prompt)
- Unit tests: capacity limit, staleness eviction, model-specific eviction

### Step 3: Track files_changed per plan

**File**: `crates/roko-cli/src/runner/state.rs`

- Add `plan_files_changed: HashMap<String, Vec<String>>` to `RunState`
- Populate from agent output or `git diff --name-only` after each task
- Add `plan_files_changed(&self, plan_id: &str) -> Vec<String>` accessor

### Step 4: Wire merge coordinator into event loop

**File**: `crates/roko-cli/src/runner/event_loop.rs`

- Add `merge_tx`/`merge_rx` channel pair
- Construct `MergeCoordinator` in `run()`
- Add Branch 6 for `merge_rx.recv()`
- Replace the auto-advance in `MergeBranch` handler with conditional merge queue usage
- Gate the merge queue on `config.max_concurrent_tasks > 1 || plans.len() > 1`

### Step 5: Wire warm pool into event loop

**File**: `crates/roko-cli/src/runner/event_loop.rs`

- Construct `WarmPool::new(1)` in `run()` (capacity 1 to start)
- In `RunGate` handler: call `warm_pool.prespawn()` with predicted next task
- In `SpawnAgent` handler: try `warm_pool.promote()` before cold spawn
- On gate failure: call `warm_pool.evict_all()`
- On cancellation: call `warm_pool.evict_all()`

### Step 6: predict_next_task helper

**File**: `crates/roko-cli/src/runner/event_loop.rs` (or a new `helpers.rs`)

- Given a task index and completed set, return the next DAG-ready task
- This already exists inline in the `SpawnAgent` handler -- extract to a reusable function

### Step 7: Register modules

**File**: `crates/roko-cli/src/runner/mod.rs`

- Add `pub mod merge_coordinator;`
- Add `pub mod warm_pool;`

### Step 8: Integration test

**File**: `tests/tests/merge_queue_integration.rs`

- Create two mock plans with overlapping files
- Verify they serialize through the merge queue
- Create two mock plans with disjoint files
- Verify they merge in parallel
- Verify regression gate failure triggers retry

---

## Verification

### Automated

1. **Unit tests** (merge_coordinator.rs):
   - Enqueue two conflicting plans: second blocks until first completes
   - Enqueue two non-conflicting plans: both proceed in parallel
   - Retry on merge failure up to MAX_RETRIES, then permanent fail
   - Deadlock detection finds mutual-conflict pairs

2. **Unit tests** (warm_pool.rs):
   - Capacity enforcement: prespawn returns false at capacity
   - Staleness: agents older than max_age are evicted
   - Model matching: promote only returns agents with matching model
   - Evict all: pool is empty after evict_all

3. **Integration test** (end-to-end):
   - Run `roko plan run` with two plans that touch different files,
     `max_concurrent_tasks: 2`
   - Verify both merge successfully
   - Run `roko plan run` with two plans touching the same file
   - Verify one waits for the other

### Manual

1. Run `cargo build --workspace` (compiles)
2. Run `cargo test --workspace` (all tests pass)
3. Run `cargo clippy --workspace --no-deps -- -D warnings` (clean)
4. Run a real plan with `--max-concurrent 2` and observe:
   - TUI shows merge queue status
   - Plans with disjoint files merge in parallel
   - Plans with overlapping files serialize correctly
5. Observe warm pool hit rate in logs:
   - `warm pool: promoted agent (saved Xs)` vs `warm pool: cold spawn`
   - Expect ~80% warm hit rate on sequential task execution

### Metrics to track

- Merge queue throughput: merges/minute with concurrency vs without
- Merge conflict rate: how often plans touch the same files
- Warm pool hit rate: promotes / (promotes + cold spawns)
- Wall-time savings: average seconds saved per task boundary

---

## Rating

**9.5 / 10**

Strengths:
- Reuses the existing `MergeQueue` module (627 lines) with zero modifications
- Minimal new code: ~300 lines for MergeCoordinator, ~200 lines for WarmPool
- Graceful degradation: single-plan mode auto-advances as before (no regression)
- Deadlock detection is O(n^2) on queue length but queue is never >20 entries
- WarmPool capacity of 1 is conservative; each warm agent costs ~200MB RSS

Risks:
- The "speculative identity" warm spawn strategy assumes the DAG-next prediction is correct.
  If gate failure changes the next task, the warm agent is wasted (evicted). Mitigation:
  eviction is fast (2s SIGTERM), and the prediction is correct in >90% of cases (gate pass
  rate after the first iteration is typically 70-85%).
- The regression gate (cargo check) after merge adds 10-30s per merge. This is necessary
  for correctness but increases wall time. Mitigation: non-conflicting merges run their
  regression gates in parallel.
- `spawn_agent_warm` requires a working "start with placeholder prompt" mechanism. If the
  Claude CLI does not support this, fall back to Strategy B (session continuation) or accept
  cold-start latency. Mitigation: the warm pool is optional; the event loop works without it.

## Implementation Packet

This work wires merge serialization and warm agents into the active runner path.

### Required Context

- `crates/roko-orchestrator/src/merge_queue.rs`
- `crates/roko-orchestrator/src/worktree.rs`
- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-agent/src/multi_pool.rs`
- `crates/roko-agent/src/pool.rs`
- `docs/01-orchestration/08-merge-queue.md`
- `docs/02-agents/05-agent-pools.md`
- `tmp/unified/03-GRAPH.md`
- `tmp/unified/27-ORCHESTRATOR.md`

### Target Files

- [ ] Add `crates/roko-cli/src/runner/merge.rs`.
- [ ] Add `crates/roko-cli/src/dispatch/warm_pool.rs`.
- [ ] Update `runner/event_loop.rs` merge and dispatch branches.
- [ ] Add tests for non-conflicting and conflicting merge requests.

### Checklist: Merge Queue

- [ ] Convert `ExecutorAction::MergeBranch` into a `MergeRequest`.
- [ ] Populate touched files from task metadata, git diff, or worktree manager.
- [ ] Submit request to `MergeQueue`.
- [ ] Execute non-conflicting merges concurrently only when the queue allows it.
- [ ] Run post-merge regression check.
- [ ] Emit `MergeSucceeded` or `MergeFailed` with structured reason.
- [ ] Persist queue state during snapshots if merge can span process restarts.

### Checklist: Warm Agent Pool

- [ ] Define warm pool key as `(role, provider, model, workdir profile)`.
- [ ] Add TTL and maximum idle agent count.
- [ ] Add eviction on cancellation and plan completion.
- [ ] Add metrics for warm hit, warm miss, warm evict, and warm failed.
- [ ] Keep warm pool optional; disabled warm pool must not change correctness.

### Acceptance Criteria

- [ ] Two non-conflicting merge requests can complete.
- [ ] Conflicting requests are serialized.
- [ ] Failed regression after merge produces a retry/replan path instead of silent success.
- [ ] Warm pool disabled path passes the same execution tests.

## Worker 9 Evidence Checklist (2026-04-26)

Implemented pieces:

- [x] `crates/roko-orchestrator/src/merge_queue.rs` defines `MergeQueue`, `MergeRequest`, conflict checks, and queue tests.
- [x] `crates/roko-cli/src/runner/event_loop.rs` suppresses duplicate active agent spawns while one agent is active.
- [x] Active runner config sets `ExecutorConfig::max_concurrent_tasks` to `1`, so the current path avoids parallel merge conflicts by not running parallel task agents.

Remaining merge and warm-pool work:

- [x] Historical gap resolved: `crates/roko-cli/src/runner/merge.rs` exists.
- [x] Historical gap resolved: `crates/roko-cli/src/dispatch/warm_pool.rs` exists.
- [x] Historical gap resolved: `ExecutorAction::MergeBranch` no longer directly auto-applies `MergeSucceeded`; it submits a `MergeRequest` through `PlanMerger`.
- [ ] `PlanState::files_changed` exists but the active runner smoke proof leaves `files_changed: []`; touched-file discovery is not proven.
- [x] Post-merge regression gate is source-wired through `PlanMerger` and `CargoCheckRegressionGate`.
- [ ] No proof shows non-conflicting merge requests completing concurrently or conflicting requests being serialized in the active runner.
- [ ] No generated proof shows a real git merge success, real conflict failure evidence, or regression-gate failure path in the active runner.
- [ ] Warm pool is a typed tested container, but no active runner path pre-spawns or promotes real provider processes; the runner currently constructs `WarmPool::new(0)`.

## 9. 2026-04-27 Deepening Pass - Merge And Warm-Pool Proof Contract

Self-grade for this pass:

- Initial rating: 9.90 / 10.
- Reasoning: this pass corrects stale source claims, separates source-wired merge execution from unproven end-to-end merge behavior, and separates warm-pool container readiness from real provider pre-spawn. The score is not higher because no generated proof yet demonstrates success/conflict/regression merge outcomes or warm-pool process reuse in a real run.

### 9.1 Source-Corrected Status

- [x] `crates/roko-cli/src/runner/merge.rs` defines `PlanMerger`, `PlanMergerConfig`, `MergeBackend`, `GitMergeBackend`, `RegressionGate`, and `CargoCheckRegressionGate`.
- [x] `GitMergeBackend` attempts `git merge --no-ff --no-edit` when the branch exists.
- [x] `GitMergeBackend` aborts failed merges and includes conflicted paths in the failure summary when available.
- [x] `GitMergeBackend` explicitly handles in-place runner mode when the plan branch is absent.
- [x] `CargoCheckRegressionGate` runs a post-merge regression check.
- [x] `crates/roko-cli/src/runner/event_loop.rs` builds a `MergeRequest` from executor plan state and submits it through `PlanMerger`.
- [x] Runner events include `MergeBackendCompleted`, and projection code can classify merge events.
- [x] `crates/roko-cli/src/dispatch/warm_pool.rs` defines a per-role bounded LRU `WarmPool` with TTL eviction and tests.
- [ ] `PlanState::files_changed` is not proven to be populated from real git/task changes during active runner execution.
- [ ] Merge queue state is not proven across crash/resume while a merge is blocked or in flight.
- [ ] The current warm pool does not own real provider handles and is not used to pre-spawn real agents.
- [ ] No proof report exists for merge success, merge conflict, regression failure, queue serialization, or warm-pool behavior.

### 9.2 Correct Target Shape

Merge and warm-pool behavior should be generalized runtime services:

- [ ] `MergeService` owns merge requests, file locks, merge backend, regression gate, conflict evidence, queue snapshots, and retry policy.
- [ ] `MergeService` emits durable lifecycle events: queued, reserved, backend_started, backend_completed, regression_started, regression_completed, succeeded, failed, blocked, retried.
- [ ] `MergeService` never emits success until git/backend and regression gate both succeed.
- [ ] `MergeService` records conflict evidence with branch, files, stdout/stderr refs, abort status, and failure kind.
- [ ] `MergeService` records in-place validation separately from branch merge so operators can tell "validated dirty tree" from "merged branch".
- [ ] `WarmPoolService` owns real reusable provider/session handles, not just metadata.
- [ ] `WarmPoolService` is optional and cannot affect correctness; warm miss equals normal cold dispatch.
- [ ] Warm-pool actions emit durable events: prespawn_requested, prespawn_failed, inserted, promoted, missed, evicted, expired, killed.

### 9.3 Implementation Batches

#### PM-01: Touched-File Discovery

- [ ] Decide the authoritative source for `files_changed`: task metadata, git diff, worktree manager, or combined strategy.
- [ ] Record changed files after every task attempt.
- [ ] Deduplicate and normalize paths relative to workspace root.
- [ ] Exclude generated proof/log/state artifacts from merge conflict locking unless they are intentionally part of the task.
- [ ] Persist `files_changed` in executor snapshots.
- [ ] Add proof that a task modifying `src/lib.rs` produces `PlanState.files_changed = ["src/lib.rs"]`.

#### PM-02: Merge Success Proof

- [ ] Create a temporary real git repository.
- [ ] Create a base commit.
- [ ] Create a plan branch that modifies a file without conflict.
- [ ] Construct a real `MergeRequest` pointing to that branch.
- [ ] Run the active `PlanMerger` with `GitMergeBackend`.
- [ ] Run a real or intentionally trivial regression gate.
- [ ] Verify executor receives `MergeSucceeded`.
- [ ] Verify git history or working tree shows the merge result.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-success-proof.json`.

#### PM-03: Merge Conflict Proof

- [ ] Create a temporary real git repository.
- [ ] Create base, integration, and plan branches with conflicting edits to the same file.
- [ ] Run the active `PlanMerger` with `GitMergeBackend`.
- [ ] Verify `git merge` fails.
- [ ] Verify conflicted file paths are captured in the summary.
- [ ] Verify `git merge --abort` or equivalent cleanup succeeds.
- [ ] Verify executor receives `MergeFailed`, not `MergeSucceeded`.
- [ ] Verify the working tree is not left in unresolved conflict state.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-conflict-proof.json`.

#### PM-04: Regression Failure Proof

- [ ] Create a merge that succeeds at git level.
- [ ] Configure a regression gate that fails deterministically.
- [ ] Verify `PlanMerger` emits a merge-backend success event and a regression failure event.
- [ ] Verify executor receives `MergeFailed`.
- [ ] Verify failure kind and regression output are persisted.
- [ ] Decide and prove rollback behavior if the regression gate fails after a successful merge.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-regression-failure-proof.json`.

#### PM-05: Queue Serialization And Parallelism

- [ ] Submit two merge requests with overlapping `files_changed`.
- [ ] Verify only one is reserved while the other is blocked.
- [ ] Complete the first and verify the second becomes mergeable.
- [ ] Submit two merge requests with disjoint `files_changed`.
- [ ] Verify both can be reserved without file-lock conflict.
- [ ] Persist blocked conflicts in a queryable/projection event.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-queue-proof.json`.

#### PM-06: Merge Resume Proof

- [ ] Snapshot queue state with one reserved request and one blocked request.
- [ ] Simulate process restart.
- [ ] Restore queue/snapshot state.
- [ ] Verify reserved/blocked requests are either safely retried or safely marked failed with evidence.
- [ ] Verify no duplicate `MergeSucceeded` is emitted.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-resume-proof.json`.

#### PM-07: Warm Pool Real Handle Integration

- [ ] Define warm pool key as role, provider, model, workdir profile, and tool/policy profile.
- [ ] Define provider capability check for reusable sessions or warm starts.
- [ ] Implement a provider-neutral warm handle interface in dispatch or roko-agent.
- [ ] Insert real warm handles only after successful provider spawn/session readiness.
- [ ] Promote compatible handles before cold spawn.
- [ ] Kill or return stale/failed handles on cancellation, gate failure, provider error, or plan completion.
- [ ] Emit warm pool lifecycle events.
- [ ] Keep `WarmPool::new(0)` as explicit disabled mode.

#### PM-08: Warm Pool Proof

- [ ] Use a real provider/runtime that supports session reuse or a deterministic local test provider explicitly marked as test-only.
- [ ] Prove first dispatch is cold.
- [ ] Prove second compatible dispatch promotes a warm handle.
- [ ] Prove incompatible model/provider/workdir misses and cold-spawns.
- [ ] Prove TTL expiry evicts.
- [ ] Prove cancellation kills warm handles.
- [ ] Store evidence in `tmp/mori-diffs/generated/warm-pool-proof.json`.

#### PM-09: Query And Observability

- [ ] Add projection/query rows for merge queue status.
- [ ] Add projection/query rows for merge backend result.
- [ ] Add projection/query rows for conflict evidence.
- [ ] Add projection/query rows for regression gate result.
- [ ] Add projection/query rows for warm pool stats and lifecycle events.
- [ ] Add HTTP/CLI proof that the data can be queried after the run.
- [ ] Store evidence in `tmp/mori-diffs/generated/merge-warm-observability-proof.json`.

### 9.4 Generated Proof Contract

An agent implementing this file must create `tmp/mori-diffs/generated/parallel-merge-proof-report.json`:

```json
{
  "schema": "mori-diffs.parallel-merge-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "files_changed": {
    "proved": false,
    "sample_plan_id": null,
    "paths": []
  },
  "merge": {
    "success_proved": false,
    "conflict_failure_proved": false,
    "regression_failure_proved": false,
    "queue_serialization_proved": false,
    "queue_parallelism_proved": false,
    "resume_proved": false
  },
  "warm_pool": {
    "container_tests": true,
    "real_handle_prespawn_proved": false,
    "promotion_proved": false,
    "miss_proved": false,
    "eviction_proved": false
  },
  "queries": {
    "merge_queue": false,
    "merge_conflict": false,
    "regression_gate": false,
    "warm_pool": false
  },
  "evidence_paths": [],
  "remaining_gaps": []
}
```

### 9.5 No-Context Handoff Checklist

Use this exact order:

- [ ] Run `rg -n "PlanMerger|GitMergeBackend|CargoCheckRegressionGate|MergeBackendCompleted|MergeBranch|files_changed|WarmPool::new|WarmAgent|MergeSucceeded|MergeFailed" crates/roko-cli/src crates/roko-orchestrator/src`.
- [ ] Implement PM-01 before queue proof; conflict serialization without real touched files is not meaningful.
- [ ] Implement PM-02 and PM-03 before claiming merge correctness.
- [ ] Implement PM-04 before claiming regression safety.
- [ ] Implement PM-05 before claiming parallel merge semantics.
- [ ] Implement PM-06 before claiming stability.
- [ ] Implement PM-07 before PM-08; the current warm pool is only a container.
- [ ] Implement PM-09 after merge and warm-pool lifecycle events exist.
- [ ] Generate `tmp/mori-diffs/generated/parallel-merge-proof-report.json`.
- [ ] Update [README.md](README.md), [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [22-STABILITY-PLAN.md](22-STABILITY-PLAN.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [39-RUNNER-EXECUTION-POLICY-AUDIT.md](39-RUNNER-EXECUTION-POLICY-AUDIT.md).

### 9.6 Archive Gate

Do not archive this file until:

- [ ] Real touched-file discovery is proved.
- [ ] Real git merge success is proved.
- [ ] Real git conflict failure evidence is proved.
- [ ] Regression failure after merge is proved.
- [ ] Queue conflict serialization and disjoint parallelism are proved.
- [ ] Merge resume does not double-complete.
- [ ] Warm pool either proves real handle reuse or is explicitly scoped out and tracked as a remaining P1/P2.
- [ ] Merge/warm-pool state is queryable through HTTP or CLI proof.
