+++
id = "bug-07ce0f"
kind = "bug"
title = "DF-0826 F4: Shared target dir plus attempt worktrees can make cargo test the wrong crate version"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/worktree"]
created = 2026-08-26
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F4: Stale worktrees cause cargo to compile wrong crate version"
discovered_from = "audit:tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F4: Stale worktrees cause cargo to compile wrong crate version"
anchors = ["CARGO_TARGET_DIR", ".roko/worktrees/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
With attempt worktrees present, cargo resolved/compiled the worktree's crate version and tests added to the main tree were not found (0 matched) until worktrees were force-removed.

Imported without verification from:
- `tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F4: Stale worktrees cause cargo to compile wrong crate version`

How to verify: Reproduce with a live attempt worktree and a new test in main tree.
