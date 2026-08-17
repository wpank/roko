# 02 — Cell and Protocols

> The universal computation unit. Signals in, Signals out. Declares typed I/O, capabilities, and protocol conformance. Every Cell is a learner via predict-publish-correct (Friston 2006).

**Subsumes**: Module, Tool, Gate, Router, Composer, Scorer, Policy, Substrate, Connector, Operator.

---

## 1. Cell Trait

A Cell is atomic computation with an identity, typed inputs and outputs, declared capabilities, protocol conformance, and cost estimation. Every first-class primitive in Roko is a Cell or composed of Cells.

```rust
/// The universal computation unit.
///
/// A Cell declares what it consumes, what it produces, what protocols it
/// speaks, and what capabilities it requires. The runtime uses these
/// declarations for type-checking edges, capability intersection, cost
/// budgeting, and protocol dispatch.
pub trait Cell: Send + Sync + 'static {
    /// Stable identifier. Content-addressed from (name, version, author).
    fn id(&self) -> CellId;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Semantic version.
    fn version(&self) -> Version;

    /// Typed input schema. `None` means "accepts any Signal."
    fn input_schema(&self) -> Option<&TypeSchema>;

    /// Typed output schema. `None` means "produces any Signal."
    fn output_schema(&self) -> Option<&TypeSchema>;

    /// Capabilities this Cell requires to run (see S3).
    fn capabilities(&self) -> &Capabilities;

    /// Which of the 9 protocols this Cell conforms to.
    fn protocols(&self) -> &[ProtocolId];

    /// Estimated cost to execute once, in USD-equivalent microcents.
    /// Used by the Route protocol for EFE cost terms and by Compose
    /// for budget-constrained assembly. Returns `None` if cost is
    /// input-dependent and cannot be estimated statically.
    fn estimated_cost(&self) -> Option<Cost>;

    /// Estimated wall-clock time to execute once.
    fn estimated_duration(&self) -> Option<Duration>;

    /// Execute the Cell. Consumes input Signals, produces output Signals.
    /// The runtime supplies a `CellContext` with Bus access, Store handle,
    /// budget remaining, trace context, and cancellation token.
    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError>;
}
```

### CellId

```rust
/// Content-addressed Cell identity.
/// Deterministic: same (name, version, author) -> same CellId.
pub struct CellId(pub ContentHash);

impl CellId {
    pub fn compute(name: &str, version: &Version, author: &Author) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(version.to_string().as_bytes());
        hasher.update(author.to_string().as_bytes());
        Self(ContentHash(hasher.finalize().into()))
    }
}
```

### CellContext

```rust
/// Runtime context provided to every Cell execution.
pub struct CellContext {
    /// Access to Bus for publishing Pulses (predictions, lifecycle events).
    pub bus: Arc<dyn Bus>,

    /// Access to Store for reading/writing Signals.
    pub store: Arc<dyn Store>,

    /// Remaining budget for this execution scope (Graph or Agent).
    pub budget_remaining: Cost,

    /// Distributed trace context for observability.
    pub trace: TraceContext,

    /// Cancellation token — checked between steps.
    pub cancel: CancellationToken,

    /// The Cell's own calibration table (for predict-publish-correct).
    pub calibration: Arc<CalibrationTable>,

    /// Current Agent identity if running inside an Agent scope.
    pub agent: Option<AgentId>,

    /// Run ID if running inside a Flow.
    pub run_id: Option<RunId>,
}
```

---

## 2. The Nine Protocols

Every Cell conforms to one or more of 9 protocols. Each protocol is an `async_trait` with well-typed signatures. All protocols support **predict-publish-correct** (see [S3.10](#310-predict-publish-correct)).

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProtocolId {
    Store,
    Score,
    Verify,
    Route,
    Compose,
    React,
    Observe,
    Connect,
    Trigger,
}
```

---

### 2.1 Store Protocol

Persisted storage of Signals. Defined in [doc-01](01-SIGNAL.md) S11.

```rust
#[async_trait]
pub trait StoreProtocol: Cell {
    /// Persist a Signal, returning its reference.
    async fn put(&self, signal: Signal) -> Result<SignalRef>;

    /// Retrieve by identity.
    async fn get(&self, id: &SignalId) -> Result<Option<Signal>>;

    /// Structured query (kind, tags, time range, score thresholds).
    async fn query(&self, query: StoreQuery) -> Result<Vec<Signal>>;

    /// HDC similarity search. Returns (ref, distance) pairs.
    async fn query_similar(
        &self,
        fingerprint: &HdcVector,
        radius: f32,
        limit: usize,
    ) -> Result<Vec<(SignalRef, f32)>>;

    /// Remove Signals below balance threshold. Returns prune report.
    async fn prune(&self, threshold: f64) -> Result<PruneReport>;
}
```

---

### 2.2 Score Protocol

Rate a Signal along 5 dimensions (relevance, quality, confidence, novelty, utility). Score Cells are learners: they predict quality, publish predictions, and receive corrections from gate verdicts via calibration.

```rust
#[async_trait]
pub trait ScoreProtocol: Cell {
    /// Rate a Signal, producing a 5-dimensional Score.
    async fn score(
        &self,
        signal: &Signal,
        context: &ScoreContext,
    ) -> Result<Score>;
}

/// Context for scoring decisions.
pub struct ScoreContext {
    /// Recent Signals in the same topic for relative scoring.
    pub neighbors: Vec<SignalRef>,

    /// The query or task that prompted this scoring.
    pub query: Option<String>,

    /// HDC fingerprint of the current attention focus.
    pub attention_focus: Option<HdcVector>,
}
```

---

### 2.3 Verify Protocol

The load-bearing protocol. Serves four roles simultaneously: **reward function** (continuous `Verdict.reward: f64`), **relabeling oracle** (hindsight relabeling of failed trajectories), **safety boundary** (`verify_pre()` can veto execution), and **economic attestation** (reputation flows from verified work). See [doc-00](00-INDEX.md) Design Principle 5.

```rust
#[async_trait]
pub trait VerifyProtocol: Cell {
    /// Pre-action check. Called BEFORE a Cell executes.
    /// Returns `Verdict` — if `hard_pass` is false, execution is vetoed.
    async fn verify_pre(
        &self,
        input: &[Signal],
        plan: &ActionPlan,
        ctx: &VerifyContext,
    ) -> Result<Verdict>;

    /// Post-action check. Called AFTER a Cell produces output.
    /// Returns `Verdict` with evidence, reward, and criteria evaluation.
    async fn verify_post(
        &self,
        input: &[Signal],
        output: &[Signal],
        ctx: &VerifyContext,
    ) -> Result<Verdict>;

    /// Streaming check. Called periodically during long-running execution.
    /// Enables early termination if a hard criterion is violated mid-stream.
    async fn verify_stream(
        &self,
        partial_output: &[Signal],
        ctx: &VerifyContext,
    ) -> Result<StreamVerdict>;
}
```

#### Verdict

```rust
/// The output of a Verify check. Contains a continuous reward alongside
/// binary pass/fail, typed evidence, and separate hard/soft criteria.
pub struct Verdict {
    /// Continuous reward signal for learning. Domain-specific scale.
    /// Routing uses this to update EFE estimates. Episode logging records it.
    pub reward: f64,

    /// Conjunctive hard criteria — ALL must pass. Binary AND.
    /// If any hard criterion fails, the Verdict is a hard fail regardless
    /// of soft criteria or reward.
    pub hard_pass: bool,
    pub hard_criteria: Vec<CriterionResult>,

    /// Pareto soft criteria — multi-objective, NEVER weighted-sum.
    /// Soft criteria produce a Pareto front; dominated solutions are
    /// discarded. No single-scalar collapse (Goodhart-resistant).
    pub soft_criteria: Vec<CriterionResult>,

    /// Typed evidence collected during verification.
    /// Evidence is separate from Criterion — a Criterion references
    /// Evidence by kind, but Evidence exists independently.
    pub evidence: Vec<Evidence>,

    /// Wall-clock time spent verifying.
    pub duration: Duration,

    /// Optional explanation for human review.
    pub explanation: Option<String>,
}

pub struct StreamVerdict {
    /// Whether to continue execution.
    pub continue_execution: bool,

    /// Partial verdict (may be updated by verify_post).
    pub partial: Verdict,
}
```

#### Criterion

```rust
/// A single verification criterion. Criteria are either hard (conjunctive)
/// or soft (Pareto). The distinction is structural, not a weight.
pub struct CriterionResult {
    pub criterion: Criterion,
    pub passed: bool,
    pub score: f64,            // 0.0..=1.0 for this specific criterion
    pub evidence_refs: Vec<EvidenceRef>,
}

/// The 19 criterion kinds.
#[non_exhaustive]
pub enum Criterion {
    // ── Correctness ──────────────────────────
    Compiles,
    TestsPassing,
    TypeSafe,
    NoRegressions,

    // ── Quality ──────────────────────────────
    ClippyClean,
    FormattingClean,
    CoverageAbove { threshold: f64 },
    ComplexityBelow { threshold: f64 },
    DiffReasonable { max_lines: usize },

    // ── Safety ───────────────────────────────
    NoSecretLeak,
    PermissionsRespected,
    SandboxIntact,
    InvariantPreserved { invariant: String },

    // ── Economic ─────────────────────────────
    WithinBudget { max_cost: Cost },
    WithinDeadline { max_duration: Duration },

    // ── Semantic ─────────────────────────────
    RelevantToTask,
    ConsistentWithContext,
    NoDuplication,

    // ── Custom ───────────────────────────────
    Custom { name: String, description: String },
}
```

#### Evidence

Evidence is typed separately from Criterion. A Criterion references Evidence but does not own it. This separation enables: reuse of evidence across criteria, independent evidence collection, and evidence aggregation across multiple verification passes.

```rust
/// Typed verification evidence. 19 kinds.
pub struct Evidence {
    pub kind: EvidenceKind,
    pub content: Value,
    pub collected_at: DateTime<Utc>,
    pub collector: CellRef,
}

#[non_exhaustive]
pub enum EvidenceKind {
    CompileOutput,
    TestResult { suite: String, passed: u32, failed: u32 },
    ClippyDiagnostic,
    DiffStats { insertions: u32, deletions: u32, files: u32 },
    CoverageReport { line_pct: f64, branch_pct: f64 },
    RuntimeTrace,
    MemoryProfile,
    SecurityScan,
    LlmJudgment { model: String, prompt_hash: ContentHash },
    HumanReview { reviewer: String },
    BenchmarkResult { metric: String, value: f64, unit: String },
    TypeCheckOutput,
    SandboxLog,
    CostReport { total: Cost, breakdown: Vec<(String, Cost)> },
    SchemaValidation,
    RegressionDiff,
    PermissionAudit,
    InvariantCheck { invariant: String },
    Custom { name: String },
}
```

#### Pairwise Bradley-Terry Judges

For subjective criteria (code quality, relevance, consistency), Verify uses **pairwise comparison** aggregated via Bradley-Terry maximum likelihood estimation (Bradley & Terry 1952). This avoids the well-known instability of absolute Likert-scale LLM judgments.

```rust
/// A pairwise comparison between two candidate outputs.
pub struct PairwiseJudgment {
    pub judge: CellRef,
    pub candidate_a: SignalRef,
    pub candidate_b: SignalRef,
    pub winner: PairwiseWinner,
    pub criterion: Criterion,
    pub confidence: f64,
    pub reasoning: Option<String>,
}

pub enum PairwiseWinner {
    A,
    B,
    Tie,
}

/// Aggregated Bradley-Terry scores for a set of candidates.
pub struct BradleyTerryResult {
    /// Per-candidate strength parameter (log-scale, higher is better).
    pub strengths: Vec<(SignalRef, f64)>,
    /// Convergence achieved (should be < 1e-6 for valid results).
    pub convergence: f64,
    /// Number of comparisons used.
    pub comparisons: usize,
}
```

**Disjoint-family panels**: To prevent correlated errors, judges are drawn from disjoint model families (e.g., one Anthropic, one OpenAI, one open-source). A panel of 3 from different families provides better calibration than 5 from the same family. The **Variance Inequality** mandates that the verifier ensemble is spectrally cleaner (lower variance on ground-truth benchmarks) than the generator — no LLM judging itself.

---

### 2.4 Route Protocol

Select among candidate Cells or models for a given task. Uses **Expected Free Energy** (EFE, Friston 2006) rather than LinUCB (linear upper confidence bound), incorporating both epistemic value (information gain from trying uncertain candidates) and pragmatic value (expected task reward).

```rust
#[async_trait]
pub trait RouteProtocol: Cell {
    /// Select the best candidate for the given context.
    async fn route(
        &self,
        candidates: &[RouteCandidate],
        context: &RouteContext,
    ) -> Result<RouteResult>;
}

/// A routing candidate (Cell, model, or agent).
pub struct RouteCandidate {
    pub id: CellRef,
    pub name: String,
    pub estimated_cost: Cost,
    pub estimated_duration: Duration,
    pub capabilities: Capabilities,
    /// Historical EFE components for this candidate.
    pub history: Option<CandidateHistory>,
}

pub struct CandidateHistory {
    pub trials: u32,
    pub mean_reward: f64,
    pub reward_variance: f64,
    pub mean_cost: Cost,
}

/// Context for routing decisions. Includes regime awareness.
pub struct RouteContext {
    /// Current operating regime, derived from system telemetry.
    /// Affects risk tolerance and exploration rate.
    pub regime: Regime,

    /// Agent vitality (remaining_budget / initial_budget).
    /// Low vitality → conservative routing (exploit over explore).
    pub vitality: f64,

    /// Task complexity estimate (0.0..=1.0).
    pub complexity: f64,

    /// Time pressure (0.0 = no deadline, 1.0 = deadline imminent).
    pub urgency: f64,

    /// Budget remaining for this scope.
    pub budget_remaining: Cost,

    /// Signals providing context for the routing decision.
    pub context_signals: Vec<SignalRef>,
}

/// Four operating regimes, derived from system telemetry.
/// Each regime implies different exploration/exploitation tradeoffs.
pub enum Regime {
    /// Low failure rate, stable throughput. Explore freely.
    Calm,
    /// Baseline operation. Standard EFE balance.
    Normal,
    /// Elevated failure rate or resource pressure. Reduce exploration.
    Volatile,
    /// Critical failures or near-budget-exhaustion. Pure exploitation.
    Crisis,
}

pub struct RouteResult {
    /// Selected candidate.
    pub selected: CellRef,

    /// EFE score (lower is better — minimizing free energy).
    /// Negative = confident good outcome. Positive = uncertain/costly.
    pub efe_score: f64,

    /// Decomposition for observability.
    pub pragmatic_value: f64,   // expected reward
    pub epistemic_value: f64,   // information gain
    pub cost_term: f64,         // economic cost penalty

    /// Runner-up candidates for diversity (used by Replan on failure).
    pub alternatives: Vec<(CellRef, f64)>,
}
```

**Why EFE instead of LinUCB**: LinUCB (Li et al. 2010) is a contextual bandit that balances exploration/exploitation via an upper confidence bound on linear reward predictions. EFE (Friston 2006, active inference) subsumes this by also modeling the *value of reducing uncertainty* — the epistemic term. An agent that has never tried a cheap model on simple tasks has high epistemic value for that pairing, even if the pragmatic expectation is uncertain. EFE naturally produces the progressive cascade (T0 pattern-match -> T1 cheap model -> T2 expensive model) without hand-coded tier thresholds, because trying cheaper options first has high epistemic value until their performance bounds are learned.

---

### 2.5 Compose Protocol

Assemble multiple Signals into a single output Signal under a budget constraint. Uses a **VCG auction** (Vickrey 1961, Clarke 1971, Groves 1973) with 8+ context bidders. Each bidder proposes sections with stated value; the auction selects the budget-feasible subset that maximizes total stated value, charging each bidder their externality cost (ensuring truthful bidding is the dominant strategy).

```rust
#[async_trait]
pub trait ComposeProtocol: Cell {
    /// Assemble Signals into a composed output under budget.
    async fn compose(
        &self,
        bids: Vec<ComposeBid>,
        budget: &ComposeBudget,
        ctx: &ComposeContext,
    ) -> Result<ComposeResult>;
}

/// A bid from a context bidder for inclusion in the composed output.
pub struct ComposeBid {
    /// Which bidder is bidding.
    pub bidder: BidderId,

    /// The section being offered.
    pub section: ComposeSection,

    /// Stated value (truthful under VCG — lying reduces payoff).
    pub value: f64,

    /// Token cost of including this section.
    pub token_cost: u32,

    /// Section effect posterior: beta distribution tracking correlation
    /// between including this section and downstream gate success.
    pub effect: BetaPosterior,
}

pub struct ComposeSection {
    pub name: String,
    pub content: String,
    pub kind: ComposeSectionKind,
    pub source_signals: Vec<SignalRef>,
}

pub enum ComposeSectionKind {
    SystemInstruction,
    TaskDescription,
    CodeContext,
    ResearchContext,
    EpisodeHistory,
    HeuristicGuidance,
    ToolDocumentation,
    SafetyConstraint,
    Custom(String),
}

/// Budget constraint for composition.
pub struct ComposeBudget {
    /// Maximum tokens in composed output.
    pub max_tokens: u32,

    /// Maximum USD-equivalent cost.
    pub max_cost: Cost,

    /// Minimum sections that MUST be included (safety, task description).
    pub required_sections: Vec<ComposeSectionKind>,
}

/// Section effect tracking via beta-distribution posteriors.
/// Updated after each gate verdict: alpha += 1 on gate pass, beta += 1 on fail.
pub struct BetaPosterior {
    pub alpha: f64,   // successes + prior
    pub beta: f64,    // failures + prior
}

impl BetaPosterior {
    /// Expected value: alpha / (alpha + beta).
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Variance: captures uncertainty. High variance = few observations.
    pub fn variance(&self) -> f64 {
        (self.alpha * self.beta)
            / ((self.alpha + self.beta).powi(2) * (self.alpha + self.beta + 1.0))
    }
}

pub struct ComposeResult {
    /// The assembled output Signal.
    pub composed: Signal,

    /// Which bids were accepted.
    pub accepted: Vec<BidderId>,

    /// VCG payments (externality cost each bidder is charged).
    pub payments: Vec<(BidderId, f64)>,

    /// Total token usage.
    pub total_tokens: u32,

    /// Total cost.
    pub total_cost: Cost,
}
```

**8+ context bidders** (the system ships these; extensions can register more):

| Bidder | What it bids | Source |
|---|---|---|
| `TaskBidder` | Task description, acceptance criteria | Current plan task |
| `CodeBidder` | Relevant code files, definitions | roko-index / tree-sitter |
| `ResearchBidder` | Research findings, citations | roko-research artifacts |
| `EpisodeBidder` | Prior episodes on similar tasks | roko-learn episodes |
| `HeuristicBidder` | Relevant heuristics with calibration | roko-neuro |
| `ToolBidder` | Tool documentation for enabled tools | MCP tool manifests |
| `SafetyBidder` | Safety constraints, contract terms | Agent contract YAML |
| `NeuroBidder` | Distilled knowledge entries | roko-neuro knowledge store |

**Why VCG instead of heuristic ordering**: Heuristic approaches (fixed section ordering, manual weights) fail as the number of context sources grows — you cannot manually tune weights for 8+ bidders across heterogeneous tasks. VCG's incentive compatibility ensures bidders report truthful values without central coordination. The auction naturally adapts to budget pressure: under tight budgets, only the highest-value sections survive. Section effect tracking (beta posteriors updated from gate verdicts) provides the feedback signal for bidders to improve their valuations over time.

**Novelty attenuation**: Sections that appear frequently across compositions receive attenuated value: `effective_value = stated_value * (1 / (1 + ln(freq)))`. This prevents habituation — common boilerplate gradually loses its bid strength, making room for novel context.

---

### 2.6 React Protocol

Watch the Pulse stream and emit Signals and/or Pulses in response. **Breaking change from v1**: React operates on Pulses (ephemeral), not Signals. This is the correct separation — React handles real-time event processing; Store handles durable data.

```rust
#[async_trait]
pub trait ReactProtocol: Cell {
    /// Process incoming Pulses and produce reactions.
    ///
    /// A React Cell subscribes to a set of Bus topics and processes
    /// matching Pulses. It can emit new Pulses (for downstream React
    /// Cells or coordination) and/or graduate results to Signals
    /// (for durable persistence).
    async fn react(
        &self,
        pulses: &[Pulse],
        ctx: &ReactContext,
    ) -> Result<ReactOutput>;

    /// Topic filter — which Pulses this React Cell wants to see.
    fn subscription(&self) -> TopicFilter;
}

pub struct ReactContext {
    pub bus: Arc<dyn Bus>,
    pub store: Arc<dyn Store>,
    pub agent: Option<AgentId>,
    pub cancel: CancellationToken,
}

/// React output: Pulses for ephemeral transport, Signals for durable storage.
pub struct ReactOutput {
    /// New Pulses to publish on Bus.
    pub pulses: Vec<Pulse>,

    /// Signals to persist in Store (graduated reactions).
    pub signals: Vec<Signal>,
}
```

---

### 2.7 Observe Protocol

Read-only observation producing observation Signals. Used by Lenses ([doc-04](04-SPECIALIZATIONS.md)) for telemetry, StateHub projections, and c-factor computation.

```rust
#[async_trait]
pub trait ObserveProtocol: Cell {
    /// Observe the current state and produce observation Signals.
    /// Read-only: Observe Cells MUST NOT mutate state.
    async fn observe(&self, ctx: &ObserveContext) -> Result<Vec<Signal>>;
}

pub struct ObserveContext {
    pub bus: Arc<dyn Bus>,
    pub store: Arc<dyn Store>,
    pub query: Option<String>,
    pub time_range: Option<TimeRange>,
}
```

---

### 2.8 Connect Protocol

Lifecycle-managed connection to external systems.

```rust
#[async_trait]
pub trait ConnectProtocol: Cell {
    /// Establish connection.
    async fn connect(&self, config: &ConnectConfig) -> Result<ConnectionHandle>;

    /// Query the connected system.
    async fn query(&self, handle: &ConnectionHandle, query: Value) -> Result<Value>;

    /// Execute a command against the connected system.
    async fn execute(&self, handle: &ConnectionHandle, command: Value) -> Result<Value>;

    /// Gracefully disconnect.
    async fn disconnect(&self, handle: ConnectionHandle) -> Result<()>;

    /// Health check.
    async fn health(&self, handle: &ConnectionHandle) -> Result<ConnectionHealth>;
}

pub struct ConnectionHandle {
    pub id: ConnectionId,
    pub protocol: String,
    pub connected_at: DateTime<Utc>,
}

pub enum ConnectionHealth {
    Healthy,
    Degraded { reason: String },
    Disconnected,
}
```

---

### 2.9 Trigger Protocol

Listen for events and fire Graphs. Fully specified in [doc-06](06-TRIGGER-SYSTEM.md).

```rust
#[async_trait]
pub trait TriggerProtocol: Cell {
    /// Arm the trigger. Begins listening for matching events.
    async fn arm(&self, binding: &TriggerBinding) -> Result<TriggerHandle>;

    /// Disarm the trigger. Stops listening.
    async fn disarm(&self, handle: TriggerHandle) -> Result<()>;

    /// Poll for pending trigger events (non-blocking).
    async fn poll(&self, handle: &TriggerHandle) -> Result<Vec<TriggerEvent>>;
}
```

---

## 3. Cross-Cutting Concerns

### 3.1 TypeSchema

Every Cell declares its input and output schema. TypeSchema enables compile-time edge validation in Graphs ([doc-03](03-GRAPH.md)) — mismatched types are caught when the Graph is loaded, not at runtime.

```rust
/// Structural type schema for Cell I/O.
pub enum TypeSchema {
    /// Any Signal (no constraint).
    Any,

    /// Signal must have this Kind.
    OfKind(Kind),

    /// Signal payload must match this JSON Schema.
    JsonSchema(serde_json::Value),

    /// One of several accepted schemas.
    OneOf(Vec<TypeSchema>),

    /// All schemas must match simultaneously.
    AllOf(Vec<TypeSchema>),

    /// Array of a specific schema.
    ArrayOf(Box<TypeSchema>),

    /// Named record with typed fields.
    Record(BTreeMap<String, TypeSchema>),
}

impl TypeSchema {
    /// Check whether an output schema is compatible with an input schema.
    /// Used during Graph validation to verify edges.
    pub fn is_compatible(&self, other: &TypeSchema) -> bool {
        match (self, other) {
            (_, TypeSchema::Any) => true,
            (TypeSchema::Any, _) => false, // Any output may not satisfy a specific input
            (TypeSchema::OfKind(a), TypeSchema::OfKind(b)) => a == b,
            (TypeSchema::OneOf(variants), target) => {
                variants.iter().all(|v| v.is_compatible(target))
            }
            (source, TypeSchema::OneOf(variants)) => {
                variants.iter().any(|v| source.is_compatible(v))
            }
            _ => false, // conservative: reject unproven compatibility
        }
    }
}
```

### 3.2 Capabilities — Three-Layer Intersection

Capabilities are fail-closed: a Cell only runs if the intersection of three capability sets is non-empty. This prevents capability escalation through composition.

```rust
/// Capabilities form three layers. The effective capability set is the
/// intersection of all three. This is fail-closed: if any layer denies
/// a capability, it is denied regardless of the other two.
pub struct Capabilities {
    /// What the Cell's code can do (declared by the Cell author).
    pub declared: CapabilitySet,

    /// What the Agent's contract allows (declared by the Agent operator).
    pub granted: CapabilitySet,

    /// What the Space's policy permits (declared by the Space owner).
    pub permitted: CapabilitySet,
}

impl Capabilities {
    /// Effective capabilities = intersection of all three layers.
    pub fn effective(&self) -> CapabilitySet {
        self.declared
            .intersection(&self.granted)
            .intersection(&self.permitted)
    }
}

#[derive(Clone, Debug)]
pub struct CapabilitySet(BTreeSet<Capability>);

#[non_exhaustive]
pub enum Capability {
    ReadFile,
    WriteFile,
    Execute { sandbox: SandboxLevel },
    Network { allow_list: Vec<String> },
    LlmCall { models: Vec<String> },
    StoreRead,
    StoreWrite,
    BusPublish { topics: Vec<TopicFilter> },
    BusSubscribe { topics: Vec<TopicFilter> },
    HumanEscalation,
    SpawnAgent,
    ModifyPlan,
    Custom(String),
}

pub enum SandboxLevel {
    None,       // no shell access
    Readonly,   // read-only shell
    Sandboxed,  // sandboxed shell (default)
    Full,       // unrestricted (requires explicit grant)
}
```

### 3.3 Five Implementation Tiers

Cells range from zero-code configuration to native Rust, forming a **Spectral Package Interface** (SPI). Higher tiers unlock more capability but require more expertise. Lower tiers are accessible to non-programmers.

| Tier | Name | Cell Defined By | Capabilities | Target Audience |
|---|---|---|---|---|
| T0 | Prompts | System prompt text + role config | LlmCall only | Domain experts |
| T1 | Config | TOML parameters on existing Cells | Varies by base Cell | Power users |
| T2 | Declarative Tools | JSON/TOML tool manifests + MCP | Tool execution | Developers |
| T3 | WASM | Compiled WASM module (any source language) | Sandboxed compute | Plugin developers |
| T4 | Native Rust | `impl Cell for MyCell` | Full capability set | Core developers |

```toml
# T0 example: a code-review Cell defined entirely by prompt
[block.code-reviewer]
tier = "prompt"
system_prompt = """
You are a senior code reviewer. Focus on:
1. Correctness
2. Readability
3. Performance
Provide specific, actionable feedback.
"""
input_schema = { kind = "Diff" }
output_schema = { kind = "Markdown" }
protocols = ["score"]
```

```toml
# T1 example: configuring an existing Route Cell
[block.conservative-router]
tier = "config"
base = "builtin://efe-router"
[block.conservative-router.params]
exploration_rate = 0.05
regime_override = "volatile"
max_cost_per_call = 0.10
```

### 3.4 Cell Lifecycle Events as Pulses

Every Cell execution emits lifecycle Pulses on Bus. These are the sole source of runtime observability — there is no separate telemetry channel.

| Event | Topic | Graduates? |
|---|---|---|
| Cell execution started | `block.{id}.started` | No (noise) |
| Cell execution completed | `block.{id}.completed` | Batch (per-run summary) |
| Cell execution failed | `block.{id}.failed` | Yes (forensic) |
| Cell prediction published | `prediction.{id}` | No (calibration input) |
| Cell cost charged | `cost.charged` | Yes (accounting) |
| Cell capability denied | `safety.capability.denied` | Yes (audit) |

---

## 3.5 Protocol Conformance Declaration

A Cell declares which protocols it conforms to. The runtime checks conformance at registration time (not at call time — fail early).

```rust
/// Registered Cell with verified protocol conformance.
pub struct RegisteredCell {
    pub block: Arc<dyn Cell>,
    pub protocols: Vec<ProtocolConformance>,
}

pub struct ProtocolConformance {
    pub protocol: ProtocolId,
    /// Version of the protocol this Cell was built against.
    pub version: Version,
    /// Whether conformance was verified (tests passed at registration).
    pub verified: bool,
}
```

---

### 3.6 Cost Estimation

Every Cell provides optional cost and duration estimates. These feed into Route (EFE cost term), Compose (budget-constrained assembly), and the execution engine ([doc-05](05-EXECUTION-ENGINE.md)) for budget enforcement.

```rust
/// Cost in USD-equivalent microcents (1 microcent = $0.00000001).
/// Using integer arithmetic avoids floating-point rounding in accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cost(pub u64);

impl Cost {
    pub const ZERO: Cost = Cost(0);

    pub fn from_usd(dollars: f64) -> Self {
        Cost((dollars * 100_000_000.0) as u64)
    }

    pub fn to_usd(&self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }
}
```

---

### 3.7 CellError

```rust
pub enum CellError {
    /// Input Signals did not match the declared input schema.
    SchemaViolation { expected: TypeSchema, got: Kind },

    /// A required capability was denied.
    CapabilityDenied { capability: Capability, layer: String },

    /// Budget exhausted before completion.
    BudgetExhausted { spent: Cost, limit: Cost },

    /// Execution timed out.
    Timeout { elapsed: Duration, limit: Duration },

    /// Cancelled via CancellationToken.
    Cancelled,

    /// Upstream Cell failed (propagated in Graphs).
    UpstreamFailure { block: CellRef, error: Box<CellError> },

    /// Pre-action Verify vetoed execution.
    PreVerifyVeto { verdict: Verdict },

    /// Internal error.
    Internal(anyhow::Error),
}
```

---

### 3.8 CellRef

```rust
/// Lightweight reference to a Cell (used in Graphs, Verdicts, Evidence).
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CellRef {
    pub id: CellId,
    pub name: String,
    pub version: Version,
}
```

---

### 3.9 Protocol Composition

A single Cell can implement multiple protocols. For example, a `CodeReviewCell` might implement both `ScoreProtocol` (rate code quality) and `VerifyProtocol` (check for regressions). The runtime dispatches based on the protocol being invoked, not the Cell type.

```rust
// Example: a Cell implementing two protocols.
pub struct CodeReviewCell { /* ... */ }

impl Cell for CodeReviewCell {
    fn protocols(&self) -> &[ProtocolId] {
        &[ProtocolId::Score, ProtocolId::Verify]
    }
    // ...
}

#[async_trait]
impl ScoreProtocol for CodeReviewCell {
    async fn score(&self, signal: &Signal, ctx: &ScoreContext) -> Result<Score> {
        // Rate code quality on 5 dimensions
        todo!()
    }
}

#[async_trait]
impl VerifyProtocol for CodeReviewCell {
    async fn verify_pre(&self, _input: &[Signal], _plan: &ActionPlan, _ctx: &VerifyContext) -> Result<Verdict> {
        // Pre-check: does the diff look reasonable?
        todo!()
    }

    async fn verify_post(&self, input: &[Signal], output: &[Signal], ctx: &VerifyContext) -> Result<Verdict> {
        // Post-check: compile, test, clippy
        todo!()
    }

    async fn verify_stream(&self, _partial: &[Signal], _ctx: &VerifyContext) -> Result<StreamVerdict> {
        // No streaming check for code review
        Ok(StreamVerdict { continue_execution: true, partial: Verdict::default() })
    }
}
```

---

### 3.10 Predict-Publish-Correct

Every operator is a learner. This is the structural learning pattern that makes all 9 protocols self-improving (Friston 2006, active inference made structural).

**The pattern**:
1. Before acting, the Cell publishes a **prediction** Pulse on `prediction.{block_id}` — what it expects the outcome to be.
2. The Cell executes and produces output.
3. Reality (gate verdicts, downstream results) publishes an **outcome** Pulse on `outcome.{block_id}`.
4. A `CalibrationPolicy` (in `roko-learn`) subscribes to both topics, joins by `lineage_hint`, computes error, and publishes an **update** Pulse on `calibration.{block_id}.updated`.
5. The Cell subscribes to its own calibration topic and adjusts its internal parameters.

```rust
/// Per-operator calibration table. Updated by CalibrationPolicy.
pub struct CalibrationTable {
    /// Running mean prediction error.
    pub mean_error: f64,

    /// Running variance of prediction error.
    pub error_variance: f64,

    /// Number of calibration updates received.
    pub updates: u64,

    /// Per-context-key error rates (e.g., per task type, per model).
    pub context_errors: BTreeMap<String, f64>,

    /// Last update timestamp.
    pub last_updated: DateTime<Utc>,
}

/// Published by CalibrationPolicy after joining prediction + outcome.
pub struct CalibrationUpdate {
    pub block_id: CellRef,
    pub prediction: Value,
    pub outcome: Value,
    pub error: f64,
    pub context_key: Option<String>,
}
```

**Why predict-publish-correct instead of a learning subsystem**: Learning is not a separate module bolted onto execution — it emerges from the same pub/sub fabric that carries heartbeats and gate verdicts. Every Cell improves by construction. The CalibrationPolicy is itself a React Cell subscribing to prediction/outcome topics. The system learns using its own primitives, not bespoke infrastructure (Design Principle 7: elegance through composition).

---

### 3.11 TOML Registration

Cells can be registered via TOML for tiers T0-T2 (prompts, config, declarative tools). The runtime discovers and registers Cells from the workspace configuration.

```toml
# roko.toml — Cell registration
[[blocks]]
name = "code-reviewer"
tier = "prompt"
version = "0.1.0"
protocols = ["score", "verify"]
system_prompt = "..."

[[blocks]]
name = "conservative-router"
tier = "config"
version = "0.1.0"
base = "builtin://efe-router"
[blocks.params]
exploration_rate = 0.05

[[blocks]]
name = "github-connector"
tier = "declarative"
version = "0.1.0"
protocols = ["connect"]
mcp_server = "github"
```

---

## 4. Citations

| Concept | Citation |
|---|---|
| Active inference, free energy principle | Friston, K. (2006). A free energy principle for the brain. *Journal of Physiology-Paris*, 100(1-3), 70-87. |
| VCG auction mechanism | Vickrey, W. (1961). Counterspeculation, auctions, and competitive sealed tenders. *Journal of Finance*, 16(1), 8-37. Clarke, E. (1971). Multipart pricing of public goods. *Public Choice*, 11, 17-33. Groves, T. (1973). Incentives in teams. *Econometrica*, 41(4), 617-631. |
| Bradley-Terry pairwise comparison | Bradley, R. A., & Terry, M. E. (1952). Rank analysis of incomplete block designs: I. The method of paired comparisons. *Biometrika*, 39(3/4), 324-345. |
| Expected Free Energy for routing | Friston, K. et al. (2015). Active inference and epistemic value. *Cognitive Neuroscience*, 6(4), 187-214. |
| Prospect theory (somatic markers) | Kahneman, D., & Tversky, A. (1979). Prospect theory: An analysis of decision under risk. *Econometrica*, 47(2), 263-292. |
| Hyperdimensional computing | Kanerva, P. (2009). Hyperdimensional computing: An introduction to computing in distributed representation with high-dimensional random vectors. *Cognitive Computation*, 1(2), 139-159. |
| Goodhart's Law and weighted-sum failure | Goodhart, C. A. E. (1984). Problems of monetary management: The UK experience. *Monetary Theory and Practice*, 91-121. |
| LinUCB (replaced by EFE) | Li, L. et al. (2010). A contextual-bandit approach to personalized news article recommendation. *WWW 2010*, 661-670. |

---

## 5. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `Cell` trait compiles with `id`, `input_schema`, `output_schema`, `capabilities`, `protocols`, `estimated_cost`, `execute` | Compile check |
| All 9 protocol traits compile with full type signatures | Compile check |
| `Verdict` has `reward: f64`, `hard_pass: bool`, `hard_criteria`, `soft_criteria`, `evidence` | Compile check |
| Hard criteria are conjunctive: single hard fail -> overall fail | Unit test |
| Soft criteria produce Pareto front, not weighted sum | Unit test with 3+ criteria showing non-dominated set |
| Evidence is typed separately from Criterion (19 `EvidenceKind` variants) | Compile check |
| `PairwiseJudgment` and `BradleyTerryResult` compile | Compile check |
| Route uses EFE: `pragmatic_value + epistemic_value + cost_term` | Unit test showing exploration of uncertain candidates |
| `RouteContext` includes `regime: Regime` and `vitality: f64` | Compile check |
| Compose uses VCG auction with `ComposeBid.value` and `ComposeBudget` | Unit test: truthful bidding dominates lying |
| Section effect tracked via `BetaPosterior` updated from gate verdicts | Unit test: alpha increments on pass, beta on fail |
| 8 built-in bidders registered | Integration test |
| React operates on `Pulse` (not `Signal`), returns `ReactOutput { pulses, signals }` | Compile check |
| `TypeSchema::is_compatible` rejects mismatched edges | Unit test |
| Capabilities three-layer intersection: denied in any layer -> denied overall | Unit test |
| 5 implementation tiers: T0 prompt Cell loads from TOML | Integration test |
| Cell lifecycle events published as Pulses on Bus | Integration test: execute Cell, verify Pulses received |
| `CalibrationTable` receives updates from `CalibrationPolicy` | Integration test with mock prediction + outcome |
| Predict-publish-correct: prediction Pulse -> outcome Pulse -> calibration update Pulse | Integration test on Bus |
| `Cost` integer arithmetic: no floating-point rounding in accounting | Unit test: `Cost::from_usd(0.01).to_usd() == 0.01` |
| `CellError::PreVerifyVeto` carries the `Verdict` that vetoed | Compile check |
| Protocol composition: Cell implementing 2+ protocols dispatches correctly | Integration test |
| Disjoint-family panels: 3 judges from different families > 5 from same family (calibration test) | Evaluation test |
