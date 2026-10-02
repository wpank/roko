+++
id = "gap-5ddf9b"
kind = "gap"
title = "PK27 ViabilityBench proof: Confidence sequences, McNemar's test and CUPED in analysis/ (+5 more)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 27
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK27"
anchors = ["benchmarks/viabilitybench/driver/vb.py"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-5ebb4f", "gap-1149aa", "gap-46fd19"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_confidence_sequence_keeps_anytime_coverage' benchmarks/viabilitybench/analysis/test_toolkit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_toolkit.py -k test_confidence_sequence_keeps_anytime_coverage -q"

[[verify]]
command = "grep -qw 'def test_glmm_and_irt_reproduce_reference_fits' benchmarks/viabilitybench/analysis/models/test_models.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/models/test_models.py -k test_glmm_and_irt_reproduce_reference_fits -q"

[[verify]]
command = "grep -qw 'def test_envelope_stops_at_the_first_failed_level' benchmarks/viabilitybench/analysis/test_envelope.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_envelope.py -k test_envelope_stops_at_the_first_failed_level -q"

[[verify]]
command = "grep -qw 'def test_simulator_reports_coverage_in_band' benchmarks/viabilitybench/analysis/test_simulate.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_simulate.py -k test_simulator_reports_coverage_in_band -q"

[[verify]]
command = "grep -qw 'def test_replay_io_reads_records_deterministically' benchmarks/viabilitybench/analysis/test_replay.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay.py -k test_replay_io_reads_records_deterministically -q"

[[verify]]
command = "grep -qw 'def test_log1_refuses_to_start_without_the_lock' benchmarks/viabilitybench/driver/test_campaign.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -k test_log1_refuses_to_start_without_the_lock -q"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK27, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3335 | M | p2 | Confidence sequences, McNemar's test and CUPED in analysis/ | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3335-analysis-cs-mcnemar-cuped.md` |
| 2 | 3337 | M | p3 | GLMM and 2PL IRT for H3 and the envelope's difficulty scale | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3337-analysis-glmm-and-2pl-irt.md` |
| 3 | 3338 | M | p1 | The H1 envelope and the Holm mapping: fixed-sequence tests over levels 1-5 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3338-analysis-envelope-and-holm-mapping.md` |
| 4 | 3339 | M | p2 | Synthetic simulator: 95% CI coverage of 0.93-0.97 with a planted 0.15 effect | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3339-analysis-synthetic-simulator-coverage.md` |
| 5 | 3340 | S | p2 | Replay IO: read run records and Roko's S01 records deterministically | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3340-analysis-replay-io.md` |
| 6 | 3341 | M | p1 | Pre-registration lock builder and blinding; LOG1 refuses to start without the lock | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3341-prereg-lock-builder-and-blinding.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/blind.py`, `benchmarks/viabilitybench/analysis/cs.py`, `benchmarks/viabilitybench/analysis/cuped.py`, `benchmarks/viabilitybench/analysis/envelope.py`, `benchmarks/viabilitybench/analysis/holm.py`, `benchmarks/viabilitybench/analysis/lock.py`, `benchmarks/viabilitybench/analysis/mcnemar.py`, `benchmarks/viabilitybench/analysis/models/glmm.py`, `benchmarks/viabilitybench/analysis/models/irt.py`, `benchmarks/viabilitybench/analysis/models/test_models.py`, `benchmarks/viabilitybench/analysis/replay.py`, `benchmarks/viabilitybench/analysis/simulate.py`, `benchmarks/viabilitybench/analysis/test_envelope.py`, `benchmarks/viabilitybench/analysis/test_lock.py`, `benchmarks/viabilitybench/analysis/test_replay.py`, `benchmarks/viabilitybench/analysis/test_simulate.py`, `benchmarks/viabilitybench/analysis/test_toolkit.py`, `benchmarks/viabilitybench/driver/campaign.py`, `benchmarks/viabilitybench/driver/test_campaign.py`, `benchmarks/viabilitybench/driver/vb.py`, `benchmarks/viabilitybench/requirements-analysis.lock`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK20 (gap-5ebb4f), PK21 (gap-1149aa), PK22 (gap-46fd19).
- Suggested model: sonnet.
- 2026-10-02 (filer-grpD, backlog wave reports, PK96 gap-a6dab7): task 3340 (replay IO) is one of the producers
  `benchmarks/viabilitybench/analysis/figlib.py`'s "Metric names" section names for metric names the figure/table
  scripts already declare in their `READS` but nothing emits yet — at minimum `regret_cum`
  (`fig_f9_routing.py:32`: "cumulative regret against the oracle after p tasks"), which needs the replayed
  policy's per-step choices this task's "Replay IO: read run records ... deterministically" is meant to provide.
  Check `fig_f9_routing.py` and `figlib.py:52-54` for the exact name/clause contract (metric name, `stream.position`
  clause) before closing this task so the already-written reader doesn't need its own follow-up fix.

## Progress

Wave 5, 2026-10-03, branch `work/gap-5ddf9b` (Python only; every verify passes, the whole bench suite is 562
passed, 7 skipped).

- 3335: implemented at fe2129f08. `cs.py` (the betting confidence sequence, anytime p, paired contrasts, built on
  `audit/estimate.py`'s hedged capital), `mcnemar.py`, `cuped.py` (with the LOG1 covariate), `test_toolkit.py`.
- 3337: implemented at 97bca1b59. Decision 3336 (a): numpy and scipy pinned with hashes in
  `requirements-analysis.lock`, imported only under `analysis/models/` (GLMM by adaptive Gauss-Hermite ML, 2PL IRT
  by EM, ICC); each fit matches an independent reference. The models' tests skip without the stack, so its verify
  needs `pip install --require-hashes -r benchmarks/viabilitybench/requirements-analysis.lock` in the venv first.
- 3338: implemented at 5729f4949. `envelope.py` (E*, Holm p per level, the F3/T7 metric names), `holm.py`
  (graphical Holm, S09 §5's multiplicity), and an additive `bootstrap.p_value` with BCa's z0 and acceleration kept.
- 3339: implemented at 9ce9a25b9, full run at 6d5fb847d. Done-when not met: in the full run (1,000 campaigns,
  B = 10,000) the VS-rate interval covers 0.975 ± 0.005, above the 0.93-0.97 band (difference 0.968 and ratio 0.970
  at its top; the other four criteria in band). S09 §4.1's two-stage bootstrap counts the seeds' noise twice;
  resampling n_h − 1 tasks per stratum, tasks alone, covered about 0.95 in 300-campaign checks. Needs Will's choice.
- 3340: implemented at 53a2c1f68. `replay.py` on `report.load_runs`, with S01 copies, blinding hook, stream order
  and the `stream.position == p` cut F9 reads (checked through figlib).
- 3341: implemented at fd3e5f3e6. `lock.py` (build, `--check`, committed, require), `blind.py`; `vb run` and
  `vb campaign` refuse LOG1, live `E-` experiments and `requires_lock` manifests without a committed, clean lock.
  No lock was written or taken (3345).
