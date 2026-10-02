+++
id = "gap-ac2611"
kind = "gap"
title = "PK34 M2 loop-liveness: Add the loop_audit module: LoopSpec, closed reason codes and the registry loader (+2 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 34
size = "M"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK34"
anchors = ["crates/roko-learn/src/lib.rs"]
lane = "rust-cold"
parent = "spec-446a41"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn registry_rejects_loop_without_default_policy' crates/roko-learn/src/loop_audit/ && grep -rqw 'fn reason_code_precedence_picks_cheapest_first' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn registry_rejects_loop_without_default_policy && cargo test -p roko-learn reason_code_precedence_picks_cheapest_first"

[[verify]]
command = "grep -rqw 'fn embedded_registry_declares_every_loop' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn embedded_registry_declares_every_loop"

[[verify]]
command = "grep -rqw 'fn assignment_uniform_sticky_and_independent_across_layers' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn assignment_uniform_sticky_and_independent_across_layers && grep -rqw 'fn composed_route_propensity_sums_to_one' crates/roko-learn/src/loop_audit/ && cargo test -p roko-learn composed_route_propensity_sums_to_one"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK34, slice 51xx, phase 4), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 5102 | S | p2 | Add the loop_audit module: LoopSpec, closed reason codes and the registry loader | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5102-loop-audit-module-specs-reason-codes-registry-loader.md` |
| 2 | 5103 | M | p2 | Embed the loop registry: every S03 loop, with static findings re-verified at HEAD | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5103-embed-loop-registry-with-findings-verified-at-head.md` |
| 3 | 5108 | S | p2 | Loop layers, holdout schedules, nesting, placebo and composed propensities over telemetry::assign | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5108-loop-layers-schedules-nesting-placebo-propensity.md` |

## Why it matters

Phase 4: loops re-closed (S02). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/5100-epic-m2-loop-liveness-audit.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-learn/src/lib.rs`, `crates/roko-learn/src/loop_audit/assign.rs`, `crates/roko-learn/src/loop_audit/canary.rs`, `crates/roko-learn/src/loop_audit/census.rs`, `crates/roko-learn/src/loop_audit/cs.rs`, `crates/roko-learn/src/loop_audit/estimators.rs`, `crates/roko-learn/src/loop_audit/exposure.rs`, `crates/roko-learn/src/loop_audit/faults.rs`, `crates/roko-learn/src/loop_audit/ledger.rs`, `crates/roko-learn/src/loop_audit/loops.toml`, `crates/roko-learn/src/loop_audit/mod.rs`, `crates/roko-learn/src/loop_audit/sim.rs`, `crates/roko-learn/src/loop_audit/spec.rs`, `crates/roko-learn/src/loop_audit/state.rs`.

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
