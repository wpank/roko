+++
id = "gap-e8cb4d"
kind = "gap"
title = "Whitepaper §5 Cybernetic mechanisms"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/03-MECHANISMS.md (learning and memory; regulation and audits)"
anchors = ["docs/whitepaper/05-cybernetic-mechanisms.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-35a614", "gap-af0b57"], blocks = [], related = ["spec-6ac537", "spec-b7303f", "gap-424bf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/05-cybernetic-mechanisms.md && grep -qi 'guarded commit' docs/whitepaper/05-cybernetic-mechanisms.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/05-cybernetic-mechanisms.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Whitepaper §5 cybernetic mechanisms merged in ddad1f2bf; tags match the matrix; paperlint --strict passes."
+++

## Problem

The whitepaper must explain how Roko is meant to improve. That means the learning loops, the four regulation mechanisms
and guarded commit with rollback. The four mechanisms are:
- M1: a bounded controller;
- M2: a loop-liveness audit;
- M3: a calibrated self-model;
- M4: random deep audits.

Most of this is designed rather than built, so each part needs its status.

## Why it matters

"Improves measurably" is half the thesis, and it is the claim most likely to be overstated. No learning loop has shown
a measured benefit in either era (tldr/00, point 6), yet docs/v3 called learning an "exponential flywheel" (tldr/05
§5). This section states the honest version: measured, guarded improvement.

## Where

`docs/whitepaper/05-cybernetic-mechanisms.md` (new; gap-0191eb creates it as a stub).

## Current state

Checked at `41c7ffbd6`.

- **M1–M4 and guarded commit are MISSING.** They are specified in `tmp/cybernetic-harness/specs/`:
  - S02 re-closes the loops;
  - S03 is the loop-liveness audit;
  - S04 is the self-model;
  - S05 is the audits;
  - S06 is the controller.
- **The loops are still changing.**
  - Today's `ce3bdcbb8` persists retry feedback per plan and lets adaptive thresholds set retry budgets.
  - The portal session's `feat/learning-completion-loops` branch is still open.
- **The research draft's §4.3–4.9** describe the mechanisms as designed, but they name eight identifiers that do not
  exist (W9 F3).

## Plan

1. **The essential variables:** verified pass rate, cost per verified task, false-green rate and latency.
2. **The learning loops:** routing, retry feedback, failure memory, playbooks, prompt experiments and adaptive
   thresholds. For each, say what it senses and what it changes, with its tag from the matrix.
3. **M1–M4 and guarded commit, one paragraph each:**
   - what it regulates, its sensor, its actuator and its bounds (the research draft's Table T1);
   - its tag;
   - its spec and epic E17 (spec-6ac537).
4. **The analogy mechanisms,** in one sentence, as proposals to park (tldr/05 §3), not as features: affect, offline
   consolidation, HDC similarity and the conductor.

## Done when

- [ ] Every mechanism carries a tag at a commit that matches the matrix, and every designed identifier is tagged as
      designed.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **How the specs are cited** depends on dec-2cd76a, part 5.
- Lane `paper`; no hot files.
- **2026-09-29, wk-wp-s5:** written on `work/gap-e8cb4d` at `d231cb588`, with the four new bibliography keys in
  `352c79b88` and claim CM3 reworded in `8ffa71cd4` (the count of 16 loops has no tracked source). Every tag is taken
  from the status matrix pinned at `a17d4dadd`, as its rows stand on `work/gap-35a614` at `e035b5efe`. The static half
  of the verify passes. The paperlint half can't run here, because `tools/paperlint.py` hasn't merged; the in-progress
  copy from `work/gap-af0b57` passes `--strict`, `--budget 1.3` and `--check-identifiers`. The committed paperlint
  `0cca5d0cb`, which counts words by the README's rule, also passes `--strict`: 935 words, 1.10×. Close once gap-af0b57
  and gap-35a614 have merged and the verify passes on the merged branch.
