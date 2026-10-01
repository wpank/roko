+++
id = "bug-6bcd52"
kind = "bug"
title = "commands/plan.rs keeps an unused resolve_budget_ceiling whose 7 tests still expect the old warn-only --budget-override"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d31457"
anchors = ["crates/roko-cli/src/commands/plan.rs::resolve_budget_ceiling", "crates/roko-cli/src/graph_task_dispatch/budget.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d31457"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'fn resolve_budget_ceiling' crates/roko-cli/src/commands/plan.rs"
+++

## Problem

gap-d31457 made `--budget-override` a hard plan ceiling. `commands/plan.rs` still has a second, unused `resolve_budget_ceiling` with 7 tests that assert the old warn-only behaviour. The budget.rs test `overrides_and_a_zero_ceiling_let_a_spent_day_dispatch` still calls `--budget-override` a bypass.

## Plan

Delete the unused function and its tests (or point them at the live path), and rename or reword the budget.rs test.

## Done when

- The verify passes, and the budget tests describe the current behaviour.

## Notes

- Reported on 2026-10-01 by the worker on gap-d31457, during the evening close-out round.
