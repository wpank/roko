+++
id = "gap-b367bf"
kind = "gap"
title = "Graph cancellation lands only between nodes; a running gate command is not signalled"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-graph/engine", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-graph/src/engine.rs::FlowHandle::cancel", "crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-gate/src/cancel_safe_command.rs::output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn interrupt_stops_running_gate_command" crates/roko-cli/src && cargo test -p roko-cli --lib interrupt_stops_running_gate_command'
+++

`FlowHandle::cancel` is honoured between nodes and waves (engine.rs checks `cancel.is_cancelled()` at :1818 and :2154). On interrupt, plan_runner SIGTERMs the registered agent process trees so in-flight agent nodes settle, then abandons the graph after 3 s. A long non-agent step, such as a gate's `cargo test`, is not signalled: it can keep running past the interrupt, and if it outlives the drain its node re-runs on resume.

Fix: register gate and verify child processes with the interrupt path (or run them under the cancel token) so an interrupt stops them too. Add a test named `interrupt_stops_running_gate_command`.
