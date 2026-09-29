+++
id = "find-012d91"
kind = "finding"
title = "Cross-Platform Message Audit Trail"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/416-cross-platform-message-audit-trail.md#416 — Cross-Platform Message Audit Trail"
discovered_from = "audit:tmp/backlog/archive/416-cross-platform-message-audit-trail.md#416 — Cross-Platform Message Audit Trail"
anchors = ["crates/roko-core/src/engram.rs", "crates/roko-fs/src/lib.rs", ".roko/engrams.jsonl", "crates/roko-core/src/wire_protocol.rs", "crates/roko-cli/src/chat_session.rs", "crates/roko-runtime/src/state_hub.rs", "crates/roko-serve/src/routes/", ".roko/messages.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
no unified view of agent-human messages across transports; conversation history is per-session and ephemeral. Every `ChatMessage` flowing through a `ChatBridge` (from #414) is a meaningful interaction between an agent and a human (or between agents in a group). Today these messages live only in…

Imported without verification from:
- `tmp/backlog/archive/416-cross-platform-message-audit-trail.md#416 — Cross-Platform Message Audit Trail`

Some cited files are gone: `.roko/engrams.jsonl`, `.roko/messages.jsonl`.

How to verify: Check: Every `ChatMessage` produces a Signal with kind `chat:message`; Messages in the same conversation share a `thread_id` across platforms; Per-conversation DAG linkage via Signal `parents` field [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
