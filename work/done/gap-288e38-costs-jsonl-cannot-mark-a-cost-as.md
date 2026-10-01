+++
id = "gap-288e38"
kind = "gap"
title = "costs.jsonl cannot mark a cost as estimated: CostRecord has no usage-source field"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-learn/costs_db", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-learn/src/costs_db.rs::CostRecord", "crates/roko-learn/src/costs_db.rs::create_cost_record", "crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback", "crates/roko-core/src/usage.rs::UsageSource", "crates/roko-cli/src/commands/diagnose.rs"]
links = { depends_on = [], blocks = [], related = ["bug-690dc6", "q-1faa0c", "bug-2b1ddc", "bug-dc4d63", "spec-b7303f", "gap-528762"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/^pub struct CostRecord {/,/^}/p' crates/roko-learn/src/costs_db.rs | grep -qE 'pub (cost|usage)_source:' && grep -rqw 'fn a_timed_out_attempt_cost_record_is_marked_estimated' crates/roko-cli/src && cargo test -p roko-cli --lib a_timed_out_attempt_cost_record_is_marked_estimated"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:24Z"
by = "coordinator (session 7622b882)"
size = "S"
claimed_at = "2026-10-01T09:06:33Z"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including a_timed_out_attempt_cost_record_is_marked_estimated and rows_carry_their_cost_source; bench suite 373 passed (wk-tamper). Merged c9b6348d4 (work/gap-288e38 7623a1458)."
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
- A normal attempt's row says its usage was reported (`cli_usage` for a CLI agent, `provider_usage` for an API).
- Old rows still parse.
- The `[[verify]]` command passes.

## Notes

- Tests construct `CostRecord` literally in several crates (`costs_log.rs`, `diagnose.rs`). They need the new field.
  The struct does not derive `Default`, so `..Default::default()` needs that derive first.
- Overlap: epic `spec-b7303f` lists "cost records have no source", and its `VerdictRecord` (`gap-528762`)
  carries a `cost_source`. No child of that epic adds the field to `costs.jsonl`'s `CostRecord`. If the settled
  attempt record replaces `costs.jsonl` as the cost truth, close this item as superseded.

- **wk-tamper (2026-10-01):** Implemented on `work/gap-288e38`; cargo verification deferred to the batch check (static
  review and `cargo +nightly fmt --all` only). Re-checked at ea5fbe4b2: since bug-aa2044 the Graph attempt's
  `costs.jsonl` row already carried a flattened `cost_source` (`SettledCostRow`), but `CostRecord`, which every reader
  parses, had no field for it, and helper-call rows had none at all.
- The field is `cost_source: roko_learn::telemetry::CostSource` (S01 §4.4's `cost.source`, which flat records spell
  `cost_source`: `provider_usage`, `cli_usage`, `estimated`, `mock`, `unknown`), `#[serde(default)]`, so old rows read
  `unknown` and Graph rows written since bug-aa2044 read back their recorded source. `UsageSource` was not used: a
  second, PascalCase field beside `cost_source` would duplicate it. `SettledCostRow` no longer adds its own
  `cost_source`, since the record now carries it (the JSON key is unchanged).
- Set from the verdict's `cost.source` at the Graph write site, and from each helper call's `usage_obs` (with the CLI
  backend flag) for helper and refused-failover rows. `unknown` where the source is not known: plan authoring
  (`AgentCapture` has no usage source), `create_cost_record` and episode-derived rows. (The settlement-sink
  receipt's constructor went with `graph_execution/feedback.rs`, which bug-8a78e1 deleted.)
- Shown apart: `CostsLog::estimated_cost`, `roko status` (`estimated_cost_usd` in JSON, an "Estimated:" line in the
  cost summary), and `roko diagnose` attempts (`cost_source`). The bench's Roko runner prices an attempt `estimated`
  when S01's verdict meters it with `cost.source` estimated, unless the proxy metered it.
- The `[[verify]]` now greps for the field itself (`pub cost_source:`), not just the word "estimated".
