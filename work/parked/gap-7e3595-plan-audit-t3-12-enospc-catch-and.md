+++
id = "gap-7e3595"
kind = "gap"
title = "[plan-audit T3-12] ENOSPC catch-and-reclaim"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/worktree"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)"
anchors = ["worktree/target write paths"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
No handling for 'No space left on device'; mori catches it at three sites and retries after reclaim.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#Tier 3: Polish (Nice-to-have for full mori parity)`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

Warning: every file this item cites is gone (`worktree/target`) — likely obsolete or moved.

How to verify: grep -rn 'ENOSPC\|StorageFull\|No space left' crates/
