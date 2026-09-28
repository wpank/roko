+++
id = "gap-7f5eb6"
kind = "gap"
title = "CLI plan runs are invisible to roko serve: in-process StateHub and an unused IPC bridge"
status = "open"
triage = "verified"
severity = "p1"
goal = "visibility"
subsystem = ["roko-cli/state-hub", "roko-serve/events"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-audit/01-FINDINGS.md#A3"
discovered_from = "doc:tmp/portal-audit/01-FINDINGS.md"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs::start_hub_ipc_server", "crates/roko-runtime/src/state_hub.rs::SharedStateHub::new_in_process", "crates/roko-cli/src/graph_execution/plan_runner.rs:602"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -rn "start_hub_ipc_server(" crates --include="*.rs" | grep -v "crates/roko-cli/src/state_hub_ipc.rs" | grep -q .'
+++

`roko plan run` publishes into a per-process hub (`crate::state_hub::shared_state_hub()` -> `SharedStateHub::new_in_process()`: no event log, no socket), while `roko serve` builds its own hub in `crates/roko-serve/src/state.rs`.
The bridge that would join them, `start_hub_ipc_server` (`crates/roko-cli/src/state_hub_ipc.rs:114`), has no production caller, so a plan started from the CLI never appears on `/api/events` or in the portal.
Fix: give `plan run` a hub bridge (opt-in first) and let serve ingest bridged events.
Planned in `plans/portal-programme/03-backend-live-events` T06/T07, proven by T11.
