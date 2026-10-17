+++
id = "gap-f45a6d"
kind = "gap"
title = "A Graph task's verify steps publish no output while they run: gate_output_line is never sent, so a failing test cannot scroll by live"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-gate/shell", "roko-core/dashboard"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "f0716d418"
source = "team-lead, round v6 (2026-10-17): spec-v3 P4 and D4 need gate_output_line on the Graph path"
discovered_from = "hermes-roko-v6 round: R6, what streams live"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-gate/src/shell.rs::ShellGate", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_streams_its_output_as_it_runs' crates/roko-cli && cargo test -p roko-cli --lib a_check_streams_its_output_as_it_runs"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T09:52:01Z"
commit = "f0716d418"
by = "commit trailer"
executor = "unknown"
forced = false
evidence = "f0716d418: a Graph task's verify steps publish each line their command writes as gate_output_line while they run (graph_task_dispatch/check_output.rs, one forwarder per step, in the first run and the re-run after an auto-fix): plan_id, task_id, attempt (from 1), gate (the step), line (at most 500 characters), stream (stdout, stderr, or roko for roko's note of what it skipped); bounded per step by a burst of 80 lines, then 40 a second, and 1,000 in all; all before the step's gate_result. roko-gate: ShellGate::with_output_sink tags each line with its stream. serve: /api/events plan and task filters include the lines; /health feature check_output_lines; docs/v3/26-HTTP-API.md. Verify: a_check_streams_its_output_as_it_runs, a_chatty_check_is_bounded, the_budget_allows_a_burst_then_a_rate_within_a_cap and output_sink_tags_each_line_with_its_stream pass. On 5b59e6dca (main 9df2f5948 merged in; main's later commits change no crate): fmt; clippy -D warnings clean on roko-core, roko-gate, roko-runtime, roko-cli and roko-serve; lib tests roko-cli 3766 (7 ignored, gist skipped), roko-core 2037, roko-gate 705, roko-runtime 213, roko-serve 1050 with 2 timeouts in service_factory's fake-CLI gateway tests under the loaded parallel run, which pass alone (2 of 2); plan_branch_integration 5; scratch HOME. Scripted showcase, $0 (tmp/hermes-roko-v6/roko-delivery/evidence/f45a6d): p1-slugify's two failing attempts each stream the unittest failure, 30 lines on stderr, before gate_result."
+++

## Problem

A Graph task's verify steps publish nothing while they run. `DashboardEvent::GateOutputLine` (`gate_output_line`)
and `TuiBridge::gate_output_line` exist, but only tests build the event: `run_verify_steps` runs each step through a
`ShellGate` with no line sink, and the step's output reaches the hub once, at its end, in `gate_result`. A failing test
cannot scroll by as it runs (spec-v3 §6 "check output line", P4 and D4), and a long check reads as silence.

## Why it matters

Goal `hermes`: the plugin's live stream and route sheet show a check's output as it happens; with R6 (tool steps and
words live) it is the last part of an attempt that is not.

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps`: builds each step's `ShellGate`, in the
  first run and in the re-run after an auto-fix.
- `crates/roko-gate/src/shell.rs::ShellGate`: has a line sink, without the stream each line came from.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent::GateOutputLine`: no attempt, no stream.

## Plan

1. `ShellGate::with_output_sink`: each line of stdout and stderr, with its stream (`OutputLine`, `OutputStream`).
2. `GateOutputLine` gains `attempt` and `stream`.
3. A forwarder per verify step publishes its lines as they come, bounded: a burst, then a rate per second, a cap per
   step, each line cut to a length; a last line from roko (`stream: "roko"`) counts what it skipped; all before the
   step's `gate_result`.
4. serve: `/api/events` plan and task filters include it; `/health` feature `check_output_lines`; docs.

## Done when

The `[[verify]]` test passes: a verify step's lines are published with their streams and attempt before its
`gate_result`; a unit test shows a chatty check bounded.
