+++
id = "find-3d0d97"
kind = "finding"
title = "Fly deploy hardcoded values"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/deploy"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Fly deploy hardcodes app name `roko-agent` and region `iad`. Make configurable.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values`

How to verify: Source: CLI audit report 12. Check the described code path for: Fly deploy hardcodes app name `roko-agent` and region `iad`. Make configurable.
