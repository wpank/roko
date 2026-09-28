+++
id = "find-f2ef68"
kind = "finding"
title = "Disk pressure from target/ (106 GB) and worktree churn (143/day)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/resources"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/ROKO-STATE-2026-09-04.md#Disk Context"
discovered_from = "audit:tmp/nous-research/ROKO-STATE-2026-09-04.md#Disk Context"
anchors = ["roko doctor disk", "roko cache prune"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Disk was 97% full; target/ was 106 GB and 143 worktrees were created in one day, per dev-audit resource constraints. Check whether disk admission and cleanup now bound this.

Imported without verification from:
- `tmp/nous-research/ROKO-STATE-2026-09-04.md#Disk Context`

How to verify: Run roko doctor disk; du -sh target/.
