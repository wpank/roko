+++
id = "gap-bfef5a"
kind = "gap"
title = "[rag RAG-21] Plan-context MCP tool for mid-turn plan state queries"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-mcp-code"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-21-plan-context-mcp-tool.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-21-plan-context-mcp-tool.md"
anchors = ["crates/roko-mcp-code/src/lib.rs", "crates/roko-cli/src/task_parser.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Agents on multi-task plans cannot query plan state (goal, sibling task outputs/files) mid-turn; add an MCP tool exposing plan context.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-21-plan-context-mcp-tool.md`

How to verify: Check roko-mcp-code tool list for plan context.
