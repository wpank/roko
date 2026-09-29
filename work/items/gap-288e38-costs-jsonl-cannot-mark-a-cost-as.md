+++
id = "gap-288e38"
kind = "gap"
title = "costs.jsonl cannot mark a cost as estimated: CostRecord has no usage-source field"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-learn/costs_db", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-learn/src/costs_db.rs::CostRecord", "crates/roko-learn/src/costs_db.rs::create_cost_record", "crates/roko-cli/src/graph_task_dispatch.rs:1913", "crates/roko-core/src/usage.rs::UsageSource", "crates/roko-cli/src/commands/diagnose.rs"]
links = { depends_on = [], blocks = [], related = ["bug-690dc6", "q-1faa0c", "bug-2b1ddc", "bug-dc4d63", "spec-b7303f", "gap-528762"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^pub struct CostRecord {/,/^}/p' crates/roko-learn/src/costs_db.rs | grep -qE 'usage_source|estimated' && grep -rqw 'fn a_timed_out_attempt_cost_record_is_marked_estimated' crates/roko-cli/src && cargo test -p roko-cli --lib a_timed_out_attempt_cost_record_is_marked_estimated"
+++

## Problem

Since `e0673e3e0`, a Claude CLI attempt killed at its timeout reports the usage it streamed as
`UsageSource::Estimated`. The usage comes from summing the `assistant` events and pricing them from the model registry;
it is not the provider's final bill. Graph dispatch writes that cost to `.roko/learn/costs.jsonl` as a
`roko_learn::costs_db::CostRecord`. The record has no field for the usage source, so an estimated partial cost reads
exactly like a provider-reported one. `AgentResult.usage_obs.source` carries the source, but Graph dispatch never reads
it: `graph_task_dispatch.rs` has no reference to `usage_obs` or `UsageSource`.

## Why it matters

Goal `core`, and the whitepaper's honest numbers. Readers of `costs.jsonl` cannot tell measured spend from estimates:

- `roko status` (`CostsLog::total_cost`);
- `roko diagnose` attempt lists (05f8854ce);
- the serve cost projection;
- benchmark and paper tables.

More estimated rows are coming: `bug-2b1ddc` (interrupted attempts) and `bug-dc4d63` (Codex/Gemini). `bug-690dc6` and
`q-1faa0c` asked for such costs to be "marked as partial rather than zero". The agent result is marked now; the
durable record is not.

## Where

- `crates/roko-learn/src/costs_db.rs::CostRecord` (:26-55): 14 fields, no source.
  `create_cost_record` (:683) builds one from a `Usage` (which has no source either).
- `crates/roko-cli/src/graph_task_dispatch.rs` (:1912-1927): the Graph `CostRecord` built in `emit_feedback`.
- Other constructors: `crates/roko-cli/src/graph_execution/feedback.rs:149`,
  `crates/roko-learn/src/runtime_feedback/episode_helpers.rs`.
- Readers: `crates/roko-learn/src/costs_log.rs` (`total_cost` :157), `crates/roko-cli/src/commands/diagnose.rs`,
  `crates/roko-serve/src/projection_contract.rs`.
- `crates/roko-core/src/usage.rs::UsageSource` (`ProviderReported | Estimated | Unknown`) and
  `UsageObservation.source`.

## Current state

Checked statically at `33e107da1`: the struct and every constructor lack a source field. The data is available at the
Graph write site through `dispatch.result.usage_obs`.

## Plan

1. Add `#[serde(default)] pub usage_source: roko_core::usage::UsageSource` to `CostRecord`. Old rows then read as
   `Unknown`, and `roko-learn` already depends on `roko-core`. A bare `estimated: bool` would also work, but loses
   `Unknown`.
2. Set it at the Graph write site from `dispatch.result.usage_obs` (`Unknown` when absent), and at the other
   constructors.
3. Surface it: mark estimated attempts in `roko diagnose`, and split `total_cost` into reported and estimated where
   totals are shown.
4. Test `a_timed_out_attempt_cost_record_is_marked_estimated`: extend the `STREAMS_THEN_TIMES_OUT_PROVIDER` fixture
   used by `a_timed_out_attempt_settles_the_spend_it_streamed`, and assert that the `costs.jsonl` row has
   `usage_source = "Estimated"`.

## Done when

- A timed-out attempt's `costs.jsonl` row says it is estimated.
- A normal attempt's row says `ProviderReported`.
- Old rows still parse.
- The `[[verify]]` command passes.

## Notes

- Tests construct `CostRecord` literally in several crates (`costs_log.rs`, `diagnose.rs`). They need the new field.
  The struct does not derive `Default`, so `..Default::default()` needs that derive first.
- Overlap: epic `spec-b7303f` lists "cost records have no source", and its `VerdictRecord` (`gap-528762`)
  carries a `cost_source`. No child of that epic adds the field to `costs.jsonl`'s `CostRecord`. If the settled
  attempt record replaces `costs.jsonl` as the cost truth, close this item as superseded.
