+++
id = "bug-ae28ac"
kind = "bug"
title = "Daily and agent-lifetime budget limits are not enforced by the Graph engine"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::admit_task_budget", "crates/roko-cli/src/graph_task_dispatch/budget.rs::task_budget_ceiling_usd", "crates/roko-cli/src/graph_execution/plan_runner.rs::resolve_budget_ceiling", "crates/roko-learn/src/costs_log.rs::CostsLog::cost_today", "crates/roko-core/src/config/budget.rs::BudgetConfig"]
links = { depends_on = [], blocks = [], related = ["gap-d31457", "q-778b4f", "q-e23804"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_daily_budget_blocks_dispatch' crates/roko-cli/src/ && cargo test -p roko-cli graph_daily_budget_blocks_dispatch && ! grep -A1 -E '\"budget\\.max_(daily|agent_lifetime)_usd\"' crates/roko-cli/src/graph_task_dispatch.rs | grep -q NOT_ENFORCED"
+++

## Problem

`[budget].max_daily_usd` and `[budget].max_agent_lifetime_usd` in `roko.toml` have no effect on
`roko plan run`. Set `max_daily_usd = 1.0`, spend more than $1 today, and `plan run` still dispatches
every task. The only sign is a one-time `tracing::warn!` ("config keys set to non-default values have no
effect on `plan run`") and a line in `roko config doctor`, both produced by `graph_engine_inert_settings`.

Expected: once today's spend (UTC calendar day, across all runs) reaches `max_daily_usd`, the Graph
dispatcher refuses new provider dispatches with a clear `BudgetExceeded` error, the way the deleted
Runner-v2 loop did. `max_agent_lifetime_usd` should either be enforced or stop being advertised as a
plan-run control.

## Why it matters

Goal `core` (plan runs work reliably). A daily ceiling is the operator's last guard against a runaway
plan set or a retry loop spending money overnight. Today the key is accepted and silently ignored,
which is worse than not having it.

Related: `gap-d31457` (plan ceiling; an explicit `--budget-override <AMOUNT>` only warns). Parked
questions `q-778b4f` and `q-e23804` (all budget defaults are `0.0` = unlimited): do not change defaults
here.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::graph_engine_inert_settings` (about line 793): lists
  both keys with reason `NOT_ENFORCED` (lines 879-887). Remove an entry once the key is enforced.
- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::admit_task_budget` (about line
  1400): per-task admission. It is called from `TaskDispatcher::dispatch` (about line 3193), right after
  the plan reservation `self.budget_ledger.reserve(&spec.plan_id, self.budget_policy)` (line 3184). The
  daily check belongs next to it.
- `GraphPlanBudgetLedger` / `GraphPlanBudgetPolicy` (lines 332-590, same file): in-process spend per
  plan in micro-USD; `policy.continue_on_exhaustion` is true for `--budget-override` and `--no-budget`.
- `GraphTaskDispatcher::dispatch_streaming` (about line 4127): reserves plan budget but calls no task
  admission. It has no production caller at HEAD (only tests), so it can share the new helper or be
  left alone.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::resolve_budget_ceiling` (line 510) and the call
  at line 848: where CLI flags and config become the plan budget policy. Entry point:
  `roko plan run <dir>` -> `commands/plan.rs` -> `run_graph_plan`.
- `crates/roko-learn/src/costs_log.rs::CostsLog::cost_today` (line 227): today's UTC spend from
  `.roko/learn/costs.jsonl` (the Graph dispatcher appends one `CostRecord` per dispatch at
  `graph_task_dispatch.rs` about line 1660, asynchronously via `tokio::spawn`).
- `crates/roko-core/src/config/budget.rs::BudgetConfig`: field docs promise "new dispatches are
  blocked" (daily) and "an `AgentBudgetExhausted` event is emitted and the agent is drained" (lifetime).
  Neither exists on any path.

## Current state

- Per-task caps are done: `task_budget_ceiling_usd` (line 605) combines `budget.max_task_usd` x tier
  multiplier with `budget.max_task_retry_usd`, enforced by `GraphTaskSpendLedger::admit`, committed in
  `725f21e05`. That was half of the original scope.
- `max_daily_usd`: no reader on the Graph path. Runner-v2 leftovers (`runner/types.rs:2404`
  `RunConfig::max_daily_usd`, `runner/state.rs:239-243` `daily_budget_exhausted` /
  `prior_daily_cost_usd`) are populated but read by nothing.
- `max_agent_lifetime_usd`: no reader anywhere. Graph runs have no long-lived agent: each task attempt
  is a fresh provider session, so "agent lifetime" spend equals attempt spend, already bounded by the
  per-task caps. The only lifetime tracker, `roko_agent::lifecycle::BudgetTracker` (`max_total_usd`),
  is constructed only in tests.
- `roko run` (`commands/util.rs::cmd_run`, about line 389) uses `max_plan_usd` as a *daily* guard via
  `CostsLog::cost_today`, and ignores `max_daily_usd` too.
- This repo's `roko.toml` sets both keys to `0.0`, so no warning fires here.

## Plan

1. Daily cap. In `run_graph_plan` (`plan_runner.rs`), next to `.with_plan_budget(..)` (about line
   1142), read today's prior spend once (`CostsLog::at(<workdir>/.roko/learn/costs.jsonl)
   .cost_today()`; a missing file means 0) and pass it with the ceiling through a new builder such as
   `with_daily_budget(max_daily_usd, prior_today_usd)`. Store both on `GraphTaskDispatcher` with the
   UTC date the prior spend was read for. One dispatcher (`Arc`, line 1173) serves every plan in the
   set.
2. Add `admit_daily_budget(&self, spec)` next to `admit_task_budget` and call it in `dispatch` right
   after `admit_task_budget`. Daily spend = prior spend + the spend this process recorded since that
   read (add a counter where `self.task_spend.record(..)` runs, about line 3785) + in-flight
   `reserved_micro_usd` across all plans. Do not use `budget_ledger`'s `spent_micro_usd`: on resume it is
   restored from the checkpoint and is already in `costs.jsonl`, so it would double-count. At or over
   `max_daily_usd`: return
   `RokoError::BudgetExceeded { dimension: "daily_cost_micro_usd", .. }`. Mirror the existing policy:
   `--no-budget` skips the check, `--budget-override` warns and continues. On a UTC date change,
   re-read the log instead of carrying yesterday's total.
3. Design choice, prior spend source:
   - (a) read the log once per run plus in-process ledger (recommended): cheap, deterministic, same as
     Runner-v2's `prior_daily_cost_usd`. Misses spend by other concurrent roko processes.
   - (b) re-read `costs.jsonl` before every dispatch: sees other processes, but reads the whole
     growing file each time and lags because records are appended asynchronously.
4. Lifetime cap. Recommended: stop presenting it as a plan-run control. Change its reason in
   `graph_engine_inert_settings` from `NOT_ENFORCED` to `NO_READER` (or a new reason such as "applies to
   long-running agents; plan runs are bounded by budget.max_task_usd"), and fix the doc comment in
   `BudgetConfig`. Alternative: wire it into `roko agent start` through
   `roko_agent::lifecycle::BudgetTracker::max_total_usd`. That is a separate feature; file an item.
5. Remove the `budget.max_daily_usd` entry from `graph_engine_inert_settings` and extend
   `inert_settings_list_only_changed_keys_the_graph_engine_ignores` (about line 7303) so a non-default
   `max_daily_usd` is not reported.
6. Tests in `graph_task_dispatch.rs`: `graph_daily_budget_blocks_dispatch` (prior spend at the ceiling
   -> `dispatch` returns `BudgetExceeded` before any provider call), a warn-only case with
   `continue_on_exhaustion`, and a zero-ceiling unlimited case.

## Done when

- With `max_daily_usd = 1.0` and at least $1 of today's records in `.roko/learn/costs.jsonl`,
  `roko plan run` refuses the first dispatch with a budget error naming `budget.max_daily_usd`.
- `--no-budget` disables the check; `--budget-override` logs a warning and continues.
- Neither key is reported as `not enforced by the Graph engine` by `roko config doctor`.
- Verify: `grep -rqw 'fn graph_daily_budget_blocks_dispatch' crates/roko-cli/src/ && cargo test -p roko-cli graph_daily_budget_blocks_dispatch && ! grep -A1 -E '"budget\.max_(daily|agent_lifetime)_usd"' crates/roko-cli/src/graph_task_dispatch.rs | grep -q NOT_ENFORCED`

## Notes

- Money-control path: fail closed on a malformed ceiling (negative, NaN, infinite), treat `0.0` as
  unlimited (the documented contract), and treat an unreadable costs log as 0 with a warning, not as a
  hard error.
- Do not change default values (parked decisions `q-778b4f`, `q-e23804`).
- Touches `graph_task_dispatch.rs`, a large file that many items edit. Coordinate with other items
  anchored there. Otherwise independent of other work.
- The dispatcher is shared by all plans of a plan set, so the daily sum must cover every plan in the
  ledger, not only `spec.plan_id`.
- Out of scope, worth its own item: `roko run` treats `max_plan_usd` as a daily ceiling
  (`commands/util.rs::cmd_run`); it should probably read `max_daily_usd` instead.

## Original notes

budget.max_task_usd, max_task_retry_usd, max_daily_usd and max_agent_lifetime_usd are read only by Runner-v2, so `plan run` has no per-task or daily spend protection.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-20: Budget Enforcement Disabled by Default`

How to verify: grep budget config reads under graph_execution/.

Verified 2026-09-28: narrowed - per-task caps are now enforced on the Graph path in the working tree (uncommitted; graph_task_dispatch.rs:1074-1306, budget.max_task_usd x tier multiplier + budget.max_task_retry_usd), but budget.max_daily_usd and budget.max_agent_lifetime_usd are still flagged NOT_ENFORCED by the Graph config audit (graph_task_dispatch.rs:790-799).

Re-verified 2026-09-29 at d9e79e9d8: the per-task caps (budget.max_task_usd x tier multiplier and budget.max_task_retry_usd, task_budget_ceiling_usd) are committed in 725f21e05. budget.max_daily_usd and budget.max_agent_lifetime_usd are still reported NOT_ENFORCED by graph_engine_inert_settings and have no reader on the Graph path.
