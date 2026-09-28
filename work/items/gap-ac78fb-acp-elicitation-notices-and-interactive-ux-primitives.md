+++
id = "gap-ac78fb"
kind = "gap"
title = "ACP Elicitation, Notices, and Interactive UX Primitives"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives"
discovered_from = "audit:tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives"
anchors = ["crates/roko-acp/src/transport.rs", "crates/roko-acp/src/bridge_events/permissions.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'elicitation/create' crates/roko-acp/src"
+++
these are the UX primitives that make roko expressive and aesthetic in editors. The best AI agents in 2026 are distinguished not by what they can do, but by how expressively they communicate what they're doing. Research across Zed, Cursor, JetBrains, VS Code, and Windsurf shows five UX patterns…

Imported without verification from:
- `tmp/backlog/archive/433-acp-elicitation-notices-interactive-ux.md#433 — ACP Elicitation, Notices, and Interactive UX Primitives`

Some cited files are gone: `elicitation/create`, `session/request_permission`.

How to verify: Check: Outbound JSON-RPC request transport implemented with response correlation; `elicitation/create` sends structured forms to the editor; Plan approval uses elicitation when client supports it [evidence: 00-INDEX (2026-09-21) listed active: ACP v2, Editor UX, and MCP Modernization (#18, #39]

Verified 2026-09-28: still true. This is editor UX polish, so p1 -> p2. Nothing in crates/roko-acp/src mentions `elicitation/create`. Outbound request/response correlation does exist (the transport.rs pending-request registry, :32/:69/:229, used for `session/request_permission`), so the transport prerequisite is partly in place. No elicitation forms, notices or elicitation-based plan approval exist.
