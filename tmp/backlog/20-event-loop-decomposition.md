# Decompose event_loop.rs into focused modules

**Status:** Backlog
**Priority:** P2 — maintainability
**Size:** XL (2–3 weeks of incremental work)
**Tracked in:** `.roko/GAPS.md` § "event_loop.rs is a ~23.1K-line god object"

---

## What this document is

A backlog spec for breaking up the largest source file in the codebase — the runner's
`event_loop.rs` — into a set of focused, independently testable modules. No new
behavior is added. This is purely structural.

---

## Background: the runner module

`crates/roko-cli/src/runner/` is the plan execution engine for roko. When you run
`roko plan run plans/ --engine runner-v2`, this module owns:

- Loading plan definitions from TOML files
- Building the DAG of task dependencies
- Dispatching tasks to LLM agents
- Running gate (validation) pipelines after each task
- Persisting state snapshots to disk so interrupted runs can resume
- Merging agent worktrees back to the main branch
- Learning from outcomes (provider health, playbooks, error patterns)
- Triggering dream consolidation after a run completes
- Cleaning up worktrees, orphan processes, and disk resources

The module currently has 27 source files. Most of the concerns listed above have at
least partial representation in dedicated files. The file counts tell the story:

```
agent_events.rs         892 lines   — AgentEvent parsing and emission
agent_stream.rs         940 lines   — streaming agent output line-by-line
attempt_ownership.rs   1372 lines   — ownership of task attempts across restarts
branch_cleanup.rs       395 lines   — cleaning up merged/abandoned branches
conductor_adapter.rs   1258 lines   — integrating the conductor watcher
deadlines.rs            449 lines   — per-task deadline tracking
extension_loader.rs     (ext loading)
gate_dispatch.rs       2137 lines   — gate pipeline invocation
github_workflow.rs      962 lines   — GitHub CI integration
merge.rs               1122 lines   — merge queue operations
output_sink.rs         1851 lines   — TUI/log output routing
persist.rs             1074 lines   — task state persistence
projection.rs           630 lines   — named-surface projection
prompt_experiments.rs   465 lines   — model A/B experiment assignment
resume.rs               578 lines   — snapshot-based run resumption
snapshot_writer.rs      494 lines   — writing the runner snapshot to disk
state.rs               1586 lines   — RunState: the in-memory execution state
task_dag.rs            1083 lines   — DAG data structure and traversal
types.rs               2645 lines   — RunConfig, RunReport, and supporting types
...
event_loop.rs         23146 lines   — everything else
```

---

## The problem

`event_loop.rs` is 23,146 lines. It contains 225 functions. It defines the primary
public entry point (`pub async fn run(...)`) but also houses large clusters of
functionality that were never moved out during earlier extraction rounds.

**Why this causes pain:**

1. **Merge conflicts on every feature branch.** Any change to agent dispatch, model
   selection, learning integration, cleanup, or replan logic touches this file.
   Concurrent branches collide constantly.

2. **Slow iteration on tests.** There are approximately 9,100 lines of test code
   spread across 11 `#[cfg(test)]` modules inside this file (starting at lines 252,
   14025, 16491, 18645, 21655, 21741, 22003, 22387, 22481, 22638, and 22949).
   Running a single test for, say, disk-budget tracking requires compiling the entire
   23K-line file plus all of its imports.

3. **Cognitive load.** Reading the file to understand one concern requires navigating
   past code for ten others. Functions for dream consolidation sit 10K lines away from
   the model-routing helpers that call them.

4. **The pattern repeats.** `event_loop.rs` replaced `orchestrate.rs` (the previous
   ~21K-line god object). Without deliberate extraction, the same mass will accumulate
   again after the next round of features.

---

## What already exists

The runner module has been extracted incrementally. The following modules already
exist and own coherent slices of the original functionality:

| Module | Purpose |
|---|---|
| `gate_dispatch.rs` (2137 LOC) | Gate pipeline evaluation: builds rung inputs, invokes `roko-gate`, returns `GateCompletion` |
| `persist.rs` (1074 LOC) | Writing task terminal events, ledger entries, and attempt records to disk |
| `snapshot_writer.rs` (494 LOC) | Serializing the `RunState` snapshot to `.roko/state/state-snapshot.json` |
| `merge.rs` (1122 LOC) | Merge queue: queuing, owning, completing, and rolling back worktree merges |
| `branch_cleanup.rs` (395 LOC) | Deleting merged or abandoned git branches after a task completes |
| `resume.rs` (578 LOC) | Reading an existing snapshot and reconstructing `RunState` for `--resume-plan` |
| `attempt_ownership.rs` (1372 LOC) | Tracking which agent owns which task attempt across restarts and cancellations |
| `deadlines.rs` (449 LOC) | Per-task deadline enforcement independent of the main event loop tick |
| `conductor_adapter.rs` (1258 LOC) | Bridging the conductor watcher results into runner state transitions |
| `agent_events.rs` (892 LOC) | `AgentEvent` variants and the logic that maps raw agent output into them |
| `agent_stream.rs` (940 LOC) | Streaming agent subprocess output line-by-line and parsing JSON frames |

These extractions prove the approach works. Each of the above started inside
`event_loop.rs` and was moved without changing any public API.

---

## What remains in event_loop.rs

After the extractions above, the following coherent slices are still inside
`event_loop.rs` as of the 2026-08-16 audit (23,146 lines, 225 functions):

### 1. Agent dispatch and model selection (~1,200 lines)
Functions that choose a model for a task, apply cognitive and daimon routing biases,
enforce phase and tier caps, and fire the pre/post inference hooks.

Key functions: `candidate_model_slugs`, `health_filtered_knowledge_candidates`,
`phase_capped_model`, `model_cap_decision`, `efe_dispatch_tier`,
`cognitive_cost_adjusted_model`, `merge_cognitive_routing_bias`,
`cognitive_dispatch_policy`, `fire_pre_inference_hook`, `fire_post_inference_hook`,
`fire_on_gate_hook`, `fire_on_error_hook`.

### 2. Replan and gate-failure recovery (~800 lines)
Logic that, when a gate fails, decides whether to retry the task, promote it to a
revised task definition, or abandon the plan. Uses the learning store to count past
failures and pick a strategy.

Key functions: `build_gate_failure_plan_revision`, `maybe_apply_gate_failure_plan_revision`,
`gate_failure_revision_failure_key`, `gate_failure_revision_evidence`,
`revised_task_for_gate_failure`, `gate_failure_replan_enabled`,
`gate_failure_replan_cap`, `build_gate_retry_context`, `begin_gate_retry_rollover`,
`publish_gate_failure_diagnosis`, `spawn_cross_cut_gate_failure_cascade`.

### 3. Learning integration (~1,000 lines)
Emission of learning signals after each agent turn and gate: publishing to the
provider health registry, recording error patterns discovered during a gate, writing
efficiency events, seeding playbooks on first run, querying similar episodes, and
formatting the when/then context block injected into the system prompt.

Key functions: `publish_learning_agent_event`, `learning_task_id`,
`record_discovered_error_patterns`, `record_gate_failure_reflection`,
`format_discovered_patterns_section`, `format_similar_episodes_section`,
`format_when_then_playbooks`, `seed_playbooks_if_empty`, `lessons_from_post_gate_reflections`,
`run_advanced_learning_completion`, `record_cli_provider_outcome`.

### 4. Dream and memory triggers (~500 lines)
Post-run callbacks that fire dream consolidation, memory maintenance, episode
compaction, and log rotation. These are "shutdown" concerns that run after the final
plan completes.

Key functions: `run_dream_consolidation_if_enabled`, `run_dream_consolidation`,
`run_memory_maintenance`, `compact_episodes_if_needed`, `rotate_large_logs`,
`register_agent_feed`.

### 5. Resource and worktree cleanup (~700 lines)
Admission checks (disk space before starting), periodic GC during a run, and
post-plan cleanup of orphan worktrees, stale targets, and oversized logs.

Key functions: `run_pre_plan_resource_maintenance`, `run_gc_if_needed`,
`cleanup_orphan_worktrees`, `worktree_cleanup_eligible`, `post_plan_cleanup`,
`check_plan_disk_budget`, `disk_pre_check`, `publish_resource_metric`,
`publish_worktree_count`, `compute_target_dir_size_bytes`.

### 6. Timeout and cancellation (~900 lines)
The per-task and global timeout machinery: persisting timeout ledger entries for
replay, enforcing owned deadlines, cancelling individual attempts, stopping all
agents on shutdown, and restoring failed cancellations on resume.

Key functions: `handle_global_timeout`, `enforce_owned_deadlines`,
`enforce_owned_deadlines_at`, `cancel_exact_attempt`, `stop_all_agents`,
`restore_failed_cancellation`, `timeout_ledger_entry`, `persist_timeout_terminal`,
`replay_timeout_terminals`, `producer_is_gone_at_deadline`.
(Note: the simpler deadline tracking is already in `deadlines.rs`; this cluster is
the heavier cancellation-execution logic that belongs alongside it or in its own
`cancellation.rs`.)

### 7. Report building (~300 lines)
Constructing the `RunReport` and `PlanReport` returned by `run(...)`. Summarizes
task outcomes, budgets, gate results, and timing.

Key functions: `build_report`, `build_plan_report`, `classify_report_task`.

### 8. Test helpers and test modules (~9,100 lines)
Eleven `#[cfg(test)]` modules covering extension startup, E33 telemetry producers,
the main integration tests, post-gate reflections, provider rate limits, error pattern
sharing, worktree lifecycle, plan disk budget, reflection, post-plan cleanup, and disk
budget tracking.

The test code for each cluster above lives interleaved with or near the production
code it exercises. It should move with that code when the production code moves.

---

## Proposed extraction targets

The goal is not to extract everything at once. Each target below is a self-contained
unit of work that can be completed, tested, and committed independently.

### Target A: `runner/dispatch_model.rs`
Move the model selection and routing functions (cluster 1 above) into their own
module. These functions do not touch `RunState` directly — they read `RunConfig` and
return a model slug or dispatch policy. The pre/post inference hooks can move here
too since they logically belong with the dispatch decision.

Estimated size: ~1,200 lines of production code + nearby tests.
Risk: low. Functions are mostly pure or accept `&RunConfig`.

### Target B: `runner/replan.rs`
Move the gate-failure recovery and replan logic (cluster 2) into its own module.
This is already partially isolated — `gate_dispatch.rs` owns gate invocation, but the
decision about what to do after a gate fails lives in `event_loop.rs`.

Estimated size: ~800 lines.
Risk: medium. Several functions call into `RunState` via mutable references; their
signatures will need to accept explicit state parameters rather than closing over a
local binding.

### Target C: `runner/learning_integration.rs`
Move the learning emission functions (cluster 3) into their own module. These already
have a natural boundary: they are called at fixed hooks (after agent turn, after gate,
after plan completion) and do not need to see the full event loop.

Estimated size: ~1,000 lines.
Risk: low. Functions are largely write-only (emit events, write files).

### Target D: `runner/lifecycle.rs`
Combine the dream/memory triggers (cluster 4) and the resource/worktree cleanup
(cluster 5) into a single `lifecycle.rs` module covering startup and shutdown
resource management. These functions are called at the very beginning and very end of
`run(...)`, making them easy to identify and extract.

Estimated size: ~1,200 lines.
Risk: low. Functions are largely async and self-contained.

### Target E: `runner/cancellation.rs`
Move the timeout and cancellation execution logic (cluster 6) into its own module
alongside `deadlines.rs`. The ledger entry serialization and the `cancel_exact_attempt`
machinery form a coherent subsystem.

Estimated size: ~900 lines.
Risk: medium. Cancellation interacts closely with the main event loop tick; some
functions will need to accept explicit channel or state handles rather than captures.

### Target F: `runner/report.rs`
Move the report-building functions (cluster 7) into their own module. These are
purely data transformation — they take a final `RunState` and produce a `RunReport`.
Zero risk of breakage.

Estimated size: ~300 lines.
Risk: very low.

### Target G: colocate tests with their modules
Each of the 11 `#[cfg(test)]` modules in `event_loop.rs` tests a specific
subsystem. After the production code for that subsystem moves to its own module, the
test module should move with it. This is not a separate step — it is part of each of
the A–F targets above.

---

## Approach: incremental extraction

The extraction must be incremental. Attempting to restructure the whole file in a
single branch creates an unresolvable merge conflict with every concurrent change.

**The procedure for each target:**

1. Create a new file at `crates/roko-cli/src/runner/<target>.rs`.
2. Move the identified functions verbatim — no behavior changes.
3. Add `pub mod <target>;` to `runner/mod.rs`.
4. Replace the moved functions in `event_loop.rs` with `use crate::runner::<target>::*;`
   (or explicit imports). The compiler will identify any missed dependencies.
5. Move the corresponding `#[cfg(test)]` module(s) into the new file.
6. Run `cargo test -p roko-cli` and fix any import errors.
7. Commit. The commit message should say "refactor: extract runner/<target>.rs" with no
   functional changes.

Repeat for each target. Verify between moves. Do not combine multiple targets in one
commit.

**Each extraction is a leaf branch.** Rebase onto main before opening a PR.
`event_loop.rs` changes in every batch, so stale diffs will conflict.

---

## Anti-patterns to avoid

**Do not create a `utils.rs` or `helpers.rs`.** If a function does not belong in
any of the named modules, that is a signal that the module list is incomplete — add a
new named module. Generic dumping grounds reproduce the problem at smaller scale.

**Do not change public API during extraction.** The public surface of the runner
module is:
```rust
pub use event_loop::{PlanReport, RunReport, run};
pub use plan_loader::{Plan, load_plan, load_plan_lenient, load_plans, scaffold_missing_crates};
pub use sse_stream::SseStreamClient;
pub use types::RunConfig;
```
These re-exports in `mod.rs` must remain unchanged. If a function was not previously
`pub`, it should not become `pub` just because it moved to a new file; use `pub(super)`
or `pub(crate)` as appropriate.

**Do not extract into a separate crate prematurely.** The runner module is `pub(crate)`
to `roko-cli`. Hoisting it into a new crate (`roko-runner`) is a valid long-term goal
but requires threading workspace dependencies and is far more disruptive than moving
files within the crate. Do that only after the intra-module extractions are done.

**Do not move code and change it at the same time.** A commit that both extracts a
function and changes its behavior is impossible to review. Move first, change second,
in separate commits.

---

## Acceptance criteria

1. After all extractions, `event_loop.rs` is under 5,000 lines. The file retains the
   `pub async fn run(...)` entry point and the high-level event dispatch loop, but
   delegates all substantial logic to the extracted modules.

2. `cargo test -p roko-cli` passes with zero failures after each individual extraction
   commit. No test is deleted or skipped.

3. The public API of the `runner` module is unchanged: `run`, `RunReport`, `PlanReport`,
   `RunConfig`, `Plan`, `load_plans`, `load_plan`, `load_plan_lenient`,
   `scaffold_missing_crates`, and `SseStreamClient` remain re-exported from `runner/mod.rs`
   with identical signatures.

4. Each new module has a module-level doc comment (one paragraph) describing what it
   owns and what it does not own. For example, `dispatch_model.rs` should document that
   it owns model selection but not agent process spawning.

5. Running `cargo clippy -p roko-cli -- -D warnings` is clean after each extraction.

---

## References

- `.roko/GAPS.md` § "event_loop.rs is a ~23.1K-line god object" — the canonical gap entry
- `crates/roko-cli/src/runner/mod.rs` — module registry and public re-exports
- `crates/roko-cli/src/runner/event_loop.rs` — the file being decomposed (23,146 lines as of 2026-08-16)
- Earlier extraction PRs that established the pattern: `gate_dispatch.rs`, `persist.rs`,
  `merge.rs`, `branch_cleanup.rs`, `snapshot_writer.rs`
