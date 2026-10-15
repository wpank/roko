+++
id = "gap-8d1463"
kind = "gap"
title = "The development-record rollup (work/telemetry/ROLLUP.md) has not been regenerated since 2026-10-01, and the field-capture watcher captured no run after 2026-09-29"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["tools/work_telemetry", "tools/field_capture"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-40"
anchors = ["tools/work_telemetry.py::cmd_rollup", "work/telemetry/ROLLUP.md", "tmp/cybernetic-harness/tools/field_capture.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_render_refreshes_the_rollup' tools/test_work.py && python3 tools/test_work.py -k test_render_refreshes_the_rollup"
+++

## Problem

Two parts of the development record stopped on their own (status-quo assessment 2026-10-15 at 7b6a5af44, finding 40;
`tmp/status-quo/2026-10-15/programme/E1-plan-vs-delivered.md` lines 36, 273-282 and 362-366):

- `work/telemetry/ROLLUP.md` and `rollup.json` say "report date 2026-10-01" (HEAD `ad14d38d7`). The rollup is meant to
  be daily (`tools/work_telemetry.py rollup`), but nothing runs it, so it covers none of the work since. New events
  still land in `work/telemetry/events/` (four untracked event files at b42578bab).
- The field-capture watcher (`tmp/cybernetic-harness/tools/field_capture.py`, pid 96293, started about 2026-09-29)
  snapshots finished plan runs in the repo and its worktrees. It captured nothing after 2026-09-29, because the plan
  runs since then ran in scratch workspaces outside its reach. On 2026-10-15 the process was no longer running.

## Why it matters

Goal `proof`: the programme's "record of the work" (how the backlog got done, and the papers' "in use" evidence)
depends on both. A record that silently stops is worse than none, because readers trust its dates.

## Where

- `tools/work_telemetry.py::cmd_rollup` (writes ROLLUP.md and rollup.json); `tools/work.py` (render).
- `tmp/cybernetic-harness/tools/field_capture.py` and `field_rollup.py` (untracked).

## Current state

Both tools work when run by hand; no hook, skill or schedule runs them.

## Plan

1. Run the rollup from the step that already runs after merges: have `work.py render` (or the `/work-batch` merge
   step) call `work_telemetry.py rollup`, so the record moves with the views.
2. Test `test_render_refreshes_the_rollup` in `tools/test_work.py`: a render with a new event updates the rollup's
   report date.
3. Field capture: either point it at the workspaces where runs happen (for example the workspaces registered with
   `roko serve`, or a list in its config) and run it under the daemon or launchd, or retire it and say so in the
   programme's evidence README. Recommended: retire it unless the papers' "in use" section needs new field evidence;
   the run records (`.roko/runs/<run>/`) already hold what it copied.

## Done when

The `[[verify]]` command passes, ROLLUP.md shows a current report date, and field capture is either running against
the workspaces in use or recorded as retired.

## Notes

ROLLUP.md says "Not for workers": no view or skill reads it, so refreshing it changes no agent's behaviour.
