+++
id = "find-7c5e23"
kind = "finding"
title = "S6: No Streaming — The 2026 Spinner Problem"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S6. No Streaming — The 2026 Spinner Problem"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S6. No Streaming — The 2026 Spinner Problem"
anchors = ["chat_inline.rs:1457-1491", "inline/primitives/streaming.rs", "dispatch_direct.rs:141-279", "dispatch_direct.rs:285-338", "runner/event_loop.rs:338-345", "run.rs:583-678", "roko run", "StreamingState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 5 open checklist fixes: S6.1 Connect `StreamingState` to dispatch path; S6.3 Forward Claude CLI stream-json to UI increm; S6.4 Use `"stream": true` for Anthropic API; S6.5 Print agent progress to terminal during pla; S6.6 Show real-time output for `roko run`

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S6. No Streaming — The 2026 Spinner Problem`

Some cited files are gone: `runner/event_loop.rs`.

How to verify: Check each open sub-item (S6.1, S6.3, S6.4, S6.5, S6.6). S6.4 likely closed (native Anthropic SSE streaming, backlog status 2026-09-03); check StreamingState wiring and plan-run progress output.
