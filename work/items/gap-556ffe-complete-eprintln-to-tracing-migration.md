+++
id = "gap-556ffe"
kind = "gap"
title = "Complete eprintln! to tracing Migration"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/381-eprintln-to-tracing-migration.md#381 — Complete eprintln! to tracing Migration"
discovered_from = "audit:tmp/backlog/archive/381-eprintln-to-tracing-migration.md#381 — Complete eprintln! to tracing Migration"
anchors = ["crates/roko-cli/src/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
162 calls bypass structured logging. Status-quo audit identified 317 eprintln! calls. ~130 were migrated in a prior batch, but verification found 162 still remain in roko-cli alone. These bypass tracing/structured logging, making production diagnosis harder and polluting stderr.

Imported without verification from:
- `tmp/backlog/archive/381-eprintln-to-tracing-migration.md#381 — Complete eprintln! to tracing Migration`

How to verify: Check whether the gap described in tmp/backlog/archive/381-eprintln-to-tracing-migration.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
