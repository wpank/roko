+++
id = "bug-a362b8"
kind = "bug"
title = "A terminal plan run that finishes reads as stopped: the dashboard closes itself when the plan set completes, roko takes that for the operator quitting, prints 'stopped by closing the TUI … resume with --resume-plan' and exits 130 after delivering the work"
status = "open"
triage = "verified"
severity = "p1"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/graph_execution", "roko-cli/tui"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::pending_interrupt", "crates/roko-cli/src/graph_execution/plan_runner.rs::TuiSession", "crates/roko-cli/src/tui/app/channels.rs::update_plan_completion_exit", "crates/roko-cli/src/tui/app/event_loop.rs::run"]
links = { depends_on = [], blocks = [], related = ["gap-0d6ae5", "bug-415db3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_tui_that_closes_itself_at_the_end_does_not_stop_the_run' crates/roko-cli/src && cargo test -p roko-cli --lib a_tui_that_closes_itself_at_the_end_does_not_stop_the_run"
+++

## Problem

A plan run on a terminal opens the full-screen dashboard (the TUI). When the plan set completes, the TUI closes
itself, and roko reads that as the operator quitting. So a run that passed and delivered its work ends with:

```
Graph Engine stopped by closing the TUI: 1/1 plan(s) attempted, 3 task(s), $0.44; resume with --resume-plan
…
Plan greeting delivered into roko/batch/graph-42f42492-… at 8580e4b…
```

It exits 130, the code for an interrupt. A failed plan exits 130 too, instead of 1. Seen on both terminal runs of
round v8's $0 captures at a0089c19c:
- `tmp/hermes-roko-v8/design/terminal/before/run-plan.tty.*`;
- `tmp/hermes-roko-v8/design/terminal/before/run-broken.tty.*`.

Expected: the run reports how it ended (complete, or failed with exit 1), and asks for no resume.

## Why it matters

Goal visibility. `roko plan run` and `roko run plans/<dir>` on a terminal are the interactive paths a person uses
first, and both lie about a good run. Scripts that run them on a pty see 130.

## Where

- `crates/roko-cli/src/tui/app/channels.rs::update_plan_completion_exit` sets `running = false` once the plan set
  is terminal (`with_exit_on_plan_completion`, set by the runner).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `TuiSession::closed_by_operator` treats any `Ok(())` from the TUI thread as a quit;
  - `pending_interrupt` then records `PlanRunInterrupt::TuiClosed` (exit 130,
    `PlanRunInterrupt::exit_code`). The plan-set driver loop calls it on every pass, including the passes after
    the plans finished, while delivery runs.

## Current state

`closed_by_operator` came with bug-415db3's fix (725f21e05): a TUI closed mid-run should stop the run rather than
leave a silent terminal. The TUI's own exit at completion was never told apart from a quit.

## Plan

1. `App::run_to_exit` returns why the TUI closed:
   - `TuiExit::Quit`, when the person quit;
   - `TuiExit::Finished`, when the plan set completed or the run finished;
   - `TuiExit::Stopped`, when its owner asked it to stop.
   `App::run` keeps its signature.
2. The plan runner's TUI thread uses `run_to_exit`. `closed_by_operator` is true only for `Quit`. A TUI that
   finished is joined, and stderr goes back to the terminal at once, so delivery's messages show.
3. A test drives `pending_interrupt` with a finished TUI (no stop) and a quit (`TuiClosed`).

## Done when

- A terminal run whose plan passes ends with its normal summary and exit 0, and a failed one with exit 1.
- `a_tui_that_closes_itself_at_the_end_does_not_stop_the_run` passes.
- The `[[verify]]` passes.

## Notes

- Related: gap-0d6ae5 (closing the TUI mid-run stops the run). Round v8 makes the inline live view the terminal
  default, with the TUI behind `--tui`; this bug still applies there.
