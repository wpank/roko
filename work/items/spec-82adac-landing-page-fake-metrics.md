+++
id = "spec-82adac"
kind = "spec"
title = "Landing page fake metrics"
status = "open"
triage = "unverified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/21-landing-page-fake-metrics.md#21 — Landing page fake metrics"
discovered_from = "audit:tmp/backlog/21-landing-page-fake-metrics.md#21 — Landing page fake metrics"
anchors = ["nunchi/roko"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Reputational risk: fabricated traction numbers are visible to investors and partners. During an April 2026 dogfood audit, the landing page was found to display hardcoded placeholder metrics that were inserted during design scaffolding and never replaced with real data. These numbers have never…

Imported without verification from:
- `tmp/backlog/21-landing-page-fake-metrics.md#21 — Landing page fake metrics`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-PRD-1 (Subsystem: Product / Landing)`
- `tmp/archive/08-15-26/MASTER-TASKS.md#1. Demo & Pitch (P2-4 landing page updates)`

Warning: every file this item cites is gone (`nunchi/roko`) — likely obsolete or moved.

How to verify: Check: Loading `nunchi.network` shows no hardcoded numeric traction metrics. If counters exist, they are either absent, replaced with honest placeholder text, or connected to a real data source.; A case-insensitive search for "engram" across the… [evidence: CONSOLIDATED P1-PRD-1: open; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Triage note 2026-09-28: Cannot verify from this repo. Per tmp/backlog/21-landing-page-fake-metrics.md the nunchi.network landing page lives in the separate `nunchi-dashboard` repo; the in-repo landing pages (demo/demo-app/src/pages/Landing.tsx, apps/portal/src) contain none of the cited numbers. Check the live site or nunchi-dashboard. Subsystem `roko-cli/runner` is wrong (external product site).
