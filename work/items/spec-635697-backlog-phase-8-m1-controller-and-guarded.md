+++
id = "spec-635697"
kind = "spec"
title = "Backlog Phase 8 — M1 controller and guarded commit (S06)"
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

Phase 8: M1 controller and guarded commit (S06): 12 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK61: M1 controller: HarnessParams: the M1 knob surface with notch ladders, θ₀, an atomic handle and a… (+7 more)
- PK62: M1 controller: Controller: IDLE, SEARCH and HOLD with Thompson plus Ashby moves, dwell, rollback and… (+9 more)
- PK63: M1 controller: HomeostasisSink: one task resolution per chain, registered in the Graph feedback facade (+4 more)
- PK64: M4 deep audits: DP3: dispatch applies the verify depth the ladder sets, V0 to V4 (S05 task 12, part 2)
- PK65: M2 loop-liveness: Census measures L-M1, L-M3 and L-M4 from their receipts
- PK66: M3 self-model: After a low-confidence pass, request deeper verification from S05's ladder before… (+1 more)
- PK67: ViabilityBench proof: Replay R-H6: S06's controllers A0-A5 plus A3-gated and A3-mis (+2 more)
- PK68: ViabilityBench proof: P1 manifests: E-P1-live, E-P1-ext, E-fd-api, E-drift and E-T5-loo
- PK69: ViabilityBench proof: Run E-H5-live: H5-A0, H5-A1 and H5-A3 (BL4) (+7 more)
- PK70: M1 controller: B3 and B7: publish verify-depth floors and audit boosts to M4, with automatic audit…
- PK71: M1 controller: Force M1 to shadow when M2 demotes L-M1, and allow B4 'on' moves only for live loops (+9 more)
- PK72: M1 controller: roko learn commits and roko learn rollback for guarded stores

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
