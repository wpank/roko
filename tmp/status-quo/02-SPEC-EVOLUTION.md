# 02 — Spec Evolution: V1 → V2 → V2-Depth

**What changed between spec versions and where the codebase actually sits.**

> Current correction, 2026-07-07: this file is useful for spec history, but some runtime framing predates the final reconciliation. The live plan executor is Runner v2 when invoked explicitly, while the default `roko plan run` Clap value is `graph` and that Graph path dry-runs plan tasks. Use `01-EXECUTIVE-SUMMARY.md`, `12-ROADMAP.md`, `13-CURRENT-STATE-MATRIX.md`, `31-GRAPH-CELLS-ENGINE.md`, and `36-ORCHESTRATION-RUNNERS.md` for current execution truth.

---

## V1 Specification (`docs/v1/`)

22 sections defining the original architecture.

### Core Model
- **1 noun**: `Engram` (now aliased as `Signal`) — the universal data atom
- **6 verb traits**: `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy` (now aliased: Substrate→Store, etc.)
- **Universal loop**: `query → score → route → compose → act → verify → write → react`
- **5 layers**: Core, Runtime, Agent, Compose, Application
- **3 cognitive cross-cuts**: Persistence, learning, safety applied across all layers

### V1 Features Fully Implemented
- [x] Signal (Engram) type with content variants, hash DAG, metadata
- [x] All 6 verb traits with concrete implementations
- [x] FileSubstrate (JSONL) for signal persistence
- [x] Gate pipeline (11 gate types, 7-rung escalation)
- [x] Agent dispatch with tool loop and safety
- [x] Plan DAG execution with parallel scheduling
- [x] Episode logging and learning feedback
- [x] CLI with 45+ subcommands
- [x] SystemPromptBuilder with 9 template layers
- [x] HDC hyperdimensional vectors for fingerprinting

### V1 Features Partially Implemented
- [ ] `loop_tick()` universal loop — defined but never called in production
- [ ] VCG auction for composition — built but greedy path dominates
- [ ] Cold substrate archival — built but no runtime trigger
- [ ] Safety contracts — wired and fail-closed for bundled roles, with advanced warrant/taint/budget hooks still incomplete

---

## V2 Specification (`docs/v2/`)

29 documents redefining the architecture with 4 universal patterns.

### Paradigm Shift: "Everything is a Graph of Cells"

| V1 Concept | V2 Concept | Status in Codebase |
|------------|------------|-------------------|
| Engram (noun) | Cell (node in graph) | `Cell` type exists in `roko-graph`, **not wired** |
| 6 verb traits | Cell behaviors (trait impls on cells) | V1 traits still used at runtime |
| Universal loop | `Pulse`/`Bus` kernel cycle | `Pulse` type exists, **never driven** |
| Substrate (persistence) | Graph persistence (cells + edges) | FileSubstrate (V1) still used |
| Flat signal list | DAG of cells with typed edges | `roko-graph` has DAG, **not wired** |
| Layer hierarchy | Graph layers (cells can span) | V1 5-layer model still applies |

### 4 Universal Patterns (V2)
1. **Predict-Publish-Correct**: Every cell predicts, publishes, and self-corrects
2. **Demurrage**: Knowledge decays over time; unused cells lose priority
3. **EFE Routing**: Expected Free Energy guides routing decisions
4. **Graph-of-Graphs**: Recursive composition — graphs contain sub-graphs

### V2 Documents (29 total)
```
00-architecture.md          — Top-level overview
01-signal.md through 28-*   — Per-component specifications
```

### What's Implemented from V2
- [x] `Signal` type alias (for Engram) — naming only
- [x] `Store` trait alias (for Substrate) — naming only
- [x] `roko-graph` crate — Cell, Graph, typed edges, topo sort, DAG executor
- [x] Pulse type definition
- [ ] Bus kernel (Pulse scheduler) — **not implemented**
- [ ] Predict-publish-correct cycle — **not implemented**
- [ ] Demurrage — **not implemented**
- [ ] EFE routing — **not implemented**
- [ ] Graph-of-Graphs composition — **not implemented**
- [ ] Cell-based agent model — **not implemented**

---

## V2-Depth Specifications (`docs/v2-depth/`)

155+ detailed depth documents across 23 directories. These are the engineering-level specs for V2.

### Directories
```
agent/          — Agent architecture depth (dispatch, lifecycle, MCP)
bus/            — Pulse/Bus kernel specs
cell/           — Cell type system, behaviors, lifecycle
chain/          — Witness chain (Phase 2+)
compose/        — Composition engine, VCG auction
conductor/      — Orchestration, watchers, circuit breakers
core/           — Core types, signal evolution
daimon/         — Affect engine, somatic markers
deploy/         — Deployment patterns
dreams/         — Offline consolidation, imagination
gate/           — Gate pipeline, adaptive thresholds
graph/          — Graph runtime, cell DAG
index/          — Code intelligence
lang/           — Language support (Rust, TS, Go)
learn/          — Learning subsystems
mcp/            — MCP protocol integration
neuro/          — Knowledge store, distillation
orchestrator/   — Plan execution, DAG scheduling
primitives/     — HDC vectors, tier routing
runtime/        — Process supervisor, event bus
safety/         — Contracts, role auth
serve/          — HTTP control plane
tui/            — Terminal UI
```

### Key Depth Docs Not Yet Reflected in Code
| Doc | What It Specifies | Codebase Status |
|-----|-------------------|-----------------|
| `bus/pulse-scheduler.md` | Pulse tick scheduler for Bus kernel | Not implemented |
| `cell/cell-lifecycle.md` | Cell birth, mutation, death, GC | Not implemented |
| `graph/graph-runtime.md` | Graph executor replacing PlanRunner | roko-graph exists but not wired |
| `core/signal-v2.md` | Signal as Cell with typed content | Signal is still V1 Engram |
| `compose/efe-routing.md` | Expected Free Energy for routing | Not implemented |
| `learn/demurrage.md` | Knowledge decay over time | Not implemented |
| `conductor/predict-correct.md` | Predict-publish-correct loop | Not implemented |

---

## Where the Codebase Actually Sits

```
V1 Spec ████████████████████░░ ~90% implemented
V2 Naming ██████████████████░░ ~98% migrated (aliases)
V2 Architecture ██░░░░░░░░░░░░░░░░░░ ~10% implemented
V2-Depth █░░░░░░░░░░░░░░░░░░░ ~5% implemented
```

### The Gap
The codebase is a **V1 runtime with V2 names**. The V2 architecture (Cell/Graph/Pulse/Bus) is specified in 184+ documents and partially built in `roko-graph`, but the runtime still executes through V1's `PlanRunner` + `orchestrate.rs` monolith.

### Decision Needed
1. **Full V2 migration**: Replace PlanRunner with Graph runtime, adopt Cell model, implement Pulse/Bus kernel
2. **Formalize V1**: Accept that V1 is the runtime, treat V2 docs as aspirational, focus on V1 polish
3. **Incremental V2**: Keep V1 runtime but gradually introduce V2 concepts where they add value (e.g., demurrage for knowledge decay, EFE for routing)

Option 3 is recommended — it preserves the working self-hosting pipeline while allowing V2 concepts to be adopted incrementally.
