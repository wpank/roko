+++
id = "bug-19b341"
kind = "bug"
title = "A fresh re-run of a plan takes over the previous run's kept attempt branch, because attempt checkout ids hash plan, task and attempt but not the run"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/orchestrator", "roko-cli/graph_execution"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R4-4"
discovered_from = "agent:dogfood-will (round v6 run 4, 2026-10-17)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::format_attempt_worktree_id", "crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-06bd18", "bug-c872f6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fresh_rerun_keeps_the_earlier_runs_attempt_branch' crates/roko-cli/ && cargo test -p roko-cli fresh_rerun_keeps_the_earlier_runs_attempt_branch"
+++

## Problem

A new run of a plan takes over the previous run's kept attempt branch: an attempt's checkout id is a hash of plan,
task and attempt number, with no run in it.

Round v6 dogfood run 4 (2026-10-17, roko b2970cb12) ran `plans/svf-g4` twice with `--fresh`, first pinned to
gpt-5.4-mini, then to claude-sonnet-4-6. Both runs ended "1 attempt branch(es) kept:
roko/attempt/attempt-f5eb1fb23fcb582b2392", the same branch. Its reflog:

```
16:10:34  created at the base aa2f83f           (run 1, the strong run, attempt 1)
16:12:01  8b95d3b  roko: commit accepted attempt  (run 1's accepted work)
16:12:47  reset to the base aa2f83f               (run 2, the top run, attempt 1)
16:13:48  1503ef3  roko: commit accepted attempt  (run 2's accepted work)
```

Run 2 reset run 1's kept branch to the base and committed its own work there. Here nothing was lost: run 1's accepted
work also lives on its batch branch. But a kept attempt holds work no batch has whenever the attempt failed and was
kept to inspect or resume. A fresh re-run of the plan then resets that branch, and the work is gone. Each run also
reports the same name as its own kept branch, so a person can't tell which run a kept branch belongs to.

## Why it matters

Goal `core`. Kept attempt checkouts exist so failed work can be inspected and resumed (`roko attempts`, `roko
resume`). bug-06bd18 made a new run able to start while an older run's checkout is kept; it did so by reusing the
checkout. That turns "kept" into "kept until the next run".

## Where

- `crates/roko-cli/src/orchestrator/worktree/mod.rs::format_attempt_worktree_id` hashes `plan\0task\0attempt`.
  `format_scoped_attempt_worktree_id` adds a namespace, but the Graph path does not use it.
- `crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider` derives the worktree id
  from the attempt id (`worktree_id`).
- `crates/roko-cli/src/commands/attempts.rs` lists and resolves kept attempts by these ids.

## Current state

The ids are unique within a run and collide across runs of the same plan. The `--fresh` flag archives the old run's
state but not its kept attempt branches.

## Plan

1. Scope the attempt checkout id by run: use `format_scoped_attempt_worktree_id(run_id, …)` on the Graph path, and
   keep the old unscoped id readable for existing kept checkouts.
2. A new run never resets a branch that another run kept. If the id must stay plan-scoped, rename the old branch
   first (`roko/attempt/<id>-<run>`) and say so.
3. Add the test `fresh_rerun_keeps_the_earlier_runs_attempt_branch`: run 1 keeps a failed attempt's branch, run 2 of
   the same plan with `--fresh` passes, and run 1's branch still points at run 1's work.

## Done when

- The `[[verify]]` passes.
- Two runs of one plan report different kept attempt branches.

## Notes

- Related: bug-06bd18 (done, the reattach fix), bug-c872f6 (a CLI run's two run ids).
- Evidence: the reflog above, from the scratch project of `tmp/hermes-roko-v6/dogfood-will/evidence/run4/`, and the
  two runs' logs (`tmp/hermes-roko-v6/dogfood-will/logs/g4-strong.out`, `g4-top.out`).
