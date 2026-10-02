+++
id = "gap-ac1aed"
kind = "gap"
title = "PK101 Papers: Research paper Appendix E and the ViabilityBench README: the artefact as released (+5 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 101
size = "L"
hold = "waits on Will's deferred decision(s) 3333, 3346, 3363, 7101, 9524, 9539 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["paper"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK101"
anchors = ["benchmarks/viabilitybench/README.md", "tmp/cybernetic-harness/paper/00-README.md", "tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md", "tmp/cybernetic-harness/paper/sections/00-abstract.md", "tmp/cybernetic-harness/paper/sections/01-introduction.md", "tmp/cybernetic-harness/paper/sections/02-background.md", "tmp/cybernetic-harness/paper/sections/03a-related-work.md", "tmp/cybernetic-harness/paper/sections/03b-related-work.md", "tmp/cybernetic-harness/paper/sections/04-system.md", "tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md", "tmp/cybernetic-harness/paper/sections/06-results-economics.md", "tmp/cybernetic-harness/paper/sections/07-results-regulation.md", "tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/09-limitations.md", "tmp/cybernetic-harness/paper/sections/10-conclusion.md", "tmp/cybernetic-harness/paper/sections/A-benchmark.md", "tmp/cybernetic-harness/paper/sections/B-spec-standard.md", "tmp/cybernetic-harness/paper/sections/C-audit-protocol.md", "tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md", "tmp/cybernetic-harness/paper/sections/E-reproducibility.md", "tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = ["gap-6aaee9", "gap-11cec6", "gap-d4a1c1", "gap-9b5582"], blocks = [], related = ["dec-536bbd", "gap-85f86a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import re,sys;bad=[f for f in ('E-reproducibility.md',) if re.search(r'\\[\\[',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\" && grep -q '^## Reproducing the paper' benchmarks/viabilitybench/README.md"

[[verify]]
command = "python3 -c \"import re,sys,glob;tags=set();[tags.update(re.findall(r'(?:WIRED|PARTIAL|BROKEN|ORPHANED|MISSING|BUILT-UNWIRED|REMOVED|DOCS-ONLY|UNPROVEN)@([0-9a-f]{7,40})',open(f).read())) for f in glob.glob('tmp/cybernetic-harness/paper/sections/*.md')];sys.exit(0 if len(tags)==1 and not ({'ed0c33bd5','0cf9bacfe','407ce30d5'} & tags) else 1)\" && python3 -c \"import re,sys;bad=[f for f in ('A-benchmark.md','B-spec-standard.md','C-audit-protocol.md') if re.search(r'\\[\\[',open('tmp/cybernetic-harness/paper/sections/'+f).read())];sys.exit(1 if bad else 0)\""

[[verify]]
command = "python3 tools/paperlint.py --budget 1.2 tmp/cybernetic-harness/paper/sections/0*.md tmp/cybernetic-harness/paper/sections/10-conclusion.md && python3 tools/paperlint.py --report tmp/cybernetic-harness/paper/sections/0*.md tmp/cybernetic-harness/paper/sections/10-conclusion.md | python3 -c \"import re,sys;n=sum(int(m) for m in re.findall(r'(\\d+) words ·',sys.stdin.read()));sys.exit(0 if n<=18000 else 1)\""

[[verify]]
command = "grep -qE '^Verdict: (accept|accept with fixes)' tmp/cybernetic-harness/paper/REVIEW.md && grep -qE '^Numbers checked: [0-9]+ of [0-9]+, all match' tmp/cybernetic-harness/paper/REVIEW.md"

[[verify]]
command = "python3 tools/paperlint.py --strict --require-status final tmp/cybernetic-harness/paper/sections/ && python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[[verify]]
command = "grep -qE '^Submitted: arXiv:[0-9]{4}\\.[0-9]{4,5}' tmp/cybernetic-harness/paper/00-README.md"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK101, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9540 | M | p2 | Research paper Appendix E and the ViabilityBench README: the artefact as released | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9540-paper-reproducibility-appendix-and-bench-readme.md` |
| 2 | 9541 | M | p2 | Research paper: re-pin every remaining status tag and close the appendix AS-BUILT markers at the campaign tag | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9541-paper-repin-status-tags-at-campaign-tag.md` |
| 3 | 9542 | S | p2 | Research paper: final length pass to about 17,000 words of main text | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9542-paper-final-length-pass.md` |
| 4 | 9543 | M | p2 | Research paper internal review: a cold read, a number spot-check against the bundles and bibliography QA | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9543-paper-internal-review-and-number-check.md` |
| 5 | 9544 | S | p2 | Research paper final lock: every section final and strict lint clean | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9544-paper-final-lock-strict-lint.md` |
| 6 | 9545 | S | p2 | Submit the research paper to arXiv and record its id | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9545-paper-submit-to-arxiv.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/README.md`, `tmp/cybernetic-harness/paper/00-README.md`, `tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md`, `tmp/cybernetic-harness/paper/REVIEW.md`, `tmp/cybernetic-harness/paper/sections/00-abstract.md`, `tmp/cybernetic-harness/paper/sections/01-introduction.md`, `tmp/cybernetic-harness/paper/sections/02-background.md`, `tmp/cybernetic-harness/paper/sections/03a-related-work.md`, `tmp/cybernetic-harness/paper/sections/03b-related-work.md`, `tmp/cybernetic-harness/paper/sections/04-system.md`, `tmp/cybernetic-harness/paper/sections/05-evaluation-protocol.md`, `tmp/cybernetic-harness/paper/sections/06-results-economics.md`, `tmp/cybernetic-harness/paper/sections/07-results-regulation.md`, `tmp/cybernetic-harness/paper/sections/08-discussion.md`, `tmp/cybernetic-harness/paper/sections/09-limitations.md`, `tmp/cybernetic-harness/paper/sections/10-conclusion.md`, `tmp/cybernetic-harness/paper/sections/A-benchmark.md`, `tmp/cybernetic-harness/paper/sections/B-spec-standard.md`, `tmp/cybernetic-harness/paper/sections/C-audit-protocol.md`, `tmp/cybernetic-harness/paper/sections/D-metrics-statistics.md`, `tmp/cybernetic-harness/paper/sections/E-reproducibility.md`, `tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md`.

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

- Waits on: PK100 (gap-6aaee9), PK69 (gap-11cec6), PK95 (gap-d4a1c1), PK99 (gap-9b5582).
- On hold until Will takes the deferred decision(s) 3333, 3346, 3363, 7101, 9524, 9539 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: dec-536bbd, gap-85f86a. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
