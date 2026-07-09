# Milestone: Q1 — Foundation

> The two-medium kernel becomes the canonical runtime story and existing subsystems start migrating away from ad hoc transport surfaces.

**Target**: Q1 (full-team estimate; 1 dev + AI agents: adjust accordingly)
**Status**: Active — Planned
**Owner**: Kernel engineer (primary)
**Prerequisites**: None (this is the first milestone)
**Unlocks**: [Q2 — Learning Substrate](milestone-q2-learning-substrate.md)
**Roadmap quarter risk**: Kernel refactor
**Last reviewed**: 2026-04-19

---

## Headline

The two-medium kernel (`Engram` + `Pulse`) becomes the canonical runtime story and existing subsystems start migrating away from ad hoc transport surfaces.

---

## Quarter demo

A self-hosting plan flow runs on the new kernel vocabulary and no longer depends on scattered local publication types.

---

## Tracks

| Track | Scope | Primary docs | Refactor phases |
|---|---|---|---|
| Kernel | Land Phases A–C: `Pulse`, `Bus`, `Datum`, operator generalization, seven-step loop, first subsystem migration | `reference/01-engram/`, `reference/04-bus/`, `strategy/refactor-phases/` | A, B, C |
| Naming | Finish the canonical rename pass; keep the glossary authoritative | `GLOSSARY.md`, `ALIASES.md` | A |
| Modularity | Extract kernel seams (`roko-bus` crate); scaffold the SPI boundary | `reference/11-crate-map.md` | B |
| Observability baseline | Ship the first Roko-specific dashboards and transport-level telemetry | `analysis/readiness-audit/` | C (parallel) |

---

## Deliverables

### Kernel track

- [ ] Phase A complete: architecture docs use two-medium / two-fabric framing throughout
- [ ] Phase B complete: `roko-core` exports `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`
- [ ] Phase C complete: subsystem-specific transport enums gone from migrated paths; TUI polling loops eliminated
- [ ] Self-hosting plan flow runs on new kernel vocabulary

### Naming track

- [ ] `GLOSSARY.md` authoritative for all kernel vocabulary
- [ ] `ALIASES.md` maps public-facing names to canonical internal names
- [ ] No canonical doc still uses stale equivalence disclaimers

### Modularity track

- [ ] `roko-bus` crate scaffolded with the Bus trait seam
- [ ] SPI boundary identified and documented in `reference/11-crate-map.md`

### Observability track

- [ ] First Roko-specific dashboards exist
- [ ] Transport-level telemetry (topic message counts, Bus queue depths) observable

---

## Exit criteria

- [ ] `cargo build --workspace` passes with no transport-enum imports on migrated paths
- [ ] Phase C exit criteria all pass (see [`strategy/refactor-phases/success-metrics.md`](../refactor-phases/success-metrics.md))
- [ ] Demo: self-hosting plan flow on new kernel vocabulary
- [ ] Checkpoint: "Is the kernel cutover still safer than incremental patching?" → Go

---

## Current status

Not started. Strategy docs migration (Cluster I) is the first groundwork.

---

## Risk

**Kernel refactor**: the migration may surface unexpected coupling in Phase C. Mitigation: strict subsystem-by-subsystem order; each subsystem's shims removed before the next starts.

**Month-1 checkpoint**: "Is the kernel cutover still safer than incremental patching?" If no → pause and evaluate incremental patching as an alternative.

---

## REF alignment

| REF | Scope | Lands in |
|---|---|---|
| REF02 | `Pulse` (Engram vs. Pulse distinction) | Phase B |
| REF03 | `Bus` as first-class | Phase B |
| REF04 | `Datum` and generalized operators | Phase B |
| REF05 | Seven-step loop | Phase A (docs) + Phase B (code) |
| REF06 | Phased refactor plan | Phase A–C overall |
| REF07 | Naming and glossary | Phase A |
| REF08 | Code sketches | Phase B |
| REF20 | Modularity and composability | Naming + Modularity tracks |
| REF33 | Observability and telemetry | Observability track |

---

## See also

- [`strategy/refactor-phases/README.md`](../refactor-phases/README.md) — the A–D phase mechanics
- [`strategy/roadmap/milestone-q2-learning-substrate.md`](milestone-q2-learning-substrate.md) — the milestone this unlocks
- [`strategy/roadmap/dependencies.md`](dependencies.md) — full dependency graph
