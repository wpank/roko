+++
id = "gap-7c9e48"
kind = "gap"
title = "Wave Execution Dispatch Loop"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/commands"]
created = 2026-09-21
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop"
discovered_from = "audit:tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs::PlanSetScheduler", "crates/roko-cli/src/graph_execution/plan_set.rs::PlanFootprint::of", "crates/roko-cli/src/graph_execution/plan_set.rs::CargoWorkspace::load", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-core/src/config/schema.rs::default_max_parallel_plans", "roko.toml:343", "crates/roko-cli/tests/graph_plan_callers.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

Backlog #396 asked `roko plan run <dir>` to run independent plans at the same time instead of strictly one after
another. The concurrent dispatch loop now exists (commit `725f21e05`), but three things keep the item open:

1. A bare `roko plan run plans/` still runs one plan at a time. The concurrency limit defaults to 1
   (`[conductor] max_parallel_plans = 1`), so parallel plans only happen with `--max-parallel-plans N` or a config
   change. Nobody has recorded whether opt-in is the intended design.
2. Even with a higher limit, many "independent" plans will not start together. The footprint check makes a plan
   run alone when any of its tasks declares no `files`/`crates_touched`, or writes a workspace build input such
   as the root `Cargo.toml`. It also keeps two plans apart when one writes a package the other builds. When
   `cargo metadata` fails, every plan that edits Rust conflicts with every other one.
3. Nothing proves plans actually overlap end to end. The only concurrency tests are scheduler unit tests in
   `plan_set.rs`. No integration test and no recorded live run show two plans running at once, or the diamond
   A → {B, C} → D running B and C together.

Expected: with independent plans and a limit above 1, the run log shows every admitted plan starting before
the first one finishes, and there is a test that fails if that ever stops being true.

## Why it matters

- Goal `core` (plan runs work reliably). The original estimate was that 30-plan batches took about 5x longer
  when run one at a time. That is a throughput gap, not a broken loop, which is why the item is p1 and not p0.
- The code path exists but is unproven. A future refactor of `run_graph_plan` could quietly serialize plans
  again and no test would notice.
- Related: `gap-0001a1` (run-scoped namespaces and a queue for independent runs; separate, larger work),
  `gap-c09fc7` (agents are not told other plans run beside them), `gap-439794` (file conflicts inside a plan's
  wave), `find-8872ad` (server admits one run per workspace), `gap-6bc156` (inert `max_concurrent_plans` knobs),
  `gap-d58ae8` (Mori-shaped workflow contract), `gap-be8416` (older copy of this gap, already superseded).

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan` (line ~701): the entry point for
  `roko plan run`, `roko do --plan`, `roko prd plan --execute` and serve plan runs.
  - Lines ~835-846 resolve the limit: `max_parallel_plans.unwrap_or(roko_config.conductor.max_parallel_plans).max(1)`.
    They also refuse `--worktree-per-task` when the limit is above 1.
  - Lines ~1087-1093 compute `plan_set_conflicts`, but only when the limit is above 1 and there is more than one
    plan.
  - Lines ~1336-1420 are the plan-set driver: a `PlanSetScheduler`, a `FuturesUnordered` of
    `run_admitted_plan` futures, and per-plan `PlanControl`s for TUI commands.
- `crates/roko-cli/src/graph_execution/plan_set.rs`:
  - `PlanSetScheduler::{new, admit, finish, stop}` decides what starts. A plan needs its prerequisites to have
    succeeded, a free slot, and no footprint overlap with a running plan.
  - `PlanFootprint::of` builds each plan's write, build and affects areas, and decides when a plan runs alone.
  - `CargoWorkspace::load` runs `cargo metadata`. When it fails it returns `None`, and then every Rust plan
    conflicts with every other Rust plan.
- `crates/roko-core/src/config/schema.rs::default_max_parallel_plans` (line ~1761) returns `1`. The repo's
  `roko.toml` line 343 also sets `max_parallel_plans = 1`.
- `crates/roko-cli/src/main.rs` (line ~2155-2165): the `--max-parallel-plans N` flag, range 1..=64. Its help text
  says the default is 1.
- `crates/roko-cli/src/runner/plan_dag.rs::CrossPlanDag::compute`: now used only by `plan_set_entries`
  (plan_runner.rs ~line 95) to label each plan's wave for the dashboard, plus `plan list --waves`. It does not
  drive dispatch, and it does not need to.
- `crates/roko-cli/tests/graph_plan_callers.rs`: an end-to-end harness that runs the real `roko` binary with a
  mock agent script shadowing `claude`/`codex`/`gemini` on `PATH`. `--log-file events.jsonl` records
  `dashboard.plan_started` and `dashboard.plan_completed` events. Use this pattern for the missing test.

## Current state

- Done in `725f21e05`: scheduler-driven concurrent dispatch; `--max-parallel-plans` and
  `[conductor] max_parallel_plans`; per-plan controls (cancel-before-start reaches the right plan); fail-fast;
  prerequisite blocking (a failed plan's dependents are reported blocked, independent plans still run); footprint
  conflicts; the in-flight budget log line.
- Later commits touching these files (`5c62bf0d4`, `4ca38b5c8`, `c41e78c7a`, `188c43c8d`) came from the portal
  programme. They did not change the scheduling rules.
- Scheduler unit tests in `plan_set.rs`: `one_slot_starts_plans_in_execution_order`,
  `independent_plans_share_the_slots`, `dependents_wait_for_success_and_later_plans_do_not`,
  `conflicting_plans_never_overlap_and_keep_their_order`, `a_later_plan_does_not_overtake_an_earlier_conflicting_one`,
  `fail_fast_blocks_everything_after_a_failure`, `stop_and_cancel_leave_plans_unstarted`,
  `without_a_package_graph_rust_plans_conflict_and_others_do_not`.
- Tests that do not exist: an integration test that runs `run_graph_plan` or the binary with two or more plans
  and a limit above 1, and a diamond test.
- The old `[[verify]]` (`grep -q CrossPlanDag plan_runner.rs`) was removed on 2026-09-29. It matched the
  display helper and passed whether or not plans ran concurrently.

## Plan

1. Decide the default and record it in `work/DECISIONS.md`.
   - Option A (recommended for now): keep 1 and document that parallel plan sets are opt-in. Reasons: plans share
     the operator's working tree; agents' ad-hoc `cargo build` can see another plan's half-finished edits
     (`gap-c09fc7`); tasks inside a plan still have no file-overlap check (`gap-439794`); and the worst-case spend
     multiplies by the limit, since each plan has its own `max_plan_usd` ceiling.
   - Option B: raise `default_max_parallel_plans` to a small number (2-4) and change `roko.toml:343`, the
     `main.rs` flag help, `tui/config_meta.rs` (`conductor.max_parallel_plans`) and the doc comment at
     plan_runner.rs ~674 to match. Only do this after `gap-c09fc7` lands.
2. Add an end-to-end test, `independent_plans_run_side_by_side`, in `crates/roko-cli/tests/graph_plan_callers.rs`
   or a new `tests/plan_set_parallel.rs`:
   - Seed two plans (`plans/a/tasks.toml`, `plans/b/tasks.toml`) with no `depends_on_plan`. Give them disjoint,
     non-Rust `files` (for example `notes/a.md` and `notes/b.md`) so the footprint check does not serialize them.
   - Make the mock agent sleep about 2 s so the plans have time to overlap.
   - Run `roko --json plan run plans --no-tui --max-parallel-plans 2 --log-file events.jsonl`.
   - Assert that both `dashboard.plan_started` events come before the first `dashboard.plan_completed`.
   - Add a control case with `--max-parallel-plans 1` and assert no overlap.
3. Add a diamond test, `diamond_plan_set_runs_middle_plans_together`: plans A, B→A, C→A, D→{B, C}, limit 4.
   - Assert that B and C both start after A completes and before either of them completes.
   - Assert that D starts after both B and C complete.
   - A scheduler-level unit test in `plan_set.rs` next to `independent_plans_share_the_slots` is enough if the
     end-to-end version is too slow.
4. Improve the "why is my plan waiting" output. The driver already logs `plan waits: it cannot share the working
   tree` and emits `graph.plan_waiting`. Check that `roko plan run --no-tui` shows the reason at the default log
   level, so an operator can see why 30 plans did not all start. Keep the conflict rules as they are. Loosening
   them is a separate decision.
5. Optional, recorded as evidence and not required for the verify: run a live set of 3 or more independent
   non-Rust plans with `--max-parallel-plans 3`, and note the run id in this item's `[closed]` evidence.

## Done when

- `work/DECISIONS.md` records whether plan-set parallelism is opt-in (default 1) or on by default. If it is on by
  default, the config default, `roko.toml`, the CLI help and the TUI config metadata all agree.
- An integration test shows two independent plans overlapping with `--max-parallel-plans 2`, and not overlapping
  with `--max-parallel-plans 1`.
- A test covers the diamond order A → {B, C together} → D.
- Verify:
  `grep -rqw 'fn independent_plans_run_side_by_side' crates/roko-cli/ && cargo test -p roko-cli independent_plans_run_side_by_side && cargo test -p roko-cli independent_plans_share_the_slots`

## Notes

- Do not bring back `CrossPlanDag` waves as the dispatch mechanism. Wave barriers make a whole wave wait for its
  slowest plan. The scheduler admits each plan as soon as its own prerequisites finish, which is strictly better.
  `CrossPlanDag` stays as a display helper.
- `--worktree-per-task` with a limit above 1 is refused on purpose (plan_runner.rs ~840), because per-task
  worktrees are never merged back. Do not lift that refusal here. See `gap-d58ae8` and `gap-0001a1`.
- Checkpoints are keyed per plan under `.roko/state/graph/<plan>/`. Concurrent plans in one run are fine, but two
  concurrent runs of the same workspace are not. That is `gap-0001a1` and `find-8872ad`, not this item.
- The end-to-end test runs the real binary. Isolate `HOME`, shadow the agent CLIs, and unset `ANTHROPIC_API_KEY`
  the way `graph_plan_callers.rs::run_roko` does, so no real model is ever called.
- Safe to do in parallel with most work. It collides with anything that edits `plan_runner.rs` around the
  plan-set driver: `bug-230de6` (events.jsonl) and `bug-a3760a` (merge step) touch the same module.

## Original notes

largest single performance gap; 30-plan batches take 5x longer without wave parallelism. `CrossPlanDag::compute` (in `crates/roko-cli/src/runner/plan_dag.rs`) uses Kahn's algorithm to group plans into execution waves. Wave 0 contains all plans with no cross-plan dependencies; wave N contains all…

Imported without verification from:
- `tmp/backlog/archive/396-wave-execution-dispatch.md#396 — Wave Execution Dispatch Loop`

Some cited files are gone: `crates/roko-core/src/config.rs`.

How to verify: Check: `roko plan run plans/` with 30 independent plans dispatches all 30 graph engines before any one completes (confirmed via `RUST_LOG=info` showing concurrent `running plan via Graph Engine` lines).; `roko plan run plans/` with a diamond… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: crates/roko-cli/src/graph_execution/plan_runner.rs:1197 still runs plans strictly one after another (`for plan_id in &plan_execution_order`, each awaited); CrossPlanDag::compute is used only for `roko plan list --waves` and summaries (commands/plan.rs:67, :382). Severity lowered p0 -> p1: a throughput gap, not a broken core loop.

Re-verified 2026-09-29: the dispatch loop landed in 725f21e05. It consists of graph_execution/plan_set.rs::PlanSetScheduler with DAG-aware admission and footprint-conflict waiting, a FuturesUnordered plan-set driver in plan_runner.rs::run_graph_plan, and --max-parallel-plans / [conductor] max_parallel_plans. Three things remain. First, the default is still 1 (roko-core config schema.rs default_max_parallel_plans, roko.toml:343), so a bare `roko plan run plans/` does not dispatch independent plans concurrently. Either raise the default or record that opt-in is the intended design. Second, footprint admission serializes plans with overlapping write/build areas, and without cargo metadata it treats every Rust edit as touching every package, so 30 independent Rust plans may not all start at once. Third, no live 30-plan or diamond run has been recorded. CrossPlanDag::compute now feeds only the plan_set_entries summary (plan_runner.rs:95), not dispatch.

The [[verify]] command is unsound (see the check notes). Proposed replacement, not yet validated: `cargo test -p roko-cli independent_plans_run_side_by_side && cargo test -p roko-cli independent_plans_share_the_slots. The current `grep -q CrossPlanDag plan_runner.rs` matches the plan_set_entries display helper (plan_runner.rs:95) and passes whether or not plans run concurrently.`.

Removed [[verify]] `grep -q 'CrossPlanDag' crates/roko-cli/src/graph_execution/plan_runner.rs` on 2026-09-29: it passes while the problem still exists.

Checked 2026-09-29: Checked against the ready-queue scheduler (bbf6517fc): it changes scheduling inside a plan (ready tasks no longer wait for their whole wave, and a failure skips only its dependants), not across plans. max_parallel_plans still defaults to 1 and no integration test shows two plans overlapping, so all three points above still stand.

2026-10-01 (wk-planrun): partial on work/gap-dd4826; cargo verification deferred to the batch check. The item stays open for the default decision (Plan step 1).
- Step 2 was already done at BASE, and this item's Current state misses it. `independent_plans_run_side_by_side` and `one_plan_at_a_time_keeps_the_execution_order` (`graph_execution/plan_runner.rs`, since `725f21e05`) run `run_graph_plan` with two independent plans at `max_parallel_plans` 2 and 1. They assert that both plans start before either ends, and that one slot keeps the execution order. `main.rs` tests the `--max-parallel-plans` parsing. The `[[verify]]`'s test names resolve at BASE.
- Step 3, new: `diamond_plan_set_runs_middle_plans_together` (`graph_execution/plan_set.rs`, next to `independent_plans_share_the_slots`). In A → {B, C} → D with four slots, B and C start together once A succeeds, and D starts only when both have.
- Step 4 is fine as it is: a waiting plan logs `plan waits: it cannot share the working tree with a running plan` (with `waits_for` and `reason`) at `info`. The CLI's default directive `roko=info` shows it on stderr without `--tui`.
- Still open: step 1, whether plan-set parallelism stays opt-in (`default_max_parallel_plans` = 1, `roko.toml` `max_parallel_plans = 1`) or is on by default. That is Will's call, and needs a decision item for `work/DECISIONS.md`; the Plan recommends staying opt-in until gap-c09fc7 lands. Step 5 (a live run of 3 or more independent plans) needs a provider.
