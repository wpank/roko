+++
id = "gap-9e3134"
kind = "gap"
title = "PK13 Attempt ledger: Plan runs price model calls from the dated price snapshot (+1 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
rank = 13
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "5bb643122"
source = "tmp/backlog/2026-10-02-complete-and-wire PK13"
anchors = ["crates/roko-cli/src/dispatch_v2.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-core/src/config/provider.rs"]
lane = "rust-cold"
parent = "spec-99d417"
links = { depends_on = ["gap-f548c1", "gap-a0043b", "gap-08120e"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'pricing_snapshot' crates/roko-cli/src/dispatch_v2.rs"

[[verify]]
command = "grep -rqw 'fn plan_run_prices_from_the_dated_snapshot' crates/roko-cli/src/ && cargo test -p roko-cli plan_run_prices_from_the_dated_snapshot"

[[verify]]
command = "grep -rq 'api_equiv_usd = ' crates/roko-cli/src"

[[verify]]
command = "grep -rqw 'fn verdict_and_cost_rows_carry_the_snapshot_price' crates/roko-cli/src/ && cargo test -p roko-cli verdict_and_cost_rows_carry_the_snapshot_price"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T00:41:57Z"
commit = "5bb643122"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-02T22:19:30Z"
forced = false
evidence = "Gate 5a (merged into main as 5bb643122, tree identical to work/backlog-batch-5a apart from work/): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 12,477 passed over 12 crates, roko-cli bin + golden-path canaries + plan_revise/plan_validate/plan_spec_gate integration tests (only bug-2a31bc's two known alias tests fail), ViabilityBench suite 548 passed, portal vitest + tsc; every [[verify]] passes."
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK13, slice 21xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2114 | M | p2 | Plan runs price model calls from the dated price snapshot | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2114-plan-runs-price-from-the-dated-snapshot.md` |
| 2 | 2115 | S | p1 | Verdicts and cost rows carry the API-equivalent cost, the uncached cost and the price snapshot id | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2115-verdicts-and-cost-rows-carry-the-snapshot-price.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2100-the-attempt-ledger-counts-tokens-and-money.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-core/src/config/provider.rs`.

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

- Waits on: PK07 (gap-f548c1), PK08 (gap-a0043b), PK12 (gap-08120e).
- Suggested model: opus.

## Progress

- 2114: implemented at 90b936ac8; cargo verification deferred to the batch gate.
- 2115: implemented at b95a1549b; cargo verification deferred to the batch gate.
