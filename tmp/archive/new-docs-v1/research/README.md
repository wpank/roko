# Research

> Theoretical foundations, research perspectives, and frontier ideas. This folder contains
> the "why this works" and "where this could go" — as opposed to `reference/`, which contains
> the "what it is" and "how it works".

**Status**: Active (Cluster G complete; foundations and all six perspectives written)
**Last reviewed**: 2026-04-19

---

## What This Folder Is

`research/` is the intellectual scaffolding of the Roko documentation tree. It contains:

- **Foundations** — the academic and theoretical grounding for architecture decisions
- **Perspectives** — extended essay collections, each exploring Roko through a distinct lens
- **Innovations** — individual innovations and their provenance (split from the 117 KB monolith)
- **References** — paper citations with one-paragraph summaries

This folder is not part of the specification. Nothing here is required reading for
implementing or operating Roko. Everything here is context: why certain architectural
choices were made, what research supports them, and what open problems remain.

---

## Contents

### `foundations/` — Theoretical Grounding

| File | Topic | Status |
|---|---|---|
| [`active-inference.md`](foundations/active-inference.md) | Free-energy minimization, prediction-error loops, Markov blankets, dual-process | Cluster G |
| [`autocatalysis.md`](foundations/autocatalysis.md) | Kauffman RAF sets, self-improving scaffolds, NK landscapes, autocatalytic growth | Cluster G |
| [`c-factor.md`](foundations/c-factor.md) | Collective intelligence factor, Woolley et al., multi-agent coordination | Cluster G |
| [`cybernetics.md`](foundations/cybernetics.md) | Cybernetic regulation, Conant-Ashby, Beer VSM, EWMA, variety engineering | Cluster G |

These foundational documents explain the research traditions that motivated specific
Roko design decisions. Each links to the architecture document where the decision
was implemented.

See [`foundations/README.md`](foundations/README.md) for the suggested reading order and
cross-references.

### `perspectives/` — Extended Essay Collections

Each perspective is a folder containing 5–7 files. Together they form a self-contained
essay collection exploring Roko through one conceptual lens. The arc within each folder is:
*what is this lens → what does it illuminate → what follows for Roko*.

| Folder | Lens | Files |
|---|---|---|
| [`attention-as-currency/`](perspectives/attention-as-currency/) | Attention as an economic resource; scoring and routing as allocation markets | 6 |
| [`immune-system/`](perspectives/immune-system/) | Adversarial robustness as layered immune response; Gate and Neuro as innate/adaptive layers | 6 |
| [`temporal-topology/`](perspectives/temporal-topology/) | Knowledge as topological space; Decay and Dreams as topological operators | 7 |
| [`emergent-goals/`](perspectives/emergent-goals/) | Goal formation as attractor dynamics; Daimon and Policy as attractor-shaping mechanisms | 6 |
| [`energy-model/`](perspectives/energy-model/) | Cognitive effort as energy budget; T0/T1/T2 tiers as metabolic states | 6 |
| [`collective-intelligence/`](perspectives/collective-intelligence/) | Roko as a cognitive collective; c-factor, transactive memory, and ensemble diversity | 6 |

See [`perspectives/README.md`](perspectives/README.md) for the full perspective index and
recommended navigation paths.

### `innovations/` — Per-Innovation Files

Source: `docs/00-architecture/30-cross-pollination-innovations.md` (117 KB monolith, 2903 lines)

This folder will contain one file per distinct innovation described in that document —
an estimated 30–80 files. Each file covers one idea, its research provenance, its
implementation status in Roko, and how it interacts with other innovations.

Status: **Cluster H** (not yet populated).

### `references/` — Paper Summaries

One file per cited paper or technical report. Each file contains:
- Full citation (authors, year, venue, DOI or arXiv ID)
- One-paragraph summary of the key contribution
- How Roko uses or extends the finding
- Link to the architecture document that cites it

Status: **Cluster G** (populated as part of Cluster G's foundational content; references
are embedded in perspective and foundation files pending extraction into standalone pages).

---

## Entry Points by Interest

| If you want to understand… | Start here |
|---|---|
| Why Roko's scoring is structured the way it is | [`perspectives/attention-as-currency/`](perspectives/attention-as-currency/) |
| The basis for active inference in the cognitive loop | [`foundations/active-inference.md`](foundations/active-inference.md) |
| Why Roko has multiple cognitive speeds | [`perspectives/energy-model/`](perspectives/energy-model/) |
| How collective intelligence is measured | [`foundations/c-factor.md`](foundations/c-factor.md) |
| The immune system analogy for safety and gating | [`perspectives/immune-system/`](perspectives/immune-system/) |
| How knowledge topology shapes memory design | [`perspectives/temporal-topology/`](perspectives/temporal-topology/) |
| How goals can emerge unintentionally | [`perspectives/emergent-goals/`](perspectives/emergent-goals/) |
| How Roko functions as a cognitive collective | [`perspectives/collective-intelligence/`](perspectives/collective-intelligence/) |
| Self-improvement and autocatalytic growth | [`foundations/autocatalysis.md`](foundations/autocatalysis.md) |
| Roko's relationship to the research frontier | [`frontier-summary.md`](frontier-summary.md) |
| All research paper citations | [`references/`](references/) |

---

## How the Perspectives Tree Works

Each perspective folder is independent: a reader can open any folder and read straight
through without needing the others. Within a folder, the standard arc is:

```
00-overview.md         — The lens: what is this perspective about?
01-<concept>.md        — The foundational theory
02-<mechanism>.md      — How the mechanism works
03-roko-application.md — How Roko maps onto the theory
04-implications.md     — What follows for design (constraints, measurements, protocols)
05-open-questions.md   — Unresolved questions at the frontier
```

Some perspectives have an additional file between 01 and 03 when the theory requires more
unpacking (see `temporal-topology/` for an example with 7 files).

The perspective files are *essays*, not *specifications*. They are intentionally longer and
more exploratory than `reference/` pages. Claims here should cite papers or Roko codebase
results; purely speculative claims should be marked as such.

---

## Authorship Note

The content in `research/` is the most speculative part of the documentation tree. Where
`reference/` describes what ships, `research/` often describes what is possible, what the
design is working toward, and what the open questions are. Hold the claims here to a higher
standard of sourcing: every causal claim should cite a paper or an empirical result from
the Roko codebase itself.

---

## See Also

- [`reference/12-design-principles.md`](../reference/12-design-principles.md) — how research maps to architectural decisions
- [`research/frontier-summary.md`](frontier-summary.md) — the cross-cutting frontier narrative
- [`reference/README.md`](../reference/README.md) — the specification tree

## Open Questions

- Should `references/` be auto-generated from a BibTeX file, or maintained manually?
  Manual is more context-rich; auto is easier to keep current.
- Should individual perspective files cross-link to each other more aggressively? Currently
  each perspective is self-contained. Cross-perspective links would make the research tree
  a richer web.
- When `innovations/` (Cluster H) is populated, should it be integrated into the entry-points
  table above, or kept as a separate navigable sub-tree?
