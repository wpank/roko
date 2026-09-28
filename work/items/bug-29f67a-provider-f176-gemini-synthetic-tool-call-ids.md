+++
id = "bug-29f67a"
kind = "bug"
title = "[provider F176] Gemini synthetic tool call IDs use SystemTime::now() — collision possible"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F176"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F176"
anchors = ["crates/roko-agent/src/tool_loop/backends/gemini_native.rs", "SystemTime::now()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Gemini function call IDs are generated as `format!("gemini-{name}-{ts_nanos}")` using `SystemTime::now()`. Calls made within the same nanosecond (or on clocks with low resolution) produce the same ID.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F176`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/backends/gemini_native.rs whether still true: Gemini synthetic tool call IDs use `SystemTime::now()` — collision possible
