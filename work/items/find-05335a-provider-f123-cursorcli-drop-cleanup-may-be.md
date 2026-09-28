+++
id = "find-05335a"
kind = "finding"
title = "[provider F123] CursorCli Drop cleanup may be skipped on Tokio shutdown"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F123"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F123"
anchors = ["crates/roko-agent/src/provider/cursor_cli.rs", "CursorCli"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`CursorCli` implements `Drop` to clean up the subprocess. Tokio may skip `Drop` for tasks that are cancelled during runtime shutdown, leaving orphaned subprocess children.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F123`
- `tmp/archive/provider-audit/03-cli-subprocess.md`

How to verify: Confirm in crates/roko-agent/src/provider/cursor_cli.rs whether still true: `CursorCli` Drop cleanup may be skipped on Tokio shutdown
