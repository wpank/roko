+++
id = "reg-3621b1"
kind = "regression"
title = "The research paper's claims aggregate is stale since the 2026-10-02 rewrite, so gap-c4d630's claims.py --check fails"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "status-quo-2026-10-15#finding-35"
discovered_from = "gap-6ee3c5: the verifies still failing after the superseded ones were retired; B's recommendation 4"
anchors = ["tmp/cybernetic-harness/paper/tools/claims.py", "tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["gap-c4d630", "gap-aed6f1", "gap-6ee3c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tmp/cybernetic-harness/paper/tools/claims.py --check"
+++

## Problem

`python3 tmp/cybernetic-harness/paper/tools/claims.py --check`, gap-c4d630's verify, exits 1 in the main checkout:
`CLAIMS-EVIDENCE.md` was not regenerated after the research paper was rewritten as a design paper on 2026-10-02, so it
no longer matches the claim ledgers in the new sections (more than 1,500 diff lines at 7b6a5af44, per
`tmp/status-quo/2026-10-15/backlog/B-top-level-and-regressions.md`, recommendation 4).

## Why it matters

Goal `whitepaper`: the aggregate is where a reader checks each claim against its evidence. gap-c4d630 is done and its
tool works; only its output is stale. So this is a regression, not a check that the rewrite made obsolete, and
gap-6ee3c5 left gap-c4d630's verify live.

## Where

`tmp/cybernetic-harness/paper/tools/claims.py` aggregates the sections' claim ledgers into
`tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md` (its `--out` default). Both are untracked: work in the main checkout,
or in a worktree with a copy of `tmp/cybernetic-harness/`.

## Current state

The new sections carry claim ledgers; the aggregate still reflects the archived empirical draft.

## Plan

1. Keep a copy of the current `CLAIMS-EVIDENCE.md` (untracked files have no history: gap-d7b6a4).
2. Run `python3 tmp/cybernetic-harness/paper/tools/claims.py` and read the change: a claim that lost its evidence is a
   finding for the paper's review (gap-aed6f1).
3. Re-run with `--check`.

## Done when

The `[[verify]]` command passes.

## Notes

Same paper as gap-aed6f1 (the research paper's independent review and final lint pass): coordinate with whoever holds it.
