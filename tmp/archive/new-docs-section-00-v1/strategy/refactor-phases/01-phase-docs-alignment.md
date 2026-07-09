# Phase A — Docs Alignment

> Normalize the architecture docs around the two-medium, two-fabric framing before any kernel changes exist.

**Status**: Planned
**Phase index**: 01 of 04
**Duration estimate**: 1 week
**Risk**: None to runtime (doc-only)
**Merge shape**: Reversible by revert; no runtime effect
**Depends on**: nothing (this phase is the starting point)
**Unlocks**: [Phase B — Kernel Addition](02-phase-kernel-addition.md)
**Last reviewed**: 2026-04-19

---

## Goal

Remove terminology drift from the architecture documentation and make the later code work easy to justify and verify. The goal is not to invent new architecture — it is to ensure that what the docs describe matches the intended design so that implementers reading them during Phase B and C do not encounter contradictions.

---

## Scope

- Rewrite foundational architecture chapters to the two-medium (`Engram` / `Pulse`) and two-fabric (`Substrate` / `Bus`) framing.
- Remove stale equivalence disclaimers from the canonical narrative.
- Ensure the glossary defines all load-bearing names once, with cross-references from every chapter that introduces them.
- Synchronize loop, layer, and cross-cut terminology with the newer vocabulary.
- Flag chapters that describe target-state behavior not yet in code with explicit `Status: Specified` markers rather than inline disclaimers.

---

## Prerequisites

None. Phase A is safe to land before any code work begins.

If kernel work slips, the docs can still describe the intended shape; the chapters should remain explicit that the code is not yet there (via frontmatter `Status` tag, not inline disclaimers threaded through the prose).

---

## Deliverables

1. Architecture chapters describe `Engram` as the durable medium and `Pulse` as its ephemeral sibling.
2. `Bus` is treated as a kernel fabric, not a later proposal, across all relevant chapters.
3. `Topic` and `TopicFilter` are named as first-class routing concepts with glossary entries.
4. The universal loop chapter reads as seven steps with co-equal `PERSIST` and `BROADCAST`.
5. The five-layer taxonomy places `Bus` at Layer 0 alongside `Substrate`.
6. Glossary (`GLOSSARY.md`) has entries for: `Engram`, `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`, `PulseSource`.
7. No canonical chapter still relies on a stale equivalence disclaimer as its primary explanation.

---

## Exit Criteria

- [ ] The architecture chapter set consistently uses the two-medium / two-fabric framing.
- [ ] `GLOSSARY.md` has entries for `Engram`, `Pulse`, `Bus`, `Topic`, and `TopicFilter`.
- [ ] The loop chapter (`reference/06-loop/`) and the layer chapter (`reference/08-layers/`) both reflect the seven-step / L0 Bus framing.
- [ ] No canonical chapter still relies on the old equivalence disclaimer as its primary explanation.

---

## Current Status

Not started. This phase will begin with the Cluster I strategy docs migration and associated reference tree work.

---

## Roadmap Alignment

Phase A is the doc-alignment portion of the **Q1 Foundation** milestone. See [`strategy/roadmap/milestone-q1-foundation.md`](../roadmap/milestone-q1-foundation.md).

---

## Risks

- **Doc/runtime drift** can widen if Phase A lands far ahead of Phase B and readers mistake the spec for a description of the current code. Mitigation: explicit `Status: Specified` frontmatter on all target-state pages.

---

## See Also

- [Phase B — Kernel Addition](02-phase-kernel-addition.md)
- [dependencies.md](dependencies.md)
- [success-metrics.md](success-metrics.md)
- [`reference/06-loop/`](../../reference/06-loop/README.md) — universal cognitive loop (target of this phase's doc work)
- [`GLOSSARY.md`](../../GLOSSARY.md)
