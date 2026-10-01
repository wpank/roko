+++
id = "bug-e35fdb"
kind = "bug"
title = "roko learn role-costs averages unknown costs as $0 through compute_role_profiles"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-learn/efficiency", "roko-cli/commands/learn"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-0320da at dd7cb7320)"
anchors = ["crates/roko-learn/src/efficiency.rs", "crates/roko-cli/src/commands/learn.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-0320da"], blocks = [], related = ["bug-9bffe2", "bug-9a6799", "bug-0320da"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn role_profiles_skip_unknown_costs' crates/roko-learn/src/ && cargo test -p roko-learn --lib role_profiles_skip_unknown_costs"
+++

## Problem

`roko learn role-costs` (`cmd_learn_role_costs`, `crates/roko-cli/src/commands/learn.rs:2717`) prints per-role cost profiles from `roko_learn::efficiency::compute_role_profiles` (:2740; `efficiency.rs:736`). `compute_role_profiles` averages every event's cost, so events whose cost is unknown count as $0. bug-0320da's branch adds `AgentEfficiencyEvent::has_known_cost` (`efficiency.rs:279`), which this needs.

## Why it matters

One settled record per attempt (epic spec-b7303f): per-role averages are understated wherever costs went unmeasured. The same error bug-9bffe2 fixes for `--cfactor`. p3.

## Where

`compute_role_profiles`. The root fix belongs there, so every caller benefits.

## Plan

1. In `compute_role_profiles`, average only events with `has_known_cost()`, and report how many were unknown.
2. Show the unknown count in `learn role-costs`.
3. Add `role_profiles_skip_unknown_costs`.

## Done when

- [ ] Role profiles never average an unknown cost as $0.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-0320da's branch, which adds `has_known_cost`.
