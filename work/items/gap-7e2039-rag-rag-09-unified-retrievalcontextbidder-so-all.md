+++
id = "gap-7e2039"
kind = "gap"
title = "[rag RAG-09] Unified RetrievalContextBidder so all retrieval sources compete in VCG auction"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-09-retrieval-context-bidder.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-09-retrieval-context-bidder.md"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs:1649 WorkdirKnowledgeSource", "crates/roko-cli/src/prompt_helpers.rs:231 code_context_for_task", "crates/roko-cli/src/dispatch_helpers.rs:727", "crates/roko-compose/src/context_provider.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Knowledge (WorkdirKnowledgeSource::collect) and code-index context bypass the ContextBidderRegistry/VCG auction via raw PromptSections and string concatenation; no unified bidder arbitrates token budget.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-09-retrieval-context-bidder.md`

Some cited files are gone: `crates/roko-cli/src/dispatch_helpers.rs`, `crates/roko-cli/src/prompt_helpers.rs`.

How to verify: Check whether knowledge/code context flow through ContextBidderRegistry::propose_context.
