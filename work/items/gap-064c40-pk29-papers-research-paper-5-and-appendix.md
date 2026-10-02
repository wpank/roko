+++
id = "gap-064c40"
kind = "gap"
title = "PK29 Papers: Research paper §5 and Appendix D: reconcile with S09 v1.4 and the confirmed defaults,…"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 29
size = "S"
subsystem = ["paper"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK29"
anchors = ["tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md", "tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md"]
lane = "paper"
parent = "spec-fef7c5"
links = { depends_on = [], blocks = [], related = ["dec-39c781"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! (python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md 2>&1 | grep -qE '\\[(marker|banned|header|citation|identifier|status-tag|link)\\]') && grep -q 'S09 v1.4' tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md && grep -q 'S09 v1.4' tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md"
+++

## Problem

This package delivers 1 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK29, slice 95xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9509 | S | p2 | Research paper §5 and Appendix D: reconcile with S09 v1.4 and the confirmed defaults, ready for the lock | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9509-paper-protocol-and-metrics-match-s09-v1-4.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md`, `tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md`.

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

- Waits on: nothing.
- Existing work items this package covers or touches: dec-39c781. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.

## Progress

- 9509: implemented in place on 2026-10-02 in the main checkout's untracked `tmp/cybernetic-harness/paper/sections/`
  (`05-evaluation-protocol.md`, `D-metrics-statistics.md`), so no commit holds the text; strict paperlint leaves only
  number findings (8 + 19), the `[[verify]]` command passes, and `claims.py --check` passes after regenerating
  `CLAIMS-EVIDENCE.md`. Both headers name S09 v1.4 and its sha256 (`8c630d72…be4cea4b`). Picked up v1.4's rule of one
  `vb run` at a time per secret file (§5.6, §5.8, D.5); `[[TAB T2]]` became the Table 2a/2b captions; the ledger was
  checked against `budget.toml` and `driver/ledger.py`, and D.1–D.12 against `analysis/metrics.py`, `passk.py` and
  `report.py`, all at `976220c3e` (D's new "As built" paragraph; ledger rows C5.34, C5.35, CD.27); "first" is gone;
  D9, D10 and D13 read as confirmed on 2026-10-02 (8102, 5101, 7102); D28–D36 keep their "(default)" labels because
  dec-39c781 is still open. Open for others: the toolkit's plan-level false greens (gate passed, VF = 0) are broader
  than S09 §4.9's "whole-feature suite failed"; BL0's $14 cap and `roko_ladder`/BL14 (3301, 3302) are noted in §5.7
  and D.3 but wait for S09 v1.5, which §5 and D must pick up before the lock.
