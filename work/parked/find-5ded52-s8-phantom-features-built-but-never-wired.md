+++
id = "find-5ded52"
kind = "finding"
title = "S8: Phantom Features — Built But Never Wired"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S8. Phantom Features — Built But Never Wired"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S8. Phantom Features — Built But Never Wired"
anchors = ["episode_logger.rs:1086", "runtime_feedback/dreams.rs:20", "model_router.rs:1157", "prompt.rs:870", "contract.rs:442", "workflow_engine.rs:474", "main.rs:2080", "util.rs:230"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Systemic audit finding (2026-04-28) with 7 open checklist fixes: S8.1 Auto-trigger compaction on session start or; S8.2 Implement dream trigger consumer (inline at; S8.3 Call `LinUCBRouter::save()` from `CascadeRo; S8.5 Wire cumulative cost enforcement into tool ; S8.7 Share StateHub between TUI…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S8. Phantom Features — Built But Never Wired`

Warning: every file this item cites is gone (`runtime_feedback/dreams.rs`) — likely obsolete or moved.

How to verify: Check each open sub-item (S8.1, S8.2, S8.3, S8.5, S8.7, S8.8, S8.9). S8.2 (dream trigger) may be closed by resident dream scheduling; check LinUCB save, cumulative cost enforcement in tool context, share/StateHub co-location.
