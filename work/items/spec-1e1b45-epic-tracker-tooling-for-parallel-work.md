+++
id = "spec-1e1b45"
kind = "spec"
title = "Epic: tracker tooling for parallel work"
status = "open"
triage = "unverified"
severity = "p1"
goal = "tooling"
size = "L"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (R1–R7); W7-orchestration-model.md; W13-active-sessions-coordination.md"
anchors = ["tools/work.py::validate", "tools/work.py::pick_next", "tools/work.py::cmd_claim", "work/lanes.toml"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "tracker"
links = { depends_on = ["gap-130a3e", "gap-d1f787", "gap-823dce", "gap-c9e61b", "gap-2bc1b9", "gap-25065c", "dec-b75b96", "gap-d6fd3c"], blocks = [], related = ["spec-f2463d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f work/lanes.toml && grep -qw 'def test_next_mix_respects_lane_caps' tools/test_work.py && grep -qw 'def test_next_treats_files_changed_in_other_worktrees_as_busy' tools/test_work.py && grep -qw 'def test_claim_refuses_an_overlapping_footprint' tools/test_work.py && grep -qw 'def test_epics_view_counts_children' tools/test_work.py && grep -qw 'def test_new_writes_every_field_from_flags' tools/test_work.py && python3 tools/test_work.py"
+++

## Problem

`tools/work.py` copes with about four workers on one machine (W1): claims are atomic, `next` keeps anchored files
apart and `close` checks the verify command. The plan now runs 6–10 agents at once, plus other sessions, on one
tracker (PLAN.md §1). At that scale the tool fails in four ways (W1, W7, W13):

- **Collisions.** `next` sees only claimed items' anchors, not the files other worktrees are changing (on 09-29, 63
  changed files and 6 of them covered), and `claim` never re-checks the footprint.
- **Expiry.** Claims last a fixed 24 hours with no renewal, and `next` and `claims` prune claim files as a side
  effect.
- **No lanes.** The `lane`, `parent` and `milestone` fields the epics use are ignored, so a batch cannot be balanced
  across lanes, and nothing shows an epic's progress.
- **Friction.** `new` takes few fields, so agents hand-edit TOML; 122 open checklist rows sit outside `work/`; and
  sessions started inside worktrees have no work skills.

## Why it matters

Goal `tooling`, ranked first within it. Every other epic's items are worked through this tool, so it limits how much
of the plan can run in parallel. PLAN.md §4 starts the tracker lane with E14.1–E14.3.

## Where

- `tools/work.py`: `validate`, `pick_next`/`cmd_next`, `cmd_claim`/`load_claims`/`prune_claims`, `cmd_new`,
  `render_root` and `main`. Tests: `tools/test_work.py`.
- New files: `work/lanes.toml`, `work/EPICS.md`, `tools/work_import_checklist.py`.
- Local-only: `.claude/skills/` and `.claude/settings.json`.

## Current state

Checked at `41c7ffbd6`:
- `tools/work.py` (1,383 lines) and `tools/test_work.py` (13 tests) are tracked since `f99e45dba`.
- The main checkout has another session's uncommitted edits to `compute_drift`, `touched_report` and `cmd_hook`,
  which none of these items touch.
- No lanes file, and no `list`, `show` or `status`.

## Plan

This is the implementation plan. One agent at a time works on `tools/work.py`.

1. **Safe picks first:** lanes and fields (gap-130a3e), then the worktree scan in `next` (gap-d1f787), then the
   locked, renewable claim (gap-823dce). Together they are enough to start 6–10 agents (W1 R7).
2. **Views and input:** `list`, `show`, `status` and `EPICS.md` (gap-c9e61b), and `new` with every field (gap-2bc1b9).
   Both wait for gap-130a3e.
3. **The import** of the remaining checklist rows (gap-25065c), after gap-2bc1b9.
4. **The skills decision** (dec-b75b96) can be taken at any time; E13's skill edits (gap-92033c) follow it.
5. E13's `work.py` items (gap-d0643c, gap-0b9056) slot in after step 1.

## Done when

- [ ] gap-130a3e: work.py: validate the lane, parent and milestone fields, and add next --lane and --mix
- [ ] gap-d1f787: work.py next treats files changed in any other worktree as busy
- [ ] gap-823dce: work.py claim re-checks the footprint under a lock, can be renewed, and expires by size
- [ ] gap-c9e61b: work.py list, show and status, plus a generated EPICS.md with progress per epic and lane
- [ ] gap-2bc1b9: work.py new accepts every item field as a flag
- [ ] gap-25065c: Import the rest of the research programme's checklist as unverified work items
- [x] dec-b75b96: Decide how the work skills reach sessions started inside worktrees
- [ ] gap-d6fd3c: Install the work skills at user level so sessions in worktrees get them
- [ ] The epic's `[[verify]]` command (the whole `tools/test_work.py` suite, with the new tests present) passes on
      the merged branch.

## Notes

- `tools/work.py` is a single file, so this lane runs serially. W1 R7 suggests splitting it into modules if it grows
  much further.
- Out of scope (W1, W7): a `review` status with `approve`, `reserve` path locks, a `guard` edit hook, `stats`, per-lane
  session owners and a pre-commit lane hook. File them if wanted.
- Every change keeps today's item files and `.roko/work-claims/` readable.
- **Needs Will:** the first `work/lanes.toml` (lane caps), and dec-b75b96.
- **Decided 2026-09-29 (Will):** the work skills are installed at user level (dec-b75b96), done by gap-d6fd3c.
