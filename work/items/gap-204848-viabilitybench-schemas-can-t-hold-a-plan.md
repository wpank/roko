+++
id = "gap-204848"
kind = "gap"
title = "ViabilityBench schemas can't hold a plan-slice run record without a placeholder ladder, or a PL task"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/schema"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "7165b1c08"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-bench-slice's report on gap-89f393)"
anchors = ["benchmarks/viabilitybench/schema/run-record.schema.json", "benchmarks/viabilitybench/schema/task.schema.json", "benchmarks/viabilitybench/families/plan_slice/feature.schema.json", "benchmarks/viabilitybench/families/plan_slice/slicekit.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-89f393", "gap-1cd676", "gap-3c430e", "gap-455aef"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_plan_slice_row_validates_without_a_placeholder_ladder' benchmarks/viabilitybench/schema/test_schemas.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/schema/test_schemas.py -k test_a_plan_slice_row_validates_without_a_placeholder_ladder -q"

[closed]
at = 2026-09-29
commit = "7165b1c08"
evidence = "7165b1c08: run-record task.ladder allows null (PL rows, no level); vb.feature/1 moved to schema/feature.schema.json with examples/feature.json and a validate.py 'feature' kind; the plan-slice fixtures drop the placeholder ladder 5 (slicekit sets null and rejects a [run_record] ladder). Verify passes: schema/test_schemas.py::test_a_plan_slice_row_validates_without_a_placeholder_ladder (every fixture's and the example's PL row validates for roko_plan and fd_claude with ladder null; 0, 6 and 'plan' fail). pytest benchmarks/viabilitybench: 225 passed, 2 opt-in skips."
+++

## Problem

- `vb.run_record/1` requires `task.ladder`, a closed enum of 1–5. A plan-slice feature has no difficulty level, so gap-89f393 puts `ladder = 5` in every PL row as a documented placeholder (`families/plan_slice/README.md`, "Run records"), and every ℓ analysis has to remember to filter PL rows out.
- `vb.task/1` can't describe a PL instance: `family` allows only F1–F8, and required fields such as `visible_verify`, `planted_gaming`, `recoverability` and `truth_suite` belong to generated families. The slice keeps its own `vb.feature_source/1` schema in `families/plan_slice/feature.schema.json`, outside `schema/`.

## Why it matters

The pilot benchmark (epic spec-567e52) reports from these records. A placeholder that looks like a real level can leak into an ℓ-level analysis, and a schema kept outside `schema/` misses the shared validation and examples.

## Where

The anchors, plus `schema/validate.py` and `schema/test_schemas.py`, which validate the schemas and their examples.

## Current state

At BASE, `run-record.schema.json` has `task.ladder = {"enum": [1, 2, 3, 4, 5]}`, and `task.schema.json` has `family = {"enum": ["F1", …, "F8"]}`. gap-1cd676 carries a note about the ladder field from gap-3c430e.

## Plan

1. Allow `null` in `task.ladder`, and say in its description that `null` means "no ladder level" (PL rows). Decide whether that is a compatible change within `/1` or needs `/2`.
2. Move `feature.schema.json` into `schema/` as `vb.feature_source/1`, with an example, and have `slicekit.py` load it from there.
3. Either give `vb.task/1` a PL variant, or document that PL instances use `vb.feature_source/1` instead.
4. Drop the placeholder from the slice's `[run_record]` tables and README.
5. Add `test_a_plan_slice_row_validates_without_a_placeholder_ladder`.

## Done when

- [ ] A PL run record validates with `ladder = null`, and no fixture carries the placeholder 5.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-29 (wk-bench-slice): the schema that moved to `schema/feature.schema.json` is `vb.feature/1`, the
  per-instance manifest and the PL counterpart of `vb.task/1`. It is not `vb.feature_source/1`, the `feature.toml`
  authoring format, which stays with the family (`slicekit.py check` validates it, as `common/knobs.py` does for
  `vb.ladder/1`).
- Plan step 3: PL instances use `vb.feature/1` in place of `vb.task/1`. The two READMEs and the schema description
  say so, and `validate.py` gained the kind `feature`.
- Plan step 1: `vb.run_record/1` stays `/1`. Allowing `null` only widens what validates, and no record was written
  before the change. The driver builds `task.ladder` from `vb.task/1` manifests, which still require a level.
