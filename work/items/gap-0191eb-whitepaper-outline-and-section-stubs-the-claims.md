+++
id = "gap-0191eb"
kind = "gap"
title = "Whitepaper outline and section stubs: the claims each section makes and their sources"
status = "open"
triage = "unverified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§3 E1, proposed outline)"
anchors = ["docs/whitepaper/README.md", "docs/whitepaper"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["dec-2cd76a", "gap-af0b57"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/README.md && grep -q '^## Outline' docs/whitepaper/README.md && grep -q '^## Conventions' docs/whitepaper/README.md && test -f docs/whitepaper/references.bib && test -f docs/whitepaper/appendix-status-matrix.md && test $(ls docs/whitepaper/[01][0-9]-*.md | wc -l) -ge 11"
+++

## Problem

The whitepaper has no outline. Before section writers start, each needs four things: its file, its word budget, the
claims its section makes, and where the evidence for each claim is.

## Why it matters

Every section item, gap-353d57 through gap-ec516e, anchors a file this item creates and follows the conventions it
sets. Fixing the claims and sources first stops ten parallel writers from drifting into claims the evidence doesn't
support.

## Where

All new:
- `docs/whitepaper/README.md`: a title block (a working title until dec-2cd76a closes), the thesis, `## Outline` and
  `## Conventions`.
- One stub per section, `00-abstract.md` through `10-related-work.md` (the epic lists the names), and
  `appendix-status-matrix.md`, which gap-35a614 later regenerates.
- `references.bib`.

## Current state

Checked at `41c7ffbd6`: `docs/whitepaper/` does not exist.
- **Outline:** PLAN §3 E1 proposes ten sections and a status-matrix appendix, with no abstract. This item adds one,
  `00-abstract.md`.
- **Keys:** tldr/04 cites ten keys, and three of them are in no bib file yet. gap-370d3c lists where each key is.

## Plan

1. **Outline table in README:** file, section, word budget, owning item, claims (short ids such as `I1`) and
   sources.
   - The budgets total about 6,450 words: abstract 150; §1 600, §2 550, §3 750, §4 950, §5 850, §6 450, §7 700,
     §8 550, §9 550, §10 500.
   - The generated appendix has no budget.
2. **Conventions in README:**
   - line 1 of every file is a status header: `Status: stub|draft|reviewed · budget N words · owner <item id>`;
   - status tags are written `TAG@<short sha>`, using the tldr/00 vocabulary;
   - markers follow the research draft's grammar (`[[TODO: …]]`, `[[CITE?: …]]`);
   - citations are `[@key]`, taken from `references.bib`;
   - a footnote on each number names its commit, snapshot, rollup or key;
   - banned words, taken from the draft's style rules;
   - figures go in `figures/`.
3. **Stubs:** each has a `Status: stub` header, a `[[TODO: …]]` line, and its section's claims, each with a source.
4. **Seed `references.bib`** with the ten tldr/04 keys.

## Done when

- [ ] README has the outline and the conventions.
- [ ] The 11 stubs and the appendix stub exist, each listing its claims and sources.
- [ ] `references.bib` holds the ten keys.
- [ ] The `[[verify]]` command passes.

## Notes

- **Stubs fail `paperlint --strict` on purpose** (`Status: stub`, `[[TODO`), so each section item stays open until
  its section is written.
- **Don't wait for dec-2cd76a.** Use PLAN's defaults, and update README when the decision closes.
- **The verify checks structure only,** so it stays true after the sections are written.
- Lane `paper`; no hot files.
- **Decided 2026-09-29 (Will):** the title, audience, length and citation rule are in dec-2cd76a's closing evidence. Put them in `docs/whitepaper/README.md`.
