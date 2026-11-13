+++
id = "bug-addd24"
kind = "bug"
title = "roko plan status shows a working task as waiting until its try settles, and nothing shows the plan check while it runs: the report ignores attempt-open lines and the run's plan_check_started events"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "24871a613"
source = "round v8 lead and guide-v8: phase 7's live terminal frames (Undertow p7-ship, run 31e52490)"
discovered_from = "agent:guide-v8 (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::diagnose_tasks", "crates/roko-cli/src/plan_status_view.rs::PlanStatus"]
links = { depends_on = [], blocks = [], related = ["bug-2e5023"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_shows_what_is_running' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_shows_what_is_running"

[closed]
at = 2026-11-13
at_ts = "2026-11-13T00:24:13Z"
commit = "24871a613"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-12T23:32:59Z"
forced = false
evidence = "24871a613 on work/bug-addd24: roko plan status and roko diagnose show a run in progress as it is. A task whose try started reads running: the report reads the run's attempt-open lines, and an open line with no verdict of the same attempt key is the task's open try ('running · 40s', 'try 2 · running · 5s' from the second try). While the run goes, the report reads the run's event index for a plan_check_started with no plan_check_finished, and the table adds 'plan check <command> checking'. Red first: plan_status_shows_what_is_running failed on 134dc0467 with '○ task 2  Wire the parser  waiting' while its try was open, as in guide-v8's phase 7 frames; it passes now. Also: is_ancestor's merge-base takes --no-optional-locks, and plan_status_writes_nothing compares inodes and times too. On an APFS copy of Undertow's phases 5-7 (git behind a logging shim), roko-item10a's finished-run screens equal roko-item8a's apart from times, no index's inode or mtime changed, and every git call passed --no-optional-locks (repro/item10-vs-item8a.txt). Branch checks: nightly fmt; roko-cli lib 4153 passed, 0 failed; bin tests (diagnose, plan_status, status) 21 passed; workspace clippy -D warnings clean; the binary builds (bin/roko-item10a)."
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
