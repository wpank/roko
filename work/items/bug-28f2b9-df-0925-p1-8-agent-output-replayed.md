+++
id = "bug-28f2b9"
kind = "bug"
title = "Agent output replayed in one burst after the agent finishes"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui", "crates/roko-cli/src/graph_task_dispatch.rs:3472", "crates/roko-cli/src/graph_task_dispatch.rs:3875"]
links = { depends_on = ["dec-578863"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-09-29
by = "plan:portal-programme/03c-backend-local-access#T10"
run_id = "graph-03c-backend-local-access-ecb447db-1cfe-4ad3-9f93-2bc34ba59f34"
evidence = "LIVE-OUTPUT-CHECK: PASS (15 checks). 'PASS a tool step arrives while the task is still running' and 'PASS the step names the tool' confirm that tool steps now stream live during agent execution rather than bursting after it finishes. The immune-boundary decision (dec-578863) was the blocker; 03c T12-T20 implemented the chosen option."
+++
forward_dispatch_events_to_tui runs after run_shared_agent_bridge returns, so the output pane is blank for the whole agent turn and then fills at once.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes`

How to verify: Watch F3/F1 output during a long agent turn.

Verified 2026-09-28: (working tree; file mid-edit by a concurrent session) forward_dispatch_events_to_tui is still called only after `factory.run_shared_agent_bridge(request).await` returns, on both the buffered and the streaming dispatch paths; the 2026-09-28 continuation log leaves streaming agent output to portal plan 03.

Verified 2026-09-28 (static check against 3d0ee4d02): forward_dispatch_events_to_tui (graph_task_dispatch.rs:2699, doc comment: 'Called after run_shared_agent_bridge returns') is still invoked only after the provider call has completed: :3472 after the failover dispatch loop (run_shared_agent_bridge[_with_config] awaited at :4254-4260) and :3875 after `run_shared_agent_bridge(request).await` at :3868. The uncommitted diff of graph_task_dispatch.rs adds nothing that streams events during the turn. Same defect as gap-be0ac2.

**Blocked by dec-578863.** Streaming before the immune boundary is a trust/safety decision. Liveness is now shown via `agent_heartbeat` (every 5 s during a turn) and `agent_spawned` at dispatch start — the output pane still bursts, but the operator knows the agent is working throughout the turn.
