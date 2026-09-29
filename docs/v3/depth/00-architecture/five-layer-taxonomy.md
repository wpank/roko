# The Five-Layer Taxonomy

> **v3 depth file** -- `/docs/v3/depth/00-architecture/five-layer-taxonomy.md`
> Canonical source: v1 `docs/v1/00-architecture/12-five-layer-taxonomy.md`
> Status: **Shipping** -- 39 workspace members organized into five layers plus cognitive
> cross-cuts. Strict downward dependencies enforced. Graph is the sole execution engine.

---

## 1. The Layer Diagram

```
+------------------------------------------------------+
|                   Applications                        |
|  (coding agent, chain agent, research agent, custom)  |
+------------------------------------------------------+
|  Layer 4: ORCHESTRATION                               |
|  DAGs, scheduling, state machines, multi-agent coord  |
+------------------------------------------------------+
|  Layer 3: HARNESS                                     |
|  Gates, conductor, monitoring, interventions, eval    |
+------------------------------------------------------+
|  Layer 2: SCAFFOLD                                    |
|  Context engineering, prompts, enrichment, memory     |
+------------------------------------------------------+
|  Layer 1: FRAMEWORK                                   |
|  Connections, roles, tools, model routing, safety     |
+------------------------------------------------------+
|  Layer 0: RUNTIME / KERNEL                            |
|  Process lifecycle, Store, Bus, HDC, I/O, clock       |
+------------------------------------------------------+

  COGNITIVE CROSS-CUTS (injected into multiple layers):
  Neuro (knowledge) | Daimon (motivation) | Dreams (offline learning)
  + Inference Optimization | Safety & Provenance | Observability & Telemetry
```

**Dependencies flow STRICTLY downward.** Layer 4 may depend on Layer 3, never the reverse.
Cross-cutting concerns are injected via trait objects, never via direct imports of higher
layers. Higher-layer communication flows through Store and/or Bus, not through direct crate
coupling.

---

## 2. Layer 0: Runtime / Kernel

**Purpose**: Process lifecycle, the two-fabric kernel surface (Store + Bus), supervision,
cancellation, I/O, adaptive clock.

**Key Crates**:
- `roko-core` -- kernel types: Signal (Engram), Score, Decay, all 12 traits, Query, Context,
  Budget, Kind, ContentHash, Provenance, Taint, Datum, Pulse, Topic, TopicFilter, Verdict
- `roko-primitives` -- HDC vectors (HdcVector), Hamming similarity, inference tiers
- `roko-runtime` -- ProcessSupervisor, cancellation tokens, EventBus, PulseBus, adaptive clock
- `roko-fs` -- FileSubstrate (JSONL persistence), ArchiveColdSubstrate, GC, file layout

**What Lives Here**:
- Process spawning and lifecycle management (`ProcessSupervisor`)
- Store trait for durable Signal persistence and query
- ColdStore trait for archival storage
- Bus trait for topic-addressed Pulse delivery
- Topic and TopicFilter as routing handles
- HDC vectors and similarity for durable memory search
- Cancellation tokens and graceful shutdown
- The adaptive clock managing Gamma/Theta/Delta frequencies
- Basic I/O primitives

**Kernel Traits at L0**: Store, ColdStore, Bus, Substrate (alias)

**Beer VSM Mapping**: System 1 (Operations) -- the primary activities of the organization.

---

## 3. Layer 1: Framework

**Purpose**: Connections to external systems (LLMs, tools, MCP), roles, model routing, safety.

**Key Crates**:
- `roko-agent` -- 12 LLM provider kinds, connection pools, MCP client, tool dispatch loop,
  safety layer (role auth, pre/post-execution checks, AgentContract, AgentWarrant)
- `roko-std` -- 35 default tool definitions (16 executable local + 19 GitHub MCP), default
  trait implementations

**What Lives Here**:
- LLM backend connections (Anthropic, Claude CLI, Codex CLI, OpenAI-compat, Cursor ACP/CLI,
  Perplexity, Gemini API/CLI, Cerebras, Hermes, OpenClaw)
- Tool registry and tool dispatch
- MCP (Model Context Protocol) client for external tool integration
- Model routing logic (CascadeRouter decisions propagate here)
- Safety layer (trust-origin IFC, five-head corrigibility, sandbox policy, capability wrappers)
- Agent lifecycle management

**Kernel Traits at L1**: Route (model/tool selection), Score (tool relevance)

**Beer VSM Mapping**: System 2 (Coordination) -- anti-oscillation, ensuring components work
together without conflict.

---

## 4. Layer 2: Scaffold

**Purpose**: Context engineering, prompt assembly, enrichment, memory access.

**Key Crates**:
- `roko-compose` -- SystemPromptBuilder (9-layer prompt assembly with 11 role templates),
  prompt templates, context enrichment, token budget management

**What Lives Here**:
- SystemPromptBuilder with `RoleSystemPromptSpec` for 9-layer prompts
- Prompt templates for different agent roles (coder, researcher, planner, reviewer, etc.)
- Context enrichment (injecting relevant knowledge, history, and tool descriptions)
- Token budget management within prompts
- Attention bidding (Neuro/Task/Research context bidders)

**Kernel Traits at L2**: Score (relevance for context selection), Compose (prompt assembly)

**Beer VSM Mapping**: System 3 (Control) -- resource allocation and internal management.

---

## 5. Layer 3: Harness

**Purpose**: Verification, monitoring, interventions, evaluation.

**Key Crates**:
- `roko-gate` -- 19 verification gates, 7-rung pipeline, adaptive thresholds (EMA-based)

**What Lives Here**:
- Gate pipeline (compile, test, clippy, diff, format, schema, judge, simulation)
- Adaptive gate thresholds persisted in `.roko/learn/gate-thresholds.json`
- Monitoring and health checks
- Conductor watchers and circuit breakers

**Kernel Traits at L3**: Verify (gate pipeline), React (conductor watchers, circuit breakers)

**Beer VSM Mapping**: System 3* (Audit) -- monitoring and verification of operations.

---

## 6. Layer 4: Orchestration

**Purpose**: Plan DAGs, parallel execution, state machines, multi-agent coordination.

**Key Crates**:
- `roko-graph` -- Sole execution engine (DAG of Cells, bounded parallel waves, conditional
  routing, cost enforcement, resume-durable checkpoints)
- `roko-conductor` -- Reactive watchers, circuit breakers, health monitoring
- `roko-execution` -- RuntimeServices builder, diagnostic service, execution control

**What Lives Here**:
- Plan discovery and DAG construction
- Graph engine with Cell-based execution (7 cognitive Cells, 5 Verify Cells)
- Parallel task execution with dependency ordering and merge queue
- State machine for plan phases (Pending -> Running -> Gated -> Complete)
- Session persistence and resumption (`.roko/state/graph/` checkpoints)
- Worktree management for parallel code modifications
- ProcessSupervisor tracks and shuts down agents

**Kernel Traits at L4**: React (state machine transitions, plan reactions)

**Beer VSM Mapping**: System 4 (Intelligence) -- environmental scanning and adaptation.

---

## 7. Cognitive Cross-Cuts

Cross-cutting concerns are injected across multiple layers rather than living at any single
level. They are provided as trait objects, never as direct imports:

| Cross-Cut | Crate | Injected Into | Role |
|---|---|---|---|
| **Neuro** | `roko-neuro` | L2 (context), L3 (knowledge gates), L4 (planning) | Knowledge management, tier decay, distillation |
| **Daimon** | `roko-daimon` | L0 (clock), L1 (routing), L2 (context bidding) | Motivation, PAD vector, behavioral states, energy accounting |
| **Dreams** | `roko-dreams` | L0 (scheduling), Neuro (consolidation) | Offline learning, NREM replay, REM imagination |
| **Learning** | `roko-learn` | L1 (routing), L3 (gate thresholds), L4 (plan adaptation) | Episodes, playbooks, bandits, experiments, efficiency |
| **Safety** | `roko-agent/safety` | L1 (dispatch), L3 (gates) | Role auth, taint tracking, immune Graph |
| **Observability** | `roko-core/obs` | All layers | Metrics, tracing, telemetry |

---

## 8. Trait x Layer Map

| Trait | L0 Runtime | L1 Framework | L2 Scaffold | L3 Harness | L4 Orchestration |
|---|---|---|---|---|---|
| **Store** | MemorySubstrate, FileSubstrate | -- | -- | -- | -- |
| **ColdStore** | ArchiveColdSubstrate | -- | -- | -- | -- |
| **Bus** | PulseBus | -- | -- | -- | -- |
| **Score** | -- | ToolRelevanceScorer | RelevanceScorer, RecencyScorer | -- | -- |
| **Verify** | -- | -- | -- | CompileGate, TestGate, etc. | -- |
| **Route** | -- | CascadeRouter, LinUCBRouter | -- | -- | -- |
| **Compose** | -- | -- | SystemPromptBuilder, ContextComposer | -- | PlanComposer |
| **React** | -- | -- | -- | CircuitBreakerPolicy | PlanPhasePolicy |
| **Observe** | -- | Feed observers | -- | -- | -- |
| **Connect** | -- | Provider connections | -- | -- | -- |
| **Trigger** | -- | 7 trigger sources | -- | -- | -- |

---

## 9. Dependency Rules

### 9.1 Strict Downward Dependencies

```
L4 depends on -> L3, L2, L1, L0
L3 depends on -> L2, L1, L0
L2 depends on -> L1, L0
L1 depends on -> L0
L0 depends on -> (nothing above)
```

### 9.2 Cross-Cut Injection

Cross-cutting crates are NOT layer-bound. They are injected as `&dyn Trait` objects:

```rust
fn compose_with_knowledge(
    composer: &dyn Compose,
    knowledge: &dyn Store,
    bus: &dyn Bus,
    budget: &Budget,
    scorer: &dyn Score,
    ctx: &Context,
) -> Result<Signal> {
    let knowledge_signals = knowledge
        .query(&Query::of_kind(Kind::Insight).limit(5), ctx)
        .await?;
    composer.compose(&knowledge_signals, budget, scorer, ctx)
}
```

### 9.3 Why This Matters

Strict layering prevents circular dependencies, ensures each layer can be tested
independently, and allows layer-level replacement. When a higher layer needs durable state
or live coordination, it talks through Store and/or Bus, not by importing peer or lower-layer
crates directly.

---

## 10. The Complete Crate Map by Layer

| Layer | Crate | Status | Purpose |
|---|---|---|---|
| **L0** | `roko-core` | Shipping | Signal, 12 kernel traits, Score, Decay, Query, Context |
| **L0** | `roko-primitives` | Shipping | HDC vectors, Hamming similarity, shared types |
| **L0** | `roko-runtime` | Shipping | ProcessSupervisor, cancellation, EventBus, PulseBus |
| **L0** | `roko-fs` | Shipping | FileSubstrate, ArchiveColdSubstrate, GC, layout |
| **L1** | `roko-agent` | Shipping | 12 LLM providers, tool dispatch, MCP, safety |
| **L1** | `roko-std` | Shipping | Default trait impls, 35+ tool definitions |
| **L1** | `roko-plugin` | Shipping | Plugin SDK, signed dependencies, WASM hooks |
| **L2** | `roko-compose` | Shipping | Prompt assembly, 11 role templates |
| **L3** | `roko-gate` | Shipping | 19 gates, 7-rung pipeline |
| **L4** | `roko-graph` | Shipping | Sole execution engine, DAG cells, cost state |
| **L4** | `roko-conductor` | Shipping | Watchers, circuit breaker, diagnosis |
| **L4** | `roko-execution` | Shipping | RuntimeServices builder, diagnostics |
| **Cognitive** | `roko-learn` | Shipping | Episodes, playbooks, bandits, experiments |
| **Cognitive** | `roko-neuro` | Shipping | Knowledge store, tier progression, HDC |
| **Cognitive** | `roko-daimon` | Shipping | Affect/motivation (PAD vectors) |
| **Cognitive** | `roko-dreams` | Shipping | Offline consolidation, daemon scheduling |
| **Chain** | `roko-chain` | Shipping | Local registry, marketplace, arena, DeFi stubs |
| **Plugin** | `roko-index` | Shipping | Code parsing, symbol graphs, HDC fingerprints |
| **Lang** | `roko-lang-{rust,ts,go}` | Shipping | Language-specific support |
| **MCP** | `roko-mcp-{code,github,slack,scripts,stdio}` | Shipping/Partial | MCP integrations |
| **CLI** | `roko-cli` | Shipping | User-facing binary, TUI, plan runner |
| **Server** | `roko-serve` | Shipping | HTTP control plane (~376 routes) |
| **Server** | `roko-agent-server` | Shipping | Per-agent HTTP sidecar (14 routes) |
| **ACP** | `roko-acp` | Shipping | Agent Client Protocol for editor integration |
| **Demo** | `roko-demo` | Shipping | Demo/example binary |

---

## 11. Healthy Dependency Patterns

**L0 crates**: Zero upward dependencies. Clean kernel boundary.

**L1 crates**: Depend only on L0. Clean framework layer.

**L4 crate** (`roko-cli`): Depends on all layers. Expected for the entry-point binary.

**MCP crates**: Zero internal dependencies. Clean utility layer.

**Cross-cut crates**: Injected via trait objects across layer boundaries. No layer violations
when properly wired through the fabric traits.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Beer 1972, *Brain of the Firm* | Viable System Model: 5 recursive subsystems. Maps to Roko's 5 layers. |
| Ashby 1956, *An Introduction to Cybernetics* | Law of Requisite Variety: motivates compositional layer design. |
| Ousterhout 2018, *A Philosophy of Software Design* | Information hiding and deep module design. |
| Parnas 1972, CACM 15(12) | Information hiding: modules hide design decisions. Each layer hides its implementation. |

---

## Cross-References

- `vision-and-thesis.md` -- Why five layers (Beer VSM mapping)
- `synapse-traits-12.md` -- Traits distributed across layers
- `substrate-trait.md` -- Store deep dive (L0 kernel fabric)
- `bus-transport-fabric.md` -- Bus deep dive (L0 kernel fabric)
- `scorer-gate-router-composer-policy.md` -- Five operational traits by layer
- `naming-and-glossary.md` -- Canonical terminology
