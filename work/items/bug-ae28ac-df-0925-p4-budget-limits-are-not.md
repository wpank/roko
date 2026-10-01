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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "9f2da6a2b"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::admit_task_budget", "crates/roko-cli/src/graph_task_dispatch/budget.rs::task_budget_ceiling_usd", "crates/roko-cli/src/graph_execution/plan_runner.rs::resolve_budget_ceiling", "crates/roko-learn/src/costs_log.rs::CostsLog::cost_today", "crates/roko-core/src/config/budget.rs::BudgetConfig"]
links = { depends_on = [], blocks = [], related = ["gap-d31457", "q-778b4f", "q-e23804"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_daily_budget_blocks_dispatch' crates/roko-cli/src/ && cargo test -p roko-cli graph_daily_budget_blocks_dispatch && ! grep -A1 -E '\"budget\\.max_(daily|agent_lifetime)_usd\"' crates/roko-cli/src/graph_task_dispatch/inert_settings.rs | grep -q NOT_ENFORCED"
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
- 2026-10-01, wk-scheduler: implemented on `work/bug-ae28ac` at `fb667090d`. Cargo verification is deferred to the
  batch check: no clone could be taken (disk under 30 GB), so only fmt and the verify's greps ran.
  - Daily cap: plan (a). `CostsLog::spend_on` (roko-learn) reads today's spend from `.roko/learn/costs.jsonl`. It
    runs once at run start (`GraphTaskDispatcher::prime_daily_budget`, called from `run_graph_plan`), and again on
    the first dispatch after midnight UTC.
  - Today's spend is that read plus what this process has recorded since. `GraphTaskSpendLedger` now keeps a process
    total and a count of unpriced calls; every `record` site passes the call's `Usage`.
  - Once the day reaches the ceiling, `plan_dispatch_stop` stops the run from starting further tasks. `dispatch` and
    `dispatch_streaming` also refuse with `BudgetExceeded { dimension: "daily spend in micro-USD
    (budget.max_daily_usd)" }` before any provider call.
  - Overrides match the plan ceiling: `--budget-override` warns and continues, `--no-budget` skips the check, and
    `0.0` is unlimited.
  - Calls still in flight count only once they settle, not as reservations. A plan reservation can be the whole
    remaining plan budget, so counting it would refuse calls that fit. The day can therefore overshoot by the calls
    running when it reaches the ceiling.
  - Fails closed: a call that used tokens at $0 (never priced) makes the day's spend unknown, and that counts as over
    the ceiling (`RokoError::Config`). The same goes for one recorded at a negative or NaN cost, from the log or from
    this process. A negative, NaN or infinite ceiling refuses every dispatch. An unreadable log counts as $0 and logs
    a warning, per the Notes.
  - The lifetime cap is not enforced. Its inert-settings reason now says that each plan-run attempt is a fresh
    provider session bounded by `budget.max_task_usd` and `budget.max_task_retry_usd`. `BudgetConfig`'s docs no longer
    promise an `AgentBudgetExhausted` event. Neither key is reported as not enforced by the Graph engine any more.
  - The `[[verify]]` grep now reads `graph_task_dispatch/inert_settings.rs`, where `graph_engine_inert_settings` moved.
  - Tests in `graph_task_dispatch/budget.rs`: `graph_daily_budget_blocks_dispatch` (prior spend at the ceiling, so
    no task starts and dispatch is refused before the provider runs), the run's own spend, the overrides and a zero
    ceiling, unknown spend, a malformed ceiling, and a new UTC day re-reading the log. roko-learn adds
    `spend_on_counts_one_day_and_tells_unpriced_calls_apart`.
