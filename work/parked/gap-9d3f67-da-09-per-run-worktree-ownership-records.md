+++
id = "gap-9d3f67"
kind = "gap"
title = "DA-09: Per-run worktree ownership records for safe cleanup"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/worktree"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/dev-audit/09-additional-live-run-findings.md#Worktree safety constraint"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Worktree safety constraint"
anchors = ["WorktreeManager", ".roko/worktrees/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Cleanup must distinguish Roko-owned attempt worktrees from operator/external ones and preserve dirty or timed-out patches; audit asks for per-run records (owner, run/attempt ID, heartbeat, base commit, dirty state, cleanup eligibility). No implementation status recorded.

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#Worktree safety constraint`

How to verify: Inspect worktree manager/registry for owner/heartbeat/eligibility fields and ownership-aware cleanup.
