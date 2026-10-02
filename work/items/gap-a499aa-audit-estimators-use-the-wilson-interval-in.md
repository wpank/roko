+++
id = "gap-a499aa"
kind = "gap"
title = "Audit estimators use the Wilson interval in every cell (Wald misses S05's coverage target)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["bench/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave 1, task 7107 report; Will's decision 2026-10-02"
anchors = ["benchmarks/viabilitybench/audit"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_wilson_interval_meets_coverage_in_every_cell' benchmarks/viabilitybench/audit/ && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit -k test_wilson_interval_meets_coverage_in_every_cell"
+++

## Problem

Simulation of task 7107's audit estimators (`benchmarks/viabilitybench/audit/`) showed S05 §4.5's interval rule misses its coverage target: Wald once n_eff ≥ 30 and ≥ 5 events gives 0.909 (tilted, ρ=0.30, 200 units), 0.927 (tilted, ρ=0.15, 400) and 0.933 (uniform) against SC1's 0.93, while Wilson at n_eff in every cell gives at least 0.957.

## Why it matters

The false-green estimate is M4's headline number (V6, the paper's H5). Will decided on 2026-10-02 to use Wilson everywhere.

## Where

`benchmarks/viabilitybench/audit/` (the estimators from 7107, merged in e53136640) and S05 §4.5 (`tmp/cybernetic-harness/specs/S05-deep-audits.md`, untracked: edit in place).

## Current state

The Wald branch is implemented as specified; `replay.py` prints the tilted ρ=0.30 cell as NOT MET.

## Plan

1. Use the Wilson interval at n_eff in every cell; drop the Wald branch.
2. Amend S05 §4.5 to say so, citing this item and the simulation figures above.
3. Make the coverage test assert ≥ 0.93 in every cell, tilted ρ=0.30 included.

## Done when

The verify passes and `replay.py` reports every cell MET.

## Notes

Decided by Will, 2026-10-02 (coordinator question round). Backlog context: tmp/backlog/2026-10-02-complete-and-wire.

## Progress

- 2026-10-02: implemented at 9f469522c (branch `work/gap-46fd19-2`). The estimators use Wilson at Kish n_eff in every
  cell (no Wald branch); the 7114 fixture is regenerated. `test_wilson_interval_meets_coverage_in_every_cell` replays
  the 200-unit window 1,000 times at every ρ, uniform and tilted, and requires coverage ≥ 0.93 and |bias| ≤ 0.01 in
  all six cells. The verify passes, and `replay.py` on that window reports every cell met (tilted ρ = 0.30: 0.909 NOT
  MET at the base, 0.989 met now). S05 §4.5 is amended in place (untracked). Two backlog specs still describe the
  Wald/Wilson switch and should follow the fixture: 7114 (`estimate.rs`) and 7131.
