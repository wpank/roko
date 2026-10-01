+++
id = "bug-7dbce5"
kind = "bug"
title = "resolved_overrides maps --budget-override 0 to BudgetPolicy::Disabled, unlike the live path"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d31457"
anchors = ["crates/roko-cli/src/resolved_overrides.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d31457"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'BudgetPolicy::Disabled' crates/roko-cli/src/resolved_overrides.rs"
+++

## Problem

After gap-d31457, `--budget-override 0` means "no plan ceiling", and the per-task and daily ceilings still hold. `resolved_overrides.rs` maps it to `BudgetPolicy::Disabled`. Nothing in production reads that field, so it is dead and misleading.

## Plan

Delete the field, or make it match the live path.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-childenv, working on gap-d31457, during the evening close-out round.
