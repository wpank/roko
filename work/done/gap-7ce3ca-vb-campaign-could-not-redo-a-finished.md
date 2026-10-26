+++
id = "gap-7ce3ca"
kind = "gap"
title = "vb campaign could not redo a finished unit whose run an infrastructure failure spoiled: moving it to .abandoned left the unit finished"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "ccc19e6aa"
source = "gap-327242 worker (claude-agent), 2026-10-20: Pilot B unit roko-fixed-s1-2"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/campaign.py::cmd_campaign", "benchmarks/viabilitybench/driver/test_campaign.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242", "bug-c6318e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_a_finished_unit_whose_run_was_moved_aside_runs_again' benchmarks/viabilitybench/driver/test_campaign.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -q -p no:cacheprovider -k 'moved_aside_runs_again'"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T16:05:55Z"
commit = "ccc19e6aa"
forced = false
evidence = "ccc19e6aa: campaign.cmd_campaign counts a finish event with exit 0 only while its run directory is still in the experiment directory, so a finished unit whose run was moved to <experiment>.abandoned/ runs again under the next run id; the Resuming paragraph says so. Needed for Pilot B (gap-327242): roko-fixed-s1-2 finished with 9 tasks spoiled by Cerebras 402s. Verify: test_campaign.py -k moved_aside_runs_again passed (run, rerun runs nothing, move aside, rerun redoes it as only-2; the log keeps both attempts). test_campaign.py 10 passed."
+++

## Problem

`vb campaign` treated a unit as finished, for good, once its log held a `finish` event with exit 0. A finished run
that an infrastructure failure spoiled could not be redone: moving its directory to `<experiment>.abandoned/` (the
documented way to discard a stopped unit's run) left the unit finished, so the campaign skipped it.

Pilot B (gap-327242, 2026-10-20) needed this. Unit `roko-fixed-s1-2` exited 0 with 20 records, but Cerebras refused
the account (HTTP 402) for its last 9 tasks (bug-c6318e).

## Why it matters

The only other ways to redo the unit were to edit the campaign log by hand, or to add a manifest block that re-runs
some instances, whose records would then collide with the spoiled ones (`metrics.check_unique`).

## Where

- `benchmarks/viabilitybench/driver/campaign.py::cmd_campaign`: the `finished` set.
- The module docstring's "Resuming" paragraph.

## Current state

Fixed on `work/gap-327242`. A `finish` event counts only while its run directory is still in the experiment
directory, so a finished unit whose run was moved to `<experiment>.abandoned/` runs again under the next run id. The
log keeps every attempt, and the ledger still counts the abandoned run's spend, since `.abandoned` lies inside the
results root.

## Plan

1. Done: the rule and a test (`test_a_finished_unit_whose_run_was_moved_aside_runs_again`).

## Done when

- [x] Moving a finished unit's run aside makes the next campaign redo it: the `[[verify]]` test.

## Notes

- Redoing a whole unit discards its good runs too. The abandoned directory keeps them, and the bundle's page says
  why it was redone, so this is not selective reporting.
