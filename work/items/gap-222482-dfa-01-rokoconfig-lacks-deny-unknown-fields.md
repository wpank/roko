+++
id = "gap-222482"
kind = "gap"
title = "DFA-01: RokoConfig lacks deny_unknown_fields (typos and phantom sections silently accepted)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits"
anchors = ["RokoConfig", "deny_unknown_fields"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
CLI audit found RokoConfig does not reject unknown fields, so typos and phantom sections ([isfr], [[gate]]) are silently accepted even after the merge/unknown-key fixes.

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`

How to verify: Add a typo key to roko.toml and run roko config validate.
