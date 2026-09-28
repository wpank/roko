+++
id = "gap-9c8ac0"
kind = "gap"
title = "#375 — Unified transcript and provider/tool parity"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/375-unified-transcript-tool-provider-parity.md##375 — Unified transcript and provider/tool parity"
discovered_from = "audit:tmp/backlog/archive/375-unified-transcript-tool-provider-parity.md##375 — Unified transcript and provider/tool parity"
anchors = ["tmp/tool-audit/00-INDEX.md", "tmp/tool-audit/01-event-schema.md", "crates/roko-cli/src/runner/tui_bridge.rs", "crates/roko-cli/src/runner/agent_events.rs", "crates/roko-cli/src/tui/app.rs", "TranscriptEvent", "TranscriptStore", "AgentOutput"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[partial] In progress (StateHub semantic stream slice landed) — The TUI receives multiple string-tail projections and serializes tool calls as `[tool_call]` text. Provider-specific streams, runtime tool events, task output files, and inline rendering therefore disagree about ordering and…

Imported without verification from:
- `tmp/backlog/archive/375-unified-transcript-tool-provider-parity.md##375 — Unified transcript and provider/tool parity`

Some cited files are gone: `crates/roko-cli/src/runner/agent_events.rs`, `crates/roko-cli/src/tui/app.rs`, `tmp/tool-audit/00-INDEX.md`, `tmp/tool-audit/01-event-schema.md`.

How to verify: Check whether the gap described in tmp/backlog/archive/375-unified-transcript-tool-provider-parity.md still exists at the anchored paths. [evidence: own status: In progress (StateHub semantic stream slice landed)]
