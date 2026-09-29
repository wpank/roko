+++
id = "spec-530c45"
kind = "spec"
title = "Idea: health-check timeout for the agent sidecar"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent-server"]
created = 2026-09-10
updated = 2026-09-28
source = ".roko/prd/ideas.md"
discovered_from = "audit:.roko/prd/ideas.md"
anchors = ["crates/roko-agent-server/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Captured idea (2026-09-10) without PRD: add a health check timeout to the per-agent sidecar (roko-agent-server).

Imported without verification from:
- `.roko/prd/ideas.md`

How to verify: grep roko-agent-server health route for timeouts.
