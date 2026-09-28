+++
id = "bug-095f57"
kind = "bug"
title = "[cli-audit hidden] 4 executable roko-mcp-code tools missing from tools/list"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-mcp-code"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented"
anchors = ["crates/roko-mcp-code/", "backlog #349"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-mcp-code executes 4 tools that are not advertised in its tool list. #349 (list=call inventory) claimed done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Hidden / Undocumented`
- `tmp/archive/cli-audit-2026-09-21/26-mcp.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Diff tools/list names against call dispatch match arms in roko-mcp-code.
