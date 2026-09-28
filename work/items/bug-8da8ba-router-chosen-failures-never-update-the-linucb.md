+++
id = "bug-8da8ba"
kind = "bug"
title = "Router-chosen failures never update the LinUCB model"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-cli/runtime-feedback", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs:125"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

On a failed outcome the routing sink only calls `record_confidence_outcome(model, false)` (`runtime_feedback/routing.rs:125`); per a local audit that path updates counters but never the contextual bandit, so LinUCB learns from successes only.
Persisted router state inspected in that audit showed failures in the totals but none in the bandit arms.
Fix: feed failures to the bandit with reward 0 through the same path as successes; add a test.

Verified 2026-09-28 (static check against 3d0ee4d02): On failure the routing sink calls only self.router.record_confidence_outcome(&outcome.model, false) (crates/roko-cli/src/runtime_feedback/routing.rs:125), while the success path feeds the bandit through observe_multi_objective (routing.rs ~106-122). CascadeRouter::record_confidence_outcome (crates/roko-learn/src/cascade_router.rs:1455-1510) touches only confidence_stats and never references linucb or the bandit. The gateway does the same on failure (crates/roko-gateway/src/gateway.rs:648). The Graph-path RoutingSink (graph_execution/feedback.rs:358) was not checked.
