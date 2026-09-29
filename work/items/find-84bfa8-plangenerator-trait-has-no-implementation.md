+++
id = "find-84bfa8"
kind = "finding"
title = "PlanGenerator trait in roko-execution has no implementation"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-execution/src/plan_generator.rs::PlanGenerator"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'impl PlanGenerator' crates/ --include='*.rs' | grep -v 'DefaultPlanGenerator' | grep -v target/"
+++

`PlanGenerator` is a trait declared in `crates/roko-execution/src/plan_generator.rs`. It has one implementation, `DefaultPlanGenerator`, whose pipeline is test-only scaffolding and does not match the working pipeline used in production.

All callers that actually generate plans use `prd::generate_plan_from_prd` directly (fenced-TOML extraction, repair, model escalation, write to `plans/<slug>/tasks.toml`). The trait abstraction exists but is never threaded through any production call site.

The `PlanGenerator` trait is the right long-term abstraction — it would allow injecting the generator in tests without a real LLM, and standardise the pipeline across the CLI, serve, and PRD routes. It is currently dead weight: referencing it misleads a reader into thinking there is a production implementation. Either implement the trait with the real pipeline, or delete it and document the working call site as the canonical path.

See `tmp/portal-audit/03-CONTRACT.md §2.2` which explicitly warns: "do not dispatch through `PlanGenerator` … The trait has **no implementation anywhere**".
