+++
id = "gap-cfe656"
kind = "gap"
title = "ViabilityBench plan-slice false greens count any VF=0, not specifically a failed hidden suite"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK29 gap-064c40)"
discovered_from = "gap-064c40 (reconciling paper text against S09 v1.4 surfaced this, though gap-064c40 itself only touched paper sections)"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py::plan_slice", "tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_plan_level_false_green_matches_hidden_suite_failure' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_plan_level_false_green_matches_hidden_suite_failure -q"
+++

## Problem

`analysis/metrics.py::plan_slice` computes the plan-level slice's `plan_level_false_greens` as:

```python
"plan_level_false_greens": sorted(f"{row['arm']}:{row['task']['instance_id']}" for row in kept.rows
                                  if row["arm"] in ROKO_ARMS and reported_pass(row) and not vs_minus(row)),
```

That is "the whole-plan gate passed, and VF (verified feature) is 0". But `tmp/cybernetic-harness/specs/S09-experiments.md:635`
defines the plan-level false green narrower: "features the whole-plan gate passed that **the hidden suite failed**."

Per the same spec's v1.3 note (`S09-experiments.md:25`), VF = 0 for three independent reasons, only one of which is
"the hidden (whole-feature) suite failed":

1. the whole-feature (hidden) suite fails, or
2. the base repo's pre-existing visible tests fail at the arm's final commit (added in v1.3, independent of the
   hidden suite), or
3. S05's A1 tamper-diff finds something.

So the code's `not vs_minus(row)` folds all three into one "false green" count. A feature whose hidden suite passed
but whose base-repo visible tests regressed, or whose A1 tamper check tripped, is counted as a plan-level false
green even though the hidden suite did not fail — broader than the spec's definition.

## Why it matters

Pilot benchmark (epic spec-567e52), proof goal. `plan_level_false_greens` is one of the plan-level slice's
process measures (S09 §4.9), read directly into the report `vb report` prints and the companion claims. A
broader-than-spec count overstates how often the whole-plan gate is fooled, conflating gate-fooling with two
unrelated failure modes (base regressions, tamper detection) that have their own existing fields on the row. PK29
(gap-064c40) flagged this while reconciling the paper's §5/Appendix D text against S09 v1.4: it must be fixed
before the pre-registration lock, since `plan_level_false_greens` is a number the lock freezes.

## Where

- `benchmarks/viabilitybench/analysis/metrics.py::plan_slice` (the `"plan_level_false_greens"` list comprehension,
  around line 379) and `::vs_minus` (the VF computation it reuses).
- `benchmarks/viabilitybench/analysis/metrics.py::reported_pass` (around line 165: "a final gate verdict of `passed`
  for Roko arms").
- Spec: `tmp/cybernetic-harness/specs/S09-experiments.md:613-635` (App. D.12's outcome definitions; this file is a
  tracked spec, not the excluded paper — do not touch `tmp/cybernetic-harness/paper/*` or `docs/whitepaper/*` for
  this item).

## Current state

Confirmed at HEAD by reading `metrics.py`: `plan_level_false_greens` filters on `reported_pass(row) and not
vs_minus(row)` only; there is no row-level distinction for "hidden suite specifically failed" versus the other two
VF=0 causes. The run record does carry a `visible_clean` style per-check breakdown under the row's label fields
(`completion`, `visible_clean`, `hidden`, `integrity`, per the run-record schema's label object) that a fix could
read instead of the folded VF.

## Plan

1. In `plan_slice`, replace `not vs_minus(row)` for the false-green filter with a check specifically on the row's
   hidden-suite label (the `hidden` field of the label object, or equivalent), so a feature only counts as a
   plan-level false green when the gate passed AND the hidden/whole-feature suite specifically failed — not when
   only the base-repo visible tests or the A1 tamper check tripped.
2. Keep (or add alongside) the broader VF=0-while-gate-passed count under a distinct name if it's still useful
   diagnostically, so nothing that currently reads `plan_level_false_greens` silently loses information.
3. Add a test with synthetic rows covering all three VF=0 causes, asserting only the hidden-suite-failure row is
   counted as a plan-level false green.

## Done when

- `plan_level_false_greens` counts only gate-passed rows whose hidden/whole-feature suite failed, matching
  `tmp/cybernetic-harness/specs/S09-experiments.md:635` / App. D.12.
- The `[[verify]]` command passes.

## Notes

- Do not edit `tmp/cybernetic-harness/paper/*` or `docs/whitepaper/*` — only `tmp/cybernetic-harness/specs/**` and
  `benchmarks/viabilitybench/**` are in scope for this item.
- Filed from a backlog-wave follow-up report (PK29, working on gap-064c40, which itself only reconciled paper text
  and does not touch `metrics.py`).
