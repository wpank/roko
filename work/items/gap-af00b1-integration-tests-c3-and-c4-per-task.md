+++
id = "gap-af00b1"
kind = "gap"
title = "Integration tests C3 and C4: per-task commits on a plan branch, and a whole-plan gate that catches tasks that break together"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e6"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (canaries C3 and C4)"
anchors = ["crates/roko-cli/tests/plan_branch_integration.rs"]
lane = "rust-cold"
parent = "spec-a0e40a"
links = { depends_on = ["gap-3b5361", "bug-a3760a", "spec-f830c4", "gap-60233f", "bug-50caf2", "gap-4ec59f"], blocks = [], related = ["gap-cd3529", "gap-3aa9cb", "find-70edcb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn c3_each_passed_task_commits_once_on_the_plan_branch' crates/roko-cli/tests/ && grep -rqw 'fn c4_meta_verify_catches_tasks_that_break_together' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_branch_integration"
+++

## Problem

Each fix in epic spec-a0e40a gets its own unit test. Nothing runs a real plan end to end and checks that:

- every passed task left exactly one commit on the plan branch;
- your checkout was never touched;
- tasks that pass alone but break together are caught before the plan reports success.

## Why it matters

These are canaries C3 and C4 of assessment W8 (gates G3 and G4), the exit check for epic spec-a0e40a, and part of
the golden-path suite (gap-3aa9cb). Without them, a later change could strand work or touch the operator's checkout
unnoticed.

## Where

- **New file:** `crates/roko-cli/tests/plan_branch_integration.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`. It configures a scripted fake provider
  through `roko.toml` and runs the built binary with `assert_cmd`. `tests/common/mod.rs` has `seed_git_repo` and
  `write_executable`.

## Current state

Checked at `41c7ffbd6`: no test covers merging or a plan-level verify on the Graph path. `tests/merge_proof.rs` and
`tests/runner_integration.rs` exercise the old `PlanMerger`, not a Graph run.

## Plan

1. **C3, `c3_each_passed_task_commits_once_on_the_plan_branch`:** a seeded git repo, a three-task plan, and a fake
   provider whose agents edit only their task's `files`. Assert:
   - `roko/plan/<plan>` has exactly one commit per passed task, each touching only that task's `files`;
   - the checkpoint records each task's `accepted_commit`;
   - the operator checkout's HEAD, index and `git status --porcelain` are identical before and after;
   - after killing the run mid-task and resuming with `--resume-plan`, no edit is left unattributed.
2. **C4, `c4_meta_verify_catches_tasks_that_break_together`:** a two-crate workspace.
   - Task A changes a public signature in crate `a`; its verify runs crate `a`'s tests.
   - Task B's scripted edit in crate `b` still uses the old signature; its verify is scoped to its own file (the R-1
     pattern in find-70edcb).
   - Each task passes its own verify, but `[meta] verify` fails because crate `b` no longer builds.
   - Assert the plan is not `succeeded`, and `roko plan status` names the failed `[meta] verify` step.

## Done when

- [ ] Both tests exist and pass.
- [ ] Reverting gap-3b5361's commit step or gap-60233f's gate makes C3 or C4 fail. Check this once by hand and say
      so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- W8's C4 also covers two lints (red-on-base, overlapping concurrent `files`). gap-b3fa0a and gap-a8d786 test those.
- Fake provider only, no network, under a minute per test. No hot file: write it while the fixes land, merge it
  last, and use gap-3aa9cb's shared provider if it has landed.
