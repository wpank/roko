+++
id = "gap-3006e9"
kind = "gap"
title = "The Graph engine records no task-ready or dispatch time, so the slot waits of multi-task plans can't be measured"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-graph/engine", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix3's report)"
anchors = ["crates/roko-graph/src/engine.rs", "crates/roko-cli/src/graph_execution/"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = [], blocks = [], related = ["gap-04e8e2", "gap-1cd676"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tasks_record_when_they_became_ready_and_were_dispatched' crates/roko-graph/src/ && cargo test -p roko-graph --lib tasks_record_when_they_became_ready_and_were_dispatched"
+++

## Problem

Nothing in `crates/roko-graph/src/` or `crates/roko-cli/src/graph_execution/` records when a task became ready (all its dependencies settled) or when it was dispatched (a slot and a provider were free). There is no `ready_at`, `dispatched_at` or equivalent in the checkpoint or the activity log. The time a ready task waits for a parallelism slot or a provider rate limit can't be measured.

## Why it matters

- **Scheduler** (epic spec-a78d57): slot waits are the measure of whether the parallelism cap and the rate limits cost time.
- **Benchmark:** S09 §4.9 reports queue waits beside makespan for the plan-level slice. gap-04e8e2 gave the bench records a `queue_wait_s` field, but the Roko arm can only fill it from rate-limit waits seen at the proxy, never from slot waits.

## Where

The Graph engine's node-state transitions (`crates/roko-graph/src/engine.rs`), and the checkpoint and activity log that `graph_execution` writes under `.roko/state/graph/<plan>/`.

## Current state

At 7fa54b873, tasks carry start and end times, but no ready or dispatch time.

## Plan

1. Stamp `ready_at` when a node's last dependency settles, and `dispatched_at` when it gets a slot. Keep both in the checkpoint and the activity log, per attempt.
2. Report the slot wait (`dispatched_at − ready_at`) per task in the run's stats, and let the benchmark's Roko runner sum it per feature.
3. Add `tasks_record_when_they_became_ready_and_were_dispatched`.

## Done when

- [ ] Every task of a Graph run has ready and dispatch times, and its slot wait can be computed from the records.
- [ ] The `[[verify]]` command passes.
