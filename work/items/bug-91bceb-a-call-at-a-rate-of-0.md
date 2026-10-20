+++
id = "bug-91bceb"
kind = "bug"
title = "A call at a rate of 0 still reads as unpriced on its attempt verdict: billed_usd stays null and the helper totals count it as unpriced"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-1bc222 worker, 2026-10-16: found while threading priced into the spend ceilings"
discovered_from = "bug-1bc222"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs::attempt_cost", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::settle_helper_calls"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-1bc222", "bug-c67afc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_free_rate_attempt_is_billed_at_zero' crates/roko-cli/ && cargo test -p roko-cli a_free_rate_attempt_is_billed_at_zero"

[[verify]]
command = "grep -rqw 'fn free_helper_calls_are_not_unpriced' crates/roko-cli/ && cargo test -p roko-cli free_helper_calls_are_not_unpriced"
+++

## Problem

Two attempt-level reports still read a call at a rate of 0 as unpriced. Both test `roko_core::Usage::has_known_cost()`
alone, which cannot tell a call never priced from one priced at a rate of 0: both used tokens at $0 (bug-1bc222).

1. `attempt_cost` (`crates/roko-cli/src/graph_task_dispatch/attempt.rs`) fills the verdict's `billed_usd` only
   `if cost.source != CostSource::Unknown && usage.has_known_cost()`. An API attempt on a model priced at $0/$0 that
   used tokens settles with `billed_usd: null` (unknown), while its cost row says `priced: true, cost_usd: 0`.
2. `settle_helper_calls` (`crates/roko-cli/src/graph_task_dispatch/helper_calls.rs`) counts a helper call in
   `HelperCallsUsage::unpriced_calls` when `!call.usage.has_known_cost()`. That field's doc says "Calls that used
   tokens but have no price". The same call's cost row is judged by `SideCall::priced` (`usage_is_priced`). So a helper
   model priced at 0/0 shows on the verdict and the episode (`helper_unpriced_calls`) as unpriced helper calls, and
   its cost rows say priced.

Expected: a call at a rate of 0 is billed $0 and is not counted as unpriced. A call whose model has no rate stays
unknown and unpriced.

## Why it matters

Goal truth. The verdict, the episode and the cost rows contradict each other about the same call. Readers of
`billed_usd` (cost per verified task, the M1 controller's cost inputs) see an unknown cost for a free attempt. Readers
of `helper_unpriced_calls` see unknown spend that does not exist.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::attempt_cost`: the verdict's `billed_usd` for an API provider.
- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::settle_helper_calls`: the helper totals'
  `unpriced_calls`.

## Current state

The cost rows judge priced-ness with `dispatch_v2::usage_is_priced` (backlog 2109). bug-1bc222 moved the spend
ceilings to the same judgement (`GraphTaskDispatcher::dispatch_priced`, `SideCall::priced`). These two reports were
out of that item's scope and still use `has_known_cost()`. `tui/dashboard_types.rs` reads efficiency events'
`has_known_cost()` for display; check it while here.

## Plan

1. `settle_helper_calls`: count `!call.priced` instead of `!call.usage.has_known_cost()`.
2. `attempt_cost`: take the dispatch's priced judgement from the settling dispatcher
   (`GraphTaskDispatcher::dispatch_priced`), and set `billed_usd = Some(cost)` for a priced API call.
3. Tests:
   - `a_free_rate_attempt_is_billed_at_zero`: an OpenAI-compatible attempt on a model priced at 0/0 settles with
     `billed_usd == 0`; with no rate, `billed_usd` stays null.
   - `free_helper_calls_are_not_unpriced`: a helper call to a model priced at 0/0 that used tokens is not in
     `unpriced_calls`; one to a model with no rate is.

## Done when

Both `[[verify]]` commands pass.

## Notes

Related: bug-1bc222, bug-c67afc. bug-c67afc may change what `usage_is_priced` says for a call with no reported usage.
Reuse that function rather than adding another judgement.
