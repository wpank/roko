+++
id = "gap-d30038"
kind = "gap"
title = "Native Anthropic SSE Streaming Foundation"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/provider"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/192-native-agent-telemetry.md#192 — Native Anthropic SSE Streaming Foundation"
discovered_from = "audit:tmp/backlog/archive/192-native-agent-telemetry.md#192 — Native Anthropic SSE Streaming Foundation"
anchors = ["crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:685", "crates/roko-agent/src/provider/anthropic_api/stream.rs"]
links = { depends_on = [], blocks = [], related = ["bug-7ef619", "gap-7fba5a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib anthropic_api::stream"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8: the native Anthropic backend streams. It implements LlmBackend::stream_turn (crates/roko-agent/src/provider/anthropic_api/tool_loop.rs:685), sends stream: true (:473), and decodes SSE in provider/anthropic_api/stream.rs, with fixture tests for text, parallel tool calls (:998), thinking deltas (:1166), and mid-stream errors and truncation (:1110, :1240). ToolLoop consumes stream_turn (crates/roko-agent/src/tool_loop/mod.rs:1430). tool_loop.rs also has an uncommitted 8-line rate-limit classification change from a concurrent session, unrelated to streaming. gap-7fba5a, which says the Anthropic adapter never streams, looks stale."
+++
Native Anthropic SSE Streaming Foundation

Imported without verification from:
- `tmp/backlog/archive/192-native-agent-telemetry.md#192 — Native Anthropic SSE Streaming Foundation`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.7 Enable Anthropic API streaming`

Some cited files are gone: `tmp/provider-audit/07-streaming-protocol.md`.

How to verify: Check: First text/reasoning/tool event is emitted before response completion.; Parallel tool JSON reconstructs exactly and produces one start/end pair per tool.; Usage equals the fixture totals without duplication. [evidence: own status: Done (2026-09-03) — native Anthropic SSE streaming implemented; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): L | 7 |; (newer evidence overrides own status…]

Verified 2026-09-28: fixed; see `[closed].evidence`.
