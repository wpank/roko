+++
id = "gap-439794"
kind = "gap"
title = "File-conflict detection before same-wave task dispatch"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-graph/engine"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch"
anchors = ["crates/roko-graph/src/engine.rs::execute_with_status_tracking_parallel", "crates/roko-graph/src/engine.rs::execute_parallel_at_tick_validated", "crates/roko-graph/src/convert.rs", "crates/roko-graph/src/types.rs::Node", "crates/roko-graph/src/fingerprint.rs::plan_graph_fingerprint", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = ["gap-4835e7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'fn same_wave_tasks_with_overlapping_files_are_serialized' crates/roko-graph/src && cargo test -p roko-graph --lib same_wave_tasks_with_overlapping_files_are_serialized"
+++

## Problem

Within one plan, tasks that are ready in the same topological wave are started together. The only limit is a
semaphore sized to the plan's `max_parallel`. Nothing checks whether two of those tasks declare the same `files`.
Unless `--worktree-per-task` is passed, all tasks share one working tree, so two agents can edit the same file at
the same time. One can overwrite the other's work, and whole-project checks see half-written edits.

Observed in dogfood on 2026-09-28 (commit message of `3049b7fcf`): plan `08b` ran 4 tasks at a time in one tree.
T08's whole-project `tsc` gate failed three times on errors in `PlanView.tsx`, a file its sibling T12 was still
editing. FailFast then aborted the plan, although the tree type-checked once T12 finished.

Expected: two tasks whose declared `files` overlap (the same path, or one path inside the other's directory) never
run at the same time in a shared working tree. The later one waits until the earlier one finishes. Tasks that do
not overlap still run in parallel.

## Why it matters

- Goal `core` ("Plan runs work reliably"). Concurrent writes silently lose work or fail gates for reasons outside
  the task. That makes parallel plans unreliable, and parallelism is how plan runs get fast.
- It is a precondition for raising `max_parallel` safely in self-hosting runs without paying for per-task worktrees.
- Related:
  - `gap-4d835d`: the engine runs a plan wave by wave; a scheduler rewrite would touch the same loop.
  - `gap-4835e7` (parked): conflict-aware replan after merge conflicts.
  - `3049b7fcf` mitigates the gate symptom (see Current state).

## Where

- `crates/roko-graph/src/engine.rs`. There are two copies of the parallel wave loop, and both need the fix:
  - `execute_with_status_tracking_parallel` (line 2106): the loop plan runs actually use. The path is
    `GraphEngine::start` (line 1663), then `execute_with_status_tracking` (line 1762), then this function when
    `max_concurrent_nodes > 1`. `run_one_plan` calls `engine.start(cell_ctx)` (`plan_runner.rs:2077`).
  - `execute_parallel_at_tick_validated` (line 944): used by `GraphEngine::execute` and `execute_parallel`, for
    example by `roko graph run`.
  - Both compute `topological_waves` (`crates/roko-graph/src/topo.rs:88`), spawn each ready node into a `JoinSet`,
    and gate it only with `Semaphore::new(max_concurrent_nodes)`. `sem.acquire()` is at lines 1087 and 2320.
- `crates/roko-graph/src/convert.rs`: plan-to-graph conversion.
  - Line 57 sets `graph.policy.max_concurrent_nodes` from `max_parallel`.
  - Lines 191-197 copy each task's `files` into the node config under the `files` key. Only the executor and gate
    use it. Scheduling does not.
- `crates/roko-graph/src/types.rs::Node` (line 81): has `id`, `cell_type`, `config`, `inputs`, `outputs` and
  `execution_class`. There is no field for resource or exclusivity.
- `crates/roko-graph/src/fingerprint.rs::plan_graph_fingerprint` (line 141): the checkpoint identity. It hashes node
  `id`, `cell_type` and `execution_class`, the edges, and the non-default `policy`, but not other node fields.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - Lines 1154-1171: per-task worktrees are opt-in (`--worktree-per-task`).
  - Lines 1906-1911: `max_parallel` comes from `--max-tasks` or `meta.max_parallel`.
- Existing overlap logic to learn from. It is not reusable from `roko-graph`, because it lives in `roko-cli`:
  - `crates/roko-cli/src/graph_execution/plan_set.rs`: `Area::overlaps` and `plan_conflicts`, around lines
    367-513. These are plan-level footprints.
  - `crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs`: `InFlightTasks` (the registry of in-flight
    attempts with their workdir and declared `files`), `lexical` and `declares`.

## Current state

- Re-checked 2026-09-29 at HEAD `a17d9d766`. Neither parallel loop checks file overlap. Inside a plan, nothing
  serializes tasks by `files`.
- `725f21e05` added footprint conflicts across plans (`plan_set.rs`). Overlapping plans no longer share the working
  tree at the same time, but this applies only across plans and only when `max_parallel_plans > 1`.
- `3049b7fcf` (2026-09-28) added settle-and-reverify: a failing verify step waits for in-flight siblings in the same
  workdir and re-runs once. The wait is bounded by `[gates] sibling_settle_secs`, default 600. This reduces false
  gate failures but does not stop two tasks writing one file.
- Per-task worktrees avoid the race. They move the conflict to merge time, and they are opt-in.

## Plan

1. Pick where to enforce exclusion:
   - **A. Engine-level resource locks (recommended).** Add `#[serde(default, skip_serializing_if = "Vec::is_empty")]
     pub exclusive: Vec<String>` to `Node`. Fill it in `convert.rs` from `PlanTaskInfo.files` for the task node,
     and in `ProductionPlanTopology` for executor nodes. In both parallel loops, acquire per-key async locks before
     `sem.acquire()`, so a blocked node does not hold a concurrency slot. Acquire keys in sorted order and all at
     once, to avoid deadlock. Put this in one shared helper, for example a `ResourceLocks` in a new
     `roko-graph/src/resource_lock.rs`. This covers `roko graph run` too, keeps the DAG unchanged, and matches the
     item's `[[verify]]`. `plan_graph_fingerprint` ignores extra node fields, so checkpoints stay resumable.
   - **B. Host-level file leases.** Before the executor launches the agent, extend `InFlightTasks` in
     `sibling_settle.rs` so an attempt waits while another in-flight attempt in the same workdir declares an
     overlapping file. This is the smallest change and needs no engine edit. However, the waiting node holds an
     engine semaphore slot, it covers plan runs only, and the `[[verify]]` must move to a `roko-cli` test.
   - **C. Ordering edges in `convert.rs`.** Not recommended. Extra edges change `plan_graph_fingerprint`, so
     existing checkpoints stop resuming. They also change failure semantics: a `Success` edge makes the second task
     skip when the first fails.
2. Define overlap. Normalize paths lexically, then treat two paths as overlapping if they are equal or one is a
   component-wise prefix of the other (a directory entry covers the files below it). A task with no `files`
   declares nothing and takes no locks. Keep the helper small, and document that it mirrors
   `sibling_settle::declares`.
3. Skip exclusion when tasks do not share a tree. With `--worktree-per-task`, have `plan_runner.rs` turn it off, for
   example by leaving `exclusive` empty or through a builder flag on `GraphEngine`. Do not add a non-default
   `GraphPolicy` field: `non_default_policy` feeds the fingerprint.
4. Emit a `tracing::info!` when a node waits on a lock, naming the conflicting node and file, so a run log shows why
   a task did not start.
5. Add tests in `roko-graph` (`#[cfg(test)]` in `engine.rs`, or the new module):
   - `same_wave_tasks_with_overlapping_files_are_serialized`: two root nodes with overlapping `exclusive` and
     `max_concurrent_nodes = 2`. The cells record a shared "currently running" counter, and the test asserts the
     maximum stays 1. Run it through `GraphEngine::start`, the path plan runs use, and through `execute`.
   - A companion test: disjoint files still overlap in time, with a maximum of 2.

## Done when

- In a shared working tree, two same-wave tasks with overlapping `files` never run at the same time. Disjoint tasks
  still run in parallel.
- Both parallel loops enforce it, and a waiting task does not consume a concurrency slot.
- Existing checkpoints still resume, because the fingerprint is unchanged.
- `grep -rq 'fn same_wave_tasks_with_overlapping_files_are_serialized' crates/roko-graph/src && cargo test -p roko-graph --lib same_wave_tasks_with_overlapping_files_are_serialized`
  passes.

## Notes

- `engine.rs` is the hot core of the only plan executor. Keep the change in one shared helper, run the full
  `roko-graph` tests, and avoid running in parallel with `gap-4d835d`, which rewrites the same wave loops.
- If option B is chosen instead, change the `[[verify]]` to a guarded `roko-cli` test in the same format.
- Tasks with empty `files` are common in hand-written plans and stay unrestricted. Say so in the plan-authoring
  docs if the behaviour is user-visible.
- Do not change `sibling_settle` behaviour here. It stays as the second line of defence for whole-project checks
  across tasks that do not overlap.

## Original notes

Tasks in the same topological wave can write the same file concurrently; check file-set overlap in execute_parallel_at_tick_validated() before launching.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-04: Add file-conflict detection in wave dispatch`

How to verify: Check engine for file-overlap serialization of parallel cells.

Verified 2026-09-28: execute_parallel_at_tick_validated (crates/roko-graph/src/engine.rs:944) launches ready nodes with no file-set overlap check, and nothing in roko-graph or graph_execution/ serializes same-wave tasks by files. Per-task worktree isolation is opt-in (graph_execution/plan_runner.rs:1059, --worktree-per-task), so by default parallel tasks share one workspace.

Rechecked 2026-09-29 at d9e79e9d8: still true inside a plan. 725f21e05 added plan-level footprint conflicts (graph_execution/plan_set.rs:367, :473-513, applied at plan_runner.rs:1087-1093), so overlapping plans no longer share the working tree at once, but only across plans and only when max_parallel_plans > 1. execute_parallel_at_tick_validated (engine.rs:944) still launches same-wave nodes under a semaphore alone; the task files list (roko-graph convert.rs:129, :191-197) is passed to gate config, not used for scheduling. Per-task worktrees remain opt-in (plan_runner.rs:1154-1168, --worktree-per-task). 3049b7fcf re-verifies verify failures caused by concurrent siblings, which mitigates a symptom but does not stop two tasks writing one file.
