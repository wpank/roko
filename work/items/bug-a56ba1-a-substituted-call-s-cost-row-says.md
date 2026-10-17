+++
id = "bug-a56ba1"
kind = "bug"
title = "A substituted call's cost row says priced when the model that served it has no rate, so later runs count its unknown cost as $0"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-1bc222 worker, 2026-10-16: found while threading priced into the spend ceilings"
discovered_from = "bug-1bc222"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback", "crates/roko-cli/src/graph_task_dispatch/served_model.rs::dispatch_priced"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-1bc222", "bug-c67afc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_substitute_with_no_rate_writes_an_unpriced_cost_row' crates/roko-cli/ && cargo test -p roko-cli a_substitute_with_no_rate_writes_an_unpriced_cost_row"
+++

## Problem

When an API provider serves another model than the one launched (bug-31438d), `price_by_served_model`
(`crates/roko-cli/src/graph_task_dispatch/served_model.rs`) prices the call at the served model's rates. With no rate
for that model, the call's cost is unknown and stays $0 ("With no price for it the cost is unknown (0), not the
launched model's"). The attempt's cost row, written in `emit_feedback`
(`crates/roko-cli/src/graph_task_dispatch/feedback.rs`, the W05b `CostRecord`), still computes `priced` from the
launched model:

```rust
priced: Some(crate::dispatch_v2::usage_is_priced(
    &dispatch.result.usage,
    self.pricing_snapshot().as_deref(),
    dispatch.target.model_profile.as_ref(),
    &dispatch.target.model_slug,
)),
```

So when the launched model has a rate, the row says `priced: true, cost_usd: 0`, which reads as a free call.

Expected: `priced: false` for a call the served model's missing rate left unpriced. A substitute at a rate of 0 stays
`priced: true`.

## Why it matters

Goal truth. `roko_learn::costs_log::is_unpriced` trusts `priced`, so `roko show costs` counts the call as $0. So does
the daily spend ceiling of every later process, which reads its baseline from `.roko/learn/costs.jsonl`
(`GraphDailyBudget`, `budget.max_daily_usd`). A money control then fails open. Since bug-1bc222, the process that made
the call judges it by the served model (`GraphTaskDispatcher::dispatch_priced`) and stops dispatching. Its own cost
row disagrees.

## Where

- `crates/roko-cli/src/graph_task_dispatch/feedback.rs::emit_feedback`: the attempt cost row's `priced`.
- `crates/roko-cli/src/graph_task_dispatch/served_model.rs::dispatch_priced`: the judgement the spend ceilings use
  since bug-1bc222: the served model on an API provider that substituted one, else the launched model.
- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs` (`SideCall::of`): the rows of a call failover refused
  are also judged by the launched model. Check whether they should follow.

## Current state

bug-1bc222 changed only the spend ceilings: the day's and the plan's unpriced-call counts take `dispatch_priced`.
The cost row keeps the launched model's judgement from backlog 2109. Test
`spend_ceilings_judge_a_substitute_by_the_model_that_served` (served_model.rs) covers the ceiling side.

## Plan

1. In `emit_feedback`, set the cost row's `priced` from `self.dispatch_priced(dispatch)`. Then a call's row and its
   process's ceilings agree by construction.
2. Decide the same for `SideCall::of` (refused failover calls): reuse the judgement, or note why not.
3. Add test `a_substitute_with_no_rate_writes_an_unpriced_cost_row`, next to
   `records_carry_the_provider_reported_model` (served_model.rs tests). Use the OpenAI-compatible mock with real token
   usage, answering as another model than the priced `gpt-oss-120b`:
   - answering as a model with no rate writes `priced: false`;
   - answering as a model priced at 0/0 writes `priced: true`.

## Done when

The `[[verify]]` command passes, and a substituted call's cost row agrees with the spend ceilings of the process that
made it.

## Notes

- Related: bug-1bc222 (ceilings), bug-c67afc (two other ways a cost row contradicts itself). Read bug-c67afc's plan
  before changing `usage_is_priced` itself.
- `feedback.rs` is a hot file; keep the change to the one `priced` expression.
