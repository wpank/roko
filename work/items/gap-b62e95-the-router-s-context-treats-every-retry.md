+++
id = "gap-b62e95"
kind = "gap"
title = "The router's context treats every retry as a first attempt"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
subsystem = ["roko-cli/model-routing"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/s02-s06-loops.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s02-s06-loops.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch/routing_context.rs::build_routing_context", "crates/roko-cli/src/runtime_feedback/routing.rs::build_fallback_routing_context"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/^fn build_routing_context(/,/^}/p' crates/roko-cli/src/graph_task_dispatch.rs | grep -qE 'has_prior_failure: false|iteration: 0,' && cargo test -p roko-cli --lib graph_task_dispatch::tests::routing_context_marks_retry_after_failure"
+++
The routing context passed to the cascade router hard-codes `has_prior_failure: false` (`graph_task_dispatch.rs:2987`, `dispatch/model_routing.rs:635`), along with a zero iteration count and zero conductor load. LinUCB cannot tell a retry after a failed verify from a first attempt, so it cannot learn to escalate after failures, and several of its context features never vary.

Rechecked 2026-09-29 at d9e79e9d8. The production site is build_routing_context in crates/roko-cli/src/graph_task_dispatch.rs; it has moved from :2987 to :3162. crates/roko-cli/src/dispatch/model_routing.rs:635 is a unit-test fixture. A second production default with has_prior_failure: false is build_fallback_routing_context in crates/roko-cli/src/runtime_feedback/routing.rs:136-143, used when feedback settles without the original context.
