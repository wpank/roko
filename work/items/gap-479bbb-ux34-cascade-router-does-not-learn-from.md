+++
id = "gap-479bbb"
kind = "gap"
title = "UX34: cascade router does not learn from manual model/backend overrides (force_backend)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#7. Deferred / Blocked (UX34)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#7. Deferred / Blocked (UX34)"
anchors = ["CascadeRouter", "force_backend"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Manual overrides (`--force-backend`/model override) are not recorded as learning signal for CascadeRouter. Also appears as MASTER-IMPLEMENTATION-PLAN 5.5.1 (RouterOverride event -> FeedbackService -> RouterSink) and CLAUDE.md remaining-work #17.

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#7. Deferred / Blocked (UX34)`
- `tmp/archive/08-17-26/subsystem-audits/MASTER-IMPLEMENTATION-PLAN.md#5.5 Cascade Learning UX (5.5.1 override recording)`

How to verify: Check whether an override path emits a router feedback/override event consumed by CascadeRouter persistence (.roko/learn/cascade-router.json). Backlog #90 (UX34 override isolation: overrides must not poison bandit stats) is archived as closed; this item is the remaining "learn from overrides" half.
