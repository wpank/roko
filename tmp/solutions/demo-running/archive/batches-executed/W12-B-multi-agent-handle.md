# W12-B: Multi-Plan Concurrency -- Per-Plan Agent Handle, FailPlan Attribution, Per-Plan Iteration

**Priority**: P1 -- correctness (multi-plan execution fundamentally broken)
**Effort**: 2-3 hours
**Files to modify**: 2 files
**Dependencies**: W11-A (for `fatal_tx` on RunContext), W12-A (for `gate_sem` on RunContext)

## Cross-Batch Overlap Warning (W12 event_loop.rs)

All four W12 batches touch `event_loop.rs`. This batch (W12-B) has the most extensive changes. Overlaps:

- **Line 95 (RunContext struct)**: W12-A also modifies this struct (adds `gate_sem` field). Non-conflicting -- different fields. Apply both.
- **Line 974-989 (RunContext construction)**: W12-A also modifies this block (adds `gate_sem`). Non-conflicting -- different lines within the block.
- **Lines 1009/1039 (timeout branches)**: W12-C adds a `timed_out` guard to the same Branch 5 and post-select check. W12-B changes `&mut agent_handle` to `&mut agent_handles` in both. These are interacting: the agent must apply W12-B's variable rename AND W12-C's guard condition.
- **Line 1028 (cancellation branch)**: W12-B renames `stop_active_agent` to `stop_all_agents`. W12-D's budget enforcement (line 415) also calls this function. If W12-B is applied first, W12-D must use `stop_all_agents`; if not, it must use `stop_active_agent`. W12-D documents this dependency.
- **Line 3060 (`stop_active_agent`)**: W12-B replaces this function entirely. W12-D's budget enforcement references it.

**Apply order**: Apply W12-B before W12-C and W12-D. W12-C and W12-D both reference the variable names established by W12-B.

## Problem

Three related bugs prevent correct multi-plan concurrent execution:

1. **Single agent_handle slot**: `RunContext` holds `agent_handle: &'a mut Option<AgentHandle>` -- one slot. When a second plan's `SpawnAgent` fires, the guard `ctx.state.agent_active || ctx.agent_handle.is_some()` silently suppresses the spawn.

2. **FailPlan wrong plan attribution**: `FailPlan { plan_id, reason }` calls `ctx.state.task_failed()` which uses `ctx.state.plan_id` (the *current* plan on state) not the `plan_id` from the action.

3. **Shared iteration counter**: `state.iteration` is a single `u32`. In multi-plan runs, gate results for earlier plans carry the wrong attempt number.

## Exact Code to Change

### File 1: `crates/roko-cli/src/runner/event_loop.rs`

#### Change 1: Replace single `agent_handle` with per-plan HashMap in RunContext

**Find this code** (line 95, inside `RunContext` struct):
```rust
    agent_handle: &'a mut Option<AgentHandle>,
```

**Replace with:**
```rust
    agent_handles: &'a mut HashMap<String, AgentHandle>,
```

#### Change 2: Update variable declaration in `run()`

**Find this code** (line 313):
```rust
    let mut agent_handle: Option<AgentHandle> = None;
```

**Replace with:**
```rust
    let mut agent_handles: HashMap<String, AgentHandle> = HashMap::new();
```

#### Change 3: Update RunContext construction in `run()`

**Find this code** (line 974, the `RunContext` construction):
```rust
                    let mut ctx = RunContext {
                        executor: &mut executor,
                        task_index: &task_index,
                        skip_enrichment: &skip_enrichment,
                        config,
                        tui: &tui,
                        state: &mut state,
                        agent_handle: &mut agent_handle,
                        agent_tx: &agent_tx,
                        gate_tx: &gate_tx,
                        paths: &paths,
                        merge_queue: &merge_queue,
                        snapshot_writer: &snapshot_writer,
                        prompt_cache: &prompt_cache,
                        factory: &factory,
                    };
```

**Replace with:**
```rust
                    let mut ctx = RunContext {
                        executor: &mut executor,
                        task_index: &task_index,
                        skip_enrichment: &skip_enrichment,
                        config,
                        tui: &tui,
                        state: &mut state,
                        agent_handles: &mut agent_handles,
                        agent_tx: &agent_tx,
                        gate_tx: &gate_tx,
                        paths: &paths,
                        merge_queue: &merge_queue,
                        snapshot_writer: &snapshot_writer,
                        prompt_cache: &prompt_cache,
                        factory: &factory,
                    };
```

Note: If W11-A was applied first, `fatal_tx` is also present. If W12-A was applied, `gate_sem`
is also present. Keep those fields -- just change `agent_handle` to `agent_handles`.

#### Change 4: Update the agent-active guard in dispatch_action

**Find this code** (line 2015):
```rust
            if ctx.state.agent_active || ctx.agent_handle.is_some() {
                debug!(
                    plan_id = %plan_id,
                    task = %task_id,
                    current_plan = %ctx.state.plan_id,
                    current_task = %ctx.state.current_task,
                    "agent already active — suppressing duplicate spawn"
                );
                return;
            }
```

**Replace with:**
```rust
            if ctx.agent_handles.contains_key(plan_id.as_str()) {
                debug!(
                    plan_id = %plan_id,
                    task = %task_id,
                    "agent already active for this plan — suppressing duplicate spawn"
                );
                return;
            }
```

#### Change 5: Update agent handle storage after successful spawn

**Find this code** (line 2351):
```rust
                            *ctx.agent_handle = Some(handle);
```

**Replace with:**
```rust
                            ctx.agent_handles.insert(plan_id.to_string(), handle);
```

#### Change 6: Update `is_exited` handler

**Find this code** (line 493):
```rust
                if is_exited {
                    let exit_code = if let Some(handle) = agent_handle.take() {
                        let pid = handle.pid;
                        let code = handle.wait().await;
                        roko_agent::process::unregister_pid(pid);
                        code
                    } else if let AgentEvent::Exited { exit_code } = event {
                        exit_code
                    } else {
                        None
                    };
```

**Replace with:**
```rust
                if is_exited {
                    let exit_code = if let Some(handle) = agent_handles.remove(&state.plan_id) {
                        let pid = handle.pid;
                        let code = handle.wait().await;
                        roko_agent::process::unregister_pid(pid);
                        code
                    } else if let AgentEvent::Exited { exit_code } = event {
                        exit_code
                    } else {
                        None
                    };
```

#### Change 7: Update `save_agent_pids` in Branch 4

**Find this code** (line 1004):
```rust
                if let Some(ref handle) = agent_handle {
                    let _ = persist::save_agent_pids(&paths, &[handle.pid]);
                }
```

**Replace with:**
```rust
                {
                    let pids: Vec<u32> = agent_handles.values().map(|h| h.pid).collect();
                    if !pids.is_empty() {
                        let _ = persist::save_agent_pids(&paths, &pids);
                    }
                }
```

#### Change 8: Update Branch 5 (plan timeout) call

**Find this code** (line 1010):
```rust
            _ = &mut plan_timeout => {
                handle_plan_timeout(
                    &executor,
                    &plans,
                    &mut state,
                    &mut agent_handle,
                    &paths,
                    &merge_queue,
                    &tui,
                    config,
                    &snapshot_writer,
                )
                .await?;
            }
```

**Replace with:**
```rust
            _ = &mut plan_timeout => {
                handle_plan_timeout(
                    &executor,
                    &plans,
                    &mut state,
                    &mut agent_handles,
                    &paths,
                    &merge_queue,
                    &tui,
                    config,
                    &snapshot_writer,
                )
                .await?;
            }
```

#### Change 9: Update Branch 6 (cancellation) call

**Find this code** (line 1028):
```rust
                stop_active_agent(&mut agent_handle, &mut state, Duration::from_secs(3)).await;
```

**Replace with:**
```rust
                stop_all_agents(&mut agent_handles, &mut state, Duration::from_secs(3)).await;
```

#### Change 10: Update post-select timeout guard call

**Find this code** (line 1039):
```rust
        if tokio::time::Instant::now() >= plan_deadline {
            handle_plan_timeout(
                &executor,
                &plans,
                &mut state,
                &mut agent_handle,
                &paths,
                &merge_queue,
                &tui,
                config,
                &snapshot_writer,
            )
            .await?;
        }
```

**Replace with:**
```rust
        if tokio::time::Instant::now() >= plan_deadline {
            handle_plan_timeout(
                &executor,
                &plans,
                &mut state,
                &mut agent_handles,
                &paths,
                &merge_queue,
                &tui,
                config,
                &snapshot_writer,
            )
            .await?;
        }
```

#### Change 11: Update `handle_plan_timeout` signature and body

**Find this code** (line 3008):
```rust
async fn handle_plan_timeout(
    executor: &ParallelExecutor,
    plans: &[Plan],
    state: &mut RunState,
    agent_handle: &mut Option<AgentHandle>,
    paths: &PersistPaths,
    merge_queue: &MergeQueue,
    tui: &TuiBridge,
    config: &RunConfig,
    writer: &SnapshotWriter,
) -> Result<()> {
```

**Replace with:**
```rust
async fn handle_plan_timeout(
    executor: &ParallelExecutor,
    plans: &[Plan],
    state: &mut RunState,
    agent_handles: &mut HashMap<String, AgentHandle>,
    paths: &PersistPaths,
    merge_queue: &MergeQueue,
    tui: &TuiBridge,
    config: &RunConfig,
    writer: &SnapshotWriter,
) -> Result<()> {
```

**Find this code** (line 3028):
```rust
    stop_active_agent(agent_handle, state, Duration::from_secs(3)).await;
```

**Replace with:**
```rust
    stop_all_agents(agent_handles, state, Duration::from_secs(3)).await;
```

#### Change 12: Replace `stop_active_agent` with `stop_all_agents`

**Find this code** (line 3060):
```rust
async fn stop_active_agent(
    agent_handle: &mut Option<AgentHandle>,
    state: &mut RunState,
    grace: Duration,
) {
    if let Some(handle) = agent_handle.take() {
        let pid = handle.pid;
        handle.kill(grace).await;
        roko_agent::process::unregister_pid(pid);
    } else if let Some(pid) = state.agent_pid.take() {
        roko_agent::process::unregister_pid(pid);
    }
    state.agent_active = false;
    state.agent_pid = None;
    state.agent_turn_completed = false;
}
```

**Replace with:**
```rust
async fn stop_all_agents(
    agent_handles: &mut HashMap<String, AgentHandle>,
    state: &mut RunState,
    grace: Duration,
) {
    for (_plan_id, handle) in agent_handles.drain() {
        let pid = handle.pid;
        handle.kill(grace).await;
        roko_agent::process::unregister_pid(pid);
    }
    if let Some(pid) = state.agent_pid.take() {
        roko_agent::process::unregister_pid(pid);
    }
    state.agent_active = false;
    state.agent_pid = None;
    state.agent_turn_completed = false;
}
```

#### Change 13: Fix FailPlan wrong plan attribution

**Find this code** (line 2657):
```rust
        ExecutorAction::FailPlan { plan_id, reason } => {
            warn!(plan_id = %plan_id, reason = %reason, "plan failed");
            ctx.state.task_failed();
            ctx.tui
                .task_completed(&ctx.state.plan_id, &ctx.state.current_task, "failed");
            ctx.tui.plan_completed(plan_id, false);
```

**Replace with:**
```rust
        ExecutorAction::FailPlan { plan_id, reason } => {
            warn!(plan_id = %plan_id, reason = %reason, "plan failed");
            ctx.state.tasks_failed += 1;
            ctx.state.roll_into_totals();
            ctx.tui
                .task_completed(plan_id, &ctx.state.current_task, "failed");
            ctx.tui.plan_completed(plan_id, false);
```

Note: `roll_into_totals` is `fn roll_into_totals(&mut self)` -- check if it is `pub`. If not,
make it `pub` in `state.rs`. Currently at line 443 it is private (`fn roll_into_totals`).
Change it to `pub fn roll_into_totals` in state.rs.

### File 2: `crates/roko-cli/src/runner/state.rs`

#### Change 14: Make `roll_into_totals` public

**Find this code** (line 443):
```rust
    fn roll_into_totals(&mut self) {
```

**Replace with:**
```rust
    pub fn roll_into_totals(&mut self) {
```

#### Change 15: Replace single `iteration` with per-task HashMap

**Find this code** (line 71):
```rust
    /// Iteration count for the current task (retries).
    pub iteration: u32,
```

**Replace with:**
```rust
    /// Iteration count per task, keyed by `"{plan_id}:{task_id}"`.
    pub iterations: HashMap<String, u32>,
```

#### Change 16: Update `RunState::new` initialization

**Find this code** (line 156):
```rust
            iteration: 0,
```

**Replace with:**
```rust
            iterations: HashMap::new(),
```

#### Change 17: Update `reset_for_task`

**Find this code** (line 407):
```rust
        self.iteration = 0;
```

**Replace with (remove the line entirely or replace with a comment):**
```rust
        // iteration is per-task in self.iterations, set from executor state
```

#### Change 18: Update `current_attempt_ref`

**Find this code** (line 184):
```rust
    pub fn current_attempt_ref(&self) -> TaskAttemptRef {
        TaskAttemptRef::new(
            self.plan_id.clone(),
            self.current_task.clone(),
            self.iteration.max(1),
        )
    }
```

**Replace with:**
```rust
    pub fn current_attempt_ref(&self) -> TaskAttemptRef {
        let key = format!("{}:{}", self.plan_id, self.current_task);
        let iteration = self.iterations.get(&key).copied().unwrap_or(1);
        TaskAttemptRef::new(
            self.plan_id.clone(),
            self.current_task.clone(),
            iteration.max(1),
        )
    }
```

#### Change 19: Add helper method for iteration access

Add after `current_attempt_ref`:
```rust
    /// Get the iteration count for a specific plan/task pair.
    pub fn iteration_for(&self, plan_id: &str, task_id: &str) -> u32 {
        let key = format!("{plan_id}:{task_id}");
        self.iterations.get(&key).copied().unwrap_or(1)
    }

    /// Set the iteration count for a specific plan/task pair.
    pub fn set_iteration(&mut self, plan_id: &str, task_id: &str, value: u32) {
        let key = format!("{plan_id}:{task_id}");
        self.iterations.insert(key, value);
    }
```

#### Change 20: Update all `state.iteration` references in event_loop.rs

There are many references to `state.iteration` in `event_loop.rs`. For each one:

- **Line 574** (`state.iteration.max(1)` in gate completion handler):
  ```rust
  // Before:
  state.iteration.max(1),
  // After:
  state.iteration_for(&completion.plan_id, &completion.task_id),
  ```

- **Line 804** (`state.iteration = ps.iteration;`):
  ```rust
  // Before:
  state.iteration = ps.iteration;
  // After:
  state.set_iteration(&completion.plan_id, &completion.task_id, ps.iteration);
  ```

- **Line 834** (`next_attempt.unwrap_or(state.iteration + 1)`):
  ```rust
  // Before:
  next_attempt.unwrap_or(state.iteration + 1),
  // After:
  let cur_iter = state.iteration_for(&completion.plan_id, &completion.task_id);
  next_attempt.unwrap_or(cur_iter + 1),
  ```

- **Line 848** (`let attempt_num = state.iteration + 1;`):
  ```rust
  // Before:
  let attempt_num = state.iteration + 1;
  // After:
  let attempt_num = state.iteration_for(&completion.plan_id, &completion.task_id) + 1;
  ```

- **Line 851** (`if state.iteration >= 3`):
  ```rust
  // Before:
  let strategy_hint = if state.iteration >= 3 {
  // After:
  let strategy_hint = if state.iteration_for(&completion.plan_id, &completion.task_id) >= 3 {
  ```

- **Line 1161** (`state.set_retry_backoff(..., state.iteration)`):
  ```rust
  // Before:
  state.set_retry_backoff(&completion.plan_id, failure_kind, state.iteration);
  // After:
  let iter = state.iteration_for(&completion.plan_id, &completion.task_id);
  state.set_retry_backoff(&completion.plan_id, failure_kind, iter);
  ```

- **Line 1170** (`state.iteration.max(1)`):
  ```rust
  // Before:
  state.iteration.max(1),
  // After:
  state.iteration_for(&completion.plan_id, &completion.task_id),
  ```

- **Line 1172** (`state.iteration.saturating_add(1).max(1)`):
  ```rust
  // Before:
  let next_attempt = Some(state.iteration.saturating_add(1).max(1));
  // After:
  let cur_iter = state.iteration_for(&completion.plan_id, &completion.task_id);
  let next_attempt = Some(cur_iter.saturating_add(1).max(1));
  ```

- **Line 1334** (`"attempt": state.iteration.max(1)`):
  ```rust
  // Before:
  "attempt": state.iteration.max(1),
  // After:
  "attempt": state.iteration_for(&state.plan_id, &state.current_task),
  ```

- **Line 2083** (`ctx.state.iteration = attempt_num`):
  ```rust
  // Before:
  ctx.state.iteration = attempt_num;
  // After:
  ctx.state.set_iteration(plan_id, &task_id, attempt_num);
  ```

- **Line 2502** (`ctx.state.iteration.max(1)`):
  ```rust
  // Before:
  TaskAttemptRef::new(plan_id.clone(), task_id.clone(), ctx.state.iteration.max(1));
  // After:
  TaskAttemptRef::new(plan_id.clone(), task_id.clone(), ctx.state.iteration_for(plan_id, &task_id));
  ```

- **Line 2608** (`ctx.state.iteration.max(1)` in RunVerify):
  ```rust
  // Before:
  TaskAttemptRef::new(plan_id.clone(), "plan-verify", ctx.state.iteration.max(1));
  // After:
  TaskAttemptRef::new(plan_id.clone(), "plan-verify", ctx.state.iteration_for(plan_id, "plan-verify"));
  ```

Use `grep -n 'state\.iteration[^s]' crates/roko-cli/src/runner/event_loop.rs` to find all sites.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check:
cargo check -p roko-cli 2>&1 | head -30

# Verify no remaining single agent_handle references:
grep -n 'agent_handle[^s]' crates/roko-cli/src/runner/event_loop.rs | grep -v '//'
# Should be empty or only in comments

# Verify no remaining single iteration field:
grep -n 'state\.iteration[^s]' crates/roko-cli/src/runner/state.rs
grep -n 'state\.iteration[^s]' crates/roko-cli/src/runner/event_loop.rs
# Should be empty
```

## Agent Prompt

```
Fix three multi-plan concurrency bugs in the roko runner.

## Context

Files to modify:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs`

Read the full batch spec at `/Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-B-multi-agent-handle.md` for exact find/replace blocks.

## Summary of changes

### 1. Per-plan agent handles
Replace `agent_handle: &'a mut Option<AgentHandle>` with `agent_handles: &'a mut HashMap<String, AgentHandle>` in `RunContext`. Update:
- Variable declaration in `run()`: `let mut agent_handles: HashMap<String, AgentHandle> = HashMap::new();`
- `RunContext` construction: `agent_handles: &mut agent_handles,`
- Agent-active guard (line ~2015): use `ctx.agent_handles.contains_key(plan_id.as_str())`
- Handle storage (line ~2351): `ctx.agent_handles.insert(plan_id.to_string(), handle);`
- `is_exited` handler (line ~494): `agent_handles.remove(&state.plan_id)`
- `save_agent_pids` (line ~1004): collect all PIDs from `agent_handles.values()`
- Replace `stop_active_agent` with `stop_all_agents` that drains the HashMap
- Update `handle_plan_timeout` to accept `&mut HashMap<String, AgentHandle>`
- Update all call sites passing `&mut agent_handle` to `&mut agent_handles`

### 2. FailPlan attribution (line ~2657)
Replace `ctx.state.task_failed()` with `ctx.state.tasks_failed += 1; ctx.state.roll_into_totals();`.
Replace `&ctx.state.plan_id` with `plan_id` in the `tui.task_completed()` call.
Make `roll_into_totals` `pub` in state.rs.

### 3. Per-task iteration counter
Replace `pub iteration: u32` with `pub iterations: HashMap<String, u32>` in `RunState`.
Add `iteration_for(&self, plan_id, task_id) -> u32` and `set_iteration(&mut self, plan_id, task_id, value)` helpers.
Update ALL references to `state.iteration` in event_loop.rs to use the helpers.

Run `cargo check -p roko-cli` to verify. Do NOT run full test suite (deferred).
```

## Commit

This batch is committed with all Wave 12 batches together. Do not commit individually.

## Checklist

- [ ] `agent_handle` field renamed to `agent_handles: HashMap<String, AgentHandle>` in RunContext
- [ ] Variable in `run()` changed to `HashMap`
- [ ] RunContext construction updated
- [ ] Agent-active guard uses `contains_key`
- [ ] Handle stored by plan_id after spawn
- [ ] `is_exited` handler removes by plan_id
- [ ] `save_agent_pids` collects from all handles
- [ ] `stop_active_agent` replaced with `stop_all_agents`
- [ ] `handle_plan_timeout` signature updated
- [ ] All call sites updated for new parameter types
- [ ] `roll_into_totals` made `pub` in state.rs
- [ ] `FailPlan` uses action's `plan_id`, not `state.plan_id`
- [ ] `iteration: u32` replaced with `iterations: HashMap<String, u32>` in RunState
- [ ] `RunState::new` initializes `iterations` as empty HashMap
- [ ] `current_attempt_ref` reads from `iterations` map
- [ ] `iteration_for` and `set_iteration` helpers added
- [ ] All `state.iteration` read/write sites updated (13+ sites in event_loop.rs)
- [ ] `cargo check -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. 1 issue fixed: added cross-batch overlap warning header documenting interactions with W12-A (RunContext struct), W12-C (timeout branches), and W12-D (budget enforcement / stop function). All code snippets verified against source -- exact matches confirmed. Line numbers accurate. HashMap already imported in both event_loop.rs and state.rs. `PlanState.iteration` (line 2080, executor state) correctly left untouched while `RunState.iteration` (line 2083) is updated. All 13 `state.iteration` reference sites in event_loop.rs accounted for.
