---
title: Interactive Explorer
description: Interactive visual tools for navigating the Roko architecture, data flows, and crate dependencies.
outline: [2, 3]
---

# Interactive Explorer

Visual tools for navigating the Roko system. Each page below provides a
different lens into the architecture -- start with whichever matches how you
think about the system.

All explorer pages feature interactive visualizations built with Vue
components. The architecture explorer and data flow animator render as
interactive, zoomable, clickable graphics with detail panels. Mermaid
diagrams are retained as detailed reference and fallback for environments
without JavaScript.

---

## Available Tools

### [Architecture Explorer](./architecture)

An interactive crate dependency graph with zoom, pan, hover-highlighting,
and click-to-inspect detail panels. Each node shows a crate's LOC count,
test count, and tier. Click any node to see its dependencies and dependents,
and follow a link to its documentation chapter.

**Best for:** Understanding how the major pieces fit together, or quickly
navigating to a specific component's docs.

### [Data Flow Animator](./data-flow)

An interactive animated pipeline showing the 8-stage signal flow (Query,
Score, Route, Compose, Act, Verify, Write, React) with play/pause/step
controls and per-stage descriptions. Below the interactive pipeline, five
detailed Mermaid sequence diagrams walk through specific workflows:

| Scenario | What it shows |
|----------|--------------|
| Plan Execution | Idea through PRD, plan generation, graph execution, gates, learning |
| Agent Dispatch | Model routing, prompt composition, affect modulation, tool loop |
| Gate Validation | 7-rung pipeline, adaptive thresholds, failure-to-replan feedback |
| Knowledge Query | Retrieval, HDC similarity, tier progression, dream consolidation |
| Dream Consolidation | Episode batching, HDC clustering, distillation, playbook promotion |

**Best for:** Understanding how data moves through the system during
different operations.

### [Crate Map](./crate-map)

A comprehensive Mermaid dependency diagram of all 39 workspace crates plus 3
apps. Color-coded by tier, with clickable links to the relevant documentation
chapter. Shows the actual `Cargo.toml` dependency edges, not just conceptual
relationships.

**Best for:** Understanding compile-time dependencies, finding which crate
owns a particular capability, or planning where to add new functionality.

---

## How to Read These Diagrams

**Tiers** group crates by abstraction level. Lower tiers are more foundational;
higher tiers are more user-facing. The tier numbering matches the
[Architecture Guide](/35-ARCHITECTURE#7-crate-map).

**Color coding** is consistent across all explorer pages:

| Color | Tier | Role |
|-------|------|------|
| Rose | T1: Core Kernel | Types, traits, storage primitives |
| Blue | T2: Execution Engine | Graph DAG, runtime services, process supervision |
| Orange | T3: Agent and Composition | LLM dispatch, prompt assembly, gate verification |
| Purple | T4: Learning and Memory | Episodes, knowledge, dreams, affect |
| Green | T5: External Interfaces | CLI, HTTP server, ACP, sidecar |
| Teal | T6: Code Intelligence | Parsing, symbol graphs, language providers |
| Amber | T7: MCP, Plugin, Gateway | Tool ecosystem, inference pipeline |
| Brown | T8: Chain and Economy | On-chain primitives, local state machines |
| Slate | T9: Apps and Tests | Standalone binaries, integration tests |

---

## Related

- [Architecture Guide](/35-ARCHITECTURE) -- the narrative companion to these visual tools
- [Master Index](/00-INDEX) -- full specification index
- [Crate Map and Dependencies](/depth/00-architecture/crate-map-and-dependencies) -- depth file with textual dependency analysis
