+++
id = "gap-b90650"
kind = "gap"
title = "PK98 Papers: Post the companion report to arXiv and record its id (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 98
size = "S"
hold = "waits on Will's deferred decision(s) 9524 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["companion"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK98"
anchors = ["tmp/cybernetic-harness/companion-audit/00-README.md", "tmp/cybernetic-harness/paper/sections/00-abstract.md", "tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/03a-related-work.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = ["gap-d4a1c1", "gap-d2507f"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^Posted: arXiv:[0-9]{4}\\.[0-9]{4,5}' tmp/cybernetic-harness/companion-audit/00-README.md && grep -q 'arXiv' tmp/cybernetic-harness/paper/bibliography/new-refs-companion.jsonl"

[[verify]]
command = "python3 -c \"import re,sys;bad=[f for f in ('00-abstract.md','01-introduction.md','03a-related-work.md') if re.search(r'\\[\\[CITE-COMPANION|TODO:\\s*companion-report\\s+citation',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK98, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9525 | S | p2 | Post the companion report to arXiv and record its id | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9525-companion-post-to-arxiv.md` |
| 2 | 9526 | S | p2 | Research paper: cite the posted companion in the abstract, §1 and §3a, with its final headline numbers | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9526-paper-cite-the-posted-companion.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `tmp/cybernetic-harness/companion-audit/00-README.md`, `tmp/cybernetic-harness/paper/bibliography/new-refs-companion.jsonl`, `tmp/cybernetic-harness/paper/sections/00-abstract.md`, `tmp/cybernetic-harness/paper/sections/01-introduction.md`, `tmp/cybernetic-harness/paper/sections/03a-related-work.md`.

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

- Waits on: PK95 (gap-d4a1c1), PK97 (gap-d2507f).
- On hold until Will takes the deferred decision(s) 9524 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
