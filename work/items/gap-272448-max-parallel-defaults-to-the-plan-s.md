+++
id = "gap-272448"
kind = "gap"
title = "max_parallel defaults to the plan's DAG width when task write sets are disjoint"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/task_parser", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4d79f0016"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e7"
discovered_from = "tmp/cybernetic-harness/tldr/research/B2-dag-worktrees-merge.md (Per-plan task concurrency row); tldr/05 P1 #11"
anchors = ["crates/roko-cli/src/task_parser.rs::default_max_parallel", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan", "crates/roko-cli/src/plan_policy.rs::validate_plan_budgets", "crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT"]
lane = "rust-hot"
parent = "spec-a78d57"
links = { depends_on = ["gap-439794"], blocks = [], related = ["gap-7147bb", "gap-c89b40", "gap-2623b2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn omitted_max_parallel_runs_disjoint_tasks_together' crates/roko-cli/src/ && cargo test -p roko-cli --lib omitted_max_parallel_runs_disjoint_tasks_together"
+++

## Problem

`[meta] max_parallel` defaults to 1 (`task_parser.rs::default_max_parallel`, :53), and the generator prompt tells
planners to write `max_parallel = 1  # default to 1 for safety` (`plan_generate.rs:239`, and its checklist at :363). Of
the 136 plans in `plans/`, 99 set 1 and 8 omit the key. So almost every plan runs one task at a time, however wide its
DAG. Since `bbf6517fc` the ready queue starts a task as soon as its dependencies settle, but still one at a time.

## Why it matters

tldr/05 P1 #11: `max_parallel` should equal the DAG's width when write sets are disjoint. Once write-set admission
(gap-439794) keeps overlapping tasks apart, 1 is no longer the safe default, only the slow one. This is step 3 of
epic spec-a78d57.

## Where

- `crates/roko-cli/src/task_parser.rs`: `TaskMeta.max_parallel` and `default_max_parallel`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan`: `--max-tasks`, else `meta.max_parallel` (:1912),
  goes into `plan_to_graph`; the checkpoint identity is taken at :1998.
- `crates/roko-cli/src/plan_policy.rs::validate_plan_budgets`: `PLAN_BUDGET_PARALLEL` rejects 0 (:216).
- `crates/roko-cli/src/plan_generate.rs::PLAN_GENERATOR_SYSTEM_PROMPT`: the `max_parallel = 1` lines.

## Current state

Checked at `41c7ffbd6`. `[conductor] max_agents` (default 8, `graph_execution/agent_slots.rs`) already caps concurrent
tasks across a run. `plan_graph_fingerprint` hashes the non-default policy fields, `max_concurrent_nodes` among them,
and an authored `max_parallel` (gap-7147bb).

## Plan

1. An omitted `max_parallel` means auto. Recommended: a serde default of 0 meaning auto, which keeps every reader of
   the `u32` compiling; `PLAN_BUDGET_PARALLEL` then accepts 0. An `Option<u32>` is cleaner but touches every reader.
2. Resolve auto in `run_one_plan`. If every task declares a non-empty `files` list, use the task count: the ready queue
   never runs more tasks than the DAG allows, so this is in effect the DAG's width, and `max_agents` still caps it. If
   any task declares no files, its write set is unknown: use 1 and log why.
3. Keep checkpoints resumable: convert the graph with 1, as today, and raise `max_concurrent_nodes` only after
   `prepare_graph_checkpoint` has taken the identity, as `3e7552acd` does for the failure strategy.
4. `--max-tasks` and an explicit `max_parallel` still win. The generator stops asking for `max_parallel = 1`.
5. Test `omitted_max_parallel_runs_disjoint_tasks_together` (`plan_runner.rs`, fake provider as in
   `a_failed_task_blocks_only_its_dependants`): three independent tasks with disjoint `files` overlap in time; with
   one file-less task they run one at a time; an old checkpoint still resumes.

## Done when

- [ ] A plan that omits `max_parallel`, with disjoint `files` on every task, runs its independent tasks together.
- [ ] Plans with an explicit value, or with a task that declares no files, behave as before, and old checkpoints resume.
- [ ] The `[[verify]]` command passes.

## Notes

- Waits for gap-439794: without write-set admission, overlapping tasks in an auto-width plan would share one tree.
- More parallel Rust tasks mean more concurrent cargo builds; gap-c89b40 adds the cross-process limit.
- E8.2 (gap-2623b2) rewrites the generator prompts. If it lands first, change the `max_parallel` guidance there.
- **Hot file:** `plan_runner.rs`.
- **Decided 2026-09-29 (Will):** use `Option<u32>` (an omitted value means as wide as the DAG allows) rather than a 0 sentinel.
- Implemented on `work/gap-a8d786` at `c5b3930a7`; cargo verification deferred to the batch check.
  - `TaskMeta.max_parallel` is `Option<u32>`. `plan_policy::plan_max_parallel` resolves an omitted value. If every
    task that can write declares `files`, it is the task count. Otherwise it is 1, and
    `task_with_unknown_writes` names the task. Roles that cannot write, such as researchers, don't count.
  - `run_one_plan` converts an omitted value as 1, then raises `max_concurrent_nodes` after
    `prepare_graph_checkpoint`, so checkpoint identities are unchanged. The test compares the recorded fingerprint
    with the one for 1.
  - `PLAN_CONCURRENT_OVERLAP` now also applies under auto. `roko plan run` doesn't refuse it (`ff47a28ab`). The
    generated-plan budget still rejects it, so the generator's retry asks the planner to fix the overlap.
  - No plan under `plans/` omits `max_parallel` today. One test fixture (`plan_validate_warns_on_known_model_aliases`)
    now pins `max_parallel = 1`.
