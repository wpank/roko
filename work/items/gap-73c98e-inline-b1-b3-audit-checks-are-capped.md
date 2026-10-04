+++
id = "gap-73c98e"
kind = "gap"
title = "Inline B1/B3 audit checks are capped per-audit but never reach task/plan budgets or cost rows"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/audit"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-10 follow-up reports 2026-10-04 (PK64 gap-2e4a81)"
discovered_from = "gap-2e4a81"
anchors = ["crates/roko-cli/src/audit/worker.rs::CheckOutcome", "crates/roko-cli/src/graph_task_dispatch/budget.rs::record_task_spend"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn inline_audit_check_spend_reaches_a_cost_row' crates/roko-cli/ && cargo test -p roko-cli inline_audit_check_spend_reaches_a_cost_row"
+++

## Problem

The audit worker's inline B1/B3 checks (`crates/roko-cli/src/audit/worker.rs`) are capped per audit at
`[audit] per_audit_usd` (`worker.rs:478-484`), but their spend never reaches task or plan budgets, or cost rows.
`CheckOutcome` (`worker.rs:168-176`) carries only `labels` and `cost_usd: f64` — its doc comment: "What its model
calls cost, in USD." The accumulated `cost_usd` flows into the audit's own running total (`self.spent_usd +=
audit.cost_usd`, `~line 702`) and into the audit result record (`~lines 711,723`), but nowhere in `worker.rs`
calls `record_task_spend`, touches a `BudgetLedger`, or writes a cost row the way the main dispatch path's model
calls do (confirmed: zero such references anywhere in the file).

## Why it matters

Goal: cybernetic/truth, M4 deep audits (S05). Audit spend is real spend (B1/B3 make their own model calls,
capped by `per_audit_usd`) but is invisible to anything that reads task/plan budgets or cost rows — a plan's
reported cost understates its true spend by however much its audited units' B1/B3 checks cost, and nothing
enforces a plan-level ceiling against this spend at all (only the per-audit cap and whatever
`budget_frac` bounds total audit spend against the run's spend, per earlier batches' research on
`audit.budget_exhausted`).

## Where

- `crates/roko-cli/src/audit/worker.rs::CheckOutcome`, the phase-B cost accumulation (`phase_b`, ~lines 588-640),
  `self.spent_usd` (~line 702).
- The budget/cost-row infrastructure this should reach: `crates/roko-cli/src/graph_task_dispatch/budget.rs::record_task_spend`,
  the cost-row writer the main dispatch path uses.

## Current state

Confirmed: `CheckOutcome` has no path to task/plan budgets or cost rows.

## Plan

1. Thread the audit unit's `plan_id`/`task_id` (already available via `AuditUnit`/the selection record) through
   to wherever `CheckOutcome.cost_usd` is accumulated, and call `record_task_spend` (or the audit-spend
   equivalent, if task/plan budgets shouldn't directly absorb audit cost) and write a cost row for each B1/B3
   check, the same way a regular model call does.
2. Decide whether audit spend should count against the *task's* budget (the task whose attempt was audited) or
   only against the *run's* total audit-spend cap — this affects whether `record_task_spend` is the right call
   or a separate accounting path is needed.

## Done when

- A B1/B3 check's cost appears in a cost row, attributable to its audited unit's plan/task.
- The plan's reported total spend includes audit spend.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK64's work (gap-2e4a81, done).
