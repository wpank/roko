+++
id = "gap-9b0a03"
kind = "gap"
title = "DOCS-03: Verify-then-clean stub matches in roko-std (~52) and roko-dreams (~31)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-std"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Potential Dead Code — VERIFY THEN CLEAN"
discovered_from = "audit:tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Potential Dead Code — VERIFY THEN CLEAN"
anchors = ["crates/roko-std/", "crates/roko-dreams/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Audit flagged ~52 todo/stub matches in roko-std and ~31 in roko-dreams to verify as legitimate fallbacks or remove.

Imported without verification from:
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Potential Dead Code — VERIFY THEN CLEAN`

How to verify: grep -n 'todo!\|unimplemented!\|stub' in both crates.
