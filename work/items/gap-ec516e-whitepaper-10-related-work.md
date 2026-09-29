+++
id = "gap-ec516e"
kind = "gap"
title = "Whitepaper §10 Related work"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/paper/sections/03a-related-work.md and 03b-related-work.md (§3.1-3.7)"
anchors = ["docs/whitepaper/10-related-work.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-370d3c", "gap-424bf8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/10-related-work.md && test $(grep -oF '[@' docs/whitepaper/10-related-work.md | wc -l) -ge 10 && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/10-related-work.md"
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
