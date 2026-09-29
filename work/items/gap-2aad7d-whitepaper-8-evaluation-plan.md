+++
id = "gap-2aad7d"
kind = "gap"
title = "Whitepaper §8 Evaluation plan"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md (How to prove it)"
anchors = ["docs/whitepaper/08-evaluation-plan.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["spec-567e52", "dec-b78874", "gap-327242", "gap-d9e9fe"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/08-evaluation-plan.md && grep -qi 'cost per verified task' docs/whitepaper/08-evaluation-plan.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/08-evaluation-plan.md"
+++

## Problem

The thesis is unproven. No real task has run on cheap executors with escalation, and there has been no head-to-head
comparison with Claude Code. The whitepaper must say how the thesis will be tested, what counts as success, and what
would falsify it.

## Why it matters

The first thing the author wants proven is "cheaper at equal quality": cost per verified task against Claude Code on
Opus (PLAN §1). Stating the plan in advance makes the result credible when it lands.

## Where

`docs/whitepaper/08-evaluation-plan.md` (new; gap-0191eb creates it as a stub).

## Current state

Checked at `41c7ffbd6` and in the programme documents. Three different sets of arms exist:
- **tldr/04:** Claude Code on Opus 5.5 alone; Roko with a frontier executor; Roko with cheap executors and escalation.
- **The pilot** (E12, spec-567e52): a cheap model alone; a cheap model in Roko; Claude Code on Opus 5.5. It runs 20
  hidden-test tasks with 3 seeds, for at most $15.
- **S09:** `cheap_direct`, `roko_full` and `fd_claude`, plus a 48-task probe.

None of them measures planning or integration (W9 F2).

## Plan

1. **The claim under test,** using tldr/05 decision 9's meaning of "cheaper": equal quality at a lower cost per
   verified task. The cost counts the planner and verification, at API list price, with subscription cash reported
   separately.
2. **The arms:** the pilot first, then the full comparison. Say which arm answers which question.
3. **The metrics,** stratified by task type:
   - cost per verified task;
   - verified success;
   - pass^k over seeds;
   - false greens found by hidden tests;
   - wall-clock time.
4. **The safeguards:**
   - hidden tests;
   - an isolated Claude Code config;
   - the executed model checked on every attempt;
   - the secret kept in a driver-only file.
5. **The falsifiers.** Include pilot numbers only if `vb report --pilot` (gap-d9e9fe) has run; otherwise give none.

## Done when

- [ ] The arms, metrics and falsifiers are stated, with the scope of each arm.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **The author must decide** whether the evaluation covers planning and integration (W9 PW05). Until then, say that
  the pilot measures single tasks.
- Lane `paper`; no hot files.
