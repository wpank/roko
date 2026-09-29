+++
id = "gap-25065c"
kind = "gap"
title = "Import the rest of the research programme's checklist as unverified work items"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e14"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W1-work-graph-audit.md (R5); W3a-crosswalk-core.md; W3b-crosswalk-product.md"
anchors = ["tools/work_import_checklist.py", "tools/test_work_import_checklist.py"]
lane = "tracker"
parent = "spec-1e1b45"
links = { depends_on = ["gap-130a3e", "gap-2bc1b9"], blocks = [], related = ["spec-6ac537"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq '^source = \"tmp/cybernetic-harness/execution/checklist.json#S10.T1\"' work/items/ && grep -qw 'def test_import_skips_items_already_covered' tools/test_work_import_checklist.py && python3 tools/test_work_import_checklist.py -k test_import_skips_items_already_covered"
+++

## Problem

The research programme's checklist (`tmp/cybernetic-harness/execution/checklist.json`, 262 rows) holds the build
plans of specs S01–S11. PLAN.md turned only part of it into work items (epics E1–E17). The rest lives in an untracked
JSON file that `next`, the views and `sync` cannot see, so the two trackers drift apart (W3a, "Two trackers"). S02–S07,
S10 and S11 have 153 rows; 122 are open (120 todo, 2 partial).

## Why it matters

Goal `tooling`, epic spec-1e1b45. The author's rule is "everything goes in `work/`" (PLAN.md §1), with the checklist
read-only afterwards. E17's M1–M4 work (spec-6ac537) and the S10/S11 demo and deploy work need a home.

## Where

- **New:** `tools/work_import_checklist.py` (one-shot; writes items through `work.py new` with gap-2bc1b9's flags)
  and `tools/test_work_import_checklist.py`.
- Dedupe sources: `tmp/cybernetic-harness/workstreams/manifest.json` and the W3a and W3b crosswalks in
  `workstreams/assessment/`.

## Current state

Checked at `41c7ffbd6`: no item cites a checklist row as its `source`. W3a maps some rows to open items, for example
S02.P1-1 to bug-8da8ba and bug-f68404.

## Plan

1. Take the open rows of S02–S07, S10 and S11. Skip a row that a manifest item or an open item already covers
   (W3a/W3b TRACKED rows, the manifest's hints), and list each skip with the covering id.
2. Map the fields:
   - `source = "tmp/cybernetic-harness/execution/checklist.json#<id>"`, the title, `files_owned` → anchors;
   - `effort` → `size`, noting the checklist's scale (S is up to half a day), and `milestone`;
   - lanes: L3 → `rust-hot`, L2 → `rust-cold`, L1 and L6 → `bench`, L4 → `frontend`, L5 → per `lanes.toml`;
   - `deps` → `depends_on` where the dependency was imported or is covered;
   - a runnable, guarded `verify` → `[[verify]]`; anything else, and `acceptance`, → `## Done when`.
3. Goals and parents: S02–S06 → `cybernetic`, parent spec-6ac537; S07 → `golden-path`, parent spec-e57870;
   S10 → `visibility`; S11 → `release` with `hold = "after the golden path"`.
4. Every item gets `triage = "unverified"`. `--dry-run` prints the plan; the real run writes once.

## Done when

- [ ] The dry run lists every in-scope row as imported or skipped, with the covering id.
- [ ] `check` reports 0 problems after the import.
- [ ] The `[[verify]]` command passes.

## Notes

- Will reviews the dry-run list before the real run. Do not edit `checklist.json`.
- **From wk-filer (2026-09-29):** the import must also cover open rows no epic took: S01.P0-3, -4, -5, -7, -8, -9 and -10; S08.T8–T10 and T14–T17; S09.E3–E11 (including the E4 pre-registration lock and E11, the full comparison whitepaper §8 cites); companion rows E2–E4, E6, E11 and E13.
