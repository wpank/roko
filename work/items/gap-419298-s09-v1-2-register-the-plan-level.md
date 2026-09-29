+++
id = "gap-419298"
kind = "gap"
title = "S09 v1.2: register the plan-level slice as an exploratory experiment"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:13 and 15:25, next paper wave)"
anchors = ["tmp/cybernetic-harness/specs/S09-experiments.md", "tmp/cybernetic-harness/specs/S08-benchmark-suite.md"]
lane = "paper"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-89f393", "gap-1cd676", "gap-ac4ce8", "gap-420202", "dec-39c781"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'v1.2' tmp/cybernetic-harness/specs/S09-experiments.md && grep -qi 'plan-level' tmp/cybernetic-harness/specs/S09-experiments.md"
+++

## Problem

On 2026-09-29 the author decided that the evaluation covers single tasks plus a small plan-level slice (dec-b78874):
fixture features that need whole multi-task plans (gap-89f393), run as Roko with the tier ladder against Claude Code
(gap-1cd676). The research paper's §5 already describes the slice as exploratory (gap-ac4ce8). S09, the experiments
spec, does not mention it. S09 v1.1 has no arms, n, metrics, analysis, budget line or stopping rule for it.

## Why it matters

The pre-registration lock (S09.E4, success criterion SC1) freezes S09's sha256 before any LOG1 spend. A slice that is
not in S09 at the lock is a post-hoc analysis, however it is labelled later. Its spend also needs a budget line inside
the caps. Epic spec-567e52.

## Where

- `tmp/cybernetic-harness/specs/S09-experiments.md`: the arms table, §4 (hypotheses and analysis), §4.6 (budget
  lines) and the change log at the top.
- `S08-benchmark-suite.md`, if the slice's fixtures need a family entry.
- The sources: gap-89f393, gap-1cd676 and §5 of the paper.

## Current state

Checked in MAIN on 2026-09-29: S09 is v1.1 and never says "plan-level".

## Plan

1. Add a plan-level slice section to S09: the fixtures, arms, n and seeds; metrics (makespan, cost per verified
   feature, verified-feature rate); a descriptive reporting rule, labelled exploratory and outside the Holm family; a
   budget line with its cap inside the $390 total; a stopping rule.
2. Match the paper's §6.6, which now has 13 `[[RESULT PL: …]]` slots (gap-420202).
3. Add a v1.2 line to S09's change log.
4. Check the paper's §5 against the new text (gap-ac4ce8 or its successor).

## Done when

- [ ] S09 v1.2 registers the slice before the pre-registration lock.
- [ ] The `[[verify]]` command passes.

## Notes

- The specs are untracked, so edit them in place in the main checkout.
- The exploratory label must match gap-1cd676's notes ("not pre-registered as confirmatory").
