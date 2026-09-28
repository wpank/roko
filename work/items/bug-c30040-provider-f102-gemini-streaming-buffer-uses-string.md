+++
id = "bug-c30040"
kind = "bug"
title = "[provider F102] Gemini streaming buffer uses String::from_utf8_lossy — multi-byte UTF-8 split across chunks may corrupt"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F102"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F102"
anchors = ["crates/roko-agent/src/tool_loop/backends/gemini_native.rs", "String::from_utf8_lossy"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`GeminiNativeBackend::stream_turn` accumulates SSE chunks in a `String` via `from_utf8_lossy`. If a multi-byte Unicode character is split across two TCP chunks, the first chunk's partial bytes become U+FFFD replacement characters.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F102`
- `tmp/archive/provider-audit/15-streaming.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/backends/gemini_native.rs whether still true: Gemini streaming buffer uses `String::from_utf8_lossy` — multi-byte UTF-8 split across chunks may corrupt
