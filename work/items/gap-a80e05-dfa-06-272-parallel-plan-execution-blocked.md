+++
id = "gap-a80e05"
kind = "gap"
title = "DFA-06 #272: Parallel plan execution blocked (single workspace-global lock)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/locking"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Highest-value open work"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Highest-value open work"
anchors = ["preflight.rs workspace lock", "backlog #272", "#249", "#255", "#282"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Independent plan runs cannot coexist: one workspace lock serializes plan runs (repeated dogfood friction). #272 GraphEngine-native plan queues stayed blocked on #249 (worktree lifecycle), #255 (approval/control) and #282 (checkpoint completeness).

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Highest-value open work`
- `tmp/archive/dogfood-audit-2026-09-03/05-status-update-2026-09-01.md#5. #165/#37 -> #272 — Parallel Plan Queues + Multi-Process Locking`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Remaining Open Work`
- `tmp/archive/dogfood-2026-08-23/DOGFOOD-DEBRIEF.md#Issue 2: Workspace lock prevents parallel plan runs (LOW)`
- `tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F1: Runner workspace lock prevents parallel plan execution`

How to verify: Check status of #249/#255/#282 and try two plan runs on disjoint plans.
