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
last_verified = 2026-10-01
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

## Notes

- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  Checked against the ACP v1 schema (release 1.24.1): `Create` is gone (`write_file` and added files are `edit`),
  `Terminal` serializes as `execute`, tool call content is sent as spec `ToolCallContent` (a bare unified diff
  becomes a fenced text block), locations are `{path, line}`, and MCP status and the budget ride on
  `session_info_update` under `_meta.roko`. `session_update_spec_conformance` checks every update roko emits against
  a vendored schema subset in `crates/roko-acp/tests/fixtures/`.
