+++
id = "gap-be0ac2"
kind = "gap"
title = "Agent output is forwarded to the dashboard only after the agent finishes"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/portal-audit/01-FINDINGS.md#B3"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui"]
links = { depends_on = ["dec-578863"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "plan:portal-programme/03c-backend-local-access#T10"
run_id = "graph-03c-backend-local-access-ecb447db-1cfe-4ad3-9f93-2bc34ba59f34"
evidence = "LIVE-OUTPUT-CHECK: PASS (15 checks). 'PASS a tool step arrives while the task is still running' confirmed that agent output (tool steps) now streams live before the agent finishes, resolving the burst-at-end behaviour. The immune-boundary decision (dec-578863) unblocked and was implemented in 03c T12-T20."
+++

`forward_dispatch_events_to_tui` (`graph_task_dispatch.rs:1230`) runs at `:1956` and `:3258`, after `run_shared_agent_bridge(request)` has returned (`:1922`, `:3251`).
The whole transcript arrives in one burst when the agent is already done; a watcher sees nothing while it works.
Consumers must also decode framed `roko.stream.v1` records (`runner/tui_bridge.rs`), and tool output over 2048 bytes is cut to its last 1024.

**Blocked by dec-578863.** Streaming before the immune boundary is a trust/safety decision. Liveness is now shown via `agent_heartbeat` (every 5 s during a turn) and `agent_spawned` at dispatch start — the watcher knows the agent is running even without a live transcript.
