+++
id = "gap-889682"
kind = "gap"
title = "cc_<k> and per-task cost_cv MetricRecords have no producer; fig_f4 panel d and tab_t3's CC column stay empty"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK49 gap-7ec3ef)"
discovered_from = "gap-7ec3ef"
anchors = ["benchmarks/viabilitybench/analysis/fig_f4_passk.py", "benchmarks/viabilitybench/analysis/tab_t3_headline.py", "benchmarks/viabilitybench/analysis/econ.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_cc_k_metric_is_produced' benchmarks/viabilitybench/analysis/test_econ.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_econ.py -k test_cc_k_metric_is_produced -q"
+++

## Problem

`benchmarks/viabilitybench/analysis/fig_f4_passk.py` and `tab_t3_headline.py` both read `MetricRecord`s named
`cc_<k>` (`CC_k = pass^k / pass@k [per arm, pooled cell]`) and `cost_cv` (per-task coefficient of variation of
costs across seeds) — confirmed by direct quotes in both files' own "Metric names" docstrings (`fig_f4_passk.py:27-28`,
`tab_t3_headline.py:35`) and read calls (`fig_f4_passk.py:81`, `inputs.where("cost_cv", ...)`). Nothing in the
`analysis/` tree emits either name: `econ.py` never calls `metrics.Metric(...)` at all (confirmed: zero matches;
it only builds and writes `econ-report.json`, `econ.py:682-684`), and its own `cost_cv_median` (`econ.py:288`) is
a different thing — an aggregate median across tasks, not the per-task `cost_cv` record panel d of F4 needs.

Separately: `envelope_ratio_r`/`envelope_ratio_c` — flagged as missing in the same spot by an earlier wave's note
on this same package's item (gap-7ec3ef's Notes, 2026-10-02) — are now confirmed **produced**:
`envelope.py::level_metrics` (~line 175) emits `metric=f"envelope_ratio_{name}{suffix}"` for both `r` and `c`.
That half of the earlier finding is resolved; `cc_<k>` and per-task `cost_cv` are the parts still open.

## Why it matters

Goal: proof (ViabilityBench analysis completeness). F4's panel d (cost variance across seeds) and T3's `cc_<k>`
column can never render — `fig.skip("d", "no per-task cost_cv records")` (`fig_f4_passk.py:108`) is presumably
what actually happens today, a silent skip rather than a visible gap, until a producer exists.

## Where

- `benchmarks/viabilitybench/analysis/fig_f4_passk.py` and `tab_t3_headline.py` (the readers).
- `benchmarks/viabilitybench/analysis/econ.py` (the natural place for a per-task `cost_cv` producer, since it
  already computes `cost_cv_median` from presumably the same per-task values) and/or a dedicated metric-emitting
  pass.
- Reference for the pattern to follow: `benchmarks/viabilitybench/analysis/envelope.py::level_metrics` (now
  working for `envelope_ratio_*`).

## Current state

No producer for `cc_<k>` or per-task `cost_cv` exists anywhere in `analysis/`.

## Plan

1. Add a `cc_<k>` producer: `CC_k = pass^k / pass@k` per arm, pooled cell — likely belongs in whichever script
   already computes `pass^k`/`pass@k` (check `metrics.py` or `econ.py` for those).
2. Add a per-task `cost_cv` producer in `econ.py`, emitting one `MetricRecord` per task (reusing the values
   `cost_cv_median` already aggregates from) instead of only the aggregate.
3. Confirm F4 panel d and T3's `cc_<k>` column render once both exist.

## Done when

- `fig_f4_passk.py`'s panel d renders (no longer skips for lack of `cost_cv` records).
- `tab_t3_headline.py`'s `cc_<k>` column has data.
- The `[[verify]]` command passes.

## Notes

- This narrows and updates the earlier note on `gap-7ec3ef` (2026-10-02): `envelope_ratio_r`/`envelope_ratio_c`
  are now confirmed fixed by task 6123's `envelope.py` work; only `cc_<k>` and per-task `cost_cv` remain
  unproduced. Filed as its own item since `gap-7ec3ef` is now closed and this is a real, still-open gap.
