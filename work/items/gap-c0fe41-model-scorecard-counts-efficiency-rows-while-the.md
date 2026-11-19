+++
id = "gap-c0fe41"
kind = "gap"
title = "model-scorecard counts efficiency rows while the rung readout counts verdicts: serve its tries, passes and cost from the per-rung fold, keep its per-call latency and router columns"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-20
updated = 2026-10-20
source = "round-v7-roko-speed#one-readout"
discovered_from = "agent:roko-speed (round v7, the lead's one-readout condition for gap-3217a0, 2026-10-20)"
anchors = ["crates/roko-serve/src/routes/learning/mod.rs::model_scorecard"]
links = { depends_on = ["gap-3217a0"], blocks = [], related = ["gap-3217a0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn model_scorecard_counts_the_rung_readouts_tries' crates/roko-serve/src && cargo test -p roko-serve --lib model_scorecard_counts_the_rung_readouts_tries"
+++

## Problem

`GET /api/learn/model-scorecard` and the per-rung readout of gap-3217a0 (`GET /api/learn/rungs`, `roko learn rungs`)
both count tries, passes and cost per model, from different records:

- The scorecard counts `efficiency.jsonl` rows. That is one or more rows per try, never a count of tries: the
  Hermes plugin's insights page has to say "attempts", not "tasks" (dogfood U6, plugin `insights.py::models`).
- The rung readout counts settled tries from `roko.verdict/1`, the canonical per-try record. Its rows are keyed by
  role, rung and model, and carry the verified pass, the wall time and the model-time share.

Two counts of the same thing can disagree, and a person can't tell which to trust.

## Why it matters

The lead's condition for gap-3217a0 was "one readout, not a fourth". Three readouts now overlap partly:
- `route-report` audits routing honesty (decision sources, ι, ε_honest). That is a different question, and it stays.
- `model-scorecard` gives per-model counts and cost, plus per-call latency (TTFT, tokens per second, p50/p95/p99 per
  model call) and the cascade router's trials.
- The rung readout gives the per-try numbers.

## Where

- `crates/roko-serve/src/routes/learning/mod.rs::model_scorecard`
- `crates/roko-learn/src/telemetry/rung_stats.rs` (gap-3217a0)
- The plugin's reader, `hermes_plugin_roko/insights.py::models` (plugin-wow's area).

## Plan

1. Serve the scorecard's `task_count`, `success_rate`, `total_cost_usd` and `avg_cost_usd` from the verdicts' per-rung
   fold, summed per model. Deprecate `task_count` in favour of `tries` and `verified_passes`.
2. Keep its per-call latency and router columns. Nothing else measures a single model call.
3. Tell plugin-wow, so the insights page's `models` reads `tries`.

## Done when

The scorecard's per-model tries, passes and cost equal the rung readout's, summed per model, over the same runs.
Checked by a test that builds both from one fixture.

## Notes

Filed by roko-speed as the follow-up the lead asked for, so that the readouts don't stay three overlapping ones with no
plan.
