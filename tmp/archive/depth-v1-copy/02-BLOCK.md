# 02 — Block

> The universal computation. Every piece of work in Roko implements Block.

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

---

## 2. The Block Trait

```rust
#[async_trait]
pub trait Block: Send + Sync {
    // ── Identity ──────────────────────────────────────────────
    /// Stable identifier across versions. kebab-case.
    fn name(&self) -> &str;

    /// Semver of this Block implementation.
    fn version(&self) -> &Version;

    /// Human-readable description for catalogs and search.
    fn description(&self) -> &str;

    /// Tags for filtering and discovery.
    fn tags(&self) -> &[&str] { &[] }

    // ── Typed I/O ─────────────────────────────────────────────
    /// Type of the input Signal payload.
    fn input_schema(&self) -> &TypeSchema;

    /// Type of the output Signal payload.
    fn output_schema(&self) -> &TypeSchema;

    // ── Capabilities ──────────────────────────────────────────
    /// System resources this Block requires. Engine fails closed when
    /// the Space has not granted a required capability.
    fn capabilities(&self) -> &[Capability];

    // ── Protocol conformance ──────────────────────────────────
    /// Which protocols this Block implements.
    fn protocols(&self) -> &[Protocol];

    // ── Cost estimation ───────────────────────────────────────
    /// Expected cost and duration. Used for budget enforcement and ETA.
    fn estimate_cost(&self, input: &BlockInput) -> CostEstimate {
        CostEstimate::unknown()
    }

    // ── Execution ─────────────────────────────────────────────
    /// Run the Block. The engine handles retry, cancellation,
    /// episode logging, and capability checking around this call.
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
    /// The input Signal(s), schema-validated.
    pub signals: Vec<Signal>,
    /// Resolved Macro values (if inside a Rack).
    pub macros: MacroBindings,
    /// Runtime context (run ID, graph ID, parent Block, etc).
    pub context: BlockInputContext,
}

pub struct BlockOutput {
    /// The output Signal(s), schema-validated against output_schema.
    pub signals: Vec<Signal>,
    /// Any Signals to persist (artifacts, knowledge entries, etc).
    pub persist: Vec<Signal>,
    /// Cost metrics for this invocation.
    pub metrics: BlockMetrics,
    /// Optional hint to the Graph engine for routing.
    pub next_state: Option<StateHint>,
}
```

### BlockContext (runtime provides this)

```rust
pub struct BlockContext {
    pub space: SpaceRef,             // isolation boundary
    pub run_id: RunId,               // current Flow run
    pub graph: GraphRef,             // parent Graph
    pub bus: BusHandle,              // publish ephemeral Signals
    pub store: StoreHandle,          // persist Signals
    pub model_router: RouterHandle,  // select models per role
    pub shell: ShellHandle,          // capability-gated
    pub net: NetHandle,              // capability-gated
    pub fs: FsHandle,               // capability-gated
    pub llm: LlmHandle,             // capability-gated
    pub cancel: CancellationToken,
    pub deadline: Option<Instant>,
    pub budget: BudgetTracker,
    pub episode: EpisodeRecorder,
    pub trace: TraceSpan,
}
```

Handles are gated by the capabilities the Block declared. Calling `ctx.net.fetch(...)` from a Block that did not declare `Capability::Net` errors at runtime.

---

## 3. The 9 Protocols

Protocols are interfaces that Blocks optionally implement. A Block can conform to multiple protocols. The runtime dispatches based on protocol conformance.

### 3.1 Store — put / get / query / prune Signals

**Existing trait**: `Substrate`

```rust
pub trait Store: Block {
    /// Persist a Signal.
    async fn put(&self, signal: Signal) -> Result<SignalRef>;
    /// Retrieve a Signal by ID.
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;
    /// Query Signals matching criteria.
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;
    /// Prune Signals below decay threshold.
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

**Built-in implementations**: FileStore (JSONL), MemoryStore (in-memory), ChainStore (on-chain commitments).

### 3.2 Score — rate Signal along dimensions

**Existing trait**: `Scorer`

```rust
pub trait Score: Block {
    /// Score a Signal along multiple dimensions.
    async fn score(&self, signal: &Signal, ctx: &ScoreContext) -> Result<ScoreResult>;
}

pub struct ScoreResult {
    pub relevance: f64,
    pub quality: f64,
    pub confidence: f64,
    pub novelty: f64,
    pub utility: f64,
}
```

**Built-in implementations**: LlmScorer (model-based), RuleScorer (rule-based), HdcScorer (vector similarity).

### 3.3 Verify — check Signal against truth → Verdict

**Existing trait**: `Gate`

```rust
pub trait Verify: Block {
    /// Verify a Signal against truth criteria.
    async fn verify(&self, signal: &Signal, ctx: &VerifyContext) -> Result<Verdict>;
}

pub struct Verdict {
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<Signal>,    // Finding-kind Signals
    pub evidence: Vec<Signal>,    // Evidence-kind Signals
}
```

**Built-in implementations**: CompileGate, TestGate, ClippyGate, DiffGate, LlmJudgeGate, ConsensusGate.

### 3.4 Route — select among candidates, learn from outcome

**Existing trait**: `Router`

```rust
pub trait Route: Block {
    /// Select the best candidate from a set.
    async fn route(&self, candidates: &[Signal], ctx: &RouteContext) -> Result<RouteResult>;
    /// Feed back outcome for learning.
    async fn feedback(&self, choice: &SignalRef, outcome: &Signal) -> Result<()>;
}

pub struct RouteResult {
    pub selected: SignalRef,
    pub confidence: f64,
    pub reason: String,
}
```

**Built-in implementations**: CascadeRouter (LinUCB bandit), RuleRouter (rule-based), CostRouter (cheapest viable).

### 3.5 Compose — combine Signals under budget → one Signal

**Existing trait**: `Composer`

```rust
pub trait Compose: Block {
    /// Combine Signals into one composite Signal, respecting budget.
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
```

**Built-in implementations**: PromptComposer (system prompt assembly), VcgComposer (VCG auction), GreedyComposer (top-K by score).

### 3.6 React — watch Signal stream, emit new Signals

**Existing trait**: `Policy`

```rust
pub trait React: Block {
    /// React to a Signal, possibly emitting new Signals.
    async fn react(&self, signal: &Signal, ctx: &ReactContext) -> Result<Vec<Signal>>;
}
```

**Built-in implementations**: SafetyReactor (halt on danger), BudgetReactor (alert on threshold), EscalationReactor (notify human).

### 3.7 Observe — read-only view, emit observation Signals

**New protocol** (Lens system, see [doc-09](09-TELEMETRY.md))

```rust
pub trait Observe: Block {
    /// Observe an event (read-only). Emit observation Signals.
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>>;

    /// Which event types this Lens observes.
    fn observes(&self) -> &[ObservableEventKind];

    /// Scope this Lens is attached to (Block, Graph, Agent, Space).
    fn scope(&self) -> LensScope;
}
```

**Built-in implementations**: CostLens, LatencyLens, QualityLens, EfficiencyLens, ErrorLens, DriftLens, BudgetLens, TrendLens, AnomalyLens, UsageLens.

Lenses never modify what they observe. They are pure observers that emit observation Signals onto the Bus.

### 3.8 Connect — connect / query / execute / disconnect

**Existing trait**: `Connector`

```rust
pub trait Connect: Block {
    /// Establish connection to external system.
    async fn connect(&mut self, config: &ConnectConfig) -> Result<()>;
    /// One-shot read query.
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;
    /// Mutating operation.
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;
    /// Health check.
    async fn health(&self) -> Result<HealthStatus>;
    /// Graceful disconnect.
    async fn disconnect(&mut self) -> Result<()>;
}
```

**Built-in implementations**: ChainRpcConnector, McpConnector, DatabaseConnector, WebhookConnector, ApiConnector.

### 3.9 Trigger — listen for events, fire Graphs

**New protocol** (see [doc-06](06-TRIGGER-SYSTEM.md))

```rust
pub trait Trigger: Block {
    /// Start listening for events.
    async fn arm(&mut self, binding: &TriggerBinding) -> Result<()>;
    /// Stop listening.
    async fn disarm(&mut self) -> Result<()>;
    /// Check if trigger condition is met.
    async fn poll(&self) -> Result<Option<TriggerEvent>>;
}
```

**Built-in implementations**: CronTrigger, WebhookTrigger, FileWatchTrigger, BusTrigger, ChainEventTrigger, ManualTrigger, SignalPatternTrigger.

---

## 4. TypeSchema

Blocks declare their I/O types via TypeSchema — a JSON-Schema-compatible type language with workflow-specific extensions.

```rust
pub enum TypeSchema {
    Primitive(PrimitiveType),
    Object { fields: BTreeMap<String, TypeSchema>, required: Vec<String> },
    Array { items: Box<TypeSchema>, min: Option<u32>, max: Option<u32> },
    Enum { variants: Vec<String> },
    Union { variants: Vec<TypeSchema> },
    Ref { name: String, version: Option<Version> },   // named registered type
    Signal { kind: Option<Kind> },                     // Signal of a specific Kind
    Tagged { tag: String, inner: Box<TypeSchema> },    // newtype semantics
}

pub enum PrimitiveType {
    Bool, Int, Float, String, Bytes, DateTime, Duration, Path,
}
```

The engine validates types at Graph-load time. Mismatched edges are rejected unless an adapter Block exists.

### Adapters

When two Blocks need to be wired but their types don't match, the engine looks up an **adapter Block** — a Block tagged `kind = "adapter"` that converts between types. The visual editor auto-inserts adapters where unambiguous and prompts when ambiguous.

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

Capability intersection happens at three layers:

1. **Block declaration** — what it needs
2. **Graph allow-list** — what the embedding Graph permits
3. **Space grant** — what the user has authorized

A Block may run only when all three layers permit. The system fails closed: missing permission = denied.

---

## 6. Implementation Tiers

Blocks can be implemented at different levels of complexity:

| Tier | Format | Sandboxing | Distribution |
|---|---|---|---|
| **Rust** | `impl Block for MyBlock` in crate code | Process-level | Compiled into binary |
| **WASM** | Compiled WASM module with Block interface | WASM sandbox (fuel-metered) | `.wasm` file in Block registry |
| **Script** | Bash / Python / Node with declared capabilities | OS-level process isolation | Script file + manifest.toml |
| **TOML** | Pure composition of other Blocks (a Graph) | Inherits from composed Blocks | `.toml` file |

All tiers present the same `Block` interface to the engine. The runtime adapts invocation:

- **Rust**: direct async call
- **WASM**: wasmtime instantiation with fuel metering
- **Script**: subprocess with capability-gated I/O
- **TOML**: recursive Graph interpretation

---

## 7. Block Lifecycle Events

Every Block execution emits lifecycle events as ephemeral Signals on the Bus:

```rust
pub enum BlockEvent {
    Started { block: BlockRef, run: RunId, input_hash: ContentHash },
    Completed { block: BlockRef, run: RunId, duration: Duration, cost: Cost },
    Failed { block: BlockRef, run: RunId, error: BlockError },
    Retried { block: BlockRef, run: RunId, attempt: u32, reason: String },
    Cancelled { block: BlockRef, run: RunId },
}
```

These events are consumed by:
- **Lenses** (Observe protocol) for telemetry
- **React Blocks** for policy enforcement
- **Episode Logger** for persistence
- **Dashboard** (via WebSocket) for real-time display

---

## 8. Error Model

```rust
pub enum BlockError {
    /// Input doesn't match declared schema.
    InvalidInput { reason: String },
    /// Block exceeded its time budget.
    Timeout { elapsed: Duration },
    /// Block needs a capability the Space hasn't granted.
    CapabilityDenied { needed: Capability },
    /// External system error (network, shell, LLM).
    External { source: BoxError },
    /// Logic error within the Block.
    LogicError { reason: String },
    /// Block was cancelled via CancellationToken.
    Cancelled,
}
```

Errors carry enough structure for the Graph engine to decide retry vs. escalate vs. replan (see [doc-05](05-EXECUTION-ENGINE.md)).

---

## 9. Protocol Composition

A single Block can implement multiple protocols. Common patterns:

| Pattern | Protocols | Example |
|---|---|---|
| Scoring gate | Score + Verify | LlmJudgeGate: scores quality, then verifies against threshold |
| Learning router | Route + Observe | CascadeRouter: routes models, observes outcomes for feedback |
| Reactive connector | Connect + React | ChainWatcher: connects to RPC, reacts to events |
| Observing store | Store + Observe | InstrumentedStore: stores Signals, emits metrics |

The engine dispatches to the correct protocol method based on context:
- In a verification pipeline → calls `verify()`
- In a routing decision → calls `route()`
- When a Lens is attached → calls `observe()`

---

## 10. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Block trait compiles with all methods and defaults | `cargo check` on roko-core |
| All 9 protocol traits compile as supertraits of Block | `cargo check` |
| TypeSchema validation rejects mismatched Block I/O at Graph-load time | Unit test: wire `String → i32`, expect schema error |
| Capability intersection: Block + Graph + Space must all permit | Test matrix: deny at each layer, verify closed |
| WASM Block runs sandboxed with fuel metering | Integration test: WASM Block exceeding fuel is terminated |
| Script Block runs in subprocess with capability gating | Integration test: script requests ungranteed capability, denied |
| Block lifecycle events emitted for start/complete/fail | Integration test: run Block, capture events from Bus |
| Multi-protocol Block dispatches correctly per context | Unit test: Block with Score + Verify, call each |
| Cost estimation returns CostEstimate for Blocks that declare it | Unit test on estimate_cost |
