+++
id = "gap-a21195"
kind = "gap"
title = "PK51 Specs a cheap model can execute: Export the spec features to the M3 self-model"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
rank = 51
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "73a96794e"
source = "tmp/backlog/2026-10-02-complete-and-wire PK51"
anchors = ["crates/roko-learn/src"]
lane = "rust-cold"
parent = "spec-abbc62"
links = { depends_on = ["gap-de0b87", "gap-7ec3ef"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_features_join_every_attempt' crates/roko-learn/src/ && cargo test -p roko-learn --lib spec_features_join_every_attempt"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T19:23:33Z"
commit = "73a96794e"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T17:08:30Z"
forced = false
evidence = "Gate 9b (work/backlog-batch-9b, merged into main as 73a96794e): cargo check --workspace --tests, roko-cli and roko-serve with fault-injection, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 5,854 tests over roko-cli, -learn and -serve, roko-cli bin 436 passed, the golden-path canaries and the loop-audit census run pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass, 159 fault-injection lib tests pass; every [[verify]] passes. PK51 (3240): spec_features.rs joins spec.jsonl records to attempts and every forecast carries the task's spec vector; the gate split its test fixture's json! (5bdd90e2d)."
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK51, slice 32xx, phase 6), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3240 | S | p3 | Export the spec features to the M3 self-model | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3240-export-spec-features-to-m3-self-model.md` |

## Why it matters

Phase 6: M3 calibrated self-model (S04). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3200-specs-a-cheap-model-can-execute.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-learn/src/self_model/spec_features.rs`.

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

- Waits on: PK19 (gap-de0b87), PK49 (gap-7ec3ef).
- Suggested model: sonnet.

## Progress

- 3240: implemented at 1d362d3a8 (cargo verification deferred to the batch gate)
