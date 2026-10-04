+++
id = "gap-6e7a86"
kind = "gap"
title = "X3's post-step stream is F8 honeypots only; run records don't mark S09's gaming-prone knob cells"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-17 follow-up reports 2026-10-04 (gap-90fae4, gate 17a)"
discovered_from = "gap-90fae4 (closed; own closing evidence names this as a queued caveat)"
anchors = ["benchmarks/viabilitybench/analysis/replay_closure.py", "benchmarks/viabilitybench/streams/s3_disturbance.toml"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_x3_post_step_includes_gaming_prone_knob_cells' benchmarks/viabilitybench/ && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/ -q -k test_x3_post_step_includes_gaming_prone_knob_cells"
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
