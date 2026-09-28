+++
id = "gap-b31440"
kind = "gap"
title = "[provider F175] GeminiNativeBackend::stream_turn has no TTFT timeout"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F175"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F175"
anchors = ["crates/roko-agent/src/tool_loop/backends/gemini_native.rs", "GeminiNativeBackend::stream_turn"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unlike `OpenAiCompatLlmBackend`, the Gemini streaming backend applies no timeout to the first chunk. A stalled Gemini connection blocks until the total `request_timeout` fires.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F175`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/backends/gemini_native.rs whether still true: `GeminiNativeBackend::stream_turn` has no TTFT timeout
