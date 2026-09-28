+++
id = "gap-eeaa6e"
kind = "gap"
title = "Plan CLI UX Improvements"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/405-plan-cli-ux-improvements.md#405 — Plan CLI UX Improvements"
discovered_from = "audit:tmp/backlog/archive/405-plan-cli-ux-improvements.md#405 — Plan CLI UX Improvements"
anchors = ["crates/roko-cli/src/main.rs", "crates/roko-cli/src/commands/plan.rs", "plan.md", "roko plan run plans/", "roko plan run plans/my-plan", ".roko/queue.toml", "roko.toml", "roko plan create <id> --title \"X\""]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`roko plan` is the primary daily-driver command group. A full audit (tmp/plan-audit/13-cli-ux-gaps.md) against the Mori reference CLI found five categories of gaps: missing subcommands, missing flags on `plan create`, a resume naming conflict, flag overload on `plan run`, and missing run-scoped…

Imported without verification from:
- `tmp/backlog/archive/405-plan-cli-ux-improvements.md#405 — Plan CLI UX Improvements`

Some cited files are gone: `.roko/queue.toml`.

How to verify: Check whether the gap described in tmp/backlog/archive/405-plan-cli-ux-improvements.md still exists at the anchored paths. [evidence: no status line; no index/roll-up evidence]
