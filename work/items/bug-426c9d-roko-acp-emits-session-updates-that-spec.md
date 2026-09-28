+++
id = "bug-426c9d"
kind = "bug"
title = "roko acp emits session updates that spec ACP clients drop"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-acp/protocol"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/types.rs:741", "crates/roko-acp/src/types.rs:701", "crates/roko-acp/src/types.rs:716"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`ToolCallKind` has non-spec variants such as `Create` (`types.rs:741`), and roko sends non-spec `McpStatusUpdate` / `BudgetStatusUpdate` session updates (`:701`, `:716`); per a local probe, bare-text `tool_call_update` content is also rejected.
Spec SDK clients validate notifications and silently drop the ones that fail, so shell and edit calls vanish from client transcripts.
Fix: emit only spec kinds and content shapes (e.g. `create` -> `edit`, text wrapped as content blocks), carry extensions under `_meta`, and add golden fixtures validated against the spec schema.
