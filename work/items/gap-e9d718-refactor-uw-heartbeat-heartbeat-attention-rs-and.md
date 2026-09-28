+++
id = "gap-e9d718"
kind = "gap"
title = "[refactor UW-heartbeat] heartbeat_attention.rs and heartbeat_probes.rs remain unwired (deferred as experimental)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-runtime/heartbeat"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#unwired-code-02-unwired-codemd--all-complete-as-of-2026-09-14"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#unwired-code-02-unwired-codemd--all-complete-as-of-2026-09-14"
anchors = ["heartbeat_attention.rs", "heartbeat_probes.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unwired-code audit deferred heartbeat_attention.rs (2,146 lines) and heartbeat_probes.rs (1,545 lines) as still experimental with only a soft runner dependency; neither wired nor deleted.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/COMPLETION-STATUS.md#unwired-code-02-unwired-codemd--all-complete-as-of-2026-09-14`
- `tmp/archive/refactoring-audit-2026-09-21/02-unwired-code.md`

How to verify: Locate both files, check for production callers; decide wire vs delete.
