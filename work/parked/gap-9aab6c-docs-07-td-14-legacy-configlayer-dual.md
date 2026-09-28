+++
id = "gap-9aab6c"
kind = "gap"
title = "DOCS-07 TD-14: Legacy ConfigLayer dual-loader (~1,500 LOC) alongside E42 config"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-14: ConfigLayer Legacy Dual-Loader (~1,500 LOC)"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-14: ConfigLayer Legacy Dual-Loader (~1,500 LOC)"
anchors = ["ConfigLayer"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Legacy config loading code coexists with the E42 config system, giving two paths with potential inconsistency (remove after migration verification).

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-14: ConfigLayer Legacy Dual-Loader (~1,500 LOC)`
- `tmp/dogfood/2026-09-19-session.md#2026-09-20 Continuation`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep ConfigLayer users.
