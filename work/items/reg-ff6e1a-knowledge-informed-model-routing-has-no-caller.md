+++
id = "reg-ff6e1a"
kind = "regression"
title = "Knowledge-informed model routing has no caller after Runner-v2 removal"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router", "roko-cli/dispatch"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#knowledge-store-consulted-for-model-routing----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-learn/src/cascade_router.rs::select_for_frequency_among_with_knowledge", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/knowledge_helpers.rs::build_knowledge_routing_advice"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -q 'with_knowledge_routing(workdir)' crates/roko-cli/src/graph_execution/plan_runner.rs && grep -q 'build_knowledge_routing_advice' crates/roko-cli/src/dispatch/model_routing.rs && cargo test -p roko-cli --lib knowledge_routing_moves_the_cascade_pick'''
+++

GAPS.md recorded this as RESOLVED on 2026-08-13. Runner-v2 opened the live `KnowledgeStore`, built per-model positive and negative routing advice, and passed it to `CascadeRouter::select_for_frequency_among_with_knowledge`. Runner-v2 was deleted on 2026-09-06 and nothing replaced that call. The function (`crates/roko-learn/src/cascade_router.rs:530`) now has no callers anywhere in the workspace, so persisted knowledge no longer influences model selection on any path.

Fix: build knowledge routing advice in the shared dispatch routing used by Graph plan runs (for example `crates/roko-cli/src/dispatch/model_routing.rs`) and call the knowledge-aware selector. Restore the positive, negative and no-signal tests on that path.

Rechecked 2026-09-29: unchanged. A ready-made advice builder exists but is test-only: crates/roko-cli/src/knowledge_helpers.rs::build_knowledge_routing_advice (tests routing_advice_reads_positive_and_negative_model_knowledge and routing_advice_is_empty_without_relevant_knowledge). ModelCallService's own knowledge advice (crates/roko-agent/src/model_call_service.rs::build_knowledge_advice) runs after model resolution and does not affect selection.

## Notes

- 2026-10-01 (wk-settle): implemented on work/bug-f9ae3e; cargo verification deferred to the batch check.
  Graph plan runs now weigh durable knowledge into the cascade router's pick:
  - `plan_runner.rs` calls `SharedAgentFactory::with_knowledge_routing(workdir)`, and the factory keeps it when it
    rebuilds the dispatcher.
  - The dispatcher's `ModelRouter` builds per-model advice with `knowledge_helpers::build_knowledge_routing_advice`
    for the models its guards accept, and re-ranks the health- or bias-aware pick with the new
    `CascadeRouter::apply_knowledge_among`, the knowledge step of `route_with_knowledge_among`.
  - Overrides, task hints and the ladder are unaffected. Test: `knowledge_routing_moves_the_cascade_pick`.
- The verify command changed. `select_for_frequency_among_with_knowledge` routes without the health filter, so
  calling it from dispatch would have dropped Open-circuit filtering and latency demotion. The command now checks
  the wiring and the test.
