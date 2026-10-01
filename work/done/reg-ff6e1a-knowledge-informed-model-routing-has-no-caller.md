+++
id = "reg-ff6e1a"
kind = "regression"
title = "Knowledge-informed model routing has no caller after Runner-v2 removal"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router", "roko-cli/dispatch"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "gaps-md#knowledge-store-consulted-for-model-routing----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-learn/src/cascade_router.rs::select_for_frequency_among_with_knowledge", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/knowledge_helpers.rs::build_knowledge_routing_advice"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -q 'with_knowledge_routing(workdir)' crates/roko-cli/src/graph_execution/plan_runner.rs && grep -q 'build_knowledge_routing_advice' crates/roko-cli/src/dispatch/model_routing.rs && cargo test -p roko-cli --lib knowledge_routing_moves_the_cascade_pick'''

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:55Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:11Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
