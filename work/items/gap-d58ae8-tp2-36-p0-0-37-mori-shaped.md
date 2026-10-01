+++
id = "gap-d58ae8"
kind = "gap"
title = "Mori-shaped workflow contract unproven end-to-end"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "core"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
anchors = ["crates/roko-cli/src/runner/queue_manifest.rs::QueueManifest::eligible_plans", "crates/roko-cli/src/graph_execution/plan_set.rs::outside_plan_status", "crates/roko-cli/src/graph_execution/plan_set.rs::PlanSetScheduler", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/delivery.rs::CliCompletionDeliveryService", "crates/roko-cli/src/commands/plan.rs::read_executor_state"]
links = { depends_on = ["gap-4ec59f", "bug-a3760a", "bug-8208a6"], blocks = [], related = ["gap-4ec59f", "gap-0001a1", "gap-7c9e48", "gap-c135ba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/tests/mori_workflow_parity.rs && cargo test -p roko-cli --test mori_workflow_parity -- --include-ignored"
+++

## Problem

There is no test or recorded run showing that roko does the Mori workflow on the Graph engine (the only
plan executor since Runner-v2 was deleted on 2026-09-06). The Mori contract, from the 2026-09-01 parity
audit: **queue -> eligible DAG work -> isolated parallel attempts -> gates -> serialized merge -> durable
completion -> next queue milestone**, in one command, resumable after interruption. Concretely:

1. `.roko/queue.toml` names ordered milestones. With no plan argument, `plan run` picks the current
   incomplete milestone (`--milestone` picks one); its run overrides (model, max agents) reach dispatch.
2. Dependency-ready plans are scheduled automatically; independent plans run concurrently (bounded).
3. Every attempt runs in its own worktree/branch; nothing writes into the operator's checkout.
4. Finished branches merge one at a time; a conflict produces structured evidence and leaves the
   integration branch recoverable; a regression gate runs after merge.
5. A kill and restart does not repeat committed work; the next invocation advances the queue.
6. The operator can pause after N plans, inspect, and resume; the pause survives restart.

Today `roko plan run <dir>` runs the plans in `<dir>` in the operator's working tree. It never reads
`.roko/queue.toml`, never merges anything, and ignores `--batch-size`.

## Why it matters

Goal `core` (plan runs work reliably). This is the operator experience roko is meant to replace Mori with:
hand it a roadmap and let it work through it safely. Each piece is tracked separately, but the audit's
point is that they only count when proven together; source that compiles is not a working workflow.
Related: `gap-4ec59f` (worktree isolation default, p0), `bug-a3760a` (merge step checks out branches in
the user's tree), `bug-8208a6` (pause/resume/cancel write a `control.json` Graph never reads),
`gap-d60281` (`--batch-size` and other flags ignored), `gap-0001a1` (parallel plan queues), `gap-7c9e48`
(wave dispatch loop), `gap-6ca8fb` (parked: crash/resume kill-point harness), `gap-c135ba` (parked:
remaining Mori plan-format gaps). Absorbed `gap-28ee5a` (superseded).

## Where

- `crates/roko-cli/src/runner/queue_manifest.rs::QueueManifest`: parses `.roko/queue.toml` (`run:
  RunOverrides { max_agents, mode, express, model }`, `milestones: Vec<Milestone { name, plans,
  depends_on, .. }>`); has `eligible_plans(completed)`, `milestone_order()`, `validate()`. Used only by
  `roko plan queue show/validate/init` (`commands/plan.rs` about lines 1595-1710) and the TUI queue modal
  (`tui/app/mod.rs:361`). `plan queue show` takes completion from the Runner-v2 executor snapshot
  (`commands/plan.rs::read_executor_state`), which Graph runs do not write.
- `crates/roko-cli/src/graph_execution/plan_set.rs`: `plan_set_order` (plan-level `depends_on_plan`),
  `outside_plan_status` (completion evidence: a succeeded Graph checkpoint or all tasks done),
  `PlanFootprint`/`plan_conflicts` (overlapping plans never run together), `PlanSetScheduler` (bounded
  by `[conductor] max_parallel_plans` / `--max-parallel-plans`, default 1).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `run_graph_plan`, entry of `roko plan run`.
  Lines 840-846 reject `--worktree-per-task` with parallel plans ("per-task worktrees are never merged
  back"); lines 1154-1171 attach `WorktreeExecutionWorkspaceProvider` only with `--worktree-per-task`.
- `crates/roko-cli/src/graph_execution/workspaces.rs`: workspace port over `WorktreeManager`.
- `crates/roko-cli/src/graph_execution/delivery.rs`: `CliCompletionDeliveryService` and
  `GitDeliveryBackend` (MergeQueue, PlanMerger regression gate, GitHub publish, `roko.delivery@1`
  checkpoint extension). Nothing outside the module constructs them.
- `crates/roko-cli/tests/graph_plan_callers.rs`: harness to copy. It runs the real `roko` binary with a
  mock agent script shadowing `claude`/`codex`/`gemini` on `PATH`, so no model is called.

## Current state

| Contract step | State at HEAD (d9e79e9d8) | Owner |
|---|---|---|
| 1 queue selection, milestone unlock, overrides | parser and `eligible_plans` exist; `plan run` never calls them | this item |
| 2 dependency-ready, bounded parallel plans | done in `725f21e05` (`PlanSetScheduler`, footprint overlap check) | `gap-0001a1` |
| 3 per-attempt worktrees | opt-in `--worktree-per-task`, never merged back | `gap-4ec59f` |
| 4 serialized merge, conflict evidence, regression gate | built in `delivery.rs`, not wired; `git_merge` checks out in the user's tree | `bug-a3760a` + wiring |
| 5 kill/restart without repeats | SIGINT/SIGTERM exit 130/143 with an `interrupted` checkpoint, `--resume-plan` (`725f21e05`); no process-level proof | `gap-6ca8fb` (parked) |
| 6 pause after N plans | `--batch-size` ignored on Graph; `plan pause` writes an unread `control.json` | `bug-8208a6`, `gap-d60281` |

No `crates/roko-cli/tests/mori_workflow_parity.rs` exists. `tests/merge_proof.rs` covers only the
Runner-v2 `MergeQueue`/`ParallelExecutor` state machine, not a Graph run.

## Plan

1. Write the fixture first, as `crates/roko-cli/tests/mori_workflow_parity.rs`, using the
   `graph_plan_callers.rs` harness: a temp git repo, a mock agent that edits named files, and plans
   `m1-a`, `m1-b` (independent, disjoint files), `m1-c` (same line as `m1-a`, for a conflict) and `m2-x`
   (milestone two, depends on milestone one), plus `.roko/queue.toml`. One test per contract step so
   failures are specific. Record which steps fail; that list is the real remaining work.
2. Queue-driven selection (owned here, no other item covers it). Recommended: `roko plan run` with no
   plan dir (or `--queue <file>`) loads `.roko/queue.toml`, computes completed plans with
   `plan_set::outside_plan_status`, picks the first milestone with incomplete plans (or `--milestone`),
   and runs exactly those plans through the existing plan-set path; `run.model` feeds the CLI model
   override and `run.max_agents` feeds `--max-tasks`. A second invocation then advances to the next
   milestone. Alternative: turn milestones into `depends_on_plan` edges and run the whole queue as one
   plan set. Simpler, but loses the milestone as an operator checkpoint; not recommended.
3. Point `plan queue show` at Graph completion (`outside_plan_status`) instead of `read_executor_state`.
4. Steps 3, 4 and 6 of the contract land through their owner items (`gap-4ec59f`, `bug-a3760a` then
   wiring `CliCompletionDeliveryService` into `run_graph_plan`, `bug-8208a6`/`gap-d60281`). Enable each
   fixture test as its owner lands.
5. For step 5, the fixture only needs one kill (SIGTERM while the mock agent sleeps) then
   `--resume-plan`, asserting completed plans are not re-dispatched (count mock invocations). The full
   kill-point matrix stays with `gap-6ca8fb`.

## Done when

- `mori_workflow_parity.rs` has one test per contract step 1-6, none `#[ignore]`d, all passing against
  the Graph engine with a mock provider.
- `roko plan run` with a queue manifest runs only the current milestone, and a second run advances.
- Verify: `test -f crates/roko-cli/tests/mori_workflow_parity.rs && cargo test -p roko-cli --test mori_workflow_parity -- --include-ignored`

## Notes

- Large and cross-cutting. Do it as a sequence: fixture plus queue selection (steps 1-3 of the plan) can
  start now and in parallel with the owner items; closing waits for them.
- Merge and worktree code mutates git state. Never run the fixture in the repo itself: temp repos only,
  and the merge must not touch the operator's checkout (`bug-a3760a`).
- Do not revive Runner-v2 code (`orchestrator/executor`, `ParallelExecutor`) to pass the fixture; the
  proof must go through `run_graph_plan`.
- Queue override semantics (which `RunOverrides` fields apply, and whether CLI flags beat the manifest)
  are a small design decision: the recommendation is CLI flags win, manifest beats `roko.toml`.
- 2026-09-29 (wk-filer2): bug-a3760a (`809ae920d` on `work/bug-a3760a`, in Rust batch 2) replaced the merge.
  `GitDeliveryBackend::git_merge` now merges with git plumbing only: `merge-base` for the fast-forward and
  already-merged cases, otherwise `git merge-tree --write-tree` plus `commit-tree`, then `update-ref` with the
  expected old value. It never checks out, merges or commits in `workdir`. A target branch that is checked out
  anywhere is left alone, and the result is parked at `refs/roko/delivered/<plan_id>`. The regression runs in a
  temporary detached worktree of the merge commit. `delivery.rs` no longer uses `MergeQueue`: the service's merge
  slot and the compare-and-swap serialize merges. Row 4 of Current state and the `Where` bullet on `delivery.rs`
  describe the old code. What remains for row 4 is the wiring (spec-f830c4), plus bug-453481 (merge the verified
  `commit_oid`) and bug-aaa924 (a warm regression build).
- 2026-10-01 (wk-childenv): partial on work/gap-1555ac (Plan step 3); cargo verification deferred to the batch
  check. `roko plan queue show` now takes completion from Graph state: `QueueManifest::completed_plans`
  (runner/queue_manifest.rs) counts a plan done when its Graph checkpoint succeeded or every task in its
  `tasks.toml` is done (`plan_set::outside_plan_status`, the plan-set prerequisite check). It no longer reads the
  Runner-v2 executor snapshot (`read_executor_state`), which Graph runs never write. Test:
  `completed_plans_read_graph_checkpoints_and_task_status`.
- State at BASE `ebdc0f5d5`: spec-f830c4 (done) wired delivery for `--worktree-per-task` runs, which deliver each
  plan into the run's batch branch without touching the operator's checkout
  (`a_worktree_run_delivers_each_plan_into_its_batch_branch`). Per-task worktrees are still opt-in (gap-4ec59f,
  open); pause and `--batch-size` are still open (bug-8208a6, gap-d60281). Left here: the parity fixture (Plan
  step 1) and queue-driven selection in `plan run` (step 2), which can build on `completed_plans` and
  `eligible_plans`.

## Original notes

No single proven path for queue selection/milestone unlock, dependency-ready scheduling, bounded parallel attempts in distinct worktrees, serialized merge with conflict recovery, durable restart and batch checkpoints; the parity fixture (2 milestones, conflict, kill/restart, pause/resume) was nev...

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control`
- `tmp/tui-parity2/37-MORI-WORKFLOW-PARITY-AUDIT.md#Required parity evidence`
- `tmp/tui-parity2/00-INDEX.md`
- `tmp/dogfood/2026-09-20-final-session.md#P1 (High)`

How to verify: Build the 37 parity fixture against the Graph engine and record which steps fail.

Verified 2026-09-28: still unproven - queue manifests only feed `roko plan queue show/validate/init` (commands/plan.rs:1551-1658), plans run sequentially (graph_execution/plan_runner.rs:1197), per-task worktrees are opt-in (plan_runner.rs:1059), and no parity fixture test exists in crates/roko-cli/tests. runner/queue_manifest.rs and runner/merge.rs still exist under crates/roko-cli/src/runner/ (the import warning was wrong). Absorbs gap-28ee5a.

Re-verified 2026-09-29 at d9e79e9d8: still unproven end to end. Change since last check: 725f21e05 added bounded parallel plan sets (graph_execution/plan_set.rs PlanSetScheduler, [conductor] max_parallel_plans, default 1), so plans are no longer strictly sequential. Still missing: queue-manifest selection and milestone unlock in plan run (QueueManifest only feeds `plan queue` subcommands), merge-back of per-task worktrees (opt-in; plan_runner.rs:840-845 rejects them with parallel plans because they are never merged back), and the 37 parity fixture test (2 milestones, conflict, kill/restart, pause/resume).
