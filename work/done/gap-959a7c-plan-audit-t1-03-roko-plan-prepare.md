+++
id = "gap-959a7c"
kind = "gap"
title = "[plan-audit T1-03] `roko plan prepare` companion documents (brief.md, prd-extract.md)"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/commands/plan"]
created = 2026-09-21
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)"
anchors = ["crates/roko-cli/src/commands/plan.rs", "backlog #397"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:16Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done: `roko plan prepare <plan>` exists (gap-d6fd85, gap-c56341; `crates/roko-cli/src/plan_brief.rs`) and writes the non-LLM `brief.md` (artifacts, task map, risks), which dispatch adds to each task prompt; `--full` adds decomposition.md and rubric.md. The `prd-extract.md` companion went with the PRD pipeline (merge bfd36512f)."
+++
No per-plan companion doc ecosystem. Add non-LLM `plan prepare` porting mori generate-briefs.sh: extract prerequisites/imports/quick reference, task map, authority chain, risk flags. Backlog #397.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-03: Implement `plan prepare` (companion document generation)`
- `tmp/archive/plan-audit-2026-09-23/05-companion-documents.md`

How to verify: Check `roko plan --help` for prepare.
