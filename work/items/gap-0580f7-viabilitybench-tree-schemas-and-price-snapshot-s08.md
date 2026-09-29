+++
id = "gap-0580f7"
kind = "gap"
title = "ViabilityBench tree, schemas and price snapshot (S08.T1)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/schema", "config/prices"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§5.1–5.6, §6 T1; checklist S08.T1)"
anchors = ["benchmarks/viabilitybench/schema/", "benchmarks/viabilitybench/README.md", "config/prices/2026-09-28.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["dec-b78874"], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_price_snapshot_rows_have_every_column' benchmarks/viabilitybench/schema/test_schemas.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/schema/test_schemas.py -k test_price_snapshot_rows_have_every_column -q"

[[verify]]
command = "grep -qw 'def test_spec_examples_validate_and_mutants_fail' benchmarks/viabilitybench/schema/test_schemas.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/schema/test_schemas.py -k test_spec_examples_validate_and_mutants_fail -q"
+++

## Problem

The benchmark has no tree, no result schemas and no price source. Today, costs come from two wrong places:

- **`roko.toml`'s rates.** They are wrong for gpt-oss-120b. `[models.cerebras-gptoss]` sets
  `cost_input_per_m = 0.5` and `cost_output_per_m = 1.5`; Cerebras charges $0.35 and $0.75 (S08 D9).
- **A Sonnet-rate fallback.** Unknown model slugs are priced at Sonnet's rate (S08 D6, gap-ad0d39).

Until there is one dated price snapshot, no $/VS figure can be trusted.

## Why it matters

Every later E12 item writes or reads these schemas. They enforce S08's honesty invariants (SC7): `simulated` is
always false, an unknown cost is `null` and never 0, and every MetricRecord lists its run ids. The snapshot is the
one price source for the driver, and later for S04.T01's Rust loader.

## Where

All new, and the paths follow D4 (dec-b78874):

- `benchmarks/viabilitybench/README.md`: purpose, the S08 §5.1 layout, the stdlib-only rule, results in
  `$VB_RESULTS`, and how to run the tests;
- `schema/{task,run-record,metric-record,price-snapshot,ledger}.schema.json`;
- `schema/validate.py`: a stdlib validator for the JSON Schema subset these files use (no `jsonschema`
  dependency), and `schema/test_schemas.py`;
- `config/prices/2026-09-28.toml`: id `prices-2026-09-28`, with the rows of S08 §5.6.

## Current state

Checked at `41c7ffbd6`: `benchmarks/` holds only `dev-audit/`, there is no `config/` directory, and the price rows
exist only as text in S08 §5.6.

## Plan

1. **Schemas.** Write the five schemas from the examples in S08 §5.2 and §5.4–5.6. A ledger row carries `line`,
   `attempt_key`, `reserved_usd` and `price_snapshot_id`.
2. **Validator.** Write `validate.py` to support only what the schemas use: type, required, enum, const,
   properties, items, minItems and additionalProperties.
3. **Snapshot.** Write the snapshot TOML (`roko.price_snapshot/1`).
   - Every row has every column, in S08's order, plus an optional `note`.
   - Re-check the rates on the provider pages before committing.
   - Snapshots are immutable: if a rate has changed, create a new dated file with a new id.
4. **Tests.**
   - The examples in S08 §5 validate.
   - Mutated examples fail: `simulated: true`, a cost of 0 with tokens above 0, and empty `run_ids`.
   - The snapshot loads with `tomllib`, validates, and has the full column set.

## Done when

- [ ] The five schemas, the validator and the snapshot exist, and the README describes the layout.
- [ ] Tests `test_spec_examples_validate_and_mutants_fail` and `test_price_snapshot_rows_have_every_column` pass.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **One snapshot, one file.** `config/prices/` belongs to the product: S04.T01's Rust loader will read the same
  bytes, so never write a JSON copy.
- **Not this item:** the `roko.toml` key `[pricing] snapshot` is S08.T18.
- **No provider calls.** Nothing here calls a model.
- **pytest.** It is a dev-only dependency and is not installed for the system `python3` today. Document the
  install step in the README.
- **Decided 2026-09-29 (Will):** the benchmark uses a pinned project venv. This item creates `benchmarks/viabilitybench/.venv` from a pinned requirements file with pytest in it, and every benchmark check calls `benchmarks/viabilitybench/.venv/bin/python`.
