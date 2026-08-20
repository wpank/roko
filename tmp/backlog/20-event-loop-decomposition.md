# 20 — Decompose event_loop.rs into Focused Modules

**Priority**: P2 — maintainability; not blocking correctness
**Size**: XL (2–3 weeks of incremental work)
**Crates**: `crates/roko-cli/` (runner submodule)
**Depends on**: None

---

## Background

Roko is a Rust toolkit that builds and runs plans — sequences of tasks executed by LLM
agents. The plan runner (`roko plan run`) lives in `crates/roko-cli/src/runner/`. When
you invoke `roko plan run plans/ --engine runner-v2`, the runner loads task definitions
from TOML files, builds a DAG, dispatches each task to an LLM agent, validates the result
with a gate pipeline (compile, test, clippy), persists the outcome, and then moves to the
next task. The runner also handles cancellation, state snapshots for resume, learning from
outcomes, and cleaning up worktrees and disk resources after a run.

The runner module has 27 source files and most concerns have at least partial representation
in dedicated files: `gate_dispatch.rs` owns gate invocation, `persist.rs` owns disk writes,
`merge.rs` owns the merge queue, `state.rs` owns the in-memory execution state, and so on.
These extractions prove the pattern works.

One file, however, has grown without bound: `event_loop.rs` is 23,154 lines and contains
approximately 225 functions. It is the runner's main event dispatch loop but also houses
large clusters of functionality that were never moved out during earlier extraction rounds.
This file is the most-touched file in the codebase — any change to agent dispatch, model
selection, learning, cleanup, or replan logic touches it — causing merge conflicts on every
concurrent branch.

This item breaks up `event_loop.rs` into 6 focused modules, following exactly the same
procedure that produced the existing extracted modules.

## Current State

**Verified file sizes (as of 2026-08-19):**

| File | Lines | Notes |
|---|---|---|
| `event_loop.rs` | 23,154 | the god file; contains 225 functions |
| `agent_events.rs` | 892 | already extracted |
| `agent_stream.rs` | 940 | already extracted |
| `attempt_ownership.rs` | 1,372 | already extracted |
| `branch_cleanup.rs` | 395 | already extracted |
| `conductor_adapter.rs` | 1,258 | already extracted |
| `deadlines.rs` | 449 | already extracted |
| `gate_dispatch.rs` | 2,137 | already extracted |
| `github_workflow.rs` | 962 | already extracted |
| `merge.rs` | 1,122 | already extracted |
| `output_sink.rs` | 1,851 | already extracted |
| `persist.rs` | 1,074 | already extracted |
| `projection.rs` | 630 | already extracted |
| `prompt_experiments.rs` | 465 | already extracted |
| `resume.rs` | 578 | already extracted |
| `snapshot_writer.rs` | 494 | already extracted |
| `state.rs` | 1,586 | already extracted |
| `task_dag.rs` | 1,083 | already extracted |
| `types.rs` | 2,650 | already extracted |

**Verified locations of function clusters remaining in `event_loop.rs`:**

1. **Model selection / dispatch routing** (lines ~7319–12040, ~1,200 lines of production code):
   - `cognitive_dispatch_policy` (line 7319)
   - `merge_cognitive_routing_bias` (line 7325)
   - `phase_capped_model` (line 7381)
   - `model_cap_decision` (line 7392)
   - `efe_dispatch_tier` (line 7428)
   - `cognitive_cost_adjusted_model` (line 7489)
   - `candidate_model_slugs` (line 11749)
   - `health_filtered_knowledge_candidates` (line 11762)
   - `fire_pre_inference_hook` (line 11806)
   - `fire_post_inference_hook` (line 11837)
   - `fire_on_gate_hook` (line 11873)
   - `fire_on_error_hook` (line 11904)

2. **Gate-failure recovery / replan** (lines ~4467–15631, ~800 lines of production code):
   - `build_gate_retry_context` (line 15323)
   - `maybe_apply_gate_failure_plan_revision` (line 15360)
   - `build_gate_failure_plan_revision` (line 15445)
   - `gate_failure_revision_failure_key` (line 15512)
   - `gate_failure_replan_enabled` (line 15631)
   - `build_gate_retry_context` (line 15323)
   - `begin_gate_retry_rollover` (nearby)
   - `publish_gate_failure_diagnosis` (line 15718)
   - `spawn_cross_cut_gate_failure_cascade` (line 7667)

3. **Learning integration** (lines ~1208–14243, ~1,000 lines of production code):
   - `record_cli_provider_outcome` (line 1208)
   - `publish_learning_agent_event` (line 6682)
   - `learning_task_id` (line 6728)
   - `record_discovered_error_patterns` (line 12040)
   - `record_gate_failure_reflection` (line 12104)
   - `format_when_then_playbooks` (line 12011)
   - `seed_playbooks_if_empty` (line 14243)
   - `run_advanced_learning_completion` (line 13927)
   - `format_discovered_patterns_section` (nearby)
   - `format_similar_episodes_section` (nearby)
   - `lessons_from_post_gate_reflections` (nearby)

4. **Dream / memory post-run triggers** (lines ~5235–14209, ~500 lines of production code):
   - `run_dream_consolidation_if_enabled` (line 13907)
   - `run_dream_consolidation` (line 13975)
   - `run_memory_maintenance` (line 7183)
   - `compact_episodes_if_needed` (line 12291)
   - `rotate_large_logs` (line 12337)
   - `register_agent_feed` (line 14209)

5. **Resource / worktree cleanup** (lines ~1568–12662, ~700 lines of production code):
   - `publish_resource_metric` (line 1568)
   - `run_pre_plan_resource_maintenance` (line 12370)
   - `run_gc_if_needed` (line 12449)
   - `cleanup_orphan_worktrees` (line 12519)
   - `post_plan_cleanup` (line 12662)
   - `check_plan_disk_budget` (line 9139 call site; function nearby)
   - `disk_pre_check` (line 1879 call site; function nearby)
   - `compute_target_dir_size_bytes` (line 5009 call site; function nearby)
   - `worktree_cleanup_eligible` (nearby)

6. **Timeout / cancellation** (lines ~2363–13539, ~900 lines of production code):
   - `timeout_ledger_entry` (line 12827)
   - `persist_timeout_terminal` (line 12942)
   - `replay_timeout_terminals` (line 12951)
   - `handle_global_timeout` (line 13015)
   - `enforce_owned_deadlines` (line 13153)
   - `enforce_owned_deadlines_at` (line 13200)
   - `cancel_exact_attempt` (line 13472)
   - `stop_all_agents` (line 5062 and ~5518 call sites; function nearby)
   - `restore_failed_cancellation` (line 13428)

7. **Report building** (lines 16183–16380, ~300 lines):
   - `build_report` (line 16183)
   - `build_plan_report` (line 16292)
   - `classify_report_task` (line 16348)

8. **Test code** (13 `#[cfg(test)]` modules at lines 252, 14030, 16498, 18448, 18652, 21662, 21748, 22010, 22286, 22394, 22488, 22645, 22956 — approximately 9,100 lines total).

**Public API of the runner module** (must remain unchanged, verified in `mod.rs` lines 61–64):

```rust
pub use event_loop::{PlanReport, RunReport, run};
pub use plan_loader::{Plan, load_plan, load_plan_lenient, load_plans, scaffold_missing_crates};
pub use sse_stream::SseStreamClient;
pub use types::RunConfig;
```

The `pub async fn run(...)` entry point is at `event_loop.rs` line 1718.

## Implementation Plan

Each of the 6 extraction targets below is a self-contained unit of work. Complete, test, and
commit each target independently. Do not combine multiple extractions in one commit.

### Target A: `runner/dispatch_model.rs` (~1,200 LOC)

**What to move:** Model selection and routing functions (cluster 1 above). These functions are
mostly pure — they accept `RunConfig` or simple inputs and return a model slug or policy.
The pre/post inference hooks are async but self-contained.

**Procedure:**

1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/dispatch_model.rs`.

2. Copy these functions verbatim from `event_loop.rs` into the new file (exact lines to
   confirm by grep before moving — lines shift as other changes land):
   `cognitive_dispatch_policy`, `merge_cognitive_routing_bias`, `phase_capped_model`,
   `model_cap_decision`, `efe_dispatch_tier`, `cognitive_cost_adjusted_model`,
   `candidate_model_slugs`, `health_filtered_knowledge_candidates`,
   `fire_pre_inference_hook`, `fire_post_inference_hook`, `fire_on_gate_hook`,
   `fire_on_error_hook`.

3. Add a module-level doc comment:
   ```rust
   //! Model selection and dispatch routing for the plan runner.
   //!
   //! This module owns the logic for choosing which model to call for a given
   //! task, applying cognitive and daimon routing biases, enforcing phase and
   //! tier caps, and firing pre/post-inference safety hooks. It does NOT own
   //! agent process spawning or task dispatch — those live in the dispatcher.
   ```

4. Add `pub mod dispatch_model;` to `runner/mod.rs`.

5. In `event_loop.rs`, replace each moved function body with `use crate::runner::dispatch_model::*;`
   at the top of the appropriate section, or delete the function bodies and add the use statement
   at the top of the file. Let the compiler identify any missed imports.

6. Move the corresponding `#[cfg(test)]` module(s) into `dispatch_model.rs`. The test at
   line 16552 (`merge_cognitive_routing_bias` test) and the tests at lines 16565–16643
   belong in this module.

7. Run `cargo test -p roko-cli`. Fix any import errors.

8. Run `cargo clippy -p roko-cli --no-deps -- -D warnings`. Fix any warnings.

9. Commit with message: `refactor: extract runner/dispatch_model.rs`

**Risk:** Low. Functions are mostly pure or accept `&RunConfig`.

### Target B: `runner/replan.rs` (~800 LOC)

**What to move:** Gate-failure recovery and replan logic (cluster 2 above).

Functions to move (exact line numbers — re-verify by grep before starting):
`build_gate_retry_context` (line 15323), `maybe_apply_gate_failure_plan_revision` (line 15360),
`build_gate_failure_plan_revision` (line 15445), `gate_failure_revision_failure_key` (line 15512),
`gate_failure_replan_enabled` (line 15631), `publish_gate_failure_diagnosis` (line 15718),
`spawn_cross_cut_gate_failure_cascade` (line 7667), and related helpers.

Doc comment for the new module:
```rust
//! Gate-failure recovery and plan revision for the plan runner.
//!
//! This module owns the logic for deciding what to do when a gate fails:
//! retry the task with enriched context, generate a revised task definition,
//! or abandon the plan. It reads gate outputs and learning store history to
//! make the decision. It does NOT own gate invocation (gate_dispatch.rs) or
//! learning event emission (learning_integration.rs).
```

Move the test module that tests `build_gate_retry_context` (lines 18210–18292) into this file.

**Risk:** Medium. Several functions accept `&mut RunState` parameters; their signatures
already take explicit state rather than closing over locals, so the move should be clean.
The call sites in `event_loop.rs` at lines 4467, 4542, 4631, 4686, and 3725 will need
`use crate::runner::replan::*;` imports added.

### Target C: `runner/learning_integration.rs` (~1,000 LOC)

**What to move:** Learning emission functions (cluster 3 above).

Functions to move: `record_cli_provider_outcome` (line 1208), `publish_learning_agent_event`
(line 6682), `learning_task_id` (line 6728), `record_discovered_error_patterns` (line 12040),
`record_gate_failure_reflection` (line 12104), `format_when_then_playbooks` (line 12011),
`seed_playbooks_if_empty` (line 14243), `run_advanced_learning_completion` (line 13927),
and related formatting helpers.

Doc comment:
```rust
//! Learning signal emission for the plan runner.
//!
//! This module owns functions that are called at fixed hooks (after agent turn,
//! after gate, after plan completion) to publish learning signals: provider health
//! updates, efficiency events, playbook seeds, episode queries, and the when/then
//! context block injected into the system prompt. It does NOT own the learning
//! store itself (roko-learn crate) or gate invocation.
```

Move the test at line 21771 (`record_discovered_error_patterns` test) into this file.

**Risk:** Low. Functions are largely write-only (emit events, write files). They accept
explicit path and config parameters.

### Target D: `runner/lifecycle.rs` (~1,200 LOC)

**What to move:** Dream/memory post-run triggers (cluster 4) AND resource/worktree cleanup
(cluster 5). Combine into one module since both concern run startup and shutdown.

Functions to move: `run_dream_consolidation_if_enabled` (line 13907), `run_dream_consolidation`
(line 13975), `run_memory_maintenance` (line 7183), `compact_episodes_if_needed` (line 12291),
`rotate_large_logs` (line 12337), `register_agent_feed` (line 14209), `publish_resource_metric`
(line 1568), `run_pre_plan_resource_maintenance` (line 12370), `run_gc_if_needed` (line 12449),
`cleanup_orphan_worktrees` (line 12519), `post_plan_cleanup` (line 12662), `check_plan_disk_budget`,
`disk_pre_check`, `compute_target_dir_size_bytes`, `worktree_cleanup_eligible`.

Doc comment:
```rust
//! Run lifecycle management for the plan runner.
//!
//! This module owns resource management at run startup (disk checks, GC) and
//! shutdown (dream consolidation, memory maintenance, worktree cleanup, log rotation).
//! It does NOT own per-task agent dispatch or gate invocation. These functions are
//! called at the very beginning and very end of `event_loop::run(...)`.
```

Move the disk-budget and post-plan-cleanup test modules (lines 22488–22645 and 22645–22955)
into this file.

**Risk:** Low. Functions are async and self-contained; no mutable borrow of `RunState`.

### Target E: `runner/cancellation.rs` (~900 LOC)

**What to move:** Timeout and cancellation execution (cluster 6 above).

Functions to move: `timeout_ledger_entry` (line 12827), `persist_timeout_terminal` (line 12942),
`replay_timeout_terminals` (line 12951), `handle_global_timeout` (line 13015),
`enforce_owned_deadlines` (line 13153), `enforce_owned_deadlines_at` (line 13200),
`cancel_exact_attempt` (line 13472), `stop_all_agents` (function; called at lines 5062, 5518, 5589),
`restore_failed_cancellation` (line 13428).

Doc comment:
```rust
//! Timeout enforcement and attempt cancellation for the plan runner.
//!
//! This module owns per-task and global timeout machinery: persisting timeout ledger
//! entries for replay, enforcing owned deadlines, cancelling individual task attempts,
//! stopping all agents on shutdown, and restoring failed cancellations on resume. The
//! simpler deadline tracking (is_past_deadline, tick) lives in deadlines.rs; this module
//! handles the heavier cancellation-execution logic.
```

**Risk:** Medium. `stop_all_agents` and `cancel_exact_attempt` interact closely with the
event loop's `factory` and channel handles. Their signatures already accept explicit handles,
but verify the borrow structure compiles before committing.

### Target F: `runner/report.rs` (~300 LOC)

**What to move:** Report building functions (cluster 7 above).

Functions to move: `build_report` (line 16183), `build_plan_report` (line 16292),
`classify_report_task` (line 16348).

Doc comment:
```rust
//! Report construction for the plan runner.
//!
//! This module transforms a completed RunState into the RunReport and PlanReport
//! returned by `event_loop::run(...)`. It is purely data transformation — no I/O,
//! no agent dispatch. `RunReport` and `PlanReport` types live in types.rs.
```

**Risk:** Very low. Pure data transformation functions.

### Target G: Colocate tests (not a separate step)

Each of the 13 test modules in `event_loop.rs` tests a specific subsystem. Move each test
module into its corresponding new module file as part of that module's extraction (targets
A–F above). Do not leave test code in `event_loop.rs` after its production code has moved.

---

## The Procedure for Each Target (repeated for clarity)

1. Create the new file at `crates/roko-cli/src/runner/<target>.rs`.
2. Move identified functions verbatim — no behavior changes, no renames.
3. Add `pub mod <target>;` to `runner/mod.rs`.
4. Replace the moved functions in `event_loop.rs` with use-imports. Run `cargo check` to find any missed.
5. Move the corresponding `#[cfg(test)]` module(s) into the new file.
6. Run `cargo test -p roko-cli`. Fix import errors.
7. Run `cargo clippy -p roko-cli --no-deps -- -D warnings`.
8. Commit with message `refactor: extract runner/<target>.rs`. No functional changes.

**Rebase onto main before opening a PR.** `event_loop.rs` changes in every batch; stale
diffs will conflict.

---

## Anti-patterns to Avoid

**Do not create `utils.rs` or `helpers.rs`.** If a function does not fit any named module,
add a new named module. Generic dumping grounds reproduce the problem at smaller scale.

**Do not change public API during extraction.** The public surface must remain:
```rust
pub use event_loop::{PlanReport, RunReport, run};
pub use plan_loader::{Plan, load_plan, load_plan_lenient, load_plans, scaffold_missing_crates};
pub use sse_stream::SseStreamClient;
pub use types::RunConfig;
```
These re-exports are in `runner/mod.rs` lines 61–64 and must not change. Private functions
that move files should use `pub(super)` or `pub(crate)`, not `pub`.

**Do not move code and change it in the same commit.** Move first (zero behavior change),
change second (in a separate commit). Mixed move+change commits cannot be reviewed.

**Do not hoist to a separate crate.** `roko-runner` as a standalone crate is a valid future
goal but is far more disruptive. Do intra-module extractions first.

## Acceptance Criteria

1. After all 6 targets, `event_loop.rs` is under 5,000 lines. It retains `pub async fn run(...)`
   at line 1718 (or wherever the compiler stabilizes it) and the high-level event dispatch loop,
   delegating all substantial logic to the extracted modules.

2. `cargo test -p roko-cli` passes with zero failures after each individual extraction commit.
   No test is deleted or skipped.

3. The public API of the `runner` module is unchanged: `run`, `RunReport`, `PlanReport`,
   `RunConfig`, `Plan`, `load_plans`, `load_plan`, `load_plan_lenient`,
   `scaffold_missing_crates`, and `SseStreamClient` remain re-exported from `runner/mod.rs`
   with identical signatures.

4. Each new module has a module-level doc comment (one paragraph) describing what it owns
   and what it does not own (as shown in the doc comments above).

5. `cargo clippy -p roko-cli --no-deps -- -D warnings` is clean after each extraction.

## Verification Checklist

- [ ] `wc -l crates/roko-cli/src/runner/event_loop.rs` after all extractions — under 5,000
- [ ] `cargo test -p roko-cli` — zero failures (run after each target, not just at the end)
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` — clean after each target
- [ ] `cargo check --workspace` — zero errors
- [ ] `grep -n 'pub use event_loop::\|pub use plan_loader::\|pub use sse_stream::\|pub use types::RunConfig' crates/roko-cli/src/runner/mod.rs` — same 4 lines as before
- [ ] `ls crates/roko-cli/src/runner/` shows 6 new files: `dispatch_model.rs`, `replan.rs`, `learning_integration.rs`, `lifecycle.rs`, `cancellation.rs`, `report.rs`
- [ ] Each new file starts with a `//!` module-level doc comment
- [ ] `roko plan run plans/ --engine runner-v2` runs successfully end-to-end after all extractions

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Remove 6 clusters of functions (move to new modules); net reduction from ~23K to ~5K lines |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/mod.rs` | Add `pub mod dispatch_model; pub mod replan; pub mod learning_integration; pub mod lifecycle; pub mod cancellation; pub mod report;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/dispatch_model.rs` | New file: model selection and routing (~1,200 LOC + tests) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/replan.rs` | New file: gate-failure recovery (~800 LOC + tests) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/learning_integration.rs` | New file: learning signal emission (~1,000 LOC + tests) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/lifecycle.rs` | New file: startup and shutdown resource management (~1,200 LOC + tests) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/cancellation.rs` | New file: timeout and cancellation machinery (~900 LOC + tests) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/report.rs` | New file: RunReport / PlanReport construction (~300 LOC) |
