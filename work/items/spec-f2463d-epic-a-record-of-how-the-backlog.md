+++
id = "spec-f2463d"
kind = "spec"
title = "Epic: a record of how the backlog gets done"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "L"
subsystem = ["tools/work", "work/telemetry"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md"
anchors = ["tools/work.py::cmd_claim", "tools/work.py::close_item", "tmp/cybernetic-harness/tools/field_rollup.py::load_summaries", "work/telemetry/"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "tracker"
links = { depends_on = ["bug-7b37c4", "gap-d0643c", "gap-0b9056", "gap-92033c", "gap-263de5", "gap-7984a5", "gap-ccb87e", "gap-dc6775", "gap-09e478", "bug-652bb7", "bug-469537", "gap-568056", "bug-4c4eea", "bug-f7f3bb"], blocks = [], related = ["spec-1e1b45"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f work/telemetry/DEFINITIONS.md && grep -qw 'def test_rollup_reports_cost_per_merged_item_with_coverage' tools/test_work_telemetry.py && python3 tools/test_work_telemetry.py -k test_rollup_reports_cost_per_merged_item_with_coverage"
+++

## Problem

Claude agents and sessions do most of the backlog now, and Roko will take on part of it once the P0/P1 fixes land.
Nothing records how that work gets done: who executed each item, how it was picked up, how long it took, what it cost
in tokens, how often a merge conflicted or a post-merge verify failed, and how often a person stepped in. At 11:00 on
09-29 (W12) history held no `Closes:` trailer and no `work/*` branch, claim files were deleted on close, and only about
a third of closed items had a `[closed].by`, in free text. Meanwhile the field rollup, which covers Roko's own plan
runs, overstates the autonomy index.

## Why it matters

- Goal `proof`. The whitepaper's evidence section (W12: figures FF1–FF2, tables FT1–FT3) and the cost comparison
  need this record: "cheaper at equal quality" has to count the operator's tokens too.
- The switch from Claude-executed to Roko-executed items can only be read as a before/after if the "before" is
  recorded now, with the metrics defined in advance.

## Where

- `tools/work.py`: claims, close, sync and a new `event` subcommand. `.claude/skills/work-{batch,next}` (local-only).
- New tools: `tools/work_harvest.py`, `tools/work_telemetry.py`, `tools/work_backfill.py`.
- New data: `work/telemetry/` (`events/`, `harvest/`, `manifests/`, `DEFINITIONS.md`, `ROLLUP.md`).
- `tmp/cybernetic-harness/tools/field_rollup.py` and `field_note.py` (untracked).

## Current state

Checked at `41c7ffbd6`:
- No child has started. `work/telemetry/` does not exist.
- `.claude/settings.json` now wires `work.py hook` as a PostToolUse hook; W12 found none.
- gap-09e478 (a validated evidence bundle from a Graph plan run) is still open: `scripts/run_evidence.py` does not
  read `.roko/state/graph/`.

## Plan

This is the implementation plan. It covers Phase A (Claude-executed work); Roko-executed items join the same record
after the P0/P1 fixes (W12 Phase B).

1. **Start in parallel; the files don't overlap:**
   - fix the field rollup (bug-7b37c4);
   - write `DEFINITIONS.md` (gap-7984a5) before any data is analysed;
   - build the transcript harvester (gap-263de5).
2. **Tracker changes, one at a time:** the event log (gap-d0643c), then executor fields on claim, release and close
   (gap-0b9056), then the skills (gap-92033c). The first two edit `tools/work.py`, so they interleave with E14's items.
3. **Backfill** the closed items (gap-dc6775) once the event schema exists. Run it soon, before the reflog expires.
4. **The daily rollup** (gap-ccb87e) joins events, harvest and `[closed]`; its committed manifest freezes each day.
5. **Phase B hook:** the Graph evidence bundle (gap-09e478) gives Roko-executed items the same record. PLAN.md orders
   it after step 4.

## Done when

- [x] bug-7b37c4: The field rollup counts tasks with operator interventions as automatic recoveries
- [ ] gap-d0643c: work.py event log: one append-only events file per session under work/telemetry/events/
- [ ] gap-0b9056: work.py claim and close record the executor, pick-up route, size and claim time; release records a reason
- [ ] gap-92033c: The work skills add Work-Item, Executor and Conflicts trailers to merge commits
- [x] gap-263de5: Transcript harvester: tokens, model and time per backlog item from Claude Code transcripts
- [x] gap-7984a5: DEFINITIONS.md: the development record's metrics, fixed in advance
- [ ] gap-ccb87e: Daily rollup of the development record with a committed manifest
- [ ] gap-dc6775: Backfill executors for already-closed items from [closed].by and the reflog
- [x] gap-09e478: Dogfood session evidence bundle (existing item)
- [x] bug-652bb7: Field capture can snapshot a resumed run twice, and the rollup's by-run table is not in date order
- [x] bug-469537: Field snapshots record absolute home-directory paths, so they cannot be published as they are
- [x] gap-568056: Graph runs write no .roko/state/status.json, so roko status and evidence status sampling see no live run
- [x] bug-4c4eea: The --log-file event log copies agent output verbatim, so evidence bundles hold raw agent text
- [ ] bug-f7f3bb: Under roko serve, status.json keeps showing a finished run as active, because the serve PID is still alive
- [ ] The epic's `[[verify]]` command (the rollup test, with `DEFINITIONS.md` present) passes on the merged branch.

## Notes

- **Lane `tracker`: one agent at a time on `tools/work.py`.** E14 (spec-1e1b45) edits the same file, and PLAN.md §4
  runs E14.1–E14.3 first. bug-7b37c4, gap-7984a5 and gap-263de5 don't touch `work.py` and can run alongside.
- **Untracked files:** bug-7b37c4 works in `tmp/cybernetic-harness/tools/` and gap-92033c in `.claude/skills/`. Both
  exist only in the main checkout.
- **Honesty rules (W12):** passive sources first; workers never write events or see metrics; no metric becomes a
  target; results are labelled "observational", "API-equivalent (subscription)" or "backfill".
- **gap-09e478** keeps its goal (`tooling`) and severity. Nothing in it depends on gap-ccb87e.
- **Needs Will:**
  - whether harvest rows (session ids, branches, token counts; no content) may be committed under D27 (gap-263de5);
  - the switch item ("Roko executes work items", W12 B1 and W7 §8) is not in the manifest: should it be filed, and
    which P0/P1 items gate it;
  - whether his own interventions are logged as `user`.
- **Decided 2026-09-29 (Will):** harvested counts (model, token counts, branch, session id; no transcript content) are committed under `work/telemetry/` (gap-263de5).
