+++
id = "gap-21ce75"
kind = "gap"
title = "Preserve Input/Output Token Usage in Workflow and Shared-Run Reports"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/301-workflow-usage-breakdown.md#301 — Preserve Input/Output Token Usage in Workflow and Shared-Run Reports"
discovered_from = "audit:tmp/backlog/archive/301-workflow-usage-breakdown.md#301 — Preserve Input/Output Token Usage in Workflow and Shared-Run Reports"
anchors = ["crates/roko-serve/src/routes/shared_runs.rs", "model_call_service.rs", "crates/roko-runtime/src/effect_driver.rs", "crates/roko-cli/src/runner/event_loop.rs", "shared_runs.rs", "WorkflowRunReport", "chat_types::Usage::total_tokens()", "model_call_service::token_usage"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #208 and #257 — `crates/roko-cli/src/run.rs::write_shared_workflow_run` can only copy a combined `WorkflowRunReport.token_usage` total. It writes zero/unknown input and output counts. The same loss is called out by two production TODOs in…

Imported without verification from:
- `tmp/backlog/archive/301-workflow-usage-breakdown.md#301 — Preserve Input/Output Token Usage in Workflow and Shared-Run Reports`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-runtime/src/effect_driver.rs`.

How to verify: Check whether the gap described in tmp/backlog/archive/301-workflow-usage-breakdown.md still exists at the anchored paths. [evidence: own status: Blocked on #208 and #257]
