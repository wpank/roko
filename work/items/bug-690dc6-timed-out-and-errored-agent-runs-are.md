+++
id = "bug-690dc6"
kind = "bug"
title = "Timed-out and errored agent runs are recorded at $0 with no tokens"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-agent/claude-cli", "roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs:915", "crates/roko-cli/src/graph_task_dispatch.rs:3436"]
links = { depends_on = [], blocks = [], related = ["gap-5d3b82", "q-1faa0c"], supersedes = [], duplicate_of = "" }
+++
- The Claude CLI agent's timeout path kills the process and returns a failure (`claude_cli_agent.rs:915`) before usage is parsed (`:933`). Cost comes only from the final `result` event.
- On the Graph path, a dispatch error returns (`graph_task_dispatch.rs:3436-3458`) before `task_spend.record` and the budget settlement, and the reservation is released with no spend.

So the most expensive failures, long runs that time out, look free in cost reports, budgets and learning.

Fix: parse whatever usage was streamed before the timeout, record spend on every exit path, and mark such costs as partial rather than zero.
