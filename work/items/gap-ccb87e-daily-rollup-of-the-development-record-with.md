+++
id = "gap-ccb87e"
kind = "gap"
title = "Daily rollup of the development record with a committed manifest"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A5)"
anchors = ["tools/work_telemetry.py", "tools/test_work_telemetry.py", "work/telemetry/ROLLUP.md", "work/telemetry/manifests/"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = ["gap-d0643c", "gap-263de5"], blocks = [], related = ["gap-7984a5", "gap-92033c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_rollup_reports_cost_per_merged_item_with_coverage' tools/test_work_telemetry.py && python3 tools/test_work_telemetry.py -k test_rollup_reports_cost_per_merged_item_with_coverage"
+++

## Problem

Once the event log (gap-d0643c) and the harvest (gap-263de5) exist, nothing turns them into figures, and nothing
freezes them: an event file or a harvest row could change after the fact without anyone noticing.

## Why it matters

Goal `proof`. This is the exit check of epic spec-f2463d. The rollup produces the whitepaper's development-process
figures (W12: FF1–FF2, FT1–FT3), and its committed daily manifest is what the paper cites: git is the freeze.

## Where

- **New:** `tools/work_telemetry.py` (standard library) and `tools/test_work_telemetry.py`.
- **New outputs:** `work/telemetry/ROLLUP.md`, `work/telemetry/rollup.json` and `work/telemetry/manifests/<date>.json`.
- Inputs: `work/telemetry/events/*.jsonl`, `work/telemetry/harvest/*.jsonl`, `[closed]` blocks (through
  `tools/work.py`'s `load`), merge trailers from `git log --merges` (gap-92033c) and `DEFINITIONS.md` (gap-7984a5).

## Current state

Checked at `41c7ffbd6`: none of these files exist.

## Plan

1. Join per item: claim and release events, `[closed]`, the merge trailers, the `merged` and `post-verify` events,
   and the harvested tokens for branch `work/<id>`.
2. Compute each metric exactly as `DEFINITIONS.md` defines it, and print that file's sha256 at the top of `ROLLUP.md`.
3. Tables by day, executor, kind × size and lane. Each states coverage; backfilled rows are labelled and kept apart.
4. Cost: tokens × the `prices-2026-09-28` rates (gap-0580f7 writes the snapshot), labelled "API-equivalent
   (subscription)", with orchestration overhead in its own row.
5. `manifest` subcommand: sha256 and row count per input file, written to `manifests/<date>.json`.
6. `ROLLUP.md` opens with "not for workers"; `render`, NOW.md and the skills never read it.

## Done when

- [ ] A fixture (two items, one claimed twice with a conflicted merge, harvest rows for both) gives the right
      first-try merges, attempts, conflict rate and cost per merged item, and states coverage.
- [ ] The manifest changes when any input row changes.
- [ ] The `[[verify]]` command passes.

## Notes

- Keep it out of `tools/work.py`, so it can be built while E14's items edit that file.
- If the price snapshot does not exist yet, use a small rate table in the script, named with the snapshot id.
- The skills commit the daily manifest (gap-92033c, step 5).
