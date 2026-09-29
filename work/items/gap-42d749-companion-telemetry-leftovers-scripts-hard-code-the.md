+++
id = "gap-42d749"
kind = "gap"
title = "Companion telemetry leftovers: scripts hard-code the repo path, and the E3 rating sheets still cite the freeze file"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["companion-audit"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-companion-fin's report on gap-b409fa)"
anchors = ["tmp/cybernetic-harness/companion-audit/telemetry/scripts/", "tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E3.csv", "tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E3-key.csv", "tmp/cybernetic-harness/execution/evidence/E2/build_rating_worksheets.py"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-b409fa", "gap-cdd5f4", "gap-cb86e4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq --include='*.py' '/Users/will' tmp/cybernetic-harness/companion-audit/telemetry/scripts/ && ! grep -q 'claude-md-status-2026-09-28.md' tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E3.csv tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E3-key.csv"
+++

## Problem

Two leftovers from the companion report's re-derivation (wk-companion-fin, gap-b409fa):

1. Ten scripts in `companion-audit/telemetry/scripts/` hard-code `/Users/will/dev/nunchi/roko/roko`: `drift2.py`, `drift2_at_tag.py`, `drift3.py`, `drift3_at_tag.py`, `ledger_blame_survival.py`, `ledger_classify_commits.py`, `ledger_plan_crossref.py`, `ledger_plan_crossref_at_tag.py`, `ledger_runs.py`, `ledger_scope_check.py`. They run only on one machine and one checkout. `common.py` already honours `ROKO_REPO` (gap-cdd5f4).
2. The E3 rating sheets still cite the freeze file `work/history/claude-md-status-2026-09-28.md` (the "FS" anchors) in 31 rows: 28 in `HUMAN-RATING-E3.csv` and 3 in `HUMAN-RATING-E3-key.csv`. `companion-audit/E1-REDERIVATION.md` says to re-point each one to its line in `CLAUDE.md@91b4745f8` (its line map is `fs_to_claude_map.txt` in the E1 scratch outputs) and, for the 13 with no equivalent at the tag, to the underlying evidence. `execution/evidence/E2/build_rating_worksheets.py` can rebuild the sheets.

## Why it matters

Research paper, companion and TL;DR (epic spec-f8d196): someone else must be able to re-derive the companion at the tag, and its rating sheets must cite sources that exist there.

## Where

The anchors, all under the untracked `tmp/cybernetic-harness/`.

## Current state

Counted in the main checkout on 2026-09-29.

## Plan

1. Replace the hard-coded path in each script with `common.py`'s repo resolution (`ROKO_REPO`, else the git top level).
2. Rebuild the E3 sheets with `build_rating_worksheets.py`, re-pointing the FS anchors as E1-REDERIVATION.md describes.
3. Re-run one drift script at the tag and check its output is unchanged.

## Done when

- [ ] No telemetry script hard-codes a local path, and no E3 sheet cites the freeze file.
- [ ] The `[[verify]]` command passes.
