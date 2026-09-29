+++
id = "gap-96d348"
kind = "gap"
title = "FailureStrategy::SkipFailed unreachable; FailFast hardcoded"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-graph/convert"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations"
anchors = ["crates/roko-graph/src/convert.rs::plan_to_graph", "crates/roko-graph/src/types.rs::FailureStrategy", "crates/roko-graph/src/types.rs:189", "crates/roko-cli/src/graph_execution/plan_set.rs:927"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn fail_fast_policy_stops_a_plan_at_its_first_failure' crates/roko-cli/src/graph_execution/plan_runner.rs && grep -q 'plan_failure_policy' crates/roko-core/src/config/schema.rs && cargo test -p roko-cli --lib fail_fast_policy_stops_a_plan_at_its_first_failure"

[closed]
at = 2026-09-29
commit = "9ef6f4aad"
by = "reconcile of the portal session's merges 2026-09-29 (static check)"
evidence = "9ef6f4aad (merged in abc1f4b27): [conductor] plan_failure_policy (config/schema.rs:1343) and a plan's [meta] failure_policy select the failure strategy; plans default to skip_failed and a failure blocks only its dependants. Tests fail_fast_policy_stops_a_plan_at_its_first_failure and a_failed_task_blocks_only_its_dependants in graph_execution/plan_runner.rs."
+++
plan_to_graph never sets SkipFailed, so FailFast applies in practice and cannot be configured; plan granularity is the only blast-radius control.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#Architecture recommendations`

How to verify: Check plan_to_graph failure strategy wiring.

Verified 2026-09-28 (static check against 3d0ee4d02): plan_to_graph (roko-graph/src/convert.rs:33-57) sets only graph.policy.max_concurrent_nodes, so plan runs keep GraphPolicy's default FailureStrategy::FailFast (roko-graph/src/types.rs:160, :189). Nothing in roko-cli or the tasks.toml/roko.toml schema sets failure_strategy. SkipFailed is reachable only through the graph TOML loader for `roko graph run` (roko-graph/src/loader.rs:240-250), not through plan runs. Plan sets likewise block dependents with BlockReason::FailFast (graph_execution/plan_set.rs:820, :927).
