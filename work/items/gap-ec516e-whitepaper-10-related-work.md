+++
id = "gap-ec516e"
kind = "gap"
title = "Whitepaper §10 Related work"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "dc371b1ae"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/paper/sections/03a-related-work.md and 03b-related-work.md (§3.1-3.7)"
anchors = ["docs/whitepaper/10-related-work.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-370d3c", "gap-424bf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/10-related-work.md && test $(grep -oF '[@' docs/whitepaper/10-related-work.md | wc -l) -ge 10 && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/10-related-work.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "Whitepaper §10 related work merged in 360becba3 (27 citations; bib deduped and repaired in 86e7f6ea1, 260a54f40, 3796db34e; pandoc citeproc clean); verify passes."
+++

## Problem

The whitepaper needs a short related-work section. The research draft already has a long one: §3.1–3.7 in
`03a-related-work.md` and `03b-related-work.md`, about 5,350 words of stable prose from refcheck-verified sources. The
whitepaper can afford about 500 words.

## Why it matters

The section places Roko among its neighbours without claiming to be first:
- harness engineering;
- products that pair a frontier planner with a cheaper executor;
- routing and cascades;
- verification and specification gaming;
- cybernetic framings of agents.

## Where

- `docs/whitepaper/10-related-work.md` (new; gap-0191eb creates it as a stub).
- `docs/whitepaper/references.bib`.

## Current state

Checked on 2026-09-29.
- **The research draft's related work** has the status "stable-draft". A final search for prior art is still to come,
  before that paper is submitted (`paper/OUTLINE.md`, D13).
- **Research notes C1–C4** (2026-09-29) add the product comparisons (C3) and the evidence behind the loops (C4).
- No merge today touches these sources.

## Plan

1. **Five short paragraphs:**
   - harness engineering and self-improving harnesses;
   - agent frameworks and coding agents, including frontier-planner products: `opusplan`, Devin Fusion, aider's
     architect mode and Kiro's spec waves;
   - routing and cascades;
   - verification, gaming and audits;
   - cybernetics for agents.
2. **Condense rather than rewrite:** keep the draft's citations and its hedges.
3. **Date the survey:** "literature as of 2026-09-29".

## Done when

- [ ] Every cited key is in `references.bib` and was verified with refcheck.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **`references.bib` is shared with §2** (gap-370d3c). Add keys in a separate small commit.
- **Vendor claims from C3** carry their fetch date and cite the vendor pages.
- Lane `paper`; no hot files.
- **Written on `work/gap-ec516e` (2026-09-29):** the section in `e3df0fb42`, its keys in `543b2fc2b`.
  - 550 words against the budget of 500 (1.10×), with 27 `[@` citations over 32 keys; 29 keys are new in
    `references.bib`.
  - The static part of the verify passes. `tools/paperlint.py` has not merged (gap-af0b57), so the full verify can't
    run on this branch; the in-progress copy in gap-af0b57's worktree reports the file clean under `--strict`. Close
    the item once paperlint merges and the verify passes.
  - §1, §2 and §5 may add some of the same keys: `chen2024frugalgpt` and `ong2025routellm` (IN2), `wang2026agent` and
    `ashby1960design` (DP9), `wang2026compound` and `wang2026rethinking` (CM9). Keep one copy of each on merge.
  - The four vendor pages were fetched on 2026-09-29. refcheck can't check web pages, so each entry's comment quotes
    the page instead.
