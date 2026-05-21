# 02 — Block

> The universal computation. Every piece of work in Roko implements Block. Every Block is a learner.

**Subsumes**: Module, Extension hook, Connector, Scorer impl, Gate impl, Router impl, Composer impl, Policy impl, Recipe stage.

---

## 1. Definition

A **Block** is an atomic computation that takes Signals in and produces Signals out. Every Block declares:

1. **Identity** — name, version, description, tags
2. **Typed I/O** — input and output TypeSchemas
3. **Capabilities** — what system resources it needs (fs, net, llm, shell, chain)
4. **Protocol conformance** — which of the 9 protocols it implements
5. **Cost estimation** — expected USD + wall-clock seconds

Blocks are the universal building unit. A scorer is a Block. A gate is a Block. An LLM call is a Block. A shell command is a Block. A connector is a Block. An extension hook is a Block.

**Every Block is a learner.** Through the predict-publish-correct pattern (see §3.10), each Block publishes its prediction as a Pulse on its `prediction.{name}` topic, subscribes to its `calibration.{name}.updated` error topic, and adjusts. Learning is structural, not a separate subsystem.

---

## 2. The Block Trait

```rust
#[async_trait]
pub trait Block: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &Version;
    fn description(&self) -> &str;
    fn tags(&self) -> &[&str] { &[] }
    fn input_schema(&self) -> &TypeSchema;
    fn output_schema(&self) -> &TypeSchema;
    fn capabilities(&self) -> &[Capability];
    fn protocols(&self) -> &[Protocol];

    fn estimate_cost(&self, input: &BlockInput) -> CostEstimate {
        CostEstimate::unknown()
    }

    async fn run(
        &self,
        input: BlockInput,
        ctx: &BlockContext,
    ) -> Result<BlockOutput, BlockError>;
}
```

### BlockInput / BlockOutput

```rust
pub struct BlockInput {
    pub signals: Vec<Signal>,
    pub macros: MacroBindings,
    pub context: BlockInputContext,
}

pub struct BlockOutput {
    pub signals: Vec<Signal>,
    pub persist: Vec<Signal>,       // Signals to write to Store
    pub metrics: BlockMetrics,
    pub next_state: Option<StateHint>,
}
```

### BlockContext

```rust
pub struct BlockContext {
    pub space: SpaceRef,
    pub run_id: RunId,
    pub graph: GraphRef,
    pub bus: BusHandle,              // publish Pulses
    pub store: StoreHandle,          // persist Signals
    pub model_router: RouterHandle,  // EFE-based model selection
    pub shell: ShellHandle,          // capability-gated
    pub net: NetHandle,
    pub fs: FsHandle,
    pub llm: LlmHandle,
    pub cancel: CancellationToken,
    pub deadline: Option<Instant>,
    pub budget: BudgetTracker,
    pub episode: EpisodeRecorder,
    pub trace: TraceSpan,
    pub regime: Regime,              // current operating regime (Calm/Normal/Volatile/Crisis)
    pub vitality: f64,               // agent vitality scalar
}
```

Handles are gated by declared capabilities. Calling `ctx.net.fetch(...)` from a Block that did not declare `Capability::Net` errors at runtime.

---

## 3. The 9 Protocols

Protocols are interfaces that Blocks optionally implement. A Block can conform to multiple protocols. The runtime dispatches based on protocol conformance.

### 3.1 Store — put / get / query / prune Signals

**Existing trait**: `Substrate`

```rust
pub trait Store: Block {
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    async fn query_similar(&self, fp: &HdcVector, radius: f32, limit: usize) -> Result<Vec<(SignalRef, f32)>>;
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

**Built-in implementations**: FileStore (JSONL), MemoryStore (in-memory), ChainStore (on-chain commitments).

### 3.2 Score — rate Signal along dimensions

**Existing trait**: `Scorer`

```rust
pub trait Score: Block {
    async fn score(&self, signal: &Signal, ctx: &ScoreContext) -> Result<ScoreResult>;
}
```

**Predict-publish-correct**: Scorer predicts 5-axis quality → publishes prediction → gate verdict provides ground truth → calibration error updates per-axis weights via online least-squares.

**Built-in implementations**: LlmScorer, RuleScorer, HdcScorer.

### 3.3 Verify — check Signal against truth → Verdict

**Existing trait**: `Gate` — **significantly extended**

The Verify protocol is load-bearing: it is the reward function, the relabeling oracle, the safety boundary, and the economic attestation. Four learning loops depend on it.

```rust
pub trait Verify: Block {
    /// Pre-action check: can veto execution before it starts.
    async fn verify_pre(&self, signal: &Signal, ctx: &VerifyContext) -> Result<PreVerdict>;

    /// Post-action check: evaluate result against truth criteria.
    async fn verify_post(&self, signal: &Signal, ctx: &VerifyContext) -> Result<Verdict>;

    /// Streaming verification: check Pulses as they arrive.
    async fn verify_stream(&self, pulses: &[Pulse], ctx: &VerifyContext) -> Result<Verdict> {
        // Default: no streaming verification
        Ok(Verdict::default())
    }
}

pub struct PreVerdict {
    pub proceed: bool,               // false = veto execution
    pub reason: Option<String>,
    pub modified_input: Option<Signal>, // optionally transform input
}

pub struct Verdict {
    pub passed: bool,
    pub reward: f64,                 // continuous, domain-specific learning signal
    pub confidence: f64,
    pub findings: Vec<Signal>,       // Finding-kind Signals
    pub evidence: Vec<Signal>,       // Evidence-kind Signals (typed, separate from Criterion)
    pub hard_criteria: Vec<CriterionResult>,  // conjunctive AND
    pub soft_criteria: Vec<CriterionResult>,  // multi-objective Pareto
}

pub struct CriterionResult {
    pub name: String,
    pub passed: bool,
    pub score: f64,
    pub evidence: Vec<Signal>,
}
```

**Key design decisions** (from visual-gate2):

1. **Evidence typing**: `EvidenceCollector` is separate from `Criterion`. Evidence is collected by typed collectors (screenshot, DOM, process output, diff, etc.) and evaluated by criteria independently. A single evidence bag can be evaluated by multiple criteria.

2. **Conjunctive hard + Pareto soft**: Hard criteria are AND — all must pass. Soft criteria are multi-objective Pareto — no weighted sum (Goodhart-resistant). The Verdict carries both.

3. **Pre-action and post-action**: `verify_pre()` can veto execution before it starts (safety boundary). `verify_post()` evaluates results.

4. **Continuous reward**: `Verdict.reward: f64` is a domain-specific learning signal alongside binary pass/fail. Feeds L1 parameter tuning and L2 strategy routing.

5. **Pairwise BT judges**: For LLM-judge gates, fixed-anchor pairwise comparison aggregated via Bradley-Terry MLE with disjoint-family panels (see depth docs for algorithm).

**Built-in implementations**: CompileGate, TestGate, ClippyGate, DiffGate, LlmJudgeGate, ConsensusGate.

### 3.4 Route — select among candidates, learn from outcome

**Existing trait**: `Router` — **extended with EFE and regime conditioning**

```rust
pub trait Route: Block {
    async fn route(
        &self,
        candidates: &[Signal],
        ctx: &RouteContext,
    ) -> Result<RouteResult>;

    async fn feedback(&self, choice: &SignalRef, outcome: &Signal) -> Result<()>;
}

pub struct RouteContext {
    pub regime: Regime,              // Calm / Normal / Volatile / Crisis
    pub budget_remaining: f64,
    pub task_complexity: f64,
    pub domain: String,
    pub vitality: f64,
}

pub struct RouteResult {
    pub selected: SignalRef,
    pub confidence: f64,
    pub reason: String,
    pub efe_score: f64,              // Expected Free Energy
}
```

**EFE replaces LinUCB** for T0/T1/T2 gating and L2 routing. Expected Free Energy naturally balances exploration (epistemic value) and exploitation (pragmatic value) while being cost-aware. Each timescale uses a different free-energy lower bound.

**Regime conditioning**: Route receives `regime: Signal` for context-aware selection. Calm regime → explore. Crisis → exploit cheapest reliable.

**Built-in implementations**: CascadeRouter (EFE bandit), RuleRouter, CostRouter.

### 3.5 Compose — combine Signals under budget → one Signal

**Existing trait**: `Composer` — **extended with VCG and section effects**

```rust
pub trait Compose: Block {
    async fn compose(
        &self,
        signals: &[Signal],
        budget: &ComposeBudget,
        ctx: &ComposeContext,
    ) -> Result<Signal>;
}

pub struct ComposeBudget {
    pub max_tokens: usize,
    pub max_signals: usize,
    pub priority_weights: ScoreWeights,
}

pub struct ComposeContext {
    pub bidders: Vec<Box<dyn ContextBidder>>,  // 8+ bidders
    pub section_effects: SectionEffectMap,       // beta-distribution posteriors
}
```

**VCG auction** with 8+ bidders (Neuro, Task, Research, Heuristic, Episode, Pheromone, Affect, System). Each bidder declares value for token budget. VCG allocates efficiently — pay your externality.

**Section effect tracking**: Beta-distribution posteriors track which context sections correlate with gate success. Sections that historically improve outcomes get more budget. This is learnable context assembly — the system improves at building prompts.

**Built-in implementations**: PromptComposer (9-layer), VcgComposer, GreedyComposer.

### 3.6 React — watch Pulse stream, emit new Signals

**Existing trait**: `Policy` — **breaking change: operates on Pulses**

```rust
pub trait React: Block {
    async fn react(&self, pulses: &[Pulse], ctx: &ReactContext) -> Result<ReactOutput>;
}

pub struct ReactOutput {
    pub pulses: Vec<Pulse>,          // ephemeral reactions
    pub signals: Vec<Signal>,        // durable reactions (graduated)
}
```

React operates on Pulses (ephemeral), not Signals. This is a breaking change from the v1 spec where Policy took `&[Engram]`. The rationale: policies react to live events (heartbeats, gate verdicts, budget warnings), not stored artifacts.

**Built-in implementations**: SafetyReactor, BudgetReactor, EscalationReactor, CalibrationPolicy.

### 3.7 Observe — read-only view, emit observation Signals

**New protocol** (see [doc-09](09-TELEMETRY.md))

```rust
pub trait Observe: Block {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;
    fn observes(&self) -> &[ObservableEventKind];
    fn scope(&self) -> LensScope;
}
```

Lenses never modify what they observe. StateHub projections (typed, universal) are consumed by all surfaces. CollectiveIntelligenceLens computes c-factor.

**Built-in implementations**: CostLens, LatencyLens, QualityLens, EfficiencyLens, ErrorLens, DriftLens, BudgetLens, TrendLens, AnomalyLens, UsageLens, CollectiveIntelligenceLens.

### 3.8 Connect — connect / query / execute / disconnect

**Existing trait**: `Connector`

```rust
pub trait Connect: Block {
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;
    async fn health(&self) -> Result<HealthStatus>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

**Built-in implementations**: ChainRpcConnector, McpConnector, DatabaseConnector, WebhookConnector, ApiConnector.

### 3.9 Trigger — listen for events, fire Graphs

**New protocol** (see [doc-06](06-TRIGGER-SYSTEM.md))

```rust
pub trait Trigger: Block {
    async fn arm(&mut self, binding: &TriggerBinding) -> Result<()>;
    async fn disarm(&mut self) -> Result<()>;
    async fn poll(&self) -> Result<Option<TriggerEvent>>;
}
```

**Built-in implementations**: CronTrigger, WebhookTrigger, FileWatchTrigger, BusTrigger, ChainEventTrigger, ManualTrigger, SignalPatternTrigger.

### 3.10 Predict-Publish-Correct (cross-cutting)

Every protocol supports a calibration loop. This is not a protocol itself — it is a pattern that emerges from Bus + the existing protocols:

```
1. Block O publishes Pulse("prediction.O", y_hat, lineage_hint=x.hash)
2. Reality publishes Pulse("outcome.O", y_true, lineage_hint=x.hash)
3. CalibrationPolicy joins by lineage_hint → Pulse("calibration.O.error", (y_hat, y_true, loss))
4. Block O subscribes to "calibration.O.updated" → updates internal state
```

Per-operator calibration:

| Operator | Predicts | Outcome signal | Update |
|---|---|---|---|
| Scorer | 7-axis reward | Gate verdict + episode reward | Online least-squares per axis |
| Router | selection will succeed | Gate verdict | Contextual bandit (EFE generalized) |
| Composer | prompt fits budget + wins gate | Token count + gate verdict | Section effect beta update |
| Gate | task succeeds post-patch | Next gate verdict | Threshold EMA |
| Policy | decision improves metric | Metric Pulse after decision | Per-policy online calibration |

The `CalibrationPolicy` in `roko-learn` subscribes to all `prediction.**` and `outcome.**` topics, maintains per-operator state, and publishes updates.

---

## 4. TypeSchema

Blocks declare I/O types via TypeSchema — a JSON-Schema-compatible type language.

```rust
pub enum TypeSchema {
    Primitive(PrimitiveType),
    Object { fields: BTreeMap<String, TypeSchema>, required: Vec<String> },
    Array { items: Box<TypeSchema>, min: Option<u32>, max: Option<u32> },
    Enum { variants: Vec<String> },
    Union { variants: Vec<TypeSchema> },
    Ref { name: String, version: Option<Version> },
    Signal { kind: Option<Kind> },
    Tagged { tag: String, inner: Box<TypeSchema> },
}
```

The engine validates types at Graph-load time. Mismatched edges are rejected unless an adapter Block exists.

---

## 5. Capabilities

Capabilities form the security model. A Block declares what it needs; the Space grants or denies.

```rust
pub enum Capability {
    FsRead { paths: Option<Vec<PathPattern>> },
    FsWrite { paths: Option<Vec<PathPattern>> },
    Net { domains: Option<Vec<String>> },
    Shell { commands: Option<Vec<String>> },
    Llm { providers: Option<Vec<String>> },
    Chain { read: bool, write: bool, networks: Option<Vec<String>> },
    Secrets { keys: Option<Vec<String>> },
    KnowledgeRead,
    KnowledgeWrite,
    Process { kind: ProcessKind },
    Custom { name: String, params: Value },
}
```

Three-layer intersection: Block declaration ∩ Graph allow-list ∩ Space grant = effective capabilities. Fails closed.

---

## 6. Implementation Tiers

| Tier | Format | Sandboxing | Distribution |
|---|---|---|---|
| **Prompts** | Markdown/TOML front-matter | None (no execution) | Marketplace |
| **Config** | TOML profile bundles | None | Marketplace |
| **Script** | Bash/Python/Node + manifest | OS-level process isolation | Verified publishers |
| **WASM** | Compiled WASM with Block ABI | WASM sandbox (fuel-metered) | Marketplace (recommended) |
| **Rust** | `impl Block for MyBlock` in crate | Process-level | Compiled into binary |

Progressive capability with progressive isolation. The 5-tier SPI enables external contributions at every level.

---

## 7. Block Lifecycle Events

Every Block execution emits lifecycle Pulses on the Bus:

```rust
pub enum BlockEvent {
    Started { block: BlockRef, run: RunId, input_hash: ContentHash },
    Completed { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    Failed { block: BlockRef, run: RunId, error: BlockError },
    Retried { block: BlockRef, run: RunId, attempt: u32, reason: String },
    Cancelled { block: BlockRef, run: RunId },
}
```

Consumed by Lenses (telemetry), React Blocks (policy), Episode Logger (persistence), and Dashboard (display).

---

## 8. Error Model

```rust
pub enum BlockError {
    InvalidInput { reason: String },
    Timeout { elapsed: Duration },
    CapabilityDenied { needed: Capability },
    External { source: BoxError },
    LogicError { reason: String },
    Cancelled,
    PreVerifyVeto { reason: String },    // verify_pre() vetoed execution
}
```

---

## 9. Protocol Composition

A single Block can implement multiple protocols:

| Pattern | Protocols | Example |
|---|---|---|
| Scoring gate | Score + Verify | LlmJudgeGate: scores quality, then verifies against threshold |
| Learning router | Route + Observe | CascadeRouter: routes models, observes outcomes for feedback |
| Reactive connector | Connect + React | ChainWatcher: connects to RPC, reacts to events |
| Observing store | Store + Observe | InstrumentedStore: stores Signals, emits metrics |
| Pre-verifying reactor | Verify + React | SafetyGate: verify_pre on live Pulses, React on violations |

---

## 10. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Block trait compiles with all methods | `cargo check` |
| All 9 protocol traits compile as supertraits of Block | `cargo check` |
| Verify has verify_pre + verify_post + verify_stream with defaults | Compile check |
| Verdict has reward: f64, hard_criteria, soft_criteria fields | Compile check |
| Route receives RouteContext with regime and vitality | Compile check |
| React takes &[Pulse], returns ReactOutput with pulses + signals | Compile check |
| Compose takes ComposeContext with bidders and section_effects | Compile check |
| TypeSchema validation rejects mismatched Block I/O at Graph-load time | Unit test |
| Capability intersection: Block + Graph + Space must all permit | Test matrix |
| WASM Block runs sandboxed with fuel metering | Integration test |
| Block lifecycle Pulses emitted for start/complete/fail | Integration test |
| Multi-protocol Block dispatches correctly per context | Unit test |
| Predict-publish-correct: prediction Pulse → outcome → calibration update | Integration test |
