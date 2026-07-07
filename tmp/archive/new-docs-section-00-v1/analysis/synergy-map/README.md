# Synergy Map

> One file per named synergy — where Roko's primitives amplify each other into behaviours that
> none of them could produce alone. These are analysis documents, not specifications.

**Status**: Analysis  
**Crate**: —  
**Last reviewed**: 2026-04-19

---

## Contents

| # | File | Synergy | Primitives |
|---|---|---|---|
| S1 | [synergy-01-demurrage-x-hdc.md](synergy-01-demurrage-x-hdc.md) | Self-trimming semantic memory | P6 Demurrage × P5 HDC |
| S2 | [synergy-02-heuristics-pulse-bus.md](synergy-02-heuristics-pulse-bus.md) | Continuous calibration | P7 Heuristics × P2 Pulse × P3 Bus |
| S3 | [synergy-03-cfactor-bus-hdc.md](synergy-03-cfactor-bus-hdc.md) | Diversity-aware routing | P8 c-factor × P3 Bus × P5 HDC |
| S4 | [synergy-04-replication-living-research.md](synergy-04-replication-living-research.md) | Living research | P9 Replication ledger × P7 Heuristics × P1 paper Engram |
| S5 | [synergy-05-plugin-spi-ecosystem.md](synergy-05-plugin-spi-ecosystem.md) | Ecosystem growth path | P10 Plugin SPI × P4 Substrate × P3 Bus |
| S6 | [synergy-06-cfactor-heuristics-peer-model.md](synergy-06-cfactor-heuristics-peer-model.md) | Peer-model learning | P8 c-factor × P7 Heuristics |
| S7 | [synergy-07-dreams-retroactive.md](synergy-07-dreams-retroactive.md) | Retroactive insight | Dreams × P4 Substrate × P2 Pulse |
| S8 | [synergy-08-demurrage-heuristic-relearning.md](synergy-08-demurrage-heuristic-relearning.md) | Graceful relearning | P6 Demurrage × P7 Heuristics × calibration |
| S9 | [synergy-09-hdc-consensus-agreement.md](synergy-09-hdc-consensus-agreement.md) | Substantive agreement detection | P5 HDC × Consensus × P3 Bus |
| S10 | [synergy-10-typed-context-domain-safety.md](synergy-10-typed-context-domain-safety.md) | Auditable domain safety | TypedContext × domain profiles × Gate |
| — | [00-overview.md](00-overview.md) | Primitive roster, synergy matrix, moat argument | All |
| — | [99-master-synergy-table.md](99-master-synergy-table.md) | Searchable index of all synergies and non-synergies | All |

---

## Suggested Reading Order

For readers new to the synergy framing: [00-overview.md](00-overview.md) → S1 → S3 → S7.

For readers evaluating the moat argument: [00-overview.md](00-overview.md) → S3 → S4 → S6 → [99-master-synergy-table.md](99-master-synergy-table.md).

For readers planning implementation: [99-master-synergy-table.md](99-master-synergy-table.md) (filter by `live` status).

---

## What This Folder Is

The `synergy-map/` documents are **analytical overlays** on the architecture — they explain why
certain combinations of primitives unlock compound behaviours. They are not specifications.
The canonical specification for each primitive lives in `reference/` or `subsystems/`.

The source for this entire subtree is
[`docs/tmp/refinements/34-synergy-integration-map.md`](../../_migration/cluster-F-analysis.md)
(see migration log for full provenance).

---

## What This Folder Is Not

- It is not a feature list. Named synergies are analytical claims, not product requirements.
- It is not a roadmap. For the ordered implementation plan see
  [`strategy/refactor-phases/`](../../strategy/refactor-phases/).
- It is not a duplicate of `integration-map/`. That folder covers point-to-point subsystem wiring.
  This folder covers emergent compound effects across three or more primitives.

---

## Non-Synergies

Some pairs are **intentionally not coupled**. See [00-overview.md § Non-Synergies](00-overview.md)
and [99-master-synergy-table.md § Non-Synergies](99-master-synergy-table.md) for the canonical
list with rationale.

---

## See Also

- [`analysis/integration-map/`](../integration-map/README.md) — point-to-point subsystem wiring
- [`analysis/architectural-analysis/`](../architectural-analysis/README.md) — coherence findings
- [`analysis/readiness-audit/`](../readiness-audit/README.md) — per-subsystem implementation gaps
- [`reference/`](../../reference/) — canonical primitive specifications
