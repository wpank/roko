+++
id = "spec-99d417"
kind = "spec"
title = "Backlog Phase 2 — honest measurement"
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
command = "for id in gap-f548c1 gap-a0043b gap-cc5051 gap-f61823 gap-2b5d37 gap-08120e gap-9e3134; do grep -rqw \"id = \\\"$id\\\"\" work/done/ || exit 1; done"

[closed]
at = 2026-10-27
commit = "5bb643122"
by = "work sweep 2026-10-27 (static check)"
evidence = "All 7 packages for this phase are closed in work/done/: PK07=gap-f548c1 (509e3e807), PK08=gap-a0043b (2724386ea, also gap-3698cd), PK09=gap-cc5051 (509e3e807), PK10=gap-f61823 (2724386ea), PK11=gap-2b5d37 (2347ad858), PK12=gap-08120e (e53136640), PK13=gap-9e3134 (5bb643122), all closed 2026-10-02/03."
+++

## Problem

Phase 2: honest measurement: 7 work packages from the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (its `README.md`, `00-INDEX.md` and `PACKAGES.md`).

## Why it matters

The backlog's order is safe runs, then honest measurement, then the golden-path proof, then the cybernetic loops (S02, M2, M3, M4, M1), then domains, cleanup and deploy, with the papers alongside.

## Where

- PK07: Attempt ledger: OpenAI-compatible streaming requests ask for usage, so OpenAI spend reaches costs and… (+9 more)
- PK08: Attempt ledger: roko.toml's cheap-model rates match the dated price snapshot (+11 more)
- PK09: Decision records and census: S01 becomes the one schema: merge the S03/S06 addenda, the ladder source and today's… (+7 more)
- PK10: Decision records and census: Exposure and content-decision record types, and the telemetry report reads them (+9 more)
- PK11: Decision records and census: A frozen gate settlement writes no thresholds and makes no reflection call (+1 more)
- PK12: M3 self-model: Load the dated price snapshot in roko-core and price token usage from it
- PK13: Attempt ledger: Plan runs price model calls from the dated price snapshot (+1 more)

## Current state

Filed 2026-10-02 by the coordinator.

## Plan

Work the packages through `/work-batch`, following `BUILD-RULES.md`.

## Done when

Every package of this phase is done.

## Notes

- Held tasks and deferred decisions are not filed; see the backlog folder.
