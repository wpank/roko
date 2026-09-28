+++
id = "gap-5fdb97"
kind = "gap"
title = "DOCS-06 C3: T0 reflex store lacks runtime sleep/wake loop and trigger registration"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/reflexes"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C3. T0 Reflex Store runtime loop"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C3. T0 Reflex Store runtime loop"
anchors = ["roko learn reflexes", "reflex store"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Persisted reflex execution landed but the runtime sleep/wake loop, trigger registration and cortical-state serialization are missing, so T0 reflexes do not fire automatically.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C3. T0 Reflex Store runtime loop`
- `tmp/dogfood/2026-09-18-session.md#Dogfood Round 3 (final round)`
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`

A source claims this was fixed; confirm against current code before closing.

How to verify: Check whether any runtime loop registers reflex triggers; run `roko learn reflexes`. / Locate the scaffolding named in the roadmap row and confirm no runtime caller.

Merged 2 mined candidates: m4-042, m5-070.
