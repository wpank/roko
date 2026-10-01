+++
id = "bug-7b37c4"
kind = "bug"
title = "The field rollup counts tasks with operator interventions as automatic recoveries"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/field-tools"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "1f4481133"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (F3, A8)"
anchors = ["tmp/cybernetic-harness/tools/field_rollup.py::load_summaries", "tmp/cybernetic-harness/tools/field_rollup.py::main", "tmp/cybernetic-harness/tools/field_note.py::cmd_add", "tmp/cybernetic-harness/tools/test_field_rollup.py"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_operator_noted_task_is_not_an_auto_recovery' tmp/cybernetic-harness/tools/test_field_rollup.py && grep -qw 'def test_help_writes_nothing' tmp/cybernetic-harness/tools/test_field_rollup.py && python3 tmp/cybernetic-harness/tools/test_field_rollup.py -k test_operator_noted_task_is_not_an_auto_recovery -k test_help_writes_nothing"

[[verify]]
command = "grep -q 'add_argument(\"--item\"' tmp/cybernetic-harness/tools/field_note.py"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "field_rollup.py (tmp/cybernetic-harness/tools, untracked) no longer counts a retried pass as an auto recovery when an intervention note on that task joins the same run; notes join one run at most; days sorted; --help writes nothing; field_note.py gains --item and --executor. test_field_rollup.py: 7 tests pass, all 7 fail on the old code. ROLLUP.md regenerated 14:37: autonomy index 2/41 (was 3/41). sha256: field_rollup.py 7643e80963c6, field_note.py 8a3572eb84d4, test_field_rollup.py e61a8d622d54. Watcher restarted to load it."
+++

## Problem

`tmp/cybernetic-harness/tools/field_rollup.py` turns field snapshots and notes into the metrics the whitepaper cites
(`evidence/field/ROLLUP.md`, `rollup.json`). It has four defects:

1. **Operator fixes count as auto recoveries.** `load_summaries` (lines 49–51) counts every task that passed after more
   than one attempt. The docstring (lines 12–13) excludes tasks a person fixed, but the code never reads the notes.
   08f-T05 counts although notes FN-20260929-005 and -006 record two operator interventions on it, so the autonomy
   index is overstated.
2. **Notes join runs by plan** (`by_plan_notes`, lines 77–84), so every run of a plan gets all its notes.
3. **Days are unsorted:** days from notes without a plan are appended last (lines 112–117).
4. **No argument parsing:** `main` ignores `argv`, so `--help` rewrote `ROLLUP.md` (assessment W1, "Incident").

## Why it matters

Goal `proof`, epic spec-f2463d. The whitepaper cites these numbers, and S01 addendum F6 takes this rollup as the
reference for Roko's own field report.

## Where

- `field_rollup.py`: `load_summaries` (recoveries) and `main` (note join, days, output).
- `field_note.py::cmd_add`: notes carry `plan`, `task` and `run`; summaries carry `plan_id`, `run_id` and `window`.
- **New:** `tmp/cybernetic-harness/tools/test_field_rollup.py` (`unittest`; point `ROKO_FIELD_DIR` at a temp dir).

## Current state

Checked at `41c7ffbd6`: all four defects are present. Of 122 notes, 46 name a plan, 18 a task and 2 a run. The
directory is untracked (`/tmp/*` in `.gitignore`).

## Plan

1. Drop from `_recovered_tasks` every (plan, task) with an intervention note fixed by `operator` or `user`.
2. Attach each note to one run: by `run` when set, else to the latest run of its plan that started before the note.
3. Sort the days; skip empty dates.
4. Parse arguments (`--field-dir`, `--dry-run`); `--help` writes nothing.
5. W12 A8's other half: `field_note.py add --item <work id> --executor <who>`.

## Done when

- [ ] A retried pass with an operator note is not an auto recovery (08f-T05 fixture).
- [ ] Each note counts in at most one run, and days are sorted.
- [ ] `--help` writes nothing; `field_note.py add --item` stores the id.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Work in the main checkout; worktrees lack these files. Close with `--evidence` naming the files' sha256.
- Regenerate `ROLLUP.md` once and record how the totals changed.
