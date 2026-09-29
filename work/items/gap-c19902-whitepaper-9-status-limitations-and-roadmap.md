+++
id = "gap-c19902"
kind = "gap"
title = "Whitepaper §9 Status, limitations and roadmap"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (§1 scorecard; §2 P0-P3)"
anchors = ["docs/whitepaper/09-status-and-roadmap.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-35a614", "gap-af0b57"], blocks = [], related = ["gap-0191eb", "gap-d1d92c", "dec-e70592"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/09-status-and-roadmap.md && grep -q 'appendix-status-matrix' docs/whitepaper/09-status-and-roadmap.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/09-status-and-roadmap.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Whitepaper §9 status, limitations and roadmap merged in ead159496; verify passes."
+++

## Problem

The whitepaper must close its technical part with a plain account of four things: what works today, what does not,
what the evidence cannot show, and in what order the gaps close.

## Why it matters

It is the counterweight to the ideal-state voice of §3–§6, and the place where a reader checks whether the thesis is
credible.

## Where

- `docs/whitepaper/09-status-and-roadmap.md` (new). gap-0191eb creates it as a stub; create it here if that item
  hasn't landed.
- gap-d1d92c adds Figure 3.

## Current state

Checked at `41c7ffbd6`. tldr/05 §1 scores ten vision claims (V1–V10) at `d9e79e9d8`:
- none is fully met;
- V3 is MISSING, and V7 is UNPROVEN.

Its proposals, P0–P3, are now epics E2–E17 in `work/` (PLAN §3). Some rows have moved since `d9e79e9d8`; gap-35a614
lists them.

## Plan

1. **Status:** the tag counts from the appendix, at the appendix's commit, and the ten claims re-scored against it.
   Link to `appendix-status-matrix.md`.
2. **Limitations,** each with its evidence or item:
   - there is one harness;
   - the field evidence is observational;
   - every portal task ran on one model;
   - supervising sessions did much of the work;
   - Roko is code-first in practice;
   - there are safety gaps: agents inherit provider keys (bug-7d7200; the fix is on `fix/hermetic-child-env`), and
     there is no OS sandbox;
   - no learning loop has a measured benefit.
3. **Roadmap:** the goals in order (truth, golden path, proof, cybernetic core), each with its epic ids and exit
   check. Give the order, not dates.
4. **Parking:** the subsystems tldr/05 §3 proposes to park, marked as proposed.

## Done when

- [x] Every limitation names its evidence or item.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **PLAN §4's time estimates are internal.** Publish them only if the author agrees.
- Lane `paper`; no hot files.
- **2026-09-29 (wk-wp-s9):** drafted on `work/gap-c19902` (status `draft`, 655 words); the roadmap update is
  `2d8e9cb5c`, and SR11 is added to the README in `98f9b60d5`. The verify cannot pass on this branch alone, because
  `tools/paperlint.py` (gap-af0b57) and the appendix (gap-35a614) are not in its history. A simulated merge into the
  main checkout's branch at `12df83425`, with paperlint from `work/gap-af0b57` at `764bb5f2c`, passes the full
  verify ("1 file clean"). Close this item after merging, once paperlint has landed. The counts match the merged
  matrix: 71 rows at `a17d4dadd`.
