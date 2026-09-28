+++
id = "gap-d08859"
kind = "gap"
title = "DOCS-07 TD-06: Synchronous file I/O on the Tokio runtime in runner hot paths"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/07-TECH-DEBT.md#TD-06: Sync I/O on Tokio Runtime"
discovered_from = "audit:tmp/docs-audit/07-TECH-DEBT.md#TD-06: Sync I/O on Tokio Runtime"
anchors = ["std::fs in async fn"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Blocking file I/O in async hot paths stalls executor threads under load (convert to async or spawn_blocking).

Imported without verification from:
- `tmp/docs-audit/07-TECH-DEBT.md#TD-06: Sync I/O on Tokio Runtime`

How to verify: grep std::fs in async graph_execution/serve paths.
