+++
id = "reg-ff6e1a"
kind = "regression"
title = "Knowledge-informed model routing has no caller after Runner-v2 removal"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-learn/cascade-router", "roko-cli/dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#knowledge-store-consulted-for-model-routing----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-learn/src/cascade_router.rs::select_for_frequency_among_with_knowledge", "crates/roko-cli/src/dispatch/model_routing.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -rn --include='*.rs' 'select_for_frequency_among_with_knowledge' crates | grep -v 'crates/roko-learn/src/cascade_router.rs' | grep -q .'''
+++

GAPS.md recorded this as RESOLVED on 2026-08-13. Runner-v2 opened the live `KnowledgeStore`, built per-model positive and negative routing advice, and passed it to `CascadeRouter::select_for_frequency_among_with_knowledge`. Runner-v2 was deleted on 2026-09-06 and nothing replaced that call. The function (`crates/roko-learn/src/cascade_router.rs:530`) now has no callers anywhere in the workspace, so persisted knowledge no longer influences model selection on any path.

Fix: build knowledge routing advice in the shared dispatch routing used by Graph plan runs (for example `crates/roko-cli/src/dispatch/model_routing.rs`) and call the knowledge-aware selector. Restore the positive, negative and no-signal tests on that path.
