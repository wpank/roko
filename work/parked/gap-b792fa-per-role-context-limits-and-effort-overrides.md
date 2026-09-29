+++
id = "gap-b792fa"
kind = "gap"
title = "Per-Role Context Limits and Effort Overrides"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/181-per-role-context-effort.md#181 — Per-Role Context Limits and Effort Overrides"
discovered_from = "audit:tmp/backlog/archive/181-per-role-context-effort.md#181 — Per-Role Context Limits and Effort Overrides"
anchors = ["crates/roko-core/src/config/agent.rs", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-cli/src/runner/types.rs", "RoleOverride", "AgentConfig", "AgentConfig::default()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
directly affects cost and quality on multi-role plan runs; lightweight roles at high effort waste tokens, complex roles at low effort miss edge cases. Mori had per-role context window limits (`role_context_k` with 27 entries) and per-role effort levels (`role_effort` with hardcoded tier defaults)…

Imported without verification from:
- `tmp/backlog/archive/181-per-role-context-effort.md#181 — Per-Role Context Limits and Effort Overrides`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-11`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A task with `role = "scribe"` uses `effort = "low"` when no explicit task-level or config override exists.; A task with `role = "implementer"` uses `effort = "high"` by default.; An explicit `[agent.roles.scribe] effort = "high"` in… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
