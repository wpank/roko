+++
id = "gap-dc6775"
kind = "gap"
title = "Backfill executors for already-closed items from [closed].by and the reflog"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A7)"
anchors = ["tools/work_backfill.py", "tools/test_work_backfill.py", "work/telemetry/events/backfill.jsonl"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = ["gap-d0643c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_backfill_labels_every_row_and_maps_executors' tools/test_work_backfill.py && python3 tools/test_work_backfill.py -k test_backfill_labels_every_row_and_maps_executors && test -s work/telemetry/events/backfill.jsonl"
+++

## Problem

94 items are closed, and none records its executor in a form a script can read. `[closed].by` is set in 33 of them,
in about 15 free-text forms, for example `plan:portal-programme/03c-backend-local-access#T10`, `session roko-b6`,
`triage check 2026-09-28` and `Will (licence decision); files added by Claude`. The other 61 have no `by`. Branch and
worktree creation times, which date when lane work started, exist only in the reflog, and reflog entries expire.

## Why it matters

Goal `proof`, epic spec-f2463d. Without a labelled backfill the record starts at zero on the day the event log lands,
and the baseline for the switch to Roko-executed items (W12 B1) is thinner than it needs to be.

## Where

- **New:** `tools/work_backfill.py` (one-shot, standard library; loads items with `tools/work.py`'s `load`) and
  `tools/test_work_backfill.py`.
- **New output:** `work/telemetry/events/backfill.jsonl` in the `roko.work_event/1` schema (gap-d0643c), with
  `source = "backfill"` on every row.

## Current state

Checked at `41c7ffbd6`: 94 closed items, 33 with `by`, no backfill.

## Plan

1. Map `by` to an executor: `plan:…` → `roko-plan`; `session …` → `claude-session`; `Will …` → `human` (with
   `assist = "claude"` when the text names Claude); `commit trailer` → `unknown`.
2. Sweep, triage and enrichment closures (`work sweep …`, `triage check …`, `work enrichment …`) become
   `verification-only`: they checked an item; they did not implement it.
3. No `by`: executor `unknown`, reason `no-by`.
4. From `git reflog --date=iso` and each worktree's HEAD log, record creation times of lane and `work/*` branches as
   `lane-start` rows.
5. Write one `closed` row per item and one `lane-start` row per branch. Never edit item files.

## Done when

- [ ] Every row has `source = "backfill"`.
- [ ] The fixture maps each `by` form above as listed.
- [ ] `work/telemetry/events/backfill.jsonl` is written and committed.
- [ ] The `[[verify]]` command passes.

## Notes

- Run it soon: by default git expires reflog entries for unreachable commits after 30 days.
- The rollup (gap-ccb87e) reports backfilled rows separately and keeps them out of the pre-registered switch
  comparison.
