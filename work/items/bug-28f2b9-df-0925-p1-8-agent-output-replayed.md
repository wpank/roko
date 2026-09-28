+++
id = "bug-28f2b9"
kind = "bug"
title = "DF-0925 P1-8: Agent output replayed in one burst after the agent finishes"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
forward_dispatch_events_to_tui runs after run_shared_agent_bridge returns, so the output pane is blank for the whole agent turn and then fills at once.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-8. Agent output is replayed in one burst after the agent finishes`

How to verify: Watch F3/F1 output during a long agent turn.

Verified 2026-09-28: (working tree; file mid-edit by a concurrent session) forward_dispatch_events_to_tui is still called only after `factory.run_shared_agent_bridge(request).await` returns, on both the buffered and the streaming dispatch paths; the 2026-09-28 continuation log leaves streaming agent output to portal plan 03.
