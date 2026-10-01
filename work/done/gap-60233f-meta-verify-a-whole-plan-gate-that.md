+++
id = "gap-60233f"
kind = "gap"
title = "[meta] verify: a whole-plan gate that runs on the integrated result"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-cli/task_parser"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "207f91da2"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e6"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (G4, canary C4); evidence/field/CASES.md (CASE-006)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskMeta", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::plan_outcome", "crates/roko-cli/src/graph_execution/plan_verify.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["spec-f830c4"], blocks = [], related = ["spec-1ced1d", "find-70edcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn meta_verify_failure_fails_a_plan_whose_tasks_passed' crates/roko-cli/src/ && cargo test -p roko-cli --lib meta_verify_failure_fails_a_plan_whose_tasks_passed"

[[verify]]
command = "grep -rqw 'fn default_meta_verify_covers_the_touched_crates' crates/roko-cli/src/ && cargo test -p roko-cli --lib default_meta_verify_covers_the_touched_crates"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 207f91da2. [meta] verify runs on the integrated result; without it a Cargo plan runs fmt, clippy and tests of the crates it touched; in worktree mode it is the delivery's post-merge regression and a failure undoes the merge into the batch; the failure is recorded and shown by plan status. Batch 16d gate on dd58c3db2 (MAIN 207f91da2 has the same code), after the coordinator's scope fix for plan_verify (cfed1c6f2): cargo check --workspace --tests, nightly fmt, clippy -p roko-cli -p roko-core -p roko-execution --keep-going -D warnings clean; lib tests pass: roko-cli 3236 (two known load flakes, turn_policy's 1 s test and gate_rows' writer wait), roko-core 1953, roko-execution 245; integration: --test plan_branch_integration 2 passed (C3 kill-and-resume, C4 whole-plan gate), --test merge_proof 4, --test runner_integration 6. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

Verify steps are per task. A plan whose tasks all passed reports `succeeded` even when the tasks break each other or
the product. `TaskMeta` has no `verify` field, and `run_one_plan` runs no check on the combined tree. Format and lint
run only if an author writes a final task for them.

## Why it matters

On 09-28 every portal plan was green, yet a browser pass found about 20 defects (CASE-006), and 59 unformatted files
reached the hand merges (research note B2). tldr/04 design rule 6: merge, then verify the merged tree. Gate G4 in
assessment W8, tested by canary C4 (gap-af00b1). Part of epic spec-a0e40a.

## Where

- `crates/roko-cli/src/task_parser.rs::TaskMeta`: add `verify: Vec<VerifyStep>`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan` and `plan_outcome`: run the gate after the last
  task settles, before the outcome is decided.
- **New file:** `crates/roko-cli/src/graph_execution/plan_verify.rs`: the default steps and the runner.
- Reuse `plan_set.rs::PlanFootprint::of` (`affects`: written packages plus reverse dependencies) and
  `runner/gate_dispatch.rs::spawn_plan_verify` (Runner-v2's plan-level verify, test callers only).

## Current state

Checked at `41c7ffbd6`: `TaskMeta` has no verify field, and nothing runs after the last task.
`DeliveryBackend::run_regression` runs a hard-coded `cargo check --workspace`, and nothing constructs that backend.
No existing item covers this (assessment W8: "not filed").

## Plan

1. Parse `[meta] verify`, with the same `VerifyStep` shape as tasks. Add `verify` to the known-field lists in
   `prd.rs` and `plan_generator.rs` and to `TasksFile::validate_against_schema`, so `--strict` (PLAN_035) accepts it.
2. Default when absent, for a Cargo workspace: `cargo fmt --all --check`, then clippy with `-D warnings` and
   `cargo test`, each over the crates in `PlanFootprint.affects`. Other projects get no default, and
   `plan validate` warns.
3. UI plans: accept a `phase = "browser"` step (CASE-006). The author supplies the browser driver.
4. Run the gate once every task has passed, on the integrated tree. After spec-f830c4 that is the plan-branch tip,
   in a detached worktree, never your checkout. A failure makes the plan `failed`, records the output in the
   checkpoint, and shows in `roko plan status`.
5. Use the same steps as `GitDeliveryBackend::run_regression`'s check, replacing its hard-coded cargo check.

## Done when

- [ ] A plan whose tasks all pass but whose `[meta] verify` fails does not report `succeeded`.
- [ ] Without `[meta] verify`, a Rust plan runs fmt, clippy and the affected crates' tests.
- [ ] `plan validate --strict` accepts `[meta] verify`.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Hot file: `plan_runner.rs`. Waits for spec-f830c4 (the integrated tree) and the portal session's branches.
- Keep the default cheap: affected crates only, never `cargo test --workspace`. spec-1ced1d's impact scoping could
  refine that set later.
- 2026-09-30 (wk-integrate): Implemented on `work/spec-f830c4` at `7f7a58f0e`; cargo verification deferred to the batch check.
  - `[meta] verify` takes task-style verify steps (`TaskMeta.verify`). `prd.rs` and `plan_generator.rs` list it as a known meta field, and `validate_against_schema` rejects a step without a command.
  - Without it, a plan in a Cargo workspace gets `plan_verify::default_plan_verify` over `PlanFootprint.affects`: `cargo fmt -p … -- --check`, `cargo clippy -p … --no-deps -- -D warnings` and `cargo test -p …`. When the package graph is unknown: fmt and clippy over the workspace, and no tests. Other projects get no default.
  - Under `--worktree-per-task` the steps are the delivery's regression check on the plan's merge into the batch branch; otherwise `check_plan_in_place` runs them in the working tree once every task passed. A failure fails the plan, is recorded in the checkpoint (`roko.plan.verify@1`, or the batch receipt), and `roko plan status` shows it (`plan check:`; JSON `plan_check_failure`).
  - Tests: `meta_verify_failure_fails_a_plan_whose_tasks_passed` (both modes), `default_meta_verify_covers_the_touched_crates`, `plan_verify_stops_at_the_first_failed_step`; canary C4 (gap-af00b1).
  - Not done: `plan validate` does not warn about a non-Rust plan without `[meta] verify` (`plan_validate.rs` belongs to another worker).
