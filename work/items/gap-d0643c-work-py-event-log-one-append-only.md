+++
id = "gap-d0643c"
kind = "gap"
title = "work.py event log: one append-only events file per session under work/telemetry/events/"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "13c6418bf"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A1, A2)"
anchors = ["tools/work.py::cmd_claim", "tools/work.py::cmd_release", "tools/work.py::main", "tools/test_work.py", "work/telemetry/README.md"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'roko.work_event/1' tools/work.py && grep -qw 'def test_claim_and_release_append_events_to_the_session_file' tools/test_work.py && python3 tools/test_work.py -k test_claim_and_release_append_events_to_the_session_file"

[closed]
at = 2026-10-01
commit = "13c6418bf"
by = "wk-gates"
evidence = "13c6418bf: claim and release append roko.work_event/1 rows to work/telemetry/events/<session>.jsonl in the main checkout (release before deleting the claim); new 'work.py event merged|post-verify|escape|intervention' with --merge-sha/--conflicts/--fixups/--rc/--caused-by; session from --session, $WORK_SESSION or the claimant; linked worktrees log nothing; no view reads the log; work/telemetry/README.md has the layout and schema. tools/test_work.py: claim+release give two valid rows in one session file, event merged carries the merge sha, a worktree logs nothing; the [[verify]] command passes."
+++

## Problem

`tools/work.py` keeps no history of how an item was worked. A claim is a file in `.roko/work-claims/`, and
`close_item`, `cmd_release` and `prune_claims` delete it, so the claim time, the claimant and any release are lost.
`[closed]` holds a date and a free-text `by`. Nothing records merges, post-merge verify results, escapes or
interventions.

## Why it matters

Goal `proof`, epic spec-f2463d. First-try merges, claim-to-merge time and coverage (W12 A6) are computed from these
events, and they feed the whitepaper's development-process figures. gap-0b9056 (executor fields) and gap-ccb87e (the
rollup) build on this log.

## Where

- `tools/work.py`: `cmd_claim`, `cmd_release`, and `main` (a new `event` subcommand).
- **New:** `work/telemetry/README.md`; event files `work/telemetry/events/<session>.jsonl` are created at run time.
- Tests: `tools/test_work.py` (the `RepoTest` fixture).

## Current state

Checked at `41c7ffbd6`: no `work/telemetry/` and no event code. The main checkout's `tools/work.py` has another
session's uncommitted edits in `compute_drift`, `touched_report` and `cmd_hook`; this item does not touch them.

## Plan

1. Schema `roko.work_event/1`, one JSON object per line: `ts`, `event`, `item`, `executor`, `via`, `session`,
   `branch`, `concurrency` (live claims at that moment) and `source` (`live`, `harvest`, `reconciled` or `backfill`),
   plus fields specific to the event.
2. `log_event()` appends to `<main checkout>/work/telemetry/events/<session>.jsonl`. `session` comes from `--session`,
   else `WORK_SESSION`, else a slug of `--by`. Only that session writes the file, so sessions never conflict.
3. `claim` and `release` log an event; `release` logs before it deletes the claim.
4. New `work.py event merged|post-verify|escape|intervention <id>` with `--merge-sha`, `--conflicts N`,
   `--fixups N`, `--rc N` and `--caused-by ID`.
5. Workers in worktrees log nothing (W12: workers never write events or see metrics). `render`, the views and the
   skills never read the log.
6. `work/telemetry/README.md`: the layout, the schema, and "not for workers".

## Done when

- [ ] `claim` then `release` in the fixture append two valid events to one session file.
- [ ] `event merged` appends a row carrying the merge sha.
- [ ] No view reads `work/telemetry/`.
- [ ] The `[[verify]]` command passes.

## Notes

- Tracker lane: one agent at a time on `tools/work.py`; E14's items (spec-1e1b45) edit it too.
- Standard library only.
