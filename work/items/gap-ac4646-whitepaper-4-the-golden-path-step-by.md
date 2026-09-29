+++
id = "gap-ac4646"
kind = "gap"
title = "Whitepaper §4 The golden path step by step, with Figure 2"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md (the loop, steps 1-11)"
anchors = ["docs/whitepaper/04-golden-path.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-35a614", "gap-af0b57"], blocks = [], related = ["gap-d1d92c", "gap-370d3c", "spec-98f76d", "spec-a0e40a", "spec-a78d57", "spec-f09094"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/04-golden-path.md && grep -q 'Figure 2' docs/whitepaper/04-golden-path.md && test $(grep -cE '(WIRED|PARTIAL|ORPHANED|BUILT-UNWIRED|DOCS-ONLY|MISSING|REMOVED|BROKEN|UNPROVEN)@[0-9a-f]{7}' docs/whitepaper/04-golden-path.md) -ge 11 && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/04-golden-path.md"
+++

## Problem

The golden path is the thesis in motion, in 11 steps: author, compile the spec, size and split, route, schedule,
isolate, verify, recover, integrate, review, learn. The whitepaper needs each step as designed, with today's status
beside it.

## Why it matters

This is the core of the pitch and of the `golden-path` goal (epics E5–E11). For each step, a reader must be able to
see what works now and which epic closes the gap.

## Where

`docs/whitepaper/04-golden-path.md` (new; gap-0191eb creates it as a stub).

## Current state

Checked at `41c7ffbd6`. tldr/04, written at `d9e79e9d8`, tags the steps:
- **MISSING:** steps 3, 4 and 10;
- **ORPHANED and MISSING:** step 9;
- **PARTIAL:** the rest.

Since then:
- step 5 moved with the ready queue (`445a60d0d`, `3e7552acd`; gap-4d835d is still open);
- step 8 moved with the adaptive retry budgets (`99adacd6d`, `41c7ffbd6`);
- epic E2's honest-verdict fixes (spec-e9d7ec) will change step 7.

## Plan

1. **One short subsection per step,** each with:
   - the design, in the ideal-state voice;
   - today's status, with the tag from the status matrix (gap-35a614) at the matrix's commit;
   - the principle from §2 behind the step;
   - the epic that closes the gap: E5 tier ladder, E6 integration, E7 scheduler, E8 specs, E9 diff check,
     E10 watchdog, E11 acceptance tests.
2. **A summary table** of the 11 steps with their tags.
3. **A Figure 2 draft:** the loop, with each step marked by its tag. Write a caption plus a text diagram in a fenced
   block; gap-d1d92c draws the SVG.
4. **Say plainly where the promise is untested:** no real task has yet run on a cheap executor with escalation.

## Done when

- [ ] Each of the 11 steps carries a tag at a commit that matches the matrix.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **Recompute tldr/04's figures** (for example, the share of sequential plans) from tracked data such as `plans/`, or
  from a frozen snapshot, and give each a footnote.
- Lane `paper`; no hot files.
- **Written on `work/gap-ac4646` at `4bc903cea`** (2026-09-29). Tags are at the status matrix's `a17d4dadd` (the
  gap-35a614 draft), re-checked with greps at `1f4481133`; no golden-path code moved between the two. The verify's
  static part passes (29 tag lines), and `paperlint --strict` from the gap-af0b57 draft is clean (1,188 words, 1.25×).
  Close once gap-af0b57 merges `tools/paperlint.py`; if the matrix is re-pinned, re-check the tags and move `@a17d4dadd`.
