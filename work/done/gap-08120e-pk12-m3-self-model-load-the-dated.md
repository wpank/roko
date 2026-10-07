+++
id = "gap-08120e"
kind = "gap"
title = "PK12 M3 self-model: Load the dated price snapshot in roko-core and price token usage from it"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 12
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "e53136640"
source = "tmp/backlog/2026-10-02-complete-and-wire PK12"
anchors = ["crates/roko-core/src/config/schema.rs", "crates/roko-core/src/lib.rs"]
lane = "rust-cold"
parent = "spec-99d417"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn pricing_snapshot_reprices_one_row_per_provider' crates/roko-core/ && cargo test -p roko-core pricing_snapshot"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T15:39:04Z"
commit = "e53136640"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-02T11:55:03Z"
forced = false
evidence = "Backlog gate 1: work/backlog-batch-1 at 660e1a8f8 (cargo check, clippy -D warnings, 10,933 lib tests, 16 golden-path and new canaries, 471 ViabilityBench tests, paperlint and status_matrix clean); every [[verify]] of this item passed there. Merged as e53136640 with an identical tree."
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK12, slice 61xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 6104 | S | p1 | Load the dated price snapshot in roko-core and price token usage from it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6104-load-the-dated-price-snapshot-in-roko-core-and-price-token.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/6100-epic-m3-calibrated-self-model.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-core/src/pricing_snapshot.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: nothing.
- Suggested model: opus.

## Progress

- 6104: implemented at 0835464d9. Implemented on `work/gap-08120e` at `0835464d9`; cargo verification
  deferred to the batch gate. The verify's static part (`grep -rqw 'fn pricing_snapshot_reprices_one_row_per_provider'
  crates/roko-core/`) passes; `cargo test -p roko-core pricing_snapshot` (12 tests) is for the gate. Decision 2113
  rule 1 (named, else newest `config/prices/*.toml`, else a built-in copy) is `PriceSnapshot::for_workspace`; an
  unlisted model prices to `None`. Rule 2's new dated snapshot for every routed model and rule 3's per-provider
  billing are not in this task's files (no task owns the first; 2115 owns `provider.rs`).
