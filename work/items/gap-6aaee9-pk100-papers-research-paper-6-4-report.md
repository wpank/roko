+++
id = "gap-6aaee9"
kind = "gap"
title = "PK100 Papers: Research paper §6.4: report H3, the spec × model interaction (+9 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 100
size = "L"
hold = "waits on Will's deferred decision(s) 3333, 3346, 3363, 7101, 9524 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["paper"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK100"
anchors = ["tmp/cybernetic-harness/paper/sections/00-abstract.md", "tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md", "tmp/cybernetic-harness/paper/sections/04-system.md", "tmp/cybernetic-harness/paper/sections/06-results-economics.md", "tmp/cybernetic-harness/paper/sections/07-results-regulation.md", "tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/09-limitations.md", "tmp/cybernetic-harness/paper/sections/10-conclusion.md", "tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = ["gap-5ebb4f", "gap-2ca903", "gap-7c9a9c", "gap-e9218e", "gap-85d176", "gap-50346f", "gap-7ec3ef", "gap-63fd4c", "gap-fd96c8", "gap-4a5109", "gap-f7bab8", "gap-ed1a08", "gap-11cec6", "gap-d4a1c1", "gap-a6dab7", "gap-b90650", "gap-9b5582"], blocks = [], related = ["dec-536bbd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/06-results-economics.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT H3|FIG F5)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/06-results-economics.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT H4|FIG F9)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/06-results-economics.md').read();sys.exit(1 if re.search(r'\\[\\[',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check && python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections/06-results-economics.md"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/07-results-regulation.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT H7|FIG F10|TAB T4)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/07-results-regulation.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT H5|FIG F8|TAB T10)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/07-results-regulation.md').read();sys.exit(1 if re.search(r'\\[\\[(RESULT H6|FIG F7|TAB T8)',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;t=open('tmp/cybernetic-harness/paper/sections/07-results-regulation.md').read();sys.exit(1 if re.search(r'\\[\\[',t) else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check && python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections/07-results-regulation.md"

[[verify]]
command = "python3 -c \"import re,sys;bad=[f for f in ('00-abstract.md','01-introduction.md','10-conclusion.md') if re.search(r'\\[\\[',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;bad=[f for f in ('08-discussion.md','09-limitations.md','F-ai-assistance-ethics.md') if re.search(r'\\[\\[',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\" && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "python3 -c \"import re,sys;bad=[f for f in ('04-system.md','03b-related-work.md') if re.search(r'\\[\\[',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\" && python3 tools/paperlint.py --check-identifiers tmp/cybernetic-harness/paper/sections/04-system.md tmp/cybernetic-harness/paper/sections/03b-related-work.md"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK100, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9529 | S | p2 | Research paper §6.4: report H3, the spec × model interaction | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9529-paper-results-h3-specs.md` |
| 2 | 9530 | S | p2 | Research paper §6.5: report H4, calibrated routing with escalation | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9530-paper-results-h4-routing.md` |
| 3 | 9531 | M | p2 | Research paper §6.1–§6.3 and §6.7: report H1 and H2, the campaign's integrity and the P1 synthesis | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9531-paper-results-h1-h2-economics.md` |
| 4 | 9532 | S | p2 | Research paper §7.3: report H7, loop-liveness auditing | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9532-paper-results-h7-loop-liveness.md` |
| 5 | 9533 | S | p2 | Research paper §7.1: report H5, whether audits bound the true false-green rate | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9533-paper-results-h5-audits.md` |
| 6 | 9534 | S | p2 | Research paper §7.2: report H6, regulation after injected disturbances | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9534-paper-results-h6-regulation.md` |
| 7 | 9535 | S | p2 | Research paper §7.4 and §7.5: report the closure tests X1–X4 and the P2 synthesis | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9535-paper-results-closure-tests-and-p2-synthesis.md` |
| 8 | 9536 | S | p2 | Research paper abstract, §1.5 and §10: state the headline results | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9536-paper-abstract-intro-conclusion-results.md` |
| 9 | 9537 | M | p2 | Research paper §8, §9 and Appendix F: discussion, limitations and the author's statements | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9537-paper-discussion-limitations-appendix-f-final.md` |
| 10 | 9538 | M | p2 | Research paper §4 and §3b: the system as built at the campaign tag | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9538-paper-system-section-as-built-at-tag.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `tmp/cybernetic-harness/paper/sections/00-abstract.md`, `tmp/cybernetic-harness/paper/sections/01-introduction.md`, `tmp/cybernetic-harness/paper/sections/03b-related-work.md`, `tmp/cybernetic-harness/paper/sections/04-system.md`, `tmp/cybernetic-harness/paper/sections/06-results-economics.md`, `tmp/cybernetic-harness/paper/sections/07-results-regulation.md`, `tmp/cybernetic-harness/paper/sections/08-discussion.md`, `tmp/cybernetic-harness/paper/sections/09-limitations.md`, `tmp/cybernetic-harness/paper/sections/10-conclusion.md`, `tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md`.

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

- Waits on: PK20 (gap-5ebb4f), PK30 (gap-2ca903), PK31 (gap-7c9a9c), PK37 (gap-e9218e), PK44 (gap-85d176), PK46 (gap-50346f), PK49 (gap-7ec3ef), PK50 (gap-63fd4c), PK56 (gap-fd96c8), PK57 (gap-4a5109), PK62 (gap-f7bab8), PK67 (gap-ed1a08), PK69 (gap-11cec6), PK95 (gap-d4a1c1), PK96 (gap-a6dab7), PK98 (gap-b90650), PK99 (gap-9b5582).
- On hold until Will takes the deferred decision(s) 3333, 3346, 3363, 7101, 9524 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: dec-536bbd. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
