+++
id = "bug-a0cf94"
kind = "bug"
title = "[provider F173] TTFT measurement inconsistency between OpenAI backend and collect_stream_to_response"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/openai_compat_backend"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F173"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F173"
anchors = ["crates/roko-agent/src/openai_compat_backend.rs", "crates/roko-agent/src/tool_loop/mod.rs", "collect_stream_to_response"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`OpenAiCompatLlmBackend::stream_turn` records TTFT on first TextDelta, ReasoningDelta, or ToolCallDelta. `collect_stream_to_response` records TTFT only on TextDelta. The two paths produce different TTFT values for the same reasoning/tool-call responses.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F173`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Confirm in crates/roko-agent/src/openai_compat_backend.rs, crates/roko-agent/src/tool_loop/mod.rs whether still true: TTFT measurement inconsistency between OpenAI backend and `collect_stream_to_response`
