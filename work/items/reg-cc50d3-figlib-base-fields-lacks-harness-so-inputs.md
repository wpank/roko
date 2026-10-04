+++
id = "reg-cc50d3"
kind = "regression"
title = "figlib.BASE_FIELDS lacks harness, so Inputs.where() finds no report.py MetricRecord of any arm"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-17 follow-up (gap-889682's own end-to-end check)"
discovered_from = "bug-40de03"
anchors = ["benchmarks/viabilitybench/analysis/figlib.py::BASE_FIELDS", "benchmarks/viabilitybench/analysis/figlib.py::Rec"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-40de03", "gap-889682"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_p1_figures_find_a_record_whose_only_extra_clause_is_its_own_harness' benchmarks/viabilitybench/analysis/test_figures_p1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_figures_p1.py -k test_p1_figures_find_a_record_whose_only_extra_clause_is_its_own_harness -q"
+++

## Problem

`analysis/figlib.py::BASE_FIELDS` lists the record-filter fields `Inputs.where()` treats as "placing a record in
a cell," so any OTHER `==`/`in` clause on a record makes `_narrows()` reject it unless the query itself names
that field. Since bug-40de03, `metrics.arm_metrics()` (and now `econ.cc_metrics()`, gap-889682) unconditionally
adds a `harness == "<value>"` clause to every cell's cut -- `""` for an arm's own, usual harness, a real marker
only for `cheap_direct_msa`'s mini-swe-agent cell -- but `BASE_FIELDS` was never updated to include `"harness"`.
The result: **every** `vb.metric_record/1` row `report.py::build()` or `econ.cc_metrics()` produces, for every
arm, is unfindable by `Inputs.where()` unless the caller explicitly names `"harness"` in its own `clauses=`
(which no figure or table script does, since none of them know about harness-splitting at all). Confirmed
directly: building `report.build()`'s own records for a plain synthetic arm and loading them through
`figlib.load()` returns zero records for a bare `inputs.where("vs_rate", experiment=...)` call that should
return exactly one.

## Why it matters

Goal: proof (ViabilityBench analysis completeness). This silently breaks every F-series figure and T-series
table that reads a `metrics.json` `report.py` produced after bug-40de03 landed (gate 15b) -- not a crash, just
zero records found, so every panel/column quietly renders "no records" or skips. Found while verifying
gap-889682's own "Done when" (fig_f4_passk.py's panel d must render): the new `cc_<k>`/`cost_cv` records were
*also* unfindable for the same reason, which is how this was caught.

## Where

- `analysis/figlib.py::BASE_FIELDS` -- the frozenset `_narrows()` reads; needs `"harness"` added, the same way
  `"model"` already is.
- `analysis/figlib.py::Rec.model`/`Rec.series` -- `Rec` has no `.harness` property mirroring `.model`, and
  `.series` calls `metrics.cell_name(arm, model)` with only two of `cell_name`'s three arguments (bug-40de03
  added a third, `harness`), so a harness-split cell's series name never shows its harness suffix even once
  records of it are findable.

## Current state

Fixed on `work/gap-9f9c03` in the same wave that found it (gap-889682's own package), since leaving it broken
made that item's own "Done when" unverifiable (nothing report.py or econ.py produces would ever be found by a
figure script, cc_<k>/cost_cv included). `BASE_FIELDS` gained `"harness"`; `Rec` gained a `.harness` property
(`self.eq("harness", "")`, default `""` matching `harness_of`'s own convention, never `None`); `Rec.series` now
passes it to `metrics.cell_name`. A regression test
(`test_p1_figures_find_a_record_whose_only_extra_clause_is_its_own_harness`, `analysis/test_figures_p1.py`)
builds a synthetic record with exactly a `harness == ""` extra clause and confirms `Inputs.where()` finds it;
confirmed it fails without the `BASE_FIELDS` fix and passes with it. Full `analysis/test_figures_p1.py` and
`test_figures_p2.py` suites still pass (their own fixtures happened not to exercise this path, which is why it
went unnoticed since bug-40de03 merged).

## Plan

Already done (see Current state); nothing further planned here.

## Done when

- `Inputs.where()` finds a `report.py`- or `econ.py`-produced MetricRecord of a plain (non-harness-split) arm
  without the query naming `"harness"` itself.
- The `[[verify]]` command passes.

## Notes

- Related: bug-40de03 (done; introduced the unconditional harness clause this regression is in), gap-889682
  (the package this was found and fixed alongside).
- Not independently re-verified against a *real* bench run's `metrics.json` (no live run available this wave);
  confirmed only via `report.build()`/`econ.cc_metrics()` on synthetic records plus `figlib.load()`, which is
  the same mechanism every figure/table script's own tests use.
