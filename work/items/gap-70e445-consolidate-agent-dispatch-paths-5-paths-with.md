+++
id = "gap-70e445"
kind = "gap"
title = "Consolidate agent dispatch paths (5 paths with safety/enrichment divergence; silent delegations)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)"
discovered_from = "audit:docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)"
anchors = ["crates/roko-cli/src/dispatch_v2.rs", "crates/roko-agent/src/dispatcher/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Backlog #61: five agent dispatch paths diverge on safety and enrichment; the engine audit also counted 18 silent delegation instances in CLI dispatch.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#2.3 P2 -- Medium (Selected)`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#5. Engine Audit`

How to verify: Enumerate create_agent/dispatch entry points and compare safety wrapper + enrichment application.
