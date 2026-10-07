+++
id = "gap-e73a26"
kind = "gap"
title = "homeostasis/resolution.rs's historical cost fold reads cost_usd, never the row's api_equiv_usd"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/homeostasis"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "517ab9d19"
source = "wave-5 follow-up reports 2026-10-02 (PK13 gap-9e3134)"
discovered_from = "gap-9e3134"
anchors = ["crates/roko-learn/src/homeostasis/resolution.rs::HistoricalRow"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn historical_fold_prefers_api_equiv_usd_over_cost_usd' crates/roko-learn/ && cargo test -p roko-learn historical_fold_prefers_api_equiv_usd_over_cost_usd"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T07:56:43Z"
commit = "517ab9d19"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T06:54:00Z"
forced = false
evidence = "Gate 15a (merged 517ab9d19): verify historical_fold_prefers_api_equiv_usd_over_cost_usd passes. EfficiencyRow and CostRow read api_equiv_usd and the homeostasis fold prefers it over cost_usd; the costs and efficiency writers don't emit it yet (queued for the filer)."
+++

## Problem

`crates/roko-learn/src/homeostasis/resolution.rs` has two cost-folding paths that disagree on which field to
trust:

- The **live** fold (`TaskResolution`, lines ~68-90 and ~218-256) already prefers the honest, comparable value:
  its `api_equiv_usd: Option<f64>` field sums `verdict.cost.api_equiv_usd` per attempt (line ~230-231), and only
  falls back to "unknown" when any attempt's is unknown (line 256). This path is correct.
- The **historical** fold (`EfficiencyRow`/`CostRow`/`HistoricalRow`, lines ~303-330, ~592-602) reads only
  `cost_usd` — `EfficiencyRow`/`CostRow` declare a `cost_usd: Option<f64>` field each and **no** `api_equiv_usd`
  field at all, so `HistoricalRow::from_efficiency` (line 361: `row.cost_usd.unwrap_or(0.0)`) and the `CostRow`
  read at line ~602 (`row.cost_usd.unwrap_or(0.0)`) use the raw amount even for current-format `costs.jsonl`/
  `efficiency.jsonl` rows that now also carry `api_equiv_usd` (added by the attempt-ledger pricing work) — that
  field is silently dropped by `#[serde(default)]` since the struct never declares it.

`cost_usd` is $0 (or near-$0) for subscription-billed calls (Claude Code/Codex on a subscription), while
`api_equiv_usd` is the comparable, priced-as-if-API-billed amount. So the historical fold under-counts the cost
of any chain whose attempts were subscription-billed.

## Why it matters

The historical fold feeds the same homeostat (M1 controller) decisions the live fold does, for chains folded from
pre-instrumentation history (per 8115/8106's "historical:<dir>" replay mode). If it silently reads $0-ish
`cost_usd` for subscription-billed historical attempts while the live fold correctly reads `api_equiv_usd`, the
two folds disagree on cost for the same kind of attempt depending only on which path processed it — a correctness
gap in exactly the signal the homeostat uses to judge cost-driven breaches (E2, S06).

## Where

- `crates/roko-learn/src/homeostasis/resolution.rs`:
  - `EfficiencyRow` (~line 303-315) and `CostRow` (~line 320-327): no `api_equiv_usd` field.
  - `HistoricalRow::from_efficiency` (~line 329-361): reads `row.cost_usd.unwrap_or(0.0)`.
  - The `CostRow` read around line 592-602: same pattern.
  - Contrast: `TaskResolution`'s live fold (~line 218-256), already correct.

## Current state

Live fold correct; historical fold stale. No test exercises a historical row that carries both fields and checks
which one the fold prefers.

## Plan

1. Add `api_equiv_usd: Option<f64>` to `EfficiencyRow` and `CostRow`.
2. In `HistoricalRow::from_efficiency` and the `CostRow` read path, prefer `row.api_equiv_usd` when present,
   falling back to `row.cost_usd.unwrap_or(0.0)` only when it's absent (genuinely old rows).
3. Test: a historical row with `api_equiv_usd` set to a nonzero value and `cost_usd` at `0.0` (the subscription
   case) folds to the nonzero amount, not zero; a row with only `cost_usd` (pre-field-existing) still folds
   correctly from that.

## Done when

- The historical fold prefers `api_equiv_usd` when a row has it.
- The `[[verify]]` command passes.

## Notes

- Mirror the live fold's "unknown when any attempt's is unknown" semantics (line 256) if it's cheap to do for the
  historical path too; not required to close this item, but worth doing in the same change for consistency.
