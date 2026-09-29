+++
id = "gap-35a614"
kind = "gap"
title = "Status matrix: every Roko mechanism with its status tag at a commit"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper", "tools/status_matrix"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/03-MECHANISMS.md (68 mechanisms, tagged at d9e79e9d8)"
anchors = ["docs/whitepaper/appendix-status-matrix.md", "docs/whitepaper/data/mechanisms.toml", "tools/status_matrix.py", "tools/test_status_matrix.py"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-af0b57"], blocks = [], related = ["gap-ac4646", "gap-e8cb4d", "gap-c19902", "gap-d1d92c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/status_matrix.py && python3 tools/status_matrix.py --check"

[[verify]]
command = "test -f docs/whitepaper/appendix-status-matrix.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/appendix-status-matrix.md"
+++

## Problem

The whitepaper describes Roko as designed. Readers need one place that says, for every mechanism, what actually runs,
at which commit, and where the code is. tldr/03 has such a table (68 mechanisms in 9 groups), but it sits in
gitignored `tmp/`, is tagged at `d9e79e9d8`, and several of its rows have changed since.

## Why it matters

§4, §5 and §9 take their tags from this matrix (gap-ac4646, gap-e8cb4d, gap-c19902), and Figure 3 is drawn from it
(gap-d1d92c). Generating it from data, with code checks, makes it reproducible and keeps it from drifting unnoticed.

## Where

All new:
- `docs/whitepaper/data/mechanisms.toml`: one row per mechanism, giving its group, name, tag and verdict; its code
  anchors (`path::symbol`); its evidence (commits, tests or work items); and, if it isn't WIRED, the item or spec that
  would change its tag.
- `tools/status_matrix.py` (standard library only) and `tools/test_status_matrix.py`.
- `docs/whitepaper/appendix-status-matrix.md`: generated. It replaces gap-0191eb's stub, or creates the file if that
  item hasn't landed.

## Current state

Checked at `41c7ffbd6`. These rows have changed since `d9e79e9d8`:
- parallel waves: ready-queue start, and a failure skips only its dependants (`445a60d0d`, `3e7552acd`);
- adaptive thresholds, which now set retry budgets (`99adacd6d`, `41c7ffbd6`);
- the cost of killed attempts: usage streamed before a timeout is now kept (`d4be4e872`).

Open branches will change more: `fix/hermetic-child-env`, `fix/diagnose-graph-runs` and
`feat/learning-completion-loops`.

## Plan

1. **Seed the TOML from tldr/03.** Use the research draft's plain mechanism names, not "dreams", "daimon" or "immune".
2. **Re-check every tag at one pinned commit,** from the code rather than from the tldr. Cite only tracked evidence,
   never the gitignored research notes.
3. **Render the appendix with `status_matrix.py`:** a status header taken from the data file (so the review can set
   it), the pinned commit, and one table per group, with tags written `TAG@sha`.
4. **Check every anchor at the pinned commit** with `git cat-file -e` and `git grep -w`. A row without an anchor must
   be tagged MISSING, DOCS-ONLY or REMOVED, and must name an item or spec.
5. **Two modes:** `--check` regenerates the appendix and fails on any difference or bad row; `--probe-head` lists rows
   whose anchors no longer exist at HEAD.
6. **Tests** run on a temporary git repository.

## Done when

- [ ] Every tldr/03 mechanism has a row with a tag, a commit, an anchor (or an item) and evidence.
- [ ] A test shows that `--check` fails when an anchor is removed.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **The matrix is a dated snapshot;** `work/` stays the live status (work/README rule 7). Refresh the matrix at the
  review (gap-8d2c79).
- Lane `paper`; no hot files.
