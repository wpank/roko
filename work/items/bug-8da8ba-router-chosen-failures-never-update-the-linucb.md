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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "5a9e07a26"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs:125", "crates/roko-cli/src/graph_execution/feedback.rs:391", "crates/roko-learn/src/cascade_router.rs::record_confidence_outcome", "crates/roko-gateway/src/gateway.rs:648"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn routing_sink_updates_linucb_on_failure' crates/roko-cli/src/ && grep -rqw 'fn gateway_failed_attempt_updates_linucb_like_a_success' crates/roko-gateway/src/ && cargo test -p roko-cli --lib routing_sink_updates_linucb_on_failure && cargo test -p roko-gateway --lib gateway_failed_attempt_updates_linucb_like_a_success"
+++

On a failed outcome the routing sink only calls `record_confidence_outcome(model, false)` (`runtime_feedback/routing.rs:125`); per a local audit that path updates counters but never the contextual bandit, so LinUCB learns from successes only.
Persisted router state inspected in that audit showed failures in the totals but none in the bandit arms.
Fix: feed failures to the bandit with reward 0 through the same path as successes; add a test.

Verified 2026-09-28 (static check against 3d0ee4d02): On failure the routing sink calls only self.router.record_confidence_outcome(&outcome.model, false) (crates/roko-cli/src/runtime_feedback/routing.rs:125), while the success path feeds the bandit through observe_multi_objective (routing.rs ~106-122). CascadeRouter::record_confidence_outcome (crates/roko-learn/src/cascade_router.rs:1455-1510) touches only confidence_stats and never references linucb or the bandit. The gateway does the same on failure (crates/roko-gateway/src/gateway.rs:648). The Graph-path RoutingSink (graph_execution/feedback.rs:358) was not checked.

Rechecked 2026-09-29 at d9e79e9d8: still open. The previously unchecked Graph-path RoutingSink (graph_execution/feedback.rs:357-393) records router-selected outcomes, success or failure, only through record_confidence_outcome (:391). On Graph runs, therefore, only RoutingObservationSink (wired at plan_runner.rs:943) feeds successes to LinUCB. Failures reach it through neither sink, and only manual overrides take the dampened record_override_outcome path.

## Notes

- Implemented on `work/bug-8da8ba` at `51ed5719c`; cargo verification deferred to the batch check.
- Scope: both routing sinks (`observe_router_outcome`), the gateway, Path B and ACP now record a failure as a trial without a success plus a reward-0 LinUCB update. The outcome is still `succeeded`; bug-c34782 swaps in the settled verdict at the marked spots. The override path (bug-f68404) is unchanged.
