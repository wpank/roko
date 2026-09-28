+++
id = "bug-9b1bed"
kind = "bug"
title = "DA-bak-17: Daimon state not updated by plan runs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-daimon"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#5. Daimon State Stale"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#5. Daimon State Stale"
anchors = [".roko/state/daimon.json", "DaimonState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
.roko/state/daimon.json was last updated 2026-05-05 (tick_count 12) despite many runs; proposal: update daimon at least on task completion. Needs re-check for the Graph engine.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#5. Daimon State Stale`

How to verify: Run a plan and check daimon.json mtime/tick_count change.
