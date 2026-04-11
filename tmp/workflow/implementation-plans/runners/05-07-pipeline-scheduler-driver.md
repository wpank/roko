# Runner 05-07 — Pipeline + TaskScheduler + EffectDriver

> **Give this entire file to a fresh agent.** These three plans are tightly coupled and should land together.

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. Goal: extend the single-prompt pipeline FSM into a multi-task DAG executor, wire the `TaskScheduler` into `WorkflowEngine`, and make `EffectDriver` handle all new action variants including gate feedback, concurrent wave dispatch, merge, and safety.

**Read first:**

1. `tmp/workflow/implementation-plans/05-pipeline-multi-task.md` — FSM extension
2. `tmp/workflow/implementation-plans/06-task-scheduler-integration.md` — DAG + wave
3. `tmp/workflow/implementation-plans/07-effect-driver-completion.md` — action handlers
4. `crates/roko-runtime/src/pipeline_state.rs` — current `PipelineStateV2`
5. `crates/roko-runtime/src/task_scheduler.rs` — current `TaskScheduler`
6. `crates/roko-runtime/src/effect_driver.rs` — current `EffectDriver`
7. `crates/roko-runtime/src/workflow_engine.rs` — current `WorkflowEngine`

---

## Phase A: Pipeline FSM Extension (Plan 05)

### A1: Extend `Phase` enum

**File:** `crates/roko-runtime/src/pipeline_state.rs`

Add: `Enriching`, `DispatchingWave { wave: u32 }`, `Verifying { task_id: String }`, `DocRevision { task_id: String }`, `Merging { plan_id: String }`, `AwaitingReplan { task_id: String, reason: String }`.

Add `task_id: Option<String>` to `Implementing`, `AutoFixing`, `Gating`, `Reviewing`, `Committing` (backward compat: `None` for single-prompt).

### A2: Extend `PipelineInput`

Add: `EnrichmentDone { brief }`, `EnrichmentSkipped`, `TaskCompleted { task_id, output }`, `TaskFailed { task_id, error }`, `WaveCompleted { wave }`, `VerifyPassed { task_id }`, `VerifyFailed { task_id, failures }`, `DocRevisionDone { task_id }`, `MergeSucceeded { plan_id }`, `MergeFailed { plan_id, conflict }`, `ReplanRequested { task_id, reason }`, `ReplanApproved { task_id }`, `ReplanRejected { task_id }`.

Add `GateFailureRecord { gate_name, rung, kind: FailureClass, stderr, exit_code }`.

### A3: Add `step_actions() -> Vec<PipelineAction>`

New method alongside `step()`. Returns multiple actions for wave dispatch. `step()` becomes thin wrapper calling `step_actions().first()`.

Add new `PipelineAction` variants: `SpawnEnricher`, `SpawnImplementerForTask { task_id, gate_feedback, review_findings }`, `SpawnAutoFixerForTask { task_id, error_context }`, `RunGateForTask { task_id, rung }`, `RunVerifyStepsForTask { task_id }`, `SpawnReviewerForTask { task_id }`, `SpawnScribeForTask { task_id }`, `CommitForTask { task_id, message }`, `SubmitMerge { plan_id }`, `EmitWarning(String)`, `NoOp`.

### A4: Implement `FailureClassifier`

Create `crates/roko-runtime/src/failure_classifier.rs`. Pure, no I/O, no async.

- `classify(failure: &GateFailureRecord, task_id: &str) -> NextAction`
- Rules: dedup identical failures → `EscalateModel`; classify by stderr patterns → `AutoFix`/`RetryWithContext`/`NeedsHuman`/`Halt`/`Decompose`
- Track `consecutive_failures` and `replan_count_per_plan`
- Enforce `replan_max_per_plan` cap

### A5: Define `WorkflowTemplate::PlanExecution`

Add `PlanExecutionConfig { max_concurrent_tasks, task_timeout_secs, gate_template, merge_strategy, doc_revision, replan_max_per_plan }`.

Implement PlanExecution transitions: `Pending → Enriching → DispatchingWave → (per-task loops) → Merging → Complete`.

### A6: Tests

- Unit test every `(Phase, PipelineInput)` transition
- Diamond DAG test: A → {B,C} → D → E
- Failure dedup test
- Old checkpoint resume test

---

## Phase B: TaskScheduler Integration (Plan 06)

### B1: Add `compute_waves()`

**File:** `crates/roko-runtime/src/task_scheduler.rs`

BFS layering: wave 0 = no deps, wave 1 = depends only on wave 0, etc.

### B2: Add retry cooldown

`TaskRetryState { attempts, last_failure_ms, backoff_ms }`. `ready_tasks_at(now_ms)` filters by cooldown. Exponential backoff: 1s, 2s, 4s... cap 60s.

### B3: Cross-plan dependencies

`DependencyRef::parse("plan:task")` splits on `:`. Add `plan_id` to `SchedulableTask`.

### B4: Wire into WorkflowEngine

For `PlanExecution` template, `WorkflowEngine::run_with_cancel` creates a `TaskScheduler` and queries `ready_tasks()` / `next_batch()` to drive wave dispatch.

### B5: Tests

- Diamond DAG order + parallelism
- File-overlap serialization
- Retry backoff exponential
- Cross-plan dependency

---

## Phase C: EffectDriver Completion (Plan 07)

### C1: Fix gate feedback

**File:** `crates/roko-runtime/src/effect_driver.rs`

Replace `gate_feedback: Vec::new()` in `spawn_agent` with actual gate feedback from pipeline state. Verify: `rg 'gate_feedback: Vec::new\(\)' crates/roko-runtime/src/effect_driver.rs` returns 0.

### C2: Add `spawn_for_role` helper

Single helper that builds `PromptSpec`, calls `prompt_assembler.assemble`, calls `model_caller.call`, returns `EffectOutcome`. All 6 spawn functions delegate to it.

### C3: Add `execute(PipelineAction)` dispatch for all new variants

Exhaustive match — NO wildcard `_ =>` arm.

### C4: Concurrent fanout

In `WorkflowEngine`, when `step_actions()` returns multiple actions:

```rust
futures::stream::iter(actions)
    .map(|a| driver.execute(a))
    .buffer_unordered(max_concurrent)
    .collect::<Vec<_>>()
    .await
```

### C5: Create `MergeService` trait + `GitMergeService`

**File:** `crates/roko-runtime/src/merge_service.rs`

Extract logic from `crates/roko-orchestrator/src/merge_queue.rs`. Support `DirectCommit`, `Worktree`, `PullRequest` strategies.

### C6: Add to `EffectServices`

`merge_service`, `worktree_service`, `persistence`, `safety` all added to `EffectServices`.

### C7: Cancellation

Check `CancelToken` per stream chunk inside `spawn_for_role`. Return `EffectOutcome::Failed` if cancelled.

---

## Verification Checklist

```bash
# Pipeline multi-task phases exist
rg 'Phase::(Enriching|DispatchingWave|Verifying|DocRevision|Merging|AwaitingReplan)' crates/roko-runtime/src/pipeline_state.rs
# returns 6+

# Failure classifier is pure
rg 'use tokio|use std::fs|async fn' crates/roko-runtime/src/failure_classifier.rs
# returns 0

# Gate feedback not empty
rg 'gate_feedback: Vec::new\(\)' crates/roko-runtime/src/effect_driver.rs
# returns 0

# WorkflowEngine uses TaskScheduler
rg 'TaskScheduler' crates/roko-runtime/src/workflow_engine.rs
# returns 1+

# Concurrent fanout
rg 'buffer_unordered' crates/roko-runtime/src/workflow_engine.rs
# returns 1+

# No wildcard arm in execute
# (compiler enforces exhaustiveness; verify manually)

cargo test --workspace
```
