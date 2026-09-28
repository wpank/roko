+++
id = "gap-4aebb8"
kind = "gap"
title = "DOCS-06 C1: Post-gate reflection uses deterministic synthesis instead of an LLM call"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/reflection"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C1. Post-Gate Reflection (actual LLM call)"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C1. Post-Gate Reflection (actual LLM call)"
anchors = ["post-gate reflection"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The reflection store exists but synthesizes patterns deterministically; the agent never actually reflects on gate failures (decision: wire a real provider call).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C1. Post-Gate Reflection (actual LLM call)`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Half-Implemented Features — FINISH ALL`
- `tmp/dogfood/2026-09-19-session.md#Fixes Applied This Session`
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`

A source claims this was fixed; confirm against current code before closing.

How to verify: Find reflection synthesis code and check for a provider call. / Locate the scaffolding named in the roadmap row and confirm no runtime caller.

Merged 2 mined candidates: m4-040, m5-073.
