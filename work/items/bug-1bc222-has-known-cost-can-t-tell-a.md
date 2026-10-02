+++
id = "bug-1bc222"
kind = "bug"
title = "has_known_cost can't tell a free-rate model from an unpriced one, so it wrongly trips the daily (and soon plan) spend ceiling"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-core/chat-types"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK07 gap-f548c1)"
discovered_from = "gap-f548c1 (backlog task 2111, implemented at 4add4f8b0 on work/gap-f548c1; reported directly by its executor, w3-pk07)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/budget.rs::record_task_spend", "crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphTaskSpendLedger", "crates/roko-core/src/chat_types.rs::has_known_cost"]
lane = "rust-hot"
links = { depends_on = ["gap-f548c1"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn unpriced_ceiling_ignores_a_genuinely_free_rate' crates/roko-cli/ && cargo test -p roko-cli unpriced_ceiling_ignores_a_genuinely_free_rate"
+++

## Problem

`roko_core::Usage::has_known_cost()` (`crates/roko-core/src/chat_types.rs:187`) cannot tell a call that is
genuinely free (a model with a real, configured rate of $0/$0) from one whose price is simply unknown:

```rust
pub fn has_known_cost(&self) -> bool {
    self.cost_usd.abs() > f32::EPSILON
        || (self.total_tokens() == 0 && self.cache_read_tokens == 0)
}
```

If a call consumed real tokens (`total_tokens() != 0`) and its reported `cost_usd` is `0.0` — true both for "roko
has no rate for this model" and for "roko correctly priced this model at $0" — `has_known_cost()` returns `false`
either way. The function's own doc comment and its test `has_known_cost_false_when_tokens_but_zero_cost`
(chat_types.rs:571) name this as the intended behaviour for the *unknown*-price case, but nothing distinguishes it
from the *known-zero*-price case.

Today on `main`, this already mis-fires the **daily** spend ceiling: `GraphTaskSpendLedger::record`
(`crates/roko-cli/src/graph_task_dispatch/budget.rs:443-448`) increments its process-wide `unpriced_calls` counter
whenever `!usage.has_known_cost()`, with no way to know the model is actually priced at zero. That counter feeds
`ProcessSpend` (`process_spend()`, budget.rs) and `admit_daily_budget`'s `daily_stop()` check refuses the next
dispatch once any unpriced call is seen under a configured `max_daily_usd`.

Backlog task 2111 (`gap-f548c1`, implemented on the unmerged branch `work/gap-f548c1` at commit `4add4f8b0`) adds a
second, **plan-level** instance of the same flaw: a new `GraphPlanBudgetLedger::record_unpriced(plan_id)`, called
directly from `record_task_spend` on the same `!usage.has_known_cost()` test, with no plan-ceiling path yet
existing on `main` to compare against. So once that branch merges, a $0/$0-priced model will trip *both* the daily
ceiling (already true today) and the new plan ceiling (true from the moment 2111 lands).

The same backlog wave's task 2109 already added the correct check for this, just not here:
`crate::dispatch_v2::usage_is_priced(usage, profile, model_slug)` (on `work/gap-f548c1`,
`crates/roko-cli/src/dispatch_v2.rs:2294`) is `usage.has_known_cost() || model_has_price(profile, model_slug)` —
it also asks whether roko has *any* configured rate for the model (even $0 counts, per `model_has_price`'s doc
comment: "A rate of 0 is free, and still a rate"). `usage_is_priced` is already used at the cost-*row* call sites
(`helper_calls.rs:98`, `feedback.rs:471`, `plan_authoring.rs:602`, each with a `profile`/`model_slug` in scope) but
was deliberately **not** threaded into `record_task_spend`'s or `GraphTaskSpendLedger::record`'s ceiling checks, to
avoid touching more of the hot dispatch files than necessary (reported directly by task 2111's executor, PK07).

## Why it matters

Goal: truth (honest cost/ceiling accounting). A model roko has deliberately priced at $0 — a real, supported
configuration, not a bug — should never be refused or isolated for "unknown cost." As written, any plan or run using
such a model under a configured `budget.max_daily_usd` (and, once 2111 merges, any `max_plan_usd`) stops dispatching
new tasks after its very first call, even though the spend is fully known (zero). Related: gap-f548c1 (the package
this was found inside, task 2111 specifically), bug-39d15f (done: a different unknown-model pricing gap).

## Where

- Root check: `crates/roko-core/src/chat_types.rs::has_known_cost` (line 187) — cannot distinguish the two cases by
  design; this is correct and intentional for `has_known_cost` itself, the bug is in using it alone as a ceiling
  signal.
- Daily ceiling (affected today, on `main`): `crates/roko-cli/src/graph_task_dispatch/budget.rs::GraphTaskSpendLedger::record`
  (lines 443-448) and `record_task_spend` (line 658), which calls it.
- Plan ceiling (affected once `gap-f548c1`/task 2111 merges): the new `GraphPlanBudgetLedger::record_unpriced`,
  called from `record_task_spend` on `work/gap-f548c1`.
- The six callers of `record_task_spend` that would need a priced/model context threaded through, per that branch:
  `crates/roko-cli/src/graph_task_dispatch.rs:1562,1611`, `graph_task_dispatch/failover.rs:194`,
  `graph_task_dispatch/helper_calls.rs:297`, `graph_task_dispatch/streaming.rs:299,429` (line numbers as of
  `work/gap-f548c1`; two of these files are hot and shared with other in-flight work, so they will shift again).
- The existing, correct check to reuse: `crate::dispatch_v2::usage_is_priced` /
  `crate::dispatch_v2::model_has_price` (`crates/roko-cli/src/dispatch_v2.rs`, added by task 2109 — not yet on
  `main`, only on `work/gap-f548c1`).

## Current state

On `main` today: the daily-ceiling half is real and reproducible right now, independent of `gap-f548c1` — a call
with tokens and `cost_usd == 0.0` always increments `GraphTaskSpendLedger`'s `unpriced_calls`, regardless of whether
roko has a real $0 rate for that model.

On `work/gap-f548c1` (unmerged, live-claimed by its executor as of 2026-10-02): task 2109 added `usage_is_priced`/
`model_has_price` and wired them into cost-row `priced` flags; task 2111 added the plan-level unpriced-ceiling
check, reusing the same `has_known_cost()` signal rather than `usage_is_priced`, so the new plan ceiling inherits
the same blind spot by construction. Neither task threads a priced/profile signal into `record_task_spend` or
`GraphTaskSpendLedger::record`.

## Plan

1. Give `record_task_spend` (and `GraphTaskSpendLedger::record`) what they need to call `usage_is_priced` instead
   of `usage.has_known_cost()` alone: either pass `(profile: Option<&ModelProfile>, model_slug: &str)` through from
   each of the six call sites (all have a `dispatch.target` or equivalent with this in scope, per PK07), or pass a
   pre-computed `priced: bool` (mirroring `SideCall::priced`, `helper_calls.rs:52` on the branch) so the hot files
   only gain one extra argument instead of a dependency on `dispatch_v2`.
2. Update `GraphPlanBudgetLedger::record_unpriced`'s caller (once merged) and `GraphTaskSpendLedger::record` to only
   count a call as unpriced when `!priced` by this smarter rule, not `!has_known_cost()`.
3. Add a regression test: a call with real tokens, `cost_usd = 0.0`, and a model/profile with a configured $0 rate
   must NOT trip either ceiling; the same call with no configured rate at all still must.
4. Depends on `gap-f548c1` (task 2111) having merged, since the plan-ceiling half and `usage_is_priced` itself only
   exist on that branch.

## Done when

- A call whose model has a real, configured $0 rate increments neither the daily nor the plan unpriced-call count.
- A call whose model has no configured rate at all still does (the existing, correct behaviour for genuinely unknown
  cost is preserved).
- The `[[verify]]` command passes.

## Notes

- Reported directly by PK07 (w3-pk07), the executor of `gap-f548c1`/task 2111, who deliberately left this unfixed
  in 2111 to avoid widening that task's footprint in the hot dispatch files, and flagged it for separate tracking.
- Do not confuse this with bug-39d15f (done): that one was about an unknown *model* having no price row at all;
  this one is about a model that *does* have a price (zero) being misread as priced-unknown.
- The six call-site line numbers above are from `work/gap-f548c1` and will shift again once that branch (and any
  other hot-file work landing alongside it) merges; re-check them before implementing.
