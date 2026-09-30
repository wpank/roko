+++
id = "gap-1920ba"
kind = "gap"
title = "File exclusion misses read-against-write races: a whole-project verify reads a sibling's half-written file"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-graph", "roko-cli/graph-task-dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "626e182a9"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-scheduler's report on gap-439794, branch work/gap-4d835d)"
anchors = ["crates/roko-graph/src/exclusion.rs", "crates/roko-graph/src/engine.rs::execute_ready_queue", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = [], blocks = [], related = ["gap-439794", "gap-4f3063", "bug-ea9959", "gap-272448"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_whole_project_verify_never_runs_while_a_sibling_edits' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_whole_project_verify_never_runs_while_a_sibling_edits"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 626e182a9. VerifyStep.scope (declared, or inferred from the command) decides what a verify step reads: a step waits, up to sibling_settle_secs, only for siblings editing files inside its scope, and a verifying attempt never counts as an editor. Batch 16c gate on 7902e44a3 (MAIN 626e182a9 has the same code): cargo check --workspace --tests, nightly fmt and clippy -p roko-cli -p roko-core -p roko-graph --keep-going -D warnings clean; lib tests pass: roko-cli 3221 (three known flakes: the turn_policy 1 s test and the verification efficiency wait pass alone; the routing crash-recovery test is a separate WAL-lock flake handed to wk-settle), roko-core 1953, roko-graph 474. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

gap-439794 (`c5466b5cb` on `work/gap-4d835d`) keeps two tasks apart only when their declared write sets (`files`) overlap. A task whose verify step reads the whole project still runs beside a task that is writing some other file. The plan 08b failure that gap-439794 cites is this case:

- T08 declares `planRows.ts`, and T12 declares `PlanView.tsx`, so their write sets are disjoint and exclusion doesn't apply.
- T08's whole-project `tsc` verify read T12's half-written `PlanView.tsx` and failed.

## Why it matters

Most verify steps are whole-project checks (`tsc --noEmit`, `cargo check`, `npm test`). With `max_parallel > 1` in one working tree, they still fail spuriously, which costs retries and records false failures. Fixing this is a precondition for raising `max_parallel` safely (epic spec-a78d57, gap-272448).

## Where

- `crates/roko-graph/src/exclusion.rs` and `crates/roko-graph/src/engine.rs::execute_ready_queue`: write-set admission, on `work/gap-4d835d`.
- `crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs`: the only handling today. When a verify step fails while siblings are mid-attempt, it waits for them (bounded by `[gates] sibling_settle_secs`) and re-runs the step once.

## Current state

`sibling_settle` repairs a failure after the fact, and only among attempts in the same process (gap-4f3063). It is flaky under load (bug-ea9959). In the simple topology a task's execution and its verify are one node. A scheduler read-set there would have to hold the whole tree for the whole task, which would serialize everything.

## Plan

Design choice:

- **Option A (recommended): a verify-scope rule in the dispatcher.** Each shared working tree gets a read/write lease:
  - an agent attempt holds the lease shared while it edits;
  - a whole-project verify step takes it exclusively, so it waits for siblings' current edits, and new edits wait for it;
  - a verify step that declares a scope (paths) conflicts only with writers inside that scope.

  This works in both topologies.
- **Option B: read sets in the scheduler.** Add `Node.reads` next to `exclusive`, with `"."` for whole-tree readers. Only the rich topology has a separate gate node to attach it to.
- **Option C: verify in a snapshot.** Run whole-project verify steps against the base plus the task's own changes. This isolates fully, but costs a checkout and a warm build for each verify.

Steps for Option A:

1. Add the lease next to `sibling_settle`'s registry of in-flight siblings.
2. Treat a verify step as whole-project unless it declares a scope.
3. Keep `sibling_settle` as the fallback for reads nobody declared.
4. Add `a_whole_project_verify_never_runs_while_a_sibling_edits`.

## Done when

- [ ] A whole-project verify step never runs while a sibling that shares the tree is mid-edit.
- [ ] Tasks that don't verify the whole project still run in parallel.
- [ ] The `[[verify]]` command passes. If Option B or C is chosen, move the test and the verify command to the crate that holds the fix.

## Notes

- 2026-09-30, checked at `8a88c6267` (wk-scheduler, design agreed with the coordinator). The premise holds: engine
  exclusion (`exclusion.rs`) covers only declared write sets, and `sibling_settle` repairs a failed verify step only
  after the fact.
- Deferred. Start only after these have landed:
  - wk-tamper's `sibling_settle.rs` changes (gap-6d172d, batch 13);
  - batch 12's `graph_task_dispatch.rs`;
  - wk-gates' `verification.rs` work.
  Agree the `VerifyStep` field with wk-taskdef first; the coordinator will tell it to expect the question.
- Design:
  - Only a whole-project verify step waits for siblings that are mid-edit. A crate- or path-scoped step runs at once.
  - A step's scope comes from a `scope` on `VerifyStep`. Failing that, it comes from its command, conservatively:
    `cargo test -p X` scopes to `crates/X`, and an unknown command counts as whole-project.
  - An attempt that is verifying never counts as an editor: a `verifying` flag beside `settling` in
    `InFlightAttempt`, which `begin_settle` and `settled` also honour.
  - New edits wait for a running whole-project step. That wait goes where an attempt starts, in
    `graph_task_dispatch.rs`.
- Rejected: a blanket wait before every verify step, bounded by `sibling_settle_secs` (600 s by default). It would
  hold every parallel plan behind its siblings' agent turns, giving up makespan for a race that only whole-project
  reads can hit.
- Tests to restructure: `a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles` and
  `a_verify_failure_left_in_a_sibling_file_blames_the_sibling` assume a step runs while a sibling edits. Once
  whole-project steps wait, the settle path covers only edits that start after the step does.
- 2026-09-30, from wk-taskdef (gap-0f3980, not merged yet):
  - `VerifyStep` is free for a `scope` field.
  - `TaskDef` gains `hints: roko_core::TaskHints`. Any new `TaskDef` literal needs `hints: Default::default()` once
    gap-0f3980 lands.
  - The `tasks.toml` keys `exclusive_files` (an `Option<bool>`; `None` means true) and `parallel_group` parse into
    `task.hints`. If the scheduler starts reading either, drop it from `TaskDef::unused_hints()` (PLAN_039).
- 2026-09-30, wk-scheduler: implemented on `work/gap-1920ba` at `11b426da5`, branched from `f712a8e7a`. Cargo
  verification is deferred to the batch check.
  - `VerifyStep.scope` holds the declared reads. `sibling_settle/verify_scope.rs` infers the scope from the command
    when none is declared: `cargo -p X` reads `crates/X`, file tools read their operands, a tool run after `cd dir`
    reads `dir`, and anything else reads the whole project. `sibling_settle/verify_lease.rs` holds the waits:
    `begin_verify`, `begin_step` and `register_when_unread`. `verification.rs` and the attempt start in
    `graph_task_dispatch.rs` call them.
  - One refinement of the design: a scoped step does not always run at once. It waits only for siblings whose `files`
    fall inside its scope, and new edits wait only for running steps that read their files. The verifying flag is
    dropped while `cargo fix` runs, because that writes files.
  - The two settle tests needed no restructuring. Their commands name absolute marker files, which infer as path
    reads, so they still run beside the editing sibling and exercise the settle path.
  - Checks run in the clone:
    - `cargo check --all-targets` on roko-core, roko-cli and roko-serve;
    - the verify test and the new `verify_scope` and `verify_lease` tests;
    - the `graph_task_dispatch`, `task_parser` and `gate_dispatch` lib tests. 250 passed, and one failure is
      unrelated: `turn_policy::tests::a_timed_out_attempt_is_resumed_with_an_escalated_timeout` (a 1 s timeout under
      machine load), which passed 2 of 2 times on its own;
    - nightly fmt, and clippy `-D warnings` on the same crates.
