+++
id = "bug-c67afc"
kind = "bug"
title = "A Claude CLI attempt on a model the price snapshot lacks is labelled cost_source unknown while its cost row says priced = true, and a call that reported no usage counts as a known $0"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/dispatch_v2"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-23"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs::price_at_snapshot", "crates/roko-cli/src/dispatch_v2.rs::usage_is_priced", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-learn/src/costs_log.rs::is_unpriced"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ceffb3", "dec-55532d", "gap-f548c1", "gap-9e3134", "gap-546e8a", "q-3cae98"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cli_attempt_on_an_unlisted_model_keeps_its_cost_source' crates/roko-cli/ && cargo test -p roko-cli cli_attempt_on_an_unlisted_model_keeps_its_cost_source"

[[verify]]
command = "grep -rqw 'fn call_without_reported_usage_is_not_priced' crates/roko-cli/ && cargo test -p roko-cli call_without_reported_usage_is_not_priced"
+++

## Problem

Two cost records contradict themselves (status-quo assessment 2026-10-15 at 7b6a5af44, finding 23; evidence in
`tmp/status-quo/2026-10-15/backlog/A02-21-22-honest-measurement.md` section 4, points 3 and 5).

1. **A Claude CLI attempt on a model the snapshot lacks says its cost source is unknown.** The CLI agent prices its
   session at the run's snapshot itself (backlog 6105). When the session's model has no snapshot row (today
   `claude-sonnet-4-6`, the default model), `price_at_snapshot` sets `verdict.cost.source = CostSource::Unknown`
   (`attempt.rs:1333-1336`), although the tokens and the CLI's own `vendor_usd` are known and `cost_source()` had
   said `cli_usage`. The Graph cost row copies that source (`feedback.rs:532`) but computes `priced` separately with
   `usage_is_priced` (`dispatch_v2.rs:2503`), which is true because the CLI reported a dollar figure. Live:
   `docs-probe/my-project` run `7682efd5`: verdict `vendor_usd 0.0308`, `billed_usd null`, `api_equiv_usd null`,
   `source unknown`; its cost row `e772c407…:T1:1` says `cost_source unknown`, `priced true`, `cost_usd 0.0906`. The
   8 plan-authoring (strategist) rows have the same shape.
2. **A call that reported no usage is written as a known $0.** `Usage::has_known_cost`
   (`crates/roko-core/src/chat_types.rs:187`) is true when every token count is 0, and `usage_is_priced` is also true
   whenever roko has any rate for the model. So a helper call with no usage gets `priced = true, cost_usd = 0,
   cost_source = unknown` (`helper_calls.rs:122-127`, `:155`, `:491`), and `costs_log::is_unpriced` trusts `priced`
   and reads it as $0. Live: 10 `kimi-k2.5` helper rows in the e2e4 `tally` workspace (2026-10-14).

Expected: `cost_source` says where the usage came from and stays `cli_usage` when only the snapshot price is missing
(then only `api_equiv_usd`, `without_cache_usd` and `price_snapshot_id` stay null), and a row with no reported usage
says `priced = false`, so every reader counts its cost as unknown, not free.

## Why it matters

Goal `truth`. Cost per verified task, `roko show costs`, budget alerts and the M1 controller read these rows. A row
whose `cost_source` and `priced` disagree is counted differently by different readers, and a no-usage call counted as
$0 understates spend.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::price_at_snapshot`: overwrites the verdict's source.
- `crates/roko-cli/src/dispatch_v2.rs::usage_is_priced`: the `priced` flag for attempt, helper and authoring rows.
- `crates/roko-cli/src/graph_task_dispatch/feedback.rs` (attempt cost rows) and
  `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs` (helper cost rows): where the two fields are written.
- `crates/roko-learn/src/costs_log.rs::is_unpriced`: the reader that trusts `priced`.

## Current state

2109 (gap-f548c1) added `priced`; 2115 (gap-9e3134) and 6105 (gap-546e8a) added snapshot pricing. Nothing checks
that a row's two fields agree. `billed_usd` is null on every Claude CLI attempt for a separate reason: no provider
sets `billing` (`attempt.rs:1297-1306`). Will's 2026-10-15 decision 3 (dec-55532d) sets `billing = "subscription"`
for the Claude and Codex CLI providers; that is not this item. gap-ceffb3 adds the missing snapshot rows, which hides
symptom 1 for `claude-sonnet-4-6` but not for any other unlisted model.

## Plan

1. In `price_at_snapshot`, when the agent priced at the snapshot but returned no `api_equiv_usd`, leave
   `verdict.cost.source` alone; the null snapshot figures already say the API-equivalent cost is unknown (S04 §4.8).
2. Make `priced` false when the call reported no usage (no `usage_obs`, or `UsageSource::Unknown`). Do it in
   `usage_is_priced` or its callers; check `has_known_cost`'s other callers before changing that function.
3. Tests: `cli_attempt_on_an_unlisted_model_keeps_its_cost_source` (a scripted Claude CLI result priced at a snapshot
   that lacks its model: verdict source `cli_usage`, `api_equiv_usd` null, cost row `priced = true` with
   `cost_source = cli_usage`) and `call_without_reported_usage_is_not_priced` (a helper result with no usage on a
   model roko has a rate for: its row says `priced = false`).

## Done when

Both `[[verify]]` commands pass, and no cost row a Graph run writes has `cost_source = "unknown"` with
`priced = true`.

## Notes

Don't edit `config/prices/2026-09-28.toml` (snapshots are immutable). The cost-row schema doesn't change. The
`billing` setting is dec-55532d's follow-through, and the new snapshot rows are gap-ceffb3's.
