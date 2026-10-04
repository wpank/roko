+++
id = "gap-394f28"
kind = "gap"
title = "Take the ViabilityBench pre-registration lock (task 3345) once the instrument is sound"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
hold = "waits on q-ab27d3 (fd_claude_lite's model), dec-3d5714 (S09's bootstrap coverage) and Will's go-ahead; the shakedown, the hash list and frozen learning are done (gates 8b and 13b-c)"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 3345 (held in wave 8, PK30)"
discovered_from = "gap-2ca903"
anchors = ["benchmarks/viabilitybench/analysis", "benchmarks/viabilitybench/experiments"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-2ca903", "gap-31c0e8", "bug-0b7695"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/experiments/prereg.lock.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/lock.py --check benchmarks/viabilitybench/experiments/prereg.lock.json"

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/PREREG.md && grep -q 'prereg_id' tmp/cybernetic-harness/paper/PREREG.md"
+++

## Problem

Task 3345 of PK30 (gap-2ca903, gated in wave 8) takes the ViabilityBench pre-registration lock: it commits
`benchmarks/viabilitybench/experiments/prereg.lock.json` and writes `tmp/cybernetic-harness/paper/PREREG.md` (D2).
The coordinator held it: a lock freezes the instrument, and on 2026-10-03 the instrument still had open defects.

## Why it matters

Every confirmatory LOG1 result is read against the lock. Locking a known-broken instrument would either void those
results or force an amendment straight away.

## Where

`benchmarks/viabilitybench/analysis/lock.py` (`lock.build`), `experiments/prereg.lock.json`, S09 v1.6, and
`tmp/cybernetic-harness/paper/PREREG.md`.

## Current state

PK30's worker ran a preview `lock.build` at b08c80822 against S09 v1.6, writing nothing. It would lock prereg_id
`s09-v1.6-68765f70` on `prices-2026-09-28`, alpha_fw 0.05, primaries H1-H7, exploratory X1-X4/PL/closure_4, and 81
hashed files (analysis 50, audit 13, streams 15, the price snapshot, the simulation report,
`requirements-analysis.lock`).

## Plan

Before taking the lock:
1. The PK36 shakedown (`driver/test_shakedown.py`, `VB_REQUIRE_REAL_ROKO=1`) passes all eight scenarios against a
   binary built from main (it did 6/8 at 425e256d5; bug-0b7695's driver fixes target the other two).
2. **Done (gap-b001ca).** The lock's HASHED list also covers `experiments/` (log1.toml, budget.toml) and `arms/`,
   excluding the lock file itself from its own hash list; today LOG1's cells, caps and models could change after
   the lock without being caught.
3. planemit's emitted roko.toml sets `[learning] frozen = true` for a pinned-mode arm (roko_fixed, fr_claude),
   or G2's frozen-loop check fails on real LOG1 Roko runs. **Done (gap-127263, gate 13c):** a frozen run now writes its
   episodes to `.roko/runs/<run_id>/episodes.jsonl` and the driver reads them there, so planemit freezes the
   pinned-mode arms again; the bench suite and the shakedown pass with frozen runs.
4. q-ab27d3 (fd_claude_lite's model) is answered, and Will confirms the lock.

Then run 3345 as its spec says.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3345-*.md`.
- Before LOG1 itself (not the lock): task 3349 picks block B's best cheap model (provisionally glm-4.7), the Moonshot
  base URL needs checking, fd_codex needs a live probe and D42 confirmed, and a copied Codex home can rotate the
  operator's refresh token (a fresh `codex login` may be needed).
