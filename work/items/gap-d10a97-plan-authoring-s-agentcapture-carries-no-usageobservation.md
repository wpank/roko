+++
id = "gap-d10a97"
kind = "gap"
title = "Plan-authoring's AgentCapture carries no UsageObservation, so its cost rows write api_equiv_usd: None"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-learn"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-16 follow-up reports 2026-10-04 (gap-546e8a, gate 16b)"
discovered_from = "gap-546e8a (closed; fixed Graph dispatch and helper calls, not plan-authoring)"
anchors = ["crates/roko-cli/src/agent_exec.rs::AgentCapture", "crates/roko-cli/src/plan_authoring.rs::AuthoringSpend"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_authoring_cost_row_carries_api_equiv_usd' crates/roko-cli/ && cargo test -p roko-cli plan_authoring_cost_row_carries_api_equiv_usd"
+++

## Problem

`gap-546e8a` (closed at gate 16b) made Graph dispatch and helper calls fill `api_equiv_usd`/
`price_snapshot_id` on `CostRecord`/`AgentEfficiencyEvent`, but the plan-authoring path's
`AgentCapture` (`crates/roko-cli/src/agent_exec.rs::AgentCapture`) carries no `UsageObservation`
at all — its `usage` field is typed `roko_core::Usage`
(`crates/roko-core/src/chat_types.rs:128-146`), a simpler, older struct with only
`cost_usd: f32` and no `api_equiv_usd`/`price_snapshot_id` fields to carry. So plan-authoring's
own cost rows write `None` for both, by construction, and can't be priced at API rates.

`AuthoringSpend::record` (`crates/roko-cli/src/plan_authoring.rs:858-918`) builds both the
`CostRecord` and the `AgentEfficiencyEvent` from one `AgentCapture`, and its own comments
already admit the gap: "// An `AgentCapture` does not say where its usage came from.
`cost_source: roko_learn::telemetry::CostSource::Unknown`" and "// Nor does it carry the
agent's API-rate pricing. `api_equiv_usd: None, price_snapshot_id: None`" (hardcoded on both
structs). The data to fix this is available nearby but not threaded through:
`run_agent_capture_impl` (`agent_exec.rs:149-324`) already resolves a `PriceSnapshot` and calls
`crate::dispatch_v2::fill_usage_cost_from_pricing(&mut usage, snapshot.as_deref(),
resolved.profile.as_ref(), &resolved.slug)` (lines 311-316), which prices `usage.cost_usd` from
that same snapshot and reports which pricing path was used (`CallPricing`) — but the snapshot's
id and a separately-tracked API-equivalent amount (distinct from `cost_usd` only when the
call was subscription-billed) are discarded rather than carried onto `AgentCapture`.

## Why it matters

Goal: truth, same goal as `gap-546e8a`. Plan authoring (`roko run --plan`, `roko plan
generate`, research tasks per `research.rs`'s own `AgentCapture` construction) is a real,
regularly-run cost source. Without `api_equiv_usd`, the homeostasis historical cost fold
(`gap-e73a26`) under-counts plan-authoring calls the same way it under-counted Graph-dispatch
calls before `gap-546e8a` — for any plan-authoring call that happened to be subscription-billed
(cost_usd ≈ $0), its real API-equivalent cost is invisible to the fold, the M1 controller, and
anything else reading `learn/costs.jsonl`/`learn/efficiency.jsonl`.

## Where

- `crates/roko-cli/src/agent_exec.rs::AgentCapture` (needs a way to carry `api_equiv_usd`/
  `price_snapshot_id`, e.g. upgrading `usage` to `UsageObservation` or adding the two fields
  alongside it).
- `crates/roko-cli/src/agent_exec.rs::run_agent_capture_impl` (lines ~310-324; already has the
  `PriceSnapshot` and `CallPricing` in scope — just needs to carry them onto the returned
  `AgentCapture`).
- `crates/roko-cli/src/plan_authoring.rs::AuthoringSpend::record` (lines 858-918; reads
  `call.usage` into both rows; needs to read the new field(s) instead of hardcoding `None`).
- `crates/roko-cli/src/research.rs` (a second, unrelated `AgentCapture` construction site —
  check whether it needs the same treatment once the struct changes).

## Current state

Confirmed by reading `AgentCapture`, `Usage`, `run_agent_capture_impl` and
`AuthoringSpend::record` directly: both cost rows hardcode `None` for the two fields, and
the struct's own field type structurally cannot carry them today.

## Plan

1. Either change `AgentCapture.usage` from `Usage` to `UsageObservation` (which already has
   `api_equiv_usd`/`price_snapshot_id`), or add `api_equiv_usd: Option<f64>` and
   `price_snapshot_id: Option<String>` fields directly to `AgentCapture` alongside `usage`.
2. In `run_agent_capture_impl`, compute and carry the API-equivalent amount (from the same
   `PriceSnapshot`/`fill_usage_cost_from_pricing` call already in scope) and the snapshot's id
   onto the returned `AgentCapture`.
3. In `AuthoringSpend::record`, read the new field(s) into `CostRecord`/
   `AgentEfficiencyEvent` instead of hardcoding `None`.
4. Regression test: a plan-authoring call priced from a snapshot (subscription-billed,
   `cost_usd` ≈ 0) produces a cost row with a nonzero `api_equiv_usd`.

## Done when

- A plan-authoring `AgentCapture`'s cost row carries `api_equiv_usd`/`price_snapshot_id` when
  pricing data was available to compute them.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-16 follow-up, gap-546e8a, gate 16b): confirmed at main HEAD `70fc09313`.
  `gap-546e8a`'s own closing evidence covers Graph dispatch and helper calls only ("Gate fix
  f71b5de93 (inline audit-check calls priced the same way)"); plan-authoring's `AgentCapture`
  path is untouched by that fix and structurally can't carry the fields without this follow-up.

## Progress

- 2026-10-05 (w4-length): implemented on `work/gap-d10a97` at e3a657d39; cargo verification deferred to the
  batch gate.
  - `AgentCapture` gains `api_equiv_usd` and `price_snapshot_id`, and `AuthoringSpend::record` writes both into
    the cost row and the efficiency event in place of the hard-coded `None`s.
  - They are priced the way gap-546e8a prices Graph helper calls: the agent's own figure when it priced its tokens
    at the run's snapshot, else the tokens at the snapshot's row for the model. That helper moved from
    `graph_task_dispatch/helper_calls.rs` to `dispatch_v2::api_equiv`, beside the other pricing helpers, and
    helper calls now import it from there.
  - Every producer fills the fields: `run_agent_capture_impl` (plan generation, `run --plan`, research tasks),
    `plan prepare --full`'s document writer (`plan_brief.rs`) and `research::record_run_spend`.
    `record_search_spend` stays `None`, since a Perplexity search request has no tokens to price.
  - Test: `plan_authoring_cost_row_carries_api_equiv_usd`. The built-in price snapshot (2026-09-28) has no row for
    claude-sonnet-4-6, the slug the fake planner served, so the test runs the fake planner on claude-sonnet-5,
    which the snapshot prices. `fake_planner_workspace` now takes the slug.
  - Left as before: the rows' `cost_source` stays `Unknown`, because `AgentCapture` still does not say where its
    usage came from.
