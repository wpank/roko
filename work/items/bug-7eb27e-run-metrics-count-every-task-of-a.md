+++
id = "bug-7eb27e"
kind = "bug"
title = "Run metrics count every task of a succeeded plan as completed and every task of a failed plan as failed"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_execution", "roko-learn/run_metrics"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G1: run metrics)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1578", "crates/roko-learn/src/run_metrics.rs::PlanMetrics"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_metrics_count_task_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli --lib run_metrics_count_task_verdicts"
+++

## Problem

At the end of a run, `plan_runner.rs` appends one record to `.roko/learn/run-metrics.jsonl` (the block at about lines
1566–1610). For each plan it sets two counts:

- `tasks_completed`: the plan's full task count if the plan succeeded, and 0 otherwise;
- `tasks_failed`: the rest.

Per-task verdicts are never read. Unverified or skipped tasks in a succeeded plan count as completed, and the passed
tasks of a failed plan count as failed.

## Why it matters

Run metrics feed the `roko learn` reports and are the obvious source for run-level numbers in the whitepaper, so they
must agree with the checkpoint's per-task verdicts. This is part of epic spec-e9d7ec.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the run-metrics block, about lines 1566–1610. The per-plan
  count is computed at about line 1578.
- `crates/roko-learn/src/run_metrics.rs:38`: `PlanMetrics`, with the fields `completed`, `tasks_completed` and
  `tasks_failed`.

## Current state

Unchanged at `41c7ffbd6`. The Graph checkpoint already holds each task's verdict (`TaskGateVerdict`), so the data
exists.

## Plan

1. Count each plan's tasks by their checkpoint verdict: passed, unverified, skipped and failed.
2. Add `tasks_unverified` and `tasks_skipped` to `PlanMetrics` with `#[serde(default)]`, so that older rows still
   parse.
3. Keep `completed` meaning "the plan succeeded", decided by the plan-success rule in gap-29a84b.

## Done when

- [ ] A plan with one passed, one unverified, one skipped and one failed task records 1, 1, 1 and 1.
- [ ] Rows written before this change still deserialize.
- [ ] A test named `run_metrics_count_task_verdicts` exists and passes, and the `[[verify]]` command passes.

## Notes

`plan_runner.rs` is a hot file. The portal session's `sched` and learning branches edit it, so start after they merge.

Implemented on `work/bug-7eb27e` at `8e6b49f14`; cargo verification deferred to the batch check.
