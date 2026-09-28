+++
id = "gap-d6fd85"
kind = "gap"
title = "Per-Plan Companion Document Generation"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/397-companion-document-generation.md#397 — Per-Plan Companion Document Generation"
discovered_from = "audit:tmp/backlog/archive/397-companion-document-generation.md#397 — Per-Plan Companion Document Generation"
anchors = ["crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/main.rs::PlanCmd", "crates/roko-compose/src/templates/implementer.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'brief.md' crates/roko-cli/src"
+++
Every agent dispatched by roko receives a flat 9-layer system prompt assembled at dispatch time. That prompt contains the plan content and tasks, but no pre-computed orientation artifacts. In mori, agents reliably received up to 14 companion files alongside their task: a deterministic `brief.md`…

Imported without verification from:
- `tmp/backlog/archive/397-companion-document-generation.md#397 — Per-Plan Companion Document Generation`

How to verify: Check: `roko plan prepare plans/my-plan` generates `plans/my-plan/brief.md` and; `roko plan prepare plans/my-plan` is idempotent: re-running does not overwrite; `brief.md` contains: authority chain header, artifact pointer table, task map [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: No `plan prepare` subcommand and no brief.md/companion generation exist anywhere in crates/roko-cli/src; agents still receive only the dispatch-time system prompt. Severity lowered p1 -> p2: a mori-parity enhancement, not a correctness gap.
