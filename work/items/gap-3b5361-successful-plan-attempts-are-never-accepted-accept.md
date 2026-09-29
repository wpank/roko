+++
id = "gap-3b5361"
kind = "gap"
title = "Successful plan attempts are never accepted: accept_attempt has no production caller"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/worktree"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/orchestrator/worktree/mod.rs::accept_attempt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "\.accept_attempt(" crates --include="*.rs" | grep -v "orchestrator/worktree/tests.rs" | grep -q .'
+++

`accept_attempt` (`orchestrator/worktree/mod.rs:885`) is the only function that records an accepted commit for an attempt; its only caller is `orchestrator/worktree/tests.rs:1799`.
Successful attempts are released without an accepted-commit record, so nothing durably links a passed task to the commit that passed its gates.
Fix: on gate pass call `accept_attempt`, release the worktree retained for review, and record the accepted commit in the checkpoint / attempt log.

## Notes

- 2026-09-29 (wk-integrate): Implemented on `work/bug-50caf2` at `671df37dc` (on top of bug-50caf2's
  `72d3c9823`); cargo verification deferred to the batch check. In the worktree: `cargo check -p roko-cli -p
  roko-graph --lib --tests` and `cargo clippy -p roko-cli -p roko-graph -p roko-core --no-deps -D warnings`
  clean; targeted `cargo test -p roko-cli --lib` (attempt_workspace, orchestrator::worktree,
  graph_execution::workspaces, gate_adapter, graph_task_dispatch: 204 passed) and `cargo test -p roko-graph --lib`
  (471 passed).
  - Acceptance follows the settled verdict: the verdicts a completed task replays (`passed`, `unverified`) are
    accepted, `forced_accept` never is (the Graph dispatcher does not produce it today). Unverified work is
    accepted so that a completed task's work is on the plan branch; its commit says `Roko-Verdict: unverified`.
  - `WorktreeManager::accept_attempt(plan, task, attempt, &AttemptAcceptance)` (now under the operation and
    repository locks, in `orchestrator/worktree/acceptance.rs`): commits the checkout on its attempt branch
    (`add --all`, `write-tree`, `commit-tree`, compare-and-swap `update-ref`; roko's `.cursor` copy is left out;
    identity `roko`; trailers `Roko-Plan/Task/Attempt/Run/Verdict`), then folds that commit into
    `roko/plan/<plan_id>`: a fast-forward, else `merge-tree --write-tree` plus `commit-tree`, moved only by
    compare-and-swap. A conflict (`WorktreeError::Conflict`) leaves the plan branch alone. A plan branch checked
    out anywhere is never moved. The first acceptance in a process continues the branch only when its tip's
    `Roko-Run` trailer names the same run (resume), or when the attempt already builds on it; otherwise the old
    tip is kept at `refs/roko/plan-archive/<plan_id>/<tip>` and the branch restarts. The new tip is the base of
    the plan's later attempts (`create_for_attempt`).
  - `ExecutionWorkspaceProvider::accept` (with `WorkspaceAcceptRequest`, `WorkspaceAcceptance`,
    `WorkspaceError::Conflict`); `WorktreeExecutionWorkspaceProvider` calls `accept_attempt`, which is its first
    production caller. `GraphTaskDispatcher` accepts after the settled verdict, releases `RetainForReview`, and
    stamps `workspace.attempt_commit`, `workspace.plan_branch` and `workspace.accepted_commit` on the output, so
    the accepted commit is recorded in the Graph checkpoint (and the plan branch's reflog). A conflict fails the
    attempt as a retryable `Verify { gate: "plan-branch" }` and moves the task to a fresh checkout of the plan
    branch (checkout generation); any other refusal is `Rejected`, which `TaskExecutorCell` no longer retries.
  - Rich topology: the executor hands the worktree on, and `PlanGateCell` accepts it through the new
    `CellResources.workspaces` once the gate passes (kept `RetainForFailure` when it fails). A handed-on
    worktree with no workspace provider injected is an error before any rung runs.
  - Tests: `accept_folds_sibling_attempts_into_the_plan_branch`,
    `accept_refuses_a_conflicting_sibling_and_moves_no_ref`, `accept_leaves_a_checked_out_plan_branch_alone`,
    `accept_starts_afresh_from_another_runs_plan_branch`, `accept_leaves_roko_config_copies_out_of_the_commit`,
    `passed_attempt_is_accepted_onto_its_plan_branch`, `conflicting_attempt_fails_and_the_retry_starts_over`,
    `plan_gate_accepts_a_handed_on_worktree_that_passes`, `plan_gate_keeps_a_failed_worktree_without_accepting_it`.
  - Left for later: the plan branch is not delivered anywhere (spec-f830c4 split (b)); a resumed run's first
    attempts still start from `HEAD`, not from the plan branch, until its first acceptance in the new process
    (the text merge then folds them in); retained attempt worktrees of an earlier process are not re-attached
    (`acquire` uses `create_for_attempt`, which refuses a branch that is checked out); a merged sibling's tree
    is not re-verified before it lands on the plan branch (gap-60233f's whole-plan gate is the check for that).
