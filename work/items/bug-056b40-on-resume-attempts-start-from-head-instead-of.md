+++
id = "bug-056b40"
kind = "bug"
title = "On resume, attempts start from HEAD instead of the plan branch, and retained attempt worktrees aren't re-attached"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/orchestrator/worktree", "roko-cli/graph_execution"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-integrate's report, branch work/bug-50caf2 at f0445319f)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs", "crates/roko-cli/src/graph_execution/delivery.rs"]
lane = "rust-hot"
parent = "spec-a0e40a"
links = { depends_on = ["bug-50caf2"], blocks = [], related = ["bug-50caf2", "bug-8835bc", "gap-6daad9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumed_run_starts_attempts_from_the_plan_branch' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_resumed_run_starts_attempts_from_the_plan_branch"
+++

## Problem

On bug-50caf2's branch, attempts run in worktrees made by `create_for_attempt` (`crates/roko-cli/src/orchestrator/worktree/mod.rs:793`). wk-integrate reports two problems when a run resumes:

- until the first acceptance in the new process, attempts start from HEAD instead of from the plan branch, so they don't see the work already accepted in the earlier process;
- attempt worktrees retained from the earlier process aren't re-attached, because `create_for_attempt` refuses a branch that is already checked out in another worktree.

## Why it matters

Integration and a whole-plan check (epic spec-a0e40a): a resumed run redoes or contradicts accepted work, and leaves the old worktrees stranded.

## Where

`create_for_attempt`, and the resume path that sets the base for attempts (`graph_execution/`, including `delivery.rs`'s handling of checked-out targets).

## Plan

1. On resume, base new attempts on the plan branch.
2. Re-attach, or clean up and recreate, worktrees whose branch is already checked out, instead of refusing.
3. Add `a_resumed_run_starts_attempts_from_the_plan_branch`.

## Done when

- [ ] After a resume, attempts start from the plan branch, and retained worktrees are reused or recreated.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-50caf2's branch.
