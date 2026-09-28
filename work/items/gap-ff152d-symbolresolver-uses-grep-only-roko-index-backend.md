+++
id = "gap-ff152d"
kind = "gap"
title = "SymbolResolver uses grep only; roko-index backend not wired"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/symbols"]
created = 2026-05-05
updated = 2026-09-28
source = "crates/roko-compose/src/symbol_resolver.rs:14"
discovered_from = "audit:crates/roko-compose/src/symbol_resolver.rs:14"
anchors = ["crates/roko-compose/src/symbol_resolver.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
symbol_resolver.rs documents a roko-index (pre-parsed AST) resolution strategy as 'future ... Not wired yet'; symbol enrichment relies on grep over *.rs files only.

Imported without verification from:
- `crates/roko-compose/src/symbol_resolver.rs:14`

How to verify: Check whether roko-compose depends on roko-index; measure resolver latency on the workspace.
