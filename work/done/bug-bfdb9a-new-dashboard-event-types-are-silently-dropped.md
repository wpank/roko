+++
id = "bug-bfdb9a"
kind = "bug"
title = "New dashboard event types are silently dropped on the way to serve's event stream"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/bridge"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "8a3c530af"
source = "tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
anchors = ["crates/roko-serve/src/lib.rs::dashboard_event_to_server", "crates/roko-serve/src/lib.rs::server_event_to_dashboard"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/^fn dashboard_event_to_server/,/^}/p' crates/roko-serve/src/lib.rs | grep -qE '^        _ => None' && ! sed -n '/^fn server_event_to_dashboard/,/^}/p' crates/roko-serve/src/lib.rs | grep -qE '^        _ => None'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T15:16:13Z"
by = "coordinator (session 7622b882)"
claimed_at = "2026-10-01T09:06:28Z"
forced = false
evidence = "Batch 20f gate on 2ff1b7891 (MAIN has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/core/graph/serve; lib tests roko-cli 3309, roko-agent 2296, roko-core 1963, roko-serve 992, roko-graph 480 pass; extras: all eight canaries + golden_path_suite + secret_canary 11/11 + C2 2/2 + worktree_task_diff + default_engine pass, bin 429, graph_task_dispatch suite at --test-threads=32 passed 10 of 10; dashboard_event_to_server and the server event bridge list every variant (no catch-all None). Merged (work/reg-cbfff6 fc2430b99)."
+++
`dashboard_event_to_server` (`roko-serve/src/lib.rs:1919-2042`) and `server_event_to_dashboard` (`:1579-1669`) both end in `_ => None`. A new `DashboardEvent` variant compiles fine and never reaches SSE clients (the portal) unless someone remembers to add a bridge arm. `PlanSetLoaded` was bridged by hand in 725f21e05.

Fix: make both matches exhaustive, so adding a variant fails to compile until it is mapped or explicitly ignored.

## Notes

Implemented on `work/reg-cbfff6` at `daa8a3324`; cargo verification deferred to the batch check. The `[[verify]]` grep passes there.

Both matches are exhaustive. Each variant that is not bridged is listed, with a comment that gives the reason. Newly bridged from the dashboard to the server stream: `TaskBlocked` (as `ServerEvent::Execution` with `ExecutionEvent::TaskBlocked`, for gap-f59fe9), `ChainBlock`, `ChainTx`, `FeedTick`, `FeedAgentOnline` and `FeedAgentOffline`. `RunCompleted` stays behind because `ServerEvent::RunCompleted` needs a run id the dashboard event lacks. `ChainContractEvent` stays behind because it does not say whether the raw log evidence was published. 30 dashboard-only variants are listed too. In the other direction, `ExecutionEvent::TaskBlocked` maps back. `PlanStarted`, `PlanCompleted`, `ReplanTriggered` and `WatcherAlert` are listed as not bridged, as are 50 server variants. `BridgeDedup` already skips the seqs the other bridge produced, so the new arms do not echo the feed events that feed agents publish on the bus. Test: `newer_dashboard_events_bridge_both_ways`.
