+++
id = "gap-d7956c"
kind = "gap"
title = "HPA-04 §7.4/R5: Late-joining clients cannot see in-progress agent output"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/dashboard"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.4. No \"active runs\" event at connect time"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.4. No \"active runs\" event at connect time"
anchors = ["AgentState", "crates/roko-core/src/dashboard_snapshot.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Snapshot AgentState keeps only last_output_line, so a client connecting mid-run sees an active agent with no text until the next delta; proposal: buffer last N lines per agent or add GET /api/agents/:id/output.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.4. No "active runs" event at connect time`
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#R5 (Medium Priority): Buffer per-agent output in DashboardSnapshot`

How to verify: Inspect AgentState fields for an output buffer.
