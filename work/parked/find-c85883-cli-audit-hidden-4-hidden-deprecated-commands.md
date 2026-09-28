+++
id = "find-c85883"
kind = "finding"
title = "[cli-audit hidden] 4 hidden deprecated commands (layer-check, tune, dev, up) still dispatch"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/main"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented"
anchors = ["hide = true commands in main.rs", "backlog #363"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Hidden deprecated commands print warnings but still execute (layer-check->doctor, tune->learn, dev/up->serve). #363 CLI surface/legacy wrapper exit claimed done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented`
- `tmp/archive/cli-audit-2026-09-21/00-main-structure.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep main.rs for hidden LayerCheck/Tune/Dev/Up variants.
