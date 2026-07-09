# Unified Specification — Index

> **Version**: 1.0 draft
> **Date**: 2026-04-25
> **Scope**: Complete behavioral specification for Roko, replacing `tmp/workflow/` (12 docs) and `tmp/architecture/` (21 docs) with a single unified vocabulary.

---

## Vocabulary

Roko is built from **3 fundamentals**, **9 protocols**, and **10 specializations**. A developer learns 12 core concepts; everything else is a discoverable specialization.

### 3 Fundamentals

| Primitive | What It Is |
|---|---|
| **Signal** | Universal data unit. Content-addressed, typed (Kind), scored, decaying, lineage-tracked, HDC-fingerprinted. Ephemeral variant lives on Bus; persisted variant lives in Store. |
| **Block** | Atomic computation. Takes Signals in, produces Signals out. Declares capabilities and protocol conformance. |
| **Graph** | Composition of Blocks wired by typed edges. TOML-defined, serializable. Runtime interprets it. |

### 9 Protocols (Block interfaces)

| Protocol | Verb | Existing Trait (roko-core) |
|---|---|---|
| `Store` | put / get / query / prune Signals | `Substrate` |
| `Score` | rate Signal along dimensions | `Scorer` |
| `Verify` | check Signal against truth → Verdict | `Gate` |
| `Route` | select among candidates, learn from outcome | `Router` |
| `Compose` | combine Signals under budget → one Signal | `Composer` |
| `React` | watch Signal stream, emit new Signals | `Policy` |
| `Observe` | read-only view, emit observation Signals | **New** (Lens) |
| `Connect` | connect / query / execute / disconnect external I/O | `Connector` |
| `Trigger` | listen for events, fire Graphs | **New** (Trigger protocol) |

### 10 Specializations (built on fundamentals)

| Name | Definition | Source |
|---|---|---|
| **Flow** | Graph at runtime (RunId, snapshot, events) | Workflow execution |
| **Rack** | Graph with exposed Macros (knobs) + Slots (jacks) | Parameterized Workflow |
| **Trigger** | Block with Trigger protocol | Trigger system |
| **Lens** | Block with Observe protocol | Telemetry (new) |
| **Loop** | Graph that feeds output back to input | DreamCycle, adaptive gates, cascade router |
| **Memory** | Store-protocol Block with decay + dreams | Knowledge store, neuro |
| **Space** | Graph isolation boundary + capability grants | Workspace |
| **Extension** | Block that intercepts another Block's pipeline | Extension trait (8 layers) |
| **Agent** | Space + Extensions + Memory + adaptive clock | AgentRuntime |
| **Connector** | Block with Connect protocol + lifecycle | Connector trait |

---

## Concept Migration Table

Every concept from the previous specs maps to the unified vocabulary. Nothing is lost.

### From Workflow Specs (tmp/workflow/)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Module | Block | Block is the generalization. Module was a Block implementing any protocol. |
| Workflow | Graph | A Graph is the static definition; a Flow is the runtime instance. |
| Artifact | Signal (persisted) | Content-addressed, versioned, lineage-tracked — same properties as Signal. |
| Macro | Macro (on Rack) | Unchanged concept, now scoped to the Rack specialization. |
| Slot | Slot (on Rack) | Unchanged concept, now scoped to the Rack specialization. |
| TypeSchema | TypeSchema (on Block) | Unchanged. Block I/O is typed via TypeSchema. |
| Capability | Capability (on Block) | Unchanged. Three-layer intersection model preserved. |
| Expr | Expr (on Graph edges) | Unchanged. Small, total, deterministic expression language. |
| Evidence | Signal (kind: Evidence) | Evidence is a Signal kind, not a separate primitive. |
| EvidenceBag | Signal set | A collection of evidence Signals flowing through a Graph. |
| Profile | Rack (verification) | A visual-gate2 Profile is a Rack whose Graph computes a Verdict. |
| Finding | Signal (kind: Finding) | Findings are Signals emitted by verification Blocks. |
| WorkflowEvent | Observable Event | Lifecycle events become ObservableEvents consumed by Lenses. |

### From Architecture Specs (tmp/architecture/)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Extension (22 hooks, 8 layers) | Extension specialization | Extension = Block that intercepts another Block's pipeline. 8 layers preserved. |
| Connector (5 methods) | Connector specialization | Block implementing Connect protocol with lifecycle. |
| Feed | Signal stream on Bus | A Feed is ephemeral Signals on a Bus topic. No separate primitive. |
| Recipe | Graph | A Recipe is a Graph of pure-transform Blocks. |
| Knowledge Entry | Signal (persisted, Memory) | Knowledge entries are Signals with decay, tiers, HDC fingerprints. |
| Pheromone | Signal (ephemeral, typed) | Pheromones are ephemeral Signals with location hash and intensity. |
| DreamCycle | Loop specialization | DreamCycle is a Graph that feeds output back to input on delta timescale. |
| MetaAgent | Agent specialization | MetaAgent is an Agent whose Space grants structural-change capabilities. |
| Generator | Block + Trigger | A Generator is a triggered Block that produces Signals on a schedule. |
| AgentRuntime | Agent specialization | Agent = Space + Extensions + Memory + adaptive clock. |
| AgentMode | Agent config field | Ephemeral / Persistent / Reactive remain as Agent modes. |
| CorticalState | Agent internal state | Working memory, goals, beliefs, attention — Agent internals. |
| DomainProfile | Agent config field | A string label mapping to default extension/tool sets. |
| TickPipeline | Agent's 9-step Graph | The 9-step pipeline is the Agent's internal Graph. |
| Relay | Connector (relay) | Relay becomes a built-in Connector for presence and messaging. |

### From roko-core (actual code)

| Old Concept | Unified Equivalent | Notes |
|---|---|---|
| Engram | Signal | Direct identity. Engram struct already has all Signal properties. |
| Pulse | Signal (ephemeral) | A Pulse is an ephemeral Signal on the Bus. |
| Substrate | Store protocol | Put / get / query / prune — identical semantics. |
| Scorer | Score protocol | Rate along dimensions — identical semantics. |
| Gate | Verify protocol | Check against truth → Verdict — identical semantics. |
| Router | Route protocol | Select among candidates — identical semantics. |
| Composer | Compose protocol | Combine under budget — identical semantics. |
| Policy | React protocol | Watch stream, emit interventions — identical semantics. |

---

## Design Principles

1. **Signal is the universal datum.** Everything that flows through the system is a Signal. No separate Artifact, Knowledge Entry, Pheromone, Evidence, or Feed types. Kind discriminates; decay models differ; storage varies (ephemeral on Bus, persisted in Store) — but the type is one.

2. **Block is the universal computation.** Every piece of work implements Block. Protocols declare what a Block can do. A scorer, a gate, a connector, an LLM call — all Blocks.

3. **Graph is the universal composition.** Workflows, pipelines, recipes, dream cycles, gate chains — all Graphs. TOML-authored, runtime-interpreted. No separate composition primitives.

4. **Protocols are interfaces, not implementations.** A Block declares which protocols it conforms to. The runtime dispatches based on protocol. Multiple protocols per Block are fine.

5. **Specializations are conventions, not new primitives.** An Agent is "Space + Extensions + Memory + clock" — it's a pattern of fundamentals, not a new fundamental. A Lens is "Block + Observe protocol" — discoverable, not magical.

6. **Telemetry is first-class.** Every Block, Graph, Agent, and Space can have Lenses attached. Observation never modifies what it observes. Lenses compose (stack, chain, scope).

7. **Learning loops use the same primitives.** A dream cycle is a Loop. Adaptive thresholds are Loops. Cascade routing is a Loop. The system learns using Graphs, not special machinery.

8. **Safety scales with autonomy.** Six levels from observe-only to structural changes. Each level has explicit bounds, rollback, and approval requirements. Parameters have declared ranges. Structural changes require human review.

9. **Decay is default.** Signals decay unless explicitly frozen. Knowledge starts Transient. Pheromones start with a 1-hour half-life. Freshness is earned through validation, not assumed.

10. **Wire, don't build.** The existing roko-core traits map 1:1 to protocols. Engram maps 1:1 to Signal. The unified spec renames and extends — it does not replace.

---

## Reading Order

### Essential (learn the type system)

1. **[01-SIGNAL.md](01-SIGNAL.md)** — The universal data unit
2. **[02-BLOCK.md](02-BLOCK.md)** — The universal computation + 9 protocols
3. **[03-GRAPH.md](03-GRAPH.md)** — The universal composition
4. **[04-SPECIALIZATIONS.md](04-SPECIALIZATIONS.md)** — 10 patterns built on fundamentals

### Core systems

5. **[05-EXECUTION-ENGINE.md](05-EXECUTION-ENGINE.md)** — State graph runtime, failure strategies, resumability
6. **[06-TRIGGER-SYSTEM.md](06-TRIGGER-SYSTEM.md)** — Event-driven Graph firing
7. **[07-AGENT-RUNTIME.md](07-AGENT-RUNTIME.md)** — 9-step pipeline, adaptive clock, T0/T1/T2
8. **[08-EXTENSION-SYSTEM.md](08-EXTENSION-SYSTEM.md)** — Interceptor Blocks across 8 layers

### New capabilities

9. **[09-TELEMETRY.md](09-TELEMETRY.md)** — Lens system, full observability
10. **[10-LEARNING-LOOPS.md](10-LEARNING-LOOPS.md)** — 4 cybernetic loops, autonomy levels

### Knowledge and connectivity

11. **[11-MEMORY-AND-KNOWLEDGE.md](11-MEMORY-AND-KNOWLEDGE.md)** — Store + decay + dreams + HDC
12. **[12-CONNECTIVITY.md](12-CONNECTIVITY.md)** — Connect protocol, relay, cross-agent comms

### Developer experience

13. **[13-BUILTIN-BLOCK-CATALOG.md](13-BUILTIN-BLOCK-CATALOG.md)** — All shipped Blocks by protocol
14. **[14-CONFIG-AND-AUTHORING.md](14-CONFIG-AND-AUTHORING.md)** — TOML schema, scripts, WASM, Rust
15. **[15-MARKETPLACE-AND-SHARING.md](15-MARKETPLACE-AND-SHARING.md)** — Publish, fork, trust, analytics
16. **[16-SURFACES.md](16-SURFACES.md)** — CLI, TUI, Dashboard, Visual Editor
17. **[17-SECURITY-MODEL.md](17-SECURITY-MODEL.md)** — Capability intersection, delegation, recursive safety

### Infrastructure

18. **[18-ON-CHAIN-REGISTRIES.md](18-ON-CHAIN-REGISTRIES.md)** — ERC-8004, InsightStore, PheromoneRegistry
19. **[19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md)** — Competitive environments, measurement
20. **[20-DEPLOYMENT.md](20-DEPLOYMENT.md)** — Railway, Fly, local dev
21. **[21-ROADMAP.md](21-ROADMAP.md)** — Phased implementation, crate mapping

---

## Naming Decisions

| Decision | Rationale |
|---|---|
| Signal (not Engram) | "Signal" is immediately meaningful. "Engram" is domain-specific jargon. The Rust struct remains `Engram` for backward compatibility; "Signal" is the spec-level name. |
| Block (not Module) | "Module" implies a Rust module or a large unit. "Block" implies composable, small, pluggable — like a Eurorack module or a Scratch block. |
| Graph (not Workflow) | "Workflow" implies business process automation. "Graph" is precise: nodes + edges. The runtime concept is a "Flow" (Graph at runtime). |
| Rack (not Parameterized Workflow) | From modular synthesis: a rack holds modules and exposes macro knobs and patch jacks (slots) to the performer. |
| Lens (not Monitor/Probe/Watcher) | A Lens focuses attention without modifying the subject. It's read-only by definition. Stacking Lenses gives you different views of the same thing. |
| Loop (not Feedback/Cycle) | Direct and unambiguous. A Loop is a Graph that feeds output back to input. |
| Space (not Workspace/Environment) | "Workspace" is overloaded (IDE, OS). "Space" is clean: an isolation boundary with capability grants. |

---

## Document Status

| # | Title | Status | Source |
|---|---|---|---|
| 00 | Index (this document) | Draft | New |
| 01 | Signal | Draft | roko-core Engram + arch-09 Knowledge/Pheromone + wf-02 Artifact |
| 02 | Block | Draft | roko-core 6 traits + wf-02 Module + arch-03 Extension + arch-04 Connector |
| 03 | Graph | Draft | wf-02 Workflow + wf-05 Engine |
| 04 | Specializations | Draft | New synthesis |
| 05 | Execution Engine | Draft | wf-05 refined |
| 06 | Trigger System | Draft | wf-03 (terminology update) |
| 07 | Agent Runtime | Draft | arch-02 restructured as Graph-based |
| 08 | Extension System | Draft | arch-03 restructured as Block-based |
| 09 | Telemetry: Lens System | Draft | **New** |
| 10 | Learning Loops | Draft | **New** |
| 11 | Memory and Knowledge | Draft | arch-09 restructured |
| 12 | Connectivity | Draft | arch-04 + arch-05 |
| 13 | Builtin Block Catalog | Draft | wf-06 expanded |
| 14 | Config and Authoring | Draft | wf-04 (terminology update) |
| 15 | Marketplace and Sharing | Draft | wf-12 + UsageLens |
| 16 | Surfaces | Draft | wf-08/09/10/11 combined |
| 17 | Security Model | Draft | New synthesis |
| 18 | On-Chain Registries | Draft | arch-14 (terminology update) |
| 19 | Arenas, Evals, Bounties | Draft | arch-11 (terminology update) |
| 20 | Deployment | Draft | arch-17 (terminology update) |
| 21 | Roadmap | Draft | arch-18 updated |
