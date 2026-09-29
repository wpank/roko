+++
id = "gap-353d57"
kind = "gap"
title = "Whitepaper §1 Introduction"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/01-WHAT-AND-WHY.md (the idea; where it stands)"
anchors = ["docs/whitepaper/01-introduction.md", "docs/whitepaper/00-abstract.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-29a64e", "gap-424bf8", "gap-8d2c79"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/00-abstract.md && test -f docs/whitepaper/01-introduction.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/00-abstract.md docs/whitepaper/01-introduction.md"
+++

## Problem

The whitepaper has no introduction and no abstract. Together they must state the thesis and, just as plainly, what is
built and what is only designed.

## Why it matters

§1 and the abstract are the parts people quote. W9 recommends that they say "is designed to" for anything not yet
WIRED. The cheap-model half of the thesis is untested: all 210 portal attempts ran on pinned Sonnet 4.6 (tldr/04).

## Where

`docs/whitepaper/01-introduction.md` and `docs/whitepaper/00-abstract.md` (new; gap-0191eb creates them as stubs).

## Current state

Checked at `41c7ffbd6`. No text exists yet. The sources:
- tldr/00 (the ten things to know);
- tldr/01 (the idea, where it stands, when to use Roko or Claude Code, the positioning);
- the research draft's `paper/sections/01-introduction.md` §1.1–1.3 (stable prose, 3,239 words), whose framing can be
  condensed: "capable or cheap", and "regulate, and audit the regulation";
- PLAN §1, for the thesis.

## Plan

1. **The problem:** frontier models are capable but expensive, and cheap models are unreliable on their own.
2. **The thesis and the bet** (tldr/01): on decomposable, checkable work, most of the dependability comes from the
   harness. Don't claim this for sequential or integrative work (C1).
3. **Built versus designed,** with status tags:
   - WIRED: the Graph engine, checkpoints and honest per-task verdicts;
   - not WIRED: the tier ladder, escalation, integration and audits.
4. **What Roko has done:** the portal build, with its numbers taken from the frozen rollup. Name the supervising
   session and what it did (tldr/01).
5. **Contributions,** and a map of §2–§10.
6. **The abstract** (150 words), written last. The review (gap-8d2c79) refreshes its numbers.

## Done when

- [ ] No effect is claimed without data, and every number has a footnote naming its source.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close: write the section at any time, and close it once the lint
  exists.
- **Keep the numbers in step with §7** (gap-29a64e). B7 gives $174.87 and 173 tasks for the portal; the rollup gives
  $192.92 over 42 runs, non-portal runs included.
- Lane `paper`; no hot files.
