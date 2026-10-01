+++
id = "bug-71a5e6"
kind = "bug"
title = "GraphRuntimeEventAdapter maps every NodeCompleted to TaskCompleted { passed: true }, so unverified and already_satisfied tasks would read as passed on the runtime path"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "68b6844f3"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on bug-54c729, branch work/bug-4e5a59 at ebf6d4313)"
anchors = ["crates/roko-cli/src/graph_execution/"]
lane = "rust-cold"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-54c729", "gap-cd3529", "bug-7e1b6b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn runtime_adapter_keeps_the_task_outcome' crates/roko-cli/src/ && cargo test -p roko-cli --lib runtime_adapter_keeps_the_task_outcome"
+++

## Problem

`GraphRuntimeEventAdapter` turns Graph run events into runtime events and maps every `NodeCompleted` to `TaskCompleted { passed: true }`. An unverified, skipped or already_satisfied task would read as passed on the runtime path, which the runs route's runtime index ingests. Only tests construct the adapter today, so the bug is latent (wk-runstate).

## Why it matters

Honest verdicts (epic spec-e9d7ec): C1's promise is the same verdict on every surface. A producer that collapses outcomes to passed breaks it the moment it is wired.

## Plan

1. Carry the task's outcome class (`classify_task_outcome`) through the adapter instead of a bare `passed: true`.
2. Add `runtime_adapter_keeps_the_task_outcome`: unverified, already_satisfied and skipped completions keep their class.

## Done when

- [ ] The adapter keeps every outcome class.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-4e5a59` at `68b6844f3`; cargo verification deferred to the batch check.
- Neither event had room for an outcome. `GraphExecutionEvent::NodeCompleted` and `RuntimeEvent::TaskCompleted` now
  carry an optional `outcome` in the dashboard's words, with serde default, and omitted when `None`, so the wire is
  unchanged. The adapter keeps the outcome, and sets `passed` only when `classify_task_outcome` says `Passed`. A
  completion without an outcome reads as before (`passed: true`).
- Consumers report the carried outcome: `core_event_to_dashboard_events`, roko-runtime's dashboard projection, and
  serve's SSE adapter, which adds `outcome` to the `task_completed` data. The runs route already classifies it
  (bug-54c729). `NodeSkipped` already maps to `TaskSkipped`. Every other construction only gains `outcome: None`.
- Still latent: the engine never emits to its event sink (reg-cbfff6), and only tests build the adapter. Whoever
  wires them must fill `outcome` from the task's gate verdict, as `graph_tui_bridge::node_outcome` does.
