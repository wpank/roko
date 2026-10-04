+++
id = "gap-76ea03"
kind = "gap"
title = "Run manifest doesn't record the price snapshot id, though decision 2113 says it should"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "c71eabdb0"
source = "wave-5 follow-up reports 2026-10-02 (PK13 gap-9e3134)"
discovered_from = "gap-9e3134 (decision 2113)"
anchors = ["crates/roko-cli/src/graph_execution/run_manifest.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_manifest_records_the_price_snapshot_id' crates/roko-cli/ && cargo test -p roko-cli run_manifest_records_the_price_snapshot_id"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T06:53:21Z"
commit = "c71eabdb0"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T04:53:02Z"
forced = false
evidence = "Gate 14b (merged c71eabdb0): verify run_manifest_records_the_price_snapshot_id passes. RunManifests resolves the active price snapshot (PriceSnapshot::shared) and writes prices.snapshot_id at open (decision 2113); gates.py reads only ablation_flags. Gate fix 5a2c4a4f0 (read the snapshot from RokoConfig before the name is shadowed)."
+++

## Problem

Decision 2113 (`tmp/backlog/2026-10-02-complete-and-wire/2113-decide-pricing-rules-for-the-attempt-ledger.md`,
decided 2026-10-02) recommends option (b) for where the price snapshot id a run used is recorded, with the note
"the run manifest records the id" (line 52). `crates/roko-cli/src/graph_execution/run_manifest.rs` has no field
or write for a price-snapshot id today (checked: no match for `price_snapshot`, `snapshot_id`, or
`pricing_snapshot` anywhere in that file).

## Why it matters

Without the snapshot id on the manifest, nobody can later tell which dated price table (`config/prices/*.toml`,
`crates/roko-core/src/pricing_snapshot.rs`) priced a given run's costs — a prerequisite for re-deriving or
auditing any $-denominated claim about that run after the built-in snapshot changes (gap-ceffb3, open, covers the
snapshot itself having no current entry for newer models; this item is about *recording which one was used*,
a separate, decided requirement).

## Where

- `crates/roko-cli/src/graph_execution/run_manifest.rs` (or `plan_runner.rs::close_run_manifest`, which calls it)
  — where the manifest's fields are assembled and written; needs a new field.
- `crates/roko-core/src/pricing_snapshot.rs` — where the resolved snapshot id is available at runtime (the
  "built-in id" referenced by gap-ceffb3's `pricing_snapshot_builtin_copy_is_the_newest_file` test).
- `tmp/backlog/2026-10-02-complete-and-wire/2113-decide-pricing-rules-for-the-attempt-ledger.md` — the decision.

## Current state

Decided but not implemented. The manifest closes with no price-snapshot provenance.

## Plan

1. Add a `price_snapshot_id` (or similarly named) field to the run manifest's schema.
2. Resolve the snapshot id actually in effect for the run (from `pricing_snapshot.rs`'s resolution logic) and
   write it when the manifest closes (`close_run_manifest`).
3. Test: a run closes its manifest with the snapshot id that was active when it ran, independent of which
   snapshot is newest at read time.

## Done when

- The run manifest records the price-snapshot id that priced the run's costs.
- The `[[verify]]` command passes.

## Notes

- Depends conceptually on gap-ceffb3 only in that both touch pricing-snapshot provenance; they are independent
  fixes (one adds a missing dated snapshot, this one records which snapshot a run used) and can land in either
  order.

## Progress

- Implemented at 61af488ed. `RunProvenanceManifest` already had a `prices: PriceProvenance` field
  (`roko-learn/src/telemetry/records.rs`), but nothing ever wrote it. `RunManifests::capture` now
  resolves the snapshot id with `PriceSnapshot::shared(&config.pricing, workdir)` (the same
  resolution bug-1809d7 fixed for the Claude CLI agent); `RunManifests::open` writes it into
  `manifest.prices.snapshot_id` alongside the existing `ablation_flags` write, so it is captured
  once per invocation, independent of whatever is newest when the manifest is read back later.
  New test `run_manifest_records_the_price_snapshot_id`: two snapshot files in a temp workspace,
  the older one pinned; confirms the manifest records that id, not the newer file's.
- Checked whether the bench driver's G-checks want the field: `benchmarks/viabilitybench/
  analysis/gates.py`'s only S01-manifest read is `_frozen_loops`/`_ablation_flags`
  (`experiment.ablation_flags`); it does not read `prices`/`snapshot_id`, so no change there is
  needed for this item. Verify's static grep passes; `cargo test` deferred to the coordinator's
  gate.
