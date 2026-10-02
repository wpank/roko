+++
id = "gap-231407"
kind = "gap"
title = "WF-11 P1: PRD removal dead-code residue (auto_plan, prd_excerpt, source_prd validation, find_related_prds...)"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-25
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/plan_policy.rs", "crates/roko-cli/src/repo_context.rs", "crates/roko-serve/src/openapi.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:14Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done in merge bfd36512f (roko-7d): `PrdConfig`/`[prd] auto_plan` removed (`[prd]` is in REMOVED_CONFIG_KEYS), the `prd_excerpt` prompt section, `load_prd_excerpt` and the `prd-extract.md` companion removed, `source_prd` validation removed (an old plan that names one still loads: plan_policy.rs `a_plan_that_names_a_source_prd_still_loads`), `find_related_prds` removed, `RokoEvent::PrdPublished` and the publish orchestrator removed, and the openapi PRD routes removed."
+++
~385 LOC left: auto_plan config field/display, prd_excerpt + load_prd_excerpt on every dispatch, source_prd validated against .roko/prd/published (can fail plan validation), find_related_prds, prd/dry_run_fs.rs, TUI config_meta key, openapi prds tag, slug path in plan generate.

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)`
- `tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues`
- `tmp/workflow-audit/10-STATUS-UPDATE.md#Open Items / Remaining Work`

How to verify: grep -rn 'prd_excerpt\|source_prd\|find_related_prds\|auto_plan' crates/.
