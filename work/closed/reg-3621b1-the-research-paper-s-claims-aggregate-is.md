+++
id = "reg-3621b1"
kind = "regression"
title = "The research paper's claims aggregate is stale since the 2026-10-02 rewrite, so gap-c4d630's claims.py --check fails"
status = "superseded"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "47b3cdd20"
source = "status-quo-2026-10-15#finding-35"
discovered_from = "gap-6ee3c5: the verifies still failing after the superseded ones were retired; B's recommendation 4"
anchors = ["tmp/cybernetic-harness/paper/tools/claims.py", "tmp/cybernetic-harness/paper/CLAIMS-EVIDENCE.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["gap-c4d630", "gap-aed6f1", "gap-6ee3c5"], supersedes = [], duplicate_of = "gap-aed6f1" }

[[verify]]
command = "python3 tmp/cybernetic-harness/paper/tools/claims.py --check"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T19:32:00Z"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:20Z"
forced = false
evidence = "claims.py is superseded tooling: the paper moved to tools/claims_evidence.py on 2026-10-02 (that file's own generated header says 'tools/claims.py was not used. It emits an older format ... that this design paper doesn't use'; its mtime, 2026-10-02 19:32, predates this item). gap-aed6f1 (done, commit 7f3f346e5) rewrote 198 claim-ledger rows (+14/-16, 608 unique ids) and regenerated CLAIMS-EVIDENCE.md with claims_evidence.py (REVIEW.md, 'How the review was done'). Verified: regenerating via claims_evidence.py --out <tmp> now reproduces the live CLAIMS-EVIDENCE.md byte for byte except the non-deterministic 'Generated <time>' header line, so the aggregate is current, not stale. claims.py --check still fails (1500+ diff lines) only because it renders the pre-rewrite empirical-draft format (maturity@commit, blocks_on, hypothesis/spec maps); making it pass would revert CLAIMS-EVIDENCE.md to that format and regress gap-aed6f1's just-merged, reviewed output. Filed gap-9ea13c to give claims_evidence.py its own --check mode, the constructive version of what this item asked for."
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
