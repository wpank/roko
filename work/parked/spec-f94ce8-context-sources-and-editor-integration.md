+++
id = "spec-f94ce8"
kind = "spec"
title = "Context Sources and Editor Integration"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/66-context-editor-integration.md#66 — Context Sources and Editor Integration"
discovered_from = "audit:tmp/backlog/archive/66-context-editor-integration.md#66 — Context Sources and Editor Integration"
anchors = ["crates/roko-cli", "src/main.rs", "src/commands/do_cmd.rs", "src/commands/develop.rs", "src/context_loader.rs", "crates/roko-neuro", "src/context.rs", "crates/roko-acp"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[todo] CONSOLIDATED P3-UX-5: deferred — `--context` flag exists on `roko do` but is missing from `roko run`, `roko develop`, and `roko chat`; URL sources and editor context push are not supported. With ACP v2 targeting, editor context is the key to matching Zed @-mentions, Cursor @-context, and…

Imported without verification from:
- `tmp/backlog/archive/66-context-editor-integration.md#66 — Context Sources and Editor Integration`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-UX-5 (Subsystem: CLI UX)`

How to verify: Check: `roko run "Fix bug" --context src/lib.rs` loads `src/lib.rs` into the prompt. Currently `roko run` has no `--context` flag and rejects this invocation.; `roko do "Add tests" --context https://github.com/org/repo/issues/42` fetches the URL… [evidence: CONSOLIDATED P3-UX-5: deferred; 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): L | 7 |]
