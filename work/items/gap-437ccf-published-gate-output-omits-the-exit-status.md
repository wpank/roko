+++
id = "gap-437ccf"
kind = "gap"
title = "Published gate output omits the exit status, so silent failures show only the command"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:i1-portal-preview"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:4016"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -rq "fn gate_output_reports_exit_status" crates/roko-cli/src && cargo test -p roko-cli --lib gate_output_reports_exit_status'
+++

The gate output published for each verify step is `$ <command>` followed by the captured detail, if any (graph_task_dispatch.rs:4016-4017). A command that fails without printing anything, such as `test -f MISSING.md`, shows only `$ test -f MISSING.md` in the TUI and the portal, with no sign of how it failed.

Fix: append `exit <code>`, plus a stderr/stdout tail when there is one. Add a test named `gate_output_reports_exit_status`.
