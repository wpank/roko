+++
id = "find-256c4a"
kind = "finding"
title = "Worktree and branch sprawl (8 orphaned agent worktrees, 10 feature worktrees, 30+ branches)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["repo-hygiene"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/ROKO-STATE-2026-09-04.md#Active Worktrees: 19"
discovered_from = "audit:tmp/nous-research/ROKO-STATE-2026-09-04.md#Active Worktrees: 19"
anchors = ["git worktree list"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
19 active worktrees incl. 8 orphaned .claude/worktrees/agent-* from crashed sessions, 10 feature worktrees and 30+ branches (dev-audit, backlog-batch, cli-audit). Owner decision needed; never delete worktrees/branches without approval.

Imported without verification from:
- `tmp/nous-research/ROKO-STATE-2026-09-04.md#Active Worktrees: 19`

How to verify: git worktree list; git branch --merged main; report only, do not delete.
