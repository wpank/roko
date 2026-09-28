+++
id = "find-14bcc2"
kind = "finding"
title = "[cli-audit docs] Documentation drift: sidecar route count and test count"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["docs"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Documentation Drift"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Documentation Drift"
anchors = ["CLAUDE.md", "crates/roko-agent-server/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Agent sidecar has 14 routes (docs said 13); test count 11,948 at audit vs CLAUDE.md 10,300+. Minor doc drift remaining after other drift items were fixed.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Documentation Drift`
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Overview`

How to verify: Count roko-agent-server routes; compare against docs/CLAUDE.md claims.
