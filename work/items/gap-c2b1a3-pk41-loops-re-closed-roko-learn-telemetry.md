+++
id = "gap-c2b1a3"
kind = "gap"
title = "PK41 Loops re-closed: `roko learn telemetry check --srm` compares each randomised layer's arm shares with…"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 41
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK41"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-learn/src/telemetry/report.rs"]
lane = "rust-cold"
parent = "spec-c6e21b"
links = { depends_on = ["gap-943046", "gap-1f4bec"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn srm_check_flags_a_skewed_layer' crates/roko-learn/src/ && cargo test -p roko-learn srm_check_flags_a_skewed_layer"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK41, slice 41xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 4130 | S | p2 | `roko learn telemetry check --srm` compares each randomised layer's arm shares with its propensities | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4130-telemetry-check-srm-for-randomised-layers.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/4100-loops-reclosed-on-verified-outcomes.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-learn/src/telemetry/report.rs`.

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

- Waits on: PK35 (gap-943046), PK40 (gap-1f4bec).
- Suggested model: opus.
