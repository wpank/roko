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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e2-planset"
anchors = ["crates/roko-graph/src/engine.rs::FlowHandle::cancel", "crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-gate/src/cancel_safe_command.rs::output"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn interrupt_stops_running_gate_command" crates/roko-cli/src && cargo test -p roko-cli --lib interrupt_stops_running_gate_command'
+++

`FlowHandle::cancel` is honoured between nodes and waves (engine.rs checks `cancel.is_cancelled()` at :1818 and :2154). On interrupt, plan_runner SIGTERMs the registered agent process trees so in-flight agent nodes settle, then abandons the graph after 3 s. A long non-agent step, such as a gate's `cargo test`, is not signalled: it can keep running past the interrupt, and if it outlives the drain its node re-runs on resume.

Fix: register gate and verify child processes with the interrupt path (or run them under the cancel token) so an interrupt stops them too. Add a test named `interrupt_stops_running_gate_command`.

## Notes

- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  `ShellGate::verify`, which runs every Graph verify step, rung and whole-plan check, and `cancel_safe_command::output`
  (the other roko-gate commands) now register the running command's PID with `roko_agent::process`'s registry. They
  unregister it when it ends or the future drops (`RegisteredCommand`). On an interrupt, `terminate_in_flight_agents`
  therefore SIGTERMs a running gate command and its descendants (a `cargo test` tree) together with the agents. A
  later roko process can also clean up a gate command orphaned by a crash.
  - Test: `interrupt_stops_running_gate_command`. It uses a child test process, as bug-2b1ddc's does, so the
    interrupt signals only its own processes. A verify command traps SIGTERM into a marker.
  - Not changed: a verify step stopped this way settles as a gate failure, not a cancellation. The task isn't
    retried, because bug-ceb581 stops retries in a cancelled run, but the attempt's verdict says gate_failed.
