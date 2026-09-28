+++
id = "find-99cbb9"
kind = "finding"
title = "S1: The Dispatch Layer Is a Thin Pipe, Not an Agent Session"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/chat"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S1. The Dispatch Layer Is a Thin Pipe, Not an Agent Session"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S1. The Dispatch Layer Is a Thin Pipe, Not an Agent Session"
anchors = ["dispatch_direct.rs:293,378", "chat_inline.rs:2134", "dispatch_direct.rs", "run.rs:1124", "run.rs:1271", "unified.rs:29-42", "roko run", "SystemPromptBuilder"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 10 open checklist fixes: S1.1 Build system prompt at session start (works; S1.2 Wire `session.system_message` into dispatch; S1.3 Add tool definitions to Anthropic API and O; S1.4 Send conversation history with each turn; S1.5 Include workspace path, git…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S1. The Dispatch Layer Is a Thin Pipe, Not an Agent Session`

How to verify: Check each open sub-item (S1.1, S1.2, S1.3, S1.4, S1.5, S1.6, S1.7, S1.8, S1.9, S1.10). Chat path. Cross-check backlog #322 (chat direct-path parity, archived done 2026-09-07) and CONSOLIDATED P2-CHAT-1 (done).
