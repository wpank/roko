+++
id = "gap-29a64e"
kind = "gap"
title = "Whitepaper §7 Field evidence"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/evidence/field/CASES.md (CASE-001 to CASE-008)"
anchors = ["docs/whitepaper/07-field-evidence.md", "docs/whitepaper/evidence"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "bug-7b37c4", "gap-af0b57"], blocks = [], related = ["gap-263de5", "gap-ccb87e", "gap-7984a5", "gap-09e478", "dec-2cd76a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/07-field-evidence.md && grep -qi 'observational' docs/whitepaper/07-field-evidence.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/07-field-evidence.md"
+++

## Problem

Roko has already built something real: most of the portal, across 16 plans. The whitepaper needs that evidence, told
honestly:
- what Roko did;
- what the supervising frontier sessions did;
- what it all cost;
- where the gates missed product defects.

## Why it matters

It is the only real-world evidence before the benchmark (E12). It also shows the operator loop the thesis wants to
automate. The supervising sessions wrote and audited the plans, fixed about 25 engine defects and merged by hand
(tldr/01), at an estimated 16–20× Roko's recorded spend (W12).

## Where

- **The section:** `docs/whitepaper/07-field-evidence.md` (new; gap-0191eb creates it as a stub).
- **Frozen inputs:** `docs/whitepaper/evidence/` (new) holds the rollup and the notes the text quotes, as derived
  data only, if dec-2cd76a keeps its default.

## Current state

Checked at `41c7ffbd6`, and in gitignored `tmp/cybernetic-harness/evidence/field/`.

- **The rollup** at 11:14 today covered 42 snapshots and 121 notes. It shows:
  - 158 verified tasks;
  - $192.92 as recorded;
  - an autonomy index of 3/41, which is overstated: `field_rollup.py` counts tasks with operator notes as automatic
    recoveries (bug-7b37c4).
- **Other sources use other scopes:**
  - B7 gives $174.87 and 173 tasks for the portal alone;
  - W12 estimates the supervising sessions at about $2.7–3.4k API-equivalent for 09-25 to 09-29, against Roko's
    recorded $172.80.

## Plan

1. **The portal build:** plans, tasks, tests, attempts, first-try passes, spend as recorded, and concurrency. Take them
   all from one frozen rollup, cited by its generation time and hash.
2. **Three or four cases:**
   - CASE-001: a false green, then honest verdicts;
   - CASE-005: merges made by hand;
   - CASE-006: gates green, product unusable;
   - CASE-007: plan defects that only frontier audits caught.
3. **The operator loop and its cost:** use E13's harvested numbers if they have landed (gap-263de5, gap-ccb87e).
   Otherwise use W12's figure, labelled as an estimate.
4. **Label everything observational:** no causal claims, and no comparison with the benchmark.

## Done when

- [ ] Every number traces to the frozen rollup, a snapshot or a commit, and its scope is stated.
- [ ] The `[[verify]]` command passes.

## Notes

- **Waits for bug-7b37c4** (E13.1).
- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **Nothing copied out of `tmp/` may contain keys or transcripts** (field README, rule 2; bug-7d7200).
- Lane `paper`; no hot files.
