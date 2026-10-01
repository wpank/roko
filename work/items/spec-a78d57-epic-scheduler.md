+++
id = "spec-a78d57"
kind = "spec"
title = "Epic: scheduler"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-graph/engine", "roko-cli/graph_execution", "roko-cli/task_parser"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e7"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #11); tldr/04 step 5; tldr/research/B2-dag-worktrees-merge.md"
anchors = ["crates/roko-graph/src/engine.rs::execute_ready_queue", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/task_parser.rs::default_max_parallel"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["gap-4d835d", "gap-96d348", "gap-439794", "gap-272448", "gap-c89b40", "gap-987064", "gap-1920ba", "gap-51deff", "gap-19e596", "gap-3006e9"], blocks = [], related = ["gap-a8d786", "gap-7147bb", "gap-4ec59f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scheduler_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test scheduler_canary"
+++

## Problem

tldr/04 step 5 asks for ready-queue dispatch, as wide as the DAG and the tasks' write sets allow. At `d9e79e9d8` four
things stood in the way:

- a plan ran in topological waves, with a barrier between waves;
- one failed task skipped every later wave, related or not;
- `max_parallel` defaulted to 1;
- nothing kept two tasks that edit the same files from running at once.

Today's merge fixed the first two. The other two are open, and nothing limits cargo builds across roko processes
that share a target dir.

## Why it matters

Parallel cheap executors are where the speed comes from, yet on the portal runs a plan reached only 1.2–2.2
concurrent tasks against caps of 2–4 (B7, via tldr/04). Unsafe parallelism produces false failures: in plan 08b a
sibling's half-written file failed T08's whole-project gate three times, and FailFast then skipped three unrelated
tasks (CASE-002). This is W8's gate G6; in the hot chain E7 follows E5 (PLAN.md §4).

## Where

- `crates/roko-graph/src/engine.rs::execute_ready_queue` (:1047): the one scheduler both parallel paths now use.
  Runnable nodes are admitted at :1199 (`while running.len() < max_concurrent`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: resolves `max_parallel` (:1912) and sets
  `SkipFailed` (:2017).
- `crates/roko-cli/src/task_parser.rs::default_max_parallel` (:53) returns 1.
- `crates/roko-cli/src/runner/gate_dispatch.rs`: compile permits, which work inside one process only.

## Current state

Checked at `41c7ffbd6` (after merge `bbf6517fc`).
- **gap-4d835d looks done.** `445a60d0d` replaced both wave loops with `execute_ready_queue`: a node starts once its
  own dependencies settle. Its verify test, `a_ready_node_does_not_wait_for_its_wave`, exists (engine.rs:5371). Close
  it with that commit.
- **gap-96d348 looks done.** `3e7552acd` sets `FailureStrategy::SkipFailed` for plan runs (plan_runner.rs:2017), after
  the checkpoint identity is taken, so old checkpoints still resume. The proof is
  `a_failed_task_blocks_only_its_dependants` (plan_runner.rs:2597). The item's `[[verify]]` names
  `plan_failure_strategy_skip_failed_runs_independent_tasks`, which does not exist, so its cargo part passes without
  running a test; the closer should cite the real test. SkipFailed is hard-coded, which E7 accepts; plan sets keep
  the opt-in `--fail-fast`.
- **gap-439794 is open, but smaller.** The lock now goes in one place, the admission loop of `execute_ready_queue`.
  The two wave loops and the line numbers in its body are stale.
- **gap-272448 (new).** 99 of the 136 plans in `plans/` set `max_parallel = 1` and 8 omit it; the generator prompt
  asks for 1 (`plan_generate.rs:239`).
- **gap-c89b40 is open.** `gates.compile_concurrency` coordinates one process only.

## Plan

This is the implementation plan.

1. **Close the two done items** (gap-4d835d, gap-96d348) with the evidence above; a reviewer runs `work.py close`.
2. **Write-set admission** (gap-439794, M). In `execute_ready_queue`, admit a runnable node only if its declared
   `files` overlap no running node's. A blocked node holds no slot, and the fingerprint does not change.
3. **Auto width** (gap-272448, S), after step 2: an omitted `max_parallel` means the DAG's width when every task
   declares its files.
4. **Cross-process build slots** (gap-c89b40, M). Different files from steps 2–3, so it can run beside them.
5. **Exit check C6** (gap-987064, S), after steps 2–3.

## Done when

- [x] gap-4d835d: Graph engine runs a plan wave by wave, so a ready task waits for its whole wave (existing item;
      looks done, close with `445a60d0d`)
- [x] gap-96d348: FailureStrategy::SkipFailed unreachable; FailFast hardcoded (existing item; looks done, close with
      `3e7552acd`)
- [x] gap-439794: File-conflict detection before same-wave task dispatch (existing item)
- [x] gap-272448: max_parallel defaults to the plan's DAG width when task write sets are disjoint
- [x] gap-c89b40: Nothing limits concurrent cargo builds across processes that share a target dir (existing item)
- [x] gap-987064: Integration test C6: independent tasks still run after a failure, and tasks with overlapping files
- [x] gap-1920ba: File exclusion misses read-against-write races: a whole-project verify reads a sibling's half-written file
- [x] gap-51deff: In the rich topology a task's files are free between its executor and its gate, so an overlapping task can run in between
- [x] gap-19e596: Turn file exclusion off under --worktree-per-task, where tasks do not share a tree
- [x] gap-3006e9: The Graph engine records no task-ready or dispatch time, so the slot waits of multi-task plans can't be measured
      never run together
- [ ] The epic's `[[verify]]` command (test C6) passes on the merged branch.

## Notes

- **Existing children keep their goal and severity:** gap-4d835d (`core`, p1), gap-96d348 (`core`, p2), gap-439794
  (`core`, p1) and gap-c89b40 (`tooling`, p2). PLAN.md §5 proposes moving them to `golden-path`.
- W5 worried that the `sched` claim carried gap-386329's title. The merged commits name gap-4d835d, the right item.
- **Related:** gap-a8d786 (E8.7, a lint for overlapping files between tasks that can run together) should reuse step
  2's overlap rule. gap-7147bb: `--max-tasks` changes the plan fingerprint, and step 3 must not. gap-4ec59f: with
  per-task worktrees, write-set admission can relax.
- **Hot files:** `roko-graph/src/engine.rs` and `graph_execution/plan_runner.rs`, one writer each.
