+++
id = "gap-aed105"
kind = "gap"
title = "CLI Verb Consolidation"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/65-cli-verb-consolidation.md#65 — CLI Verb Consolidation"
discovered_from = "audit:tmp/backlog/archive/65-cli-verb-consolidation.md#65 — CLI Verb Consolidation"
anchors = ["crates/roko-cli", "src/main.rs", "src/agent_serve.rs", "src/commands/util.rs", "crates/roko-cli/src/main.rs", "main.rs", "main.rs:1431", "main.rs:1628"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[todo] CONSOLIDATED P3-UX-4: deferred — `roko --help` outputs 46+ top-level verbs with no visual grouping, overwhelming new users. The roko CLI has grown organically to include over 46 top-level subcommands. When a new user types `roko help`, they see a wall of verbs that scrolls past a single…

Imported without verification from:
- `tmp/backlog/archive/65-cli-verb-consolidation.md#65 — CLI Verb Consolidation`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-UX-4 (Subsystem: CLI UX)`

How to verify: Check: `roko --help` groups subcommands under visual headings (Core workflow, Planning, Agents, etc.) rather than a flat list.; `roko help` (short form) also shows grouped headings (not just on `--help`).; Top-level verb count visible in help… [evidence: CONSOLIDATED P3-UX-4: deferred; 00-INDEX historical claim: PR #73; 00-STATUS-SUMMARY 1. Impleme / Wave 7: Nice-to-Have / Futur: PR #73 (2026-08-31)]
