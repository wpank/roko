+++
id = "bug-addd24"
kind = "bug"
title = "roko plan status shows a working task as waiting until its try settles, and nothing shows the plan check while it runs: the report ignores attempt-open lines and the run's plan_check_started events"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 lead and guide-v8: phase 7's live terminal frames (Undertow p7-ship, run 31e52490)"
discovered_from = "agent:guide-v8 (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::diagnose_tasks", "crates/roko-cli/src/plan_status_view.rs::PlanStatus"]
links = { depends_on = [], blocks = [], related = ["bug-2e5023"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_shows_what_is_running' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_shows_what_is_running"
+++

## Problem

In round v8's phase 7 run (Undertow `p7-ship`, run 31e52490), guide-v8's terminal capture polled `roko plan status`
every 2 s, and every mid-run frame understated the run:

- **A working task read "waiting".** Task 1 read "○ waiting" from 22:58:49 until its try settled at about 23:00:30.
  Task 2 read "waiting" until 23:01:50, though it ran from 23:00:35.
- **The plan check was invisible.** While `tools/check.sh --plugin` ran, from about 23:02:20 to 23:04:59, the view
  showed only "running · 2/2 tasks" with both tasks passed.

The report's records know both:

- **Tries:** `attempts.jsonl` gets an attempt-open line when a try starts (T2's at 23:00:35.953) and a verdict when
  it settles. `diagnose_tasks` counts a task as attempted only once it has a cost row, a gate failure, an episode or
  a verdict, and none exists until the try settles.
- **The plan check:** the run's event index (`.roko/events-by-run/<sha256(run id)>.jsonl`) gets `plan_check_started`
  and `plan_check_finished` as lifecycle events, synced and flushed, with the run id and time on each. The view
  draws the plan check only from the delivery record, which is written when the delivery ends.

## Why it matters

Goal tooling. The status view is how a person watches a run they started elsewhere, from Hermes, the portal or
another terminal. A view that says "waiting" for the task that's working, and says nothing while the plan check
runs, tells them the run is stuck. It is also the guide's live terminal frame for phase 7.2.

## Where

- `crates/roko-cli/src/commands/diagnose.rs`: `RunRecords::load`, `diagnose_tasks` and `run_model`.
- `crates/roko-cli/src/plan_status_view.rs::PlanStatus::model`.

## Plan

1. Red test first: a running plan whose task 1 passed, task 2 has an open try and no verdict, and task 3 waits.
   `roko plan status` shows task 2 running, with its try and time. Then, with every task passed and the plan check
   started (and not finished) in the run's event index, a plan check row reads checking.
2. The report keeps each task's open try: its open line, with no verdict for that attempt. A task with one is
   attempted, so it reads running while the run goes and stopped after.
3. The report reads the run's event index for a plan check that started and hasn't finished, and the table adds
   its row (checking) while the run goes.

## Done when

The test passes, the `[[verify]]` passes, and a replay of phase 7's records at 23:01:30 shows task 2 running.
