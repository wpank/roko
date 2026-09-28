+++
id = "gap-0509b2"
kind = "gap"
title = "[rag RAG-16] Add `retrieve` built-in tool for mid-turn retrieval"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-std/tool"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-16-retrieve-builtin-tool.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-16-retrieve-builtin-tool.md"
anchors = ["crates/roko-std/src/tool/builtin/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
All retrieval happens at dispatch time; agents cannot explicitly request knowledge/episode/symbol retrieval mid-turn. Add a `retrieve` builtin in roko-std.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-16-retrieve-builtin-tool.md`

How to verify: Check builtin tool registry for retrieve.
