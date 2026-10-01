+++
id = "gap-4d5e2d"
kind = "gap"
title = "Efficiency tool calls on the Graph path record succeeded = null"
status = "open"
triage = "unverified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-f9ae3e"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-f9ae3e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib efficiency_tool_calls_record_outcome"
+++

## Problem

bug-f9ae3e made `ToolCallMeta.succeeded` an `Option<bool>`, so the Graph path records `null` instead of a fabricated `true`. That is honest, but efficiency events now say nothing about tool outcomes. Filling the field in needs the tool-audit records or the provider's tool results.

## Plan

Join the attempt's tool-audit or provider tool results to its tool calls, and set `succeeded` where the outcome is known. Add a test named `efficiency_tool_calls_record_outcome_*`.

## Done when

- `cargo test -p roko-cli --lib efficiency_tool_calls_record_outcome` passes.

## Notes

- Reported on 2026-10-01 by wk-settle, working on bug-f9ae3e.
