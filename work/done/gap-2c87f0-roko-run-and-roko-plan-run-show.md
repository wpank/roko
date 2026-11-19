+++
id = "gap-2c87f0"
kind = "gap"
title = "roko run and roko plan run show no readable live progress or summary: a terminal gets the full-screen dashboard, a pipe gets 'executing node' lines and 'Graph Engine complete: 1 plan(s)', roko run prints two summaries, and no run ends with its time, checks and one next step"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "L"
subsystem = ["roko-cli/graph_execution", "roko-cli/run"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "0c4503198"
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/graph_execution/plan_runner.rs::InlineProgressTelemetrySink", "crates/roko-cli/src/commands/run_cmd.rs", "crates/roko-cli/src/run.rs::print_workflow_run_report"]
links = { depends_on = ["gap-cf3940"], blocks = [], related = ["gap-0d6ae5", "bug-a362b8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_finished_run_ends_with_time_spend_checks_and_next' crates/roko-cli/src && cargo test -p roko-cli --lib run_view"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T13:01:00Z"
commit = "0c4503198"
by = "roko-recovery"
executor = "claude-agent"
size = "L"
claimed_at = "2026-11-12T11:17:11Z"
forced = false
evidence = "0c4503198 on work/gap-2c87f0: graph_execution::run_view shows a run in roko's design system, on a thread of its own: a header, one line per task redrawn in place on a terminal (only what changed, synchronized, cursor hidden) over a glass footer, one appended line per event in a pipe, CI or TERM=dumb, and a summary on stdout with Done/Failed/Stopped, tasks, checks, time, spend of the cap, where the work is (+added −removed in N files), notes and one Next command; the dashboard is opt-in (--tui); roko run prints one summary; serve-forwarded runs use the same view. Branch checks: nightly fmt; roko-cli lib 4119 passed (the run view's unit and insta snapshot tests, a_finished_run_ends_with_time_spend_checks_and_next among them), bin 481 passed (cli_parses_the_dashboard_flag), plan_branch_integration 5 of 5; workspace clippy -D warnings clean; the binary builds; Docs Lint's link check passes. $0 captures on that binary at 80x24, 100x32 and 120x40, NO_COLOR, pipe, CI, a failing plan, a one-task run and a run forwarded to roko serve: every exit code right, and no recorded frame wraps or tears (tmp/hermes-roko-v8/roko-recovery/runs/item3c/, checks.txt)."
+++

## Problem

What `roko run` and `roko plan run` print while a plan runs, and when it ends, is hard to read.

**On a terminal**, `roko plan run` and `roko run plans/<dir>` open the full-screen dashboard. At 100x32 it shows
11 panels, and some read wrong:
- "COMPLETE" at 1 of 3 tasks;
- empty agent and route tables while agents run;
- toasts over the panels;
- "~30m" left on a 17-second run.

**In a pipe or CI**, a task gets three kinds of line: `▸ executing node 'T1'...`, `✓ node 'T1' completed (5.3s)`
and `✗ node 'T1' failed: <the check's raw stderr>`. Tries, checks and retries never show. The end is:
- "Graph Engine complete: 1 plan(s), 3 task(s), $0.44";
- the run id twice;
- a rung line of 9 statistics;
- attempt branch names and full SHAs;
- no elapsed time, no check count.

**`roko run "<prompt>"`** prints two summaries one after the other: the Graph one, then a clack-style report.

**A failed run** ends with no next step.

Captures: `tmp/hermes-roko-v8/design/terminal/before/`. The proposal and mocks:
`tmp/hermes-roko-v8/design/terminal/README.md`, `compare.html`, `after/`.

## Why it matters

Goal visibility. This is what a person watches while roko works, and what round v8's guide will screen-record. Will
asked for it to be impressive, streamlined and intuitive. The lead approved the design on 2026-11-12: on a
terminal, the inline live view replaces the dashboard as the default, and the dashboard stays behind `--tui` and
in `roko dashboard`.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `run_graph_plan_body` chooses the view (`launch_tui`);
  - `InlineProgressTelemetrySink` prints the node lines;
  - the closing summary is printed near the JSON report.
- `crates/roko-cli/src/commands/run_cmd.rs` (`run_one_task`, `run_prompt_plan`) and
  `crates/roko-cli/src/run.rs::print_workflow_run_report` (the second summary).
- The data: the `DashboardEvent`s the TUI already receives through the StateHub:
  - `TaskStarted`, `AgentOutput`, `AttemptSettled`, `RetryFeedback`, `CallCost`;
  - `GateResult`, `PlanCheckStarted`/`Finished`, `DeliveryFinished`, `RunCompleted`.

## Plan

1. A run view built on item 2's style. It subscribes to the StateHub like the TUI does, and shows:
   - a header: `◆ roko  run <plan>`, plus a dim line with the task count, parallelism, model and cap;
   - one line per task: its state glyph, id, title, what it is doing now (the tool call, or the check), and its
     time;
   - a dim line under a task when a try fails and roko tries again;
   - a footer: a progress bar, then done/total, elapsed time and spend, on black glass.
2. **On a terminal:** redraw in place, only the lines that changed, at most about 10 times a second. Nothing may
   wrap at 80 columns: titles are cut to fit with `…`.
3. **In a pipe, CI or TERM=dumb:** append one line per event, the same words in the same order, nothing redrawn.
4. **When the run ends:**
   - freeze the task lines with their tries, time and spend;
   - show the plan check;
   - one result line: `✓ Done  3/3 tasks · 7 checks passed · 16.9s · $0.44 of $3.00`, or `✗ Failed …`;
   - where the work went;
   - `Next`, a command the person can paste: `git merge --ff-only <branch>`, or `roko diagnose <plan>`.
5. `roko run "<prompt>"` uses the same view, with one summary. The plan-first path shows the plan being written as
   a step of the same view.
6. `--tui` opens the dashboard as today.
7. `--json` is unchanged: stdout stays pure JSON, and progress goes to stderr.
8. Update CLAUDE.md's CLI tables and `docs/v3/28-CLI.md`.

## Done when

- A terminal run shows the live view and ends with the summary and a Next line, at 80x24, 100x32 and 120x40, with
  nothing wrapped. A pipe gets the appended lines. `--tui` gets the dashboard.
- The renderer's exact-text tests pass. The integration tests that matched the old strings ("Plans that did not
  succeed", `git merge --ff-only`) match the new ones.
- The `[[verify]]` passes.

## Notes

- Depends on item 2's style module.
- Closes the part of gap-0d6ae5 that is about the terminal default.
