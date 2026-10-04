+++
id = "gap-546e8a"
kind = "gap"
title = "learn/costs.jsonl and efficiency.jsonl writers don't emit api_equiv_usd, so the historical cost fold still falls back to cost_usd"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-learn"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-14 follow-up reports 2026-10-04 (gap-e73a26, gate 15a)"
discovered_from = "gap-e73a26 (closed; own evidence names this follow-up, 'queued for the filer')"
anchors = ["crates/roko-learn/src/costs_db.rs::CostRecord", "crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn costs_and_efficiency_rows_carry_api_equiv_usd' crates/roko-learn/ && cargo test -p roko-learn costs_and_efficiency_rows_carry_api_equiv_usd"
+++

## Problem

The homeostasis cost fold now prefers `api_equiv_usd` (`gap-e73a26`, closed at gate 15a:
`EfficiencyRow`/`CostRow` read it and the historical fold in
`crates/roko-learn/src/homeostasis/resolution.rs` prefers it over `cost_usd`), but today's
`learn/costs.jsonl` and `learn/efficiency.jsonl` writers never emit the field at all, so every
row the fold reads still has no `api_equiv_usd` to prefer and it silently falls back to
`cost_usd` in practice. `CostRecord` (`crates/roko-learn/src/costs_db.rs:28-56`, persisted by
`CostsLog` to `learn/costs.jsonl`) declares `cost_usd: f64` and no `api_equiv_usd` field at all.
`AgentEfficiencyEvent` (`crates/roko-learn/src/efficiency.rs:92-205`, persisted to
`learn/efficiency.jsonl` via its manual `Serialize` impl, lines 206-205+: 35 fields including
`cost_usd` and `cost_usd_without_cache`) likewise has no `api_equiv_usd` field, and its manual
`Serialize` impl's field list would need updating too (its own comment warns: "Keep this field
list in sync with the struct").

The value to carry is already available at the attempt level: `UsageObservation`
(`crates/roko-core/src/usage.rs:13-32`) has carried `api_equiv_usd: Option<f64>` since backlog
6105, and `bug-1809d7` (closed) fixed the Claude CLI agent's `priced_observation` to populate it
correctly from the configured pricing snapshot; the `openai_compat` provider path already prices
through the same `UsageObservation` shape. Both of the paths that would feed `CostRecord`/
`AgentEfficiencyEvent` already have a real `api_equiv_usd` to write — it just isn't threaded
into either persisted record.

## Why it matters

Goal: truth, M1 controller cost accuracy (the same goal `gap-e73a26` was filed under). Without
this, `gap-e73a26`'s fix is a no-op in production: the fold *can* prefer `api_equiv_usd`, but
every row it ever sees has `None` there (since `#[serde(default)]`/absence on deserialize), so
it always falls back to `cost_usd` anyway — silently under-counting the cost of any
subscription-billed chain the historical fold processes, exactly the bug `gap-e73a26` set out to
fix.

## Where

- `crates/roko-learn/src/costs_db.rs::CostRecord` (needs `api_equiv_usd: Option<f64>`).
- `crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent` (needs the same field, and its
  manual `Serialize` impl's field list updated to match).
- Construction sites for both (wherever `cost_usd` is currently set from a settled attempt's
  usage/verdict) — thread the same source's `api_equiv_usd` alongside it.
- `crates/roko-core/src/usage.rs::UsageObservation::api_equiv_usd` (the already-available
  source value; read-only reference).

## Current state

The fold-side fix (`gap-e73a26`) is merged and reads the field; nothing writes it, so the field
is always absent on every row in practice.

## Plan

1. Add `api_equiv_usd: Option<f64>` to `CostRecord` and `AgentEfficiencyEvent`, following
   `UsageObservation`'s own `#[serde(skip_serializing_if = "Option::is_none")]` convention.
2. At each struct's construction sites, populate it from the same settled attempt's
   `UsageObservation`/verdict `api_equiv_usd` that already supplies `cost_usd`.
3. Update `AgentEfficiencyEvent`'s manual `Serialize` impl to include the new field (its own
   comment already flags this as required when the struct changes).
4. Add a regression test per writer: a subscription-billed attempt's row carries a nonzero
   `api_equiv_usd` distinct from its near-zero `cost_usd`.

## Done when

- `learn/costs.jsonl` and `learn/efficiency.jsonl` rows carry `api_equiv_usd` when the
  underlying usage observation had one.
- The homeostasis historical fold (`gap-e73a26`) actually prefers a populated value in
  production, not just in its own unit tests.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-e73a26, gate 15a): `gap-e73a26`'s own closing evidence
  names this exact follow-up explicitly: "the costs and efficiency writers don't emit it yet
  (queued for the filer)." Confirmed at main HEAD `deba5c6f8` by reading both structs and
  `UsageObservation`/`bug-1809d7`'s fix.
