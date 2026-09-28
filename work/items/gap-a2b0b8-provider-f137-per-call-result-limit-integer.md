+++
id = "gap-a2b0b8"
kind = "gap"
title = "[provider F137] Per-call result limit integer division yields zero for empty vec"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F137"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F137"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The per-call result limit calculation uses integer division that can yield zero when the input vector is empty. Zero-capacity results trigger an assertion or produce unexpected behavior.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F137`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: Per-call result limit integer division yields zero for empty vec
