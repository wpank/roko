+++
id = "gap-04e8e2"
kind = "gap"
title = "ViabilityBench plan-slice records carry no queue waits or per-class costs, so the report prints them as not recorded"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver", "benchmarks/viabilitybench/analysis"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-report's report on gap-b24517)"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py::_feature_cell", "benchmarks/viabilitybench/analysis/metrics.py::plan_slice", "benchmarks/viabilitybench/schema/run-record.schema.json", "benchmarks/viabilitybench/driver/records.py:104", "benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-b24517", "gap-1cd676", "gap-89f393", "gap-a6e2c3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'queue_wait_s' benchmarks/viabilitybench/schema/run-record.schema.json && grep -qw 'def test_plan_slice_records_carry_queue_waits_and_class_costs' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_plan_slice_records_carry_queue_waits_and_class_costs -q"
+++

## Problem

For the plan-level slice, S09 §4.9 reports these measures per feature and arm:

- VF, cost, makespan T and queue waits. Queue waits are the time ready work spent waiting for a dispatch slot or a provider rate limit. They are summed per feature and shown beside T, never subtracted from it.
- For `roko_plan` only, the process measures: the planner's share of the cost (class `plan`), realized parallelism (Σ task busy time ÷ T), tasks escalated ÷ tasks run, and features whose integration the whole-plan gate rejected (class `integrate`).

At BASE (4315add32) nothing produces any of these:

- `analysis/metrics.py::_feature_cell` (:424) reads `execution.queue_wait_s`. No runner writes it, and the run-record schema's `execution` object has no such property (`schema/run-record.schema.json:54`). The module docstring says so: "Queue waits are read from `execution.queue_wait_s`, which no runner records yet."
- `costs` (schema :128) is a single summed set with `additionalProperties: false`. It has no per-class breakdown. `plan_slice` therefore emits `not_recorded`: "the planner's cost share, realized parallelism, escalations and gate-rejected integrations need per-class costs and task timings that run records do not carry yet" (metrics.py:285).
- `report.py` (:413) prints "not recorded" wherever a value is missing.

## Why it matters

Pilot benchmark (epic spec-567e52): E12, the plan-slice run (gap-1cd676), lists queue waits and the process measures among its outputs. Without them, the slice can't show whether Roko's makespan comes from its plan or from provider rate limits, and it can't give the planner's share of the cost.

## Where

- Consumers: `analysis/metrics.py` (`_feature_cell`, `plan_slice`) and `analysis/report.py`.
- Schema: `schema/run-record.schema.json` (`execution`, `costs`).
- Producers still to write: `driver/run_roko.py`, which reads Roko's records into attempts, and `driver/records.py` (:104, the record builder).

## Current state

The analysis side is ready and prints the gaps honestly. Neither the Roko arm nor the direct arms record queue waits, task timings or cost classes. It is not known whether Roko's Graph activity log (`.roko/state/graph/<plan>/activities.jsonl`) holds per-task ready and dispatch times. Plan-generation spend never reaches Roko's cost records (gap-a6e2c3), so the planner's share also needs that fix.

## Plan

1. **Schema.** Add `execution.queue_wait_s` (number or null) and `costs.by_class` (class → USD or null). Add per-task timings (ready, started, finished) if realized parallelism needs them. Null means unknown, never 0.
2. **Roko producer.**
   - Check whether the Graph checkpoint or activity log records when each task became ready and when it was dispatched, and whether provider rate-limit waits are visible. If they are, compute the queue waits in `run_roko.py`. If not, file a Rust item for the Graph engine to record them.
   - Take the cost classes from Roko's cost rows (plan, execute, retry, escalate, integrate). This depends on gap-a6e2c3 for the planner.
3. **Direct and CLI arms.** Record `queue_wait_s` as null when the harness can't observe waits.
4. **Analysis.** Compute the process measures when the data exist, and keep the `not_recorded` note only for what is still missing.
5. **Test.** Add `test_plan_slice_records_carry_queue_waits_and_class_costs`. A fake roko emits task timings and classed costs, and the record, the schema and the report carry them.

## Done when

- [ ] Plan-slice records carry queue waits and per-class costs whenever the arm can observe them, and null otherwise.
- [ ] The report prints them instead of "not recorded".
- [ ] The `[[verify]]` command passes.

## Notes

- Keep `metrics.py`'s rule that one unknown cost makes a cell unmeasurable: an unknown class cost must stay null, never 0.
