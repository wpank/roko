+++
id = "bug-6bcd52"
kind = "bug"
title = "commands/plan.rs keeps an unused resolve_budget_ceiling whose 7 tests still expect the old warn-only --budget-override"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d31457"
anchors = ["crates/roko-cli/src/commands/plan.rs::resolve_budget_ceiling", "crates/roko-cli/src/graph_task_dispatch/budget.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d31457"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'fn resolve_budget_ceiling' crates/roko-cli/src/commands/plan.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:14Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:36:56Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-d31457 made `--budget-override` a hard plan ceiling. `commands/plan.rs` still has a second, unused `resolve_budget_ceiling` with 7 tests that assert the old warn-only behaviour. The budget.rs test `overrides_and_a_zero_ceiling_let_a_spent_day_dispatch` still calls `--budget-override` a bypass.

## Plan

Delete the unused function and its tests (or point them at the live path), and rename or reword the budget.rs test.

## Done when

- The verify passes, and the budget tests describe the current behaviour.

## Notes

- Reported on 2026-10-01 by the worker on gap-d31457, during the evening close-out round.
- 2026-10-01 (wk-childenv): implemented on work/gap-1555ac; cargo verification deferred to the batch check.
  Deleted the unused `resolve_budget_ceiling` in commands/plan.rs and its 7 tests; the live function's test
  (`a_budget_override_is_a_hard_ceiling`, plan_runner.rs) gained the `--budget-override 0` row. In budget.rs the old
  test is now `no_budget_and_a_zero_daily_ceiling_let_a_spent_day_dispatch`, without the `--budget-override` row, and
  `a_budget_override_leaves_a_spent_day_spent` checks that the override lifts only the plan ceiling.
- Also reworded what still called `--budget-override` a bypass: the doc comments and warnings in budget.rs and
  graph_task_dispatch.rs, the unpriced-day error (now "run with --no-budget"), the `--no-budget` help in main.rs
  (it said it equals `--budget-override 0`, which now keeps the per-task and daily ceilings), and docs/v2. Left:
  `resolved_overrides.rs` still maps `--budget-override 0` to `BudgetPolicy::Disabled`; nothing in production reads it.
