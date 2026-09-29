+++
id = "gap-437ccf"
kind = "gap"
title = "Published gate output omits the exit status, so silent failures show only the command"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:i1-portal-preview"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::published_gate_output", "crates/roko-cli/src/graph_task_dispatch/gate_output_accept.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn a_silent_failure_says_how_it_ended' crates/roko-cli/src/graph_task_dispatch/gate_output_accept.rs && cargo test -p roko-cli --lib graph_task_dispatch::gate_output_accept"

[closed]
at = 2026-09-29
commit = "63bdfa9a3"
evidence = "Committed in 63bdfa9a3 (plan 08f-final-polish, built by roko): published_gate_output now takes the Verdict and appends a 60-line/16 KiB output tail plus a closing '✗ exit status N' / '✗ terminated by a signal' line on failure (crates/roko-cli/src/graph_task_dispatch.rs, gate_how_ended); acceptance tests in crates/roko-cli/src/graph_task_dispatch/gate_output_accept.rs (a_silent_failure_says_how_it_ended and siblings). Static check of its verify passes; the cargo part was not re-run while other sessions hold the build lock."
+++

The gate output published for each verify step is `$ <command>` followed by the captured detail, if any (graph_task_dispatch.rs:4016-4017). A command that fails without printing anything, such as `test -f MISSING.md`, shows only `$ test -f MISSING.md` in the TUI and the portal, with no sign of how it failed.

Fix: append `exit <code>`, plus a stderr/stdout tail when there is one. Add a test named `gate_output_reports_exit_status`.

2026-09-29: fix in progress, uncommitted. The working tree has published_gate_output(command, &Verdict) with an output tail and a closing '✗ exit status N' line, plus tests in the untracked graph_task_dispatch/gate_output_accept.rs (plan 08f). None of the new tests is named gate_output_reports_exit_status, so the current [[verify]] will keep failing after the fix lands unless it is updated.

2026-09-29: the [[verify]] named gate_output_reports_exit_status, which the fix never added; it now runs the acceptance tests that plan 08f-final-polish added in graph_task_dispatch/gate_output_accept.rs.
