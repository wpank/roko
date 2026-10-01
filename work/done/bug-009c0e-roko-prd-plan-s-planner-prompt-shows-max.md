+++
id = "bug-009c0e"
kind = "bug"
title = "roko prd plan's planner prompt shows max_parallel = 1 in its required plan structure, so most plans run serially"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/prd"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-repin's report)"
anchors = ["crates/roko-cli/src/prd.rs", "crates/roko-cli/src/commands/plan.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-272448", "gap-a8d786"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/^async fn generate_plan_from_prd_with_outcome(/,/^}/p;/^pub async fn generate_plan(/,/^}/p' crates/roko-cli/src/prd.rs | grep -q 'max_parallel = 1'"

[[verify]]
command = "grep -qF '# Add [[task]] entries below' crates/roko-cli/src/commands/plan.rs && ! grep -F '# Add [[task]] entries below' crates/roko-cli/src/commands/plan.rs | grep -q max_parallel"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:19Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18; roko plan create's scaffold and the planner's retry prompt no longer write max_parallel = 1 (both verify parts pass). Merged d9ee47916 (work/gap-2623b2 44fd6ceb0) and earlier ce7156389."
+++

## Problem

132 plans under `plans/` set `max_parallel` explicitly, and 102 of them set it to 1. The source is `roko prd plan`'s planner prompt. In `generate_plan_from_prd_with_outcome` (`crates/roko-cli/src/prd.rs:1329`), the "MINIMUM REQUIRED STRUCTURE" example the model must follow includes `max_parallel = 1` in `[meta]` (:1830-1840). The newer `plan generate` path omits the field on purpose ("tasks that do not depend on each other run together", `plan_generate.rs:209`; gap-272448). `plan_generator.rs`'s `max_parallel = 1` lines are all in its tests.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): parallel execution of independent tasks is the thesis's claim. Plans generated through `roko prd plan` serialize everything. p3.

## Where

The prompt string in `generate_plan_from_prd_with_outcome`.

## Plan

1. Remove `max_parallel = 1` from the required-structure example (or show the field omitted, with the comment `plan_generate.rs` uses).
2. Optionally have `plan validate` warn when a plan sets `max_parallel = 1` while it has independent tasks (gap-a8d786 lints related cases).

## Done when

- [ ] `roko prd plan` no longer asks for `max_parallel = 1`.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/gap-2623b2` at `5e39cb9d5`, with gap-2623b2; cargo verification deferred to the batch check.
- gap-2623b2 moved the planner prompts out of `generate_plan_from_prd_with_outcome`, now a thin wrapper, into
  `generate_plan`, the one plan generator. The retry prompt's "MINIMUM REQUIRED STRUCTURE" example now shows the
  comment the shared system prompt uses (`# max_parallel is omitted: tasks that do not depend on each other run
  together`). The verify now reads `generate_plan` too, where the prompt lives; it read only the wrapper and would
  pass vacuously.
- `roko plan create`'s scaffold (`commands/plan.rs`) wrote `max_parallel = 1` into every hand-created plan's `[meta]`.
  It now omits the field (`849449ded`), and the second `[[verify]]` checks the scaffold. Test fixtures keep their
  `max_parallel = 1`.
- The 132 existing plans are unchanged. Plan step 2 (a `plan validate` warning) is not done.
