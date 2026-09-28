+++
id = "gap-be0ac2"
kind = "gap"
title = "Agent output is forwarded to the dashboard only after the agent finishes"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#B3"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::forward_dispatch_events_to_tui", "crates/roko-cli/src/graph_task_dispatch.rs:3472", "crates/roko-cli/src/graph_task_dispatch.rs:3875", "crates/roko-cli/src/runner/tui_bridge.rs:13"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`forward_dispatch_events_to_tui` (`graph_task_dispatch.rs:1230`) runs at `:1956` and `:3258`, after `run_shared_agent_bridge(request)` has returned (`:1922`, `:3251`).
The whole transcript arrives in one burst when the agent is already done; a watcher sees nothing while it works.
Consumers must also decode framed `roko.stream.v1` records (`runner/tui_bridge.rs`), and tool output over 2048 bytes is cut to its last 1024.
Fix: forward events while the dispatch runs. Planned in `plans/portal-programme/03-backend-live-events` T04; the portal decoder is `08-portal-run` T01.

Verified 2026-09-28 (static check against 3d0ee4d02): Line numbers moved but the shape is unchanged: forward_dispatch_events_to_tui is defined at graph_task_dispatch.rs:2699 and called at :3472 and :3875, only after run_shared_agent_bridge has returned (:3868, and :4254-4260 in the failover loop). Tool output over 2048 bytes is still cut to its last 1024 (:2741-2743, a byte slice `&output[output.len() - 1024..]` that can also panic on a non-UTF-8 char boundary), and runner/tui_bridge.rs:13 still frames records with the `roko.stream.v1` prefix. Duplicate of bug-28f2b9.
