+++
id = "gap-cdd5f4"
kind = "gap"
title = "Companion report: re-derive every number at the audit/baseline-2026-09-28 tag"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["companion"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/companion-audit/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md && grep -q '91b4745f8' tmp/cybernetic-harness/companion-audit/E1-REDERIVATION.md"
+++

## Problem

The companion report's E1 step, re-deriving its numbers at a frozen tag, has not run. The tag exists:
`audit/baseline-2026-09-28` → `91b4745f8` (W9 finding 6). The numbers have drifted between documents: false greens
102/350 (29.1%) in the companion against 101/373 (27%) in CASE-001/B7, and three different counts of learning loops.

## Why it matters

The companion can ship first (D26). E1 is its critical-path step that needs no people; E2 and E3 need human raters.
Epic spec-f8d196.

## Where

- `tmp/cybernetic-harness/companion-audit/` (scripts in `telemetry/scripts/`; `common.py` honours `ROKO_REPO`).
- Extract the tag without touching git state:
  `git -C /Users/will/dev/nunchi/roko/roko archive audit/baseline-2026-09-28 | tar -x -C <scratch dir>`.

## Current state

Nothing has been re-derived.

## Plan

1. **Extract** the tag to a scratch directory.
2. **Re-run** the companion's measurement scripts with `ROKO_REPO` pointing at the extracted tree.
3. **Write** `companion-audit/E1-REDERIVATION.md`: every number the report uses, stamped `91b4745f8`, with a
   reconciliation of each drifted figure (which window and denominator).

## Done when

- [ ] E1-REDERIVATION.md lists every number with its source and the tag.
- [ ] The drift items are reconciled.
- [ ] The `[[verify]]` command passes.

## Notes

- Use derived counts only; never copy transcripts (D27).
- `git archive` is read-only. Never check out the tag in the main checkout.
