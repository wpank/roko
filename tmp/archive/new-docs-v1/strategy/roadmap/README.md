# Roadmap

> Quarter-by-quarter delivery milestones from Q1 Foundation through Q5–Q6 Phase 2 Optionality.
> This is the *when* layer over the architecture refinements. The *what* layer is `strategy/refinements/`.
> The *how the kernel transitions* layer is `strategy/refactor-phases/`.

**Source**: [`docs/00-architecture/35-consolidated-roadmap.md`](../../_migration/cluster-I-strategy.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| — | [00-overview.md](00-overview.md) | The consolidated roadmap narrative: sequencing principles, critical path, team shape | Reference |
| — | [milestone-q1-foundation.md](milestone-q1-foundation.md) | Q1: two-medium kernel + subsystem migration, naming, modularity, observability baseline | Active |
| — | [milestone-q2-learning-substrate.md](milestone-q2-learning-substrate.md) | Q2: HDC fingerprint, demurrage, heuristics, self-learning loops, c-factor, research-to-runtime | Planned |
| — | [milestone-q3-ecosystem-ux.md](milestone-q3-ecosystem-ux.md) | Q3: Plugin SPI, StateHub projection, realtime wire, developer UX, deployment shape | Planned |
| — | [milestone-q4-scale-safety-domains.md](milestone-q4-scale-safety-domains.md) | Q4: Domain profiles, safety spine, replication ledger, deployment hardening, c-factor actuation | Planned |
| — | [milestone-q5q6-phase2-optionality.md](milestone-q5q6-phase2-optionality.md) | Q5–Q6: ChainBus, MeshBus, Dreams, Composer rewrite, plugin registry | Deferred |
| — | [dependencies.md](dependencies.md) | Milestone dependency graph: what must land before what | Reference |
| — | [current-quarter.md](current-quarter.md) | What is targeted this quarter (Q1 snapshot) | Status |
| — | [beyond-current-quarter.md](beyond-current-quarter.md) | Q2, Q3, Q4 outlook and Q5–Q6 optionality | Reference |

---

## Suggested reading order

**For a new reader:** `00-overview.md` → `milestone-q1-foundation.md` → `dependencies.md`

**For a team planning Q2:** `current-quarter.md` → `milestone-q2-learning-substrate.md` → `dependencies.md`

**For a reviewer assessing scope:** `00-overview.md` → `beyond-current-quarter.md` → `dependencies.md`

---

## Quarter summary

| Quarter | Headline | Risk | Demo |
|---|---|---|---|
| Q1 | Two-medium kernel becomes canonical runtime story | Kernel refactor | Self-hosting plan on new kernel vocabulary |
| Q2 | Durable memory becomes semantically indexed and economically shaped | Demurrage rate tuning | Calibrated heuristics, HDC retrieval, c-factor inspection |
| Q3 | Runtime becomes externally legible and extensible | UX scope creep | Third-party plugin + unified CLI/TUI/Web surface |
| Q4 | Domain-shaped, auditable, multi-tenant enough for team use | Multi-tenant auth & isolation | Domain profile + auditable plan + live c-factor |
| Q5–Q6 | Phase 2 optionality | High (distributed trust) | TBD |

---

## Calibration note

This roadmap was drafted as a 5–7 engineer program. **The actual team shape is 1 developer + AI agents.** Quarter labels are full-team estimates. Simultaneous workstreams serialize rather than running in parallel; elapsed timelines stretch accordingly. See [`00-overview.md`](00-overview.md) for the team-shape adjustment section.

---

## Not-doing list (Q1–Q4)

Intentionally out of scope for Q1–Q4:

- Training custom models on accumulated episodes
- A graphical plan editor beyond existing plan views
- Full native SDK parity beyond explicitly planned client surfaces
- A first-party inference server
- A Kubernetes operator beyond Helm-grade packaging
- A native mobile app
- A standalone voice-first workflow

---

## See also

- [`strategy/refactor-phases/`](../refactor-phases/README.md) — the kernel cutover mechanics
- [`strategy/refinements/`](../refinements/README.md) — the source design proposals (REF02–REF34)
- [`analysis/readiness-audit/`](../../analysis/readiness-audit/README.md) — the current-state scorecard this roadmap sequences
- [`analysis/synergy-map/`](../../analysis/synergy-map/README.md) — why the dependency order compounds
