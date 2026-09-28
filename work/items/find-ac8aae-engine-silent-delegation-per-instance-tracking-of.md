+++
id = "find-ac8aae"
kind = "finding"
title = "[engine silent-delegation] Per-instance tracking of the 18 silent-delegation paths never verified"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
discovered_from = "audit:tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
anchors = ["crates/roko-cli/src/commands/run.rs", "crates/roko-cli/src/do_cmd.rs", "resolve_engine_flag"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Cross-reference item 13: #262 (flags), #278 (template routing), #258 (engine selection) address root causes, but completeness against the 18-instance inventory in 04-silent-delegation.md (run->do delegation, keyword intent routing, single-word prompt matching plan slugs, bare `roko "prompt"` path...

Imported without verification from:
- `tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps`
- `tmp/archive/engine-audit/04-silent-delegation.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/commands/run.rs`, `crates/roko-cli/src/do_cmd.rs`) — likely obsolete or moved.

How to verify: Walk each of the 18 instances in 04-silent-delegation.md against current run/do/bare-prompt code paths and record which remain.
