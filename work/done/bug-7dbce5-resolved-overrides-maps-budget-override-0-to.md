+++
id = "bug-7dbce5"
kind = "bug"
title = "resolved_overrides maps --budget-override 0 to BudgetPolicy::Disabled, unlike the live path"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d31457"
anchors = ["crates/roko-cli/src/resolved_overrides.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d31457"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'BudgetPolicy::Disabled' crates/roko-cli/src/resolved_overrides.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:15Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:14:32Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After gap-d31457, `--budget-override 0` means "no plan ceiling", and the per-task and daily ceilings still hold. `resolved_overrides.rs` maps it to `BudgetPolicy::Disabled`. Nothing in production reads that field, so it is dead and misleading.

## Plan

Delete the field, or make it match the live path.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-childenv, working on gap-d31457, during the evening close-out round.
- 2026-10-01 (wk-childenv): implemented on work/gap-1555ac; cargo verification deferred to the batch check.
  Deleted the dead field rather than keep a second resolution of the budget flags: `BudgetPolicy`,
  `ResolvedExecutionOverrides::budget`, the `PlanRunInput` budget inputs and their resolution in `for_plan_run`,
  with their tests here and in main.rs. `plan_runner::resolve_budget_ceiling` stays the only resolution; a caller
  that wires `for_plan_run` into `plan run` later can carry its result.
