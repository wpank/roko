+++
id = "bug-3c1c4a"
kind = "bug"
title = "ViabilityBench metrics group runs by arm only, so an arm that runs two models in one experiment is pooled"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-report's report on gap-b24517)"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py::arm_metrics", "benchmarks/viabilitybench/analysis/metrics.py::check_unique", "benchmarks/viabilitybench/analysis/report.py::build", "benchmarks/viabilitybench/arms/cheap_direct.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-b24517", "gap-327242", "gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_arm_with_two_models_is_reported_per_model' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_an_arm_with_two_models_is_reported_per_model -q"
+++

## Problem

The report's cells are arms:

- `report.build` (report.py:164) takes the distinct `record["arm"]` values.
- `metrics.arm_metrics` (metrics.py:173) cuts on `arm == …` alone.
- No metric reads a model. A run record has no top-level model; each attempt carries `model_requested` and `model_reported`.

S09 §4.2 has `cheap_direct` run two models: gpt-oss-120b × 3 seeds, plus the pool's best cheap model × 2 seeds, chosen after the pilot. `arms/cheap_direct.toml` notes that S09 adds that model. If both models' runs land in one experiment under one arm id, one of two things happens:

- the seeds collide, and `check_unique` (metrics.py:154, keyed on arm, instance, variant, seed and replicate) refuses the whole report;
- or the seeds differ, and VS rate, pass^k and $/VS pool two models into one number.

## Why it matters

Pilot benchmark (epic spec-567e52): H1 and H2 compare cells, and the best-of-pool model is its own cell of the cheap × direct row. A pooled number would compare a mix of models against `roko_full` and `fd_claude`.

## Where

The anchors. `false_greens`, `excluded` and the report's per-arm rows share the arm-only grouping.

## Current state

At BASE every arm file allows one model (`models_allow`), so pilot A is unaffected. The problem starts when S09's best-of-pool runs, or when any arm config lists two models.

## Plan

1. **Define the cell.** The cell is (arm, model), with the model taken from the attempts' `model_requested`. Refuse a run whose attempts disagree, unless its arm is declared multi-model.
2. **Group by the cell.** Use (arm, model) in `arm_metrics`, `check_unique`, the false-green and excluded lists, and the report's rows. When an arm ran only one model, keep the arm-only label, so existing reports don't change.
3. **Name the model.** Add the model clause to each MetricRecord's `cell` and `record_filter`.
4. **Test.** Add `test_an_arm_with_two_models_is_reported_per_model`. One arm runs two models, on seeds 1–3 and 1–2, and gets two separate rows with no uniqueness error.

## Done when

- [ ] An arm that ran two models gets one row per model, and none of its metrics pools them.
- [ ] The `[[verify]]` command passes.

## Notes

- S09 §4.2's table (`cheap_direct` row) and line B of the budget table state the two-model design.
