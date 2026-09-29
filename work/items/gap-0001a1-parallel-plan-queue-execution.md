+++
id = "gap-0001a1"
kind = "gap"
title = "Parallel Plan Queue Execution"
status = "superseded"
triage = "verified"
severity = "p1"
size = "S"
goal = "features"
subsystem = ["roko-cli/runner"]
created = 2026-09-05
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution"
discovered_from = "audit:tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1340", "crates/roko-cli/src/graph_execution/plan_set.rs::PlanSetScheduler", "crates/roko-cli/src/graph_checkpoint.rs::resolve_checkpoint_paths", "crates/roko-serve/src/routes/plans.rs::active_run_conflict"]
links = { depends_on = [], blocks = [], related = ["gap-a80e05", "gap-7c9e48", "gap-be8416", "gap-9084e7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^fn resolve_checkpoint_paths/,/^}/p' crates/roko-cli/src/graph_checkpoint.rs | grep -q run_id"

[closed]
at = 2026-09-29
by = "work enrichment 2026-09-29 (static check)"
evidence = "Superseded by gap-d58ae8: Parallel plan sets landed in 725f21e05 (graph_execution/plan_set.rs::PlanSetScheduler, plan_runner.rs:1340), and 08a1fd272 deliberately made one runner per workspace (workspace_lock.rs:72 acquire_runner_lock; roko-serve routes/plans.rs:213 active_run_conflict) with shared per-plan checkpoints; the remaining isolation/merge/queue work is tracked in gap-d58ae8, gap-4ec59f, gap-7c9e48 and find-8872ad."
+++

## Problem

Backlog #272 (2026-09-04) asked for several independent Graph plan runs to execute at once in one repository.
Each run was to have its own checkpoint (`.roko/state/graph/runs/<run_id>/`), Activity ledger, worktree
namespace, budget, control channel and status identity. A bounded queue/supervisor
(`PlanQueueConfig { max_active_runs, max_queued_runs, per_provider }`) would admit runs, cancel one run or the
whole queue, and rebuild itself after a restart.

At HEAD the codebase went a different way:

- Plans of one selected set run concurrently inside one run (`725f21e05`).
- Each workspace deliberately has one runner (`08a1fd272`).
- Checkpoints are per plan, shared by CLI and server runs.

A second independent run is refused, not queued: the CLI hits the runner lock, and serve answers 409.

## Why it matters

Goal `features`. The throughput need behind #272 (run many plans at once) is met inside a single run. What is
left is either against a later decision or tracked elsewhere:

- `gap-d58ae8`: Mori-shaped workflow. Bounded parallel attempts in distinct worktrees, serialized merge with
  conflict recovery, durable restart, queue-manifest selection.
- `gap-4ec59f`: worktree isolation on by default, plus startup repair.
- `gap-7c9e48`: plan-set dispatch. The default `max_parallel_plans = 1`, footprint admission, and no live
  30-plan or diamond run recorded yet.
- `find-8872ad` (p3): serve admits one run per workspace. A second independent set gets 409, so the client has
  to submit the sets together.

Risk of keeping this item open as written: its verify command requires a `run_id` in
`resolve_checkpoint_paths`. A worker who satisfies it would move checkpoints to run-scoped paths. That breaks
the shared per-plan checkpoint, which lets a CLI run and a server run resume each other.

## Where

- `crates/roko-cli/src/graph_execution/plan_set.rs::PlanSetScheduler`: admits plans of one set. A plan starts
  when its `depends_on_plan` prerequisites succeeded, a slot under `max_parallel_plans` is free, and its
  `PlanFootprint` overlaps no running plan. The module doc says every plan of a set runs in the operator's
  working tree.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `run_graph_plan` drives the set with a
  `FuturesUnordered` (scheduler built at :1340). `max_parallel_plans` is resolved from the run override or
  `[conductor] max_parallel_plans` (:834-839). `--worktree-per-task` is refused with parallel plans (:840-846).
  `AgentSlotDispatcher` caps concurrent tasks across all plans with `[conductor] max_agents`.
- `crates/roko-cli/src/graph_checkpoint.rs::resolve_checkpoint_paths` (:1059): checkpoint, activities and
  costs live per plan id under `.roko/state/graph/<plan>/`.
- `crates/roko-cli/src/workspace_lock.rs::acquire_runner_lock` (:72): exclusive `.roko/runtime/roko.runner.lock`.
  Taken by `roko plan run` (`commands/plan.rs:597`), `roko do` (`commands/do_cmd.rs:378`) and serve's run path
  (`serve_runtime.rs:834`).
- `crates/roko-serve/src/routes/plans.rs::active_run_conflict` (:213): `execute_plans` (:291) and
  `execute_plan` (:578) return 409 while any run is active.
- `crates/roko-agent` `ProviderSemaphores` (used in `crates/roko-cli/src/dispatch_v2.rs:1453`): per-provider
  concurrency permits.

## Current state

- Done: bounded parallel plan sets (`725f21e05`, 2026-09-28): `PlanSetScheduler`, `--max-parallel-plans`,
  `[conductor] max_parallel_plans` (default 1).
- Decided the other way: one runner and one hub per workspace, CLI delegation to a running server, and server
  run sets (`08a1fd272`, plan 03b, 2026-09-29). The portal contract (`tmp/portal-audit/03-CONTRACT.md` §2.3,
  §4.1; not in worktrees) found that "two runners can mutate one working tree" and fixed that by making every
  run hold `roko.runner.lock`. It also made plan directories run in place, "so a CLI run and a server run share
  one checkpoint". That is the opposite of #272's run-scoped checkpoint namespace.
- Not done, and tracked elsewhere:
  - concurrent plans are not isolated from the shared working tree (`gap-d58ae8`, `gap-4ec59f`);
  - there is no queue for a second run (`find-8872ad`);
  - the `max_parallel_plans` default and a live proof (`gap-7c9e48`).
- The note in this item's original text "no run lock was found" is stale: `acquire_runner_lock` enforces one
  run per workspace.
- Related parked item: `gap-a80e05` (DFA-06 #272: parallel plan execution blocked by a workspace-global lock).

## Plan

1. Close this item as superseded, with evidence: `725f21e05` (parallel plan sets), `08a1fd272` (one runner per
   workspace, shared per-plan checkpoints), and pointers to `gap-d58ae8`, `gap-4ec59f`, `gap-7c9e48` and
   `find-8872ad`.
2. If the owner still wants independent concurrent runs, do not reuse #272's design. Make it a new decision
   item first, because it reverses the one-runner rule. The smallest useful step is a serve-side queue:
   - `execute_plans` enqueues a set whose plans are disjoint from the running set, instead of returning 409;
   - the queue is persisted under `.roko/state/` and started when the active run finishes;
   - checkpoints stay per plan, and a plan may appear in at most one queued or running set.

   Track that under `find-8872ad`.

## Done when

- The item is closed as superseded, `[closed].evidence` cites `725f21e05` and `08a1fd272`, and
  `links.duplicate_of` or related points at `gap-d58ae8`.
- Nothing in the tracker still asks for run-scoped checkpoint paths (`.roko/state/graph/runs/<run_id>/`)
  without a decision item that reverses the one-runner rule.

## Notes

- Do not implement the current `[[verify]]` (`run_id` inside `resolve_checkpoint_paths`). It encodes the #272
  storage layout, which the portal programme replaced. Moving checkpoints would break CLI/serve shared resume
  and `roko resume <plan>`.
- The original #272 scope, if reinstated, is several days (L) and touches persistence (checkpoints, queue
  snapshot) and lock scopes. Treat it as high risk.
- #272's hard dependencies (#249 attempt worktrees, #251 run ledger, #254 merge queue) were Runner-v2-era
  backlog numbers. Runner-v2 was deleted on 2026-09-06 (`6b5da8616`).

## Original notes

[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/272-parallel-plan-queues.md#272 — Parallel Plan Queue Execution`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.5 Proof Case 5: Concurrent plan runs`

How to verify: Check: Implement the exact files, types, storage paths, and lock scopes above.; Give every run its own checkpoint, Activity ledger, worktree namespace, budget, control channel, and status identity.; Add a bounded queue/supervisor with configurable… [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: still open - the Graph runner executes plans one at a time in dependency order (graph_execution/plan_runner.rs:1197) and checkpoints live per plan id under .roko/state/graph/<plan>/ (graph_checkpoint.rs:884); there is no run-scoped namespace, bounded queue or supervisor.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Added in 725f21e05: the Graph runner no longer has to run plans strictly one at a time. PlanSetScheduler (crates/roko-cli/src/graph_execution/plan_set.rs:1-8; used at graph_execution/plan_runner.rs:1210-1308) starts plans concurrently, bounded by max_parallel_plans (plan_runner.rs:626-630; [conductor] default 1; flag plumbed through commands/plan.rs:486,550,606) and blocked by depends_on_plan and footprint overlap. What remains of #272: all plans still share the operator's working tree (plan_set.rs:4), checkpoints and activities are still keyed per plan id under .roko/state/graph/<plan>/ (resolve_checkpoint_paths, graph_checkpoint.rs:1059-1112, working tree with large uncommitted edits), and no run lock was found. There is also no run-scoped namespace (checkpoint, activity ledger, worktree namespace, budget, control channel, status identity) and no queue or supervisor for independent concurrent runs.

Checked 2026-09-29 at d9e79e9d8: the remainder is unchanged. Plan 03b (08a1fd272) added server run sets and a one-run-per-workspace admission in crates/roko-serve/src/routes/plans.rs (active_run_conflict): POST /api/plans/{id}/execute and POST /api/plans/execute return 409 while any run is active, so a second independent run is refused, not queued (recorded as find-8872ad). --worktree-per-task now refuses max_parallel_plans > 1 (graph_execution/plan_runner.rs:840). Still missing: a run-scoped namespace (checkpoint, activity ledger, worktree namespace, budget, control channel, status identity), isolation of concurrent plans from the shared working tree, and a bounded queue or supervisor for independent concurrent runs.
