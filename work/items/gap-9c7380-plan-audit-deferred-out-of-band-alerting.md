+++
id = "gap-9c7380"
kind = "gap"
title = "[plan-audit deferred] Out-of-band alerting (email/Slack/webhook) for unsupervised runs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/notifications"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap"
anchors = ["Inbox webhook alerting (cybernetic P4-15)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Neither mori nor roko alerts operators out-of-band on failures/stalls; needed for unsupervised runs.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/00-INDEX.md#Gaps Deferred to Product Roadmap`
- `tmp/archive/plan-audit-2026-09-23/01-GAP-MATRIX.md`

How to verify: Check for configurable Slack/email/webhook alert sinks on plan failure.
