+++
id = "bug-acdec9"
kind = "bug"
title = "[cli-audit inconsistencies] Fly deploy hardcodes app name `roko-agent` and region `iad`"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/deploy"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies"
anchors = ["roko deploy fly", "backlog #321"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`roko deploy fly` hardcodes app name and region. SUMMARY lists OPEN, but checklist marks #321 (deploy preview/preflight/Fly/Docker controls) done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies`
- `tmp/archive/cli-audit-2026-09-21/12-deploy-daemon-worker.md`
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Independent command/domain lanes`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep deploy code for literal "roko-agent" and "iad"; confirm flags/config override them.
