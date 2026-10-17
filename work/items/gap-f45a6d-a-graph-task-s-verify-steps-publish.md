+++
id = "gap-f45a6d"
kind = "gap"
title = "A Graph task's verify steps publish no output while they run: gate_output_line is never sent, so a failing test cannot scroll by live"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-gate/shell", "roko-core/dashboard"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "team-lead, round v6 (2026-10-17): spec-v3 P4 and D4 need gate_output_line on the Graph path"
discovered_from = "hermes-roko-v6 round: R6, what streams live"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-gate/src/shell.rs::ShellGate", "crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_check_streams_its_output_as_it_runs' crates/roko-cli && cargo test -p roko-cli --lib a_check_streams_its_output_as_it_runs"
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
