---
title: "Integration Map"
section: analysis
subsection: integration-map
---

# Integration Map

> **Source**: 24-cross-section-integration-map.md  
> **Purpose**: Documents every pairwise connection between Roko's 22 architecture sections — both wired connections and missing integrations (M1-M20).

## Structure

Each file documents **one pair** of sections:

- **Direction**: which section drives the other (or bidirectional)
- **What flows**: the Engram kinds, trait calls, or config values exchanged
- **Status**: Wired / Missing (Mx) / Partial
- **Invariants** and **failure modes** of the interaction

File naming: `<subsystem-a>-x-<subsystem-b>.md` where subsystem-a < subsystem-b alphabetically (or the driver comes first for directional pairs).

## Pair Files

### Tier 1 Missing Integrations (~310 LOC)
- [daimon-x-composition.md](./daimon-x-composition.md) — M2
- [daimon-x-orchestration.md](./daimon-x-orchestration.md) — M1
- [learning-x-composition.md](./learning-x-composition.md) — M4 (Skills→Prompts)
- [learning-x-routing.md](./learning-x-routing.md) — M6 (Cost→Routing)
- [verification-x-orchestration.md](./verification-x-orchestration.md) — M3 (Failure→Replanning)

### Tier 2 Missing Integrations (~620 LOC)
- [anti-knowledge-x-composition.md](./anti-knowledge-x-composition.md) — M15
- [code-intel-x-composition.md](./code-intel-x-composition.md) — M8
- [conductor-x-routing.md](./conductor-x-routing.md) — M9
- [learning-x-config.md](./learning-x-config.md) — M10 (Experiments→Static)
- [neuro-x-composition.md](./neuro-x-composition.md) — M5
- [orchestration-x-daimon.md](./orchestration-x-daimon.md) — M11

### Tier 3 Missing Integrations (~620 LOC)
- [code-intel-x-verification.md](./code-intel-x-verification.md) — M16
- [dreams-x-daimon.md](./dreams-x-daimon.md) — M18
- [dreams-x-neuro.md](./dreams-x-neuro.md) — M7
- [lifecycle-x-neuro.md](./lifecycle-x-neuro.md) — M20
- [neuro-x-verification.md](./neuro-x-verification.md) — M14
- [safety-x-composition.md](./safety-x-composition.md) — M13

### Tier 4 Missing Integrations (~520 LOC)
- [coordination-x-dreams.md](./coordination-x-dreams.md) — M19
- [coordination-x-orchestration.md](./coordination-x-orchestration.md) — M12
- [tech-analysis-x-heartbeat.md](./tech-analysis-x-heartbeat.md) — M17

### Wired / Stable Connections
- [agents-x-composition.md](./agents-x-composition.md) — Wired
- [agents-x-verification.md](./agents-x-verification.md) — Wired
- [conductor-x-orchestration.md](./conductor-x-orchestration.md) — Wired
- [daimon-x-learning.md](./daimon-x-learning.md) — Wired
- [learning-x-verification.md](./learning-x-verification.md) — Wired
- [neuro-x-learning.md](./neuro-x-learning.md) — Partial
- [orchestration-x-learning.md](./orchestration-x-learning.md) — Wired
- [safety-x-agents.md](./safety-x-agents.md) — Wired

## Master Lattice

See [99-master-lattice.md](./99-master-lattice.md) for the complete dependency matrix and a searchable index of all 20 missing integrations.

## Reading Guide

**For implementation**: Look up the pair file for the connection you're wiring. Each Tier 1 and Tier 2 missing-integration file includes a wiring recipe with estimated LOC.

**For architecture review**: The [00-overview.md](./00-overview.md) and [99-master-lattice.md](./99-master-lattice.md) provide the full picture.

**Cross-links with other analyses**:
- [../architectural-analysis/](../architectural-analysis/) — Findings about structural properties that constrain these connections
- [../readiness-audit/](../readiness-audit/) — Per-subsystem readiness that affects which connections are viable
