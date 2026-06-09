# Advanced Design Patterns for Exponential Returns

12 architectural patterns that create multiplicative value when applied to roko's adapter-first
architecture. Each pattern is proven in production systems, maps to concrete roko subsystems,
and compounds with other patterns. Updated with April 2026 competitive landscape data and
roko-specific differentiation analysis.

Last updated: 2026-04-29.

---

## April 2026 Context: Why Architecture Patterns Matter Now

The AI coding tool market is at an inflection point where architectural decisions determine
which products survive the next 24 months:

| Product | Architecture | Structural Limitation |
|---|---|---|
| **Cursor** ($2B ARR, $29-60B val) | Monolithic IDE fork | Single-vendor dependency (Anthropic). Fortune reported "very uncertain future" due to supply chain risk. |
| **Codex CLI** (75K stars, 3M WAU) | Monolithic Rust binary | OpenAI-locked auth. Cannot route to Claude, Gemini, or local models without forking. |
| **Devin** ($25B valuation talks) | Closed-source cloud | $500/month, no self-hosting. Proprietary model. |
| **Claude Code** ($2.5B est. revenue) | Anthropic-only CLI | Max plan throttling since March 2026. No multi-provider routing. |
| **Windsurf** (acquired ~$250M) | Absorbed into Cognition | Product direction controlled by acquirer. |
| **Roko** (18 crates, 177K LOC) | Adapter-trait composition | Each pattern below addresses a structural limitation the competitors cannot fix without rewriting. |

The 12 patterns are not aspirational architecture. They are the structural mechanisms that
convert roko's crate-based modularity into compound competitive advantages. Each pattern has
a "why now" tied to specific April 2026 market conditions.

---

## Pattern 1: Event Sourcing + CQRS

**What it is**: Store every state change as an immutable event. Reconstruct current state by
replaying events. Separate read models (queries) from write models (commands).

**Real-world**: Kafka (200K+ orgs), EventStoreDB, Axon Framework, Datomic. Banking systems
that must reconstruct any historical state. Git itself is event-sourced (commits are events).

**Why now (April 2026)**: EU AI Act Article 50 enforcement begins August 2, 2026. Article 50
requires transparency obligations for AI systems -- the ability to reconstruct any decision
chain. Event sourcing is the architectural pattern that makes compliance possible without
bolting on a separate audit system. No competing agent tool has this built-in.

**Roko application**:

Every agent action becomes an immutable event:
```rust
#[derive(Serialize, Deserialize)]
pub enum AgentEvent {
    TaskStarted { task_id: String, agent_role: String, model: String },
    ToolCalled { tool: String, args: Value, result: ToolResult },
    GateEvaluated { rung: String, passed: bool, details: Value },
    FileModified { path: PathBuf, diff: String },
    PlanRevised { reason: String, new_tasks: Vec<TaskId> },
    ModelRouted { context: String, selected: String, score: f64 },
    KnowledgeExtracted { tier: u8, content: String, source: String },
}
```

**Multiplicative value**:
- **Time-travel debugging**: replay any agent execution step-by-step -- critical when an
  agent produces incorrect code and you need to understand why
- **Branching execution**: fork from any point, try different strategies -- the simulation
  capability that none of Cursor/Codex/Devin offer
- **Audit trail**: complete provenance for every code change -- EU AI Act compliance evidence
- **Derived views**: same events produce dashboard view, learning view, billing view,
  compliance view -- one write path, unlimited read paths
- **Cross-agent learning**: Agent B replays Agent A's events to learn strategies -- the
  foundation for roko's 4 compounding learning loops

**Competitive delta**: Cursor has no event log. Codex CLI logs to stdout. Devin's logs are
proprietary and inaccessible. Claude Code streams events but does not persist or replay them.
Roko's EpisodeLogger already captures structured events to `.roko/episodes.jsonl` -- the
foundation exists.

**Compounds with**: Content-Addressable Storage (events reference CAS hashes),
Federated Learning (events are the unit of sharing), Incremental Computation (events
trigger recomputation).

---

## Pattern 2: Capability-Based Security

**What it is**: Instead of asking "who are you?" (identity-based), ask "what token do you
hold?" (capability-based). Capabilities are unforgeable tokens granting specific permissions.
They can be attenuated (restricted) but never amplified.

**Real-world**: UCAN (Fission, IPFS ecosystem), WASI capabilities, Macaroons (Google),
Plan 9 (everything is a file descriptor = capability), CloudFlare Workers (capability-scoped
API tokens), Deno permissions model.

**Why now (April 2026)**: The Shai-Hulud npm supply-chain worm (September-November 2025)
turned willingness-to-pay for agent permission controls from theoretical to post-incident.
Teams are actively asking "what can this agent access?" Codex CLI runs in a Docker sandbox
with network disabled -- a blunt instrument. Roko's capability model is fine-grained.

**Roko application**:

Agent capabilities as attenuation chains:
```rust
pub struct AgentCapability {
    /// What this capability grants
    pub action: Action,        // ReadFile, WriteFile, RunTool, CallModel, ...
    /// Scope restriction
    pub scope: Scope,          // paths, models, tools, budget
    /// Attenuation chain (who delegated this)
    pub chain: Vec<CapabilityId>,
    /// Expiry
    pub expires: Option<SystemTime>,
    /// Budget ceiling
    pub budget: Option<TokenBudget>,
}

pub enum Action {
    ReadFile { glob: String },
    WriteFile { glob: String },
    RunTool { tool_name: String },
    CallModel { model_pattern: String },
    SpawnAgent { role_pattern: String },
    AccessKnowledge { tier_max: u8 },
}
```

**Multiplicative value**:
- **Principle of least privilege**: agents get exactly the permissions they need, not root
- **Delegation without escalation**: Implementer delegates read-only to sub-agent
- **Community plugins safe by default**: WASM plugins receive only granted capabilities
- **Composable permissions**: combine capabilities with set operations
- **Enterprise compliance**: capability chains = SOC 2 audit trail

**Competitive delta**: Codex CLI uses Docker sandbox (binary: full access or no access).
Claude Code uses permission prompts (user fatigue). Cursor runs in-process with full IDE
access. Devin has a cloud sandbox with opaque permissions. Roko's capability model provides
fine-grained, auditable, delegatable permissions -- the enterprise requirement.

**Compounds with**: Adapter traits (each adapter surface = capability boundary),
Marketplace (plugins declare required capabilities), Event Sourcing (capability
grants/revocations are events), Sigstore/in-toto (capability chains are attestable).

---

## Pattern 3: Content-Addressable Storage (CAS)

**What it is**: Store data by its hash. Same content = same address, always. Enables
deduplication, caching, integrity verification, and distributed storage without coordination.

**Real-world**: Git (all objects are content-addressed), IPFS, Nix/Guix (reproducible builds),
Docker image layers, Bazel (build cache), Unison (content-addressed code).

**Why now (April 2026)**: MCP's 52% server abandonment rate (Rapid Claw audit, April 2026)
demonstrates the quality crisis in the agent ecosystem. Content-addressing enables integrity
verification -- you can prove that the tool output you received matches what was expected.
Combined with in-toto attestations, CAS provides the supply-chain verification layer that
MCP servers currently lack.

**Roko application**:

Hash everything roko produces:
```rust
pub struct ContentAddress(blake3::Hash);

impl ContentAddress {
    pub fn of(data: &[u8]) -> Self {
        Self(blake3::hash(data))
    }
}

// Every artifact gets a CAS address
pub struct Artifact {
    pub hash: ContentAddress,
    pub kind: ArtifactKind,    // Plan, Task, Episode, Knowledge, GateResult, Prompt
    pub metadata: ArtifactMeta,
}
```

**What gets hashed**:
- Plans -- deterministic address from task definitions
- Prompts -- hash of assembled prompt (enables prompt cache, already contributes 1.5-2x
  cost reduction in HAL benchmark data)
- Gate results -- hash of (code state + gate config + results)
- Episodes -- hash of (agent turn + context + outcome)
- Knowledge entries -- hash of content for dedup across agents
- Tool outputs -- hash for caching identical tool invocations

**Multiplicative value**:
- **Global deduplication**: same prompt across agents = one cache entry
- **Integrity verification**: chain witness without blockchain overhead
- **Distributed cache**: share CAS store across roko instances
- **Reproducibility**: same inputs -> same hash -> verified identical execution
- **Incremental sync**: only transfer hashes that differ between instances
- **Cost reduction**: HAL benchmark data shows 10-30x cost reduction from coordination-aware
  scaffolding; CAS-based caching is a key contributor

**Competitive delta**: No competing agent tool content-addresses its artifacts. Cursor's
codebase context is ephemeral. Codex CLI's outputs are fire-and-forget. Roko's CAS layer
makes every artifact verifiable, cacheable, and deduplicable.

**Compounds with**: Event Sourcing (events reference CAS hashes), Federated Learning
(share hashes for dedup before sharing content), Knowledge Store (deduplicate
cross-agent knowledge), Gateway Cache (L1 exact-match cache uses BLAKE3).

---

## Pattern 4: Effect Systems

**What it is**: Represent side effects (I/O, tool calls, model invocations) as data that
is interpreted by a handler. Separate "what to do" from "how to do it." The program produces
a description of effects; the runtime decides how to execute them.

**Real-world**: Haskell IO monad, Eff (OCaml algebraic effects), React (effects as data
via hooks), Redux-Saga (effects as plain objects), Unison (ability handlers).

**Why now (April 2026)**: The explosion of agent tool use (MCP has 17,468+ servers) means
agents are making more side-effecting calls than ever. Without effect systems, every tool
call is a black box. With effect systems, tool calls become interceptable, mockable,
batchable, and auditable -- essential for both testing and compliance.

**Roko application**:

Agent tool calls as effect descriptions:
```rust
pub enum AgentEffect {
    ReadFile { path: PathBuf },
    WriteFile { path: PathBuf, content: String },
    RunCommand { cmd: String, args: Vec<String> },
    CallModel { model: String, prompt: String, budget: TokenBudget },
    SearchCode { query: String, scope: SearchScope },
    QueryKnowledge { topic: String, tier_max: u8 },
    EmitSignal { kind: SignalKind, payload: Value },
    SpawnAgent { role: String, task: TaskSpec },
}

pub trait EffectHandler: Send + Sync {
    fn handle(&self, effect: AgentEffect, ctx: &EffectContext) -> Result<EffectResult>;
}
```

**Multiplicative value**:
- **Testing**: replace real handler with mock (no filesystem, no LLM calls) -- roko's
  test suite can exercise full agent workflows without API keys
- **Simulation**: "what would happen if?" without executing -- the Digital Twins pattern
  depends on this
- **Replay**: reproduce exact execution by replaying effects -- debugging agent failures
- **Interception**: middleware can transform/filter/log any effect -- the gateway pipeline
  already uses this shape
- **Batching**: collect effects, optimize (combine file reads, batch API calls)
- **Dry-run mode**: show what would happen without doing it -- `roko plan run --dry-run`
- **Conformance testing**: `ROKO_ACC=1` env-gating for real-infra tests maps directly to
  effect handler switching (mock handler for unit tests, real handler for integration)

**Competitive delta**: Codex CLI's tool calls are direct function invocations -- no
interception layer. Claude Code's tool use is synchronous and non-interceptable. Roko's
effect system makes tool calls first-class data, enabling the entire middleware stack.

**Compounds with**: Event Sourcing (effects are events before execution),
Capability Security (handler checks capabilities before executing),
Middleware Stacks (each middleware is an effect transformer).

---

## Pattern 5: Schema Registry + Protocol Evolution

**What it is**: Centralized registry of data schemas with versioning, compatibility checks,
and migration support. Enables independent evolution of producers and consumers.

**Real-world**: Confluent Schema Registry (Kafka), Protobuf (backward/forward compat),
GraphQL schema stitching, OpenAPI/AsyncAPI, Buf (protobuf linting + breaking change
detection).

**Why now (April 2026)**: The adapter ecosystem is approaching the threshold where
independent versioning becomes critical. Terraform's 3,500+ providers work because each
provider declares its schema version and the registry checks compatibility. MCP's quality
crisis (52% abandonment) is partly a versioning problem -- servers break when the protocol
evolves.

**Roko application**:

Versioned adapter trait interfaces:
```rust
/// Version declaration for adapter protocols
pub struct AdapterProtocol {
    pub name: &'static str,
    pub version: semver::Version,
    pub schema: &'static str,  // JSON Schema for input/output
    pub breaking_changes: &[BreakingChange],
}

/// Adapters declare which protocol versions they support
pub trait VersionedAdapter {
    fn supported_versions(&self) -> Vec<semver::VersionReq>;
    fn negotiate_version(&self, available: &[semver::Version]) -> Option<semver::Version>;
}
```

**Multiplicative value**:
- **Independent adapter evolution**: update provider adapter without breaking consumers
- **Marketplace safety**: schema compatibility checked at install time, not runtime
- **Migration automation**: generate adapter shims between versions
- **Multi-version support**: run v1 and v2 adapters simultaneously during transition
- **Documentation generation**: schemas produce API docs produce developer portal

**Competitive delta**: None of the competing agent tools have versioned plugin interfaces.
When Cursor updates its internal APIs, extensions break silently. Roko's schema registry
ensures adapters declare compatibility and the runtime checks it.

**Compounds with**: Registry + Auto-Discovery (schema registry IS the discovery mechanism),
Marketplace (version compatibility = trust signal), Process Boundary (schemas are the
contract between processes), Conformance Testing (schema compatibility is a test dimension).

---

## Pattern 6: Composable Middleware Stacks

**What it is**: Build complex behavior by stacking simple, independent layers. Each layer
wraps the next, transforming requests and/or responses. Layers can be added, removed, or
reordered without changing the core logic.

**Real-world**: Tower (Rust, used by Axum/Tonic/Hyper), Express.js middleware, Django
middleware, OTel Collector pipeline (receivers -> processors -> exporters), nginx modules.

**Why now (April 2026)**: The gateway pipeline (auth -> format -> safety -> cache -> route ->
optimize -> bill -> forward) is roko's richest adapter surface -- 8 layers, each a trait
boundary. This is the pattern that makes the gateway both a standalone product and an embedded
subsystem. Standalone LLM gateways cap at ~$50M ARR (OpenRouter at $30-50M). Embedded in
roko, the gateway layers feed learning loops, creating compound value that standalone gateways
cannot access.

**Roko application**:

Tower-style layers for every pipeline in roko:
```rust
/// Generic middleware layer for any adapter pipeline
pub trait Layer<S> {
    type Service;
    fn layer(&self, inner: S) -> Self::Service;
}

/// Example: gate pipeline as middleware stack
let gate_pipeline = ServiceBuilder::new()
    .layer(TelemetryLayer::new(exporter))      // OTel gen_ai.* spans
    .layer(AdaptiveThresholdLayer::new(thresholds))
    .layer(CacheLayer::new(cache))
    .layer(TimingLayer::new())
    .layer(RetryLayer::new(3))
    .service(CompileGate::new());
```

**Important caveat from Tower**: Tower's `Service` trait (two methods, one associated future
type) is famously easy to mis-implement. The async-fn-in-trait initiative explicitly cites
Tower as the case study. Roko should learn from this -- use `async_trait` for plugin-facing
APIs, not raw associated future types.

**Applicable pipelines in roko**:
- **Gate pipeline**: compile -> test -> clippy -> review (7 rungs, already exists)
- **Prompt assembly**: identity -> plan -> workspace -> context -> skills -> feedback (9 layers)
- **Model dispatch**: route -> optimize -> rate-limit -> cache -> call -> parse
- **Safety pipeline**: auth -> contract -> tool-check -> post-check (8 bundled contracts)
- **Gateway pipeline**: auth -> format -> safety -> cache -> route -> optimize -> bill -> forward

**Multiplicative value**:
- **Composability**: any layer works with any pipeline
- **Observability for free**: add OTel telemetry layer once, applies everywhere -- ~200 LOC
  for gen_ai.* span builder + constants, covering 6 vendor backends (Datadog, Honeycomb,
  Langfuse, Phoenix, Langtrace, Grafana)
- **A/B testing**: swap layers to test alternatives via ExperimentStore
- **User customization**: TOML config enables/disables/reorders layers

**Competitive delta**: Cursor's pipeline is monolithic -- no user customization. Codex CLI
has no middleware concept. Roko's every pipeline is composable, observable, and configurable.

**Compounds with**: Adapter traits (each adapter is a potential layer), Effect Systems
(layers transform effects), Gateway (already has this pipeline shape), Bevy Plugin Trait
(function-as-plugin blanket impl collapses cognitive cost).

---

## Pattern 7: Datalog / Logic Programming for Task DAGs

**What it is**: Express task dependencies and constraints as logical rules. The runtime
engine computes execution order, detects cycles, and supports incremental recomputation
when facts change.

**Real-world**: Datomic (Datalog queries over immutable facts), Souffle (high-performance
Datalog, used by Meta for static analysis), Bazel/Buck2 (build graph as constraint
satisfaction), Salsa (incremental computation in rust-analyzer), Datafrog (Rust Datalog).

**Why now (April 2026)**: As agent plans grow more complex (roko's orchestrator handles
multi-task DAGs with dependencies, parallelism, and failure replanning), the imperative
DAG execution model becomes brittle. Datalog provides declarative task scheduling that
handles incremental replanning natively -- when one task fails, only the affected subgraph
is recomputed, not the entire plan.

**Roko application**:

Task DAG as Datalog facts:
```
// Facts
task("implement-auth", implementer).
depends("test-auth", "implement-auth").
depends("review-auth", "test-auth").
gate_result("implement-auth", pass).
gate_result("test-auth", fail).

// Rules
ready(T) :- task(T, _), not blocked(T).
blocked(T) :- depends(T, Dep), not completed(Dep).
completed(T) :- task(T, _), gate_result(T, pass).
needs_replan(T) :- gate_result(T, fail), retry_count(T, N), N >= 3.
```

**Multiplicative value**:
- **Declarative DAGs**: express "what depends on what," not "how to schedule"
- **Incremental replanning**: when one task fails, only recompute affected subgraph --
  critical for roko's `build_gate_failure_plan_revision` workflow
- **Constraint propagation**: "security review required before any deploy task" = one rule
- **Query execution state**: "which tasks are blocked?" = one Datalog query
- **Cross-plan dependencies**: tasks in plan A can depend on tasks in plan B

**Competitive delta**: Codex CLI has no plan concept. Claude Code executes sequentially.
Devin has linear task chains. Roko's DAG executor already handles parallel execution with
dependencies; Datalog would make replanning declarative rather than imperative.

**Compounds with**: Event Sourcing (facts are derived from events), Incremental
Computation (Datalog engines are naturally incremental), Content-Addressable Storage
(fact identity by content hash).

---

## Pattern 8: Digital Twins for Agent Simulation

**What it is**: Create a lightweight model of the real system that can be queried for
predictions without executing the real thing. "What would happen if we ran this agent
with this prompt on this task?"

**Real-world**: Manufacturing (GE digital twins), AWS SimSpace Weaver, Unity Simulation,
Waymo (simulated driving), Netflix (Chaos Engineering simulations), AlphaCode (generate
1M candidates, filter to 10).

**Why now (April 2026)**: LLM costs are a major pain point. GPT-5.5 is $5/$30 per 1M
tokens. Claude Sonnet 4.6 is $3/$15. Users are burning $30-100 per complex task without
knowing in advance whether the approach will work. Simulation enables "try before you
spend" -- predict outcomes from historical data before committing API budget.

**Roko application**:

Simulate agent execution before committing resources:
```rust
pub struct AgentSimulation {
    /// Historical data for this (role, task_type, model) triple
    pub historical_outcomes: Vec<SimulatedOutcome>,
    /// Estimated cost (tokens, time, API calls)
    pub cost_estimate: CostEstimate,
    /// Probability of gate pass on first attempt
    pub first_pass_probability: f64,
    /// Expected number of retry cycles
    pub expected_retries: f64,
    /// Risk factors
    pub risks: Vec<RiskFactor>,
}

pub trait AgentSimulator: Send + Sync {
    /// Simulate execution without running
    fn simulate(&self, task: &TaskSpec, config: &AgentConfig) -> SimulationResult;
    /// Compare N strategies and rank by expected value
    fn compare_strategies(&self, task: &TaskSpec, configs: &[AgentConfig]) -> Vec<RankedStrategy>;
}
```

**Multiplicative value**:
- **Cost prediction**: estimate before spending -- critical for teams with LLM budgets
- **Strategy selection**: simulate 5 approaches, pick the best -- CascadeRouter can
  use simulation data to inform routing decisions
- **Risk assessment**: flag tasks likely to fail before execution
- **Capacity planning**: predict resource needs for a full plan
- **What-if analysis**: "what if we used Gemini 2.0 Flash ($0.10/$0.40) instead of
  Claude Sonnet ($3/$15) for this task type?" -- the cost difference is 30x

**Competitive delta**: No competing product offers pre-execution simulation. Cursor users
discover cost after the fact. Codex CLI users have no cost prediction. Roko's historical
episode data (already collected) is the training set for simulation models.

**Compounds with**: Event Sourcing (historical events = training data for simulation),
Learning (simulation improves from real outcomes), CascadeRouter (simulation informs
routing decisions), Effect Systems (simulation uses mock effect handlers).

---

## Pattern 9: Cellular Architecture (Supervision Trees)

**What it is**: Organize system components into hierarchical cells. Each cell has a
supervisor that monitors children and applies recovery strategies (restart, escalate,
ignore). Failures are contained within cells; the rest of the system continues.

**Real-world**: Erlang/OTP (99.9999999% uptime at Ericsson), Akka (JVM actors),
Microsoft Orleans (virtual actors), Kubernetes (pod -> replicaset -> deployment hierarchy),
systemd (service supervision).

**Why now (April 2026)**: Agent execution is inherently unreliable -- LLM API calls
fail, tool executions crash, gate evaluations timeout. The question is not "will agents
fail" but "how does the system recover?" Roko's ProcessSupervisor already tracks agents
via roko-runtime. The cellular architecture pattern generalizes this into a hierarchical
recovery system.

**Roko application**:

Agent execution as supervision tree:
```
PlanSupervisor
+-- TaskGroup("auth-module")
|   +-- AgentCell(Implementer, retry=3)
|   +-- AgentCell(Reviewer, retry=1)
|   +-- GateCell(compile+test, retry=2)
+-- TaskGroup("api-routes")
|   +-- AgentCell(Implementer, retry=3)
|   +-- GateCell(compile+test+clippy, retry=2)
+-- TaskGroup("documentation")
    +-- AgentCell(Scribe, retry=1)

// Recovery strategies per level
PlanSupervisor: on_child_failure -> replan_task_group
TaskGroup: on_child_failure -> retry_with_feedback -> escalate_to_plan
AgentCell: on_failure -> restart_with_context -> swap_model -> escalate
GateCell: on_failure -> auto_fix -> escalate
```

**Multiplicative value**:
- **Fault isolation**: one agent crash doesn't stop the plan -- roko's
  `build_gate_failure_plan_revision` already implements this at the task level
- **Recovery strategies**: configurable per-level (retry, restart, escalate, skip)
- **Resource management**: supervisor enforces budget per cell
- **Graceful degradation**: if reviewer cell fails, still commit with warning
- **Scaling**: each cell can run on different machines
- **Model swapping**: when an agent fails with model X, swap to model Y and retry --
  CascadeRouter already supports this via bandit arm selection

**Competitive delta**: Codex CLI aborts on failure. Claude Code retries the same approach.
Cursor has no supervision hierarchy. Roko's cellular architecture means intelligent,
hierarchical recovery -- retry with different model, different prompt, different strategy.

**Compounds with**: Effect Systems (supervisor intercepts effects for monitoring),
Digital Twins (supervisor uses simulation for recovery decisions), Capability Security
(supervisor grants capabilities to children), CascadeRouter (supervisor triggers model
swaps on failure).

---

## Pattern 10: Attention / Bidding Mechanisms

**What it is**: When multiple components compete for a limited resource (context window,
compute budget, time), use an auction or attention mechanism to allocate optimally. Each
component bids based on its expected value contribution.

**Real-world**: VCG auctions (Google AdWords, spectrum auctions), transformer attention
(self-attention in LLMs), resource scheduling (Kubernetes, Mesos), economic mechanism
design (Nobel Prize 2020: auction theory).

**Why now (April 2026)**: Context windows are the scarcest resource in agent execution.
Even with 1M token windows (GPT-5.5, Gemini 2.5 Pro), filling the context with irrelevant
information degrades output quality. The question is: what goes in the context window?
Roko's VCG auction is built (`vcg_allocate` exported) but the greedy path dominates at
runtime. Switching to auction-based allocation is a concrete improvement.

**Roko application** (partially exists -- VCG auction built but greedy path dominates):

```rust
pub struct ContextBidder {
    pub source: BidderSource,     // Neuro, Task, Research, Episodes, Code
    pub bid: f64,                 // Expected value of including this context
    pub tokens_requested: usize,  // How much context window space needed
    pub historical_impact: f64,   // Past correlation with task success
}

pub trait AttentionAllocator: Send + Sync {
    /// Allocate context budget across competing bidders
    fn allocate(&self, bidders: &[ContextBidder], budget: usize) -> Vec<Allocation>;
    /// Update bidder valuations based on outcome
    fn observe_outcome(&mut self, allocations: &[Allocation], outcome: &TaskOutcome);
}
```

**What gets auctioned**:
- **Context window tokens**: plan spec vs workspace map vs code context vs episodes vs
  knowledge store entries -- 5 `AttentionBidder` variants already exist in orchestrate.rs
- **Compute budget**: which tasks get more model calls
- **Agent time**: which tasks are most valuable to work on next
- **Gate depth**: which code changes get full 7-rung vs quick 3-rung validation

**Multiplicative value**:
- **Optimal resource allocation**: maximize expected value per token/dollar
- **Self-adjusting**: bidders learn from outcomes, allocation improves over time --
  this is the mechanism that makes roko's prompts better with every execution
- **Transparent prioritization**: auctions make tradeoffs explicit and debuggable
- **Multi-objective optimization**: balance quality, cost, speed simultaneously

**Competitive delta**: Cursor fills context with whatever fits. Codex CLI uses fixed
context strategies. Claude Code has no context allocation mechanism. Roko's auction-based
context allocation is a unique capability that improves with use.

**Compounds with**: Learning (bidder valuations improve from outcomes), Prompt Assembly
(auction determines prompt composition), CascadeRouter (model selection as auction),
Digital Twins (simulate allocation before committing).

---

## Pattern 11: Incremental Computation

**What it is**: Only recompute what changed. Track dependencies between computations,
and when inputs change, recompute only the affected downstream outputs. Sub-millisecond
updates for large dependency graphs.

**Real-world**: Salsa (rust-analyzer, 50K+ queries, sub-millisecond response),
Adapton (incremental algorithms), React (virtual DOM diffing), Incremental (Jane Street,
OCaml), Excel (cell dependency tracking), Make/Bazel (build graph).

**Why now (April 2026)**: As roko's plans grow in complexity (multi-task DAGs with
dependencies, parallel execution, gate validation), full recomputation becomes expensive.
When one task fails and triggers replanning, the entire prompt assembly, gate prediction,
and execution order should NOT be recomputed from scratch -- only the affected subgraph.
This is critical for interactive use cases like `roko dashboard` where real-time updates
matter.

**Roko application**:

Incremental prompt assembly and replanning:
```rust
/// Salsa-style incremental computation for roko
#[salsa::query_group(AgentQueriesStorage)]
pub trait AgentQueries {
    /// Input: task specification (changes when task is modified)
    #[salsa::input]
    fn task_spec(&self, id: TaskId) -> TaskSpec;

    /// Input: code state (changes when files are modified)
    #[salsa::input]
    fn file_content(&self, path: PathBuf) -> String;

    /// Derived: assembled prompt (recomputed only when inputs change)
    fn assembled_prompt(&self, id: TaskId) -> AssembledPrompt;

    /// Derived: gate prediction (recomputed from code + historical data)
    fn gate_prediction(&self, id: TaskId) -> GatePrediction;

    /// Derived: execution plan (recomputed only when DAG changes)
    fn execution_order(&self, plan: PlanId) -> Vec<TaskId>;
}
```

**Multiplicative value**:
- **Instant replanning**: change one task -> only recompute affected downstream tasks
- **Prompt cache hits**: prompt unchanged -> skip assembly entirely -- combined with
  Anthropic's `cache_control` prefix injection (L3 cache), this compounds cost savings
- **Live dashboard**: file change -> incrementally update affected metrics in TUI
- **Code index**: file saved -> update only that file's symbols, not full reindex
- **Gate optimization**: if code for this task didn't change, skip re-gating

**Competitive delta**: No competing agent tool uses incremental computation. Every
re-execution starts from scratch. Roko's incremental computation means that repeated
runs on similar tasks get progressively cheaper and faster.

**Compounds with**: Content-Addressable Storage (change detection via hash comparison),
Event Sourcing (events trigger incremental recomputation), Datalog (Datalog engines
are naturally incremental), TUI (incremental updates to dashboard via StateHub push).

---

## Pattern 12: Federated Learning

**What it is**: Multiple instances learn independently, then share aggregated insights
without sharing raw data. Each instance improves from collective experience while
maintaining privacy and reducing coordination overhead.

**Real-world**: Google (Gboard predictions across billions of devices), Apple (Siri
improvements), Flower framework (open-source federated learning), Brave Search
(privacy-preserving search quality), differential privacy (formal privacy guarantees).

**Why now (April 2026)**: The AI developer tool market is fragmenting across models
(GPT-5.5, Claude 4.6, Gemini 2.5 Pro, DeepSeek R1, open-source models via Ollama/vLLM).
No single team can evaluate all model/task combinations. Federated learning across roko
instances means collective intelligence: 100 roko instances running different models on
different task types produce routing data that benefits all instances.

**Roko application**:

Cross-instance intelligence sharing:
```rust
pub struct FederatedInsight {
    /// What was learned (aggregated, not raw)
    pub insight_type: InsightType,
    /// Statistical summary (not raw data)
    pub summary: InsightSummary,
    /// Confidence level
    pub confidence: f64,
    /// How many instances contributed
    pub sample_size: usize,
    /// Differential privacy budget spent
    pub epsilon: f64,
}

pub enum InsightType {
    /// Model X works better than Model Y for task type Z
    ModelRouting { task_type: String, preferred_model: String, win_rate: f64 },
    /// Prompt section S has effectiveness E for role R
    PromptEffectiveness { section: String, role: String, effectiveness: f64 },
    /// Gate rung G has false-positive rate F for language L
    GateCalibration { rung: String, language: String, false_positive_rate: f64 },
    /// Tool T is most useful for task type Z
    ToolRelevance { tool: String, task_type: String, usage_rate: f64 },
}

pub trait FederatedLearner: Send + Sync {
    /// Export aggregated insights from this instance
    async fn export_insights(&self) -> Result<Vec<FederatedInsight>>;
    /// Import insights from peer instances
    async fn import_insights(&self, insights: &[FederatedInsight]) -> Result<MergeReport>;
    /// Negotiate what to share (privacy budget)
    fn privacy_policy(&self) -> PrivacyPolicy;
}
```

**Multiplicative value**:
- **Collective intelligence**: 100 roko instances learn 100x faster than one
- **Privacy preserving**: share statistics, not code or prompts -- GDPR compliant by design
- **Cold start elimination**: new instance bootstraps from federated insights instead of
  starting from zero
- **Model routing**: community-wide routing data makes CascadeRouter better for everyone --
  "Gemini 2.0 Flash ($0.10/$0.40) is 95% as good as Claude Sonnet ($3/$15) for Python
  refactoring tasks" is the kind of insight that saves real money
- **Gate calibration**: community-wide false-positive rates improve gate accuracy --
  fewer unnecessary retries = faster pipeline = lower cost

**Competitive delta**: No competing agent tool shares learning across instances. Every
Cursor installation starts from zero. Every Codex CLI run is independent. Roko's federated
learning means the product gets better for everyone as anyone uses it -- the strongest
possible network effect.

**Compounds with**: Event Sourcing (events -> local learning -> federated sharing),
Knowledge Store (federated insights become durable knowledge), CascadeRouter
(federated routing data), Marketplace (insight sharing as community feature),
Knowledge sync (`roko knowledge sync <peer>` already exists as the mesh sync primitive).

---

## Synthesis: How Patterns Compose

The 12 patterns form a coherent stack, not isolated choices:

```
+--------------------------------------------------+
|            Federated Learning (12)                | Cross-instance
+--------------------------------------------------+
|         Digital Twins / Simulation (8)            | Prediction
+------------------------+-------------------------+
| Attention/Bidding (10) | Incremental Compute (11)| Optimization
+------------------------+-------------------------+
| Cellular Arch (9)      | Middleware Stacks (6)    | Structure
+------------------------+-------------------------+
| Datalog/DAG (7)        | Schema Registry (5)      | Coordination
+------------------------+-------------------------+
| Effect Systems (4)     | CAS (3)                  | Foundation
+------------------------+-------------------------+
| Capability Sec (2)     | Event Sourcing (1)       | Primitives
+------------------------+-------------------------+
```

**Key multiplicative compositions**:

1. **Event Sourcing + CAS + Federated Learning** = every agent action is an immutable,
   content-addressed event that can be aggregated across instances for collective learning
   without sharing raw data. This is roko's strongest network effect mechanism.

2. **Effect Systems + Capability Security + Middleware Stacks** = tool calls are data,
   checked against capability tokens, and processed through composable middleware layers.
   Testing, simulation, and production share the same code with different effect handlers.
   This is why roko can offer both sandbox and production execution from the same codebase.

3. **Datalog + Incremental Computation + Digital Twins** = task DAGs expressed as logical
   rules, incrementally recomputed when facts change, with simulation predicting outcomes
   before execution. This is how roko's replanning works: fail a task -> simulate
   alternatives -> recompute only the affected subgraph -> execute the best strategy.

4. **Attention/Bidding + Federated Learning + CascadeRouter** = context allocation and
   model routing improve from local outcomes AND cross-instance aggregated data, creating
   a collective intelligence flywheel. Every roko instance makes every other instance
   smarter.

---

## Bevy Plugin Trait as Pattern Foundation

The Bevy Plugin trait provides the empirical foundation for Pattern 6 (Composable Middleware
Stacks) in the Rust ecosystem:

```rust
pub trait Plugin: Downcast + Any + Send + Sync {
    fn build(&self, app: &mut App);           // required
    fn ready(&self, _app: &App) -> bool { true }
    fn finish(&self, _app: &mut App) {}
    fn cleanup(&self, _app: &mut App) {}
    fn name(&self) -> &str { /* type_name */ }
    fn is_unique(&self) -> bool { true }
}
```

**The masterstroke**: The blanket impl that makes any `fn(&mut App)` automatically a `Plugin`.
The simplest plugin is a five-line function. You only reach for a struct when you need
configuration.

**Roko adopts this**: `fn(&mut RokoBuilder)` automatically becomes a `RokoAdapter`.

Key design decisions from Bevy relevant to the advanced patterns:
- **Function-as-plugin blanket impl**: collapses cognitive cost from "implement a trait" to
  "write a function." Directly applicable to Pattern 6.
- **PluginGroup for ordered batches**: `plugin_group!` macro + `PluginGroupBuilder` enables
  ordered composition with `add_before()`/`add_after()` semantics.
- **Plugin disable/override**: `DefaultPlugins.build().disable::<LogPlugin>()` allows
  selective composition. Maps to Pattern 2 (Capability-Based Security).

---

## Conformance Testing: Bridge from Patterns to Ecosystem

The `roko-conformance` crate connects the abstract pattern stack to the concrete adapter
ecosystem. Modeled on Airbyte CAT (Connector Acceptance Tests) and Terraform plugin-testing:

```rust
assert_adapter_conforms::<MyAdapter>();
```

Gated by `ROKO_ACC=1` for real-network tests. Exercises:
1. **Lifecycle conformance** -- `build()` / `ready()` / `cleanup()` sequence
2. **Schema conformance** -- input/output types match declarations (Pattern 5)
3. **Capability conformance** -- declared capabilities match behavior (Pattern 2)
4. **Idempotency conformance** -- repeated calls produce same result
5. **Error conformance** -- structured errors, not panics

Earning "Verified" badge requires passing this crate in CI. Terraform's February 2018
RedMonk audit: 42 verified modules out of 376 total = >95% of all downloads. The
verification badge does nearly all the discovery work.

---

## Sigstore/in-toto at Agent Boundaries

**Build-time verification is solved**: 101M+ Rekor entries, 33,000+ OSS projects, 21M+
Fulcio certs, 16,000+ npm packages with provenance. Cosign v3 (Aug 2025) defaults to bundle
format.

**Agent-action-time verification is unsolved**: nobody ships Sigstore/in-toto primitives at
the agent boundary. The extension to Pattern 2 (Capability-Based Security):

```
Build boundary (solved):
  code -> build -> sign -> publish -> verify -> deploy

Agent boundary (Roko's extension):
  trigger -> plan -> execute -> gate -> sign -> publish -> verify
```

Each gate result in roko's pipeline can be signed as an in-toto attestation. The `Gate`
trait already produces structured verification results; wrapping them in in-toto `Statement`
format (type, subject, predicate) is a ~100 LOC adapter. Post-Shai-Hulud demand: the
September-November 2025 npm supply-chain worm turned willingness-to-pay for verification
from theoretical to post-incident.

---

## Implementation Priority

| Priority | Pattern | Effort | Existing Code | Unlock |
|---|---|---|---|---|
| P0 | Middleware Stacks (6) | Medium | Tower in roko-serve | Composable pipelines everywhere |
| P0 | Event Sourcing (1) | Medium | EpisodeLogger exists | Foundation for 5+ other patterns |
| P1 | CAS (3) | Low | BLAKE3 already a dep | Dedup, caching, integrity |
| P1 | Effect Systems (4) | Medium | Tool dispatch exists | Testing, simulation, replay |
| P1 | Incremental Computation (11) | Medium | None | Instant replanning, prompt cache |
| P2 | Capability Security (2) | Medium | Safety contracts exist | Enterprise, marketplace |
| P2 | Cellular Architecture (9) | Medium | ProcessSupervisor exists | Fault isolation |
| P2 | Attention/Bidding (10) | Low | VCG auction built | Better prompt assembly |
| P3 | Schema Registry (5) | Medium | None | Adapter ecosystem |
| P3 | Digital Twins (8) | High | Historical data exists | Cost prediction, strategy selection |
| P3 | Datalog/DAG (7) | High | DAG executor exists | Declarative task planning |
| P4 | Federated Learning (12) | High | Knowledge sync exists | Cross-instance intelligence |

---

## Sources

- Event Sourcing: Martin Fowler, Greg Young (CQRS), EventStoreDB, Kafka Streams
- Capability Security: Mark S. Miller (E language), UCAN spec, WASI capabilities model
- Content-Addressable Storage: Git internals, IPFS, Nix store, Unison language
- Effect Systems: Algebraic effects (Plotkin & Pretnar), Eff language, Redux-Saga
- Schema Registry: Confluent, Buf, AsyncAPI, OpenAPI evolution guidelines
- Tower: Tokio project, Service/Layer/ServiceBuilder pattern, hyper/axum/tonic
- Datalog: Souffle, Datomic, Datafrog (Rust), differential-datalog
- Digital Twins: GE Digital, AWS SimSpace Weaver, simulation-based optimization
- Cellular Architecture: Erlang/OTP (Joe Armstrong), Akka, Microsoft Orleans
- Auction/Attention: VCG mechanism, transformer attention, mechanism design theory
- Incremental Computation: Salsa (rust-analyzer), Adapton, Jane Street Incremental
- Federated Learning: Google FL (2016), Flower framework, differential privacy
- Market data: Cursor $2B ARR (Feb 2026), Codex CLI 75K stars/3M WAU, Devin $25B val talks
- Bevy: Plugin trait design, function-as-plugin blanket impl
- Sigstore: 101M+ Rekor entries, Cosign v3 (Aug 2025), 16,000+ npm packages with provenance
- MCP: 97M monthly SDK downloads, 17,468+ servers, 52% abandonment (Rapid Claw audit)
- EU AI Act: Article 50 enforcement August 2, 2026
- HAL benchmark: 10-30x cost reduction from coordination-aware scaffolding
- LLM pricing: GPT-5.5 $5/$30, Claude Sonnet 4.6 $3/$15, Gemini 2.0 Flash $0.10/$0.40
