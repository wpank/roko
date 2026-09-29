+++
id = "bug-65b021"
kind = "bug"
title = "[provider F029] StreamChunk::ToolCallDelta to StreamEvent conversion loses tool call arguments"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F029"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F029"
anchors = ["crates/roko-agent/src/streaming.rs:267", "crates/roko-cli/src/dispatch_v2.rs:2163"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'keyed_id.push_str(&arguments)' crates/roko-agent/src/streaming.rs"

[[verify]]
command = "cargo test -p roko-agent --lib streaming::"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8: StreamChunk no longer exists in roko-agent; the tool loop streams StreamEventKind directly, and only the legacy crates/roko-cli/src/dispatch_v2.rs:26 keeps a StreamChunk mapped from StreamEventKind. The OpenAI SSE parser keeps first-chunk arguments: crates/roko-agent/src/streaming.rs:255-274 appends any arguments that arrive with the id/name chunk to the ToolCallStart key after a SOH (0x01) separator, so the accumulator seeds the entry."
+++
When converting `StreamChunk::ToolCallDelta` to `StreamEvent`, if `id_delta` or `name_delta` is present the conversion emits `ToolCallStart` and loses `arguments_delta`. Only subsequent deltas (id/name absent) are mapped to `ToolCallDelta`. This means the first OpenAI SSE tool call chunk (which c...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F029`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Related to deferred P3-4/AR-1 streaming unification. Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: `StreamChunk::ToolCallDelta` to `StreamEvent` conversion loses tool call arguments

Verified 2026-09-28: fixed; see `[closed].evidence`.
