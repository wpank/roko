+++
id = "gap-be2f4b"
kind = "gap"
title = "[plan-audit T1-02] Add `exclusive_files` to TaskDef"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/task_parser"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-02: Add `exclusive_files` to TaskDef"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-02: Add `exclusive_files` to TaskDef"
anchors = ["crates/roko-cli/src/task_parser.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Parse exclusive_files: bool from tasks.toml and honor it in the wave file-conflict check (depends on T0-04).

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-02: Add `exclusive_files` to TaskDef`

How to verify: grep exclusive_files.
