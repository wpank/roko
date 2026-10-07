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
- 2026-10-04 (wave-15 follow-up, gap-1a8ee7): `q-55c52f` must be decided before this lock is
  taken. S09 closure test X4 needs LOG1 blocks run under the `no_gate`, `always_refine` and
  `always_escalate` policies; `experiments/log1.toml` defines none today (confirmed: no match
  for any of the three, or `refine_spec`). Adding them changes LOG1's cell count and planned
  spend, which the lock is meant to freeze — so whether to add them (and re-verify the billed
  total against BL1's cap), skip X4 for this run, or simulate the counterfactuals instead needs
  Will's answer before this item is actioned, not after.
- 2026-10-04 (wave-15 follow-up, bug-19ae56, work/bug-19ae56 not yet merged): a plan's own
  `[meta] skip_enrichment` now actually takes effect in `prompt_builder::from_task` (it was
  previously computed from unrelated inputs and silently ignored the task's real flag).
  `planemit.py` sets `skip_enrichment: bool = True` for every Roko bench arm it emits
  (`driver/planemit.py:313`, `123`), and S09 (`specs/S09-experiments.md:246`) requires every arm
  to share "the same `skip_enrichment`" for a valid comparison — so this is the intended design,
  not a bug to fix before locking. But it is a real change to what `roko run`'s prompt plans and
  every ViabilityBench Roko-arm plan actually send: confirmed in `from_task`
  (`prompt_builder.rs:213-261`) that `workspace_map`, `tasks_toml`, `workspace_context` and
  `plan_brief` all now go empty when `skip_enrichment` is true, where before this fix they did
  not reliably go empty even when the meta flag was set. Any prompt baseline, golden snapshot,
  or shakedown expectation captured before this lands may no longer match. Flagging only — no
  action needed on this lock item itself, since the behavior is correct; whoever takes the lock
  should know the bench prompts changed shape immediately before it, in case a baseline needs
  re-capturing.
