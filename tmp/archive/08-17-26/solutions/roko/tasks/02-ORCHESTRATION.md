# Orchestration Rearchitecture: Task Breakdown

> Converge 3 runtimes into 1 WorkflowEngine. Decompose the 22K LOC god object.
> Wire parallel execution, worktree isolation, wave gating, cumulative context
> handoff, failure recovery, and resume. 28 tasks across 9 phases.
>
> Sources: `impl/02-ORCHESTRATION.md`, `17-ORCH-{AUDIT,GOALS,ISSUES,PLAN,PATTERNS}.md`,
> `22-RUNNER-LESSONS.md`, codebase analysis

---

## Overview

The orchestration subsystem currently has three separate runtimes:

| Runtime | Location | LOC | Status |
|---|---|---|---|
| orchestrate.rs | `crates/roko-cli/src/orchestrate.rs` | 22,522 | Legacy dead code monolith |
| Runner v2 | `crates/roko-cli/src/runner/` (15 files) | ~9,300 | Active (CLI `roko plan run`) |
| WorkflowEngine | `crates/roko-runtime/src/workflow_engine.rs` + siblings | ~4,025 | Active (`roko run`, `roko chat`, ACP) |

**Target state**: WorkflowEngine absorbs Runner v2's operational features (worktree isolation, merge queue, parallel DAG execution, streaming, resume) and orchestrate.rs's learning features (knowledge routing, episodes, playbooks, error patterns, custody, skills). One runtime, one state machine, one dispatch path. orchestrate.rs is deleted.

**Key bottleneck**: `max_concurrent_tasks: 1` in Runner v2's event loop (`crates/roko-cli/src/runner/event_loop.rs:115`) despite full DAG/wave infrastructure in `crates/roko-orchestrator/src/dag.rs` (2,557 LOC). Parallel execution requires worktree isolation first.

**Key lesson from mega-parity runner** (195 batches, 6 hours, 177K LOC): Parallel execution with worktree isolation and wave gating achieves 10-15x speedup. Cumulative context ("what changed before you") reduces merge conflicts from ~50% to ~30%. Agents write code in 1-5 min without builds; per-task compilation adds 15-40 min overhead.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-GOD | 22K LOC god file | `crates/roko-cli/src/orchestrate.rs` | Critical |
| AP-4DISP | Four separate dispatch implementations | ACP `runner.rs`, Runner v2 `dispatch/mod.rs`, orchestrate.rs `dispatch_agent()`, EffectDriver `spawn_agent()` | Critical |
| AP-2SM | Two incompatible state machines for same concept | `PipelineStateV2` (10 states) in `crates/roko-runtime/src/pipeline_state.rs` vs `PlanPhase` (14 states) in `crates/roko-core/src/phase.rs` | High |
| AP-SERIAL | Serial default despite full DAG infra | `max_concurrent_tasks: 1` in `crates/roko-cli/src/runner/event_loop.rs:115` | High |
| AP-RUNG | Gate rung mapping duplicated | `rung_for_gate_name()` in `crates/roko-runtime/src/effect_driver.rs:645-656` duplicates `roko-gate` mapping | Medium |
| AP-AFFECT | Affect policy wired but only default used | Runner v2 passes `DaimonPolicy::default()`, EffectDriver supports full `AffectPolicy` trait but gets neutral modulation | Medium |
| AP-NOCHECK | No checkpoint for TaskScheduler state | `crates/roko-runtime/src/task_scheduler.rs` `TaskStatus` lacks Serialize/Deserialize; crash loses all task progress | High |

---

## Phase 1: Worktree Integration (Foundation)

Everything parallel depends on isolation. No parallel execution is safe without per-task worktrees.

### Task 2.1: Add WorktreeManager and MergeQueue to EffectServices
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/Cargo.toml`
**Depends On**: none

#### Context
`EffectServices` (defined at `crates/roko-runtime/src/effect_driver.rs:37-50`) is the service injection point for the WorkflowEngine. It currently holds five services: `default_model`, `model_caller`, `prompt_assembler`, `feedback_sink`, `gate_runner`, and optional `affect_policy`. It has no reference to worktree or merge infrastructure.

`WorktreeManager` exists at `crates/roko-orchestrator/src/worktree.rs` (1,203 LOC) and is fully functional: `create_for_plan()`, `remove()`, `touch()`, `reclaim_idle()`, `health()`, `clear_stale_locks()`, `prune()`. It is re-exported from `crates/roko-orchestrator/src/lib.rs:105-108`.

`MergeQueue` exists at `crates/roko-orchestrator/src/merge_queue.rs` (924 LOC) with file-conflict-aware serialization: `enqueue()`, `dequeue()`, `complete()`, `fail()`. Re-exported from `crates/roko-orchestrator/src/lib.rs:79-82`.

`roko-runtime/Cargo.toml` currently depends on `roko-core` and `roko-primitives` but NOT on `roko-orchestrator`. Adding this dependency is required.

#### Implementation Steps
1. Add `roko-orchestrator = { path = "../roko-orchestrator" }` to `crates/roko-runtime/Cargo.toml` under `[dependencies]`.
2. Add two optional fields to `EffectServices` in `crates/roko-runtime/src/effect_driver.rs`:
   ```rust
   pub worktree_manager: Option<Arc<roko_orchestrator::WorktreeManager>>,
   pub merge_queue: Option<Arc<roko_orchestrator::MergeQueue>>,
   ```
3. Update the `EffectDriver::new()` constructor -- no behavior change, just pass-through.
4. Update all existing call sites that construct `EffectServices` (search for `EffectServices {` in `crates/roko-runtime/src/` and `crates/roko-cli/src/`) to add `worktree_manager: None, merge_queue: None`.
5. Update the test mock `EffectServices` in `crates/roko-runtime/src/effect_driver.rs:822-831` to include the new fields as `None`.

#### Design Guidance
Use `Option<Arc<T>>` for the worktree/merge fields so all existing single-task workflows are unaffected. When `worktree_manager` is `None`, the EffectDriver uses `self.workdir` as-is. When `Some`, it allocates a worktree per task via `create_for_plan()`. This is the zero-regression pattern: existing callers pass `None` and behavior is identical.

#### Verification Criteria
- [ ] `cargo check -p roko-runtime` compiles without errors
- [ ] `cargo test -p roko-runtime` passes (all existing tests unchanged)
- [ ] `EffectServices` has `worktree_manager` and `merge_queue` fields
- [ ] No breaking changes to any call site constructing `EffectServices`

---

### Task 2.2: Add Worktree Allocation to EffectDriver Agent Spawn
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.1

#### Context
`EffectDriver::spawn_agent()` (at line 87-275) currently operates on `self.workdir` for all agent calls and git operations. When worktree isolation is active, each agent needs its own worktree directory. The `spawn_agent` method needs a `task_id` parameter (or an overloaded variant) so it can request a worktree from `WorktreeManager` and set the agent's working directory to the worktree path.

The current signature is:
```rust
pub async fn spawn_agent(&self, role: &str, user_prompt: &str, context: Option<&str>) -> PipelineInput
```

The `PromptAssembler::assemble()` call at line 121-132 passes `workdir: Some(self.workdir.clone())` in the `PromptSpec`. The git operations (diff at line 621-636, commit at line 340-421) use `self.workdir`. All of these must use the task-specific worktree path when isolation is active.

#### Implementation Steps
1. Add a new method `spawn_agent_in_worktree()` that accepts an additional `task_id: &str` and optional `worktree_path: Option<PathBuf>` parameter.
2. When `worktree_path` is `Some`, use it instead of `self.workdir` for:
   - `PromptSpec::workdir` in the prompt assembly call
   - `count_changed_files()` call after agent completion
   - Any future git operations
3. Keep the existing `spawn_agent()` method as a backward-compatible wrapper that calls `spawn_agent_in_worktree()` with `task_id: "default"` and `worktree_path: None`.
4. If `self.services.worktree_manager` is `Some` and `worktree_path` is `None`, allocate a new worktree via `worktree_manager.create_for_plan(task_id)`. Store the returned `WorktreeHandle::path` and use it.
5. After agent completion, touch the worktree handle via `worktree_manager.touch(task_id)` to prevent idle reclamation.
6. Add cleanup logic: if the worktree was created by this call and the agent failed, keep the worktree for debugging (do not auto-remove).

#### Design Guidance
The worktree lifecycle should be managed by the caller (WorkflowEngine run loop), not by `spawn_agent` itself. `spawn_agent_in_worktree` should accept the path, not manage creation/deletion. This keeps the EffectDriver stateless with respect to worktree lifecycle.

#### Verification Criteria
- [ ] `spawn_agent()` backward-compatible -- existing tests pass unchanged
- [ ] New `spawn_agent_in_worktree()` method accepts and uses a custom workdir
- [ ] `PromptSpec::workdir` reflects the worktree path, not the global workdir
- [ ] Unit test: `spawn_agent_in_worktree` with a custom tempdir as worktree path produces `AgentCompleted` with correct workdir context

---

### Task 2.3: Concurrent Task Dispatch in WorkflowEngine
**Priority**: P0
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
**Depends On**: Task 2.2

#### Context
`WorkflowEngine::run_with_cancel()` (in `crates/roko-runtime/src/workflow_engine.rs`, starting at line ~143) currently runs a serial state machine loop: `step() -> execute action -> feed result back -> step() -> ...`. This works for single-prompt workflows (express/standard/full) but not for multi-task plans.

The `TaskScheduler` (at `crates/roko-runtime/src/task_scheduler.rs`) already provides `next_batch()` which returns multiple ready tasks respecting `max_parallel` and file-exclusion constraints. But there is no code in WorkflowEngine that calls `next_batch()` and dispatches tasks concurrently.

The WorkflowEngine needs a new run mode for multi-task plans that:
1. Calls `TaskScheduler::next_batch()` to get dispatchable tasks
2. Spawns agents in parallel via `tokio::task::JoinSet`
3. Collects results as they complete
4. Updates `TaskScheduler` status (completed/failed)
5. Loops until `TaskScheduler::is_done()`

#### Implementation Steps
1. Add `max_parallel_tasks: usize` field to `WorkflowRunConfig` (default: 1 for backward compat).
2. Add a `run_plan()` method to `WorkflowEngine` that accepts a `Vec<SchedulableTask>` + `WorkflowRunConfig`.
3. In `run_plan()`, create a `TaskScheduler` with the tasks and `max_parallel_tasks`.
4. Main loop:
   ```rust
   let mut join_set = JoinSet::new();
   loop {
       if scheduler.is_done() { break; }
       if cancel.is_cancelled() { break; }
       let batch = scheduler.next_batch();
       for task_id in batch {
           scheduler.mark_running(task_id);
           let driver = /* clone/arc driver */;
           let worktree_path = /* allocate from WorktreeManager if available */;
           join_set.spawn(async move {
               (task_id, driver.spawn_agent_in_worktree(task_id, ..., worktree_path).await)
           });
       }
       // Wait for at least one to complete
       if let Some(result) = join_set.join_next().await {
           let (task_id, input) = result??;
           match input {
               PipelineInput::AgentCompleted { .. } => {
                   // Run gates, handle merge, mark completed
                   scheduler.mark_completed(&task_id);
               }
               PipelineInput::AgentFailed { error } => {
                   scheduler.mark_failed(&task_id, error);
               }
           }
       }
   }
   ```
5. When `max_parallel_tasks == 1`, the behavior degenerates to serial execution (same as today).

#### Design Guidance
Use `tokio::task::JoinSet` for the concurrent dispatch, not manual `tokio::spawn` with a `Vec<JoinHandle>`. JoinSet provides ordered completion and cancellation. The EffectDriver must be `Arc`-wrapped for concurrent use -- its `services` field already uses `Arc` for all trait objects, but `feedback_totals` uses `tokio::sync::Mutex` which is safe for concurrent access.

#### Verification Criteria
- [ ] New `run_plan()` method dispatches tasks from `TaskScheduler::next_batch()` concurrently
- [ ] Serial execution (max_parallel=1) produces identical results to current behavior
- [ ] Integration test: 3 independent tasks complete in parallel (elapsed < 3x single-task time)
- [ ] File-exclusion constraint: tasks with overlapping files are serialized

---

### Task 2.4: Post-Task Merge via MergeQueue
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.3

#### Context
After a task completes in its worktree, its changes need to be merged into the integration branch. The EffectDriver currently commits directly via `git add -A && git commit` in `commit()` (at `effect_driver.rs:340-421`) with no merge queue coordination.

`MergeQueue` at `crates/roko-orchestrator/src/merge_queue.rs` provides file-conflict-aware serialized merging. `PostMergeRunner` at `crates/roko-orchestrator/src/post_merge.rs` runs regression gates after each merge. Runner v2 uses both via `PlanMerger` at `crates/roko-cli/src/runner/merge.rs`.

The merge flow should be:
1. Task completes in worktree -> run gates in worktree
2. If gates pass -> `MergeQueue::enqueue()` with files_changed
3. Wait for MergeQueue slot (file-overlap check)
4. Merge worktree branch into integration branch
5. Run `PostMergeRunner::check()` on integration branch
6. If post-merge passes -> `MergeQueue::complete()` -> mark task completed
7. If post-merge fails -> `MergeQueue::fail()` -> revert merge -> retry task

#### Implementation Steps
1. Add a `merge_task_result()` method to EffectDriver that:
   - Gets the list of changed files via `git diff --name-only` in the worktree
   - Creates a `MergeRequest` with plan_id=task_id, branch_name from worktree handle, files_changed
   - Calls `merge_queue.enqueue(request)` if merge_queue is available
   - Polls `merge_queue.dequeue()` to get the merge slot
   - Executes `git merge` from worktree branch into integration branch
   - Calls `merge_queue.complete()` on success or `merge_queue.fail()` on error
2. Add a `run_post_merge_gate()` method that runs compile/clippy on the integration branch after merge
3. Wire `merge_task_result()` into the `run_plan()` loop after gates pass and before marking completed
4. When `merge_queue` is `None`, fall back to the current `commit()` behavior (direct commit in workdir)

#### Design Guidance
The merge step should be a separate method, not inlined into the run loop, so it can be tested independently. The PostMergeRunner pattern from roko-orchestrator should be reused rather than reimplemented. Consider adding a `MergeService` trait to keep EffectDriver decoupled from the git implementation.

#### Verification Criteria
- [ ] Tasks in separate worktrees merge their changes via MergeQueue
- [ ] File-overlapping merges are serialized (not concurrent)
- [ ] Post-merge regression gate catches integration errors
- [ ] Fallback: when `merge_queue` is `None`, commit behavior is unchanged
- [ ] Unit test: two tasks with overlapping files merge sequentially

---

### Task 2.5: Parallel Execution Configuration in WorkflowConfig
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs`
**Depends On**: Task 2.3

#### Context
`WorkflowConfig` (at `crates/roko-runtime/src/pipeline_state.rs:37-46`) currently has four fields: `has_strategy`, `has_review`, `max_iterations`, `max_autofix_attempts`. It needs `max_parallel_tasks` and `worktree_isolation` to configure parallel execution.

The TOML parsing in `parse_workflow_config_toml()` (lines 151-236) already handles the `[workflow]` table and `[[workflow.steps]]` array. New keys need to be added to this parser.

The CLI config at `crates/roko-cli/src/config.rs` has `ExecutorConfig` (referenced via `config.executor.max_concurrent_tasks`) which already supports `max_concurrent_tasks`. This needs to flow into `WorkflowConfig` when WorkflowEngine is used for plan execution.

#### Implementation Steps
1. Add to `WorkflowConfig`:
   ```rust
   pub max_parallel_tasks: usize,      // default: 1
   pub worktree_isolation: bool,       // default: false
   ```
2. Update `WorkflowConfig::express/standard/full()` presets to include the new fields (all default to serial, no isolation).
3. Add TOML parsing for `max_parallel_tasks` and `worktree_isolation` in `parse_workflow_config_toml()` following the existing pattern (lines 216-227).
4. Add `Default` impl update to set `max_parallel_tasks: 1, worktree_isolation: false`.
5. Wire `ExecutorConfig::max_concurrent_tasks` into `WorkflowConfig::max_parallel_tasks` at the CLI entry points.

#### Design Guidance
Keep `worktree_isolation` as a boolean rather than a full `WorktreeConfig` at this level. The detailed worktree config (max_live, idle_ttl, worktrees_root) should come from the global roko.toml config, not from the per-workflow TOML. WorkflowEngine should construct WorktreeManager from the global config when `worktree_isolation` is true.

#### Verification Criteria
- [ ] `WorkflowConfig::from_toml_str("max_parallel_tasks = 4\nworktree_isolation = true")` parses correctly
- [ ] Default `WorkflowConfig` has `max_parallel_tasks: 1, worktree_isolation: false`
- [ ] Existing TOML parsing tests continue to pass
- [ ] New test: round-trip checkpoint preserves `max_parallel_tasks` and `worktree_isolation`

---

## Phase 2: Context Handoff (ORCH-006)

### Task 2.6: Cumulative Context Buffer
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` (new module or inline)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.3

#### Context
EffectDriver passes a flat `context` string to agents via `spawn_agent()` (line 143-146):
```rust
let user_content = context.map_or_else(
    || user_prompt.to_string(),
    |ctx| format!("{user_prompt}\n\n## Additional Context\n\n{ctx}"),
);
```

There is no mechanism to build cumulative context showing what other agents in the same plan have changed. The mega-parity runner identified this as the single most impactful context improvement (merge conflicts reduced from ~50% to ~30%).

orchestrate.rs has `load_prior_task_outputs()` and `with_task_failure_context()` (not ported) that provide similar functionality. These should be the reference for the port.

#### Implementation Steps
1. Create a `CumulativeContext` struct:
   ```rust
   pub struct CumulativeContext {
       changes: Vec<TaskChangeSummary>,
       max_tokens: usize,  // default 4000
   }
   pub struct TaskChangeSummary {
       pub task_id: String,
       pub files_changed: Vec<String>,
       pub diff_stat: String,           // "+45 -12"
       pub functions_added: Vec<String>,
       pub functions_modified: Vec<String>,
   }
   ```
2. After each task completes, compute a git diff summary in the task's worktree via `git diff --stat HEAD~1` and `git diff --name-only HEAD~1`.
3. Add a `render()` method that produces a markdown section:
   ```markdown
   ## What Changed Before You
   Tasks completed in this plan before your task:
   ### T1: Wire compile gate
   - `src/gate/compile.rs` (+45 -12)
   ```
4. Implement token budget management: truncate oldest task summaries when exceeding `max_tokens`.
5. Pass the rendered context into `spawn_agent_in_worktree()` as the `context` parameter.

#### Design Guidance
The context buffer should be per-plan, not global. Each plan execution holds its own `CumulativeContext` that grows as tasks complete. For large plans (20+ tasks), the token truncation is critical -- signature-only views (function name + parameter types, no body) keep overhead manageable. Consider using `roko-index` for function signature extraction when available, with a fallback to `git diff --stat`.

#### Verification Criteria
- [ ] `CumulativeContext::render()` produces valid markdown with file changes
- [ ] Token budget truncation removes oldest summaries first
- [ ] After 3 task completions, the 4th task receives context about all 3 prior tasks
- [ ] Unit test: context with 20 tasks truncates to stay within 4000 tokens

---

### Task 2.7: Gate Failure Context for Retries
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: Task 2.6

#### Context
`PipelineStateV2` carries `last_gate_failure: Option<String>` (line 546) which is a raw string. When a gate fails and the agent retries, the retry context is unstructured:
```rust
context: Some(format!("Previous attempt failed gate '{gate}'. Error:\n{output}"))
```
(pipeline_state.rs lines 676-679)

A structured failure context with per-gate breakdowns, attempt count, and error pattern matching would improve retry success rates.

#### Implementation Steps
1. Add a `FailureRecord` struct to `pipeline_state.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FailureRecord {
       pub attempt: u32,
       pub gate_name: String,
       pub gate_output: String,
       pub diff_summary: Option<String>,
   }
   ```
2. Replace `last_gate_failure: Option<String>` with `failure_history: Vec<FailureRecord>` in `PipelineStateV2`.
3. Update `step()` GateFailed handlers to push to `failure_history` instead of overwriting `last_gate_failure`.
4. Render the failure history as structured context when spawning retry agents.
5. Maintain backward compat: `checkpoint()` / `from_checkpoint()` must handle both the old `last_gate_failure` field and the new `failure_history` field (use `#[serde(default)]`).

#### Design Guidance
Keep the failure history bounded (last 5 failures max) to prevent unbounded growth. The structured format enables ErrorPatternStore matching in a later phase.

#### Verification Criteria
- [ ] Gate failures append to `failure_history` instead of overwriting
- [ ] Retry agent receives structured context with all prior failures
- [ ] Checkpoint round-trip preserves failure history
- [ ] Backward compat: old checkpoints without `failure_history` deserialize correctly

---

## Phase 3: Feature Extraction from orchestrate.rs (ORCH-002)

### Task 2.8: Export Gate Rung Mapping from roko-gate (ORCH-008)
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: none

#### Context
`EffectDriver` at `crates/roko-runtime/src/effect_driver.rs:645-656` has a duplicate `rung_for_gate_name()` function that mirrors the mapping in `crates/roko-gate/src/gate_service.rs`. The code includes a TODO:
```rust
/// TODO: expose this mapping from roko-gate as a public function so this duplicate is not needed.
```

This is a straightforward deduplication. The canonical mapping lives in roko-gate. The EffectDriver should import it.

#### Implementation Steps
1. Find the `rung_for_name` function (or equivalent) in `crates/roko-gate/src/gate_service.rs` and make it `pub`.
2. Re-export it from `crates/roko-gate/src/lib.rs`.
3. Add `roko-gate` as a dependency in `crates/roko-runtime/Cargo.toml` (it is already a dev-dependency, move to `[dependencies]`).
4. Replace the local `rung_for_gate_name()` in `effect_driver.rs` with an import from `roko_gate`.
5. Remove the TODO comment.

#### Design Guidance
If the roko-gate function has a different signature, create a thin wrapper. The important thing is one source of truth for rung assignments.

#### Verification Criteria
- [ ] `rung_for_gate_name` in `effect_driver.rs` is replaced with import from `roko-gate`
- [ ] `cargo test -p roko-runtime` passes
- [ ] `cargo test -p roko-gate` passes
- [ ] The TODO comment is removed

---

### Task 2.9: Add Rung and Confidence Fields to GateVerdict (ORCH-018)
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.8

#### Context
`GateVerdict` in `crates/roko-core/src/foundation.rs:284-296` carries `gate_name`, `passed`, `skipped`, `skip_reason`, `output`, `duration_ms` but not `rung` or `confidence`. The EffectDriver re-derives rung from the gate name (ORCH-008) and uses hardcoded confidence (1.0 for deterministic, 0.5 for heuristic) at lines 308-314:
```rust
let rung = rung_for_gate_name(&verdict.gate_name);
let confidence = if rung <= 4 { 1.0_f64 } else { 0.5_f64 };
```

The code includes a TODO:
```rust
// TODO: add `rung: u8` and `confidence: f64` to GateVerdict in
// roko-core/src/foundation.rs so callers don't need to re-derive them.
```

#### Implementation Steps
1. Add two fields to `GateVerdict` in `crates/roko-core/src/foundation.rs`:
   ```rust
   #[serde(default)]
   pub rung: u8,
   #[serde(default = "default_confidence")]
   pub confidence: f64,
   ```
2. Use `#[serde(default)]` on both fields for backward compatibility with existing serialized verdicts.
3. Update `GateRunner` implementations in `crates/roko-gate/` to populate `rung` and `confidence` when creating verdicts.
4. Update the EffectDriver's `run_gates()` method to read `verdict.rung` and `verdict.confidence` directly instead of re-deriving them.
5. Remove the TODO comment from `effect_driver.rs`.

#### Design Guidance
Use `#[serde(default)]` so old serialized verdicts (without rung/confidence) deserialize correctly with `rung: 0, confidence: 0.0`. Callers should check if `confidence == 0.0` and re-derive if needed, as a migration path.

#### Verification Criteria
- [ ] `GateVerdict` has `rung: u8` and `confidence: f64` fields
- [ ] Deserialization of old JSON without these fields works (default values)
- [ ] EffectDriver reads `verdict.rung` and `verdict.confidence` directly
- [ ] Both TODO comments removed
- [ ] `cargo test --workspace` passes

---

### Task 2.10: Define EffectDriver Service Traits for orchestrate.rs Features
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: none

#### Context
orchestrate.rs implements features that belong in dedicated service traits, not a 22K-line monolith. The feature extraction plan (ORCH-002) calls for six service trait families. This task defines the traits; subsequent tasks implement them.

The traits should follow the existing pattern in `foundation.rs`: `ModelCaller`, `PromptAssembler`, `FeedbackSink`, `GateRunner`, `AffectPolicy` -- all `Send + Sync + 'static` with async methods returning `roko_core::Result<T>`.

The highest-impact features to extract (from the PLAN priority analysis):
1. Knowledge routing (`build_knowledge_routing_advice()` in orchestrate.rs)
2. Episode recording (`EpisodeLogger` in roko-learn)
3. Playbook queries (`PlaybookStore` in roko-learn)
4. Error pattern queries (`ErrorPatternStore` in roko-learn)

#### Implementation Steps
1. Add to `crates/roko-core/src/foundation.rs`:
   ```rust
   /// Knowledge routing service -- queries durable knowledge store for task context.
   #[async_trait]
   pub trait KnowledgeRouter: Send + Sync {
       async fn route(&self, task_description: &str, role: &str) -> Result<Vec<String>>;
   }

   /// Episode recording service -- records agent turns and gate results.
   #[async_trait]
   pub trait EpisodeRecorder: Send + Sync {
       async fn record_turn(&self, run_id: &str, role: &str, model: &str, tokens: u64, cost: f64) -> Result<()>;
       async fn record_gate(&self, run_id: &str, gate_name: &str, passed: bool) -> Result<()>;
       async fn finalize(&self, run_id: &str, succeeded: bool) -> Result<()>;
   }

   /// Error pattern query service.
   #[async_trait]
   pub trait ErrorPatternQuery: Send + Sync {
       async fn match_error(&self, gate_output: &str) -> Result<Option<String>>;
   }
   ```
2. Add optional service fields to `EffectServices`:
   ```rust
   pub knowledge_router: Option<Arc<dyn KnowledgeRouter>>,
   pub episode_recorder: Option<Arc<dyn EpisodeRecorder>>,
   pub error_pattern_query: Option<Arc<dyn ErrorPatternQuery>>,
   ```
3. Update all call sites constructing `EffectServices` to pass `None` for the new fields.

#### Design Guidance
Keep traits minimal. Each trait should have 2-4 methods maximum. Use `Option<Arc<dyn Trait>>` so the EffectDriver degrades gracefully when a service is not available. Do NOT try to port the implementations in this task -- just define the contracts.

#### Verification Criteria
- [ ] Traits defined in `foundation.rs` with documented contracts
- [ ] `EffectServices` has optional fields for each new trait
- [ ] All existing code compiles with `None` for new fields
- [ ] `cargo test --workspace` passes

---

### Task 2.11: Implement KnowledgeRouter for WorkflowEngine
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lib.rs` (or new file)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.10

#### Context
orchestrate.rs has `build_knowledge_routing_advice()` which queries the neuro (knowledge) store for context relevant to the current task. The `KnowledgeStore` exists at `crates/roko-neuro/` and is fully functional. The `ContextAssembler` and `TierProgression` types exist there too.

This task implements the `KnowledgeRouter` trait using the existing `KnowledgeStore`, then wires it into EffectDriver so that dispatched agents receive knowledge context.

#### Implementation Steps
1. Create a `NeuroKnowledgeRouter` struct in `crates/roko-neuro/` that wraps `KnowledgeStore`.
2. Implement the `KnowledgeRouter` trait from `roko-core::foundation`:
   - `route()` queries the knowledge store for entries matching the task description
   - Returns relevant knowledge entries as formatted strings
3. In the EffectDriver's `spawn_agent()` method, if `knowledge_router` is `Some`, query it before prompt assembly and inject results into the `PromptSpec::gate_feedback` or a new context section.
4. Wire the router construction in CLI entry points where `KnowledgeStore` is already loaded.

#### Design Guidance
The knowledge router should be stateless (queries only, no writes). Writes happen via the episode recording path. Keep the query lightweight -- limit to 5 results, with total token budget of 2000 tokens for knowledge context.

#### Verification Criteria
- [ ] `NeuroKnowledgeRouter` implements `KnowledgeRouter` trait
- [ ] EffectDriver queries knowledge router before agent dispatch (when available)
- [ ] Knowledge context appears in the prompt sent to the agent
- [ ] `cargo test -p roko-neuro` passes
- [ ] Graceful degradation: when knowledge_router is None, behavior is unchanged

---

### Task 2.12: Implement EpisodeRecorder for WorkflowEngine
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs` (or new adapter file)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.10

#### Context
`EpisodeLogger` exists at `crates/roko-learn/src/episode_logger.rs` and records agent turns + gate results to `.roko/episodes.jsonl`. orchestrate.rs calls it throughout the dispatch/gate loop. Runner v2 also records episodes via its event loop.

WorkflowEngine's `FeedbackSink` trait records `FeedbackEvent::ModelCall` and `FeedbackEvent::GateResult` but these are generic feedback events, not structured episodes. The episode format includes `Episode { plan_id, task_id, agent_role, model, turns, gates, outcome, ... }`.

This task wraps `EpisodeLogger` in an `EpisodeRecorder` trait implementation and wires it into EffectDriver.

#### Implementation Steps
1. Create a `LearnEpisodeRecorder` adapter that wraps `EpisodeLogger` and implements `EpisodeRecorder`.
2. `record_turn()` creates an entry in the current episode's turns list.
3. `record_gate()` adds a gate verdict to the current episode.
4. `finalize()` writes the completed episode to `.roko/episodes.jsonl`.
5. Wire into EffectDriver: after each `spawn_agent()` call, call `episode_recorder.record_turn()`. After each `run_gates()` call, call `episode_recorder.record_gate()`. At workflow completion, call `finalize()`.

#### Design Guidance
The adapter should be thread-safe (multiple concurrent agents recording turns). Use a `DashMap` or `tokio::sync::Mutex<HashMap>` keyed by run_id to track in-flight episodes.

#### Verification Criteria
- [ ] `LearnEpisodeRecorder` implements `EpisodeRecorder` trait
- [ ] Episodes are written to `.roko/episodes.jsonl` on workflow completion
- [ ] Each episode contains turns (agent calls) and gates (gate results)
- [ ] Concurrent episodes (from parallel tasks) do not interleave

---

## Phase 4: Configurable Failure Recovery (ORCH-015)

### Task 2.13: Failure Policy Configuration
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
**Depends On**: none

#### Context
Gate failure recovery in `PipelineStateV2::step()` (lines 662-687) is hardcoded:
```rust
if self.autofix_attempts < self.config.max_autofix_attempts {
    // autofix
} else if self.iteration < self.config.max_iterations {
    // re-implement
} else {
    // halt
}
```

All gate failures are treated identically. A trivial clippy warning triggers the same recovery as a type error. The mega-parity runner's gate-specific recovery table shows that compile failures -> autofix, test failures -> reimplement, clippy -> autofix (trivial), fmt -> run formatter.

#### Implementation Steps
1. Add a `FailurePolicy` config struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FailurePolicy {
       pub default_action: FailureAction,
       pub default_max_attempts: u32,
       pub per_gate: HashMap<String, GateFailurePolicy>,
   }
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GateFailurePolicy {
       pub action: FailureAction,
       pub max_attempts: u32,
   }
   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   pub enum FailureAction {
       AutoFix,
       Reimplement,
       Skip,
       Halt,
       Escalate,
   }
   ```
2. Add `failure_policy: FailurePolicy` to `WorkflowConfig` with a sensible default.
3. Add TOML parsing for `[workflow.failure]` and `[workflow.failure.<gate>]` tables.
4. Replace the hardcoded match arms in `step()` with a lookup against the failure policy:
   ```rust
   let policy = self.config.failure_policy.policy_for(&gate);
   match policy.action {
       FailureAction::AutoFix if self.autofix_attempts < policy.max_attempts => { ... }
       FailureAction::Reimplement if self.iteration < self.config.max_iterations => { ... }
       FailureAction::Skip => { /* advance past gates */ }
       FailureAction::Escalate => { /* emit EscalateModel action */ }
       _ => { /* halt */ }
   }
   ```
5. Add a new `PipelineOutput::EscalateModel` variant for model escalation.

#### Design Guidance
The default failure policy should match current behavior exactly (autofix first, then reimplement, then halt) so this is a zero-regression change. Per-gate overrides are additive. The `Escalate` action is a new concept that the EffectDriver will need to handle (switch to a stronger model and retry). This task only adds the state machine support; the EffectDriver handling is a separate task.

#### Verification Criteria
- [ ] Default `FailurePolicy` produces identical behavior to current hardcoded logic
- [ ] Per-gate override: compile -> AutoFix(3), test -> Reimplement(2), clippy -> AutoFix(1)
- [ ] TOML `[workflow.failure.compile]` parses correctly
- [ ] All existing `PipelineStateV2` tests pass unchanged
- [ ] New test: custom policy routes test failures to Reimplement

---

## Phase 5: TaskScheduler Resume and Robustness (ORCH-009)

### Task 2.14: Add Serialize/Deserialize to TaskStatus
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/task_scheduler.rs`
**Depends On**: none

#### Context
`TaskStatus` at `crates/roko-runtime/src/task_scheduler.rs:23-38` is:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Blocked,
    Ready,
    Running,
    Completed,
    Failed { error: String },
    Skipped,
}
```

It lacks `Serialize` and `Deserialize` derives. WorkflowEngine checkpoints `PipelineStateV2` state but not `TaskScheduler` state. A crash during multi-task execution loses all task-level progress.

#### Implementation Steps
1. Add `Serialize, Deserialize` derives to `TaskStatus` enum.
2. Add `Serialize, Deserialize` derives to `SchedulableTask` struct.
3. Add a `TaskSchedulerSnapshot` struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct TaskSchedulerSnapshot {
       pub task_statuses: HashMap<String, TaskStatus>,
       pub max_parallel: usize,
   }
   ```
4. Add `checkpoint()` -> `TaskSchedulerSnapshot` and `from_snapshot()` -> `TaskScheduler` methods.
5. In `from_snapshot()`, reconstruct the `tasks` map from the original task definitions and apply the saved statuses.

#### Design Guidance
The snapshot should store task statuses only, not the full task definitions (those come from the plan file). On resume, the caller provides the task definitions and the snapshot provides the statuses. Tasks not in the snapshot default to `Blocked` (new tasks added between runs). Tasks in the snapshot but not in the task list are ignored (deleted tasks).

#### Verification Criteria
- [ ] `TaskStatus` serializes and deserializes correctly
- [ ] `checkpoint()` captures current task statuses and max_parallel
- [ ] `from_snapshot()` restores a TaskScheduler to the saved state
- [ ] Completed tasks remain completed after resume
- [ ] Running tasks revert to `Ready` on resume (conservative -- they may have crashed)

---

### Task 2.15: Extend WorkflowEngine Checkpoint to Include TaskScheduler State
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.14

#### Context
`EffectDriver::save_checkpoint()` (at `effect_driver.rs:438-464`) serializes only `PipelineStateV2` to JSON. In the multi-task `run_plan()` flow (from Task 2.3), the `TaskScheduler` state also needs checkpointing. A crash at task 15 of 20 should resume from task 15, not restart from task 1.

#### Implementation Steps
1. Create a `WorkflowCheckpoint` struct that combines both:
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct WorkflowCheckpoint {
       pub pipeline_state: PipelineStateV2,
       pub task_scheduler: Option<TaskSchedulerSnapshot>,
       pub cumulative_context: Option<CumulativeContext>,
       pub timestamp_ms: u64,
   }
   ```
2. Add a `save_workflow_checkpoint()` method to EffectDriver that serializes `WorkflowCheckpoint`.
3. Add a `load_workflow_checkpoint()` function that deserializes and returns the components.
4. In `run_plan()`, call `save_workflow_checkpoint()` after each task completion.
5. Add a `resume_plan()` method to WorkflowEngine that loads the checkpoint and reconstructs the TaskScheduler.

#### Design Guidance
Use atomic write (tmp + rename) pattern already established in `save_checkpoint()`. The checkpoint file should be at `.roko/state/workflow-{run_id}.json`. Backward compat: if the checkpoint is a bare `PipelineStateV2` (old format), load it as pipeline_state with task_scheduler=None.

#### Verification Criteria
- [ ] `save_workflow_checkpoint()` writes combined state atomically
- [ ] `load_workflow_checkpoint()` restores TaskScheduler to the correct state
- [ ] Resume after simulated crash skips completed tasks
- [ ] Backward compat: old PipelineStateV2-only checkpoints still loadable

---

## Phase 6: Speculative Execution (ORCH-005)

### Task 2.16: Wire Speculative Task Dispatch
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/task_scheduler.rs`
**Depends On**: Task 2.3

#### Context
`SpeculativeExecution` struct exists at `crates/roko-orchestrator/src/executor/mod.rs:65-75`:
```rust
pub struct SpeculativeExecution {
    pub plan_id: String,
    pub task: String,
    pub expected_minutes: u32,
    pub elapsed_minutes: u32,
    pub backup_role: AgentRole,
}
```

The `ExecutorConfig` has `speculative_threshold_multiplier` (line 171). But no code in Runner v2 or WorkflowEngine triggers speculative spawns.

The trigger condition (from the PLAN): speculatively dispatch a task when it is on the critical path, its dependencies are 80%+ complete, and speculative cost is within budget.

#### Implementation Steps
1. Add a `speculative_candidates()` method to `TaskScheduler` that returns tasks where:
   - The task is `Blocked` (not yet ready)
   - 80%+ of its dependencies are `Completed`
   - The remaining dependencies are `Running` (likely to complete soon)
2. In the `run_plan()` loop, after dispatching the normal batch, check for speculative candidates.
3. Spawn speculative agents in separate worktrees with a `CancelToken`.
4. When a dependency fails, cancel the speculative agent via `CancelToken`.
5. When all dependencies complete and the task becomes ready, "adopt" the speculative execution -- do not re-dispatch.
6. Track speculative outcomes (hit/miss) for cost accounting.

#### Design Guidance
Keep speculative execution behind a config flag (`enable_speculation: bool`, default false). Track speculative cost separately from normal cost. Do not speculate on more than `max_parallel / 2` tasks simultaneously to prevent resource exhaustion.

#### Verification Criteria
- [ ] Speculative dispatch triggers when dependencies are 80%+ complete
- [ ] Speculative agent is cancelled when a dependency fails
- [ ] Speculative agent result is adopted when all dependencies complete
- [ ] Speculation disabled by default (opt-in)
- [ ] Cost tracking distinguishes speculative vs normal execution

---

## Phase 7: Anti-Pattern Pre-Gate (ORCH-016)

### Task 2.17: Anti-Pattern Check Registry
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/anti_pattern.rs` (new file)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs`
**Depends On**: none

#### Context
The mega-parity runner uses fast grep-based anti-pattern checks (AP-1 through AP-10) that catch common LLM code generation mistakes in milliseconds. These are not integrated into WorkflowEngine or any gate. The checks are:

- AP-1: Stub gates that return pass (silent-pass)
- AP-2: `block_on` in async code
- AP-3: Duplicate trait definitions vs foundation.rs
- AP-5: Raw `Command::new("claude")` shell-outs
- AP-6: Inline prompt strings (`format!("You are a...")`)
- AP-7: std::sync::Mutex held across .await
- AP-8: Empty function bodies
- AP-9: unimplemented!/unreachable! left behind
- AP-10: Hardcoded localhost/port in non-test code

#### Implementation Steps
1. Create `crates/roko-gate/src/anti_pattern.rs` with:
   ```rust
   pub struct AntiPatternCheck {
       pub id: String,
       pub name: String,
       pub pattern: regex::Regex,
       pub file_glob: String,
       pub exclude_paths: Vec<String>,  // e.g., "tests/", "*_test.rs"
       pub severity: Severity,
       pub message: String,
   }
   pub enum Severity { Error, Warning }

   pub struct AntiPatternRegistry {
       checks: Vec<AntiPatternCheck>,
   }
   impl AntiPatternRegistry {
       pub fn default_checks() -> Self { /* AP-1 through AP-10 */ }
       pub fn run(&self, workdir: &Path, exempt: &[String]) -> Vec<AntiPatternViolation>;
   }
   ```
2. Register all 10 AP checks with their regex patterns.
3. `run()` walks .rs files in workdir (skipping test files for applicable checks), applies regex, collects violations.
4. Add `pub mod anti_pattern;` to `crates/roko-gate/src/lib.rs`.
5. Re-export `AntiPatternRegistry` and `AntiPatternViolation`.

#### Design Guidance
Anti-pattern checks must be fast (< 100ms for the full workspace). Use `walkdir` + `memmap2` or simple `std::fs::read_to_string` + `Regex::is_match`. Do not use tree-sitter or AST parsing -- these are grep-level checks. The false positive rate from the mega-parity runner was ~2-3% (mostly AP-10 for localhost in legitimate config code).

#### Verification Criteria
- [ ] All 10 AP checks registered with correct regex patterns
- [ ] `run()` completes in < 100ms on a typical crate (< 10K LOC)
- [ ] AP-7 (mutex across await) detects the pattern in synthetic test code
- [ ] Exempt list excludes specific AP IDs for a task
- [ ] Test files are excluded from AP-8 (empty functions are common in test stubs)

---

### Task 2.18: Wire Anti-Pattern Checks as Pre-Gate Step
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: Task 2.17

#### Context
Anti-pattern checks should run before any compilation gate. They execute in milliseconds and catch structural mistakes that compilation/tests do not detect (e.g., stubs that always return success, inline prompt strings).

The EffectDriver's `run_gates()` method (line 280-333) runs gates via `self.services.gate_runner.run_gates(config)`. Anti-pattern checks should run before this call.

#### Implementation Steps
1. Add `anti_pattern_registry: Option<Arc<AntiPatternRegistry>>` to `EffectServices`.
2. In `EffectDriver::run_gates()`, if `anti_pattern_registry` is `Some`:
   - Run AP checks first
   - If any AP check has `Severity::Error`, return `PipelineInput::GateFailed` immediately (before compilation)
   - If only `Severity::Warning`, include violations in the gate report but do not fail
3. Include AP check results in the `GateReport` verdicts as `ap:<id>` named gates.
4. Pass the task's `ap_exempt` list (from tasks.toml) through to the AP runner.

#### Design Guidance
AP checks are "rung -1" -- they run before rung 0 (compile). They should be reported as gate verdicts so the affect policy and learning subsystem can track them. Use the existing `GateVerdict` struct with `gate_name: "ap:AP-7"`.

#### Verification Criteria
- [ ] AP checks run before compilation gates
- [ ] AP Error violations fail the gate immediately (no wasted compile time)
- [ ] AP Warning violations are reported but do not fail the gate
- [ ] AP violations appear in `GateReport` as `ap:<id>` verdicts
- [ ] `ap_exempt` list suppresses specific checks

---

## Phase 8: Cost-Aware Scheduling (ORCH-010)

### Task 2.19: Priority-Based Task Ordering in TaskScheduler
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/task_scheduler.rs`
**Depends On**: none

#### Context
`TaskScheduler::next_batch()` (at `task_scheduler.rs:81-135`) iterates `self.status` in HashMap order (non-deterministic) and picks ready tasks based on file exclusion and parallelism constraints. It does not consider:
- Downstream dependency count (tasks that unblock more work should run first)
- Estimated cost (cheaper tasks first to secure early wins)
- Critical path membership (zero-slack tasks are time-critical)

#### Implementation Steps
1. Add a `priority_score()` method to `TaskScheduler`:
   ```rust
   fn priority_score(&self, task_id: &str) -> f64 {
       let dependents = self.count_downstream_dependents(task_id);
       let dependents_score = dependents as f64 * 0.5;
       // Critical path bonus would require DAG integration -- defer to later
       dependents_score
   }
   ```
2. Add `count_downstream_dependents()`: count all tasks that transitively depend on this task.
3. In `next_batch()`, collect all Ready tasks, sort by `priority_score()` descending, then apply file exclusion and parallelism constraints in sorted order.
4. Add a `tier` field to `SchedulableTask` for cost estimation:
   ```rust
   pub tier: Option<String>,  // "mechanical", "focused", "integrative", "architectural"
   ```
5. Factor tier into priority score: mechanical tasks get a +0.3 bonus (cheap, quick wins).

#### Design Guidance
The priority scoring should be deterministic given the same inputs. Use `BTreeMap` or sort by (score DESC, task_id ASC) for deterministic tie-breaking. The score function can be extended later with critical path integration from `UnifiedTaskDag::slack()`.

#### Verification Criteria
- [ ] Tasks with more downstream dependents are dispatched first
- [ ] Mechanical-tier tasks are prioritized over architectural-tier tasks (when dependency count is equal)
- [ ] File exclusion still prevents conflicting tasks from running concurrently
- [ ] Deterministic ordering: same inputs produce same batch order
- [ ] Existing tests pass with new ordering (may need updates for deterministic expectations)

---

## Phase 9: Adaptive Parallelism

### Task 2.20: Adaptive Parallelism Controller
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` (or new file)
**Depends On**: Task 2.3

#### Context
With parallel execution enabled, runtime signals should dynamically adjust concurrency:
- Error rate > 30% -> reduce max_parallel by 50%
- Error rate < 10% -> increase max_parallel by 1 (up to configured max)
- Merge conflicts > 20% -> reduce max_parallel
- Disk < 5GB -> pause dispatch entirely

The mega-parity runner validated that PARALLEL=15 was optimal for 20 API workers, but this varies with task complexity and error rates.

#### Implementation Steps
1. Create an `AdaptiveParallelism` struct:
   ```rust
   pub struct AdaptiveParallelism {
       base_max_parallel: usize,
       current_max_parallel: usize,
       outcome_window: VecDeque<bool>,  // true=success, false=failure
       window_size: usize,              // default: 10
   }
   ```
2. Add `adjust(&mut self, task_succeeded: bool) -> usize` that updates the window and recalculates.
3. Add `check_disk_space()` that queries available space via `statvfs` (unix) or `GetDiskFreeSpaceEx` (windows).
4. Wire into `run_plan()`: before each `next_batch()`, call `adaptive.adjust()` and pass `current_max_parallel` to the scheduler.
5. Add config: `adaptive_parallelism: bool` (default false) in `WorkflowConfig`.

#### Design Guidance
The adjustment should be smooth (no oscillation). Use a sliding window of the last N task outcomes. Minimum parallel is always 1. Maximum parallel is the configured `max_parallel_tasks`. Disk space checking should be lazy (check every 60 seconds, not every dispatch).

#### Verification Criteria
- [ ] Error rate spike (3/10 failures) reduces max_parallel from 4 to 2
- [ ] Low error rate (0/10 failures) increases max_parallel back toward configured max
- [ ] Disk space below 5GB pauses dispatch
- [ ] Disabled by default; opt-in via config
- [ ] Unit test: simulate error rate changes and verify parallelism adjustments

---

## Phase 10: Wave Gating

### Task 2.21: Wave-Boundary Gate Execution
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: Task 2.3, Task 2.4

#### Context
Per-task compilation takes 15-40 minutes (cargo check for 18 crates). Wave gating -- running compilation once per wave of tasks instead of per-task -- reduces this to 3-8 minutes per wave. The mega-parity runner used this pattern with PARALLEL=15 and achieved 10x speedup.

Three gating strategies:
1. **Per-task**: Each task runs gates individually (safest, slowest)
2. **Wave**: Gates run once after all tasks in a wave complete and merge (balanced)
3. **Deferred**: No gates during execution; compile at end (fastest, riskiest)

The `UnifiedTaskDag::waves()` method in `crates/roko-orchestrator/src/dag.rs` already partitions tasks into waves via BFS layering.

#### Implementation Steps
1. Add a `gate_strategy` field to `WorkflowConfig`:
   ```rust
   pub enum GateStrategy {
       PerTask,     // default: run gates after each task
       PerWave,     // run gates after each wave completes
       Deferred,    // no gates during execution
   }
   ```
2. In `run_plan()`, track wave membership for each task.
3. When `GateStrategy::PerWave`:
   - After each task completes, merge its worktree into the integration branch
   - When all tasks in the current wave have completed and merged:
     - Run gates on the integration branch (not individual worktrees)
     - If gates fail, identify the offending merge via `git bisect` on merge commits
     - Retry only the offending task(s)
4. When `GateStrategy::Deferred`:
   - Skip all gates during execution
   - After all tasks complete, run a single gate pass on the final state
5. Add a "no-build" prompt section injected when `GateStrategy != PerTask`, telling agents not to compile.

#### Design Guidance
Wave gating requires tracking which wave each task belongs to. This can be derived from `TaskScheduler` or computed via `UnifiedTaskDag::waves()`. The offending-merge identification for wave gate failures is best done via git bisect on the merge commits within the wave.

#### Verification Criteria
- [ ] `GateStrategy::PerWave` runs gates once per wave, not per task
- [ ] `GateStrategy::Deferred` skips all gates until the end
- [ ] Wave gate failure identifies which task caused the regression
- [ ] "No-build" prompt section injected for wave/deferred strategies
- [ ] Per-task gating (default) is unchanged from current behavior

---

## Phase 11: State Machine Convergence (ORCH-003)

### Task 2.22: Phase Adapter Between PipelineStateV2 and PlanPhase
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/pipeline_state.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/phase.rs`
**Depends On**: none

#### Context
Two state machines model the same concept:
- `PipelineStateV2::Phase` (10 states) at `crates/roko-runtime/src/pipeline_state.rs:365-390`
- `PlanPhase` (14 states) at `crates/roko-core/src/phase.rs`

They are not interoperable. Monitoring tools must handle both. A workflow starting as a simple run (PipelineStateV2) cannot report status in PlanPhase terms.

Differences:
- PipelineStateV2 has `Strategizing, Committing, Cancelled` -- PlanPhase does not
- PlanPhase has `Enriching, Verifying, DocRevision, RegeneratingVerify, Merging, Done, Skipped` -- PipelineStateV2 does not
- PipelineStateV2 `Halted{reason: String}` vs PlanPhase `Failed{reason: FailureKind}`

#### Implementation Steps
1. Create a `PhaseAdapter` module with bidirectional mapping functions:
   ```rust
   pub fn pipeline_to_plan_phase(phase: &Phase) -> PlanPhase { ... }
   pub fn plan_phase_to_pipeline(phase: &PlanPhase) -> Phase { ... }
   ```
2. Define the mapping:
   - `Pending -> Queued`
   - `Strategizing -> Enriching` (closest semantic match)
   - `Implementing -> Implementing`
   - `Gating -> Gating`
   - `AutoFixing -> AutoFixing`
   - `Reviewing -> Reviewing`
   - `Committing -> Merging`
   - `Complete -> Done`
   - `Halted{reason} -> Failed{FailureKind::Other(reason)}`
   - `Cancelled -> Skipped`
3. Add a common `PhaseLabel` enum that both can map to for unified monitoring.
4. Implement `From<Phase> for PhaseLabel` and `From<PlanPhase> for PhaseLabel`.

#### Design Guidance
The adapter should be lossy but not crash -- unmappable states should map to the closest equivalent with a log warning. The `PhaseLabel` enum is the unified monitoring interface; both state machines can report their status through it. Do not attempt to unify the state machines themselves -- that is a much larger change.

#### Verification Criteria
- [ ] All 10 `Phase` variants map to a `PlanPhase` variant
- [ ] All 14 `PlanPhase` variants map to a `Phase` variant
- [ ] Round-trip: `pipeline_to_plan_phase(plan_phase_to_pipeline(x))` is semantically equivalent to `x`
- [ ] Monitoring code can use `PhaseLabel` for unified status display

---

## Phase 12: Affect Policy Wiring (ORCH-011)

### Task 2.23: Wire DaimonState as AffectPolicy for WorkflowEngine
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-daimon/src/lib.rs` (or adapter)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: none

#### Context
EffectDriver supports `AffectPolicy` via `EffectServices::affect_policy` (line 49) but Runner v2 only provides `DaimonPolicy::default()`. The full `DaimonState` affect engine in orchestrate.rs (`load_or_new` at line 299) provides somatic signals, strategy coordinates, and dispatch modulation.

`roko-daimon` crate has `AffectEngine`, `DaimonState`, `StrategyCoordinates`. These need to be wrapped in an `AffectPolicy` impl and wired into EffectServices at CLI entry points.

#### Implementation Steps
1. Create a `DaimonAffectPolicy` adapter in `crates/roko-daimon/` that wraps `DaimonState` and implements `roko_core::foundation::AffectPolicy`.
2. `pre_dispatch()` -> query DaimonState for current behavioral state and PAD values.
3. `modulate_dispatch()` -> compute exploration rate, tier bias, turn limit factor from DaimonState.
4. `on_task_outcome()` -> feed outcome back to DaimonState for learning.
5. `on_gate_result()` -> feed gate results for somatic marker updates.
6. Wire into CLI entry points (`crates/roko-cli/src/run.rs`) where `EffectServices` is constructed -- load `DaimonState` from `.roko/state/daimon.json` and wrap in `DaimonAffectPolicy`.

#### Design Guidance
The DaimonState should persist across runs (load from disk, save on completion). The affect modulation should be conservative -- start with small adjustments (exploration_rate 0.0-0.3) until the system is validated. The fallback should be `DaimonPolicy::default()` which provides neutral modulation.

#### Verification Criteria
- [ ] `DaimonAffectPolicy` implements `AffectPolicy` trait
- [ ] EffectDriver receives non-default modulation values from DaimonState
- [ ] DaimonState persists to `.roko/state/daimon.json` between runs
- [ ] Fallback: when DaimonState fails to load, neutral modulation is used

---

## Phase 13: Supervision and Health

### Task 2.24: Wire ProcessSupervisor into WorkflowEngine
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/effect_driver.rs`
**Depends On**: Task 2.3

#### Context
`ProcessSupervisor` at `crates/roko-runtime/src/process.rs` (1,354 LOC) provides Erlang-style supervision strategies (OneForOne, OneForAll, RestForOne) with configurable restart limits. It tracks processes via `ProcessHandle` with unique `ProcessId`, cooperative shutdown, and session metadata.

Currently, WorkflowEngine spawns agents via `EffectDriver::spawn_agent()` which calls `model_caller.call()` but does not track the agent process via ProcessSupervisor. This means:
- No timeout enforcement (stuck agents run forever)
- No restart on transient failure
- No process inventory for the dashboard

#### Implementation Steps
1. Add `process_supervisor: Option<Arc<ProcessSupervisor>>` to `EffectServices`.
2. When spawning agents, if ProcessSupervisor is available:
   - Create a `SpawnConfig` with the agent's label, timeout, and session metadata
   - Register the process with ProcessSupervisor
   - Set a timeout via `ProcessSessionConfig::timeout_ms`
3. On agent completion or failure, deregister from ProcessSupervisor.
4. On timeout, ProcessSupervisor sends SIGTERM (grace period) then SIGKILL.
5. Set default `SupervisionStrategy::OneForOne { max_restarts: 1 }` so transient failures get one retry.

#### Design Guidance
The ProcessSupervisor should be optional (like all EffectServices). When not available, timeout enforcement is the caller's responsibility (which may mean no enforcement). The supervision strategy should be configurable per-task via the tasks.toml `timeout_secs` field.

#### Verification Criteria
- [ ] Spawned agents are registered with ProcessSupervisor
- [ ] Timeout enforcement kills stuck agents after configured seconds
- [ ] OneForOne strategy retries once on transient failure
- [ ] Process inventory is available for dashboard queries
- [ ] No ProcessSupervisor: behavior is unchanged

---

### Task 2.25: Disk Space Monitoring (ORCH-020)
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: Task 2.20

#### Context
The mega-parity runner learned that disk exhaustion from cargo build caches causes silent failures. With parallel execution and worktrees, each worktree can consume 500MB (source) + 5-15GB (target dir). 15 concurrent worktrees = 7.5GB source + potentially 75GB+ target.

WorkflowEngine has no disk space monitoring. Cargo builds in worktrees can exhaust available space without warning.

#### Implementation Steps
1. Add a `check_disk_space()` utility function:
   ```rust
   async fn check_disk_space(path: &Path) -> Option<u64> {
       #[cfg(unix)]
       {
           use std::os::unix::fs::MetadataExt;
           let stat = nix::sys::statvfs::statvfs(path).ok()?;
           Some(stat.blocks_available() * stat.block_size())
       }
   }
   ```
2. In `run_plan()`, check disk space every 60 seconds (or before each wave).
3. If available space < 5GB, pause dispatch (do not start new tasks, let running ones complete).
4. Log a warning when space < 10GB.
5. Emit `RuntimeEvent::ResourceWarning` when disk is low.

#### Design Guidance
The 5GB threshold is a configurable minimum. For development machines, 5GB is reasonable; for servers, it should be higher. Shared target directories (`CARGO_TARGET_DIR`) reduce the per-worktree disk impact from 5-15GB to near zero.

#### Verification Criteria
- [ ] Disk space check runs periodically during plan execution
- [ ] Dispatch pauses when available space < 5GB
- [ ] Warning logged when space < 10GB
- [ ] RuntimeEvent emitted for monitoring

---

## Phase 14: orchestrate.rs Retirement

### Task 2.26: Audit Feature Parity Before Retirement
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**: none (audit only)
**Depends On**: Tasks 2.8-2.12

#### Context
Before deleting orchestrate.rs (22,522 lines), every feature in the "Features Only in Dead Code" table (from the AUDIT) must be classified as: ported to WorkflowEngine, explicitly deprecated, or documented as a gap.

Features from the audit table (Section 10):
- Dream consolidation (line 7589+)
- Daimon affect engine (full, line 266+)
- Knowledge routing (`build_knowledge_routing_advice()`)
- VCG auction (`vcg_allocate()`)
- Custody audit chain (`CustodyLogger`)
- Skill extraction (`SkillLibrary::extract()`)
- Anophily remediation
- C-factor computation (`CFactorSummary`)
- 30+ enrichment steps
- Predictive calibration (`CalibrationTracker`)
- Section effectiveness (`SectionEffectivenessRegistry`)
- Error pattern queries (`ErrorPatternStore`)
- Model experiments (`ModelExperimentStore`)
- Heartbeat monitoring (`HeartbeatClock`)
- Routing decision log (`RoutingDecisionLog`)

#### Implementation Steps
1. For each feature in the table, determine:
   - Is it ported to WorkflowEngine? (via Tasks 2.10-2.12 or prior work)
   - Is it used by Runner v2? (still active)
   - Is it dead code with no consumers?
2. Create a classification table in `.roko/GAPS.md`.
3. Features that are ported -> mark as "ported, verified".
4. Features that are only in orchestrate.rs and have no consumer -> mark as "deprecated".
5. Features that are valuable but not yet ported -> mark as "gap, track for future" with specific task IDs.

#### Design Guidance
Do not block deletion on porting every feature. Some features (VCG auction, anophily remediation) may be genuinely unused. The goal is to make a conscious decision about each feature, not to port them all.

#### Verification Criteria
- [ ] Every feature in the "Features Only in Dead Code" table is classified
- [ ] Classification table written to `.roko/GAPS.md`
- [ ] No feature is accidentally lost -- each is either ported, deprecated, or tracked

---

### Task 2.27: Delete orchestrate.rs and Remove Dead Imports
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (delete)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs` or `main.rs` (remove mod)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` (remove orphaned deps)
**Depends On**: Task 2.26

#### Context
orchestrate.rs is 22,522 lines. Its import block (lines 1-210) pulls from every crate in the workspace. After feature extraction and parity audit, the file can be deleted.

The legacy `roko plan run` entry point that calls orchestrate.rs (if any remains) must be redirected to Runner v2 or WorkflowEngine.

#### Implementation Steps
1. Remove `mod orchestrate;` (or equivalent) from the CLI crate's module tree.
2. Delete `crates/roko-cli/src/orchestrate.rs`.
3. Run `cargo check --workspace` to find orphaned imports and fix them.
4. Remove any `use` statements in other files that reference `orchestrate::`.
5. Check `crates/roko-cli/Cargo.toml` for dependencies that were only used by orchestrate.rs. Remove orphaned dependencies.
6. Run `cargo test --workspace` to verify nothing breaks.
7. Update `CLAUDE.md` to remove references to orchestrate.rs file paths and mark it as retired.

#### Design Guidance
Do this in a single commit for clean git history. The commit message should reference the feature parity audit (Task 2.26) that verified all valuable features are ported or tracked.

#### Verification Criteria
- [ ] `crates/roko-cli/src/orchestrate.rs` no longer exists
- [ ] `cargo build --workspace` compiles without errors
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes
- [ ] No orphaned imports referencing orchestrate
- [ ] CLAUDE.md updated

---

### Task 2.28: Update Runner v2 to Use WorkflowEngine for Task Dispatch
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/mod.rs`
**Depends On**: Tasks 2.1-2.5

#### Context
Runner v2's event loop at `crates/roko-cli/src/runner/event_loop.rs` (3,035 LOC) has its own dispatch path: it constructs a `Dispatcher`, resolves agent runtimes via `resolve_agent_runtime()`, and dispatches agents via `spawn_agent_result_bridge()`. This is separate from EffectDriver's `spawn_agent()`.

Once WorkflowEngine supports parallel task dispatch (Tasks 2.1-2.5), Runner v2 should delegate to WorkflowEngine for agent dispatch rather than maintaining its own dispatch path. This eliminates the AP-4DISP anti-pattern (four dispatch implementations).

Runner v2 adds significant operational features not in WorkflowEngine:
- Line-by-line streaming output parsing (`agent_stream.rs`)
- Real-time TUI updates via `StateHub`
- Episode and efficiency event recording
- Dream consolidation on plan completion

These need to be preserved by wiring them into WorkflowEngine's event system.

#### Implementation Steps
1. Identify the dispatch call sites in `event_loop.rs` (search for `spawn_agent`, `resolve_agent_runtime`, `Dispatcher`).
2. Replace direct agent dispatch with calls to `EffectDriver::spawn_agent()` or `spawn_agent_in_worktree()`.
3. Wire Runner v2's `StateHub` updates into WorkflowEngine's `RuntimeEvent` emissions.
4. Preserve streaming output parsing by implementing the `ModelCaller` trait with streaming support.
5. Update `max_concurrent_tasks` in `ExecutorConfig` construction to use the configured value instead of hardcoded `1`.
6. Preserve episode/efficiency event recording by implementing the `EpisodeRecorder` trait.
7. Keep dream consolidation trigger by subscribing to WorkflowEngine's completion events.

#### Design Guidance
This is the highest-risk task because Runner v2 is the active CLI execution path. The migration should be incremental: start by having Runner v2 use EffectDriver for dispatch while keeping its own event loop. Full migration to WorkflowEngine's `run_plan()` is a later step. Test each step with `roko plan run <dir>` on a real plan.

#### Verification Criteria
- [ ] `roko plan run <dir>` uses EffectDriver for agent dispatch
- [ ] TUI updates still work (StateHub receives events)
- [ ] Episode recording still produces `.roko/episodes.jsonl` entries
- [ ] Streaming output parsing still works
- [ ] `max_concurrent_tasks` respects config (not hardcoded to 1)
- [ ] All existing Runner v2 functionality preserved

---

## Dependency Graph

```
2.1 EffectServices worktree fields
 |
 v
2.2 Worktree allocation in spawn_agent
 |
 v
2.3 Concurrent task dispatch -----------> 2.6 Cumulative context
 |                                           |
 v                                           v
2.4 Post-task merge via MergeQueue       2.7 Gate failure context
 |
 v
2.5 Parallel config in WorkflowConfig
 |
 v
2.28 Runner v2 uses WorkflowEngine

2.8 Export rung mapping (independent)
 |
 v
2.9 Rung/confidence on GateVerdict

2.10 Service traits (independent)
  |
  +---> 2.11 KnowledgeRouter impl
  +---> 2.12 EpisodeRecorder impl

2.13 Failure policy config (independent)

2.14 TaskStatus serialize (independent)
 |
 v
2.15 Extend checkpoint to include TaskScheduler

2.16 Speculative dispatch (depends on 2.3)

2.17 Anti-pattern registry (independent)
 |
 v
2.18 Wire AP checks as pre-gate

2.19 Priority-based task ordering (independent)

2.20 Adaptive parallelism (depends on 2.3)

2.21 Wave gating (depends on 2.3, 2.4)

2.22 Phase adapter (independent)

2.23 DaimonState as AffectPolicy (independent)

2.24 ProcessSupervisor in WorkflowEngine (depends on 2.3)

2.25 Disk space monitoring (depends on 2.20)

2.26 Feature parity audit (depends on 2.8-2.12)
 |
 v
2.27 Delete orchestrate.rs
```

## Priority Summary

| Priority | Tasks | Total Effort |
|---|---|---|
| P0 | 2.1, 2.2, 2.3, 2.4, 2.6, 2.14, 2.15 | ~35 hours |
| P1 | 2.5, 2.7, 2.8, 2.9, 2.10, 2.11, 2.12, 2.13, 2.17, 2.18, 2.21, 2.28 | ~54 hours |
| P2 | 2.16, 2.19, 2.20, 2.22, 2.23, 2.24, 2.25, 2.26, 2.27 | ~35 hours |

**Critical path**: 2.1 -> 2.2 -> 2.3 -> 2.4 -> 2.28 (32 hours for core parallel execution + Runner v2 migration)

**Independent work streams** (can proceed in parallel):
- Stream A: 2.8 -> 2.9 (5 hours, gate improvements)
- Stream B: 2.10 -> 2.11, 2.12 (14 hours, feature extraction)
- Stream C: 2.13 (4 hours, failure policy)
- Stream D: 2.14 -> 2.15 (5 hours, resume)
- Stream E: 2.17 -> 2.18 (7 hours, anti-pattern gates)
