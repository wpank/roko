+++
id = "gap-b6d6cf"
kind = "gap"
title = "ViabilityBench cost classes have no audit/spec-refinement/predictor class, so roko_full's G5 $/VS-without-audit can't be computed"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/schema"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK29 gap-064c40)"
discovered_from = "gap-064c40 (reconciling paper text against S09 v1.4 surfaced this, though gap-064c40 itself only touched paper sections)"
anchors = ["benchmarks/viabilitybench/schema/validate.py::COST_CLASSES", "benchmarks/viabilitybench/schema/run-record.schema.json"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_cost_classes_include_audit_and_spec_refinement' benchmarks/viabilitybench/schema/test_schemas.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/schema/test_schemas.py -k test_cost_classes_include_audit_and_spec_refinement -q"
+++

## Problem

`benchmarks/viabilitybench/schema/validate.py:49` defines the complete set of recognized attempt cost classes:

```python
COST_CLASSES = ("plan", "execute", "retry", "escalate", "integrate")  # costs.by_class (S09 §4.9)
```

The same enum is repeated in `benchmarks/viabilitybench/schema/run-record.schema.json` (`attempts[].cost_class`
and `costs.by_class`, around lines 92 and 157). There is no `audit`, `spec-refinement` or `predictor` class, so no
attempt's spend can be attributed to any of those activities — only to planning, executing, retrying, escalating or
integrating.

## Why it matters

`roko_full` (S09's full-mechanism arm; see `tmp/cybernetic-harness/specs/S09-experiments.md` BL6 "P1 live
`roko_full`: 360 + 60, plus audit spend ≤ 12% (S05 SC6)", gated at **G5**) spends money on M4's random deep audits,
and plausibly on spec refinement and predictor-model calls, beside the base execute/retry/escalate/integrate spend
every arm has. S05 SC6 caps audit spend at 12% of the arm's total. Without a cost class to hold that spend
separately, there is no way to compute "$/VS without audit spend" (or without spec-refinement or predictor spend)
for `roko_full` at G5 — every attempt's cost is folded into the five existing classes regardless of what kind of
work produced it, so audit/refinement/predictor spend cannot be subtracted back out of the total to see the "pure
execution" cost-per-verified-success the G5 analysis needs.

## Where

- `benchmarks/viabilitybench/schema/validate.py::COST_CLASSES` (line 49) and its validation of `costs.by_class`
  (around lines 220-233: every known class must sum to `api_equiv_usd`).
- `benchmarks/viabilitybench/schema/run-record.schema.json` (`attempts[].cost_class` enum, `costs.by_class`
  properties).
- Spec: `tmp/cybernetic-harness/specs/S09-experiments.md` (BL6, G5) and `tmp/cybernetic-harness/specs/S05-deep-audits.md`
  (SC6's 12% audit-spend cap) — tracked specs, in scope; do not touch the excluded paper/whitepaper directories.

## Current state

Confirmed at HEAD: `COST_CLASSES` has exactly five members (`plan`, `execute`, `retry`, `escalate`, `integrate`).
Nothing in `validate.py` or the schema recognizes `audit`, `spec-refinement`, or `predictor`. Whatever code
attributes M4 audit attempts, spec-refinement calls, or predictor-model calls currently has nowhere to record that
distinction in a run record's `cost_class`/`by_class` fields — it must be folding that spend into one of the five
existing classes (most likely `execute` or `integrate`), losing the information needed to isolate it later.

## Plan

1. Decide the right taxonomy addition with whoever owns S05/M4 (the deep-audit spec) and S09/G5: likely add
   `audit`, `spec_refinement` and `predictor` (or fold under fewer names if some don't apply distinctly in code
   today — check what M4's audit runner and any spec-refinement/predictor call sites actually are before deciding
   exact names).
2. Extend `COST_CLASSES` in `validate.py` and the two schema enums to match.
3. Wire whatever currently classifies an attempt's `cost_class` (likely in `driver/run_roko.py` or the Rust side
   that tags attempts, per `crates/roko-serve`'s/`roko-learn`'s cost rows — check both) to use the new classes
   where applicable.
4. Add an `analysis/metrics.py` (or `tab_t9_costs.py`) computation of "$/VS without audit spend" (subtract
   `costs.by_class.audit` etc. from the numerator) for `roko_full`, gated to run at G5.
5. Add a schema test asserting the new classes validate and sum correctly.

## Done when

- `COST_CLASSES` (and the schema) include a way to separate audit, spec-refinement and predictor spend from the
  base execute/retry/escalate/integrate/plan classes.
- A "$/VS without audit spend" (or equivalent) figure can be computed for `roko_full` at G5 from recorded data.
- The `[[verify]]` command passes.

## Notes

- This is a taxonomy decision as much as a code change — don't invent class names unilaterally without checking
  what M4's audit code and any spec-refinement/predictor call sites are actually called in the Rust side first.
- Filed from a backlog-wave follow-up report (PK29, working on gap-064c40, which itself only reconciled paper text
  and does not touch the schema or drivers).
