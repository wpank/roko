+++
id = "gap-be0ac2"
kind = "gap"
title = "Agent output is forwarded to the dashboard only after the agent finishes"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#B3"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`forward_dispatch_events_to_tui` (`graph_task_dispatch.rs:1230`) runs at `:1956` and `:3258`, after `run_shared_agent_bridge(request)` has returned (`:1922`, `:3251`).
The whole transcript arrives in one burst when the agent is already done; a watcher sees nothing while it works.
Consumers must also decode framed `roko.stream.v1` records (`runner/tui_bridge.rs`), and tool output over 2048 bytes is cut to its last 1024.
Fix: forward events while the dispatch runs. Planned in `plans/portal-programme/03-backend-live-events` T04; the portal decoder is `08-portal-run` T01.
