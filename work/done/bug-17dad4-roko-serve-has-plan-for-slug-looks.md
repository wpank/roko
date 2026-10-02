+++
id = "bug-17dad4"
kind = "bug"
title = "roko-serve has_plan_for_slug looks only in .roko/plans, but generated plans now land in the workspace plans directory"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-e3df7d"
anchors = ["crates/roko-serve/src/routes/prds.rs::has_plan_for_slug"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-e3df7d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-serve --lib has_plan_for_slug"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:06Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:16Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`has_plan_for_slug` (`routes/prds.rs`) decides whether a PRD already has a plan by looking under `.roko/plans`. Since bug-e3df7d, `roko plan generate` and the job runner write plans to the workspace plans directory (`roko_fs::workspace_plans::workspace_plans_dir`), so the PRD routes report "no plan" for plans that exist.

## Plan

1. Look in the workspace plans directory (the same resolver `plan run` uses), and keep `.roko/plans` as a legacy fallback.
2. Add a test named `has_plan_for_slug_*` that finds a plan written under `plans/`.

## Done when

- `cargo test -p roko-serve --lib has_plan_for_slug` passes, and the test covers a plan under the workspace plans directory.

## Notes

- Reported on 2026-10-01 by the worker on bug-e3df7d, during the evening close-out round.
- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `has_plan_for_slug` now finds a plan directory `<slug>/` (with `tasks.toml` or `plan.md`) in the workspace plans
  directory or the legacy `.roko/plans/`, and still accepts a legacy `.roko/plans/<slug>.json` or `.toml` file. Before
  this, it accepted only those legacy files, so it missed directory plans even in `.roko/plans/`. Tests:
  `has_plan_for_slug_finds_a_plan_in_the_workspace_plans_dir` and `has_plan_for_slug_reads_legacy_plans`.
