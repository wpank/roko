# Tasks: 14 -- Runner Patterns (Native Integration)

> Bring the mega-parity runner's proven patterns into roko's native Rust
> execution infrastructure. These patterns were validated across ~195 parallel
> batches, ~6 hours wall time, and 177K LOC. The shell scripts worked; now the
> Rust crates must deliver the same behavior natively.
>
> **Source plans**: `impl/14-RUNNER-PATTERNS.md`, `22-RUNNER-LESSONS.md`,
> `01-LESSONS-AND-APPROACHES.md`

---

## Overview

The mega-parity runner proved five operational patterns that are essential for
reliable parallel code generation at scale:

1. **Worktree isolation** -- each task gets its own git worktree; no shared
   mutable state. Eliminates file corruption, stale reads, git index conflicts.
2. **Wave gating** -- defer compilation to wave boundaries instead of per-task.
   10-100x speed improvement (15-40 min/task -> 1-5 min/task).
3. **Context handoff** -- telling agent B what agent A changed reduces merge
   conflicts by ~40%. Cumulative context sections are the mechanism.
4. **Anti-pattern pre-gates** -- fast grep-based checks catching LLM code-gen
   mistakes in milliseconds, before any compilation.
5. **Resume from disk state** -- any long-running process will crash. Simple
   `.result` files on disk enable kill-restart-continue at any point.

**Critical constraint**: The infrastructure for most of this *already exists*
in `roko-orchestrator` and `roko-runtime` but is never called from the live
runner in `roko-cli/src/runner/`. This is another instance of the project's
core anti-pattern: "built but never connected." The primary work is *wiring*,
not building.

---

## Anti-Patterns to Remove

These are specific instances found in the codebase that must be addressed
during implementation. Each task references which anti-patterns it resolves.

| ID | Anti-Pattern | Where | Impact |
|---|---|---|---|
| **APR-1** | WorktreeManager exists but runner never calls it | `roko-orchestrator/src/worktree.rs` has `WorktreeManager` with create/health/locks; `roko-cli/src/runner/event_loop.rs` dispatches agents into the shared workdir | All tasks share one working directory; parallel agents corrupt each other's files |
| **APR-2** | MergeQueue exists but runner never enqueues | `roko-orchestrator/src/merge_queue.rs` has `MergeQueue` with enqueue/reserve/overlap detection; `roko-cli/src/runner/merge.rs` wraps it as `PlanMerger` but the event loop does not connect completions to it | Merges are ad-hoc, no file-overlap serialization |
| **APR-3** | PipelineStateV2 has no wave-level gating phase | `roko-runtime/src/pipeline_state.rs` `Phase` enum: `Pending, Strategizing, Implementing, Gating, AutoFixing` -- no `WaveGating` variant | Every task triggers individual gate runs (compile+clippy+test), 10-100x slower than wave-level |
| **APR-4** | No "do not build" prompt injection | `roko-compose/src/prompt_assembly_service.rs` has 9-layer builder but no `BuildPolicy` concept | Agents run `cargo build/check/test` during task execution, burning 15-40 min per task when deferred gating would cost 3-8 min per wave |
| **APR-5** | No cumulative context between tasks | Effect driver dispatches agents with task prompt only; no "what changed before you" section | Agents in later waves don't know what earlier tasks modified; ~30% merge conflict rate |
| **APR-6** | No anti-pattern pre-gate | `roko-gate/src/` has 11 gate types but no grep-based anti-pattern checker | Structural LLM mistakes (stub gates, inline prompts, `block_on` in async) only caught by slow compilation |
| **APR-7** | Plan run CLI missing `--gate-mode`, `--only`, `--pause` flags | `roko plan run` has `--dry-run`, `--resume-plan`, `--fresh`, `--approval`; missing operational flags from mega-parity runner | Cannot control gate deferral, selective task execution, or inter-wave inspection from CLI |
| **APR-8** | TaskScheduler has no priority ordering | `roko-runtime/src/task_scheduler.rs` `next_batch()` returns ready tasks without priority sorting | Critical-path tasks may be delayed behind non-critical work, extending total run time |
| **APR-9** | No model escalation on repeated failure | `roko-agent/src/task_runner.rs` has `TaskRunnerError::ModelEscalation` variant but the runner event loop does not use it to switch models | Cheap model failures burn retries without trying a stronger model; 5-10% of tasks need escalation |
| **APR-10** | No disk space monitoring before worktree creation | `WorktreeManager::create()` creates worktrees without checking available disk | 15 worktrees at 500MB each + cargo builds can exhaust disk; silent failures |

---

## Existing Infrastructure (Do Not Rebuild)

These components are built and tested. Tasks below wire them; they do NOT
reimplement any of this.

| Component | Location | Key Methods |
|---|---|---|
| `WorktreeManager` | `crates/roko-orchestrator/src/worktree.rs` | `create()`, `create_for_plan()`, `touch()`, `health()`, `clear_stale_locks()`, `prune()` |
| `MergeQueue` | `crates/roko-orchestrator/src/merge_queue.rs` | `enqueue()`, `reserve_next_mergeable()`, `mark_merged()`, `mark_failed()` |
| `PlanMerger` | `crates/roko-cli/src/runner/merge.rs` | `submit()`, `drain_next()`, `GitMergeBackend`, `CargoCheckRegressionGate` |
| `UnifiedTaskDag` | `crates/roko-orchestrator/src/dag.rs` | `waves()`, `critical_path()`, `fuse_linear_chains()`, `cpm_analysis()` |
| `TaskScheduler` | `crates/roko-runtime/src/task_scheduler.rs` | `next_batch()`, `mark_running()`, `mark_done()`, `mark_failed()` |
| `PostMergeRunner` | `crates/roko-orchestrator/src/post_merge.rs` | `check()` -- regression detection after merge |
| `PipelineStateV2` | `crates/roko-runtime/src/pipeline_state.rs` | `step()` -- pure state machine |
| `EffectDriver` | `crates/roko-runtime/src/effect_driver.rs` | Side-effect executor for pipeline |
| `WorkflowEngine` | `crates/roko-runtime/src/workflow_engine.rs` | Ties pipeline + effect driver |
| `ReplanStrategy` | `crates/roko-orchestrator/src/replan.rs` | `FailureDisposition`, `PlanRevisionRequest`, `ReplanResult` |
| `RepairEngine` | `crates/roko-orchestrator/src/repair.rs` | `FailureContext`, `RepairDecision`, `RepairAction` |
| `PromptAssemblyService` | `crates/roko-compose/src/prompt_assembly_service.rs` | 9-layer prompt builder with knowledge/episodes/playbook injection |
| `OrchestratorSnapshot` | `crates/roko-orchestrator/src/runtime_snapshot.rs` | `with_merge_queue()`, `with_worktrees()`, `to_json()`, `from_json()` |
| `TaskDefFingerprint` | `crates/roko-cli/src/runner/persist.rs` | `from_task()` -- already computes task fingerprints |
| `RunStateSnapshot` | `crates/roko-cli/src/runner/persist.rs` | `atomic_write()`, `append_jsonl()`, `recover_jsonl()` |
| `TokenCounter` | `crates/roko-compose/src/token_counter.rs` | `for_model()`, `count()` |
| `ContextPackCache` | `crates/roko-learn/src/context_pack_cache.rs` | Fingerprint-keyed prompt cache |

---

## Phase 0: Worktree Isolation (Tasks 14.1-14.5)

Foundation: each task gets its own git worktree. Never a shared working
directory for parallel tasks.

### Task 14.1: Wire WorktreeManager into runner event loop for per-task worktrees

**Resolves**: APR-1

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- add worktree allocation before agent dispatch
- `crates/roko-cli/src/runner/types.rs` -- add `worktree_manager` field to `RunConfig`

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/worktree.rs` -- `WorktreeManager::create()`, `WorktreeHandle`
- `crates/roko-cli/src/runner/merge.rs` -- `PlanMerger` already wraps `MergeQueue`

**What**: When the runner event loop dispatches a task to an agent, allocate a
worktree via `WorktreeManager::create()` and set the agent's working directory
to the worktree path. Currently all tasks share the main repo working directory
(`RunConfig::workdir`).

**Steps**:
1. Add `worktree_manager: Option<Arc<WorktreeManager>>` to `RunConfig`
2. Add `worktree_isolation: bool` to `RunConfig` (default `false` for backward compat)
3. In the event loop, when dispatching a task and `worktree_isolation` is true:
   - Call `worktree_manager.create(task_id, branch_name).await?` to get a `WorktreeHandle`
   - Pass `handle.path` as the agent's working directory instead of `config.workdir`
4. After task completes (success or failure), call `worktree_manager.touch(task_id)`
   but do NOT remove the worktree (project rule: never auto-delete worktrees)
5. Store active `WorktreeHandle`s in a `HashMap<String, WorktreeHandle>` on the run
   context for resume support

**Acceptance criteria**:
- `roko plan run` with `worktree_isolation = true` creates worktrees under `.roko/worktrees/`
- Each task agent operates in its own worktree directory
- Worktrees survive after task completion (never auto-deleted)
- `roko plan run` without the flag works exactly as before (no regression)

---

### Task 14.2: Implement three-tier branch model in WorktreeManager

**Resolves**: APR-1 (branch management)

**File to modify**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Implement the source/integration/task branch hierarchy from the
mega-parity runner. Currently `format_branch_name()` creates task branches
but there is no integration branch concept.

**Steps**:
1. Add `pub async fn create_integration_branch(&self, run_id: &str) -> Result<String, WorktreeError>`
   that creates branch `roko/run-{run_id}` from current HEAD
2. Modify `create()` to accept optional `base_branch: Option<&str>` parameter;
   when provided, fork from that branch instead of deriving from `self.config.base_branch`
3. Add `integration_branch: Option<String>` field to `WorktreeConfig` so all
   task worktrees in a run fork from the same integration branch
4. Add `pub fn backup_branch_name(run_id: &str, task_id: &str) -> String` that
   returns `roko/{run_id}-{task_id}-backup-{timestamp}` for retry preservation
5. Update `format_branch_name()` to use pattern `roko/{run_id}-{task_id}`

**Acceptance criteria**:
- Integration branch created once per plan run
- All task worktrees fork from integration branch, not source branch
- `git branch --list 'roko/*'` shows expected hierarchy after a run
- Existing callers of `create_for_plan()` still work (backward compat)

**Depends on**: 14.1

---

### Task 14.3: Wire serialized merge via PlanMerger into runner event loop

**Resolves**: APR-2

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- connect task completion to merge queue

**Files to read** (do not modify):
- `crates/roko-cli/src/runner/merge.rs` -- `PlanMerger::submit()`, `drain_next()`
- `crates/roko-orchestrator/src/merge_queue.rs` -- `MergeQueue::enqueue()`, overlap detection

**What**: After a task completes successfully, enqueue its changes for
serialized merge into the integration branch via `PlanMerger`. The `PlanMerger`
already wraps `MergeQueue` and has `GitMergeBackend` + `CargoCheckRegressionGate`;
it just needs to be called from the event loop.

**Steps**:
1. Add `plan_merger: Option<PlanMerger>` to the run context
2. After a task reaches completion, collect changed files via `git diff --name-only HEAD~1`
   in the task's worktree
3. Build `MergeRequest { plan_id, branch_name, files_changed, priority }` and
   call `plan_merger.submit(request, gate_tx)`
4. Process merge completions via `drain_next()` in the event loop's select
5. On merge conflict, record the conflict details and mark the task for
   reprocessing (don't silently swallow)
6. On merge success, update the integration branch state

**Acceptance criteria**:
- Task merges are serialized (no concurrent `git merge` operations)
- Tasks touching disjoint files merge without waiting for each other
- Tasks touching overlapping files are serialized by the queue
- Merge conflicts are recorded and reported, not silently swallowed

**Depends on**: 14.1, 14.2

---

### Task 14.4: Add worktree disk space monitoring

**Resolves**: APR-10

**File to modify**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Monitor available disk space before creating worktrees. The mega-parity
runner showed 15 worktrees at 500MB each = 7.5GB, plus cargo builds can add
5-15GB per worktree. Pause dispatch when space is low.

**Steps**:
1. Add `fn check_disk_space(path: &Path) -> Result<DiskStatus, WorktreeError>`:
   ```rust
   pub struct DiskStatus {
       pub available_bytes: u64,
       pub total_bytes: u64,
   }
   ```
   Use `statvfs` on Unix (via `nix` crate or raw libc)
2. Add `min_disk_bytes: u64` to `WorktreeConfig` (default 5GB = 5_368_709_120)
3. In `create()`, call `check_disk_space()` before creating. Return
   `WorktreeError::InsufficientDisk { available, required }` when below threshold
4. Add a new `InsufficientDisk` variant to `WorktreeError`
5. Emit tracing warning when available space < 2x the estimated worktree size

**Acceptance criteria**:
- `WorktreeManager::create()` fails gracefully when disk is below threshold
- Warning logged when disk space is low but not critical
- Disk check does not block normal operations (< 1ms)
- New error variant has a clear user-facing message

---

### Task 14.5: Add worktree cleanup utilities (non-destructive)

**File to modify**: `crates/roko-orchestrator/src/worktree.rs`

**What**: Add methods for cleaning up worktree artifacts (build caches, temp
files) without deleting worktrees or branches. Incremental build caches are the
primary disk consumer (~2GB per worktree with cargo builds).

**Steps**:
1. Add `pub async fn clean_build_cache(&self, task_id: &str) -> Result<u64, WorktreeError>`
   that removes `target/` directories within the worktree; returns bytes freed
2. Add `pub async fn clean_all_build_caches(&self) -> Result<u64, WorktreeError>` for batch cleanup
3. Add `pub fn worktree_sizes(&self) -> Result<Vec<(String, u64)>, WorktreeError>` for monitoring
4. Do NOT add any method that deletes worktrees or branches automatically
   (project rule: never delete worktrees or branches)
5. Extend existing `clear_stale_locks()` (already at line 594) to also handle
   locks in worktree subdirectories

**Acceptance criteria**:
- `clean_build_cache()` removes only `target/` directories, nothing else
- Worktree source files and git history are never touched
- `worktree_sizes()` returns accurate sizes for monitoring display
- No method auto-deletes worktrees or branches

---

## Phase 1: Wave Gating (Tasks 14.6-14.10)

Defer compilation to wave boundaries instead of per-task verification.
10-100x speed improvement measured in production.

### Task 14.6: Add WaveGating phase to PipelineStateV2

**Resolves**: APR-3

**File to modify**: `crates/roko-runtime/src/pipeline_state.rs`

**What**: Extend the pipeline state machine with a `WaveGating` phase that
accumulates completed tasks and triggers gates at wave boundaries. Currently
`Phase` gates every task individually.

**Steps**:
1. Add `WaveGating` variant to the `Phase` enum (after `Gating`)
2. Add `wave_gate_mode: WaveGateMode` to `WorkflowConfig`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
   pub enum WaveGateMode {
       #[default]
       PerTask,       // current behavior
       PerWave,       // gate after each wave completes
       Deferred,      // gate only at end of plan
   }
   ```
3. When `wave_gate_mode` is `PerWave`, the state machine transitions from
   `Implementing` to `WaveGating` only when all tasks in the current wave
   have reached `AgentCompleted`
4. In `WaveGating`, emit `PipelineOutput::RunGates` once for the entire wave
5. On gate success, transition to dispatching the next wave
6. On gate failure, include which wave failed and the gate output
7. `PerTask` produces identical behavior to current (no regression)

**Acceptance criteria**:
- `WaveGateMode::PerTask` produces identical behavior to current
- `WaveGateMode::PerWave` runs gates once per wave, not per task
- State machine transitions covered by unit tests
- `WaveGateMode::Deferred` gates only at end of plan

---

### Task 14.7: Implement no-build context injection

**Resolves**: APR-4

**File to modify**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: When wave gating is active, inject a "do not compile" instruction
into the system prompt. Mega-parity runner proved this reduces task time from
15-40 min to 1-5 min with ~95% compliance (99% when placed in system prompt).

**Steps**:
1. Add `BuildPolicy` enum to the prompt assembly module:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
   pub enum BuildPolicy {
       #[default]
       Allowed,
       Prohibited,
   }
   ```
2. Add `build_policy: BuildPolicy` to `PromptSpec` (or the assembly input struct)
3. When `BuildPolicy::Prohibited`, inject as a high-priority system prompt
   section (layer 1, before task description):
   ```
   IMPORTANT: Do NOT run `cargo build`, `cargo check`, `cargo test`, `cargo clippy`,
   or any other compilation command. The runner will verify your changes at the wave
   gate. Focus only on writing correct code.
   ```
4. Place in layer 1 (not a context file) per lesson: system prompt placement
   achieves 99% compliance vs 95% for context files
5. When `WaveGateMode::PerWave` or `Deferred`, automatically set `BuildPolicy::Prohibited`

**Acceptance criteria**:
- Agents dispatched under wave gating receive the no-build instruction
- Instruction appears early in the system prompt (layer 1)
- Agents dispatched without wave gating do NOT receive the instruction
- Per-task override allows specific tasks to build when needed

**Depends on**: 14.6

---

### Task 14.8: Wire wave gate execution into runner event loop

**Resolves**: APR-3 (execution side)

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- wave gate dispatch
- `crates/roko-cli/src/runner/gate_dispatch.rs` -- wave-level gate runner

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/post_merge.rs` -- `PostMergeRunner::check()`
- `crates/roko-cli/src/runner/merge.rs` -- `CargoCheckRegressionGate`

**What**: When the state machine emits wave-level gate events, execute gates
against the integration branch (where all wave task merges accumulated), not
against individual worktrees.

**Steps**:
1. Add `async fn run_wave_gate(integration_dir: &Path, gate_configs: &[GateConfig]) -> Vec<GateVerdict>`
   to `gate_dispatch.rs`
2. The method runs in the integration worktree directory, which has all merged
   task changes for the wave
3. Execute configured gates in order: compile -> clippy -> custom shell -> test
4. Collect all verdicts and return them as a batch
5. Track wave gate duration and emit it as a runner event
6. If any gate fails, include raw output for failure attribution (Task 14.10)

**Acceptance criteria**:
- Wave gates run in the integration worktree, not individual task worktrees
- Gate output includes enough information to identify which task caused failure
- Wave gate duration is tracked and reported
- All configured gates (compile, clippy, test, custom) are supported

**Depends on**: 14.3, 14.6

---

### Task 14.9: Add gate mode and build policy CLI flags to `roko plan run`

**Resolves**: APR-7 (gate mode)

**Files to modify**:
- `crates/roko-cli/src/main.rs` -- add CLI args to `PlanCmd::Run`
- `crates/roko-cli/src/commands/plan.rs` -- map CLI args to `RunConfig`

**What**: Expose wave gate mode as CLI flags on `roko plan run`, matching the
mega-parity runner's operational controls.

**Steps**:
1. Add to `PlanCmd::Run` in `main.rs`:
   ```rust
   #[arg(long, value_enum, default_value = "per-task")]
   gate_mode: GateMode,  // per-task, per-wave, deferred
   #[arg(long)]
   no_gate: bool,        // shorthand for --gate-mode deferred
   #[arg(long)]
   no_build: bool,       // force BuildPolicy::Prohibited regardless of gate mode
   ```
2. Map CLI flags to `RunConfig` fields (add `gate_mode` and `build_policy` to `RunConfig`)
3. `--no-gate` is equivalent to `--gate-mode deferred`
4. Add `[execution]` section to `roko.toml` schema for persistent defaults:
   ```toml
   [execution]
   gate_mode = "per-task"
   ```
5. CLI flags override TOML config

**Acceptance criteria**:
- `roko plan run --gate-mode per-wave` uses wave-level gating
- `roko plan run --no-gate` defers all gating to end of run
- `roko plan run` without flags uses per-task gating (backward compatible)
- `roko plan run --no-build` injects build prohibition regardless of gate mode

**Depends on**: 14.6

---

### Task 14.10: Implement wave gate failure bisection

**Files to modify**:
- `crates/roko-cli/src/runner/gate_dispatch.rs` -- bisection logic

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/dag.rs` -- wave task lists

**What**: When a wave gate fails, determine which task(s) in the wave caused
the regression. The mega-parity runner used `git log` + bisect across merge
commits.

**Steps**:
1. Add `async fn bisect_wave_failure(integration_dir: &Path, wave_task_ids: &[String], gate_fn: F) -> Vec<String>`
2. Retrieve list of merge commits in the wave from `git log --merges`
3. For each merge commit, check if reverting it fixes the gate failure:
   `git revert --no-commit <merge_sha>` -> run gates -> `git reset --hard`
4. Return the task IDs whose merge commits, when reverted, fix the failure
5. Mark offending tasks for retry with gate failure output as context
6. Log the bisection process for debugging

**Acceptance criteria**:
- Bisection correctly identifies the task that introduced a compile error
- Offending task is retried with failure context from gate output
- Non-offending tasks in the wave are not retried
- Bisection works for multiple simultaneous offenders

**Depends on**: 14.8

---

## Phase 2: Context Handoff (Tasks 14.11-14.15)

The hardest problem: telling agent B what agent A changed. Cumulative context
reduced merge conflicts by ~40%.

### Task 14.11: Build cumulative context section generator

**File to modify**: `crates/roko-compose/src/prompt_assembly_service.rs` (add helper function)

**Files to read** (do not modify):
- `crates/roko-compose/src/token_counter.rs` -- `TokenCounter::for_model()`, `count()`

**What**: Generate a "What Changed Before You" section that shows each task
what files were modified by prior tasks in the plan.

**Steps**:
1. Define `CompletedTaskSummary`:
   ```rust
   pub struct CompletedTaskSummary {
       pub task_id: String,
       pub files_changed: Vec<(String, i32, i32)>,  // path, lines_added, lines_removed
       pub brief_description: String,
   }
   ```
2. Add `pub fn cumulative_context(completed: &[CompletedTaskSummary], token_budget: usize) -> String`
3. Format as markdown:
   ```
   ## What Changed Before You
   Files modified by prior tasks in this plan:
   - `src/gate/compile.rs` (+45 -12): Added run_compile_gate, modified gate_pipeline
   - `src/lib.rs` (+3 -0): Added `pub mod gate;`
   ```
4. When total tokens exceed `token_budget` (default 4000), truncate oldest
   task summaries first, keeping most recent changes visible
5. Use `TokenCounter` from the compose crate for byte counting

**Acceptance criteria**:
- Cumulative section generated from completed task data
- Token budget respected (never exceeds default)
- Oldest entries truncated first when budget exceeded
- Empty section returned when no prior tasks

---

### Task 14.12: Wire cumulative context into agent dispatch

**Resolves**: APR-5

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- track completed summaries,
  inject context before dispatch

**Files to read** (do not modify):
- `crates/roko-compose/src/prompt_assembly_service.rs` -- `cumulative_context()` from 14.11

**What**: Before dispatching each task's agent, generate the cumulative context
section from all previously completed tasks in the plan and inject it into the
prompt.

**Steps**:
1. Track `completed_summaries: Vec<CompletedTaskSummary>` on the run context
2. After each task completes, collect changed files via `git diff --stat`
   in the task's worktree and append to `completed_summaries`
3. Before dispatching a new task, call `cumulative_context(&completed_summaries, 4000)`
4. Inject the result into the agent's system prompt (layer 5, contextual knowledge)
5. Also inject the list of files the current task will modify (from task config)
   so the agent can check those files against prior changes

**Acceptance criteria**:
- Each dispatched agent receives cumulative section with prior task changes
- Section grows as more tasks complete
- Token budget prevents section from consuming too much context
- First task in a plan receives an empty cumulative section

**Depends on**: 14.1, 14.11

---

### Task 14.13: Implement failure context accumulation for retries

**Files to modify**: `crates/roko-cli/src/runner/event_loop.rs` (or a new helper module)

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/repair.rs` -- `FailureContext` struct (existing, has
  `plan_id`, `task_id`, `retry_count`, `error_summary`, `failed_gate`)
- `crates/roko-cli/src/runner/persist.rs` -- serialization helpers

**What**: When a task fails and is retried, accumulate structured failure context
(gate output, diff, error pattern) so the retry agent has full information.
The `FailureContext` struct already exists in `roko-orchestrator/src/repair.rs`
but is not populated with gate output or diff data in the runner.

**Steps**:
1. After gate failure, capture: gate name, truncated gate output (2000 chars),
   the agent's diff (`git diff` in worktree), and any detected error pattern
2. Format as a "Previous Attempts" prompt section:
   "Attempt 1 failed because: [gate output]. Your changes: [diff summary]."
3. On retry dispatch, inject this section into the system prompt
4. For attempt 3+, include context from all prior failures
5. Ensure failure history survives checkpoint/resume (serialize to RunStateSnapshot)

**Acceptance criteria**:
- Retry attempts receive full context from all prior failures
- Failure context truncated to prevent exceeding token budgets
- Failure history survives checkpoint/resume (serializable)
- Third attempt includes context from both attempt 1 and attempt 2

---

### Task 14.14: Implement structured handoff documents for multi-role workflows

**File to modify**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: For multi-pass workflows (strategist -> implementer -> reviewer),
generate structured handoff documents instead of passing raw strings.

**Steps**:
1. Add `StrategyBrief` struct: `approach`, `key_constraints`, `files_to_modify`,
   `files_not_to_modify`, `estimated_complexity`
2. Add `ReviewFindings` struct: `must_fix: Vec<Finding>`, `nits: Vec<Finding>`
   where `Finding` has `file`, `line`, `description`
3. Add `fn format_strategy_brief(brief: &StrategyBrief) -> String` and
   `fn format_review_findings(findings: &ReviewFindings) -> String`
4. Parse agent output into these structures using regex patterns for common
   formats (numbered lists, `file:line` patterns)
5. Fallback to raw string when parsing fails (no crash on unusual formats)

**Acceptance criteria**:
- Strategist output parsed into `StrategyBrief` with structured fields
- Review findings parsed into `must_fix` and `nit` categories
- Implementer receives formatted brief with clear scope boundaries
- Fallback to raw string when parsing fails

---

### Task 14.15: Add context-pack directory support for shared agent knowledge

**Resolves**: APR-4 (supplementary)

**Files to modify**:
- `crates/roko-compose/src/prompt_assembly_service.rs` -- read context pack dir
- `crates/roko-core/src/config/mod.rs` -- add `context_pack_dir` config field

**Files to read** (do not modify):
- `crates/roko-learn/src/context_pack_cache.rs` -- `ContextPackCache` (existing cache mechanism)
- `crates/roko-fs/src/layout.rs` -- `context_pack_cache_dir()` (existing path helper)

**What**: Support the mega-parity runner's context-pack pattern: a directory
of markdown files prepended to every agent prompt. Rules, architecture,
anti-patterns, performance contracts.

**Steps**:
1. Add `context_pack_dir: Option<PathBuf>` to `PromptAssemblyService` config
2. If set, read all `*.md` files from the directory, sorted by filename
   (00-RULES.md first, 05-NO-BUILD.md last)
3. Concatenate into a single context string with file separators
4. Inject as system prompt layer 2 (after role identity, before domain knowledge)
5. Track total token count; warn if context pack exceeds 8000 tokens
6. Add `[execution.context_pack_dir]` to `roko.toml`; per-plan overrides supported

**Acceptance criteria**:
- Files in context-pack directory injected into every agent prompt
- Files ordered by filename (numeric prefix sorting)
- Warning emitted when pack exceeds 8000 tokens
- Per-plan override works when specified in plan config

---

## Phase 3: Anti-Pattern Pre-Gates (Tasks 14.16-14.19)

Fast grep-based checks catching LLM code-gen mistakes in milliseconds.

### Task 14.16: Implement AntiPatternChecker with configurable rules

**Resolves**: APR-6

**File to create**: `crates/roko-gate/src/anti_pattern.rs`
**File to modify**: `crates/roko-gate/src/lib.rs` -- add `pub mod anti_pattern;`

**What**: A fast, regex-based checker that scans agent output for known LLM
code-generation anti-patterns. Runs in milliseconds, no compilation needed.

**Steps**:
1. Define `AntiPatternRule`:
   ```rust
   pub struct AntiPatternRule {
       pub id: String,              // e.g. "AP-1"
       pub name: String,
       pub pattern: Regex,
       pub description: String,
       pub severity: Severity,      // Error, Warning
       pub file_glob: Option<String>,
       pub exemptions: Vec<String>,
   }
   ```
2. Implement the 10 checks from the mega-parity runner:
   - AP-1: Stub gates returning pass (`Ok(GateVerdict::pass` without real check)
   - AP-2: `block_on` in async code
   - AP-3: Duplicate trait definitions vs foundation types
   - AP-5: Raw `Command::new("claude")` shell-outs
   - AP-6: Inline prompt strings (`format!("You are a"`)
   - AP-7: `std::sync::Mutex` held across `.await`
   - AP-8: Empty function bodies (`{ }` or `{ todo!() }`)
   - AP-9: `unimplemented!` / `unreachable!` left behind
   - AP-10: Hardcoded localhost/port in non-test code
3. Add `AntiPatternChecker::check(files: &[PathBuf]) -> Vec<AntiPatternViolation>`
4. Support per-task exemptions via `ap_exemptions: ["AP-10"]`

**Acceptance criteria**:
- All 10 checks execute in < 100ms for a typical task diff
- Each violation includes: rule ID, file, line number, matched text
- Exemptions work per-task
- False positive rate tracked via violation metadata

---

### Task 14.17: Wire AntiPatternChecker as pre-gate in the runner pipeline

**Resolves**: APR-6 (wiring)

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- run AP checks after agent completion
- `crates/roko-cli/src/runner/gate_dispatch.rs` -- integrate AP check results

**Files to read** (do not modify):
- `crates/roko-gate/src/anti_pattern.rs` -- `AntiPatternChecker::check()` from 14.16

**What**: Run anti-pattern checks after agent completion but before compilation
gates. Catches structural mistakes without waiting for `cargo check`.

**Steps**:
1. After `AgentCompleted`, before transitioning to gate phase, run
   `AntiPatternChecker::check()` on files changed by the agent
2. If any `Severity::Error` violations found, treat as gate failure:
   inject violation details into retry context and re-dispatch
3. If only `Severity::Warning` violations, log but continue to gates
4. In wave-gate mode, still run AP checks per-task (fast enough at <100ms)
   even when compilation is deferred
5. Track AP check duration as a runner event

**Acceptance criteria**:
- Anti-pattern checks run on every task completion, regardless of gate mode
- Error-severity violations trigger immediate retry (no wasted compilation)
- Warning-severity violations logged but do not block
- AP checks complete in < 100ms per task

**Depends on**: 14.16

---

### Task 14.18: Add anti-pattern false-positive tracking and exemption learning

**File to modify**: `crates/roko-gate/src/anti_pattern.rs`

**What**: Track false positive rates per rule and per file pattern. After N
false positives for a rule+file combination, auto-suggest an exemption.

**Steps**:
1. Add `AntiPatternStats` persisted to `.roko/learn/anti-pattern-stats.json`:
   ```rust
   pub struct AntiPatternStats {
       pub per_rule: HashMap<String, RuleStats>,
   }
   pub struct RuleStats {
       pub total_fires: u64,
       pub false_positives: u64,
       pub auto_exemptions: Vec<String>,
   }
   ```
2. When task succeeds on retry after AP failure, mark the prior AP firing
   as a potential false positive
3. When false positive rate for a rule+file exceeds 50% over 10+ firings,
   suggest an exemption (log at warn level)
4. Persist stats after each plan run

**Acceptance criteria**:
- False positive rate tracked per rule
- Stats survive across runs (persisted to disk)
- Auto-exemption suggestions appear in logs when rate is high
- Manual exemption override works via task config

**Depends on**: 14.16

---

### Task 14.19: Add custom anti-pattern rules via roko.toml

**Files to modify**:
- `crates/roko-gate/src/anti_pattern.rs` -- load custom rules
- `crates/roko-core/src/config/mod.rs` -- parse `[[anti_pattern]]` TOML sections

**What**: Allow users to define custom anti-pattern rules in `roko.toml` in
addition to the built-in 10 rules.

**Steps**:
1. Add `[[anti_pattern]]` section to `roko.toml` schema:
   ```toml
   [[anti_pattern]]
   id = "AP-CUSTOM-1"
   name = "hardcoded_api_key"
   pattern = 'sk-[a-zA-Z0-9]{32,}'
   severity = "error"
   file_glob = "*.rs"
   ```
2. Parse custom rules alongside built-in rules in `AntiPatternChecker::new()`
3. Custom rules use same `AntiPatternRule` struct and exemption system
4. Built-in rules can be disabled via `[anti_pattern_defaults] disable = ["AP-10"]`
5. Invalid regex in custom rules produces clear error at config load time

**Acceptance criteria**:
- Custom rules defined in `roko.toml` loaded and applied
- Built-in rules can be disabled per-project
- Custom rules participate in false-positive tracking
- Invalid regex produces clear error at config load time

**Depends on**: 14.16

---

## Phase 4: Resume and Result Tracking (Tasks 14.20-14.24)

The `--continue` pattern: kill-restart-continue at any point.

### Task 14.20: Implement per-task result file tracking for manual intervention

**Files to modify**:
- `crates/roko-cli/src/runner/persist.rs` -- result file I/O
- `crates/roko-cli/src/runner/event_loop.rs` -- write result files on status transitions

**What**: Write per-task `.result` files to disk as a coordination mechanism.
The mega-parity runner proved that simple files on disk enable manual
intervention: mark a task as success, skip it, or force a retry.

Note: `persist.rs` already has `atomic_write()`, `append_jsonl()`, and
`recover_jsonl()`. This task extends that with per-task result files.

**Steps**:
1. Define result file location: `.roko/state/runs/{run_id}/{task_id}.result`
2. Write result file on each task status transition:
   ```json
   {"status": "success", "elapsed_ms": 12345, "commit": "abc123", "files_changed": 3}
   ```
3. Valid statuses: `in_progress`, `success`, `failed`, `blocked`, `skipped`
4. On `--resume-plan`, read all `.result` files and reconstruct task states
5. Support manual override: if a human writes `success` to a `.result` file,
   the scheduler treats that task as completed and unblocks dependents
6. Write result files atomically (use existing `atomic_write()`)

**Acceptance criteria**:
- Each task produces a `.result` file at its designated path
- `--resume-plan` reads result files and skips completed tasks
- Manually writing `success` to a result file unblocks dependents on resume
- Result files written atomically

---

### Task 14.21: Wire TaskDefFingerprint for mid-run edit detection

**Files to modify**:
- `crates/roko-cli/src/runner/resume.rs` -- use fingerprints during resume

**Files to read** (do not modify):
- `crates/roko-cli/src/runner/persist.rs` -- `TaskDefFingerprint::from_task()` already exists

**What**: The `TaskDefFingerprint` struct and `from_task()` method already exist
in `persist.rs`. The resume logic in `resume.rs` already has `prepare_resume()`
which accepts `snapshot_fingerprints`. Verify this is fully wired and handle
the case where a task definition changed between runs.

**Steps**:
1. Verify `TaskDefFingerprint::from_task()` is called at plan load time and
   stored in the checkpoint
2. In `prepare_resume()`, compare stored fingerprints against current task defs
3. If a task's fingerprint changed, mark it as ready for re-execution and
   log: "Task {id} definition changed since last run, re-executing"
4. If dependencies of a changed task need re-running, cascade the reset
5. Report mismatches in `ResumeReport` (already has `TaskMismatch` struct)

**Acceptance criteria**:
- Editing a task's prompt between runs causes it to re-execute on resume
- Unchanged tasks still skipped on resume
- Dependency cascading: if task A changed and task B depends on A, both re-run
- Mismatches reported clearly in resume output

---

### Task 14.22: Harden JSONL recovery for partial writes

**Files to modify**: `crates/roko-cli/src/runner/persist.rs`

**What**: The `recover_jsonl()` function already exists in `persist.rs`. Verify
it handles all edge cases from crash scenarios and add fsync to `append_jsonl()`.

**Steps**:
1. Verify `recover_jsonl()` skips malformed lines with warnings (it does)
2. Add `fsync` call to `append_jsonl()` after write to minimize data loss window:
   ```rust
   file.sync_data()?;  // fsync to ensure durability
   ```
3. Add a recovery mode that detects truncated final entries (incomplete JSON)
   and logs them as warnings rather than errors
4. Ensure `atomic_write()` (already exists) uses write-to-tmp-then-rename
   pattern (verify it does -- it does at line 141)
5. Add a test: simulate crash (truncated write) and verify recovery

**Acceptance criteria**:
- Truncated JSONL lines skipped with warning, not crash
- Complete lines before truncation point successfully recovered
- `append_jsonl()` uses fsync for durability
- Test covers simulated crash recovery

---

### Task 14.23: Add `--only` flag for selective task execution

**Resolves**: APR-7 (partial)

**Files to modify**:
- `crates/roko-cli/src/main.rs` -- add `--only` arg to `PlanCmd::Run`
- `crates/roko-cli/src/commands/plan.rs` -- filter task DAG

**What**: Allow running only specific tasks from a plan, matching the
mega-parity runner's `--only A,B,C` flag.

**Steps**:
1. Add `--only <task_ids>` CLI flag (comma-separated list of task IDs)
2. When `--only` is set, filter the task DAG to include only specified tasks
   and their transitive dependencies
3. Tasks not in the set are marked as `Skipped` in result files
4. Combine with `--resume-plan`: `--only T5,T6 --resume-plan` re-runs T5 and T6
   but skips everything else
5. Validate that all specified task IDs exist in the plan (error early)

**Acceptance criteria**:
- `roko plan run --only T5,T6` runs only T5 and T6 (and their deps)
- Tasks not in the list are skipped
- `--only` combined with `--resume-plan` works correctly
- Invalid task IDs produce clear error before execution starts

---

### Task 14.24: Enhance `--dry-run` with wave plan preview

**Files to modify**:
- `crates/roko-cli/src/commands/plan.rs` -- `cmd_plan_dry_run()` (already exists at line 822)

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/dag.rs` -- `UnifiedTaskDag::waves()`, `critical_path()`

**What**: Enhance the existing `--dry-run` to show wave structure and critical
path analysis. Currently `cmd_plan_dry_run()` exists but may not show wave
breakdown or critical path.

**Steps**:
1. Build the DAG via `UnifiedTaskDag` and compute waves
2. Display wave structure:
   ```
   Wave 0 (3 tasks, parallel):
     T1: "Wire episode logging" [mechanical, 2 files]
     T2: "Wire cascade router"  [mechanical, 1 file]
   Wave 1 (2 tasks, parallel):
     T4: "Wire replan logic" [integrative, 4 files]  deps: T1, T2
   Total: 5 tasks, 2 waves, max parallelism: 3
   ```
3. Highlight critical path from `UnifiedTaskDag::critical_path()`
4. Include file overlap warnings (tasks in same wave touching same files)
5. No git operations or agent dispatches during dry run

**Acceptance criteria**:
- Wave structure displayed with task details and dependencies
- Critical path highlighted
- File overlap warnings shown for potential merge conflicts
- No execution occurs during dry run

---

## Phase 5: DAG Scheduling and Advanced Integration (Tasks 14.25-14.30)

### Task 14.25: Wire critical path priority into TaskScheduler

**Resolves**: APR-8

**Files to modify**: `crates/roko-runtime/src/task_scheduler.rs`

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/dag.rs` -- `critical_path()`, `CpmResult`

**What**: When multiple tasks are ready to dispatch, prioritize critical path
tasks. Currently `next_batch()` returns ready tasks without priority sorting.

**Steps**:
1. Add `priority: TaskPriority` to `SchedulableTask`:
   ```rust
   pub struct TaskPriority {
       pub critical_path: bool,
       pub fan_out: usize,       // number of downstream dependents
       pub tier: u8,             // 0=mechanical, 1=focused, 2=integrative
   }
   ```
2. In `next_batch()`, sort ready tasks by: critical_path desc, fan_out desc, tier asc
3. Critical path tasks dispatched first (cannot afford failure delay)
4. High fan-out tasks next (unblock the most work)
5. Lower-tier tasks before higher-tier (mechanical tasks complete faster)

**Acceptance criteria**:
- Critical path tasks always dispatched before non-critical
- Fan-out priority breaks ties among non-critical tasks
- Priority does not affect correctness (only dispatch order)
- Dispatch order is deterministic for the same DAG

---

### Task 14.26: Implement auto-cherry-pick conveyor belt

**Files to modify**: `crates/roko-cli/src/runner/merge.rs` (extend `PlanMerger`)

**What**: Background process that watches for completed task merges and
cherry-picks them into a target branch. The mega-parity runner's "conveyor
belt" pattern.

**Steps**:
1. Add `AutoPickConfig` to `PlanMerger`:
   ```rust
   pub struct AutoPickConfig {
       pub target_branch: String,
       pub interval_secs: u64,         // polling interval (default 90)
       pub auto_resolve: bool,         // accept --theirs on conflict
       pub verify_after_pick: bool,    // run cargo check after each cycle
   }
   ```
2. Add `spawn_auto_pick(config, merge_queue) -> JoinHandle` that polls for
   completed merges and cherry-picks to target branch
3. On conflict: if `auto_resolve` is true, use `git checkout --theirs .`;
   otherwise mark as needing manual resolution
4. Save pick state to `.roko/state/auto-pick.json` (survives restart)
5. Track cherry-pick events for monitoring

**Acceptance criteria**:
- Completed task changes auto cherry-picked to target branch
- Conflict resolution respects `auto_resolve` config
- Pick state survives process restart
- Cherry-pick progress visible via events/dashboard

**Depends on**: 14.3, 14.20

---

### Task 14.27: Implement model escalation on repeated failure

**Resolves**: APR-9

**Files to modify**:
- `crates/roko-cli/src/runner/event_loop.rs` -- escalation logic on retry

**Files to read** (do not modify):
- `crates/roko-agent/src/task_runner.rs` -- `TaskRunnerError::ModelEscalation` (already exists)
- `crates/roko-learn/src/cascade_router.rs` -- routing observation API

**What**: When a cheap model fails the same gate repeatedly, escalate to a
stronger model. The `TaskRunnerError::ModelEscalation` variant already exists
but the runner event loop does not act on it to switch models.

**Steps**:
1. Add `ModelEscalation` config to `RunConfig`:
   ```rust
   pub struct ModelEscalationConfig {
       pub enabled: bool,
       pub escalation_after: u32,      // attempts before escalating (default 2)
       pub strong_model: Option<String>,  // override; otherwise use CascadeRouter
   }
   ```
2. In the event loop, track attempt count per task
3. When attempt count exceeds `escalation_after`, override the model for the
   next dispatch with `strong_model` (or ask CascadeRouter for a stronger option)
4. Log: "Task {id}: escalating from {cheap} to {strong} after {n} failures"
5. Record escalation as a CascadeRouter observation for future routing

**Acceptance criteria**:
- First 2 attempts use the configured/default model
- Third attempt uses the stronger model
- Escalation logged and trackable
- CascadeRouter receives the observation for future routing

**Depends on**: 14.13

---

### Task 14.28: Wire chain fusion from DAG into TaskScheduler

**Files to modify**: `crates/roko-runtime/src/task_scheduler.rs`

**Files to read** (do not modify):
- `crates/roko-orchestrator/src/dag.rs` -- `fuse_linear_chains()`, `FusionConfig`

**What**: Wire `UnifiedTaskDag::fuse_linear_chains()` into the scheduler so
linear sequences of mechanical tasks are collapsed into single dispatch units.

**Steps**:
1. Add `fusion_enabled: bool` to scheduler construction or a separate config
2. When enabled, call `dag.fuse_linear_chains(&FusionConfig::default())` before
   converting to `SchedulableTask`s
3. Fused tasks get combined prompts: "Step 1: [task A]. Step 2: [task B]."
4. Fused tasks inherit the union of all constituent task file scopes
5. If any step in a fused task fails, entire fused unit fails (but individual
   step results tracked)
6. Default: `max_chain_length=3`, `same_tier_only=true`

**Acceptance criteria**:
- Linear chains of mechanical tasks fused into single dispatch units
- Fused tasks produce correct combined prompts
- Fusion reduces total wave count (verified via `--dry-run`)
- Fused task failure attributed to the failing step

**Depends on**: 14.25

---

### Task 14.29: Enhance `--resume-plan` with parallel re-execution

**Files to modify**: `crates/roko-cli/src/runner/event_loop.rs`

**Files to read** (do not modify):
- `crates/roko-cli/src/runner/resume.rs` -- `prepare_resume()`

**What**: When running with `--resume-plan` after a partial failure, re-execute
only failed/blocked tasks while preserving successful results. Support parallel
re-execution of independent failed tasks.

**Steps**:
1. On resume, load result files (14.20) and task fingerprints (14.21)
2. Classify tasks: `success` -> skip, `failed` -> re-execute, `blocked` -> check
   if dependency now resolved, `in_progress` -> treat as failed (stale)
3. Rebuild the DAG with only tasks that need re-execution
4. Dispatch re-execution tasks in parallel where dependencies allow
5. Merge re-executed tasks via merge queue
6. Support multiple `--resume-plan` cycles (each picks up where last left off)
7. Clear stale worktree locks before re-execution (via `clear_stale_locks()`)

**Acceptance criteria**:
- `--resume-plan` skips successful tasks and re-runs failed ones
- Previously blocked tasks re-evaluated against current dependency state
- Parallel re-execution works for independent failed tasks
- Multiple consecutive resume cycles converge to all-success

**Depends on**: 14.20, 14.21, 14.22

---

### Task 14.30: Add `--pause` flag for inter-wave inspection

**Resolves**: APR-7 (partial)

**Files to modify**:
- `crates/roko-cli/src/main.rs` -- add `--pause` arg to `PlanCmd::Run`
- `crates/roko-cli/src/runner/event_loop.rs` -- pause between waves

**What**: Pause between waves so a human can inspect the merged result, fix
issues, and resume. Essential for large plans where wave gate failures need
human judgment.

**Steps**:
1. Add `--pause` flag to `PlanCmd::Run` in `main.rs`
2. After each wave completes and merges, print summary:
   ```
   Wave 2 complete: 5/5 tasks succeeded, 3 files modified
   Integration branch: roko/run-20260429
   Press Enter to continue, or 's' to stop...
   ```
3. Wait for user input before dispatching next wave
4. On 's': save checkpoint and exit cleanly (resumable with `--resume-plan`)
5. While paused, human can inspect integration branch, run manual tests
6. After resume, re-read integration branch state (human may have made changes)

**Acceptance criteria**:
- `--pause` stops execution between waves and waits for input
- User can inspect and modify integration branch during pause
- Enter resumes execution; 's' saves and exits
- Changes made during pause visible to subsequent waves

**Depends on**: 14.20

---

## Dependency Graph

```
Phase 0 (Worktree Isolation):
  14.1 (WorktreeManager wire)
  14.2 (three-tier branches) ---- depends on 14.1
  14.3 (serialized merge) ------- depends on 14.1, 14.2
  14.4 (disk monitoring) -------- independent
  14.5 (cleanup utilities) ------ independent

Phase 1 (Wave Gating):
  14.6  (WaveGatePhase) --------- independent
  14.7  (no-build injection) ---- depends on 14.6
  14.8  (wave gate execution) --- depends on 14.3, 14.6
  14.9  (CLI flags) ------------- depends on 14.6
  14.10 (bisection) ------------- depends on 14.8

Phase 2 (Context Handoff):
  14.11 (cumulative context) ---- independent
  14.12 (wire into dispatch) ---- depends on 14.1, 14.11
  14.13 (failure context) ------- independent
  14.14 (structured handoff) ---- depends on 14.11
  14.15 (context-pack files) ---- independent

Phase 3 (Anti-Pattern Pre-Gates):
  14.16 (AntiPatternChecker) ---- independent
  14.17 (wire as pre-gate) ------ depends on 14.16
  14.18 (FP tracking) ----------- depends on 14.16
  14.19 (custom rules via toml) - depends on 14.16

Phase 4 (Resume & Result Tracking):
  14.20 (result files) ---------- depends on 14.1
  14.21 (task fingerprints) ----- independent (verify existing)
  14.22 (JSONL recovery) -------- independent (harden existing)
  14.23 (--only flag) ----------- depends on 14.20
  14.24 (--dry-run enhance) ----- independent
  14.30 (--pause flag) ---------- depends on 14.20

Phase 5 (DAG Scheduling):
  14.25 (critical path priority)  independent
  14.26 (auto-cherry-pick) ------ depends on 14.3, 14.20
  14.27 (model escalation) ------ depends on 14.13
  14.28 (chain fusion) ---------- depends on 14.25
  14.29 (parallel resume) ------- depends on 14.20, 14.21, 14.22
```

**Cross-phase independent tasks** (can start immediately):
14.1, 14.4, 14.5, 14.6, 14.11, 14.13, 14.15, 14.16, 14.21, 14.22, 14.24, 14.25

**Critical path** (longest sequential chain):
14.1 -> 14.2 -> 14.3 -> 14.8 -> 14.10

---

## Key Numbers (From Operational Data)

| Metric | Value | Source |
|---|---|---|
| Per-task time (with build) | 15-40 min | Mega-parity runner |
| Per-task time (no build) | 1-5 min | Mega-parity runner |
| Wave gate (cargo check) | 3-8 min | 18-crate workspace |
| Worktree creation | ~2 sec | git worktree add |
| Worktree size | ~500 MB | Source only, no target/ |
| Merge conflict rate | ~30% | Large runs, shared files |
| AP false positive rate | 2-3% | Mostly AP-10 |
| Agent compliance (no-build) | 95-99% | System prompt > context file |
| Optimal parallelism | 15 | MacBook Pro, limited by RAM |

## Decision Matrix

| Plan Size | Recommended Config |
|---|---|
| 1-5 tasks | `--gate-mode per-task` (low overhead, immediate feedback) |
| 5-20 tasks | `--gate-mode per-wave` (10x faster, good safety) |
| 20+ tasks | `--gate-mode deferred --no-build` (maximum speed) |
| Mechanical tasks | Cheap model, no audit, wave gates |
| Architectural tasks | Strong model, per-task gates, structured handoff |
