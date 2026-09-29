+++
id = "bug-652bb7"
kind = "bug"
title = "Field capture can snapshot a resumed run twice, and the rollup's by-run table is not in date order"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/field-tools"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "cac54e574"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:39, wk-rollup's report on bug-7b37c4)"
anchors = ["tmp/cybernetic-harness/tools/field_capture.py::run_pass", "tmp/cybernetic-harness/tools/field_capture.py::capture_run", "tmp/cybernetic-harness/tools/field_rollup.py::load_summaries", "tmp/cybernetic-harness/tools/field_rollup.py::main"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = ["bug-7b37c4", "bug-469537", "gap-29a64e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_resumed_run_counts_once' tmp/cybernetic-harness/tools/test_field_rollup.py && python3 tmp/cybernetic-harness/tools/test_field_rollup.py -k test_resumed_run_counts_once"

[[verify]]
command = "grep -q 'def test_by_run_rows_are_in_date_order' tmp/cybernetic-harness/tools/test_field_rollup.py && python3 tmp/cybernetic-harness/tools/test_field_rollup.py -k test_by_run_rows_are_in_date_order"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "A resumed run is captured once and the rollup's by-run table is in date order (tmp, wk-field-tools): test_resumed_run_counts_once and test_by_run_rows_are_in_date_order pass. The watcher was restarted on the new code."
+++

## Problem

Two defects in the field-evidence pipeline:

1. **A resumed run can be counted twice.** `field_capture.run_pass` keys its index by
   `workspace|plan_id|run_id|status|updated_at_ms` (`field_capture.py:375`). A resumed run that finishes later has a new
   `updated_at_ms`, so it is captured again. `capture_run` names the snapshot directory after the day the run ended
   (`snapshots/<day>/<workspace>__<plan>__<run8>`, `:337-339`). A second capture on the same day overwrites the first; a
   capture on a later day adds a second directory for the same run. `field_rollup.load_summaries` (`:45-68`) reads every
   `summary.json` without deduplicating by run, so that run's tasks, attempts, interventions and cost count twice.
2. **The "By run" table is out of order.** `field_rollup.main` sorts runs by `end` alone, with a missing `end` sorting
   as the empty string (`:151`). The Date column shows `end`, or `start`, or the snapshot folder's date (`:164`). Runs
   with no `end` therefore sort first, so `evidence/field/ROLLUP.md` lists 2026-09-14 and 2026-09-15 rows before a run
   dated 2026-08-22 (`cli-ux-consistency`).

## Why it matters

The rollup is the source of the whitepaper's field numbers (§7: 42 runs, autonomy index 2/41) and of the development
record (epic spec-f2463d). A double-counted run would change them silently, and the out-of-order table misleads anyone
reading the evidence.

## Where

- `tmp/cybernetic-harness/tools/field_capture.py`: `run_pass` (the index key) and `capture_run` (the snapshot path).
- `tmp/cybernetic-harness/tools/field_rollup.py`: `load_summaries` and the sort in `main`.
- Tests: `tmp/cybernetic-harness/tools/test_field_rollup.py` (stdlib unittest, each test in its own temporary field
  directory; `python3 tmp/cybernetic-harness/tools/test_field_rollup.py -k PATTERN`).

## Current state

Checked in MAIN on 2026-09-29: 42 snapshots and 42 distinct (plan, run) pairs, so no run has been counted twice yet.
The ordering defect is visible in today's `ROLLUP.md`.

## Plan

1. **Capture:** key the index by workspace, plan and run. When a run is captured again, replace its earlier snapshot
   directory instead of adding one.
2. **Rollup:** deduplicate by (plan_id, run_id), keeping the latest capture, and print a warning when it drops one, so
   snapshots taken before the fix are handled too.
3. **Order:** sort by the same value the Date column shows.
4. Add the tests `test_resumed_run_counts_once` (two snapshots of one run on different days count once) and
   `test_by_run_rows_are_in_date_order`.

## Done when

- [ ] A run captured twice appears once in `ROLLUP.md` and `rollup.json`.
- [ ] The "By run" table is in date order.
- [ ] Both `[[verify]]` commands pass, and the existing tests in `test_field_rollup.py` still pass.

## Notes

- These files are untracked (`tmp/cybernetic-harness/`), so edit them in place in the main checkout.
- A capture watcher may be running (`evidence/field/.capture.pid`). Restart it after the fix so it loads the new code,
  as was done for bug-7b37c4.
- Don't edit the frozen copies in `docs/whitepaper/evidence/`.
