+++
id = "bug-55b151"
kind = "bug"
title = "Cascade router has no observations for providers used in plan execution"
status = "open"
triage = "unverified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-18
updated = 2026-09-28
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
anchors = ["cascade-router.json", "roko learn router", "crates/roko-cli/src/graph_execution/feedback.rs:358"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Cerebras/groq models showed 0 observations despite successful plan runs; the router loaded a stale snapshot and plan execution apparently does not record observations for all providers.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as "unavailable" (0 obs)`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-15: Cascade router shows 0 obs for kimi-k2.6 despite prior successful runs`

How to verify: Run a Graph-engine task on a non-default provider; check router observation counts.

Check on 2026-09-28 was inconclusive: Graph plan execution now has its own routing feedback sink (crates/roko-cli/src/graph_execution/feedback.rs:58, RoutingSink at :358-363, a save at :433; file touched by 725f21e05), so 'plan execution records no router observations' may no longer hold. Observations are keyed by the loaded slug list (record_confidence_outcome returns bool, cascade_router.rs:1455) and could still be dropped for cerebras/groq/kimi slugs missing from that list. Deciding needs the item's live check: run a Graph-engine task on a non-default provider, then compare `roko learn router` observation counts.
