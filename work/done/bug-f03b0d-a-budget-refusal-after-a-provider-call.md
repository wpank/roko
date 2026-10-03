+++
id = "bug-f03b0d"
kind = "bug"
title = "A budget refusal after a provider call publishes no AgentCompleted, so the dashboard row stays running"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "4f6ae807f"
source = "bug-ba53d1 fix (w4-length, 2026-10-03)"
discovered_from = "bug-ba53d1"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-ba53d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn budget_refusal_after_a_call_closes_the_row' crates/roko-cli/src/ && cargo test -p roko-cli budget_refusal_after_a_call_closes_the_row"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T17:45:03Z"
commit = "4f6ae807f"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T17:16:22Z"
forced = false
evidence = "Gate 9a (work/backlog-batch-9a, merged into main as 4f6ae807f): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 3,538 roko-cli tests, roko-cli bin 436 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), the bench analysis suite 96 passed, the shakedown 8/8 against the batch binary; every [[verify]] passes. A settlement failure after a call now publishes AgentOutput with the budget reason and then AgentCompleted, closing the row (db923ad83); settlement fails only on an invalid cost or a failed checkpoint write."
+++

## Problem

When the budget settlement refuses after a provider call has run, no `AgentCompleted` event is published, so the
attempt's dashboard row stays "running"; with bug-ba53d1's open-row counting, a later spawn on that row isn't counted
as a call either. Found by w4-length while fixing bug-ba53d1 (2026-10-03).

## Why it matters

The dashboard shows a dead attempt as running, and run metrics undercount calls after a budget stop.

## Where

The budget settlement path after a call in `crates/roko-cli/src/graph_task_dispatch/budget.rs` and the attempt's
TUI publishing (`crates/roko-cli/src/graph_task_dispatch/tui_forward.rs`, `runner/tui_bridge.rs`).

## Current state

Every other end of an attempt publishes `AgentCompleted`; this one doesn't.

## Plan

1. Publish `AgentCompleted` (failed, with the budget reason) when the settlement refuses after a call.
2. A test: a call settles over the plan ceiling, the row closes, and the next spawn on the same row counts.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Related: bug-ba53d1 (open-row call counting).
