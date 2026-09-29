+++
id = "gap-7fba5a"
kind = "gap"
title = "TP1-MX.3/MX.9: Anthropic API adapter never streams; several providers are batch-only"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)"
anchors = ["crates/roko-agent/src/provider/", "stream: true"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The Anthropic Messages adapter never sends stream:true, so output appears only at completion; Gemini/Cerebras/Ollama are batch-only and Claude CLI is burst-y. No backlog spec covers provider streaming parity.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#Medium-term items (from v1 audit)`
- `tmp/archive/dogfood-audit-2026-09-03/03-dogfood-runbook.md#Provider streaming behavior varies`
- `tmp/archive/dogfood-audit-2026-09-03/04-observability-reference.md#Provider streaming quality affects observability`
- `tmp/archive/ux-audit-2026-09-21/18-backlog-cross-reference.md#6. Enable Anthropic API streaming`

How to verify: grep the Anthropic provider request builder for stream.
