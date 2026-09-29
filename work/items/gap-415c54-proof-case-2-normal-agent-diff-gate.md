+++
id = "gap-415c54"
kind = "gap"
title = "Proof Case 2: Normal agent diff + gate + merge"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:3924", "crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider", "crates/roko-graph/src/engine.rs::GraphEngine::with_merge_queue", "crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-graph/src/workspace.rs::WorkspaceReleasePolicy", "crates/roko-cli/src/graph_execution/plan_runner.rs:840"]
links = { depends_on = ["spec-f830c4"], blocks = [], related = ["spec-f830c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn worktree_task_diff_is_gated_merged_and_cleaned' crates/roko-cli/ && cargo test -p roko-cli worktree_task_diff_is_gated_merged_and_cleaned"
+++

## Problem

Proof case 2 from the dogfood audit has never been shown on the Graph engine. The case: an agent makes a real
diff in its task worktree and exits normally, then gate, commit/merge, summary and cleanup all finish with no
manual step.

Today the chain breaks at commit/merge. Run `roko plan run <dir> --worktree-per-task` on a plan whose task
edits a file:

1. The agent edits a file in `.roko/worktrees/<attempt>/`.
2. The authored verify steps run in that worktree and pass.
3. Nothing commits the edit or merges it anywhere.
4. The success path calls `release(WorkspaceReleasePolicy::Delete)`. `WorktreeManager::remove` refuses a dirty
   checkout, so the release fails with a warning and the worktree stays on disk.

The user's branch never receives the change, and cleanup does not finish. Without `--worktree-per-task` the
agent edits the user's tree in place: diff, gate and summary work (seen live in the 2026-09-25
portal-programme run), but there is no commit or merge step either.

Expected: the verified edit is committed and merged into the run's target (a plan or batch branch, then the
user's branch), the run summary reports it, and the attempt worktree and branch are removed. No operator
action.

## Why it matters

Goal `core`. This is the basic happy path of isolated plan execution. Until it works, worktree isolation
cannot be the default (`gap-4ec59f`), plans cannot run in parallel in isolated trees (`gap-d58ae8`), and every
roko-built change has to be committed by hand. Related: `spec-f830c4` (#404 batch branch integration),
`bug-a3760a` (`git_merge` runs `git checkout` in the user's tree), `gap-5d3b82` (proof case 1: agent early
exit), `gap-161be1` (proof case 3: baseline filtering).

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs:3924-3948`: "Worktree isolation: release on success". It
  releases the lease with `Delete` and has the comment "can be merged separately via the delivery pipeline.
  For now the worktree is cleaned up." No commit happens before this point.
- `crates/roko-cli/src/graph_task_dispatch.rs:3374`: the lease is acquired per attempt.
  `settle_task_verification` (:1807) runs the gate in the lease path.
- `crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider`: acquire →
  `WorktreeManager::create_for_attempt` (from `HEAD`); release(Delete) → `WorktreeManager::remove`, which
  refuses dirty checkouts (`orchestrator/worktree/mod.rs:1044`).
- `crates/roko-graph/src/workspace.rs::WorkspaceReleasePolicy` (:151). `Delete` is documented as "used for
  successful attempts after their commits have been merged". Today it is used before any merge.
- `crates/roko-graph/src/engine.rs`: `with_merge_queue` (:428) and the enqueue hook after a successful graph
  (:876-890, :1297). It sends a `MergeRequest { plan_id, branch_name: "", files_changed, priority }`.
  `roko-cli` never attaches a `MergeEnqueuer`.
- `crates/roko-graph/src/delivery.rs`: `CompletionDeliveryService` trait, `CompletionDeliveryRequest`
  (branch, commit OID, target branch, changed files), receipts and states.
- `crates/roko-cli/src/graph_execution/delivery.rs`: `GitDeliveryBackend` (:100; its `git_merge` checks out
  the target in the workdir, see `bug-a3760a`) and `CliCompletionDeliveryService` (:343). Both are constructed
  only in that file's tests.
- `crates/roko-cli/src/runner/merge.rs::PlanMerger` and `crates/roko-cli/src/orchestrator/merge_queue.rs::MergeQueue`:
  a serialized merger with `git merge-tree` conflict prediction. Used only in tests
  (`crates/roko-cli/tests/merge_proof.rs`, `runner_integration.rs`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the `--worktree-per-task` wiring (:1154-1171), and
  the guard that refuses parallel plans because "per-task worktrees are never merged back" (:840-846).

Entry point: `roko plan run <dir> --worktree-per-task` → `run_graph_plan` → `GraphTaskDispatcher`.

## Current state

- Checked at `a17d9d766`: diff → gate → summary works in place (portal run, 2026-09-25). Commit/merge is
  missing on the Graph path in both modes. The delivery and merge code exists but is only exercised by tests.
- No test runs a Graph plan with `worktree_per_task: true` end to end.
- Building blocks for a proof:
  - unit tests in `graph_task_dispatch.rs` (for example near :5994 and :6114) write a `fake-claude.sh` into a
    temp dir and point the provider at it;
  - `plans/portal-programme/_harness/fake-claude` is a deterministic Python stand-in for the `claude` CLI.
    Point `[providers.claude_cli] command` at it; a prompt line `ARTIFACT <path>` makes it write that file.

## Plan

1. Land the merge path first. This item is the proof, not the implementation. Merge-back of a successful
   attempt is designed in `spec-f830c4` (batch branch, `PlanMerger`, merge in a temporary worktree) and in
   step 4 of `gap-4ec59f`. The required behaviour:
   - on task success, `git add -A && git commit` in the attempt worktree, on the attempt branch
     (`format_attempt_branch_name`);
   - merge that commit serially into the run's integration branch without touching the user's checkout
     (`bug-a3760a`);
   - only then call `release(Delete)`;
   - dependent tasks branch from the integration branch.
2. Add an integration test, `worktree_task_diff_is_gated_merged_and_cleaned` in `crates/roko-cli/tests/` or
   in `graph_task_dispatch.rs` tests. Setup: a temp git repo, a one-task plan whose fake agent writes a file,
   and a `verify` step that checks the file (`test -f …`). Run it with worktree mode on and assert:
   - the task passes;
   - the file is committed on the integration or target branch (`git log -1 --name-only`);
   - `.roko/worktrees/` holds no checkout for the attempt, and `git worktree list` shows only the main tree;
   - the attempt branch is gone;
   - the run summary or checkpoint records the merge (commit OID).
3. Add a second case: a verify failure keeps the worktree (`RetainForFailure`) and merges nothing.
4. Do one live proof with the portal harness `fake-claude` in a scratch repo (never in the user's checkout):
   `roko plan run <scratch>/plans/<plan> --worktree-per-task`. Record the command, the resulting commit and
   `git worktree list` output in this item's closing evidence.

## Done when

- The integration test passes, and the scenario ends with the change committed and merged, the worktree and
  branch removed, and no warning "worktree release failed".
- A failing-verify variant retains its worktree and merges nothing.
- A live run in a scratch repo is recorded as closing evidence (command, commit sha, `git worktree list`).
- The suggested verify passes:
  `grep -rqw 'fn worktree_task_diff_is_gated_merged_and_cleaned' crates/roko-cli/ && cargo test -p roko-cli worktree_task_diff_is_gated_merged_and_cleaned`.
  The current `[[verify]]` (remove a comment, mention `CompletionDeliveryService` in `plan_runner.rs`) can pass
  without any merge working.

## Notes

- Depends on the merge-back implementation (`spec-f830c4`, overlapping `gap-4ec59f`). Do not implement merge
  here separately. Pick up whichever of those items lands the merge, then prove it here.
- Never let a merge `git checkout` in the user's working tree (`bug-a3760a`). Run all proofs in a scratch
  repository or temp dir.
- This is git-mutation code (commits, branches, worktree removal). The repository mutation lock
  (`WorktreeManager::acquire_repository_mutation_lock`) must cover commit and merge.
- Safe to run in parallel with items that do not touch `graph_task_dispatch.rs` or `plan_runner.rs`.

## Original notes

Agent makes a real worktree diff and exits normally; gate, commit/merge, summary, and cleanup all terminate without manual intervention.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.2 Proof Case 2: Normal agent diff + gate + merge`

How to verify: Source: Dogfood audit, proof case 2. Check the described code path for: Agent makes a real worktree diff and exits normally; gate, commit/merge, summary, and cleanup all terminate without manual intervention.

Verified 2026-09-28: The diff + gate + summary half ran live in the 2026-09-25 portal run (in place, no worktree). Commit/merge is not wired on the Graph path: with --worktree-per-task a successful attempt's lease is released with WorkspaceReleasePolicy::Delete (graph_task_dispatch.rs 'Worktree isolation: release on success': 'can be merged separately via the delivery pipeline. For now the worktree is cleaned up'), and graph_execution/delivery.rs::CliCompletionDeliveryService is constructed only in its own tests.
