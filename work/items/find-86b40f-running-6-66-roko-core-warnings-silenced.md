+++
id = "find-86b40f"
kind = "finding"
title = "[running #6] 66 roko-core warnings silenced with #[allow(dead_code)] instead of removal"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#6. ~~66 compiler warnings in roko-core~~ — DONE"
discovered_from = "audit:tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#6. ~~66 compiler warnings in roko-core~~ — DONE"
anchors = ["#[allow(dead_code)] in crates/roko-core/src/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The fix for 66 roko-core compiler warnings added #[allow(dead_code)] to all 66 legacy-compat items rather than deleting or wiring them.

Imported without verification from:
- `tmp/archive/running-audit-2026-09-21/06-REMAINING-WORK.md#6. ~~66 compiler warnings in roko-core~~ — DONE`

How to verify: grep -rn 'allow(dead_code)' crates/roko-core/src | wc -l; check which items have callers.
