+++
id = "gap-7d9379"
kind = "gap"
title = "[rag RAG-06] TUI retrieval panels in Learning tab"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/rag-audit-2026-09-21/backlog/RAG-06-tui-retrieval-panels.md"
discovered_from = "audit:tmp/archive/rag-audit-2026-09-21/backlog/RAG-06-tui-retrieval-panels.md"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-cli/src/tui/state.rs", "crates/roko-cli/src/tui/views/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
F10 Learning has Route/History/Efficiency sub-views but nothing for retrieval precision, latency, cache-hit rate, knowledge health or retrieval experiments.

Imported without verification from:
- `tmp/archive/rag-audit-2026-09-21/backlog/RAG-06-tui-retrieval-panels.md`

Some cited files are gone: `crates/roko-cli/src/tui/state.rs`.

How to verify: Check F10 sub-views for a retrieval panel.
