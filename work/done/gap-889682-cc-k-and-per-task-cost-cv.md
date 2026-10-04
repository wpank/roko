+++
id = "gap-889682"
kind = "gap"
title = "cc_<k> and per-task cost_cv MetricRecords have no producer; fig_f4 panel d and tab_t3's CC column stay empty"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "d16bc9969"
source = "wave-8 follow-up reports 2026-10-03 (PK49 gap-7ec3ef)"
discovered_from = "gap-7ec3ef"
anchors = ["benchmarks/viabilitybench/analysis/fig_f4_passk.py", "benchmarks/viabilitybench/analysis/tab_t3_headline.py", "benchmarks/viabilitybench/analysis/econ.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_cc_k_metric_is_produced' benchmarks/viabilitybench/analysis/test_econ.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_econ.py -k test_cc_k_metric_is_produced -q"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T13:06:33Z"
commit = "d16bc9969"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T10:31:59Z"
forced = false
evidence = "Gate 17b (merged d16bc9969): verify test_cc_k_metric_is_produced passes. econ.py::cc_metrics produces cc_<k> and per-task cost_cv MetricRecords, with a task-cluster bootstrap for cc_k's interval."
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

## Progress

- Added `econ.py::cc_metrics(records, experiment_id, ks=KS, b=B, seed=0, alpha=0.05) -> list[metrics.Metric]`,
  following `envelope.py::envelope_metrics`'s own pattern (a self-contained producer returning `metrics.Metric`
  objects that `report.metric_record` turns into `vb.metric_record/1` rows, not a new output file or schema).
  Iterates `metrics.cells(records)` itself (bug-40de03's 3-tuple) and reuses `independent_runs`/`_labels`/
  `pass_at_k`/`passk.pass_k`/`_spend` -- the exact values `_arm` already computes for `pass_hat_k`/`pass_at_k`/
  `variance.cost_by_task` -- rather than recomputing anything new.
- `cost_cv`: one `MetricRecord` per (cell, task) with a `task.instance_id == "<id>"` clause, value = that task's
  `stdev/mean` of `costs.api_equiv_usd` across its independent seeds (skipped below two known-cost seeds), no CI
  (a description of spread, matching `fig_f4_passk.py`'s own read). Straightforward; the harder part was `cc_<k>`.
- `cc_<k> = pass^k / pass@k`: F4's panel c needs a real CI (`need_ci=True`; confirmed `figlib.Output.take` raises
  `FigureError` without one, not a silent skip). Tried reusing `bootstrap.paired_bootstrap` first (CPR's own
  pattern) and reverted it: `paired_bootstrap`'s row-level resampling resamples seeds *within* a task too, so a
  replicate is a flat row sequence -- fine for a per-row-weighted statistic like `vs_rate`'s, but `pass_k`/
  `pass_at_k` are nonlinear per-task formulas, and once a task is drawn more than once its repeated rows collapse
  indistinguishably into the same flat sequence, silently under-counting the resample. Wrote `_cc_ci` instead: a
  direct cluster bootstrap that resamples *tasks* with replacement, keeping each task at its own observed seed
  pattern (valid since `pass_k`/`pass_at_k` are already means over tasks of a task-level statistic), using
  enumeration-indexed dict keys (`{(index, task): labels[task] for index, task in enumerate(drawn)}`) so a
  repeated task contributes its own term instead of collapsing in the replica's `labels` mapping. Percentile, not
  BCa (CPR's own ratio interval): no bias/acceleration estimator exists for this ratio yet, and inventing one was
  out of this item's scope -- documented as a deliberate simplification, not a silent gap.
- Reused `UNDEFINED_CPR`/`_bound()`/`_quantile()` (already in `econ.py`) for a degenerate replicate (pass@k = 0 or
  undefined) rather than inventing a parallel sentinel, same convention as CPR's own replicates with no VS.
- Caught and fixed a severe, unrelated-looking regression while checking this item's own "Done when" end to end
  (not just the unit test): `fig_f4_passk.py`'s real query shape found *zero* of my new `cc_3`/`cost_cv` records.
  Traced it to `figlib.py::BASE_FIELDS` never having gained `"harness"` when bug-40de03 made every cell's cut
  carry a harness clause -- which meant **every** record `report.py`/`econ.py` ever produces, for every arm, was
  equally unfindable, not just mine. Filed and fixed as its own item, reg-cc50d3 (own commit on this branch), since
  leaving it broken would have made this item's "Done when" unverifiable regardless of `cc_metrics` being correct.
- Verified end to end (not just the required unit test): built `cc_metrics`'s records, serialized them with
  `report.metric_record`, wrote them to a `.jsonl`, loaded them with `figlib.load`, and confirmed
  `inputs.where("cc_3", ..., clauses={"stream.id": (None, "p1_core")})` and `inputs.where("cost_cv", ...,
  clauses={"task.instance_id": figlib.ANY})` -- `fig_f4_passk.py`'s own exact query shapes -- both find them,
  after reg-cc50d3's fix (zero before it).
- Added `test_cc_k_metric_is_produced` (`analysis/test_econ.py`): eight identical tasks (c = 3 of 5 seeds) give
  closed-form, non-degenerate pass^k/pass@k (so the bootstrap never hits a degenerate replicate), checked against
  hand-computed CC_1 = 1 (the k = 1 identity, pass^1 == pass@1 always), CC_3 = 0.1 and CC_5 = 0 exactly; confirms
  `cost_cv` is 0 for every task (uniform cost); validates every produced record against `metric-record` schema
  the way `test_envelope_metrics_carry_the_names_f3_and_t7_read` does; and confirms `cheap_x` (`golden_records()`'s
  always-failing arm, pass@k = 0 at every k) gets no `cc_<k>` at all. Verified the test is load-bearing by
  deliberately inverting the ratio in `_cc` and confirming the test fails, then restoring it.
- Verify: named `[[verify]]` command -> 1 passed. Full `benchmarks/viabilitybench/analysis` suite: 108 passed.
  Full `benchmarks/viabilitybench/audit` suite (unaffected, run anyway per this wave's instruction): 62 passed.
