+++
id = "gap-9b5582"
kind = "gap"
title = "PK99 Papers: Research paper §5 and Appendix D: cite the pre-registration lock for every number and… (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 99
size = "S"
hold = "papers rewritten 2026-10-02 (session roko-55) as design papers; the old empirical draft is archived in tmp/cybernetic-harness/paper/archive/2026-10-02-empirical-draft/. This item's tasks target the old text: Will decides whether to re-scope or supersede it"
subsystem = ["paper"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK99"
anchors = ["tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md", "tmp/cybernetic-harness/paper/sections/06-results-economics.md", "tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = ["gap-5ddf9b", "gap-064c40", "gap-2ca903", "gap-a6dab7"], blocks = [], related = ["dec-536bbd", "gap-1cd676"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "head -1 tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md | grep -q '^Status: final' && head -1 tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md | grep -q '^Status: final' && python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md && grep -q 'prereg.lock.json' tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/06-results-economics.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT PL|TAB T11)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK99, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9527 | S | p2 | Research paper §5 and Appendix D: cite the pre-registration lock for every number and mark them final | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9527-paper-protocol-cite-lock-and-freeze.md` |
| 2 | 9528 | S | p2 | Research paper §6.6: report the plan-level slice from its bundle | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9528-paper-results-plan-level-slice.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md`, `tmp/cybernetic-harness/paper/sections/06-results-economics.md`, `tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md`.

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

- Waits on: PK27 (gap-5ddf9b), PK29 (gap-064c40), PK30 (gap-2ca903), PK96 (gap-a6dab7).
- Existing work items this package covers or touches: dec-536bbd, gap-1cd676. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
