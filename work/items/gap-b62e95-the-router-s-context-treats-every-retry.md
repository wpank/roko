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
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/s02-s06-loops.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s02-s06-loops.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:2987", "crates/roko-cli/src/dispatch/model_routing.rs:635"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The routing context passed to the cascade router hard-codes `has_prior_failure: false` (`graph_task_dispatch.rs:2987`, `dispatch/model_routing.rs:635`), along with a zero iteration count and zero conductor load. LinUCB cannot tell a retry after a failed verify from a first attempt, so it cannot learn to escalate after failures, and several of its context features never vary.
