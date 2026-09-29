+++
id = "gap-1920ba"
kind = "gap"
title = "File exclusion misses read-against-write races: a whole-project verify reads a sibling's half-written file"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-graph", "roko-cli/graph-task-dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-scheduler's report on gap-439794, branch work/gap-4d835d)"
anchors = ["crates/roko-graph/src/exclusion.rs", "crates/roko-graph/src/engine.rs::execute_ready_queue", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = [], blocks = [], related = ["gap-439794", "gap-4f3063", "bug-ea9959", "gap-272448"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_whole_project_verify_never_runs_while_a_sibling_edits' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_whole_project_verify_never_runs_while_a_sibling_edits"
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
