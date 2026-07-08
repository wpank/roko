---
title: "Integration Map: Overview"
section: analysis
subsection: integration-map
id: im-00
source: 24-cross-section-integration-map.md (§1-3, §9)
tags: [overview, dependency-matrix, section-inventory, wired-connections]
---

# Integration Map: Overview

> **Source**: 24-cross-section-integration-map.md  
> **Date**: 2026-04-13  
> **Scope**: All pairwise relationships between 22 documentation sections; 20 missing integrations identified; ~2,070 LOC estimated to close all gaps.

## Section Inventory

The 22 sections map to architectural concerns and implementation layers:

| # | Section | Primary Crate | Layer |
|---|---|---|---|
| 00 | Architecture | `roko-core` | All |
| 01 | Orchestration | `roko-orchestrator` | L4 |
| 02 | Agents | `roko-agent` | L1 |
| 03 | Composition | `roko-compose` | L2 |
| 04 | Verification | `roko-gate` | L3 |
| 05 | Learning | `roko-learn` | Cross-cut |
| 06 | Neuro | `roko-neuro` | Cross-cut |
| 07 | Conductor | `roko-conductor` | L3 |
| 08 | Chain | `roko-chain` | Domain plugin |
| 09 | Daimon | `roko-daimon` | Cross-cut |
| 10 | Dreams | `roko-dreams` | Cross-cut |
| 11 | Safety | `roko-agent::safety` | Cross-cut |
| 12 | Interfaces | `roko-cli`, `roko-serve` | L4 |
| 13 | Coordination | (trait impls) | Cross-cut |
| 14 | Identity/Economy | `roko-chain` ext | Domain plugin |
| 15 | Code Intelligence | `roko-index` | L2 |
| 16 | Heartbeat | (orchestrate.rs) | L0/L1 |
| 17 | Lifecycle | (CLI subcommands) | L4 |
| 18 | Tools | `roko-std` | L1 |
| 19 | Deployment | (build/ops) | Infrastructure |
| 20 | Technical Analysis | `roko-oracle` (planned) | L2/L3 |
| 21 | References | (docs only) | Documentation |

---

## Dependency Matrix Legend

Each cell encodes relationship type(s) from the **row section** to the **column section**:
- **D** = Data flow (Engrams flow from row → column)
- **T** = Trait usage (row implements traits column consumes)
- **C** = Configuration (row's parameters affect column's behavior)
- **I** = Integration point (currently wired)
- **M** = Missing integration (should exist but doesn't)
- **—** = No significant dependency

---

## Currently Wired Primary Data Flows

The core execution path that ships today:

```
CLI[12] → commands → Orchestrator[01]
Orchestrator[01] → task dispatch → Agents[02]
Orchestrator[01] → role spec → Composition[03]
Composition[03] → system prompt → Agents[02]
Agents[02] → output → Verification[04]
Agents[02] → tool calls → Tools[18]
Verification[04] → verdict → Orchestrator[01]
Verification[04] → verdict + stats → Learning[05]
Learning[05] → cascade routing → Agents[02]
Orchestrator[01] → load signal → Conductor[07]
Conductor[07] → circuit state → Orchestrator[01]
Learning[05] → episodes → CLI[12]
```

## Cross-Cut Wiring Status

| Cross-Cut | Wired to | Status |
|---|---|---|
| Neuro (06) | Composition (03) | **Partial** — Insights only; Warnings, CausalLinks, AntiKnowledge missing (M5, M15) |
| Neuro (06) | Verification (04) | **Missing** — knowledge-informed thresholds (M14) |
| Daimon (09) | Router via CascadeRouter | **Wired** — PAD vector biases model-tier selection |
| Daimon (09) | Composition (03) | **Wired** — affect biases context weights |
| Daimon (09) | Orchestration (01) | **Missing** — PAD-modulated scheduling (M1) |
| Dreams (10) | Neuro (06) | **Missing** — consolidated knowledge (M7) |
| Dreams (10) | Daimon (09) | **Missing** — depotentiation (M18) |

---

## The 20 Missing Integrations at a Glance

| # | Name | From→To | Tier | LOC |
|---|---|---|---|---|
| M1 | Daimon → Orchestration | 09→01 | 1 | ~60 |
| M2 | Daimon → Composition | 09→03 | 1 | ~45 |
| M3 | Failure → Replanning | 04→01 | 1 | ~80 |
| M4 | Skills → Prompts | 05→03 | 1 | ~55 |
| M5 | Neuro → Composition (full) | 06→03 | 1 | ~90 |
| M6 | Cost → Routing | 05→05 | 1 | ~70 |
| M7 | Dreams → Neuro | 10→06 | 2 | ~200 |
| M8 | Code Intel → Composition | 15→03 | 2 | ~120 |
| M9 | Conductor → Routing (direct) | 07→05 | 2 | ~45 |
| M10 | Experiments → Static | 05→00 | 2 | ~90 |
| M11 | Orchestration → Daimon | 01→09 | 2 | ~40 |
| M12 | Pheromones → Orchestration | 13→01 | 3 | ~150 |
| M13 | Safety → Composition | 11→03 | 3 | ~80 |
| M14 | Neuro → Gate Thresholds | 06→04 | 3 | ~60 |
| M15 | AntiKnowledge → Composition | 06→03 | 2 | ~35 |
| M16 | Code Intel → Verification | 15→04 | 3 | ~100 |
| M17 | Tech Analysis → Heartbeat | 20→16 | 3 | ~150 |
| M18 | Dreams → Daimon | 10→09 | 3 | ~80 |
| M19 | Coordination → Dreams | 13→10 | 4 | ~120 |
| M20 | Lifecycle → Neuro (restore) | 17→06 | 3 | ~100 |

**Grand total**: ~2,070 LOC ≈ 1.2% of current codebase (~177K LOC)

---

## Integration Priority Roadmap

### Tier 1 — Self-Hosting Critical (~310 LOC)
Enables autonomous operation:
- [M3: verification-x-orchestration](./verification-x-orchestration.md) — failure→replanning
- [M4: learning-x-composition](./learning-x-composition.md) — skills→prompts
- [M6: learning-x-routing](./learning-x-routing.md) — cost→routing
- [M1: daimon-x-orchestration](./daimon-x-orchestration.md) — PAD-modulated scheduling
- [M2: daimon-x-composition](./daimon-x-composition.md) — affect-modulated context

### Tier 2 — Cognitive Integration (~620 LOC)
Enables self-improvement:
- [M5: neuro-x-composition](./neuro-x-composition.md) — full knowledge injection
- [M15: anti-knowledge-x-composition](./anti-knowledge-x-composition.md) — anti-knowledge (part of M5)
- [M8: code-intel-x-composition](./code-intel-x-composition.md) — code-aware context
- [M9: conductor-x-routing](./conductor-x-routing.md) — load→routing
- [M10: learning-x-config](./learning-x-config.md) — experiments→static config
- [M11: orchestration-x-daimon](./orchestration-x-daimon.md) — outcomes→affect

Plus Event Bus infrastructure (~200 LOC) that enables all future M-items cheaply.

### Tier 3 — Full Cognitive Loop (~620 LOC)
Enables Dreams and meta-cognition:
- [M7: dreams-x-neuro](./dreams-x-neuro.md)
- [M18: dreams-x-daimon](./dreams-x-daimon.md)
- [M14: neuro-x-verification](./neuro-x-verification.md)
- [M13: safety-x-composition](./safety-x-composition.md)
- [M16: code-intel-x-verification](./code-intel-x-verification.md)
- [M20: lifecycle-x-neuro](./lifecycle-x-neuro.md)

### Tier 4 — Collective Intelligence (~520 LOC)
Enables multi-agent coordination:
- [M12: coordination-x-orchestration](./coordination-x-orchestration.md)
- [M19: coordination-x-dreams](./coordination-x-dreams.md)
- [M17: tech-analysis-x-heartbeat](./tech-analysis-x-heartbeat.md)

---

## Proposed New Connections (Beyond M1-M20)

### Event Bus as Universal Dependency Inverter
Replace direct function calls in `orchestrate.rs` with typed topic channels. Each missing integration (M1-M20) becomes ~20-40 LOC instead of 40-200 LOC. Estimated infrastructure cost: ~200 LOC. See [99-master-lattice.md](./99-master-lattice.md).

### Four-Dimensional Provenance
Tag cross-section Engram flows with dimension (Dataflow / ControlFlow / Telemetry / Scheduling) for debuggability. Extends `Provenance` struct in `roko-core`. Backward compatible.

### Dreams as Off-Loop Projection Builder
Treat Dreams as an event-log projection with cursor-based catch-up. Closes M7 and enables crash recovery. See [dreams-x-neuro.md](./dreams-x-neuro.md).

### Compiled Dependency Graph
Express the cross-section dependency graph as a compile-time `CrateManifest` artifact, validated at build time.

---

## Cross-References
- [../architectural-analysis/](../architectural-analysis/) — Architectural findings that constrain these connections
- [../readiness-audit/](../readiness-audit/) — Per-subsystem readiness affecting viability of each connection
- [../synergy-map/](../synergy-map/) — Primitive-level synergies that provide further motivation for some pairs
- [99-master-lattice.md](./99-master-lattice.md) — Searchable index of all pairs
