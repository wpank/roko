+++
id = "bug-b589fe"
kind = "bug"
title = "A plan-check hand-back's try is not reported when no 100 ms tick sees it run: its watch starts from the plan's last statuses, where its task already reads complete, so the try's end is no change and the hub and .roko/events.jsonl never get its task_completed"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "06dcdf980"
source = "round v8 lead: release PR #89's Test + Clippy job (run 44524474195) and coverage job (run 44518589408) failed a_failed_plan_check_goes_back_to_the_task_whose_file_it_names at plan_runner.rs:11662"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::rerun_handed_back_tasks", "crates/roko-cli/src/graph_execution/plan_runner.rs::FlowWatch"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["find-5bcba7", "bug-58a1d6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn for_handback_try' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_handback_try_that_no_tick_saw_is_reported a_failed_plan_check_goes_back_to_the_task_whose_file_it_names a_plan_check_is_handed_back"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T15:02:33Z"
commit = "06dcdf980"
forced = false
evidence = "06dcdf980 on work/bug-b589fe. PR #89's CI failed a_failed_plan_check_goes_back_to_the_task_whose_file_it_names at plan_runner.rs:11662 in Test + Clippy (run 44524474195) and coverage (run 44518589408): .roko/events.jsonl held one task_completed for T1, not two, while the attempt log had both verdicts; the run awaits its event log's finish before returning, so the event was never published, not half-read. Cause: rerun_handed_back_tasks started the try's FlowWatch from the plan's last statuses, where T1 already read complete from its try before, with completions held until the try's graph gives their verdicts (bug-58a1d6); when no 100 ms tick saw the try running, its end was complete after complete, no change, and no task_started or task_completed was published. Fix: FlowWatch::for_handback_try starts the try's watch without its task, so its end is always reported with its verdict; FlowWatch::report_end is the end diff moved out unchanged. Red first: a_handback_try_that_no_tick_saw_is_reported got no task_completed (left []) on the seam that kept the task, while the two hand-back tests passed on the refactor. Green: it passes; every plan_runner test passes (99); the hand-back test passed 12 of 12 run 4 at a time; workspace clippy clean."
+++

## Problem

A whole-plan check that fails goes back to the task whose file it names as a hand-back try (find-5bcba7). The try runs
in a graph of its own, `rerun_handed_back_tasks`, watched by a `FlowWatch` that publishes the try's task events to the
run's hub. A completion is held until the try's graph gives its verdict (bug-58a1d6), and the graph's end publishes
what the ticks did not.

The watch starts from the plan's last statuses, in which the task already reads complete from its try before. A tick
every 100 ms compares the graph's statuses with that view. If no tick sees the try running, because it ended within the
first sleep or the sleep overshot on a loaded machine, the try's end is complete after complete: no change. Nothing is
published, neither a task_started nor a task_completed for the hand-back's try. The run's attempt log has the try's
verdict, but the hub and `.roko/events.jsonl` miss the try.

Seen on 2026-11-19 in release PR #89's CI: `a_failed_plan_check_goes_back_to_the_task_whose_file_it_names` failed at
`plan_runner.rs:11662`, the events.jsonl check, with left ["passed"], right ["passed", "passed"]. It failed in the Test +
Clippy job (run 44524474195, shared tree) and in the coverage job (run 44518589408). Its attempt-log check just before
passed. `run_graph_plan_observed` awaits the event log's finish before it returns, so the file was complete. The event
was never published.

## Why it matters

Goal core. The live views and the records built from the hub show a hand-back's try as never having run: the terminal
run view, the dashboard, serve's run summary, and the portal and plugin run cards. The try's verdict is still in the
attempt log, so the two records disagree. It is also the last red test of PR #89 on Linux.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::rerun_handed_back_tasks`: `FlowWatch::new(statuses).holding_completions()`.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::FlowWatch`.

## Plan

1. `FlowWatch::for_handback_try` starts the try's watch without its task, which starts over. Its completions stay held.
2. `FlowWatch::report_end` publishes, at the graph's end, what the ticks did not, as before.
3. Test `a_handback_try_that_no_tick_saw_is_reported`: a try that started and ended between ticks is reported with its
   verdict. It fails before the fix.

## Done when

A hand-back's try is reported whether or not a tick saw it run, and the `[[verify]]` passes.
