+++
id = "gap-4f3063"
kind = "gap"
title = "Sibling-settle re-verify sees only attempts in the same process and re-runs a failed step once"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w4a-reverify"
anchors = ["crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

When a verify step fails while sibling tasks are still writing, `sibling_settle` waits for those writers and re-runs the step. The in-flight registry (`InFlightTasks`) is an in-memory map, so two `roko plan run` processes on the same tree do not see each other's running attempts, and a failure caused by the other process's edits is charged to the task. The step is re-run only once: if a sibling's final edit still breaks the build, the task fails (and names the sibling via `blocked_by_sibling`) instead of waiting it out.

Fix: share in-flight state across processes (through the workspace hub or a lock directory), and decide whether to wait for later sibling edits.
