+++
id = "spec-f830c4"
kind = "spec"
title = "#404 — Batch Branch Integration"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-30
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration"
discovered_from = "audit:tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_execution/delivery.rs::GitDeliveryBackend", "crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-cli/src/runner/merge.rs::PlanMerger", "crates/roko-cli/src/orchestrator/merge_queue.rs::MergeQueue", "crates/roko-cli/src/orchestrator/worktree/mod.rs::WorktreeManager::accept_attempt", "crates/roko-graph/src/delivery.rs::CompletionDeliveryRequest"]
links = { depends_on = [], blocks = [], related = ["gap-415c54"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'roko/batch/' crates/roko-cli/src && grep -qE 'CliCompletionDeliveryService::(new|with_store)' crates/roko-cli/src/graph_execution/plan_runner.rs crates/roko-cli/src/commands/plan.rs && grep -rqw 'fn batch_branch_merges_plan_in_temp_worktree' crates/roko-cli/src && cargo test -p roko-cli batch_branch_merges_plan_in_temp_worktree"
+++

## Problem

`roko plan run` never integrates task results through git. After every task passes its gates, nothing is
committed, merged, promoted or tagged:

- Default mode (no flag): tasks edit the shared working tree in place and leave uncommitted changes for a human.
- `--worktree-per-task`: each attempt runs in `.roko/worktrees/attempt-<20 hex>` on branch
  `roko/attempt/attempt-<20 hex>`. The agent's changes are never committed. On success the lease is released with
  `WorkspaceReleasePolicy::Delete`; the manager refuses to delete a dirty worktree, so release fails with the
  warning "worktree release failed (best-effort); worktree may remain on disk" and the work is stranded. Nothing is
  merged back. Attempts start from `base_branch = "HEAD"`, so a dependent task does not see its predecessor's work.
- There is no `roko/batch/<run-id>` branch, no promotion to a target branch, no `roko/run/<run-id>` tag and no
  `roko.batch@1` checkpoint extension.

Expected (original spec #404): each run gets a batch integration branch. Each finished plan is merged into it in a
temporary worktree through the merge queue and checked by a post-merge regression gate. At the end the batch is
promoted to the target branch and tagged. All of this resumes safely after a crash.

## Why it matters

- Goal `features`. It is also the missing half of the self-hosting loop (goal `core`): without integration, "roko
  develops itself" ends in a dirty tree or stranded worktrees.
- Without a batch branch, two plans that each pass alone but break each other are not caught before the target
  branch changes, and concurrent completions race on `HEAD`. Resume cannot tell which plans were integrated.
- Related: `gap-415c54` (Proof Case 2: diff + gate + merge) is the end-to-end proof of this wiring and waits on it.
  `bug-a3760a` (`GitDeliveryBackend::git_merge` runs `git checkout` in the user's working tree) is a defect in the
  backend this item wires in: fix it here or first. `gap-4ec59f` step 4 (merge-back before worktree isolation
  becomes the default) needs the same commit-and-merge path. `gap-d58ae8` assumes a plan integration branch.

## Where

- `crates/roko-cli/src/commands/plan.rs::cmd_plan_run_engine` (~line 2518): `roko plan run` entry; parses
  `--worktree-per-task`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1154): builds the task dispatcher and, only with
  `worktree_per_task`, a `WorktreeManager` (`base_branch: "HEAD"`, root `.roko/worktrees`) wrapped in
  `WorktreeExecutionWorkspaceProvider`. The batch branch and delivery service belong here.
- `crates/roko-cli/src/graph_task_dispatch.rs`: "Worktree isolation: acquire" (~line 3360) and "release on success"
  (~line 3925, comment "For now the worktree is cleaned up"): where a successful attempt should be committed.
- `crates/roko-cli/src/graph_execution/workspaces.rs::release`: `Delete` calls `WorktreeManager::remove`, which
  returns `DirtyWorktree` for uncommitted changes.
- `crates/roko-cli/src/orchestrator/worktree/mod.rs`: `create_for_attempt` (bases an attempt on the plan's last
  accepted tip if set), `accept_attempt` / `accepted_for_plan` (record a plan's accepted commit; no caller on the
  Graph path), `format_branch_name` (`roko/plan/<plan_id>`).
- `crates/roko-cli/src/graph_execution/delivery.rs`: `CliCompletionDeliveryService` (states `Prepared -> Queued ->
  Merged -> RegressionPassed -> [Published] -> Delivered`, writes `roko.delivery@1`) and `GitDeliveryBackend`
  (`merge` enqueues into `MergeQueue` and ignores the result, then `git_merge` runs `git checkout <target>` and a
  merge in `workdir`; `run_regression` runs `cargo check --workspace --quiet`). Constructed only in its own tests.
- `crates/roko-cli/src/runner/merge.rs`: `PlanMerger`, `PlanMergerConfig`, `GitMergeBackend` (merge-tree conflict
  prediction, fast-forward preference), `CargoCheckRegressionGate`, `MergeResolution::fail`
  (`#[allow(dead_code)] // production caller not yet connected`). `orchestrator/merge_queue.rs::MergeQueue`:
  serialization by file overlap. Both are used only in tests (`tests/merge_proof.rs`, `tests/runner_integration.rs`).
- `crates/roko-graph/src/delivery.rs::CompletionDeliveryRequest`: `delivery_id`, `run_id`, `plan_id`, `lease_id`,
  `branch`, `commit_oid`, `target_branch`, `changed_files`, `publish`; no `batch_branch`.
- `crates/roko-cli/src/graph_checkpoint.rs`: checkpoint extension names (`roko.delivery@1` and others); add
  `roko.batch@1` here. Target branch: nearest existing setting is `[project] fresh_base_branch` (`"main"` in
  `roko.toml`).

## Current state

- Every building block exists and is unit-tested; none is instantiated by a real run
  (`grep -rn 'CliCompletionDeliveryService::new\|PlanMerger::new\|MergeQueue::new' crates/roko-cli/src` finds
  tests only). No `git commit` happens anywhere on the Graph path.
- The Runner-v2 loop that used some of these pieces was deleted on 2026-09-06 (`6b5da8616`). The merge/delivery
  files last changed in `244f564e1`.
- The original spec assumed per-plan branches from wave execution (#396) and worktree isolation (#400). At HEAD,
  isolation is per task attempt and opt-in, and branches are `roko/attempt/...`.

## Plan

Design choice: what is integrated, and when.

- Option A (recommended): integrate only in `--worktree-per-task` mode, per plan. Commit each successful attempt
  and fold it into the plan branch; when the plan's last task succeeds, merge the plan branch into the batch
  branch. In-place mode keeps today's behaviour. Matches the existing `accepted` tip design and the spec.
- Option B: merge every successful attempt straight into the batch branch and base new attempts on the batch tip.
  Simpler cross-plan semantics, but many more merges and regression runs.

Steps (Option A):

1. Commit on success (`graph_task_dispatch.rs`, "release on success"): `git add -A` and `git commit` in the lease
   path (message `roko: <plan_id>/<task_id>`). Merge that commit into `roko/plan/<plan_id>` serially, then record
   it with `accept_attempt`. `accept_attempt` only moves the tip, so parallel sibling tasks would overwrite each
   other without the merge. Release with `RetainForReview` until the plan is delivered.
2. Add `BatchBranch` (in `runner/merge.rs` or a new `graph_execution/batch.rs`): `create_or_reset(run_id)` creates
   `refs/heads/roko/batch/<run-id>` at `HEAD`; `merge_plan_tip(oid)` runs
   `git worktree add --detach .roko/batch-merge-tmp <batch>`, merges there, advances the branch with
   `git update-ref`, and removes the temporary worktree.
3. Route merges through `PlanMerger` (queue + `GitMergeBackend` + `CargoCheckRegressionGate`) with
   `PlanMergerConfig::new(<temp worktree>, timeout)`. Make `GitDeliveryBackend::git_merge` work in the temporary
   worktree, and add `batch_branch: Option<String>` to `CompletionDeliveryRequest` (fall back to `target_branch`
   when `None`, so existing tests still pass).
4. In `plan_runner.rs`, build `CliCompletionDeliveryService` when `worktree_per_task` is on, and submit one request
   per plan once all its tasks succeed. Persist receipts under `roko.delivery@1`, and `run_id`, `branch_name`,
   `base_oid`, `promotion_commit`, `promotion_tag` under `roko.batch@1`.
5. Staleness: if a plan's base is behind the batch tip, cherry-pick its commits onto the tip before merging.
6. Promotion, opt-in (for example `--promote`; default off): after every plan is `Delivered`, fast-forward or merge
   the batch into the target branch in a temporary worktree, create the annotated tag `roko/run/<run-id>`, and
   record both.
7. Resume: skip `Delivered` plans, continue `Merged`/`RegressionPassed` plans from the next step, redo
   `Prepared`/`Queued`. Exit non-zero and list the plans that end in `RegressionFailed`, `Conflict` or
   `TerminalFailed`. Remove the `dead_code` allow on `MergeResolution::fail`.

## Done when

- `roko plan run --worktree-per-task <dir>` on a two-plan fixture creates `roko/batch/<run-id>` and merges both plans
  into it without changing the primary checkout's branch or files. It records `roko.batch@1` in
  `.roko/state/graph/<plan>/checkpoint.json`.
- A regression failure after a merge ends that delivery in `RegressionFailed`, keeps the plan branch, and makes the
  command exit non-zero naming the plan. With promotion on, the target advances and tag `roko/run/<run-id>` exists.
- Temp-repo unit tests cover `create_or_reset`, merging in a temporary worktree and the cherry-pick refresh. A
  resume test starts from a `Merged` receipt and does not merge again.
- Verify (proposed; the current `[[verify]]` only greps for the branch-name string):

  ```
  grep -rq 'roko/batch/' crates/roko-cli/src && grep -qE 'CliCompletionDeliveryService::(new|with_store)' crates/roko-cli/src/graph_execution/plan_runner.rs crates/roko-cli/src/commands/plan.rs && grep -rqw 'fn batch_branch_merges_plan_in_temp_worktree' crates/roko-cli/src && cargo test -p roko-cli batch_branch_merges_plan_in_temp_worktree
  ```

## Notes

- Risky area: the history of the user's repository. Never run `git checkout`, `reset` or `merge` in the primary
  checkout. Merge only in temporary worktrees and move refs with `git update-ref`. Promotion must be opt-in and
  must never push.
- Keep plan and attempt branches after merge (the maintainer keeps them for inspection). Do not add an automatic
  `git branch -D`.
- `gap-415c54`, `gap-4ec59f` (step 4) and `bug-a3760a` touch the same code: the "release on success" block and
  `GitDeliveryBackend::git_merge`. Make one change and do not run them in parallel. The same applies to other work
  in `graph_execution/plan_runner.rs` dispatcher setup.
- `CargoCheckRegressionGate` runs `cargo check --workspace` over ~1M LOC. It is slow, so set the timeout in
  `PlanMergerConfig` with care. Parallel runs contend for the cargo build lock.
- 2026-09-29 (wk-filer2): bug-a3760a (`809ae920d` on `work/bug-a3760a`, in Rust batch 2) replaced the merge.
  `GitDeliveryBackend::git_merge` now merges with git plumbing only: `merge-base` for the fast-forward and
  already-merged cases, otherwise `git merge-tree --write-tree` plus `commit-tree`, then `update-ref` with the
  expected old value. It never checks out, merges or commits in `workdir`. A target branch that is checked out
  anywhere is left alone, and the result is parked at `refs/roko/delivered/<plan_id>`. The regression runs in a
  temporary detached worktree of the merge commit. `delivery.rs` no longer uses `MergeQueue`: the service's merge
  slot and the compare-and-swap serialize merges. So the `Where` bullet on `GitDeliveryBackend` is stale, and Plan
  step 3 is moot. Merging into `roko/batch/<run-id>` needs only that branch as the request's target, since it is
  never checked out. It does not need `PlanMerger`, whose `GitMergeBackend` still merges in and auto-commits its
  workdir (bug-207f35). Before step 4 wires delivery in, fix bug-453481 (delivery merges the branch head, not
  `commit_oid`) and bug-aaa924 (the regression builds from a cold target dir).
- 2026-09-30 (wk-integrate): Implemented on `work/spec-f830c4` at `8268c7498`; cargo verification deferred to the batch check.
  - Under `--worktree-per-task` the run opens `roko/batch/<run-id>` at `HEAD` by compare-and-swap; a resumed run continues the branch its checkpoint records (`roko.batch@1`). Every plan's attempts start from the batch, and each plan whose tasks all passed is delivered into it by `CliCompletionDeliveryService` with `GitDeliveryBackend`: a plumbing merge of the plan branch's verified tip, then the regression check in a separate checkout. Deliveries run one at a time; a failed regression takes the merge back out of the batch (`7f7a58f0e`). The receipt is kept in the plan's checkpoint (`roko.delivery@1`), and a resumed delivery continues from it.
  - `--promote <branch>` (only with `--worktree-per-task`) merges the batch into that branch once every plan succeeded, parks the result at `refs/roko/delivered/run-<id>` when the branch is checked out, and tags `roko/run/<id>`. It never pushes. Plan and attempt branches are kept.
  - In the worktree: `cargo check -p roko-cli -p roko-graph --lib --tests --bins` clean; nightly rustfmt clean. Tests: `batch_branch_merges_plan_in_temp_worktree`, `batch_branch_is_created_at_head_and_continued_on_resume`, `promotion_moves_the_target_and_tags_the_run`, `a_resumed_delivery_continues_after_its_merge`, `a_worktree_run_delivers_each_plan_into_its_batch_branch`; canaries C3 and C4 (gap-af00b1) run it through the binary.
  - Left: after a resume from a `Merged` receipt, a failed regression cannot take the merge back out; the promotion result is logged but not recorded in a checkpoint; the `--worktree-per-task` refusal message for parallel plans is stale.
- 2026-09-30 (wk-integrate): Done-when coverage on `work/spec-f830c4`:
  - Regression failure after a merge: canary C4 (`c4_meta_verify_catches_tasks_that_break_together`, gap-af00b1). Its tasks both pass, the delivery's regression fails after the merge, and the receipt ends `regression_failed`. The plan branch is kept, and the batch stays at its base. `roko plan run` exits 1 and names the plan: the text summary now lists plans that did not succeed (`Plans that did not succeed: c4`), and `--json` has `plan_outcomes`. `roko plan status` names the plan and the failed step.
  - Resume from a `Merged` receipt: `a_resumed_delivery_continues_after_its_merge` makes no merge and one regression run, then ends `Delivered`.
  - Temp-repo tests: `batch_branch_is_created_at_head_and_continued_on_resume` covers the plan's `create_or_reset`. `BatchIntegration::open` creates the branch at `HEAD` by compare-and-swap and continues an existing one instead of resetting it, since a reset would drop plans already delivered. `batch_branch_merges_plan_in_temp_worktree` covers the merge: plumbing only, with the regression run in a separate checkout.
  - No cherry-pick refresh: attempts start from the batch tip, and delivery merges with `merge-tree`, so a plan branch based on an older batch tip is merged as it is. `batch_branch_merges_plan_in_temp_worktree` covers that case: plan-a, based on the old tip, is merged after plan-b moved the batch.
  - The `[[verify]]` above already runs the proposed command (`… && cargo test -p roko-cli batch_branch_merges_plan_in_temp_worktree`).

## Original notes

Roko's graph engine (`cmd_plan_run_engine`) executes plan tasks and runs per-task gate pipelines, but after all tasks pass it does nothing with the branches. The merge queue (`MergeQueue`), merge wrapper (`PlanMerger`), delivery state machine (`CliCompletionDeliveryService`), and git backends…

Imported without verification from:
- `tmp/backlog/archive/404-batch-branch-integration.md##404 — Batch Branch Integration`

How to verify: Check: At `roko plan run` start, a `roko/batch/{run-id}` branch is created (or reset to HEAD); When a plan's tasks complete and all gates pass, its branch is merged into the batch; The `PlanMerger` (with `MergeQueue` serialization, `git merge-tree`… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: No `roko/batch/` branch is created anywhere. graph_execution/delivery.rs::CliCompletionDeliveryService and its MergeQueue-backed backend are constructed only in delivery.rs tests. On success an isolated attempt's worktree is released with WorkspaceReleasePolicy::Delete (documented as deleting checkout and branch) in graph_task_dispatch.rs, so worktree-mode results are never integrated.
