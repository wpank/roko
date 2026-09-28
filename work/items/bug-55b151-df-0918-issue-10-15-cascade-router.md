+++
id = "bug-55b151"
kind = "bug"
title = "DF-0918 ISSUE-10/15: Cascade router has no observations for providers used in plan execution"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-18
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
anchors = ["cascade-router.json", "roko learn router"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Cerebras/groq models showed 0 observations despite successful plan runs; the router loaded a stale snapshot and plan execution apparently does not record observations for all providers.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as "unavailable" (0 obs)`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-15: Cascade router shows 0 obs for kimi-k2.6 despite prior successful runs`

How to verify: Run a Graph-engine task on a non-default provider; check router observation counts.
