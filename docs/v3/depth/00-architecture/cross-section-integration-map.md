# Cross-Section Integration Map

> **v3 depth file** -- `/docs/v3/depth/00-architecture/cross-section-integration-map.md`
> Canonical source: v1 `docs/v1/00-architecture/24-cross-section-integration-map.md`
> Status: **Current** (48/48 epics accepted, all subsystems wired, ~376 canonical routes)

---

## 1. Purpose

This document is the architectural X-ray of Roko. It shows how the system's subsystems
actually connect, where they should connect but do not, and which connections produce the
highest leverage. The earlier engine/event-bus proposal documented here has been realized:
`EventBus<E>` is the live transport surface, `Bus` is a kernel trait, and `StateHub` provides
typed projection between transport + storage and consumer surfaces.

The v1 version of this document identified 20 missing wiring points. As of the 2026-09-15
state, the majority have been resolved through the 48 accepted epics and the engine
convergence that made Graph the sole executor.

---

## 2. Section Dependency Matrix

Each row shows what a section provides to the column section. Empty cells indicate no
direct dependency or a coupling that the architecture intentionally avoids.

### 2.1 Active Couplings (Wired)

| Provider \ Consumer | Core | Agent | Gate | Compose | Learn | Neuro | Conductor | Graph | Serve | CLI |
|---|---|---|---|---|---|---|---|---|---|---|
| **Core** | -- | Signal types | Verdict types | Context, Score | Episode | Knowledge types | Config | Cell primitives | Route types | Config |
| **Agent** | -- | -- | -- | Provider dispatch | Episode events | -- | Health events | Provider cells | Dispatch routes | Agent commands |
| **Gate** | Verdict | Feedback | -- | Gate enrichment | Threshold data | -- | Circuit events | Verify cells | Gate routes | Gate commands |
| **Compose** | -- | System prompts | -- | -- | -- | Context query | -- | Compose cells | -- | -- |
| **Learn** | -- | Router selection | Adaptive thresholds | Playbook injection | -- | Tier progression | -- | -- | Learn routes | Learn commands |
| **Neuro** | -- | Knowledge context | -- | Retrieval | Knowledge stats | -- | -- | -- | Neuro routes | Knowledge commands |
| **Conductor** | -- | Circuit breaker | -- | -- | -- | -- | -- | -- | -- | Dashboard events |
| **Graph** | -- | -- | -- | -- | -- | -- | -- | -- | Execution routes | Plan execution |
| **Serve** | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| **CLI** | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- |

### 2.2 Additional Cross-Crate Dependencies

Beyond the primary matrix, several cross-cutting concerns create additional edges:

| Dependency | From | To | Nature |
|---|---|---|---|
| Safety layer | `roko-agent` safety/ | `roko-core` types | Trust-origin IFC, taint tracking |
| Plugin ecosystem | `roko-plugin` | `roko-core`, `roko-agent`, `roko-gate` | Signed dependency graphs, WASM hooks |
| ACP protocol | `roko-acp` | `roko-agent`, `roko-core`, `roko-learn` | External agent integration |
| Sidecar | `roko-agent-server` | `roko-agent`, `roko-core` | Per-agent HTTP |
| MCP code | `roko-mcp-code` | `roko-index` | Code intelligence |
| Execution facade | `roko-execution` | `roko-core`, `roko-agent`, `roko-graph` | RuntimeServices builder |
| Runtime | `roko-runtime` | `roko-core` | ProcessSupervisor, event bus |
| Primitives | `roko-primitives` | `roko-core` | HDC vectors, tier routing |
| Dreams | `roko-dreams` | `roko-neuro`, `roko-daimon`, `roko-fs` | Offline consolidation |
| Daimon | `roko-daimon` | `roko-core`, `roko-learn` | Affect engine, GoalTree |
| Chain | `roko-chain` | `roko-core` | Optional chain primitives |
| Filesystem | `roko-fs` | `roko-core` | JSONL substrate |

---

## 3. Data Flow Through the System

### 3.1 Signal Lifecycle

```
User input / PRD / CLI command
    │
    ▼
┌──────────┐    encode     ┌──────────┐    store     ┌──────────┐
│  Ingress │──────────────▶│  Signal  │─────────────▶│Substrate │
│ (CLI/API)│               │(Engram)  │              │ (JSONL)  │
└──────────┘               └────┬─────┘              └──────────┘
                                │ compose
                                ▼
                         ┌──────────────┐
                         │   Composer   │
                         │(9-layer SPB) │
                         └──────┬───────┘
                                │ dispatch
                                ▼
                         ┌──────────────┐
                         │    Agent     │
                         │(12 providers)│
                         └──────┬───────┘
                                │ verify
                                ▼
                         ┌──────────────┐
                         │  Gate (19)   │
                         │ 7-rung pipe  │
                         └──────┬───────┘
                                │ persist + learn
                          ┌─────┴──────┐
                          ▼            ▼
                   ┌──────────┐ ┌──────────┐
                   │Substrate │ │  Learn   │
                   │  write   │ │(episodes,│
                   └──────────┘ │playbooks)│
                                └──────────┘
```

### 3.2 Plan Execution Flow

The Graph engine is the sole executor (since #260, confirmed by #276 WorkflowEngine
retirement). Runner-v2 is retained as `--engine legacy` for one release cycle.

```
roko plan run <dir>
    │
    ▼
┌───────────────────────┐
│ WorkflowGraphController│  ← Graph templates
│ (roko-graph)           │
└───────────┬───────────┘
            │ topology
            ▼
┌───────────────────────┐
│ ProductionPlanTopology │  ← Cell DAG
│ parallel waves, cond. │
│ routing, cost enforce │
└───────────┬───────────┘
            │ per-cell
            ▼
┌───────────────────────┐
│ Cell execution         │
│ - 7 cognitive cells    │
│ - 5 verify cells       │
│ - immune decision graph│
│ - GuaranteedFinally    │
└───────────┬───────────┘
            │ checkpoint
            ▼
┌───────────────────────┐
│ Restart-durable state  │
│ Hot tick/output/budget │
│ Graph-fingerprinted   │
│ Activity resume        │
└───────────────────────┘
```

---

## 4. Kernel Protocol Integrations

### 4.1 The 12 Kernel Traits

Roko's kernel exposes 12 traits. Every subsystem speaks through these contracts:

| Trait | Primary Crate | Consumers |
|---|---|---|
| `Store` | `roko-fs` | All persistence paths |
| `ColdStore` | `roko-fs` | Archival subsystem |
| `Score` | `roko-core` | Composer, Router, Auction |
| `Verify` | `roko-gate` | Gate pipeline, immune cells |
| `Route` | `roko-learn` | CascadeRouter, model selection |
| `Compose` | `roko-compose` | SystemPromptBuilder |
| `React` | `roko-conductor` | Policy watchers |
| `Bus` | `roko-runtime` | EventBus transport |
| `Observe` | `roko-serve` | Telemetry, Lens executors |
| `Connect` | `roko-agent` | Provider adapters |
| `Trigger` | `roko-graph` | Declarative trigger runtime |
| `Substrate` | `roko-fs` | Storage fabric |

### 4.2 Bus Topic Structure

The `Bus` trait provides typed publish-subscribe. Key topic namespaces:

| Namespace | Publisher | Subscribers | Content |
|---|---|---|---|
| `plan.*` | Graph executor | TUI, serve, learn | Plan/task lifecycle events |
| `gate.*` | Gate pipeline | Learn, conductor | Verdicts, thresholds |
| `agent.*` | Agent dispatch | Conductor, learn | Provider health, cost |
| `safety.*` | Immune system | Dashboard, audit | Taint, quarantine, incidents |
| `knowledge.*` | Neuro store | Composer, dreams | Tier promotions, GC |
| `trigger.*` | Trigger runtime | Graph cells | Seven trigger sources |
| `lens.*` | Telemetry lens | SSE, dashboard | 39 production event variants |
| `feed.*` | Feed runtime | Recipe evaluator | Live data feeds |

---

## 5. HTTP Control Plane Integration

The serve crate exposes ~376 canonical REST routes (~421 including aliases) organized
across the full subsystem matrix:

| Route Group | Count | Subsystem Integration |
|---|---|---|
| Plan lifecycle | ~40 | Graph executor, CLI plan commands |
| Agent management | ~30 | Agent dispatch, sidecar proxy |
| Knowledge/neuro | ~25 | Neuro store, tier progression |
| Learning | ~20 | Episodes, playbooks, experiments |
| Gate/verification | ~15 | Gate pipeline, adaptive thresholds |
| Safety/immune | ~15 | Taint, quarantine, incident |
| Telemetry/lens | ~20 | 11 built-in Lens executors |
| Feeds/recipes | ~15 | Feed runtime, recipe evaluation |
| Triggers | ~10 | Declarative trigger management |
| Named surfaces | ~25 | Workbench, Inbox, Canvas, Minimap, Autonomy |
| Groups | ~15 | Agent groups, coordination |
| Marketplace | ~20 | Artifact marketplace stubs |
| Identity/registry | ~15 | Local passport, knowledge routes |
| Arena/eval | ~15 | Arena lifecycle, leaderboard |
| Payments | ~10 | x402, MPP sessions |
| Config/system | ~20 | Config evolution, health, SSE |
| GitHub | ~10 | Webhook triggers, PR/CI state |
| Inference gateway | ~15 | Nine-stage routing, caching |
| DeFi products | ~10 | Structured 501 routes |
| Connectivity | ~10 | Relay, envelope delivery |
| Meta-agent | ~10 | Lineage, proposal lifecycle |

---

## 6. Integration Gaps (Current)

### 6.1 Resolved Since v1

The following integration points identified in the v1 audit have been wired:

| Gap | Resolution | Epic |
|---|---|---|
| ToolDispatcher not wired | Safety pipeline active on all provider paths | E34 |
| roko-index no consumer | MCP code intelligence wired | E32 |
| PAD persistence resets | DaimonState persisted, affect-energy coupling live | E23 |
| CascadeRouter not Daimon-aware | Behavioral state biases tier selection | E25 |
| No bus trait | `Bus` is kernel trait, `EventBus<E>` is transport | Core |
| No StateHub | Typed projection layer with SSE/REST | E33 |
| Safety guards dormant | Trust-origin IFC, 5-head corrigibility | E34 |
| Process supervision | ProcessSupervisor tracks agents via roko-runtime | Core |
| MCP not wired | CLI passthrough + HTTP provider resolution | E32 |

### 6.2 Remaining Product Gaps

| Gap | Nature | Tracked In |
|---|---|---|
| Provider-owned internal trace Signals | Product residual | E34 scope boundary |
| Adaptive semantic immune memory | Broader product scope | E34 residual |
| Native Agent-to-E33 observation publication | Integration scope | E23/E33 boundary |
| Full named-surface TUI rendering | Product residual | E37 residual |
| WIT/Component Store/Bus hostcalls | Plugin roadmap | E32 residual |
| Durable storage for marketplace | Product work | E38 residual |
| Deployed contracts/consensus | daeji scope | E39+ |
| Background/WebSocket indexing | Product work | E39 residual |
| force_backend override learning | UX34 | GAPS.md |

---

## 7. Cross-Cut Integration Patterns

### 7.1 Shared Service Facade

`roko-execution` provides the `RuntimeServices` builder (#243), giving CLI, serve, and ACP
a common facade for:
- Provider selection and dispatch
- Gate pipeline construction
- Learning subsystem access
- Configuration resolution

### 7.2 Config Evolution

Config evolution (E42) provides the integration discipline for all cross-section config:
- Priority/provenance hierarchy
- Seven invariants enforced across merge
- Transactional reload with freshness diagnostics
- Profile-aware secrets management

### 7.3 Safety as Cross-Cut

The E34 safety layer is not a separate subsystem. It injects into existing crate boundaries:
- `roko-agent`: Trust-origin IFC, taint propagation
- `roko-graph`: 5-stage immune decision Graph
- `roko-core`: Capability wrappers, workspace-rooted authority
- `roko-runtime`: Provider isolation, tool cooldown
- All production hooks: Mandatory audited wiring

---

## 8. Integration Health Metrics

The following metrics characterize the integration state of the system:

| Metric | Value | Source |
|---|---|---|
| Workspace members | 39 | Cargo.toml |
| LOC (approximate) | ~1M | Workspace |
| Tests | 10,300+ | cargo test --workspace |
| Accepted epics | 48/48 | Epic manifest |
| Executable tasks | 124/124 | Plan queue |
| Canonical HTTP routes | ~376 | route inventory |
| Kernel traits | 12 | roko-core |
| Agent providers | 12 | roko-agent |
| Gate types | 19 | roko-gate |
| Production Lens variants | 39 | E33 |
| Trigger sources | 7 | E31 |
| Named surfaces | 5 | E37 |

---

## Cross-References

- [Synergy Integration Map](./synergy-integration-map.md) -- primitive-level synergy matrix
- [Implementation Readiness Audit](./implementation-readiness-audit.md) -- per-section readiness
- [Comprehensive Test Strategy](./comprehensive-test-strategy.md) -- cross-crate test matrix
- v3 `00-INDEX.md` Section 0 -- Architecture overview
