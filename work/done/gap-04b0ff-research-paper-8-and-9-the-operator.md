+++
id = "gap-04b0ff"
kind = "gap"
title = "Research paper §8 and §9: the operator loop, field-evidence threats, and trims"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/sections/08-discussion.md", "tmp/cybernetic-harness/paper/sections/09-limitations.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/08-discussion.md tmp/cybernetic-harness/paper/sections/09-limitations.md"

[[verify]]
command = "grep -qi 'operator' tmp/cybernetic-harness/paper/sections/08-discussion.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper §8 (operator loop, observational, autonomy index 2/41) and §9 (field-evidence and scope threats), within budget; field claims retyped observational; both verifies pass."
+++

## Problem

- **§8** has no account of the operator loop: from 09-25 to 09-29 the supervising sessions cost about $2.7–3.4k
  API-equivalent, about 16–20× Roko's recorded $172.80 (assessment W12).
- **§9** lacks the threats from observational field data and from the golden-path scope, and it is 2.8× its budget.

## Why it matters

Selling point 4 ("cheaper") must count the operator (field README rule 5). This is W9's PW09 and PW07. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/08-discussion.md`, `tmp/cybernetic-harness/paper/sections/09-limitations.md`. Sources: `tmp/cybernetic-harness/evidence/field/`,
`workstreams/assessment/W12-*.md`.

## Current state

Paragraph plans only.

## Plan

1. **§8:** add the operator loop and the autonomy index as description, labelled observational.
2. **§9:** add the field-evidence threat and the scope threat, then trim to at most 1.2× budget.

## Done when

- [ ] §8 describes the operator loop, and §9 names both threats.
- [ ] Both files are within 1.2× budget, and both `[[verify]]` commands pass.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
