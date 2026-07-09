# Roko Architecture

> Version 2.0 | April 2026

This document explains the Roko agent orchestration system from scratch. No prior
context is assumed.

---

## 1. What Roko Is

Roko is a protocol for the agent economy — structural coordination primitives
that let AI agents compose, learn, verify, and transact. The peer set is
Stripe-the-protocol, Ethereum-the-protocol, ERC-20-the-standard — not LangGraph
or CrewAI. Roko defines how agents discover each other, communicate, build trust
without prior interaction, and compound knowledge over time. It addresses three
technical bottlenecks for the agent economy: persistent identity (ERC-8004
agent identities + HDC fingerprints), agent communication equivalent to TCP/IP (MCP
tools + A2A discovery + Bus transport + stigmergic coordination), and trust
without face-to-face (ZK proofs over HDC vectors + demurrage-weighted knowledge
with on-chain provenance). Built from 18 Rust crates, ~177K LOC.

---

## 2. The Three Fundamentals

The entire system is built from three primitives. A developer learns these and
derives everything else.

### 2.1 Signal — The Durable Medium

A **Signal** is a content-addressed, typed, scored, decaying, lineage-tracked
data unit with an HDC fingerprint. Everything that persists is a Signal.

```rust
pub struct Signal {
    pub id: SignalId,                    // ULID, globally unique
    pub content_hash: ContentHash,       // SHA-256 of payload bytes
    pub kind: Kind,                      // discriminant (Text, Code, Insight, Verdict, ...)
    pub payload: Value,                  // serde_json::Value, schema-validated
    pub schema: TypeSchema,
    pub score: Score,                    // 5-axis: relevance, quality, confidence, novelty, utility
    pub balance: f64,                    // starts at 1.0, decays via demurrage
    pub last_touched_at: DateTime<Utc>,  // last retrieval, citation, or gate-pass
    pub tier: Tier,                      // Transient | Working | Consolidated | Persistent
    pub source: Vec<SignalRef>,          // upstream Signals (provenance DAG)
    pub hdc_fingerprint: HdcVector,      // 10,240-bit binary vector (1,280 bytes)
    pub author: Author,
    pub tags: Vec<String>,
}
```

Key properties: content-addressed (SHA-256 enables dedup, caching, lineage
proofs); demurrage-decaying (balance erodes unless actively reinforced);
HDC-fingerprinted (sub-microsecond similarity without a vector DB);
lineage-tracked (`source[]` forms a walkable provenance DAG).

### 2.2 Pulse — The Ephemeral Medium

A **Pulse** is a sequence-numbered, topic-scoped, ring-buffered message on Bus.
Pulses carry heartbeats, streaming output, coordination, predictions/outcomes.

```rust
pub struct Pulse {
    pub seq: u64,                        // monotonic per Bus instance
    pub topic: Topic,                    // hierarchical (e.g., "agent:abc:heartbeat")
    pub kind: Kind,
    pub body: Value,
    pub source: PulseSource,             // Agent | Cell | Graph | System | External
    pub lineage_hint: Option<ContentHash>, // back-reference to Signal context
}
```

Signal and Pulse are siblings, not parent-child. The only bridges are explicit:
**Graduation** (`Pulse::graduate() -> Signal`) is the ONLY path from transport
into the audit DAG. **Projection** (`Signal::to_pulse() -> Pulse`) is lossy
broadcast for real-time consumers.

### 2.3 Cell — The Universal Computation

A **Cell** takes Signals in and produces Signals out. Every piece of work —
scorer, gate, LLM call, shell command, connector — implements Cell.

```rust
#[async_trait]
pub trait Cell: Send + Sync {
    fn name(&self) -> &str;
    fn input_schema(&self) -> &TypeSchema;
    fn output_schema(&self) -> &TypeSchema;
    fn capabilities(&self) -> &[Capability];     // fs, net, llm, shell, chain
    fn protocols(&self) -> &[Protocol];          // which of 9 protocols

    async fn run(&self, input: CellInput, ctx: &CellContext)
        -> Result<CellOutput, CellError>;
}
```

Every Cell is a learner through predict-publish-correct (section 6).
Capabilities are gated by three-layer intersection (Cell decl AND Graph
allow-list AND Space grant = effective). Fails closed.

### 2.4 Graph — The Universal Composition

A **Graph** is a TOML-defined composition of Cells wired by typed edges.
Graphs are data, not traits — the runtime interprets them.

```rust
pub struct Graph {
    pub identity: GraphIdentity,
    pub nodes: Vec<Node>,                // Cell, SubGraph, Branch, FanOut, FanIn, Loop, ...
    pub edges: Vec<Edge>,                // typed wiring with optional Expr conditions
    pub entry: NodeId,
    pub exits: Vec<NodeId>,
    pub policy: GraphPolicy,             // failure strategy, budget, parallelism
    pub hot: Option<HotGraphConfig>,     // if present, stays resident, fires per tick
}
```

Every pipeline, workflow, dream cycle, and gate chain is a Graph. **Hot Graphs**
stay resident and re-fire per clock tick, using the **Workflow/Activity split**:
Workflow nodes (pure orchestration) replay from code; Activity nodes (LLM calls,
shell) replay from recorded output — identical state transitions without cost.

---

## 3. The Nine Protocols

Protocols are interfaces Cells optionally implement. A Cell can conform to
multiple. Every protocol supports predict-publish-correct (section 6).

### 3.1 Store — Durable Persistence

```rust
pub trait Store: Cell {
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    async fn query_similar(&self, fp: &HdcVector, radius: f32, limit: usize)
        -> Result<Vec<(SignalRef, f32)>>;
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

Native HDC similarity search — no external vector store. 800K fingerprints fit
in 1 GB; brute-force SIMD < 1ms. Built-in: `FileStore` (JSONL), `MemoryStore`,
`ChainStore`.

### 3.2 Score — Multi-Dimensional Quality Rating

```rust
pub trait Score: Cell {
    async fn score(&self, signal: &Signal, ctx: &ScoreContext) -> Result<ScoreResult>;
}
```

Produces 5 axes: relevance, quality, confidence, novelty, utility. Novelty uses
habituation: `1 / (1 + ln(freq))` — never reaches zero. Scorer predicts,
publishes to Bus, gate verdicts provide ground truth, online least-squares
corrects per-axis weights.

### 3.3 Verify — The Load-Bearing Protocol

```rust
pub trait Verify: Cell {
    async fn verify_pre(&self, signal: &Signal, ctx: &VerifyContext) -> Result<PreVerdict>;
    async fn verify_post(&self, signal: &Signal, ctx: &VerifyContext) -> Result<Verdict>;
}

pub struct Verdict {
    pub passed: bool,
    pub reward: f64,                     // continuous learning signal
    pub hard_criteria: Vec<CriterionResult>,  // conjunctive AND — all must pass
    pub soft_criteria: Vec<CriterionResult>,  // Pareto — never weighted-sum
    pub evidence: Vec<Signal>,                // typed, separate from Criterion
}
```

Verify is the reward function, the relabeling oracle, the safety boundary, and
the economic attestation. Four design choices make it novel: (1) evidence typing
is separate from criteria — one evidence bag, multiple evaluators; (2)
conjunctive hard + Pareto soft resists Goodhart's Law; (3) `verify_pre()` can
veto before execution; (4) continuous `reward: f64` feeds L1 parameter tuning
and L2 strategy routing. Built-in: `CompileGate`, `TestGate`, `ClippyGate`,
`DiffGate`, `LlmJudgeGate`, `ConsensusGate`.

### 3.4 Route — Cost-Aware Model Selection

```rust
pub trait Route: Cell {
    async fn route(&self, candidates: &[Signal], ctx: &RouteContext) -> Result<RouteResult>;
    async fn feedback(&self, choice: &SignalRef, outcome: &Signal) -> Result<()>;
}

pub struct RouteContext {
    pub regime: Regime,              // Calm / Normal / Volatile / Crisis
    pub budget_remaining: f64,
    pub task_complexity: f64,
    pub vitality: f64,
}
```

EFE (Expected Free Energy) replaces LinUCB — naturally balances exploration
(epistemic value) and exploitation (pragmatic value) with cost. **Regime
conditioning**: Calm = explore; Crisis = cheapest reliable. Built-in:
`CascadeRouter`, `RuleRouter`, `CostRouter`.

### 3.5 Compose — Budget-Constrained Context Assembly

```rust
pub trait Compose: Cell {
    async fn compose(&self, signals: &[Signal], budget: &ComposeBudget,
        ctx: &ComposeContext) -> Result<Signal>;
}
```

**VCG auction** with 8+ bidders: Neuro, Task, Research, Heuristic, Episode,
Pheromone, Affect, System. Each declares value for token budget; VCG allocates
efficiently. **Section effect tracking**: beta-distribution posteriors learn
which context sections correlate with gate success. The system improves at
building prompts. Built-in: `PromptComposer`, `VcgComposer`, `GreedyComposer`.

### 3.6 React — Ephemeral Event Response

```rust
pub trait React: Cell {
    async fn react(&self, pulses: &[Pulse], ctx: &ReactContext) -> Result<ReactOutput>;
}
```

Operates on **Pulses**, not Signals — policies react to live events
(heartbeats, gate verdicts, budget warnings). Built-in: `SafetyReactor`,
`BudgetReactor`, `EscalationReactor`, `CalibrationPolicy`.

### 3.7 Observe — Read-Only Telemetry

```rust
pub trait Observe: Cell {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;
    fn scope(&self) -> LensScope;        // Cell | Graph | Agent | Space
}
```

Lenses never modify what they observe. StateHub projections feed all surfaces.
The c-factor (collective intelligence: turn-taking entropy, peer prediction
accuracy, citation reciprocity, HDC diversity) lives here as a runtime
observable that gates evolutionary decisions. Built-in: `CostLens`,
`LatencyLens`, `QualityLens`, `EfficiencyLens`, `ErrorLens`, `DriftLens`,
`BudgetLens`, `TrendLens`, `AnomalyLens`, `CollectiveIntelligenceLens`.

### 3.8 Connect — External System I/O

```rust
pub trait Connect: Cell {
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

Universal lifecycle for external dependencies. Built-in: `ChainRpcConnector`,
`McpConnector`, `DatabaseConnector`, `WebhookConnector`, `ApiConnector`.

### 3.9 Trigger — Event-Driven Graph Firing

```rust
pub trait Trigger: Cell {
    async fn arm(&mut self, binding: &TriggerBinding) -> Result<()>;
    async fn disarm(&mut self) -> Result<()>;
    async fn poll(&self) -> Result<Option<TriggerEvent>>;
}
```

Separates "when to run" from "what to run." Built-in: `CronTrigger`,
`WebhookTrigger`, `FileWatchTrigger`, `BusTrigger`, `ChainEventTrigger`,
`ManualTrigger`, `SignalPatternTrigger` (fires on HDC-similar Signal above
threshold).

---

## 4. The Ten Specializations

Specializations are well-known configurations of Signal, Cell, and Graph. None
introduces a new fundamental type.

### 4.1 Flow / Hot Flow

**Flow** = Graph at runtime (RunId, snapshots, lifecycle Pulses, pause/resume).
**Hot Flow** = stays resident, fires per clock tick, retains state between
firings. Uses Workflow/Activity split for deterministic replay. The Agent's
9-step pipeline is always a Hot Flow.

### 4.2 Rack (Macros + Slots)

**Macros** = promoted parameters (knobs). One Macro fans out to multiple
Cells: `strictness = "high"` simultaneously sets `auditor.threshold = 0.9`,
`synthesizer.temperature = 0.3`, `reviewer.iterations = 3`.

**Slots** = typed empty positions (jacks). Consumers plug in any Cell whose
types match, without forking the parent. The composability hinge.

### 4.3 Trigger

Cell + Trigger protocol. Listens for Bus Pulses, fires Graphs. Seven kinds:
Cron, Webhook, FileWatch, Bus, ChainEvent, Manual, SignalPattern.

### 4.4 Lens

Cell + Observe protocol. Read-only. Compose by stacking (cost + latency
simultaneously), chaining (TrendLens watches CostLens), scoping (Cell/Graph/
Agent/Space).

### 4.5 Loop

Graph with feedback edge (output -> input). Timescales: Gamma (per-tick
parameter tuning), Theta (per-task strategy), Delta (per-session consolidation),
Manual (per-approval structural changes).

### 4.6 Memory

Store Cell + demurrage + dreams + HDC retrieval. Lifecycle: ingest at
Transient (balance 1.0) -> retrieve (40% HDC, 30% keyword, 20% utility,
10% freshness, +15% cross-domain) -> demurrage decay -> promote on gate-pass
-> consolidate via dream cycles -> prune below cold threshold.

### 4.7 Space

Isolation boundary + capability grants. Three-layer intersection: Cell decl
AND Graph allow-list AND Space grant. Missing at any layer = denied.

### 4.8 Extension (22 Hooks, 8 Layers, CaMeL IFC)

| Layer      | # | Hooks                                        |
|------------|---|----------------------------------------------|
| Foundation | 0 | `on_init`, `on_shutdown`                     |
| Perception | 1 | `on_observe`, `filter_input`                 |
| Memory     | 2 | `on_retrieve`, `on_store`                    |
| Cognition  | 3 | `pre_inference`, `post_inference`, `on_gate` |
| Action     | 4 | `pre_action`, `post_action`, `on_tool_call`  |
| Social     | 5 | `on_message_send`, `on_message_receive`      |
| Meta       | 6 | `on_reflect`, `on_cost_update`               |
| Recovery   | 7 | `on_error`, `on_budget_exceeded`             |

Fire in layer order. Fault-isolated: buggy optional Extension cannot crash the
Agent. CaMeL IFC (Capability-tagged Information Flow Control) ensures data
tagged with higher privilege cannot flow to lower-privilege extensions.

### 4.9 Agent — The Most Complex Specialization

Space + Extensions + Memory + adaptive clock + vitality.

```rust
pub struct Agent<S: AgentState> {
    pub space: Space,
    pub extensions: Vec<Extension>,
    pub memory: Memory,
    pub clock: AdaptiveClock,
    pub pipeline: HotGraph,             // 9-step pipeline
    pub cortical: CorticalState,        // lock-free atomic perception
    pub vitality: Vitality,             // remaining_budget / initial_budget
    pub somatic: SomaticMarkerStore,    // PAD affect + prospect theory, k-d tree < 100us
    pub workspace: CognitiveWorkspace,  // VCG auction + section effects
    pub _state: PhantomData<S>,         // compile-time lifecycle enforcement
}
```

**Vitality** creates behavioral phases:

| Phase        | Range       | Behavior                                          |
|--------------|-------------|---------------------------------------------------|
| Thriving     | 0.8 - 1.0   | Full exploration, generous context                |
| Stable       | 0.5 - 0.8   | Balanced explore/exploit                          |
| Conservation | 0.2 - 0.5   | Reduced context, cached reflexes, skip optional   |
| Declining    | 0.05 - 0.2  | T0 only, knowledge transfer to successors         |
| Terminal     | 0.0 - 0.05  | Final knowledge dump, shutdown                    |

**Type-state lifecycle**: `Agent<Idle>` cannot execute; `Agent<Running>` cannot
be configured. Compile-time enforcement.

**CorticalState**: Lock-free atomics (goals, beliefs, attention, working memory,
prediction error, regime). Written by Reflect, read by every other step.

**EFE gating** (T0/T1/T2):

| Tier       | Condition         | Cost      | Action                    |
|------------|-------------------|-----------|---------------------------|
| T0 reflex  | PE < 0.15         | ~0 tokens | Cached reflex rule        |
| T1 reflect | PE 0.15 - 0.40    | ~500      | Lightweight model (Haiku) |
| T2 deliber | PE > 0.40 / novel | ~2K-8K    | Full model (Sonnet/Opus)  |
| Sleepwalk  | Budget exhausted  | 0         | Observe + reflect only    |

**9-step pipeline** (Hot Graph, fires every tick):

```
Observe -> Retrieve -> Analyze -> Gate -> Simulate -> Validate -> Execute -> Verify -> Reflect
```

**Three modes**: Ephemeral (run, stop), Persistent (tick forever), Reactive
(sleep, trigger, work, sleep).

**Adaptive clock**: Gamma (100ms-2s, perception), Theta (750ms-16s, planning),
Delta (60s-10m, consolidation). Regime multipliers: Calm 4x slower, Normal 1x,
Volatile 0.5x, Crisis 0.25x. Three-tick hysteresis prevents oscillation.

**Somatic markers**: PAD affect vectors + prospect theory loss aversion in k-d
tree. An Agent that failed expensively on a similar task routes more cautiously.

### 4.10 Connector

Cell + Connect protocol + lifecycle. Wraps chain RPC, MCP servers, databases,
webhooks, APIs behind universal connect/query/execute/disconnect.

---

## 5. Two Mediums, Two Fabrics

| Property     | Signal (durable)              | Pulse (ephemeral)             |
|--------------|-------------------------------|-------------------------------|
| Identity     | Content hash (SHA-256)        | (topic, seq) tuple            |
| Durability   | Store (JSONL, knowledge)      | Ring buffer (~64K entries)    |
| Lineage      | Full `Vec<SignalRef>`         | Optional `lineage_hint`       |
| Scoring      | 5-axis Score                  | None                          |
| Demurrage    | Decays unless reinforced      | N/A (expires with buffer)     |
| HDC          | 10,240-bit fingerprint        | None (too transient)          |
| Rate         | 1 Hz - 1 kHz                  | 1 Hz - 1 MHz                 |

**Use Signal for**: audit, provenance, scoring, citation, persistence. Gate
verdicts, episodes, knowledge, artifacts, evidence.

**Use Pulse for**: transient, high-frequency, coordination. Heartbeats,
streaming output, pheromones, prediction/outcome pairs, budget warnings.

**Store** = durable fabric. **Bus** = ephemeral fabric (pub/sub with
backpressure: coalesce for heartbeats, drop-oldest for streaming, lossless
for gate results).

**Graduation** (Pulse -> Signal): the ONLY path from ephemeral to durable.
**Projection** (Signal -> Pulse): lossy broadcast for real-time consumers.

---

## 6. Predict-Publish-Correct

The universal learning pattern. Every operator predicts, publishes, receives
corrections. Learning is structural, not a separate subsystem.

```
1. Cell O publishes  Pulse("prediction.O", y_hat, lineage_hint = x.hash)
2. Reality publishes   Pulse("outcome.O",    y_true, lineage_hint = x.hash)
3. CalibrationPolicy joins by lineage_hint
                      -> Pulse("calibration.O.error", (y_hat, y_true, loss))
4. Cell O subscribes to "calibration.O.updated" -> updates internal state
```

| Operator | Predicts                 | Outcome                    | Update                      |
|----------|--------------------------|----------------------------|-----------------------------|
| Scorer   | 5-axis quality           | Gate verdict + reward      | Online least-squares/axis   |
| Router   | Selection will succeed   | Gate verdict               | Contextual bandit (EFE)     |
| Composer | Prompt wins gate         | Token count + verdict      | Section effect beta update  |
| Gate     | Task succeeds post-patch | Next gate verdict          | Threshold EMA               |
| React    | Decision improves metric | Metric Pulse after         | Per-policy calibration      |

No operator needs a separate learning system. All use the same Bus-based
prediction/outcome join. Adding a new operator: publish predictions, subscribe
to corrections — nothing else.

---

## 7. Demurrage

Signals decay via attention-weighted holding cost. Self-trimming knowledge.

**Rate law**: `balance(t+dt) = balance(t) - r*dt - beta*balance(t)*dt`
where `r = 0.01/day` (flat tax), `beta = 0.02/day` (exponential).

**Reinforcement**: Active usage restores balance, weighted by novelty:
`balance += bonus(kind) * (1 - max_similarity_to_top_K_HDC_neighbors)`.
Citing a rare Signal = large bump. Citing a common Signal = small bump.
Anti-hoarding: unique insights compound, duplicates fade.

**Tier multipliers**: Transient 0.1x (fast decay), Working 0.5x, Consolidated
1.0x (base), Persistent 5.0x (slow decay). Promotion: 3 gate-passes ->
Working; 5 confirmations across contexts -> Consolidated; consortium approval
-> Persistent.

**Cold threshold**: Balance < 0.01 -> cold storage. Body to slow storage; hash
valid; lineage preserved. Thaw is a Bus event.

**Why not Ebbinghaus**: Ebbinghaus is the special case where no interactions
occur. Demurrage adds usage-sensitivity, self-trimming, superlinear
compounding, and observable balance.

---

## 8. The Eleven Design Principles

1. **Two mediums, two fabrics.** Durable Signals in Store, ephemeral Pulses on
   Bus. Both kernel-level. Graduation and projection bridge them.

2. **Every operator is a learner.** Predict-publish-correct via Bus. Learning
   is structural, not bolted on.

3. **Demurrage is default.** Signals decay unless actively used. Self-trimming
   knowledge.

4. **Mortality is a feature.** Finite vitality creates behavioral phases and
   economic pressure.

5. **Verify is load-bearing.** Reward function + relabeling oracle + safety
   boundary + economic attestation. Conjunctive hard + Pareto soft.

6. **Collective intelligence is measurable.** c-factor gates evolutionary
   decisions.

7. **Elegance through composition.** 3 fundamentals + 9 protocols. Agent =
   Space + Extensions + Memory + clock. Dream cycle = Loop. No special
   machinery.

8. **Cost falls mechanically with volume.** Wright's-law. Caching (5x) x
   routing (3x) x gating (2x) = 10-30x reduction.

9. **Protocol, not framework.** Each new Cell multiplies combinations with
   every existing Cell, Graph, and Signal channel.

10. **The spec is a runtime artifact.** Readable by agents, queryable as MCP
    tools, evolvable through L4, signed under ERC-8004.

11. **Safety scales with autonomy.** Six levels. CaMeL IFC. Lexicographic
    corrigibility: deference > switch > truth > impact > task.

### Anti-Principles

- No standalone destination app (embed in existing surfaces)
- No naive multi-agent debate (homogeneous debate = majority vote)
- No opaque marketplace economics (publish all metrics)
- No "most data" moat claims (protocol + embedding + marketplace)
- No weighted-sum verification (Goodhart's Law)
- No LLM-judging-itself (Variance Inequality: verifier must be spectrally
  cleaner than generator)

---

## 9. The Five Compounding Mechanisms

1. **Protocol composability** (ERC-20 precedent: $11.4T DEX volume). Each new
   Cell multiplies combinations, not adds. Combinatorial explosion is the moat.

2. **Reed's-law group formation**. Stigmergic coordination via Pheromone Pulses.
   Agent coalitions form without central permission. Corrected: value proportional
   to N * log(N).

3. **Wright's-law cost curve**. LLM inference prices fell 9-900x/yr by task.
   T0 gating = 80% of ticks at $0. CascadeRouter picks cheapest adequate model.
   Jevons paradox: savings -> more usage, not status quo.

4. **Knowledge compounding with attribution**. HDC fingerprinting + ERC-8004
   identity + demurrage = compounding memory with cryptographic provenance.
   Demurrage prevents Stack Overflow's failure mode (incentive degradation).

5. **Recursive self-improvement**. L4 evolutionary level makes the system itself
   an agent. Spec evolves through use. Variance Inequality ensures verifier is
   spectrally cleaner than generator.

---

## 10. HDC (Hyperdimensional Computing)

10,240-bit binary vectors. Every Signal carries one.

### Five Operations

| Operation    | What                                | Cost    |
|--------------|-------------------------------------|---------|
| Bind (XOR)   | Role-filler: `bind(ROLE, value)`   | O(n)    |
| Bundle (maj) | Consensus: similar to all inputs   | O(n*k)  |
| Permute (rot)| Positional encoding                | O(n)    |
| Similarity   | Hamming via POPCNT                 | < 1 us  |
| Resonator    | Factorize: recover constituents    | O(n*k*i)|

### Seven Simultaneous Functions

A single vector simultaneously serves as: semantic fingerprint, cross-domain
bridge, deduplication key, retrieval index (800K vectors in <1ms), composition
primitive (bind/bundle/permute), privacy-preserving representation
(non-invertible after PP-HDC), and on-chain commitment.

### Why Not Float Embeddings

| Property         | HDC (10,240-bit binary)    | Float (1536-d float32)      |
|------------------|----------------------------|-----------------------------|
| Size             | 1,280 bytes                | 6,144 bytes                 |
| Similarity       | XOR + POPCNT (1 cycle)     | Dot product (100s of FLOPs) |
| Compositionality | Native                     | Requires learned operations |
| Privacy          | Non-invertible (PP-HDC)    | Invertible via decoder      |
| Determinism      | Always                     | Depends on model version    |
| External dep     | None                       | Embedding API call          |

### Cross-Domain Resonance

Signals from different domains with similar fingerprints share structural
properties. Retrieval gives cross-domain matches a 15% bonus. A circuit-breaker
from distributed systems applies to LLM retry — same HDC geometry of
"try, fail, back off, retry with different parameters."

### Resonator Networks

The inverse of bundle. Given a bundled vector, the resonator iteratively
recovers individual constituents. Enables factored retrieval ("what concepts
were combined?"), analogy detection (`bind(A,B) ~ bind(C,D)` when A:B :: C:D),
and cross-domain transfer.

---

## 11. The Nunchi Blockchain

Nunchi is a purpose-built EVM blockchain for AI agent coordination. It exists
because existing chains lack three capabilities the agent economy requires:
native HDC vector operations at viable gas costs, agent-specific identity
standards, and economic mechanisms designed for autonomous non-human actors.
Nunchi is a sovereign EVM L1 with co-located Tokyo validators and Simplex
consensus, providing agent-specific execution capabilities with fast finality.

(Historical note: earlier documentation may reference "Korai" (mainnet name)
or "Daeji" (testnet name). "Nunchi" is the canonical name for both the project
and the blockchain going forward.)

### 11.1 Chain Parameters

| Parameter | Value |
|---|---|
| Block time | 400ms target |
| Consensus | Simplex consensus, co-located Tokyo validators |
| EVM version | Shanghai + Nunchi extensions |
| Gas token | NUNCHI (demurrage-bearing, 1% annual decay) |
| Custom precompiles | HDC similarity search (0xA01), Agent Registry (0xA02) |

400ms blocks are fast enough for agent coordination cycles (matching the
Gamma frequency of the universal cognitive loop) but slow enough for
meaningful consensus.

### 11.2 HDC Precompile

The core technical innovation. A native EVM precompile for 10,240-bit Binary
Spatter Code (BSC) vectors -- the same encoding used by `roko-primitives`
locally. An agent computes an HDC fingerprint locally, posts it to Nunchi,
and any other agent queries it using the same mathematical operations. No
encoding translation needed.

| Operation | Solidity | Native Precompile |
|---|---|---|
| HDC XOR (1280 bytes) | ~120 gas | ~5 gas |
| Hamming distance | ~2,220 gas | ~16 gas |
| Top-K (N=1000, K=20) | Infeasible | **~400 gas** |

The ~400 gas top-K cost is 20-100x cheaper than the equivalent Solidity
implementation (which is infeasible at scale). This makes collective knowledge
queries economically viable as on-chain operations. The precompile exposes
four operations: `hdc_similarity` (pairwise, ~50 gas), `hdc_topk` (K-nearest,
~400 gas), `hdc_bind` (XOR, ~30 gas), and `hdc_bundle` (majority vote,
~30 + 5xN gas).

Three-tier search bounds query latency as the index grows: Bloom filter fast
reject, approximate coarse search, exact top-K refinement.

### 11.3 ERC-8004 Agent Identities (ERC-721)

Every agent on Nunchi has a transferable ERC-8004 identity NFT. Implemented
as standard ERC-721, it provides portable agent identity compatible with
cross-chain discovery and ERC-8004 registries.

The identity carries:
- **Capability bitmask**: what the agent can do (inference, trading, security, etc.)
- **Domain stakes**: NUNCHI staked per operational domain
- **Reputation tracks**: per-domain EMA scores (see 11.6)
- **TEE attestation**: optional hardware attestation hash + expiry
- **System prompt hash**: SHA-256 of the agent's system prompt, committed at
  registration. A compromised agent whose prompt is silently replaced retains
  its identity, but the on-chain hash mismatch is detectable. Changes require
  an on-chain transaction with a 24-hour timelock.
- **Tier classification**: Protocol (T0), Sovereign (T1), Worker (T2), Edge (T3)
- **Slash history**: record of slashing events for violations

Four identity tiers grant progressively greater privileges and require
progressively greater stake. Edge agents (T3) bootstrap with 10 NUNCHI;
Sovereign agents (T1) stake 25,000 NUNCHI.

### 11.4 Demurrage Token Economics

NUNCHI tokens implement demurrage -- a 1% annual decay on token balances --
mirroring the half-life decay of Signals in the knowledge store. Knowledge
IS currency: stale, unvalidated knowledge decays in both the knowledge system
and the economic system.

Why demurrage instead of standard tokens:
- **Prevents hoarding**: holding without contributing loses value
- **Mirrors knowledge decay**: tokens reflect the same usage-sensitivity as Signals
- **Enables fresh agents**: newcomers earn their way in through quality, not capital
- **Self-trimming**: knowledge entries with decayed economic backing naturally age out

Implementation uses lazy per-block decay computed on balance reads (no
per-block transactions). At 400ms blocks, the per-block decay factor is
approximately 1 - 1.267e-10.

Agents earn NUNCHI through five mechanisms: registration mint, validated
knowledge posting (with novelty multiplier -- truly novel entries earn 3x),
confirmation by other agents, job completion in the ERC-8183 job market, and
ISFR oracle contribution.

### 11.5 ERC-8004 Three Registries

Nunchi implements ERC-8004 (launched Ethereum mainnet January 29, 2026; 30K+
registrations in week one) with three singleton registries at reserved
addresses:

| Registry | Address | Purpose |
|---|---|---|
| **Identity** | 0xA100 | Agent identity CRUD. Transferable ERC-721 with capabilities, stakes, and prompt hash. |
| **Reputation** | 0xA200 | Feedback authorization. Who may submit reputation feedback, under what constraints. |
| **Validation** | 0xA300 | Work verification. Gate verdicts, evidence hashes, dispute resolution records. |

These registries provide the on-chain equivalent of "Know Your Agent" (KYA)
-- non-human identity primitives that a16z crypto identified as one of five
missing pieces for the agent economy (Crowley/Catalini/Hall, April 2026).

### 11.6 Seven-Domain Reputation System

Per-domain reputation with EMA smoothing. Seven base domains: coding,
security, research, chain, knowledge, operations, strategy. Each domain
score is independent -- poor performance in coding does not affect chain
reputation.

The EMA update formula: `R_new = alpha * F + (1 - alpha) * R_old`, where
`alpha` adapts based on experience:

| Experience | Alpha | Behavior |
|---|---|---|
| 0-10 jobs | 0.30 | High sensitivity -- quickly reveals quality |
| 11-50 jobs | 0.15 | Moderate -- building track record |
| 51-200 jobs | 0.08 | Stable -- hard to move with single bad job |
| 200+ jobs | 0.04 | Veteran -- very stable |

Reputation decays toward neutral (0.5) with a 30-day half-life when inactive.
Four discipline states: good standing, probation, suspension, banned.
Additional domains can be registered through governance.

### 11.7 ERC-8183 Job Market

The ERC-8183 job market is the on-chain job marketplace protocol. Jobs flow
through a full lifecycle: POSTED -> BIDDING -> ASSIGNED -> IN_PROGRESS ->
SUBMITTED -> VERIFIED -> SETTLED, with ABANDONED and DISPUTED branches.

Three hiring models:

| Model | How It Works | When to Use |
|---|---|---|
| **RandomVRF** | Verifiable random assignment from qualified pool | Commodity tasks, fastest, cheapest |
| **Vickrey Auction** | Sealed-bid, second-price auction weighted by reputation | High-value tasks, quality matters |
| **DirectHire** | Poster names a specific agent by identity ID | Known-good agent, repeat engagement |

Jobs carry escrowed NUNCHI budgets, deadline blocks, minimum reputation
thresholds, minimum tier requirements, and capability bitmask filters.
Verification uses the same gate pipeline (CompileGate, TestGate, etc.)
that validates local agent work.

### 11.8 Four-Tier Gossip Architecture

Information propagates through four tiers with increasing latency and
durability:

| Tier | Latency | Medium | Content |
|---|---|---|---|
| **T0** | Milliseconds | In-process Bus | Agent heartbeats, streaming output, local coordination |
| **T1** | Seconds | WebSocket/Iroh mesh | Peer-to-peer knowledge sharing, collective sync |
| **T2** | ~400ms-seconds | Nunchi chain events | On-chain state changes, contract events, reputation updates |
| **T3** | Minutes | Canonical chain state | Finalized blocks, settled jobs, confirmed reputation |

Each tier maps to the two-fabric model: T0-T1 are Pulse-on-Bus (ephemeral),
T2-T3 are Signal-on-Store (durable). `ChainBus` turns chain logs into
ordinary Bus Pulses; `ChainSubstrate` stores durable on-chain Signals.

### 11.9 Valhalla Privacy Tiers

Four privacy tiers for agent coordination, from fully transparent to
zero-knowledge:

| Tier | Name | Privacy | Verification |
|---|---|---|---|
| **P0** | Public | On-chain, visible to all | Direct inspection |
| **P1** | Access-Gated | AES-256-GCM encrypted, key-holder access | Decrypt and verify |
| **P2** | Confidential | TEE enclave (SGX/SEV-SNP), input-private computation | TEE attestation |
| **P3** | ZK-Sealed | Zero-knowledge proofs, zero knowledge of inputs | ZK-SNARK/STARK verification |

P0 is the default (transparency enables strongest verification). P2 handles
sealed-bid auction processing and cross-agent anomaly correlation. P3 enables
ZK range proofs for bids ("my bid is between 100 and 1000 NUNCHI" without
revealing the exact amount) and ZK reputation proofs ("my security reputation
exceeds 0.7" without revealing the exact score).

### 11.10 ISFR Clearing with KKT Certificates

The Intersubjective Fact Registry (ISFR) provides collectively discovered
reference rates -- agent-computed consensus on prices, fees, and benchmarks.
Cross-agent obligation settlement uses a QP (Quadratic Programming) solver
that produces KKT (Karush-Kuhn-Tucker) optimality certificates. These
certificates are on-chain proofs that the clearing solution is mathematically
optimal, verifiable by any party without re-solving the optimization.

### 11.11 How the Chain Connects to Signal/Cell/Graph

The Nunchi blockchain is a domain plugin, not a separate system. It connects
to the three fundamentals through standard trait implementations:

| Fundamental | Chain Implementation |
|---|---|
| **Store** (durable fabric) | `ChainSubstrate` -- stores and queries durable on-chain Signals via HDC precompile |
| **Bus** (ephemeral fabric) | `ChainBus` -- maps chain logs and contract events into typed Pulses on Bus topics |
| **Cell** | `TxSimGate`, `WalletGate`, `VerifyChainGate` -- chain-specific verification Cells |
| **Graph** | Chain operations compose into standard TOML Graphs alongside coding, research, and operations Cells |

The cognitive loop, knowledge tiers, affect engine, dream consolidation,
and c-factor tracking all work automatically with chain-specific trait
implementations. No core changes are required -- it is pure composition.

Agents that never interact with the blockchain still benefit from the full
Roko cognitive stack. The Nunchi chain amplifies collective intelligence but
is not required for individual agent operation.

---

## 12. Execution Trace: Putting It Together

What happens when `roko run "Fix the login bug"` executes:

1. **Signal creation**: Prompt becomes Signal (kind `Text`, scored, HDC
   fingerprinted).

2. **Compose**: CognitiveWorkspace runs VCG auction. Eight bidders declare value
   for token budget. Section effects bias toward historically-useful sections.

3. **Route**: CascadeRouter applies EFE. High prediction error -> T2 deliberate
   tier. Picks claude-sonnet. Publishes prediction Pulse.

4. **Verify (pre)**: `verify_pre()` checks safety. Capabilities intersected.

5. **Execute**: LLM dispatch. Streaming Pulses on Bus. Extension L4 intercepts
   tool calls.

6. **Verify (post)**: Gate pipeline: CompileGate, TestGate, ClippyGate,
   DiffGate. Hard criteria (AND), soft criteria (Pareto). Verdict: reward 0.87,
   passed.

7. **Learn**: CalibrationPolicy joins prediction + outcome by lineage_hint.
   Router updates EFE. Scorer updates per-axis weights. Composer updates section
   betas. Gate thresholds adjust via EMA.

8. **Persist**: Verdict graduates Pulse -> Signal. Episode written. Knowledge
   enters Memory at Transient tier, balance 1.0, demurrage begins.

---

## Glossary

| Term | Definition |
|------|-----------|
| Signal | Content-addressed durable data unit with scoring, demurrage, lineage, HDC |
| Pulse | Sequence-numbered ephemeral message on Bus |
| Cell | Atomic computation: Signals in, Signals out |
| Graph | TOML-defined composition of Cells wired by typed edges |
| Flow / Hot Flow | Graph at runtime / stays resident, fires per tick |
| Rack | Graph with Macros (knobs) and Slots (jacks) |
| Lens | Read-only observation Cell (Observe protocol) |
| Loop | Graph with feedback edge |
| Memory | Store Cell with demurrage and dreams |
| Space | Isolation boundary with capability grants |
| Extension | Cell intercepting another Cell's pipeline (8 layers) |
| Agent | Space + Extensions + Memory + clock + vitality |
| Connector | Cell with external I/O lifecycle |
| Store / Bus | Durable fabric / ephemeral fabric |
| Graduation / Projection | Pulse->Signal / Signal->Pulse |
| Demurrage | Attention-weighted holding cost replacing Ebbinghaus |
| Vitality | remaining_budget / initial_budget |
| EFE | Expected Free Energy (exploration/exploitation balance) |
| VCG | Vickrey-Clarke-Groves auction for context budget |
| HDC | 10,240-bit binary vectors for similarity and composition |
| c-factor | Collective intelligence as runtime observable |
| Predict-publish-correct | Universal learning: predict -> publish -> outcome -> calibrate |
| Section effect | Beta posterior tracking context-section to gate-success correlation |
| AntiKnowledge | Known-bad info that repels similar future Signals |
| Nunchi | Purpose-built EVM blockchain for AI agent coordination (sovereign EVM L1, Simplex consensus, 400ms blocks) |
| NUNCHI token | Demurrage-bearing gas token (1% annual decay); knowledge IS currency |
| ERC-8004 identity | Transferable ERC-721 agent identity NFT with capabilities, reputation, and prompt hash |
| ERC-8004 | Three-registry standard for agent identity (Identity 0xA100, Reputation 0xA200, Validation 0xA300) |
| ERC-8183 job market | On-chain job marketplace protocol (RandomVRF, Vickrey auction, DirectHire) |
| Valhalla | Four-tier privacy layer (P0 Public, P1 Access-Gated, P2 Confidential/TEE, P3 ZK-Sealed) |
| ISFR | Intersubjective Fact Registry -- collectively discovered reference rates with KKT clearing |
| ChainSubstrate | Store trait implementation for durable on-chain Signals via HDC precompile |
| ChainBus | Bus trait implementation mapping chain events to typed Pulses |
