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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/measurement-validity.md"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs:1033", "crates/roko-agent/src/claude_cli_agent.rs::parse_stream_usage", "crates/roko-cli/src/graph_task_dispatch.rs:3759"]
links = { depends_on = [], blocks = [], related = ["gap-5d3b82", "q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn a_timed_out_attempt_reports_the_usage_it_streamed' crates/roko-agent/src/claude_cli_agent.rs && cargo test -p roko-agent --lib a_timed_out_attempt_reports_the_usage_it_streamed && grep -q 'fn dispatch_error_records_task_spend' crates/roko-cli/src/graph_task_dispatch.rs && cargo test -p roko-cli --lib dispatch_error_records_task_spend"
+++
- The Claude CLI agent's timeout path kills the process and returns a failure (`claude_cli_agent.rs:915`) before usage is parsed (`:933`). Cost comes only from the final `result` event.
- On the Graph path, a dispatch error returns (`graph_task_dispatch.rs:3436-3458`) before `task_spend.record` and the budget settlement, and the reservation is released with no spend.

So the most expensive failures, long runs that time out, look free in cost reports, budgets and learning.

Fix: parse whatever usage was streamed before the timeout, record spend on every exit path, and mark such costs as partial rather than zero.

Re-checked 2026-09-29: both parts still open; the line anchors moved. The Claude CLI non-zero-exit and turn-cap paths already carry the streamed result-event usage through failure_with_stream_usage (claude_cli_agent.rs:1056-1059); only the timeout and wait-failure arms (:1023, :1033) return zero usage, and cost_usd falls back to 0.0 (:508). Seen again on 2026-09-29: 08f-final-polish T05's first two attempts each ran 600 s and .roko/learn/costs.jsonl records cost_usd 0.0 for both. A fix should keep the streamed assistant events' message.usage and, when an attempt is killed, report their sum priced from the model table and marked as estimated (UsageSource), for every kill path (timeout, turn cap, cancellation).
