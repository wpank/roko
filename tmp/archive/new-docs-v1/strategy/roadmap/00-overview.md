# Roadmap Overview

> The consolidated roadmap narrative: sequencing principles, critical path, dependency ladder, and team-shape calibration.

**Status**: Written
**Depends on**: [Refactor Plan Phases](../refactor-phases/README.md), [Synergy & Integration Map](../../analysis/synergy-map/README.md), [Implementation Readiness Audit](../../analysis/readiness-audit/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The architecture refinements (REF02–REF34) describe a coherent target state but create a sequencing problem — some primitives only compound if they land in dependency order. This roadmap makes that order explicit: it is the canonical delivery view over the existing design, not a second design document.

---

## The Sequencing Problem

Some primitives must land before others to produce compound value:

- `Pulse` before `Bus` migration
- `Bus` before subsystem rewiring
- HDC fingerprint before demurrage tuning
- Heuristics before c-factor actuation
- StateHub projection before multi-surface UX parity

Without an explicit sequence, the refinements risk landing in an order that either blocks each other or produces impressive-sounding docs that cannot yet produce demoable capabilities.

---

## Five Sequencing Principles

1. **Dependency order first.** Land the primitive that later work assumes before landing the dependent work.
2. **One major risk per phase.** Kernel cutovers, demurrage tuning, and multi-tenant safety splits should not peak in the same quarter.
3. **Ship visible wins.** Every quarter needs a demoable capability, not only internal cleanup.
4. **Parallelize independent tracks.** Learning, UX, and platform work can advance together once the kernel critical path is stable.
5. **Keep adjacent plans non-blocking.** This roadmap aligns with `tmp/ux-followup/` and `tmp/MASTER-PLAN.md` but does not require those tracks to stop while the refinements land.

---

## Critical Path

The architecture has one clear critical path and several dependent branches:

1. **Kernel framing and migration**: REF02 `Pulse`, REF03 `Bus`, REF04 `Datum` and generalized operators, REF05 seven-step loop, REF06 phased refactor plan, REF07 naming, REF08 code sketches.
2. **Crate boundary cleanup**: REF20 modularity and composability turns the kernel story into enforceable package seams.
3. **Learning substrate**: REF10 self-learning loops, REF11 HDC fingerprint, REF12 demurrage, REF13 c-factor, REF14 heuristics, REF16 research-to-runtime — build on the stabilized two-medium, two-fabric runtime.
4. **Surface and ecosystem work**: REF17 plugin SPI, REF22 developer UX, REF23 user UX, REF24 deployment UX, REF25 domain profiles, REF26 StateHub projection, REF27 realtime wire, REF28 CLI parity, REF29 web UI, REF30 rich UX primitives — depend on the earlier kernel and learning layers.
5. **Integrators and Phase 2**: REF31 synergy framing, REF32 safety spine, REF33 observability, REF09 Phase-2 Bus/Substrate backends — harden or extend what the earlier quarters establish.

**Rule of thumb**: if a workstream assumes a shared `Bus`, typed `Pulse` topics, or calibration-bearing heuristics, it is downstream of the kernel and learning substrate.

---

## Dependency Ladder (Key Edges)

| Upstream | Must land before | Why |
|---|---|---|
| `Pulse` + `Bus` + `Datum` | Subsystem migration + seven-step loop cleanup | Shared transport and operator vocabulary must exist before callers can converge on it |
| Kernel migration | HDC fingerprint, demurrage, heuristics, self-learning loops | Learning substrate assumes two-medium, two-fabric runtime is stable |
| HDC fingerprint + demurrage + heuristics | c-factor actuation + replication-ledger expansion | Calibration and cohort policy only compound once memory, evidence, and confidence surfaces are real |
| Plugin SPI + StateHub projection + shared wire protocol | Multi-surface UX parity + domain-profile rollout | External surfaces and extensions need one shared runtime contract |
| Safety spine + deployment hardening | Phase-2 backends + cross-deployment witness flows | Distributed trust should extend a hardened runtime, not precede it |

See [`dependencies.md`](dependencies.md) for the full graph.

---

## Team-Shape Calibration

This roadmap was originally drafted as a 5–7 engineer program. **The actual project shape is 1 developer + AI agents.**

| Role | Primary ownership |
|---|---|
| Kernel engineer | Q1 runtime changes, then stewardship through Q4 |
| Learning engineer | Q2 learning substrate, heuristics, calibration |
| UX engineer | Q3 surface work across CLI, TUI, web |
| Platform engineer | Plugin SPI, deployment, observability, safety |
| Domain lead | Q4 domain profiles and domain-specific tooling |

With 1 developer + AI agents:
- Treat quarter labels as full-team estimates, not calendar commitments.
- Reduce concurrent tracks: many Q3–Q4 tracks will serialize.
- A comfortable Q1–Q4 plan is roughly 6–12 months wall-clock, with extra range driven by how many Q3–Q4 tracks can truly run in parallel.

---

## Risk Register and Checkpoints

| Checkpoint | Question | Why it matters |
|---|---|---|
| End of Q1 month 1 | Is the kernel cutover still safer than incremental patching? | Prevents migration from turning into indefinite compatibility debt |
| End of Q2 | Is demurrage producing useful compounding instead of cold-tier churn? | Protects learning substrate from false sophistication |
| End of Q3 | Are external plugins actually installing and surviving onboarding? | Tests whether the SPI is real ecosystem leverage |
| End of Q4 | Are domain profiles producing surprising replication-ledger findings? | Distinguishes a live domain platform from a themed demo |

Supporting risks active across multiple quarters:
- HDC encoder drift across deployments — requires versioned fingerprints.
- Plugin ABI churn — requires release discipline and semver boundaries.
- c-factor should remain a diagnostic/regulator, not a direct objective to reward-hack.
- Cross-doc vocabulary drift — caught by keeping the glossary and sequencing docs aligned.

---

## One-Year Outcome

If Q1–Q4 land in order on a full-team schedule:

1. The kernel speaks one transport and storage language: `Engram`, `Pulse`, `Substrate`, `Bus`, `Topic`, `TopicFilter`, `Datum`, `PulseSource`.
2. The learning layer compounds through HDC fingerprint, demurrage, heuristics, and c-factor — not isolated experiments.
3. Plugins, StateHub projection, and surface clients share one runtime contract.
4. Domain profiles and safety infrastructure make the system auditable for team workflows.

---

## Source

- Primary source: `docs/00-architecture/35-consolidated-roadmap.md` (archived)
- Source proposal: `tmp/refinements/35-consolidated-roadmap.md` (not yet migrated)
- Related: `tmp/ux-followup/`, `tmp/MASTER-PLAN.md` (planning artifacts, not yet migrated)

---

## See Also

- [`dependencies.md`](dependencies.md) — full dependency graph
- [`current-quarter.md`](current-quarter.md) — Q1 scope and active work
- [`beyond-current-quarter.md`](beyond-current-quarter.md) — Q2–Q4 outlook
- [`strategy/refactor-phases/00-overview.md`](../refactor-phases/00-overview.md) — Phase A–D mechanics
- [`analysis/synergy-map/`](../../analysis/synergy-map/README.md) — why the dependency order compounds
