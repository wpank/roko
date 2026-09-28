+++
id = "gap-959a7c"
kind = "gap"
title = "[plan-audit T1-03] `roko plan prepare` companion documents (brief.md, prd-extract.md)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)"
anchors = ["crates/roko-cli/src/commands/plan.rs", "backlog #397"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
No per-plan companion doc ecosystem. Add non-LLM `plan prepare` porting mori generate-briefs.sh: extract prerequisites/imports/quick reference, task map, authority chain, risk flags. Backlog #397.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)`
- `tmp/archive/plan-audit-2026-09-23/05-companion-documents.md`

How to verify: Check `roko plan --help` for prepare.
