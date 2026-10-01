+++
id = "bug-e35fdb"
kind = "bug"
title = "roko learn role-costs averages unknown costs as $0 through compute_role_profiles"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-learn/efficiency", "roko-cli/commands/learn"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-0320da at dd7cb7320)"
anchors = ["crates/roko-learn/src/efficiency.rs", "crates/roko-cli/src/commands/learn.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-0320da"], blocks = [], related = ["bug-9bffe2", "bug-9a6799", "bug-0320da"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn role_profiles_skip_unknown_costs' crates/roko-learn/src/ && cargo test -p roko-learn --lib role_profiles_skip_unknown_costs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:17Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including role_profiles_skip_unknown_costs, frequency_profiles_skip_unknown_costs and the cost_comparison integration test. Merged 5750081d8 (work/hb-l5 49eadc8ab)."
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
