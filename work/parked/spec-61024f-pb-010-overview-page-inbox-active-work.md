+++
id = "spec-61024f"
kind = "spec"
title = "PB-010: Overview page (inbox, active work, vitals, activity)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
source = "tmp/portal-backlog/PB-010-overview.md#PB-010"
discovered_from = "audit:tmp/portal-backlog/PB-010-overview.md#PB-010"
anchors = ["apps/portal", "DELETE /api/inbox/{id}", "src/app/page.tsx", "tmp/portal/spec/04-OVERVIEW.md", "NeedsAttention", "ActiveWork", "VitalsPanel", "RecentActivity"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Overview page (inbox, active work, vitals, activity). The root page (`/`) — single-glance workspace status that surfaces what needs attention. This is the first thing users see and the most important demo surface.

Imported without verification from:
- `tmp/portal-backlog/PB-010-overview.md#PB-010`

How to verify: Check portal app for this feature (9 acceptance criteria, e.g. Route: `src/app/page.tsx` (server component shell) + client components; Needs Attention (Inbox): Priority-ordered alerts (max 5 visible)); cross-check plans/portal-programme/* and existing web app dirs.
