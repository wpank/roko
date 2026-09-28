+++
id = "find-ad535d"
kind = "finding"
title = "Replace Scattered 120s Timeout Defaults with Named TimeoutPolicy"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/214-timeout-policy-object.md#214 — Replace Scattered 120s Timeout Defaults with Named TimeoutPolicy"
discovered_from = "audit:tmp/backlog/archive/214-timeout-policy-object.md#214 — Replace Scattered 120s Timeout Defaults with Named TimeoutPolicy"
anchors = ["crates/roko-acp/src/builtin_tools.rs:195", "crates/roko-acp/src/builtin_tools.rs:805", "crates/roko-acp/src/bridge_events.rs:554", "crates/roko-graph/src/cells/task_executor.rs:82", "crates/roko-std/src/tool/builtin/bash.rs:40,48", "crates/roko-agent/src/cursor_agent.rs:204", "roko.toml", "crates/roko-core/src/config.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
hardcoded timeouts cause silent failures on slow networks and make timeout configuration ineffective. Multiple provider adapters and tool implementations contain `unwrap_or(120_000)` or equivalent hardcoded 120-second timeout values. These scattered defaults mean that configuring a different…

Imported without verification from:
- `tmp/backlog/archive/214-timeout-policy-object.md#214 — Replace Scattered 120s Timeout Defaults with Named TimeoutPolicy`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#Group K-1`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.7 Timeout policy consolidation`
- `tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#9 enrichment 120s hardcode)`

Some cited files are gone: `crates/roko-acp/src/bridge_events.rs`, `crates/roko-core/src/config.rs`.

How to verify: Check: `TimeoutPolicy` struct exists in `roko-core` with provider, tool, and gate timeout fields; `roko.toml` `[timeouts]` section is parsed into `TimeoutPolicy`; All known hardcoded 120_000 timeout sites use `TimeoutPolicy` values instead [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 3 |] (MASTER-TASKS #9: one 120s hardcode remained in the gate judge call.)
