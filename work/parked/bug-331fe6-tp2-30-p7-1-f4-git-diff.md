+++
id = "bug-331fe6"
kind = "bug"
title = "TP2-30 P7.1: F4 Git diff not refreshed from the active attempt worktree"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix"
discovered_from = "audit:tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix"
anchors = ["TuiState.git_diff", "tui/fs_watch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Background git refresh does not publish the active attempt-worktree diff into connected state, leaving the Diff sub-tab empty/stale (the only item still N in the 09-04 matrix).

Imported without verification from:
- `tmp/tui-parity2/30-VERIFIED-CLAIM-MATRIX.md#Verified P0-P7 claim matrix`

How to verify: Run a task and watch F4 Diff.
