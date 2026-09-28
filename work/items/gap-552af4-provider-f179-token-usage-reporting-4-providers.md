+++
id = "gap-552af4"
kind = "gap"
title = "[provider F179] Token usage reporting: 4 providers always report zero tokens"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F179"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F179"
anchors = ["CLI provider adapters"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Claude CLI, Cursor CLI, Gemini CLI, and OpenClaw report zero tokens per turn. These providers either do not surface token counts or use estimation heuristics that are not wired into the usage field.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F179`
- `tmp/archive/provider-audit/28-parity-matrix.md`

How to verify: Confirm in CLI provider adapters whether still true: Token usage reporting: 4 providers always report zero tokens
