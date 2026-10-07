+++
id = "gap-d2507f"
kind = "gap"
title = "PK97 Papers: Companion: fill the E2 and E3 results from the human ratings, and redraw Figure 3 (+2 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 97
size = "M"
subsystem = ["companion"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK97"
anchors = ["tmp/cybernetic-harness/companion-audit/00-README.md", "tmp/cybernetic-harness/companion-audit/06-REVIEW.md", "tmp/cybernetic-harness/companion-audit/E10-APPENDICES.md", "tmp/cybernetic-harness/companion-audit/E10-DRAFT.md", "tmp/cybernetic-harness/companion-audit/data/adjudication/INTEGRITY-STATS.json"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = ["gap-a6dab7"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -v '^>' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -qE '\\[\\[E(2|3):' && test -f tmp/cybernetic-harness/companion-audit/data/adjudication/E2-RATER-STATS.json && test -f tmp/cybernetic-harness/companion-audit/data/adjudication/E3-RATER-STATS.json"

[[verify]]
command = "! grep -v '^>' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -q '\\[\\[E6:' && grep -qE '^Classifier agreement: ' tmp/cybernetic-harness/companion-audit/research/E6-RESULTS.md"

[[verify]]
command = "python3 tools/paperlint.py --strict --require-status final tmp/cybernetic-harness/companion-audit/E10-DRAFT.md tmp/cybernetic-harness/companion-audit/E10-APPENDICES.md && grep -q '^## Final read' tmp/cybernetic-harness/companion-audit/06-REVIEW.md"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK97, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9521 | S | p2 | Companion: fill the E2 and E3 results from the human ratings, and redraw Figure 3 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9521-companion-fill-e2-e3-results.md` |
| 2 | 9522 | S | p2 | Companion: fill the E6 provenance results | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9522-companion-fill-e6-provenance-results.md` |
| 3 | 9523 | M | p2 | Companion report: final pass — the author's statements in, markers out, status final, strict lint, last read | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9523-companion-final-pass.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `tmp/cybernetic-harness/companion-audit/00-README.md`, `tmp/cybernetic-harness/companion-audit/06-REVIEW.md`, `tmp/cybernetic-harness/companion-audit/E10-APPENDICES.md`, `tmp/cybernetic-harness/companion-audit/E10-DRAFT.md`, `tmp/cybernetic-harness/companion-audit/data/adjudication/E2-RATER-STATS.json`, `tmp/cybernetic-harness/companion-audit/data/adjudication/E3-RATER-STATS.json`, `tmp/cybernetic-harness/companion-audit/data/adjudication/INTEGRITY-STATS.json`, `tmp/cybernetic-harness/companion-audit/figures/fig1-provenance.svg`, `tmp/cybernetic-harness/companion-audit/figures/fig3-citation-errors.svg`, `tmp/cybernetic-harness/companion-audit/research/E6-RESULTS.md`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK96 (gap-a6dab7).
- Suggested model: opus.
