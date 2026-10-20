+++
id = "spec-abbc62"
kind = "spec"
title = "Backlog Phase 6 — M3 calibrated self-model (S04)"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "9d5210330"
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-15
at_ts = "2026-10-15T15:46:18Z"
commit = "7b6a5af44"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Phase 6 roll-up: its Done when is that every package of the phase is done, and all six children (PK47-PK52) are in work/done (checked 2026-10-15 at 7b6a5af44, tmp/status-quo/2026-10-15/work-items/C2). The later M3 follow-ups gap-819df9, gap-cced54 and gap-e464e6 are not its children."
+++

## Problem

Phase 6: M3 calibrated self-model (S04): 6 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK47: M3 self-model: Claude CLI: re-price every model in modelUsage from the snapshot and keep… (+11 more)
- PK48: M3 self-model: Implement the replay baselines, including the production model ladder as it runs today (+5 more)
- PK49: M3 self-model: Compute the M3 economics report from policy replays (+7 more)
- PK50: ViabilityBench proof: Replays R-H4 and R-M3 on S04's prequential replay
- PK51: Specs a cheap model can execute: Export the spec features to the M3 self-model
- PK52: M3 self-model: Let an active self-model climb early after a failure, within the ladder's limits

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
