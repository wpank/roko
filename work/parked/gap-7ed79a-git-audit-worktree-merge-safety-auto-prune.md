+++
id = "gap-7ed79a"
kind = "gap"
title = "Git audit: worktree/merge safety (auto-prune, index.lock cleanup, conflict replan, ff-only, merge-tree)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/worktree+merge"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#4. Git Audit (`tmp/git-audit/`, 11 files, 8 findings)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#4. Git Audit (`tmp/git-audit/`, 11 files, 8 findings)"
anchors = ["crates/roko-cli/src/runner/merge.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
G01 stale worktrees exceed limit, no auto-prune; G02 no pre-merge auto-commit; G03 no pre-spawn index.lock cleanup; G04 conflicts fail closed without replan; G05 always --no-ff; G06 no merge-tree feasibility check; G07 stuck flock; G08 worktree config isolation unverified.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#4. Git Audit (`tmp/git-audit/`, 11 files, 8 findings)`
- `tmp/git-audit/`

How to verify: Check each G-item's plan file against current merge/worktree code.
