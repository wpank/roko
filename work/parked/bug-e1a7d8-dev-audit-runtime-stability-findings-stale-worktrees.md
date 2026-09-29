+++
id = "bug-e1a7d8"
kind = "bug"
title = "Dev audit: runtime stability findings (stale worktrees, conductor restarts, discarded snapshots, stale router)"
status = "parked"
triage = "unverified"
severity = "p1"
subsystem = ["roko-cli/runtime"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#1. Dev Audit (`tmp/dev-audit/`, 22 files)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#1. Dev Audit (`tmp/dev-audit/`, 22 files)"
anchors = ["roko-conductor", ".roko/learn/cascade-router.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Dev audit: 20 stale worktrees after SIGTERM, 143 conductor restarts, worktree peak 20 vs limit 8, state snapshot discarded on startup, corrupt events JSONL, router learned stale models (448 default fallbacks), MCP tools advertised but unavailable, ToolSelector legacy names.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#1. Dev Audit (`tmp/dev-audit/`, 22 files)`
- `tmp/dev-audit/22-fix-runbook.md`

How to verify: Re-check each finding against current main; the fix runbook lists P0-P3 steps.

Triage note 2026-09-28: This umbrella from the Runner-v2 era was not re-checked as a whole. Worktree, JSONL and Graph-parity sub-findings are tracked by bug-bdfb1d / gap-9d3f67, bug-dacaeb and q-1faa0c. The snapshot-discard and conductor-restart findings concerned runner/event_loop.rs, deleted in 6b5da8616. Still unchecked and tracked nowhere else: router learned stale models (448 default fallbacks), ToolSelector legacy names, and MCP tools advertised but unavailable. Split these out or re-check, then close this umbrella.
