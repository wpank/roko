+++
id = "spec-446a41"
kind = "spec"
title = "Backlog Phase 4 — loops re-closed (S02)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for id in gap-b5caf3 gap-aea13a gap-ac2611 gap-943046 gap-31c0e8 gap-e9218e gap-894977 gap-222d47; do grep -rqw \"id = \\\"$id\\\"\" work/done/ || exit 1; done"
+++

## Problem

Phase 4: loops re-closed (S02): 8 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK32: Loops re-closed: Retire the legacy holdout from Graph runs: its learning gate gates nothing (+11 more)
- PK33: Prompt composition: Delete roko-execution's duplicate prompt cache, which nothing reads (+13 more)
- PK34: M2 loop-liveness: Add the loop_audit module: LoopSpec, closed reason codes and the registry loader (+2 more)
- PK35: Loops re-closed: Section bandit: per-section posteriors and exclusion probabilities (+7 more)
- PK36: ViabilityBench proof: Shakedown: the Roko arms against the live-run defects, offline, before any paid Roko run (+2 more)
- PK37: ViabilityBench proof: Run LOG1 block A: roko_fixed on four cheap models, 960 billed runs (+1 more)
- PK38: Loops re-closed: Knowledge and playbook withhold arms: a withheld source stays out of the prompt and is… (+5 more)
- PK39: Prompt composition: Prompt-relevance canary: over two scripted runs, each prompt holds only its own task's… (+3 more)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.

Checked 2026-10-27: 2026-10-26 sweep: 7/8 packages done (PK32-PK36, PK38-PK39). PK37/gap-e9218e (LOG1 block A, 960 billed runs on roko_fixed) remains open; it is blocked by the same live-run/provider-billing issue as spec-567e52's Pilot B (commit 4c0a712da: 'roko_fixed is blocked by Cerebras 402s').
