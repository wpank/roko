+++
id = "gap-cd3529"
kind = "gap"
title = "Integration test C1: one fixture run shows the same honest verdicts on every surface"
status = "open"
triage = "unverified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (canary C1)"
anchors = ["crates/roko-cli/tests/"]
lane = "rust-cold"
parent = "spec-e9d7ec"
links = { depends_on = ["bug-7e1b6b", "bug-a843d4", "bug-94151f", "bug-7eb27e", "gap-29a84b"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn honest_verdicts_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test honest_verdicts_canary"
+++

## Problem

Each fix in epic spec-e9d7ec has its own unit test. Nothing checks, end to end, that one run's verdicts agree across
every place they surface: the Graph checkpoint, the plan outcome, the run metrics, the dashboard snapshot and the
episode records. Without such a check, a later change can bring back a false pass on one of those surfaces without
anyone noticing.

## Why it matters

This test is the exit check for epic spec-e9d7ec, and it becomes C1 in the golden-path acceptance suite (plan epic E11).
The whitepaper's honest-verdicts claim cites it.

## Where

- **New file:** `crates/roko-cli/tests/honest_verdicts_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`. It configures a scripted fake provider through
  `roko.toml` and runs the built binary with `assert_cmd`.
- **Surfaces to assert:**
  - the checkpoint, `.roko/state/graph/<plan>/checkpoint.json`;
  - `roko plan status`;
  - `.roko/learn/run-metrics.jsonl`;
  - `classify_task_outcome` in `crates/roko-core/src/dashboard_snapshot.rs`;
  - `.roko/episodes.jsonl`.

## Current state

No such test exists. The fake-provider pattern exists, and the Graph budget and resume tests use it.

## Plan

1. Write a fixture plan with four independent tasks and a scripted fake provider:
   - T1 passes its verify step;
   - T2 has no verify step;
   - T3's role is disabled;
   - T4's verify step fails.
2. Run it with the built `roko` binary, as `graph_budget_resume.rs` does.
3. Assert:
   - the checkpoint verdicts are passed, unverified, skipped (or failed, depending on bug-a843d4's fix) and failed;
   - the plan outcome is not `succeeded`;
   - the run metrics record one passed task;
   - the dashboard snapshot counts one pass;
   - the episode records mark only T1 as a success.

   Router labels are covered by epic E4's census, not here.

## Done when

- [ ] The test exists and passes.
- [ ] Reverting any one of the epic's fixes makes it fail. Check this once by hand and say so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test builds `roko-cli` but edits no hot file. It can be written while the fixes are in progress, and it merges
  last.
- Keep it under a minute: fake provider only, no network.
