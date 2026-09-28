+++
id = "find-256521"
kind = "finding"
title = "Hardcoded tmp/ and absolute paths in source"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Several `tmp/` and absolute (/Users/...) paths are hardcoded in source code and need fixing.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring`

How to verify: grep -rn '/Users/\|"tmp/' crates/ --include='*.rs' excluding tests.
