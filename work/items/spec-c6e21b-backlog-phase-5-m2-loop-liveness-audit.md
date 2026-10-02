+++
id = "spec-c6e21b"
kind = "spec"
title = "Backlog Phase 5 — M2 loop-liveness audit (S03)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

Phase 5: M2 loop-liveness audit (S03): 7 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK40: M2 loop-liveness: Re-anchor S03 §3 and §7 A1 at HEAD: five census rows have moved (+8 more)
- PK41: Loops re-closed: `roko learn telemetry check --srm` compares each randomised layer's arm shares with…
- PK42: M2 loop-liveness: Loop state machine with the six false-demotion guards (+7 more)
- PK43: M2 loop-liveness: LoopAuditor facade and the [learning.audit] config section (+5 more)
- PK44: M2 loop-liveness: DryRunPlanner over Dispatcher::plan (probes P4 and P5) (+6 more)
- PK45: ViabilityBench proof: requires_live: refuse a live manifest whose loops are not LIVE at its harness_sha (+1 more)
- PK46: ViabilityBench proof: Run E-H7-live: the S1 stream and the harmful stream (BL5)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
