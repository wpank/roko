+++
id = "gap-90fae4"
kind = "gap"
title = "replay_h5.estimate doesn't expose the audit lottery's per-position draw order, which X3 needs"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "7789cfee6"
source = "wave-15 follow-up reports 2026-10-04 (gap-1a8ee7, gate 16a)"
discovered_from = "gap-1a8ee7 (open, work/gap-1a8ee7; X3's own more vaguely stated data gap traced precisely)"
anchors = ["benchmarks/viabilitybench/analysis/replay_h5.py::estimate", "benchmarks/viabilitybench/analysis/replay_closure.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_x3_evaluates_from_a_false_green_step_replay' benchmarks/viabilitybench/analysis/ && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/ -q -k test_x3_evaluates_from_a_false_green_step_replay"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T11:30:37Z"
commit = "7789cfee6"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T09:32:38Z"
forced = false
evidence = "Gate 17a (merged 7789cfee6): verify test_x3_evaluates_from_a_false_green_step_replay passes; bench suite 744 passed. audit/replay.py exposes each lottery's per-position draws, replay_h5 exposes draw_order and stream_units, and replay_closure's X3 replays a false-green step at the 5% floor and at M1's 2x boost with S06's E3 breach rule (median delays, paired-bootstrap gain, audit-share guard). Caveat queued: no gaming-prone cells in the records."
+++

## Problem

Now that `roko learn homeostasis replay --evaluate` runs the R-H6 evaluator for real
(`work/gap-1a8ee7`, commit `466c3e2bb`, not yet merged — the CLI-entry-point half of this
item's original finding), X3 still has no data behind it, for a precise, already-documented
reason: `benchmarks/viabilitybench/analysis/replay_h5.py::estimate` doesn't expose the audit
lottery's per-position draw order, which X3 needs. The closure-replay module's own doc comment
already says so: "**X3** `median_delay_to_fg_breach(floor, boost)`... Needs the lottery replay's
own per-position draw order, to find when a false-green run would first have been caught under
the fixed (floor) and adaptive (boost) audit rate. `replay_h5.py`'s `estimate()` returns only
each cell's aggregate coverage/bias/theta_ratio..., not that per-position sequence, so this
adapter reports X3 not evaluated until a replay exposes it."
(`benchmarks/viabilitybench/analysis/replay_closure.py:27-31`).

Confirmed in `replay_h5.py::estimate` (lines 64-92): it builds `reports` by calling
`lottery_replay.replay(drawn, runs=int(reps), seed=seed, lam=...)` per selection (`uniform`,
optionally `tilted`), then folds each run's result down to nine aggregate `cells` (coverage,
bias, theta_ratio per `(rho, selection)` pair) and a handful of scalar summaries (`n_units`,
`false_greens`, `theta_census`, etc.) — never returning or exposing the underlying per-position
sequence `lottery_replay.replay` must compute internally to produce those aggregates.

## Why it matters

Goal: proof, S09 closure test X3 (cross-loop closure: M4 → M1 → M4). X3 specifically measures
*how fast* a false-green step gets caught (median resolutions from the step to breach) under a
fixed vs. an adaptive audit rate — an inherently sequential/positional question that no
aggregate statistic can answer after the fact. Without this, X3 can never evaluate, regardless
of how much replay data exists, because the data shape itself doesn't carry what the estimand
needs.

## Where

- `benchmarks/viabilitybench/analysis/replay_h5.py::estimate` (needs to expose the per-position
  sequence, not just the folded cells).
- `benchmarks/viabilitybench/analysis/replay_closure.py` (the X3 adapter that currently reports
  `evaluated: false`; the consumer once the data exists).
- `lottery_replay.replay` (wherever this lives — the function that already computes a
  per-position draw internally before `estimate` folds it away).

## Current state

`estimate()` discards the per-position sequence after folding it into aggregate cells. X3's
adapter in `replay_closure.py` already has its "not evaluated" path written and documented;
only the data source is missing.

## Plan

1. Have `lottery_replay.replay` (or a new sibling entry point) return, in addition to its
   existing aggregate summary, the per-position sequence: for each position, whether it was
   selected for audit, under which rate (floor/boost), and whether/when a false-green breach
   was first caught.
2. Expose that sequence from `replay_h5.estimate` (e.g. an additional key in its returned dict,
   or a new function `replay_h5.positions()` the X3 adapter can call).
3. Wire `replay_closure.py`'s X3 adapter to compute `median_delay_to_fg_breach` from that
   sequence instead of reporting `evaluated: false`.
4. Regression test: a synthetic stream with a known false-green step produces a real X3 verdict
   (not `evaluated: false`).

## Done when

- X3 reports a real verdict (not `evaluated: false`) when fed a replay with a false-green step.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-15 follow-up, gap-1a8ee7, gate 16a not yet merged): confirmed at main HEAD
  `7d82b944a` (both `replay_h5.py` and `replay_closure.py` are unchanged by `work/gap-1a8ee7`'s
  diff, so this is verifiable on `main` directly, independent of that branch's merge status).
  Traces the X3 half of `gap-1a8ee7`'s own, more vaguely stated "X3 has no data behind it" note
  to its precise code-level cause; `gap-ed1a08`'s closing evidence (done) independently
  confirms the same gap at a higher level ("X3 (per-position lottery breach timing)... [has] no
  data behind [it] in this codebase").

## Progress

- gap-90fae4: implemented at 2b79bf255. `audit/replay.py::draw_order` and `replay_h5.draw_order`/`stream_units` expose the lottery's per-position draws; `replay_closure` evaluates X3 from a false-green step (block A's first 40 units, then block E's) at the 5% floor and the 2x `audit_boost`, with S06's E3 breach rule, a paired bootstrap CI and the 12% audit-share guard. The verify passes (bench venv), and so do test_replay_closure, test_replay_h5, test_replay_runner and audit/tests (67).
