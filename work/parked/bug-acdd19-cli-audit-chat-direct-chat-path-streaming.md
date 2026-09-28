+++
id = "bug-acdd19"
kind = "bug"
title = "[cli-audit chat] Direct chat path streaming is cosmetic; API providers batch"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/chat"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/13-chat.md#Streaming: PARTIAL"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/13-chat.md#Streaming: PARTIAL"
anchors = ["crates/roko-cli/src/chat.rs", "ChatAgentSession", "send_turn_api", "backlog #322"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
agent chat Path A uses agent.run() batch API with a cosmetic `[streaming...]` label; legacy HTTP REPL polls; unified path streams only for Claude CLI, API providers batch via send_turn_api.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/13-chat.md#Streaming: PARTIAL`

How to verify: Check whether direct/API chat paths consume provider stream events token-by-token.
