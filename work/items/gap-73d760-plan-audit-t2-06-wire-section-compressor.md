+++
id = "gap-73d760"
kind = "gap"
title = "[plan-audit T2-06] Wire section compressor before dropping prompt sections"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-06: Wire section compressor"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-06: Wire section compressor"
anchors = ["compress_section", "SectionCompressor", "crates/roko-compose/src/system_prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
compress_section() exists but is not called when the assembled prompt exceeds budget; sections are dropped instead. Backlog #402.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T2-06: Wire section compressor`
- `tmp/archive/plan-audit-2026-09-23/10-token-optimization.md`

How to verify: grep compress_section call sites.
