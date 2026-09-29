+++
id = "gap-c135ba"
kind = "gap"
title = "WF-09/11 P4: Remaining mori plan-format gaps (fixture_keys, dependency_tags, deferred failures, workspace map)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/task_parser"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P4 — Mori Gaps (lower priority)"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P4 — Mori Gaps (lower priority)"
anchors = ["crates/roko-cli/src/task_parser.rs", "TaskDef"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Not done: per-task fixture_keys, dependency_tags, a deferred-failures.toml log for non-blocking gate failures, and workspace-map.md generation via doctor/plan validate.

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P4 — Mori Gaps (lower priority)`
- `tmp/workflow-audit/09-MORI-GAPS.md#Summary: Priority Actions`

How to verify: grep fixture_keys/dependency_tags in task_parser.rs.
