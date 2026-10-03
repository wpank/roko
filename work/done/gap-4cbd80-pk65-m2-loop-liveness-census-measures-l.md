+++
id = "gap-4cbd80"
kind = "gap"
title = "PK65 M2 loop-liveness: Census measures L-M1, L-M3 and L-M4 from their receipts"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
rank = 65
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "313eea495"
source = "tmp/backlog/2026-10-02-complete-and-wire PK65"
anchors = ["crates/roko-learn/src"]
lane = "rust-cold"
parent = "spec-635697"
links = { depends_on = ["gap-c1d920", "gap-7ec3ef", "gap-940e44", "gap-eb39c1", "gap-2e4a81"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn census_measures_meta_loops_from_receipts' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn census_measures_meta_loops_from_receipts"

[closed]
at = 2026-10-04
at_ts = "2026-10-03T23:11:20Z"
commit = "313eea495"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T22:15:12Z"
forced = false
evidence = "Gate 11a (work/backlog-batch-11a, merged into main as 313eea495): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 4,889 tests over roko-cli and roko-learn, roko-cli bin 438 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn integration tests pass; every [[verify]] passes. PK65 1/1 (5135): the measured census folds L-M1 from harness_policy rows, L-M3 from its route rows with prediction receipts, and L-M4 from audit_trust exclusions."
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK65, slice 51xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5135 | S | p3 | Census measures L-M1, L-M3 and L-M4 from their receipts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5135-census-measures-meta-loops-from-receipts.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-learn/src/loop_audit/census.rs`, `crates/roko-learn/src/loop_audit/loops.toml`.

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

- Waits on: PK43 (gap-c1d920), PK49 (gap-7ec3ef), PK60 (gap-940e44), PK63 (gap-eb39c1), PK64 (gap-2e4a81).
- Suggested model: opus.

## Progress

Worker w3 (no cargo; Rust checks deferred to the batch gate), base `af070ee10`, branch `work/gap-4cbd80`:

- 5135: implemented at `df6ead9ac`. The measured census folds L-M1 from M1's `harness_policy` rows (θ digests, S06's fixed 10% holdout, the params_digest receipt checked against the verdict's harness stamp), L-M3 from its epochs' route rows (ladder source = default arm; receipt = the attempt's prediction row), and L-M4 from route rows in which DP4's audit trust left a candidate out (`audit_trust` reason = receipt). loops.toml's notes are updated. Test `census_measures_meta_loops_from_receipts` (ε 8/9, 0.7, 0.9). Implemented on `work/gap-4cbd80` at `df6ead9ac`; cargo verification deferred to the batch gate.
