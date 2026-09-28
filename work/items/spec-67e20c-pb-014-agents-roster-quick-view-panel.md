+++
id = "spec-67e20c"
kind = "spec"
title = "PB-014: Agents: Roster + quick-view panel"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
source = "tmp/portal-backlog/PB-014-agents-roster.md#PB-014"
discovered_from = "audit:tmp/portal-backlog/PB-014-agents-roster.md#PB-014"
anchors = ["apps/portal", "src/app/agents/page.tsx", "tmp/portal/spec/06-AGENTS.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Agents: Roster + quick-view panel. Build the primary agents page: a split-panel view (32% roster | 68% quick-view) showing all agents with real-time status, role filtering, and a detail panel for the selected agent.

Imported without verification from:
- `tmp/portal-backlog/PB-014-agents-roster.md#PB-014`

How to verify: Check portal app for this feature (6 acceptance criteria, e.g. Route: `src/app/agents/page.tsx`; Left Panel — Agent Roster (32%):); cross-check plans/portal-programme/* and existing web app dirs.
