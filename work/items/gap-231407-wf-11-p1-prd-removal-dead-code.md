+++
id = "gap-231407"
kind = "gap"
title = "WF-11 P1: PRD removal dead-code residue (auto_plan, prd_excerpt, source_prd validation, find_related_prds...)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)"
discovered_from = "audit:tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/plan_policy.rs", "crates/roko-cli/src/repo_context.rs", "crates/roko-serve/src/openapi.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
~385 LOC left: auto_plan config field/display, prd_excerpt + load_prd_excerpt on every dispatch, source_prd validated against .roko/prd/published (can fail plan validation), find_related_prds, prd/dry_run_fs.rs, TUI config_meta key, openapi prds tag, slug path in plan generate.

Imported without verification from:
- `tmp/workflow-audit/11-FINAL-STATUS.md#P1 — Dead Code Cleanup (Phase 1 residuals)`
- `tmp/workflow-audit/11-FINAL-STATUS.md#Known Issues`
- `tmp/workflow-audit/10-STATUS-UPDATE.md#Open Items / Remaining Work`

How to verify: grep -rn 'prd_excerpt\|source_prd\|find_related_prds\|auto_plan' crates/.
