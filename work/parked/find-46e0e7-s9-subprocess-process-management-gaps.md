+++
id = "find-46e0e7"
kind = "finding"
title = "S9: Subprocess & Process Management Gaps"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S9. Subprocess & Process Management Gaps"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S9. Subprocess & Process Management Gaps"
anchors = ["auth_detect.rs:99-104", "mcp/client.rs:187", "dispatch_direct.rs:246", "chat_inline.rs:1313-1327", "chat_inline.rs:1299-1413", "lib.rs:300-334", "unified.rs:48", "claude_cli_agent.rs:271-645"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Systemic audit finding (2026-04-28) with 7 open checklist fixes: S9.1 Add 3s timeout to auth detection probe; S9.2 Redirect MCP stderr to log file (not inheri; S9.3 Add timeout to Claude CLI dispatch; S9.4-5 Thread CancellationToken into chat dispat; S9.6 Store chain-watcher handle, kill on…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S9. Subprocess & Process Management Gaps`

How to verify: Check each open sub-item (S9.1, S9.2, S9.3, S9.4-5, S9.6, S9.7, S9.8-9). S9.8-9 overlaps backlog #381 (eprintln->tracing); check auth-detect probe timeout, MCP stderr redirection, Claude CLI dispatch timeout, chat cancellation.
