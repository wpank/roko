+++
id = "gap-2abf34"
kind = "gap"
title = "Research paper appendices E and F: reproducibility formats and the cost of the supervising sessions"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/paper/sections/E-reproducibility.md", "tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qi 'supervising' tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md && test -f tools/paperlint.py && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Research paper App. F names the supervising sessions' estimated cost; App. E fills the formats that exist with AS-BUILT tags on the rest; verify passes."
+++

## Problem

- **App. F:** claim CF.18 ("actual spend per line") has no source, and App. F doesn't mention the supervising
  sessions' cost.
- **App. E:** has 13 TODOs about schemas.

## Why it matters

Honest accounting of the AI assistance behind the research paper. The whitepaper has no such disclosure (decided
2026-09-29), but the research paper keeps its App. F. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/paper/sections/E-reproducibility.md`, `tmp/cybernetic-harness/paper/sections/F-ai-assistance-ethics.md`.

## Current state

App. F is stable apart from CF.18. App. E is a skeleton.

## Plan

1. **App. F:** add the operator-loop cost (W12 estimate, labelled as such), and point CF.18 at the future harvest
   (E13.5).
2. **App. E:** fill in the schema formats that exist today (field snapshots, the S01 record types as designed), and keep
   `[[AS-BUILT]]` on anything not built.

## Done when

- [ ] App. F names the supervising sessions and their estimated cost.
- [ ] The `[[verify]]` command passes.

## Notes

- These files are untracked (`tmp/` is gitignored), so edit them in place in the main checkout. Only this item's files
  may be edited: other agents own the other section files at the same time.
- Keep the marker grammar in `tmp/cybernetic-harness/paper/00-README.md`. Never delete a `[[RESULT …]]` slot or turn it into a claim; results
  arrive later from S09 bundles.
- Use the thesis decided on 2026-09-29, "golden path plus cybernetics": frontier models plan, cheap models execute in
  parallel, gates and a whole-plan check verify, and feedback loops make it improve measurably. Status claims carry
  a commit.
