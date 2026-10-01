+++
id = "gap-370d3c"
kind = "gap"
title = "Whitepaper §2 Design principles"
status = "done"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md (design rules 1-8)"
anchors = ["docs/whitepaper/02-design-principles.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-ec516e", "gap-ac4646"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/02-design-principles.md && test $(grep -oF '[@' docs/whitepaper/02-design-principles.md | wc -l) -ge 8 && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/02-design-principles.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Whitepaper §2 design principles merged in 29a9dc4a7 (eight rules, 14 citations); verify passes at HEAD."
+++

## Problem

tldr/04 distils eight design rules from the literature:
1. Size tasks for the executor.
2. Split on independent outputs.
3. The planner writes the gating checks.
4. Assume visible checks will be gamed.
5. Retry twice, then escalate, then split.
6. Merge, then verify, through a queue.
7. Put ambiguity back into authoring.
8. Count cost per verified task.

The whitepaper needs these as its design principles, each with a verified citation.

## Why it matters

The rules justify the golden path, and §4 cites them step by step. They also keep the claims modest. The literature
supports about frontier quality at 2–6× lower cost on decomposable, checkable work, mostly measured on single-function,
QA and document tasks. It does not support that on sequential or integrative work (C1).

## Where

- `docs/whitepaper/02-design-principles.md` (new; gap-0191eb creates it as a stub).
- `docs/whitepaper/references.bib`.

## Current state

Checked at `41c7ffbd6` and in the bib files on 2026-09-29. The rules cite ten keys:
- **four are in the research draft's `references.bib`:** `zhao2026specbench`, `vijayvargiya2025ambigswe`,
  `edwards2026askorassume` and `rajput2026cheap`;
- **three are only in its `corpus.jsonl`:** `kwa2025measuring`, `konstantinou2024do` and `prasad2024adapt`;
- **three are in no bib file,** though research note C1 verified them with refcheck: `kim2025towards`,
  `sinha2025illusion` and `brun2011proactive`.

gap-0191eb seeds all ten. No merge today touches these sources.

## Plan

1. **One short paragraph per rule:** the rule, the evidence (its size and the setting it was measured in), and what
   Roko does about it, pointing to the step in §4.
2. **One paragraph of cybernetic vocabulary:** essential variables, regulation, and audits of the regulators. Take it
   from the research draft's §2 background. Cite *Agent Cybernetics* as the closest framing, and claim no firstness.
3. **Say where each result was measured,** as C1 does, so that a single-function result is never presented as
   repository-scale evidence.

## Done when

- [x] Each of the eight rules cites at least one verified source.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **`references.bib` is shared with §10** (gap-ec516e). Add new keys in a separate small commit, after checking them
  with `tmp/cybernetic-harness/tools/refcheck.py`.
- Lane `paper`; no hot files.
- **Written on `work/gap-370d3c` (2026-09-29)** at `d7f854a84`, with the two new bib keys (`wang2026agent`,
  `ashby1960design`) in `c9ee7cf1b`. 666 words (1.21× the budget, inside `--strict`'s 0.5–1.3×) and 14 citations,
  each with the setting it was measured in.
  - The verify's static part passes. Its paperlint part waits for gap-af0b57: `tools/paperlint.py` isn't at BASE
    `1f4481133`. The unmerged copy in gap-af0b57's worktree gives `--strict --check-identifiers` clean.
  - The section has no status tags; each rule points to its step in §4, which takes its tags from the status matrix.
