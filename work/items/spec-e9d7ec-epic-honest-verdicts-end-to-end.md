+++
id = "spec-e9d7ec"
kind = "spec"
title = "Epic: honest verdicts end to end"
status = "open"
triage = "unverified"
severity = "p0"
goal = "truth"
size = "L"
subsystem = ["roko-cli/graph_execution", "roko-core/dashboard", "apps/portal"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #1-2)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-core/src/dashboard_snapshot.rs::classify_task_outcome"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["bug-7e1b6b", "bug-a843d4", "bug-94151f", "bug-7eb27e", "gap-29a84b", "gap-cd3529", "bug-5b43a9", "gap-3506f1", "bug-b4c565", "gap-191ecd", "gap-a0f18a"], blocks = [], related = ["gap-f4b935", "bug-50caf2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn honest_verdicts_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test honest_verdicts_canary"
+++

## Problem

Per-task verdicts have been honest since 09-28: 0 false greens in 151 passes, down from about 27% before. But several
paths still turn an outcome that was never verified into a pass:

- the dashboard and the portal count unverified and skipped tasks as passed;
- tasks whose role is disabled complete as successes without running their verify steps;
- the T0 reflex path credits its rule with a gate pass before any gate runs;
- run metrics count every task of a succeeded plan as completed;
- a plan can succeed with tasks that have no verify step at all.

## Why it matters

The whitepaper quotes numbers built on these labels, and so does every progress figure and every learning signal
(router reward, episodes, playbooks). This epic is P0 #1–2 in `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`
and the first check of the golden path. Epics E4 (attempt records) and E5 (tier ladder) build on it.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: dispatch and the reflex path.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the plan outcome and the run metrics.
- `crates/roko-core/src/dashboard_snapshot.rs` and `apps/portal/src/lib/runState.ts`: the counting.
- `crates/roko-cli/src/plan_validate.rs`: the lint rules.

## Current state

Checked at `41c7ffbd6`:
- `ForcedAccept` is gone.
- Reflex outputs are stamped `Unverified`.
- Episodes are settled after the gate.

The remaining problems are the children below.

## Plan

This is the implementation plan.

1. **Fix the counting surfaces in parallel,** one agent each: the dashboard and portal (bug-7e1b6b), disabled roles
   (bug-a843d4), reflex credit (bug-94151f) and run metrics (bug-7eb27e).
2. **Set the plan-success rule** (gap-29a84b): a plan succeeds only when every task passed, and `plan validate
   --strict` rejects tasks with no verify step.
3. **Prove it end to end** with integration test C1 (gap-cd3529). The test also joins the golden-path acceptance
   suite (plan epic E11).

## Done when

- [ ] bug-7e1b6b: the dashboard counts unverified and skipped tasks as passed (existing item)
- [ ] bug-a843d4: tasks whose role is disabled pass without running their verify steps (existing item)
- [x] bug-94151f: The reflex path credits its rule with a gate pass before any gate runs
- [x] bug-7eb27e: Run metrics count every task of a succeeded plan as completed and every task of a failed plan as failed
- [x] gap-29a84b: A plan can succeed while some of its tasks never ran a verify step
- [ ] gap-cd3529: Integration test C1: one fixture run shows the same honest verdicts on every surface
- [x] bug-5b43a9: A verify-step timeout is recorded as a permanent failure, and roko diagnose counts no timed-out attempt
- [x] gap-3506f1: [[gates.rungs]] is inert on roko plan run, so workspace gate rungs guard only roko run and roko do
- [x] bug-b4c565: The T0 reflex shortcut can pass a task without running the workspace rungs
- [x] gap-191ecd: The task prompt's Verification Commands list only the task's own steps, not the workspace rungs that will also run
- [x] gap-a0f18a: planemit.py still says the workspace rungs are inert, and plan validate doesn't list which rungs will run
- [ ] The epic's `[[verify]]` command (test C1) passes on the merged branch.

## Notes

- **Existing children keep their goal and severity:** bug-7e1b6b stays in `visibility` (p2), and bug-a843d4 stays in
  `core` (p3). Re-homing existing items was not approved on 2026-09-29, so this epic tracks them through
  `depends_on` and the checklist above. A re-rank to p1 under `truth` is still proposed in `PLAN.md` §5.
- **Hot files:** `graph_task_dispatch.rs` and `plan_runner.rs`. Items 3–5 wait until the portal session's branches have
  merged and the dispatch file has been split (plan item E15.4). bug-7e1b6b and C1 can start sooner.
- **Related:** gap-f4b935 (reviewer verdicts don't gate tasks) and bug-50caf2 (the plan gate runs in the process's
  working directory).
