+++
id = "gap-0d6dab"
kind = "gap"
title = "[engine inject-deletion] `roko inject` command (and hidden dev/up/repl/layer-check verbs) never deleted"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/commands"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
discovered_from = "audit:tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
anchors = ["crates/roko-cli/src/main.rs", "inject", "tune", "dev", "up", "repl", "layer-check"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Cross-reference item 3 'Delete stubs' only partially covered: #264 removed MarketCmd but explicitly retains inject, tune, dev, up, repl; #65 hides deprecated verbs instead of deleting. No backlog item removes inject.

Imported without verification from:
- `tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps`
- `tmp/archive/engine-audit/21-backlog-cross-reference.md#3-delete-stubs`

How to verify: Check whether `roko inject` and hidden deprecated verbs still exist in the clap command tree; decide delete vs keep (product decision).
