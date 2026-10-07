+++
id = "gap-6e7a86"
kind = "gap"
title = "X3's post-step stream is F8 honeypots only; run records don't mark S09's gaming-prone knob cells"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "19f76451c"
source = "wave-17 follow-up reports 2026-10-04 (gap-90fae4, gate 17a)"
discovered_from = "gap-90fae4 (closed; own closing evidence names this as a queued caveat)"
anchors = ["benchmarks/viabilitybench/analysis/replay_closure.py", "benchmarks/viabilitybench/streams/s3_disturbance.toml"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_x3_post_step_includes_gaming_prone_knob_cells' benchmarks/viabilitybench/ && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/ -q -k test_x3_post_step_includes_gaming_prone_knob_cells"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T15:52:44Z"
commit = "19f76451c"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T09:03:19Z"
forced = false
evidence = "Gate 19 (merged 19f76451c): verify passes; bench suite 754 passed. Records mark S09's gaming-prone knob cells (F1/F3/F4/F5 at ladder 4-5, inferred; spec-3e1dc7 asks S09 to state it) and X3's post-step population uses them."
+++

## Problem

X3 is now evaluated from a false-green step replay (`gap-90fae4`, closed at gate 17a: `audit/replay.py`
exposes per-position draws, `replay_h5` exposes `draw_order`/`stream_units`, and `replay_closure`'s
X3 adapter replays the step with S06's E3 breach rule), but the post-step stream it evaluates is
only the F8 honeypots, not the mixed stream S09 actually designed. S09 §4.4's X3 row says: "a
false-green-step replay: block A streams switching to the gaming-prone knob cells and F8
honeypots (block E) at t = 40" (`tmp/cybernetic-harness/specs/S09-experiments.md:432`) —
i.e. the post-step condition should be block A records whose stream position switched to a
gaming-prone knob cell *and* block E's honeypots, together.

`replay_closure.py`'s own module doc comment already states the shortfall precisely: "The
false-green step is block A's (`p1_core`) first 40 green units in stream order, then block E's
(`log1_f8_honeypots`)... with several runs' positions interleaved. The gaming-prone knob cells
S09 also switches to are not marked in the records, so the post-step stream holds the F8
honeypots alone." (`benchmarks/viabilitybench/analysis/replay_closure.py:29-32`). Confirmed: no
run record, driver code, or stream config anywhere in `benchmarks/viabilitybench/` marks which
positions got a gaming-prone knob cell (grepped `disturbance_kind`/`knob_cell`/`gaming_prone`
across every `.py` file: zero matches). S09 §5.1's H5 design describes where these cells come
from: "F1, F3, F4, F5 with gaming-prone knob cells at 40%, F8 at 8%"
(`S09-experiments.md:351`) — F1/F3/F4/F5 are task families
(`benchmarks/viabilitybench/families/f{1,3,4,5}_*/gen.py`), and the knob/disturbance
configuration lives in `benchmarks/viabilitybench/streams/s3_disturbance.toml`/
`s3_disturbance_hooks.toml` — but nothing records, on the run record itself, whether a given
position's knob cell was the gaming-prone kind.

## Why it matters

Goal: proof, S09 closure test X3 (M4 → M1 → M4). Without the gaming-prone knob-cell positions
marked, X3 evaluates against a thinner, honeypot-only post-step signal than S09 pre-registered
— understating how fast (or whether) the controller's audit boost catches a false-green step in
the condition that's actually meant to provoke gaming, not just the honeypots designed to be
obviously gameable.

## Where

- `benchmarks/viabilitybench/streams/s3_disturbance.toml`,
  `streams/s3_disturbance_hooks.toml` (where gaming-prone knob cells are configured).
- Whichever driver code applies these hooks when generating/running block A's `p1_core` stream
  (needs to find and instrument; not yet located to a specific file).
- `benchmarks/viabilitybench/analysis/replay_h5.py`, `replay_closure.py` (the X3 adapter;
  consumes the new marker once it exists, lines 29-32 and `stream_units`).

## Current state

The honeypot half of the post-step stream works (`gap-90fae4`'s fix). The gaming-prone-knob-cell
half has no data anywhere to draw on.

## Plan

1. Locate where block A's stream applies `s3_disturbance.toml`'s hooks per position, and have
   it record on the run record (or a sibling file the driver already writes) which positions
   got a gaming-prone knob cell, not just honeypot positions.
2. Extend `replay_h5.py`'s record-reading (`stream_units`/`units`) to recognize these marked
   positions as part of the post-step population alongside block E's honeypots.
3. Regression test: a synthetic stream with a marked gaming-prone position produces a wider
   X3 post-step population than honeypots alone.

## Done when

- X3's post-step stream includes block A's marked gaming-prone-knob-cell positions, not only
  block E's honeypots.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-17 follow-up, gap-90fae4, gate 17a): confirmed at main HEAD `c796f09c1`.
  `gap-90fae4`'s own closing evidence names this exact follow-up: "Caveat queued: no
  gaming-prone cells in the records." `replay_closure.py`'s module doc comment states the same
  gap almost verbatim, including the "several runs' positions interleaved" detail (kept here as
  context on the current implementation, not a separate fix this item asks for).

## Progress

- gap-6e7a86: implemented. Both this item's own anchor and the wave assignment point at
  `streams/s3_disturbance.toml` for "where gaming-prone knob cells are configured"; that file and
  `s3_disturbance_hooks.toml` configure S08 §4.6's five *disturbance* kinds (provider_fault,
  model_swap, harder_mix, convention_flip, budget_cut), not S09's gaming-prone knob cells, which
  turn out to be a property of a task's own `(family, ladder)` pair, not a stream hook — grepped
  both files and found no family/ladder keying at all. `families/f1_pyconv/gen.py`'s
  `PLANTED_GAMING` was the other candidate marker and is also wrong: it's a family-wide constant,
  not per-instance, so it can't be the thing S09 calls "40%".
- Derived the definition from S09 §4.4/§5.1's H5 design ("F1, F3, F4, F5 with gaming-prone knob
  cells at 40%") cross-referenced with `common.knobs`'s S08 §4.4 difficulty bands: F1/F3/F4/F5 at
  ladder level 4 or 5 (2 of each family's 5 levels = 40%). No code or spec text states this as an
  explicit formula anywhere I found; it's written into `common/knobs.py`'s new
  `is_gaming_prone_knob_cell()` and its docstring as a derivation, not asserted as an
  already-official fact.
- `driver/records.py::build()` now stamps every task with `gaming_prone_knob_cell` at record-build
  time (False for a plan-slice row, whose ladder is null). `analysis/replay_h5.py` gained
  `gaming_prone_units()`, reading that stamped field or deriving it from `family`/`ladder` for a
  record from before the field existed. `analysis/replay_closure.py`'s `x3_from_matrix` now draws
  the pre-step 40 from block A units that are *not* gaming-prone, then includes block A's
  gaming-prone units together with block E's honeypots in the post-step population. Block E
  non-empty is still required to evaluate at all — kept that guard unchanged on purpose, so
  `test_closure_replays_emit_their_preregistered_estimands`'s F8-less `log1_tree` fixture keeps
  its current "not evaluated... block E" result.
- Updated `test_replay_closure.py::step_tree()` to generate its pre-step 60 at ladder 1-3 only
  (never gaming-prone), so the existing `test_x3_evaluates_from_a_false_green_step_replay`
  baseline (60 block-A units, step at 40) is unaffected; added a `gaming_prone=` parameter and a
  new test, `test_x3_post_step_includes_gaming_prone_knob_cells`, covering the wider post-step
  population and the derive-from-family/ladder fallback for a record predating the field. Added
  `test_gaming_prone_knob_cell_is_f1_f3_f4_f5_at_ladder_4_or_5` to `families/common/test_common.py`.
- Verified load-bearing: reverted `x3_from_matrix` to its old `before[:X3_STEP] + after` body, and
  the new test's `post_step_units == 130` assertion failed (120, as before the fix); restored and
  re-confirmed green. Suites run in the benchmark venv: `analysis/` full (111 passed),
  `driver/test_run_cli.py` (26 passed), `families/common/test_common.py` (37 passed), plus the
  named `[[verify]]` command directly.
