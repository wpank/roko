+++
id = "find-afc610"
kind = "finding"
title = "Update CLAUDE.md documentation drift"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["docs"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.11 Update CLAUDE.md documentation drift"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.11 Update CLAUDE.md documentation drift"
anchors = ["CLAUDE.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Route count (~317 vs ~378), sidecar routes (13 vs 14), undocumented CLI commands (`plan pause/resume/cancel/retry/status/queue`, `knowledge export/import/backfill-hdc`, `job match`), and environment variable documentation.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#2.11 Update CLAUDE.md documentation drift`

How to verify: Source: CLI audit (documentation drift section). Check `CLAUDE.md` for: Route count (~317 vs ~378), sidecar routes (13 vs 14), undocumented CLI commands (`plan pause/resume/cancel/retry/status/queue`, `knowledge…
