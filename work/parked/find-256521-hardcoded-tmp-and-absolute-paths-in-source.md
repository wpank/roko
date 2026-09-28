+++
id = "find-256521"
kind = "finding"
title = "Hardcoded tmp/ and absolute paths in source"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Several `tmp/` and absolute (/Users/...) paths are hardcoded in source code and need fixing.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#4. UX / Wiring`

How to verify: grep -rn '/Users/\|"tmp/' crates/ --include='*.rs' excluding tests.
