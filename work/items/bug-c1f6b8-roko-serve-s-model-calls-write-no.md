+++
id = "bug-c1f6b8"
kind = "bug"
title = "roko serve's model calls write no costs.jsonl rows, so cost totals miss all serve spend"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-aad63e"
anchors = ["crates/roko-serve/src/dispatch.rs", "crates/roko-agent/src/model_call_service.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-aad63e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib serve_calls_write_cost_rows"
+++

## Problem

roko serve's model calls (template dispatch, distillation) write feedback rows and gateway events but no `costs.jsonl` rows. `roko show costs` and the budget totals therefore miss all serve spend.

## Plan

Have the shared ModelCallService's feedback sink append the cost row, once, as the CLI does. Add a test named `serve_calls_write_cost_rows`.

## Done when

- `cargo test -p roko-serve --lib serve_calls_write_cost_rows` passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-aad63e, during the evening close-out round.
