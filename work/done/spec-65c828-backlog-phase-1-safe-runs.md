+++
id = "spec-65c828"
kind = "spec"
title = "Backlog Phase 1 — safe runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
subsystem = ["unknown"]
created = 2026-10-02
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "15335e2f9"
source = "tmp/backlog/2026-10-02-complete-and-wire/PACKAGES.md"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for id in gap-625195 gap-e00238 gap-d254a3 gap-198c9c gap-843aef gap-4b890c; do grep -rqw \"id = \\\"$id\\\"\" work/done/ || exit 1; done"

[closed]
at = 2026-10-27
commit = "730b43d91"
by = "work sweep 2026-10-27 (static check)"
evidence = "All 6 packages of this phase are closed in work/done/: PK01=gap-625195 (509e3e807), PK02=gap-e00238 (2724386ea), PK03=gap-d254a3 (059450273), PK04=gap-198c9c (e53136640), PK05=gap-843aef (d426c86d7), PK06=gap-4b890c (730b43d91), all closed 2026-10-02/03."
+++

## Problem

Phase 1: safe runs: 6 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK01: Failure paths: A blank provider answer fails as `empty_response` at the immune boundary, not as a… (+11 more)
- PK02: Failure paths: Immune denials leave provider health alone, and each attempt records provider health once (+7 more)
- PK03: Failure paths: A plan-branch conflict tells the next attempt what it conflicted with (+5 more)
- PK04: Operator control and guards: roko's own bash git guard lets git stash, clean, checkout and restore through (+9 more)
- PK05: Operator control and guards: Provider CLIs and MCP servers keep exported secrets whose names roko does not recognise (+7 more)
- PK06: Operator control and guards: --approval help and validate_graph_execution_options describe an approval mode that… (+4 more)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
