+++
id = "gap-d6fd85"
kind = "gap"
title = "Per-Plan Companion Document Generation"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-cli/commands"]
created = 2026-09-21
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
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

## Notes

- 2026-10-01 (wk-specq): implemented on work/gap-404fdb; cargo verification deferred to the batch check. This
  covers phases 1 and 2 of the source spec.
  - `roko plan prepare <plan-dir> [--force]` (new module `plan_brief.rs`) writes `brief.md` and, when
    `[meta] source_prd` names a PRD under `.roko/prd/`, `prd-extract.md`. No model runs, and existing files are
    kept unless `--force`. The brief has the artifact table, the task map (ID, title, role, tier, depends on,
    files), risk flags (no verify step, `max_loc` over 500, `skip_enrichment`) and plan.md's Quick Reference.
  - Dispatch reads the plan's `brief.md` into `PromptContext::plan_brief` and renders it as `# Plan Brief` in
    the runner context.
  - Tests: `prepare_writes_the_companion_documents_and_keeps_them`,
    `prepare_writes_no_extract_without_the_source_prd` and `plan_brief_reaches_the_prompt`.
  - Not done: phase 3 (`--full`: LLM-written `decomposition.md` and `rubric.md`), which the source spec scopes
    separately. No flag was added for it.
