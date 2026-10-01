+++
id = "bug-2d06bf"
kind = "bug"
title = "One broken plan in plans/ makes roko plan generate exit 1: prd::generate_plan validates every plan under the root"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e3df7d"
anchors = ["crates/roko-cli/src/prd.rs::generate_plan_from_prd"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-e3df7d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib generate_plan_validates_only"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:08Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:16Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

After generating a plan, `prd::generate_plan*` validates every plan under the plans root, not only the one it wrote. Now that generated plans share the workspace `plans/` directory (bug-e3df7d), any unrelated broken plan there makes `roko plan generate` fail.

## Plan

Validate only the generated plan's directory. Add a test named `generate_plan_validates_only_*`, with a broken sibling plan that no longer fails generation.

## Done when

- `cargo test -p roko-cli --lib generate_plan_validates_only` passes.

## Notes

- Reported on 2026-10-01 by the worker on bug-e3df7d, during the evening close-out round.
- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `prd::generate_plan` validates `plan_dir`, the plan the call wrote (or regenerated), instead of the whole plans root.
  The validation report and `artifact_valid` now describe that plan only, for `roko prd plan`, `roko plan generate`,
  serve and `roko do`. Test: `generate_plan_validates_only_the_plan_it_wrote`. It runs the generator with the fake
  planner beside a sibling plan whose tasks.toml does not parse, and checks the report covers one plan, the widget, with
  no `PLAN_001`. The fake-planner setup moved into test helpers that `test_prd_plan_does_not_regenerate_other_plans`
  shares.
