+++
id = "gap-9bfb4e"
kind = "gap"
title = "roko plan status's header gives the glass's four readings by the shared rule (design system §4.8): tasks x/y, TIME from the run's start (run_completed.duration_ms once ended), SPENT of the run's cap (recorded per invocation in the run manifest), CHECKS x of y fixed from the start"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 lead: the terminal couldn't join the matched figure on 7.2 (it read 5/5 tasks · $1.32)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/plan_status_view.rs::PlanStatus", "crates/roko-learn/src/telemetry/records.rs::RunInvocation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_reads_like_the_glass' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_reads_like_the_glass"
+++

## Problem

`roko plan status`'s header doesn't give the run's four readings, so the terminal can't show the figure Hermes and
the portal match on. On 7.2 it read "5/5 tasks · $1.32", with no time and no check count. The round's shared rule
(the design system's §4.8, approved by the lead):

- **tasks:** x/y;
- **TIME:** from the run's start, when roko accepted it, in whole seconds rounded down; once the run ended, its
  `run_completed.duration_ms`;
- **SPENT:** "of cap";
- **CHECKS:** x of y, with y fixed from the start (each task's verify steps, the workspace's rungs on each task when
  the plan runs them, and the plan's own checks) and x counted once each, on each task's last try.

The cap is a problem of its own. serve knows a run's cap only while it holds the run (`last_run.max_usd`), and
nothing on disk records it.

## Why it matters

Goal tooling. The terminal is one of the three surfaces in the guide's capture, and it must read like the others.

## Where

- `crates/roko-cli/src/plan_status_view.rs` (the header).
- `crates/roko-cli/src/commands/diagnose.rs` (the readings).
- `crates/roko-learn/src/telemetry/records.rs::RunInvocation` and
  `crates/roko-cli/src/graph_execution/run_manifest.rs` (the cap on disk).

## Plan

1. Red test first:
   - a running plan: the header's readings line reads `1/3 tasks · 2m 05s · $0.84 of $3.00 · 1/3 checks`;
   - once the run ended: `3/3 tasks · 6m 19s · $0.15 · 3/3 checks`, its `run_completed.duration_ms`.
2. Each run manifest invocation records the plan ceiling it ran under (`cap_micro_usd`). A budget raise since moves
   it.
3. The report reads TIME from the run's event index: the first event of the current part, or `run_completed`'s
   `duration_ms`. The cap comes from the manifest, and CHECKS from the plan and each task's last try.
4. The header keeps the plan's directory and state on its first line, and gives the readings on the line under it.

## Done when

The test passes and the `[[verify]]` passes.
