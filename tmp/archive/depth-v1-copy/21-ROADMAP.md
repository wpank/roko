# 21 — Roadmap

> Phased implementation plan mapping the unified spec to existing crates.

**Source**: `tmp/architecture/18-roadmap.md` (rewritten for the unified model).

---

## 1. Phase 0 — Current State

What is already built, working, and wired. This is the foundation the roadmap builds on.

### 1.1 Crate Mapping

| Crate | Unified Concepts | Status |
|---|---|---|
| `roko-core` | Signal (Engram), 6 protocols (Store/Score/Verify/Route/Compose/React) | Kernel, stable |
| `roko-agent` | Agent specialization: 9-step pipeline, 5+ LLM backends, MCP, tool loop, safety | Dispatch wired |
| `roko-orchestrator` | Graph execution (plan DAG, parallel executor, merge queue) | Wired via orchestrate.rs |
| `roko-gate` | Verify protocol: 11 gates, 7-rung pipeline, adaptive thresholds | Wired, called per-task |
| `roko-compose` | Compose protocol: prompt assembly, 9 templates, VCG auction, enrichment | Wired |
| `roko-learn` | Learning Loops 1+2: episodes, cascade router, experiments, efficiency, bandits | Fully wired |
| `roko-neuro` | Memory specialization: knowledge store, tiers, HDC fingerprints, distillation | Wired |
| `roko-dreams` | Loop 3 partial: NREM/REM/Integration phases built, no runtime trigger | Built, not triggered |
| `roko-conductor` | 10 watchers, circuit breaker, diagnosis | Wired into executor |
| `roko-runtime` | ProcessSupervisor, event bus, cancellation | Wired into PlanRunner |
| `roko-primitives` | HDC vectors, tier routing | Fully wired |
| `roko-daimon` | Affect engine, somatic markers, dispatch modulation | Wired per-task |
| `roko-serve` | ~85 HTTP routes, SSE, WebSocket on :6677 | Wired |
| `roko-agent-server` | Per-agent sidecar: `/message`, `/stream`, `/predictions`, `/research`, `/tasks` | Wired |
| `roko-cli` | All CLI commands + ratatui TUI (F1-F7 tabs) | Main entry point |
| `roko-fs` | FileSubstrate (JSONL), GC, layout | Stable |
| `roko-std` | 19 builtin tools, mock dispatcher | Stable |
| `roko-mcp-code` | Code-intelligence MCP server | Wired |
| `roko-index` | Parser + graph + HDC indexing | Built |
| `roko-chain` | Chain witness primitives, marketplace, validation registry | Partial |

### 1.2 Working End-to-End Flows

The following flows work today via CLI:

1. **Self-hosting loop**: `prd idea` -> `prd draft` -> `prd plan` -> `plan run` -> gate validate -> persist results -> `plan run --resume`
2. **Research-enhanced planning**: `research enhance-prd` -> `prd plan` with research context
3. **Automatic replan**: Gate failure triggers `build_gate_failure_plan_revision` (Loop 2)
4. **Auto-plan on publish**: `prd.auto_plan` config triggers plan generation when PRD is published
5. **Interactive monitoring**: `roko dashboard` TUI with F1-F7 tabs
6. **HTTP control plane**: `roko serve` exposes ~85 routes for external callers
7. **Agent sidecar**: `roko agent serve` with real LLM dispatch

### 1.3 What Phase 0 Lacks

In unified vocabulary terms:

- No **Observe protocol** (Lens system) -- monitoring exists but is not protocol-based
- No **Trigger protocol** -- triggers are hardcoded, not declarative
- No **Connect protocol** formalization -- connectors exist but are not protocol-based
- No **Graph authoring** -- plans are TOML task lists, not typed Graphs with edges
- No **Rack** abstraction -- no parameterized Graphs
- No **TypeSchema validation** at load time
- **Loop 3** (dream cycle) is built but has no runtime trigger
- **Loop 4** (structural adaptation) does not exist
- On-chain registries are specified but not deployed

---

## 2. Phase 1 — Wire Unified Vocabulary (Near-Term)

Add the three new protocols, formalize the Lens system, and close the Loop 3 gap.

### 2.1 Observe Protocol + 10 Built-In Lenses

**Goal**: Every Block, Graph, Agent, and Space can have Lenses attached. Observation never modifies what it observes.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define `Observe` trait in roko-core | `crates/roko-core/src/traits.rs` | S | -- |
| Define `Lens` specialization struct | `crates/roko-core/src/lens.rs` | S | Observe trait |
| Implement AgentLens | `crates/roko-conductor/src/lenses/` | M | Lens struct |
| Implement PlanLens | `crates/roko-conductor/src/lenses/` | M | Lens struct |
| Implement GateLens | `crates/roko-gate/src/lenses/` | S | Lens struct |
| Implement RouterLens | `crates/roko-learn/src/lenses/` | S | Lens struct |
| Implement MemoryLens | `crates/roko-neuro/src/lenses/` | S | Lens struct |
| Implement CostLens | `crates/roko-learn/src/lenses/` | S | Lens struct |
| Implement HealthLens | `crates/roko-serve/src/lenses/` | S | Lens struct |
| Implement ErrorLens | `crates/roko-conductor/src/lenses/` | S | Lens struct |
| Implement ThroughputLens | `crates/roko-serve/src/lenses/` | S | Lens struct |
| Implement DreamLens | `crates/roko-dreams/src/lenses/` | S | Lens struct |
| Wire Lenses into TUI dashboard | `crates/roko-cli/src/tui/` | M | All Lenses |
| Wire Lenses into HTTP routes | `crates/roko-serve/src/routes/` | M | All Lenses |

### 2.2 Trigger Protocol

**Goal**: Declarative event-driven Graph firing. Replace hardcoded trigger logic.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define `Trigger` trait in roko-core | `crates/roko-core/src/traits.rs` | S | -- |
| Implement CronTrigger | `crates/roko-runtime/src/triggers/` | M | Trigger trait |
| Implement WebhookTrigger | `crates/roko-serve/src/triggers/` | M | Trigger trait |
| Implement SignalTrigger (react to Bus events) | `crates/roko-runtime/src/triggers/` | M | Trigger trait |
| Implement ChainTrigger (react to on-chain events) | `crates/roko-chain/src/triggers/` | M | Trigger trait |
| Wire triggers into Agent reactive mode | `crates/roko-cli/src/orchestrate.rs` | M | All triggers |

### 2.3 Connect Protocol

**Goal**: Formalize the connector pattern that already exists in `roko-agent/src/connector.rs`.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define `Connect` trait in roko-core | `crates/roko-core/src/traits.rs` | S | -- |
| Refactor existing connectors to implement Connect | `crates/roko-agent/src/connector.rs` | M | Connect trait |
| Add lifecycle management (connect/disconnect/health) | `crates/roko-runtime/src/connectors/` | M | Connect trait |

### 2.4 Dream Cycle Runtime Trigger (Loop 3 Gap)

**Goal**: The dream cycle runs automatically, not just when manually invoked.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Add CronTrigger for dream cycle | `crates/roko-dreams/src/trigger.rs` | S | CronTrigger |
| Wire dream trigger into `roko serve` startup | `crates/roko-serve/src/lib.rs` | S | Dream trigger |
| Add `roko knowledge dream schedule` CLI | Already exists, wire to trigger | S | Dream trigger |

### 2.5 TypeSchema Validation

**Goal**: Validate Signal payloads against declared schemas at Graph-load time, not runtime.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Add TypeSchema validation to plan loader | `crates/roko-orchestrator/src/plan.rs` | M | -- |
| Validate task inputs/outputs at plan parse time | `crates/roko-orchestrator/src/executor.rs` | S | Plan loader |

---

## 3. Phase 2 — Full Graph Engine (Medium-Term)

Replace the current plan executor with a proper Graph engine. Plans become Graphs; tasks become Blocks.

### 3.1 TOML Graph Authoring + Loader

**Goal**: Authors define Graphs in TOML with typed edges, and the loader validates them.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define Graph TOML schema | `crates/roko-orchestrator/src/graph/schema.rs` | M | Phase 1 TypeSchema |
| Implement Graph loader with validation | `crates/roko-orchestrator/src/graph/loader.rs` | L | Graph schema |
| Migration tool: convert existing plans to Graph TOML | `crates/roko-cli/src/migrate.rs` | M | Graph loader |

### 3.2 Graph Execution Engine

**Goal**: The engine replaces the current parallel executor. It handles typed edges, conditional branching, retry strategies, and snapshot/resume.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Implement Graph executor with state machine | `crates/roko-orchestrator/src/graph/executor.rs` | L | Graph loader |
| Add Flow (runtime Graph instance) with RunId | `crates/roko-orchestrator/src/graph/flow.rs` | M | Graph executor |
| Wire Graph executor into `plan run` | `crates/roko-cli/src/orchestrate.rs` | L | Flow |
| Snapshot/resume for Graph execution | `crates/roko-orchestrator/src/graph/snapshot.rs` | M | Flow |

### 3.3 Rack (Parameterized Graphs)

**Goal**: Graphs with exposed Macros (knobs) and Slots (jacks) for reusability.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define Rack struct (Graph + Macros + Slots) | `crates/roko-orchestrator/src/graph/rack.rs` | M | Graph schema |
| Implement Macro substitution at load time | `crates/roko-orchestrator/src/graph/macro_expand.rs` | M | Rack struct |
| Implement Slot binding | `crates/roko-orchestrator/src/graph/slot.rs` | S | Rack struct |

### 3.4 Visual Graph Editor

**Goal**: Dashboard component for visual Graph authoring and debugging.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Graph visualization in TUI (ratatui canvas) | `crates/roko-cli/src/tui/graph_view.rs` | L | Graph executor |
| Flow live-view (highlight active Blocks) | `crates/roko-cli/src/tui/flow_view.rs` | M | Flow |

### 3.5 Marketplace v1

**Goal**: Publish, install, and fork Graphs and Blocks.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Block manifest format (TOML) | `crates/roko-core/src/manifest.rs` | S | -- |
| Local Block registry | `crates/roko-orchestrator/src/registry.rs` | M | Block manifest |
| `roko marketplace publish/install/fork` CLI | `crates/roko-cli/src/marketplace.rs` | L | Local registry |
| Marketplace HTTP routes | `crates/roko-serve/src/routes/marketplace.rs` | M | Local registry |

---

## 4. Phase 3 — Autonomy + Chain (Long-Term)

Full autonomous operation with on-chain anchoring.

### 4.1 Learning Loop 4 (Structural Adaptation)

**Goal**: The system proposes structural changes to its own Graphs and Blocks, subject to human approval.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Define structural change proposals | `crates/roko-learn/src/structural.rs` | M | Phase 2 Graph engine |
| Implement approval workflow | `crates/roko-serve/src/routes/approvals.rs` | M | Structural proposals |
| Wire Loop 4 into dream cycle | `crates/roko-dreams/src/structural.rs` | L | Approval workflow |

### 4.2 RecursiveSafetyMonitor

**Goal**: A Verify-protocol Block that monitors other Verify-protocol Blocks. Prevents safety bypass through composition.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Implement RecursiveSafetyMonitor | `crates/roko-gate/src/recursive_safety.rs` | L | Phase 2 Graph engine |
| Wire into Graph executor as mandatory wrapper | `crates/roko-orchestrator/src/graph/safety.rs` | M | RecursiveSafetyMonitor |

### 4.3 On-Chain Registry Deployment

**Goal**: Deploy InsightStore, PheromoneRegistry, and AgentPassport contracts to Korai.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Finalize Solidity contracts | `contracts/src/` | L | -- |
| Deploy to Mirage testnet | `contracts/deploy/` | M | Contracts |
| Implement Rust clients | `crates/roko-chain/src/clients/` | L | Deployed contracts |
| Wire passport registration into Agent startup | `crates/roko-agent/src/passport.rs` | M | Passport client |
| Wire knowledge publication from neuro store | `crates/roko-neuro/src/publish.rs` | M | InsightStore client |

### 4.4 Arena System

**Goal**: Full arena, eval, and bounty market as specified in [19-ARENAS-EVALS-BOUNTIES.md](19-ARENAS-EVALS-BOUNTIES.md).

| Task | Target | Size | Depends On |
|---|---|---|---|
| Arena types + registry in roko-chain | `crates/roko-chain/src/arena.rs` | L | On-chain contracts |
| Eval types + registry in roko-chain | `crates/roko-chain/src/eval.rs` | M | On-chain contracts |
| Bounty API routes in roko-serve | `crates/roko-serve/src/routes/bounties.rs` | L | Arena types |
| Arena API routes in roko-serve | `crates/roko-serve/src/routes/arenas.rs` | L | Arena types |
| VCG batch matching for bounties | Wire `vcg_allocate` into bounty routes | M | Bounty routes |

### 4.5 Cross-Agent Knowledge Sharing

**Goal**: Agents share knowledge Signals through the relay and on-chain InsightStore.

| Task | Target | Size | Depends On |
|---|---|---|---|
| Knowledge Signal broadcast via relay | `crates/roko-runtime/src/knowledge_sync.rs` | M | Relay |
| Selective sync based on HDC similarity | `crates/roko-neuro/src/sync.rs` | M | HDC indexing |
| On-chain knowledge discovery | `crates/roko-chain/src/knowledge_discovery.rs` | M | InsightStore client |

---

## 5. Crate-to-Concepts Mapping

Complete mapping of every crate to the unified concepts it owns.

| Crate | Fundamentals | Protocols | Specializations | Loops |
|---|---|---|---|---|
| `roko-core` | Signal, Block, Graph (types) | Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger (traits) | -- | -- |
| `roko-agent` | -- | Connect (LLM backends) | Agent (9-step pipeline, modes, clock) | -- |
| `roko-agent-server` | -- | -- | Agent (HTTP sidecar) | -- |
| `roko-orchestrator` | Graph (execution) | -- | Flow, Rack | -- |
| `roko-gate` | -- | Verify (11 implementations) | -- | -- |
| `roko-compose` | -- | Compose (9 templates, VCG) | -- | -- |
| `roko-learn` | -- | Score, Route | Loop (Loops 1+2) | Loop 1 (episode), Loop 2 (cascade) |
| `roko-neuro` | Signal (knowledge) | Store (knowledge tier) | Memory | -- |
| `roko-dreams` | -- | -- | Loop (Loop 3) | Loop 3 (dream cycle) |
| `roko-conductor` | -- | Observe (watchers) | Lens | -- |
| `roko-runtime` | -- | Trigger, Connect | Trigger, Connector | -- |
| `roko-primitives` | Signal (HDC) | -- | -- | -- |
| `roko-daimon` | -- | React (affect modulation) | Extension | -- |
| `roko-serve` | -- | Observe (health) | Lens (HTTP) | -- |
| `roko-cli` | -- | -- | Agent (CLI), Lens (TUI) | -- |
| `roko-fs` | -- | Store (JSONL) | -- | -- |
| `roko-std` | Block (19 tools) | -- | -- | -- |
| `roko-chain` | -- | Store (on-chain), Connect (chain) | Connector (chain) | -- |
| `roko-index` | -- | Store (code graph) | -- | -- |
| `roko-mcp-code` | -- | Connect (MCP) | Connector (code intel) | -- |

---

## 6. Dependency Graph

```
Phase 0 (Current State)
    |
    v
Phase 1 (Unified Vocabulary)
    |-- 1.1 Observe protocol + Lenses
    |-- 1.2 Trigger protocol
    |-- 1.3 Connect protocol
    |-- 1.4 Dream cycle trigger (Loop 3 gap)
    |-- 1.5 TypeSchema validation
    |
    v
Phase 2 (Full Graph Engine)
    |-- 2.1 TOML Graph authoring ---------> 2.3 Rack
    |-- 2.2 Graph execution engine -------> 2.4 Visual editor
    |                                   \-> 2.5 Marketplace v1
    |
    v
Phase 3 (Autonomy + Chain)
    |-- 3.1 Loop 4 (structural adaptation)
    |-- 3.2 RecursiveSafetyMonitor
    |-- 3.3 On-chain registry deployment
    |-- 3.4 Arena system
    |-- 3.5 Cross-agent knowledge sharing
```

### Parallel Tracks

- Phase 1 tasks (1.1-1.5) are largely independent and can run in parallel.
- Phase 2.3, 2.4, 2.5 depend on 2.1 (Graph schema) but are independent of each other.
- Phase 3.1 and 3.2 depend on Phase 2 (Graph engine). Phase 3.3-3.5 depend on each other but not on 3.1-3.2.

### Critical Path

```
Phase 1.1 (Observe) -> Phase 1.5 (TypeSchema) -> Phase 2.1 (Graph TOML)
    -> Phase 2.2 (Graph executor) -> Phase 3.1 (Loop 4)
```

---

## 7. Migration Strategy

### 7.1 Naming: Code vs. Spec

The unified vocabulary applies at the spec and documentation level. Existing Rust code retains its current names for backward compatibility:

| Spec Name | Code Name | When to Rename |
|---|---|---|
| Signal | `Engram` | Phase 2 (when Graph engine provides migration point) |
| Block | Module/trait impl | Phase 2 (Block trait introduced) |
| Graph | Plan/tasks.toml | Phase 2 (Graph TOML replaces plan format) |
| Store | `Substrate` | Phase 1 (add type alias) |
| Score | `Scorer` | Phase 1 (add type alias) |
| Verify | `Gate` | Phase 1 (add type alias) |
| Route | `Router` | Phase 1 (add type alias) |
| Compose | `Composer` | Phase 1 (add type alias) |
| React | `Policy` | Phase 1 (add type alias) |
| Observe | -- (new) | Phase 1 (new trait) |
| Connect | `Connector` | Phase 1 (formalize existing) |
| Trigger | -- (new) | Phase 1 (new trait) |

### 7.2 Plan-to-Graph Migration

Existing `tasks.toml` plans will be automatically convertible to Graph TOML:

```toml
# Old: tasks.toml
[[task]]
name = "implement-feature"
prompt = "..."
depends_on = ["research"]

# New: graph.toml
[[block]]
id = "implement-feature"
type = "agent"
config = { prompt = "..." }
inputs = [{ from = "research", edge = "output" }]
```

A migration CLI command will handle conversion: `roko plan migrate <dir>`.

### 7.3 Bardo Source References

Everything in this roadmap has prior art in the bardo codebase. Key references:

| Component | Bardo Source | LOC | Roko Target |
|---|---|---|---|
| Inference gateway | `bardo/apps/bardo-gateway/` | 22,800 | `crates/roko-gateway/` (new) |
| 9-step heartbeat | `bardo/crates/golem-heartbeat/` | 10,200 | `crates/roko-conductor/` |
| DeFi tools | `bardo/crates/golem-tools/` | 7,200 | `crates/roko-std/` |
| Chain runtime | `bardo/crates/golem-chain/` | 5,300 | `crates/roko-chain/` |
| Dashboard | `bardo/apps/dashboard/` | 27,000 | `apps/dashboard/` (new) |

---

## 8. Success Criteria

### Phase 1 Complete When

- [ ] `Observe`, `Trigger`, and `Connect` traits exist in roko-core
- [ ] 10 Lenses are implemented and wired into TUI + HTTP
- [ ] Dream cycle runs on a configurable schedule without manual invocation
- [ ] TypeSchema validation catches type mismatches at plan load time
- [ ] All existing tests pass (no regressions)

### Phase 2 Complete When

- [ ] A Graph can be authored in TOML and executed via `roko plan run`
- [ ] Existing plans are automatically convertible via `roko plan migrate`
- [ ] Rack substitution works (Macros are expanded, Slots are bound)
- [ ] Graph execution supports snapshot/resume
- [ ] A Block can be published to and installed from a local registry

### Phase 3 Complete When

- [ ] Loop 4 proposes a structural change and it is approved/rejected via the approval workflow
- [ ] RecursiveSafetyMonitor prevents a deliberately crafted safety bypass
- [ ] AgentPassport, InsightStore, and PheromoneRegistry are deployed on Mirage
- [ ] An Agent registers a passport, publishes a knowledge Signal, and receives a reputation attestation
- [ ] Two Agents discover each other's knowledge Signals through the relay
