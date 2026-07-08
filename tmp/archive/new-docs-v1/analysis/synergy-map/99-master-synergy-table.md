# Master Synergy Table

> Searchable index of all ten named synergies, the three named non-synergies, and the three
> emergent properties. Use this file to locate any synergy by primitive, status, or topic.

**Status**: Analysis  
**Crate**: —  
**Last reviewed**: 2026-04-19

---

## Primitive Roster Quick Reference

| # | Primitive | Short name | Status |
|---|---|---|---|
| P1 | Engram | `Engram` | Shipping |
| P2 | Pulse | `Pulse` | Scaffold |
| P3 | Bus / `EventBus<E>` | `Bus` | Built (EventBus); Scaffold (Bus trait) |
| P4 | Substrate | `Substrate` | Shipping |
| P5 | HDC fingerprint | `HDC` | Built (partial) |
| P6 | Demurrage | `Demurrage` | Specified |
| P7 | Heuristics + falsifiers | `Heuristics` | Scaffold |
| P8 | c-factor | `c-factor` | Built (partial) |
| P9 | Replication ledger | `Ledger` | Specified |
| P10 | Plugin SPI + domain profiles | `Plugins` | Scaffold |

---

## Named Synergies — Master Table

| ID | Name | File | Primitives | Status | Unlocks |
|---|---|---|---|---|---|
| S1 | Self-trimming semantic memory | [synergy-01-demurrage-x-hdc.md](synergy-01-demurrage-x-hdc.md) | P6 × P5 (+ P1, P4) | Target-state | Memory that favors uniqueness over accumulation |
| S2 | Continuous calibration | [synergy-02-heuristics-pulse-bus.md](synergy-02-heuristics-pulse-bus.md) | P7 × P2 × P3 | Target-state | Streaming confidence updates from lived outcomes |
| S3 | Diversity-aware routing | [synergy-03-cfactor-bus-hdc.md](synergy-03-cfactor-bus-hdc.md) | P8 × P3 × P5 | Target-state | Regulatory monoculture detection and correction |
| S4 | Living research | [synergy-04-replication-living-research.md](synergy-04-replication-living-research.md) | P9 × P7 × P1 | Target-state | Claims that update as evidence arrives |
| S5 | Ecosystem growth path | [synergy-05-plugin-spi-ecosystem.md](synergy-05-plugin-spi-ecosystem.md) | P10 × P4 × P3 | Target-state | Structurally safe extension along declared seams |
| S6 | Peer-model learning | [synergy-06-cfactor-heuristics-peer-model.md](synergy-06-cfactor-heuristics-peer-model.md) | P8 × P7 | Target-state | Measurable social perception, lower coordination friction |
| S7 | Retroactive insight | [synergy-07-dreams-retroactive.md](synergy-07-dreams-retroactive.md) | Dreams × P4 × P2 | Target-state | Re-learning from memory under updated priors |
| S8 | Graceful relearning | [synergy-08-demurrage-heuristic-relearning.md](synergy-08-demurrage-heuristic-relearning.md) | P6 × P7 × calibration | Target-state | Confidence softening; prevents rule stagnation |
| S9 | Substantive agreement detection | [synergy-09-hdc-consensus-agreement.md](synergy-09-hdc-consensus-agreement.md) | P5 × Consensus × P3 | Target-state | Semantic consensus; distinguishes genuine from nominal agreement |
| S10 | Auditable domain safety | [synergy-10-typed-context-domain-safety.md](synergy-10-typed-context-domain-safety.md) | TypedContext × P10 × Gate | Partial (Gate built) | Auditable, typed domain enforcement with Custody records |

---

## Synergies by Primitive

Use this table to find all synergies that touch a given primitive.

| Primitive | Participates in |
|---|---|
| P1 Engram | S1 (substrate of record), S4 (paper Engram) |
| P2 Pulse | S2 (calibration trial), S5 (lifecycle events), S7 (reinterpretation output) |
| P3 Bus | S2 (falsifier watch / calibration routing), S3 (work distribution stats), S5 (plugin lifecycle), S9 (agreement Pulses) |
| P4 Substrate | S1 (home of fingerprinted Engrams), S4 (paper Engram store), S5 (plugin read/write surface), S7 (historical Engram source), S10 (Custody record store) |
| P5 HDC | S1 (novelty score), S3 (output convergence detection), S9 (semantic endorsement fingerprint) |
| P6 Demurrage | S1 (holding cost on Engrams), S8 (holding cost on confidence) |
| P7 Heuristics | S2 (rule being calibrated), S4 (lifted claim form), S6 (peer-model form), S8 (target of confidence decay) |
| P8 c-factor | S3 (policy signal integrator), S6 (peer-model accuracy component) |
| P9 Ledger | S4 (evidence + falsification history) |
| P10 Plugins | S5 (extension surface), S10 (domain profile packaging) |
| Dreams | S7 (reinterpretation engine) |
| TypedContext | S10 (structured situation carrier) |
| Gate | S10 (typed predicate evaluator) |
| Consensus | S9 (aggregator of semantic endorsements) |

---

## Synergies by Status

### Partially live today (individual primitives exist; synergy plumbing is target-state)

| Synergy | What is live | What is missing |
|---|---|---|
| S10 | Gate operator (Built); Substrate (Shipping) | TypedContext full schema, domain profile bundles, Custody Engram writes |
| S3 | HDC partial (Built); c-factor partial (Built); `EventBus<E>` (Built) | Bus statistics API; c-factor regulatory emitter; PolicyPulse types |

### Target-state (one or more core primitives are Specified or Scaffold)

S1, S2, S4, S5, S6, S7, S8, S9 — all depend on at least one of: Demurrage (P6), Pulse (P2),
generalized Bus (P3), Replication ledger (P9), Heuristics runtime (P7), or Plugin SPI (P10).

---

## Synergies by Layer

| Layer | Synergies |
|---|---|
| Memory layer | S1 (semantic pruning), S7 (retroactive reinterpretation) |
| Knowledge layer | S4 (living research), S8 (graceful relearning), S2 (continuous calibration) |
| Social / coordination layer | S6 (peer-model), S9 (substantive agreement), S3 (diversity-aware routing) |
| Ecosystem / safety layer | S5 (plugin ecosystem), S10 (auditable domain safety) |

---

## Synergies by Number of Primitives

| Count | Synergies |
|---|---|
| 2 primitives | S6 (P8 × P7) |
| 3 primitives | S1, S2, S3, S4, S5, S7, S8, S9, S10 |

---

## Named Non-Synergies

These pairs are intentionally not coupled. The architecture stays clean by refusing to force
these connections.

| Pair | Why they do NOT couple |
|---|---|
| P5 HDC × P9 Replication ledger | Papers may be fingerprinted, but the ledger's rigor comes from evidence and falsification, not from similarity search. These are different epistemics. |
| P10 Plugins × P8 c-factor | Plugins can contribute telemetry that **informs** c-factor, but c-factor is not a plugin-selection rule. Conflating them would make c-factor dependent on the plugin registry, violating its role as a cross-cutting signal. |
| P2 Pulse × P9 Replication ledger | Different levels of granularity. Pulses are live stream material. The ledger consumes **stabilized claims**, not raw chatter. Connecting them directly would make the ledger reactive to noise. |

---

## Emergent Properties

Three compound properties are expected to emerge from the full synergy lattice once the target-
state primitives land. None are fully live today.

| Property | Synergies required | Why it emerges |
|---|---|---|
| Self-improvement without a separate training pipeline | S2, S4, S6, S8 | The runtime predicts, calibrates, and updates through its own Bus-mediated feedback. Training is a continuous process, not a periodic pipeline. |
| Inspectability at every level | S10, S9, S4, S7 | Pulse lineage, Engram lineage, heuristic provenance, Custody records, and ledger status together make every decision traceable without requiring a separate observability stack. |
| Substrate neutrality | S1, S5, S7 | Because key behaviors are driven by HDC, demurrage, heuristics, and policy, the system can swap storage or transport implementations without changing its cognitive behavior. |

---

## Cross-Synergy Dependency Graph

```
S2 ──────────────── feeds evidence to ──────────────────── S4
S8 ──────────────── confidence softening for ────────────── S4
S2 ──────────────── calibration loop reused by ─────────── S6
S8 ──────────────── applies equally to peer-model rules in ─ S6
S1 ──────────────── demurrage on Engrams; S8 ── demurrage on confidence
S3 ──────────────── uses c-factor that S6 ─── helps calibrate
S5 ──────────────── installs domain profiles consumed by ── S10
S7 ──────────────── may update HDC fingerprints affecting ─ S1
S4 ──────────────── divergent endorsements feed ─────────── S9
```

---

## How to Use This Table

**Finding which synergies gate implementation of a feature**: Look up the primitives your feature
touches in the "Synergies by Primitive" table. Every synergy listed is a compound behavior
your feature participates in — check that your implementation doesn't break any of them.

**Evaluating moat strength at a given implementation stage**: Count the number of synergies
that have all their required primitives in `Built` or `Shipping` status. The moat grows with
this count, not just with the primitive count.

**Designing tests for emergent properties**: Each emergent property lists the required synergies.
A minimal integration test for that property exercises all synergies listed, even if only
partially.

---

## See Also

- [`00-overview.md`](00-overview.md) — synergy matrix, seven-step loop, moat argument
- Individual synergy files: [S1](synergy-01-demurrage-x-hdc.md) · [S2](synergy-02-heuristics-pulse-bus.md) · [S3](synergy-03-cfactor-bus-hdc.md) · [S4](synergy-04-replication-living-research.md) · [S5](synergy-05-plugin-spi-ecosystem.md) · [S6](synergy-06-cfactor-heuristics-peer-model.md) · [S7](synergy-07-dreams-retroactive.md) · [S8](synergy-08-demurrage-heuristic-relearning.md) · [S9](synergy-09-hdc-consensus-agreement.md) · [S10](synergy-10-typed-context-domain-safety.md)
- [`analysis/integration-map/99-master-lattice.md`](../integration-map/99-master-lattice.md) — point-to-point integration index
- [`analysis/readiness-audit/99-next-actions.md`](../readiness-audit/99-next-actions.md) — implementation priority list
