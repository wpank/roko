+++
id = "gap-29a84b"
kind = "gap"
title = "A plan can succeed while some of its tasks never ran a verify step"
status = "done"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_execution", "roko-cli/plan_validate"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "c651ddc57"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e2"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (§6 decision 5; tldr/04 step 7)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/plan_validate.rs:452"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_with_unverified_task_does_not_succeed' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_with_unverified_task_does_not_succeed"

[[verify]]
command = "grep -rqw 'fn task_without_verify_is_rejected_in_strict_mode' crates/roko-cli/src/ && cargo test -p roko-cli --lib task_without_verify_is_rejected_in_strict_mode"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "A plan whose tasks ran no verify step reports unverified, not succeeded (d2c092ca5, merged 3383bd8c0). Batch 3 gate (work/rust-batch-3; crates tree identical to MAIN after the merges): cargo check --workspace --tests clean; nightly rustfmt clean after fmt-only commits db7f3f861 and 26947cd62; clippy -p roko-cli -p roko-learn -p roko-gateway -p roko-acp -p roko-serve -p roko-core --no-deps -D warnings clean; lib tests roko-cli 3072, roko-core 1919, roko-learn 1172, roko-serve 950, roko-acp 197, roko-gateway 41, 0 failed."
+++

## Problem

Two rules let a plan report success without evidence:

- **At run time:** a task whose outcome is `Unverified` (it had no verify step, or its checks never ran) still counts
  towards plan success. This was reported at `d9e79e9d8` (tldr/04 step 7; research note B4); confirm it first (see
  Current state).
- **At validation time:** `roko plan validate` accepts a task with no verify steps. The only related rule, `PLAN_011`,
  is a warning, and it fires only when `gate_rung = 0` (`plan_validate.rs:452`).

## Why it matters

tldr/05 decision 5 (default: yes) says that an unverified task fails the plan and that every task needs at least one
verify step. Without this rule, "the plan passed" can mean that nothing was checked. It is part of epic spec-e9d7ec, and
the whitepaper's claim about honest verdicts depends on it.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body`: where a plan's outcome is decided from
  its tasks.
- `crates/roko-cli/src/plan_validate.rs`: the `PLAN_0xx` lint rules, with `PLAN_011` at line 452.
- Plan status as shown by `roko plan status`, the TUI and the portal.

## Current state

The lint behaviour above was checked at `41c7ffbd6`. Re-check the run-time rule before changing it, using a fixture
plan whose only task has no verify step.

## Plan

1. **Plan outcome:** `succeeded` only when every task passed. If every task passed or was unverified, and at least one
   was unverified, the outcome is `unverified`. Otherwise it is `failed`.
2. **`plan validate`:** a task with no verify step is an error under `--strict` and a warning otherwise. Add a new
   `PLAN_0xx` code and keep `PLAN_011`.
3. **Display:** `roko plan status`, the TUI and the portal show `unverified` as a state of its own.

## Done when

- [ ] A plan whose only task has no verify step does not report `succeeded`.
- [ ] `roko plan validate --strict` rejects a task with no verify step.
- [ ] Tests `plan_with_unverified_task_does_not_succeed` and `task_without_verify_is_rejected_in_strict_mode` exist
      and pass: the two `[[verify]]` commands.

## Notes

- This item decides the outcome rule. The dashboard's counting is bug-7e1b6b, and the run-metrics counting is
  bug-7eb27e.
- Existing plans whose tasks lack verify steps will start failing `--strict`. List them in the commit message, for
  example from `roko plan validate --strict plans/`.
- Display, as implemented: `roko plan status` and `plan list` (so the portal's plan list too) show `unverified`.
  The live TUI and the portal run view still get `PlanCompleted { success: false }` and show the plan as failed; a
  state of its own there needs a dashboard event change (see bug-7e1b6b).
- Implemented on `work/bug-7eb27e` at `d2c092ca5`; cargo verification deferred to the batch check.
