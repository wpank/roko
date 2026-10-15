+++
id = "gap-1c990a"
kind = "gap"
title = "Eleven answered backlog decisions fail their own verifies because nobody wrote the outcome rows they look for (5101, 7102, 7103, 9105-9110, 9301, 9305)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["programme/decisions"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-36"
anchors = ["tmp/cybernetic-harness/DECISIONS.md", "tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md", "tmp/backlog/2026-10-02-complete-and-wire/9301-confirm-the-showcase-design-defaults-d23.md", "tmp/backlog/2026-10-02-complete-and-wire/9305-owner-registry-and-identity-for-public-artefacts.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "F=tmp/cybernetic-harness/DECISIONS.md; B=tmp/backlog/2026-10-02-complete-and-wire; grep -q 'S03 §9.10' $F && grep -F '| D6 |' $F | grep -q Decided && grep -F '| D13 |' $F | grep -q Decided && grep -F '| D7 |' $F | grep -i audit | grep -q Decided && grep -qE '^[|] *DEC-RUNQ [|]' $F && grep -qE '^[|] *T06-2 [|]' $F && grep -qE '^[|] *T06-3 [|]' $F && grep -qE '^[|] *T06-4 [|]' $F && grep -qE '^[|] *T06-5 [|]' $F && grep -qE '^[|] *DEC10 [|]' $F && grep -q '^## Outcome' $B/9301-confirm-the-showcase-design-defaults-d23.md && grep -q '^## Outcome' $B/9305-owner-registry-and-identity-for-public-artefacts.md"
+++

## Problem

Eleven decision tasks in the complete-and-wire backlog were answered (most on 2026-10-02; see
`tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md` and `tmp/status-quo/2026-10-15/decisions/I2-decision-register.md`),
but their verifies look for a record nobody wrote, so each fails:

- 5101: `S03 §9.10` in `tmp/cybernetic-harness/DECISIONS.md`.
- 7102: rows `| D6 |` and `| D13 |` there marked `Decided`; 7103: row `| D7 |` with "audit" marked `Decided`. The
  rows exist but are not marked.
- 9105-9110: rows `| DEC-RUNQ |`, `| T06-2 |` … `| T06-5 |`, `| DEC10 |` there. None exists.
- 9301, 9305: a `## Outcome` heading in their own task files. Neither has one: their outcomes are recorded only in
  front matter (`decision`, `decided`, `outcome`), per A11.

On 2026-10-15 their task files were set to `status = "done"` (they said "open" at 7b6a5af44), so the failing verifies
now sit on done tasks. Status-quo assessment 2026-10-15, finding 36 (reports A06, A08, A10, A11).

## Why it matters

Goal `tooling`: a decision's answer should be readable where the programme's documents look for it, and failing
verifies on done tasks hide real regressions.

## Where

- `tmp/cybernetic-harness/DECISIONS.md` (the programme's decision table; untracked).
- `tmp/backlog/2026-10-02-complete-and-wire/9301-*.md`, `9305-*.md`.

## Current state

The answers exist in the backlog's DECISIONS.md and in the I2 register; only the rows and headings are missing.

## Plan

1. For each of the eleven, copy the recorded answer (option, date, who) into the place its verify reads: mark the
   D6, D7 and D13 rows `Decided` with the answer, add the S03 §9.10, DEC-RUNQ, T06-2 to T06-5 and DEC10 rows, and add
   a `## Outcome` section to the 9301 and 9305 task files that restates their front-matter outcome.
2. Make no new decision: if an answer is missing or ambiguous, leave that row out and name it in the item.

## Done when

The `[[verify]]` command (the eleven verifies joined) passes.

## Notes

These files are untracked, so there is no commit to cite; record the evidence as the verify's output. If finding
35's `retire-verify` lands first, retiring these verifies is not the fix: the decisions should be recorded.
