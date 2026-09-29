+++
id = "bug-bfdb9a"
kind = "bug"
title = "New dashboard event types are silently dropped on the way to serve's event stream"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/bridge"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/spec-drift-from-commits.md"
anchors = ["crates/roko-serve/src/lib.rs::dashboard_event_to_server", "crates/roko-serve/src/lib.rs::server_event_to_dashboard"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`dashboard_event_to_server` (`roko-serve/src/lib.rs:1919-2042`) and `server_event_to_dashboard` (`:1579-1669`) both end in `_ => None`. A new `DashboardEvent` variant compiles fine and never reaches SSE clients (the portal) unless someone remembers to add a bridge arm. `PlanSetLoaded` was bridged by hand in 725f21e05.

Fix: make both matches exhaustive, so adding a variant fails to compile until it is mapped or explicitly ignored.
