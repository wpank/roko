+++
id = "gap-a323c6"
kind = "gap"
title = "Wire ImplementerTemplate into GraphTaskDispatcher"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose/templates"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/399-prompt-template-dispatch-wiring.md#399 — Wire ImplementerTemplate into GraphTaskDispatcher"
discovered_from = "audit:tmp/backlog/archive/399-prompt-template-dispatch-wiring.md#399 — Wire ImplementerTemplate into GraphTaskDispatcher"
anchors = ["crates/roko-compose/src/templates/", "crates/roko-cli/src/dispatch/prompt_builder.rs", "ImplementerTemplate", "TaskImplTemplate", "GraphTaskDispatcher::dispatch", "PromptAssembler::assemble", "TaskContext::with_context", "RoleSystemPromptSpec", "ImplementerTemplate::sections_with_context_window", "ImplementerInput"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`ImplementerTemplate` and `TaskImplTemplate` (in `crates/roko-compose/src/templates/`) are purpose-built typed prompt templates that emit 12 named, budget-capped, cache-layer-annotated sections. They are never called at dispatch time.

Imported without verification from:
- `tmp/backlog/archive/399-prompt-template-dispatch-wiring.md#399 — Wire ImplementerTemplate into GraphTaskDispatcher`
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T1-04: Wire rich prompt templates to graph engine dispatch`
- `tmp/archive/plan-audit-2026-09-23/07-prompt-context-wiring.md`

How to verify: Check: `GraphTaskDispatcher::dispatch` constructs an `ImplementerInput` (for implementer,; The following `ImplementerInput` fields are populated from real sources:; Template sections are merged with existing `PromptAssembler` source sections [evidence: own status: Backlog] / grep ImplementerTemplate usage in graph_task_dispatch.rs.

Merged 2 mined candidates: m1-122, m3-118.
