+++
id = "find-496f7c"
kind = "finding"
title = "fd_claude (claude-opus-5-5) solves the whole pilot slice: VS 1.0 at every level over 60 runs, above S08's frontier-direct bands from level 3 up (level 5 band 0.4-0.7)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-327242 worker (claude-agent), 2026-10-21: Pilot B fd_claude seeds 1-3"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/families/f1_pyconv/ladder.toml", "benchmarks/viabilitybench/families/f4_kvtool/ladder.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242", "gap-dd670b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'frontier_above_band' benchmarks/viabilitybench/families/f1_pyconv/ladder.toml"
+++

## Problem

In Pilot B (gap-327242; bundle `benchmarks/viabilitybench/reports/pilot_b/`), `fd_claude` (Claude Code 2.1.282 on
`claude-opus-5-5`, effort high) solved all 60 runs of the pilot slice: VS 1.0 at every level ℓ1-ℓ5 over seeds 1-3,
pass^3 = 1.0, 0 false greens. It took a median of 6 turns and 51 s, and $0.22 API-equivalent, per task. S08 §4.4's
design bands for frontier-direct are below that from ℓ3 up:

| level | fd_claude VS (12 runs, 4 tasks) | S08 band, frontier direct | cheap_direct gpt-oss-120b (PILOT-A-RECAL) |
|---|---|---|---|
| ℓ1 | 1.0 | ≥ 0.9 | 0.50 |
| ℓ2 | 1.0 | ≥ 0.85 | 0.50 |
| ℓ3 | 1.0 | 0.7-0.9 | 0.0 |
| ℓ4 | 1.0 | 0.5-0.8 | 0.0 |
| ℓ5 | 1.0 | 0.4-0.7 | 0.0 |

G0 checks frontier ℓ5 ≥ 0.4 only (pass), and S08's calibration rule covers only frontier ℓ5 < 0.4 ("softened"). It
says nothing about frontier-direct above its bands.

## Why it matters

- S09 §4.2's honest envelope report must show "the families and levels where frontier-direct wins". On this slice
  the frontier never loses a task, so the slice measures where cheap arms fall short. It cannot show where
  frontier-direct starts to fail, and the ℓ5 "frontier-only" level is not near the frontier's edge.
- The bands also feed the 2PL IRT difficulty scale after LOG1. A level every frontier run passes adds no information
  on that side.

## Where

- `benchmarks/viabilitybench/families/f1_pyconv/ladder.toml`, `families/f4_kvtool/ladder.toml`: the ℓ3-ℓ5 knobs.
- S08 §4.4's bands and calibration rule (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md`).

## Current state

Nothing is decided. Only F1 and F4 are measured, 4 tasks per level, and Pilot B's roko_fixed arm has not run yet
(gap-f3944c).

## Plan

1. Decide whether the frontier bands are targets the ladder must meet, as the cheap bands are, or only a guide.
2. If they are targets, add a calibration rule for frontier above band (for example, raise `k_size`/`k_files` or
   weaken `k_vis` at ℓ4-ℓ5), and check it on the other families before LOG1.

## Done when

- [ ] S08 §4.4 states what happens when frontier-direct is above its band, and the pilot slice either meets the bands
      or carries the decision: the `[[verify]]` command reads the decision note.

## Notes

- Found by gap-327242's worker on 2026-10-21.
- `fd_claude`'s ℓ5 tasks cost the most ($0.37 a task API-equivalent, against $0.20 at ℓ1-ℓ2), so the levels do
  order effort, if not outcome.
