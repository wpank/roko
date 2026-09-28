+++
id = "gap-7b731e"
kind = "gap"
title = "dispatch_v2::run_agent_result_bridge (provider-neutral events) not wired into runner"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-cli/src/dispatch_v2.rs:1563"
discovered_from = "audit:crates/roko-cli/src/dispatch_v2.rs:1563"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::run_agent_result_bridge"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
run_agent_result_bridge returns provider-neutral events with optional pid but is 'not wired into runner v2 yet' because runner v2's Started event requires an OS pid. Decide whether Graph engine should use it or delete it with Runner-v2 removal.

Imported without verification from:
- `crates/roko-cli/src/dispatch_v2.rs:1563`

How to verify: grep callers of run_agent_result_bridge in graph_execution/ and runner/.
