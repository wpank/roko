+++
id = "spec-fef7c5"
kind = "spec"
title = "Backlog Phase 3 — golden-path proof"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

Phase 3: golden-path proof: 18 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK14: Defaults that apply: A plan budget without max_turn_usd reserves a share per call, and any wait is logged (+8 more)
- PK15: Defaults that apply: One resolver for [runner] worktree_per_task, and roko run follows it (+4 more)
- PK16: Defaults that apply: Three live golden-path runs pass (pass^3), with an independent review of each merged diff (+1 more)
- PK17: Specs a cheap model can execute: Warn on unknown [[task]] keys: R3's top-level read_files never reached a prompt (+7 more)
- PK18: Specs a cheap model can execute: `plan validate --spec-quality --dynamic`: red-on-base proof inside Roko (+9 more)
- PK19: Specs a cheap model can execute: The generator prompt sizes tasks for their executor tier (+9 more)
- PK20: ViabilityBench proof: Run the direct loop's shell commands in a loopback-only network sandbox on macOS (+7 more)
- PK21: ViabilityBench proof: vb run gives a multi-model arm one endpoint and one proxy upstream per provider (+5 more)
- PK22: ViabilityBench proof: analysis/bootstrap.py: the paired bootstrap stratified by family and level (+5 more)
- PK23: ViabilityBench proof: Family F6 ts-result: the TypeScript transfer probe (+2 more)
- PK24: Specs a cheap model can execute: Refiner loop against a stub model: additive only, with sources (+3 more)
- PK25: ViabilityBench proof: Spec variants for the 48 H3 instances: precise, vague by D-v1, and the recoverability… (+3 more)
- PK26: ViabilityBench proof: run_official.py: score patches with the official SWE-bench harness (gold 60/60, empty…
- PK27: ViabilityBench proof: Confidence sequences, McNemar's test and CUPED in analysis/ (+5 more)
- PK28: M4 deep audits: Python: the vs.label schema, the audit estimators and the lottery replay (S05 task 2)
- PK29: Papers: Research paper §5 and Appendix D: reconcile with S09 v1.4 and the confirmed defaults,…
- PK30: ViabilityBench proof: Codex CLI runner for the fd_codex arm (+7 more)
- PK31: ViabilityBench proof: Run LOG1 block B: cheap_direct, 660 billed runs (+2 more)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
