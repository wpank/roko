+++
id = "gap-a6d9cd"
kind = "gap"
title = "HPA-04 §7.2/R2: AgentOutput streaming uses an in-band `\\x1eroko.stream.v1` prefix instead of typed events"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/dashboard"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.2. AgentOutput content encoding is opaque"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.2. AgentOutput content encoding is opaque"
anchors = ["STREAM_RECORD_PREFIX", "crates/roko-cli/src/runner/tui_bridge.rs", "parse_stream_record"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Text/reasoning/tool-call deltas are encoded inside AgentOutput.content behind STREAM_RECORD_PREFIX (defined in tui_bridge.rs, not a public contract); proposal: AgentTextDelta/ReasoningDelta/ToolCall/ToolResult DashboardEvent variants.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.2. AgentOutput content encoding is opaque`
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#R2 (High Priority): Structured streaming for AgentOutput`
- `tmp/hermes-product-audit/03-roko-streaming-architecture.md#7. The Semantic Stream Record Protocol (`roko.stream.v1`)`
- `tmp/hermes-product-audit/05-roko-demoability.md#4c. Structured `AgentOutput` Segments`

How to verify: grep STREAM_RECORD_PREFIX and DashboardEvent variants for typed deltas.
