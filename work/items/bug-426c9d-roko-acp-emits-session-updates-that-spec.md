+++
id = "bug-426c9d"
kind = "bug"
title = "roko acp emits session updates that spec ACP clients drop"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp/protocol"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/types.rs::ToolCallKind", "crates/roko-acp/src/types.rs:701", "crates/roko-acp/src/types.rs:716", "crates/roko-acp/src/bridge_events/helpers.rs:48", "crates/roko-acp/src/bridge_events/mod.rs:993"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE '^\\s*Create,' crates/roko-acp/src/types.rs && ! grep -rqE 'SessionUpdate::(McpStatusUpdate|BudgetStatusUpdate)' crates/roko-acp/src --include=*.rs && cargo test -p roko-acp session_update_spec_conformance"
+++

`ToolCallKind` has non-spec variants such as `Create` (`types.rs:741`), and roko sends non-spec `McpStatusUpdate` / `BudgetStatusUpdate` session updates (`:701`, `:716`); per a local probe, bare-text `tool_call_update` content is also rejected.
Spec SDK clients validate notifications and silently drop the ones that fail, so shell and edit calls vanish from client transcripts.
Fix: emit only spec kinds and content shapes (e.g. `create` -> `edit`, text wrapped as content blocks), carry extensions under `_meta`, and add golden fixtures validated against the spec schema.
