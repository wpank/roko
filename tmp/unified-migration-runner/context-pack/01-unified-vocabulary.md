# Unified Vocabulary

**NAMING PRECEDENCE (mandatory):**
1. `tmp/unified/` (22 spec files) — **canonical, overrides everything**
2. `tmp/architecture/` (21 architecture files) — **current design, supplements unified**
3. `docs/` (422 source files) — **legacy, use for algorithmic detail only**

When any naming conflict exists, unified wins. When unified is silent on a topic, architecture wins. Docs are historical context, never the source of truth for naming or structure.

This is the target vocabulary for the migration. Every rename and rewire moves toward these terms.

## 4 Fundamentals

| New Name | Old Name(s) | What |
|---|---|---|
| **Signal** | Engram, Artifact, Knowledge Entry, Pheromone | Durable datum. SHA-256 addressed, scored on 5 axes, decayed via demurrage, HDC fingerprinted. |
| **Pulse** | Envelope, Event, Message | Ephemeral datum. Lives on Bus, ring-buffered, lineage-hinted, ~seconds lifetime. |
| **Cell** | Module, Recipe stage, Block | Atomic computation unit. Typed I/O, protocol conformance, capability-gated. Every operator is a learner (predict-publish-correct). |
| **Graph** | Workflow, StateGraph, Plan/tasks.toml | Composition of Cells wired by typed edges. TOML-defined. Hot Graphs stay resident. |

## 2 L0 Kernel Primitives (not protocols — infrastructure)

| Primitive | Old Name | What |
|---|---|---|
| **Store** | Substrate / FileSubstrate | Durable persistence fabric. `put/get/query/prune`. Signals live here. |
| **Bus** | EventBus | Ephemeral transport fabric. `publish/subscribe/replay`. Pulses live here. Ring-buffered. |

Store and Bus are **dual**: Store is pull (query when needed), Bus is push (subscribe to stream). Graduation (`Pulse → Signal`) is the ONLY path from ephemeral to durable. Projection (`Signal → Pulse`) is the lossy reverse.

## 9 Protocols (trait + Cell implementations)

| New Name | Old Name | Signature | Layer |
|---|---|---|---|
| **Store** | Substrate | `put/get/query/prune` | L0 |
| **Score** | Scorer | `rate(Signal) -> ScoreVector` | L1 |
| **Verify** | Gate | `check(Signal) -> Verdict` (pre + post + stream) | L3 |
| **Route** | Router | `select(candidates) -> ranked` via EFE | L2 |
| **Compose** | Composer | `assemble(bids, budget) -> Prompt` via VCG auction | L2 |
| **React** | Policy | `watch(Pulses) -> Action` (takes Pulses, NOT Signals) | L4 |
| **Observe** | *(new)* | Read-only observation → Lens specialization | L3 |
| **Connect** | Connector | `connect/query/execute/health/disconnect` | L1 |
| **Trigger** | *(new)* | `arm/disarm/poll` → fires Graphs on events | L4 |

## 10 Specializations (Graph + protocol combos)

| Name | What | Key Trait |
|---|---|---|
| **Flow** | Graph at runtime (RunId + state + snapshots) | — |
| **Rack** | Graph + Macros (knobs) + Slots (jacks) | — |
| **Trigger** | Cell + Trigger protocol (fires Graphs on events) | TriggerProtocol |
| **Lens** | Cell + Observe protocol (read-only views, stackable) | ObserveProtocol |
| **Loop** | Graph with feedback edge (learning loops L0–L4) | — |
| **Memory** | Store + demurrage + dreams | StoreProtocol |
| **Space** | Isolation boundary + capability grants | — |
| **Extension** | Cell intercepting pipeline (8 layers, 22 hooks, CaMeL IFC) | — |
| **Agent** | Space + Extensions + Memory + clocks + vitality + CorticalState | — |
| **Connector** | Cell + Connect protocol + lifecycle management | ConnectProtocol |

## 5 Layers (protocol dependency lattice)

| Layer | Name | Protocols | What lives here |
|---|---|---|---|
| L0 | Runtime | Store + Bus | Infrastructure: persistence + transport |
| L1 | Framework | Connect + Score | External I/O + evaluation |
| L2 | Scaffold | Compose + Route | Assembly + selection (VCG + EFE) |
| L3 | Harness | Verify + Observe | Checking + monitoring |
| L4 | Orchestration | React + Trigger | Event-driven + lifecycle |

## Key Patterns

### Demurrage (Gesell 1916)
Every Signal has a `balance` that decays over time:
```
balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt
```
- `r` = flat tax/day, `beta` = exponential decay/day
- Reinforcement restores balance when Signal is retrieved, cited, or gate-passed
- Balance < 0.01 = cold → archive to slow storage
- Tier multipliers: Transient (0.1×), Working (0.5×), Consolidated (1.0×), Persistent (5.0×)

### Predict-Publish-Correct (Friston 2006)
**Every Cell is a learner.** The universal pattern:
1. Cell publishes `prediction.{cell_id}` on Bus before executing
2. Cell executes and produces output
3. Verify protocol checks output → publishes `outcome.{cell_id}`
4. CalibrationReact joins prediction + outcome by lineage → updates calibration
5. Cell adjusts future behavior based on calibration feedback

This is how routing learns, scoring calibrates, and composition improves.

### Expected Free Energy (EFE) Routing
Route protocol selects candidates by minimizing:
```
EFE(candidate) = -pragmatic_value - epistemic_value + cost_term
```
- Pragmatic = expected reward from Verify verdicts
- Epistemic = information gain (explores under-tried candidates)
- Cost = tokens × price + latency × urgency
- Regime-aware: Crisis → exploit (high pragmatic weight), Calm → explore (high epistemic weight)

### VCG Auction (Context Assembly)
Compose protocol uses Vickrey-Clarke-Groves auction:
- Context bidders (Task, Code, Research, Episode, Heuristic, Tool, Safety, Neuro) submit truthful bids
- VCG selects value-maximizing set within token budget
- Each bidder pays their externality cost (not their bid)
- Section effects tracked via Beta posteriors → sections that correlate with gate success get higher priority

### T0/T1/T2 Inference Tiers
| Tier | Name | What | When |
|---|---|---|---|
| T0 | Reflex | Pattern-match, no LLM | Known patterns, cache hits |
| T1 | Fast | Small/cached model | Moderate complexity |
| T2 | Deep | Full model (Opus, o3) | Novel problems, high stakes |

### 9-Step Agent Heartbeat (from tmp/architecture/)
```
OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT
```

### Three Cognitive Timescales
| Scale | Period | What |
|---|---|---|
| Gamma | 100ms–1s | Fast perception, reflex |
| Theta | 5s–30s | Reflective planning |
| Delta | 1m–10m | Deep consolidation, dreams |
