+++
id = "gap-dd9c2e"
kind = "gap"
title = "The audit worker's sampled audits write no cost row, so their spend never shows in learn/costs.jsonl"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-16 follow-up reports 2026-10-04 (gap-73c98e, gate 16b)"
discovered_from = "gap-73c98e (closed; own closing evidence names this follow-up, 'queued for the filer')"
anchors = ["crates/roko-cli/src/audit/worker.rs::AuditWorker", "crates/roko-cli/src/graph_task_dispatch/budget.rs::record_task_spend"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn completed_audit_writes_a_cost_row' crates/roko-cli/ && cargo test -p roko-cli completed_audit_writes_a_cost_row"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:41Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T09:03:17Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes (completed_audit_writes_a_cost_row). The audit worker writes a cost and efficiency row per model call (role audit, keyed by the audited attempt, priced), on the run's audit line; task and plan spend stay 0."
+++

## Problem

`gap-73c98e` (closed at gate 16b) made inline DP3 audit checks charge the audited task's own
budget and write cost/efficiency rows (role `audit`), through `record_task_spend`
(`crates/roko-cli/src/graph_task_dispatch/verify_depth.rs`/`budget.rs`). The audit *worker*'s
sampled audits (`crates/roko-cli/src/audit/worker.rs`, the standalone process that draws green
units into phase-B hidden-test review) never went through that fix and still writes no cost
row at all. Its spend is tracked purely in memory, against a self-imposed cap, not a task's
budget: `self.spent_usd += audit.cost_usd` (line 729), bounded by `config.budget_frac *
*self.context.run_spend.lock()` (lines 504-511) — "What the next audit may spend: `budget_frac`
of the run's model spend" (the module's own doc comment, line 19). Confirmed: zero matches for
`CostRecord`, `costs.jsonl`, `CostsLog`, `append_cost` or `record_task_spend` anywhere in
`worker.rs`. The worker's audits burn real model calls (`CheckOutcome`/`audit.cost_usd` sum
each audit's calls, lines 185-198) but never write them to `learn/costs.jsonl` or
`learn/efficiency.jsonl`.

## Why it matters

Goal: truth, same goal as `gap-73c98e`. The whole point of that fix was that audit spend is
real spend that should be visible in the primary cost-accounting log, not just tracked
internally — and it fixed exactly that for inline checks. The audit worker's sampled audits are
the *other* audit spend path (and arguably the larger one: phase-B hidden-test review against a
budget_frac of the whole run, not a single task's budget), and it has the identical blind spot
`gap-73c98e` just closed for inline checks. Any dashboard, report, or the homeostasis cost fold
reading `learn/costs.jsonl` under-counts a run's total spend by however much the audit worker
spent — invisibly, with no row to even notice is missing.

## Where

- `crates/roko-cli/src/audit/worker.rs` (`AuditWorker`, `spent_usd`, `audit.cost_usd`,
  `CheckOutcome` — the spend that needs a cost row).
- `crates/roko-cli/src/graph_task_dispatch/verify_depth.rs`/`budget.rs::record_task_spend`
  (the sibling mechanism `gap-73c98e` wired for inline checks; the pattern to follow, though the
  audit worker's spend isn't attributable to one task's budget the way inline checks are).

## Current state

The worker tracks and caps its own spend correctly against `budget_frac`, but nothing persists
that spend anywhere `learn/costs.jsonl`'s readers would see it.

## Plan

1. After each completed audit, write a cost row (and an efficiency row, matching
   `gap-73c98e`'s shape) for its model calls' cost, with a role distinguishing it from inline
   checks (e.g. `"audit_worker"` or similar, since it's not attributable to the audited task's
   own budget the way an inline check is — it's charged against the run's audit line instead).
2. Decide what `plan_id`/`task_id` such a row should carry, given the spend isn't really the
   audited task's own (perhaps the run id and the audited unit's `sel_id`/`attempt_key` instead
   of a task budget charge).
3. Regression test: a completed audit with nonzero `cost_usd` produces a cost row in
   `learn/costs.jsonl`.

## Done when

- The audit worker's sampled-audit spend appears in `learn/costs.jsonl`.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-16 follow-up, gap-73c98e, gate 16b): confirmed at main HEAD `70fc09313`.
  `gap-73c98e`'s own closing evidence names this exact follow-up: "the worker's sampled audits
  stay on the audit line (queued for the filer)."

## Progress

- gap-dd9c2e: implemented at 5cae4abb0. A worker takes a `CallLog` (`audit/worker.rs`). After each phase-B check it hands over the check's model calls, and the dispatcher's log (`GraphTaskDispatcher::audit_call_log`, `graph_task_dispatch/helper_calls.rs`) writes a cost row and an efficiency row for each one, as gap-73c98e does for inline checks: role `audit`, keyed by the audited attempt, with the model's provider from `[models]`, and `api_equiv_usd` and `price_snapshot_id` from the run's price snapshot. Plan 2's choice: the rows carry the audited task's plan, task and tier (`AuditTask` gains `tier`), and the efficiency row's `attempt_id` is `<attempt_key>/<sel_id>-<check>-<n>`, so it is not mistaken for an inline check's `audit-<check>-<n>`. Nothing charges a task's budget or the plan's: the worker still spends on the run's audit line. To share the row writing, `write_side_call_rows`'s body moved to `SideCallRows::write`, `SideCall::of_check` now looks the model up in `[models]` itself, and `AUDIT_ROLE` moved next to `HELPER_ROLE`. The verify's grep passes; cargo verification is deferred to the batch gate (`completed_audit_writes_a_cost_row`).
