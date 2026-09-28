+++
id = "find-d723f3"
kind = "finding"
title = "[provider F174] FinishReason Debug-formatted in StreamChunk → StreamEvent conversion"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F174"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F174"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs", "FinishReason", "StreamChunk → StreamEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`StreamChunk::Done(FinishReason)` is converted to `StreamEvent::Done { finish_reason: format!("{:?}") }`. Debug format produces `"Stop"` instead of wire string `"stop"`, `"ToolCalls"` instead of `"tool_calls"`. Consumers checking exact strings will not match.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F174`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Related to deferred P3-4/AR-1 streaming unification. Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: `FinishReason` Debug-formatted in `StreamChunk → StreamEvent` conversion
