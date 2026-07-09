# Analysis

> Meta-documentation about the architecture. These documents are analytical views of the system —
> audits, integration maps, and gap analyses — not specifications of the system itself.

---

## What This Folder Covers

The `analysis/` tree contains four distinct types of investigation:

| Subfolder | What it contains | Primary source |
|---|---|---|
| [`architectural-analysis/`](architectural-analysis/README.md) | Coherence analysis of the Synapse Architecture: trait sufficiency, layer violations, category theory grounding, novel proposals | `23-architectural-analysis-improvements.md` |
| [`integration-map/`](integration-map/README.md) | One file per pair of interacting subsystems — data flows, trait exchanges, missing connections | `24-cross-section-integration-map.md` |
| [`readiness-audit/`](readiness-audit/README.md) | Per-subsystem implementation readiness scores, gaps, and next actions | `31-implementation-readiness-audit.md` |
| [`synergy-map/`](synergy-map/README.md) | Where components amplify each other — the ten named synergies and their matrix | `34-synergy-integration-map.md` |

---

## What This Folder Is Not

These documents are **about** the architecture, not part of it. They do not define types, specify
traits, or describe subsystem behavior. For specifications, see:

- [`reference/`](../reference/README.md) — types, traits, operators, loops
- [`subsystems/`](../subsystems/) — implementation-level subsystem docs
- [`operations/`](../operations/README.md) — running Roko in practice

---

## How to Use This Folder

**If you are about to implement a new feature**, read [`integration-map/README.md`](integration-map/README.md)
first to understand which subsystems you will need to connect, and check
[`readiness-audit/`](readiness-audit/README.md) to see whether the target subsystem has outstanding
gaps that affect your work.

**If you are fixing an architectural issue**, start with
[`architectural-analysis/README.md`](architectural-analysis/README.md). Every finding has a numbered
file; open the relevant one for the full analysis and proposed fix.

**If you are evaluating the system's moat or composability**, read
[`synergy-map/README.md`](synergy-map/README.md) and the ten named synergy files.

**If you are planning the next implementation sprint**, the consolidated action list at
[`readiness-audit/99-next-actions.md`](readiness-audit/99-next-actions.md) is the entry point.

---

## Date of Analysis

The analyses in this folder were produced on 2026-04-12 through 2026-04-13, covering all 21
documentation sections (excluding 21-references), 350+ files, and 18 crates (~177K LOC, ~3,391 tests).

## See Also

- [`strategy/refactor-phases/`](../strategy/refactor-phases/) — the phased implementation plan
- [`strategy/roadmap/`](../strategy/roadmap/) — consolidated roadmap
- [`reference/12-design-principles.md`](../reference/12-design-principles.md) — the principles the analysis holds the architecture against
