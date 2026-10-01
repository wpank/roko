+++
id = "gap-85f86a"
kind = "gap"
title = "Research paper: re-run the prior-art listing sweep (Q13–Q15, Q17) in submission week and read the Li 2026 lead"
status = "open"
triage = "unverified"
severity = "p3"
goal = "whitepaper"
size = "S"
hold = "runs in submission week"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:40, wk-prior-art's report on gap-bb619d)"
anchors = ["tmp/cybernetic-harness/paper/PRIOR-ART-D13.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-bb619d", "gap-b4e597", "gap-65ed57"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qi '^## .*submission-week re-run' tmp/cybernetic-harness/paper/PRIOR-ART-D13.md"
+++

## Problem

The D13 prior-art kill-search (gap-bb619d, `paper/PRIOR-ART-D13.md`) ran on 2026-09-29. It recommends that every novelty sentence keep the form "we know of no … (search of 2026-09-29)", and that Q13–Q15 (the arXiv listing sweep over cs.SE, cs.MA, cs.AI, cs.LG and cs.CL, about 3 tool calls) and Q17 be re-run in submission week for anything posted after 2026-09-29. It also left a lead unread: Li, J. 2026, "From Answers to Audit Opinions: Cost-Aware Expert Verification of Financial Artifacts from Tool-Using LLM Agents" (https://doi.org/10.1145/3834580.3838756), a possible K1 precedent whose abstract was not reachable.

## Why it matters

Research paper (epic spec-f8d196): the novelty claims are dated, and a precedent posted between now and submission would falsify them.

## Where

`tmp/cybernetic-harness/paper/PRIOR-ART-D13.md`: the query table (Q13–Q15 at about :67-69, Q17 at :71), "Unread leads" (about :290) and the recommendation (about :304).

## Current state

The sweep and the lead are recorded as planned. gap-b4e597 applies the D13 edits to the paper sections.

## Plan

1. In submission week, re-run Q13–Q15 over the listings posted since 2026-09-29, and Q17, with the same filters.
2. Read the Li 2026 paper (at least its abstract) and classify it against K1.
3. Add a dated "Submission-week re-run" section to `PRIOR-ART-D13.md` with the queries, counts and any new matches. Route any edit it calls for to the section's owner, as gap-b4e597 did.
4. Update the "(search of …)" dates in the novelty sentences.

## Done when

- [ ] The re-run is recorded, the Li lead is classified, and the novelty sentences carry the new date.
- [ ] The `[[verify]]` command passes.

## Notes

- Held until submission week (the `hold` line). Remove the hold when that week starts.
