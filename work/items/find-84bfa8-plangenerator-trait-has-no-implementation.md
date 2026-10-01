+++
id = "find-84bfa8"
kind = "finding"
title = "PlanGenerator trait in roko-execution has no implementation"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-execution/src/plan_generator.rs::PlanGenerator", "crates/roko-cli/src/plan_generator.rs::DefaultPlanGenerator", "crates/roko-cli/src/prd.rs::generate_plan_from_prd"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'pub trait PlanGenerator:' crates/roko-execution/src/plan_generator.rs || grep -rn --include='*.rs' -E 'impl ([a-z_]+::)*PlanGenerator for ' crates/roko-cli/src crates/roko-serve/src crates/roko-execution/src | grep -q ."
+++

`PlanGenerator` is a trait declared in `crates/roko-execution/src/plan_generator.rs`. It has one implementation, `DefaultPlanGenerator`, whose pipeline is test-only scaffolding and does not match the working pipeline used in production.

All callers that actually generate plans use `prd::generate_plan_from_prd` directly (fenced-TOML extraction, repair, model escalation, write to `plans/<slug>/tasks.toml`). The trait abstraction exists but is never threaded through any production call site.

The `PlanGenerator` trait is the right long-term abstraction — it would allow injecting the generator in tests without a real LLM, and standardise the pipeline across the CLI, serve, and PRD routes. It is currently dead weight: referencing it misleads a reader into thinking there is a production implementation. Either implement the trait with the real pipeline, or delete it and document the working call site as the canonical path.

See `tmp/portal-audit/03-CONTRACT.md §2.2` which explicitly warns: "do not dispatch through `PlanGenerator` … The trait has **no implementation anywhere**".

Re-checked 2026-09-29 at d9e79e9d8: unchanged. Correction: DefaultPlanGenerator (crates/roko-cli/src/plan_generator.rs:46) does not implement the PlanGenerator trait (it has only an inherent impl and is built only in its tests), so the trait has no implementation at all, as the title says. The current verify command matches `impl PlanGeneratorOutcome` and a test adapter, so it passes while the problem exists.

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- Deleted rather than implemented: `prd::generate_plan` (gap-2623b2) is the one pipeline every caller uses, and it
  persists the plan, which the trait's contract ("does NOT execute, persist, or render") excludes. The trait and its
  re-exports are gone; the module docs in roko-execution and roko-cli name `prd::generate_plan` as canonical and say
  `DefaultPlanGenerator` is exercised only by its tests. `PlanGeneratorAdapter` (implemented only by a test adapter)
  and `DefaultPlanGenerator` stay; removing them is a separate cleanup.
