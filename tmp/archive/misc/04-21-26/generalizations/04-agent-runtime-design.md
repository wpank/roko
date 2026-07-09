# AgentRuntime Design: Universal Agent Process Model

## Design Principles

1. **Single runtime, many profiles** — One `AgentRuntime` trait, parameterized by domain profile
2. **Extension-based composition** — Behavior comes from layered extensions, not monolithic code
3. **Heartbeat as the clock** — All agents tick; domain profiles control frequency and gating
4. **Event-driven reactivity** — Agents subscribe to relevant event streams
5. **Type-state lifecycle** — Compile-time valid state enforcement
6. **Learnable context** — Context assembly is a feedback loop, not static budgeting

## Core Trait

```rust
/// The universal agent process. All agents — coding, blockchain, research,
/// writing, custom — implement this through extension composition.
#[async_trait]
pub trait AgentRuntime: Send + Sync {
    /// The heartbeat. Called on every tick at the configured frequency.
    /// Extensions fire their hooks during this call.
    async fn tick(&mut self) -> Result<TickOutcome>;

    /// Receive an event from the fabric (chain block, file change, pheromone, etc.)
    async fn on_event(&mut self, event: RuntimeEvent) -> Result<EventResponse>;

    /// Operator message (persistent chat input, steer command)
    async fn on_message(&mut self, msg: OperatorMessage) -> Result<AgentResponse>;

    /// Inject a task (for backwards compat with plan-based dispatch)
    async fn on_task(&mut self, task: TaskEnvelope) -> Result<TaskOutcome>;

    /// Current lifecycle state
    fn state(&self) -> LifecycleState;

    /// Extension access (for inter-extension queries)
    fn extension<T: Extension>(&self) -> Option<&T>;
    fn extension_mut<T: Extension>(&mut self) -> Option<&mut T>;
}
```

## Lifecycle (Type-State)

```rust
// Phase markers (zero-sized, compile-time only)
pub struct Provisioning;
pub struct Active;
pub struct Dreaming;
pub struct Suspended; // NEW: for task agents between tasks
pub struct Terminal;

pub struct Agent<Phase> {
    inner: AgentInner,
    _phase: PhantomData<Phase>,
}

// Valid transitions
impl Agent<Provisioning> {
    pub fn activate(self, extensions: ExtensionChain) -> Agent<Active>;
}

impl Agent<Active> {
    pub async fn tick(&mut self) -> Result<TickOutcome>;
    pub async fn on_event(&mut self, event: RuntimeEvent) -> Result<EventResponse>;
    pub fn begin_dream(self) -> Agent<Dreaming>;
    pub fn suspend(self) -> Agent<Suspended>; // pause between tasks
    pub fn begin_death(self) -> Agent<Terminal>;
}

impl Agent<Dreaming> {
    pub async fn dream_cycle(&mut self) -> Result<DreamOutcome>;
    pub fn wake(self) -> Agent<Active>;
    pub fn emergency_wake(self, reason: WakeReason) -> Agent<Active>;
}

impl Agent<Suspended> {
    pub fn resume(self, task: TaskEnvelope) -> Agent<Active>;
    pub fn begin_death(self) -> Agent<Terminal>;
}

impl Agent<Terminal> {
    pub async fn extract_genome(self) -> GenomeExtract;
}
```

## Extension Trait

```rust
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn layer(&self) -> ExtensionLayer;
    fn depends_on(&self) -> &[&str] { &[] }

    // === Session Lifecycle ===
    async fn on_boot(&mut self, _ctx: &mut BootContext) -> Result<()> { Ok(()) }
    async fn on_resume(&mut self, _ctx: &mut ResumeContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, _ctx: &ShutdownContext) -> Result<ShutdownVote> {
        Ok(ShutdownVote::Approve)
    }

    // === Heartbeat ===
    async fn on_tick_start(&mut self, _ctx: &mut TickContext) -> Result<()> { Ok(()) }
    async fn on_tick_end(&mut self, _ctx: &mut TickContext) -> Result<()> { Ok(()) }

    // === Perception ===
    async fn on_observe(&mut self, _ctx: &mut ObserveContext) -> Result<()> { Ok(()) }
    async fn on_event(&mut self, _event: &RuntimeEvent, _ctx: &mut EventContext) -> Result<()> { Ok(()) }

    // === Cognition ===
    async fn on_gate(&mut self, _ctx: &mut GateContext) -> Result<Option<CognitiveTier>> { Ok(None) }
    async fn assemble_context(&mut self, _ws: &mut CognitiveWorkspace) -> Result<()> { Ok(()) }
    async fn on_before_inference(&mut self, _ctx: &mut InferenceContext) -> Result<()> { Ok(()) }
    async fn on_after_inference(&mut self, _ctx: &mut InferenceContext) -> Result<()> { Ok(()) }

    // === Action ===
    async fn before_tool_call(&mut self, _call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }
    async fn after_tool_call(&mut self, _call: &ToolCall, _result: &ToolResult) -> Result<()> {
        Ok(())
    }

    // === Learning ===
    async fn on_outcome(&mut self, _ctx: &mut OutcomeContext) -> Result<()> { Ok(()) }
    async fn on_reflect(&mut self, _record: &DecisionCycleRecord) -> Result<()> { Ok(()) }

    // === Dreams ===
    async fn on_dream_start(&mut self, _ctx: &mut DreamContext) -> Result<()> { Ok(()) }
    async fn on_dream_phase(&mut self, _phase: DreamPhase, _ctx: &mut DreamContext) -> Result<()> { Ok(()) }
    async fn on_dream_end(&mut self, _outcome: &DreamOutcome) -> Result<()> { Ok(()) }

    // === Communication ===
    async fn on_message(&mut self, _msg: &OperatorMessage, _ctx: &mut MessageContext) -> Result<()> { Ok(()) }
    async fn on_pheromone(&mut self, _signal: &PheromoneSignal) -> Result<()> { Ok(()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionLayer {
    Foundation = 0,  // Clock, CorticalState, Lifecycle
    Perception = 1,  // EventFabric, Probes, Subscriptions
    Memory = 2,      // Neuro, Episodic, WorkingMemory
    Cognition = 3,   // Daimon, Attention, Gating, Habituation
    Action = 4,      // ToolDispatch, Safety, Execution
    Social = 5,      // Pheromones, A2A, OperatorChat
    Meta = 6,        // Dreams, Consolidation, Playbooks
    Recovery = 7,    // Compensation, Rollback, Shutdown
}
```

## Extension Chain (Registry + Firing Order)

```rust
pub struct ExtensionChain {
    extensions: Vec<Box<dyn Extension>>,
    /// Pre-computed topological order per hook
    firing_order: HashMap<HookId, Vec<usize>>,
}

impl ExtensionChain {
    pub fn builder() -> ExtensionChainBuilder;

    /// Fire a hook across all extensions in layer order
    pub async fn fire_tick_start(&mut self, ctx: &mut TickContext) -> Result<()>;
    pub async fn fire_observe(&mut self, ctx: &mut ObserveContext) -> Result<()>;
    pub async fn fire_gate(&mut self, ctx: &mut GateContext) -> Result<Option<CognitiveTier>>;
    pub async fn fire_assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()>;
    // ... one method per hook
}

pub struct ExtensionChainBuilder {
    extensions: Vec<Box<dyn Extension>>,
}

impl ExtensionChainBuilder {
    pub fn add(mut self, ext: impl Extension) -> Self;
    pub fn build(self) -> Result<ExtensionChain>; // validates deps, computes order
}
```

## The Heartbeat (Core Tick Pipeline)

```rust
pub struct HeartbeatPipeline {
    frequency: Frequency,
    tick_count: u64,
    adaptive_threshold: f64,
    last_prediction_error: f64,
}

impl HeartbeatPipeline {
    /// The 9-step pipeline. Extensions hook into each step.
    pub async fn execute_tick(
        &mut self,
        extensions: &mut ExtensionChain,
        cortical: &CorticalState,
        arena: &TickArena,
    ) -> Result<TickOutcome> {
        let mut ctx = TickContext::new(self.tick_count, self.frequency, arena);

        // 1. OBSERVE — extensions read their data sources
        extensions.fire_observe(&mut ObserveContext::from(&mut ctx)).await?;

        // 2. RETRIEVE — extensions query their knowledge stores
        extensions.fire_tick_start(&mut ctx).await?;

        // 3. ANALYZE — compute prediction error from observation vs expectation
        let prediction_error = ctx.prediction_error();

        // 4. GATE — decide cognitive tier
        let tier = if let Some(forced) = extensions.fire_gate(&mut GateContext::from(&mut ctx)).await? {
            forced
        } else {
            self.default_gate(prediction_error, cortical)
        };

        ctx.set_tier(tier);

        // 5-8: SIMULATE → VALIDATE → EXECUTE → VERIFY (conditional on tier)
        let outcome = match tier {
            CognitiveTier::T0 => TickOutcome::Suppressed,
            CognitiveTier::T1 | CognitiveTier::T2 => {
                // Assemble context
                let mut workspace = CognitiveWorkspace::new(tier);
                extensions.fire_assemble_context(&mut workspace).await?;

                // Run inference
                let mut inf_ctx = InferenceContext::new(workspace, tier);
                extensions.fire_before_inference(&mut inf_ctx).await?;
                let result = self.run_inference(&inf_ctx).await?;
                inf_ctx.set_result(result);
                extensions.fire_after_inference(&mut inf_ctx).await?;

                // Execute tool calls (if any)
                let actions = self.execute_actions(&inf_ctx, extensions).await?;

                TickOutcome::Acted { tier, actions, cost: inf_ctx.cost() }
            }
        };

        // 9. REFLECT — record decision cycle, fire learning hooks
        let record = DecisionCycleRecord::from_tick(&ctx, &outcome);
        extensions.fire_reflect(&record).await?;
        extensions.fire_tick_end(&mut ctx).await?;

        self.tick_count += 1;
        Ok(outcome)
    }

    fn default_gate(&self, pe: f64, cortical: &CorticalState) -> CognitiveTier {
        let threshold = self.adaptive_threshold;
        if pe < threshold { CognitiveTier::T0 }
        else if pe < threshold * 2.0 { CognitiveTier::T1 }
        else { CognitiveTier::T2 }
    }
}
```

## Domain Profiles

```rust
/// Domain profile controls tick frequency, extensions, and gating behavior.
#[derive(Debug, Clone)]
pub struct DomainProfile {
    pub name: String,
    pub gamma_interval: Duration,      // perception tick (default: 10s)
    pub theta_interval: Duration,      // decision tick (default: 60s)
    pub delta_interval: Duration,      // consolidation (default: 50 theta ticks)
    pub base_gate_threshold: f64,      // T0/T1/T2 boundary (default: 0.3)
    pub extensions: Vec<String>,       // which extensions to activate
    pub event_subscriptions: Vec<EventFilter>, // what events to receive
    pub context_categories: Vec<ContextCategory>, // what context to assemble
    pub default_gates: Vec<String>,    // verification gates for this domain
    pub uses_git: bool,
    pub uses_worktrees: bool,
}

// Predefined profiles
impl DomainProfile {
    pub fn coding() -> Self {
        Self {
            name: "coding".into(),
            gamma_interval: Duration::from_secs(30),  // slower: code doesn't change fast
            theta_interval: Duration::from_secs(120), // full decision every 2 min
            delta_interval: Duration::from_secs(6000), // dream every ~50 thetas
            base_gate_threshold: 0.4, // higher threshold: most ticks are idle
            extensions: vec![
                "heartbeat", "neuro", "daimon", "conductor", "tools",
                "git", "safety", "learning", "dreams", "context",
            ],
            event_subscriptions: vec![
                EventFilter::FileChange,
                EventFilter::TestResult,
                EventFilter::GateVerdict,
            ],
            context_categories: vec![
                ContextCategory::Task, ContextCategory::CodeContext,
                ContextCategory::Knowledge, ContextCategory::Playbook,
            ],
            default_gates: vec!["compile", "test", "clippy"],
            uses_git: true,
            uses_worktrees: true,
        }
    }

    pub fn blockchain() -> Self {
        Self {
            name: "blockchain".into(),
            gamma_interval: Duration::from_secs(5),    // fast: block times are 12s
            theta_interval: Duration::from_secs(30),   // full decision every 30s
            delta_interval: Duration::from_secs(1500), // dream every ~50 thetas
            base_gate_threshold: 0.2, // lower threshold: more reactive
            extensions: vec![
                "heartbeat", "neuro", "daimon", "conductor", "tools",
                "chain-subscriber", "risk", "safety", "learning", "dreams",
                "context", "pheromones", "mortality",
            ],
            event_subscriptions: vec![
                EventFilter::NewBlock,
                EventFilter::MempoolTx,
                EventFilter::PriceFeed,
                EventFilter::Pheromone,
            ],
            context_categories: vec![
                ContextCategory::Strategy, ContextCategory::Positions,
                ContextCategory::MarketState, ContextCategory::Knowledge,
                ContextCategory::Risk, ContextCategory::Mortality,
            ],
            default_gates: vec!["simulation", "invariant-check", "risk-limit"],
            uses_git: false,
            uses_worktrees: false,
        }
    }

    pub fn research() -> Self {
        Self {
            name: "research".into(),
            gamma_interval: Duration::from_secs(60),    // slow: research is deliberate
            theta_interval: Duration::from_secs(300),   // 5 min between decisions
            delta_interval: Duration::from_secs(15000), // dream less often
            base_gate_threshold: 0.35,
            extensions: vec![
                "heartbeat", "neuro", "daimon", "tools", "learning",
                "dreams", "context", "knowledge-graph",
            ],
            event_subscriptions: vec![
                EventFilter::NewPublication,
                EventFilter::DataUpdate,
                EventFilter::KnowledgeChange,
            ],
            context_categories: vec![
                ContextCategory::Topic, ContextCategory::Sources,
                ContextCategory::Knowledge, ContextCategory::Hypotheses,
            ],
            default_gates: vec!["citation-check", "factual-consistency", "quality"],
            uses_git: false,
            uses_worktrees: false,
        }
    }
}
```

## CognitiveWorkspace (Learnable Context Assembly)

```rust
pub struct CognitiveWorkspace {
    pub tier: CognitiveTier,
    pub sections: Vec<ContextSection>,
    pub total_budget_tokens: u32,
    pub used_tokens: u32,
    pub assembly_log: Vec<AssemblyReason>,
}

pub struct ContextSection {
    pub category: ContextCategory,
    pub priority: u8,           // 1-5 (5 = always include, 1 = drop first)
    pub allocation: f64,        // fraction of budget (learned)
    pub content: String,
    pub tokens: u32,
    pub metadata: SectionMetadata,
}

pub struct ContextPolicy {
    pub revision: u32,
    pub allocations: HashMap<ContextCategory, f64>,
    pub regime_overrides: HashMap<String, HashMap<ContextCategory, f64>>,
    pub phase_overrides: HashMap<BehavioralPhase, HashMap<ContextCategory, f64>>,
    pub task_overrides: HashMap<String, HashMap<ContextCategory, f64>>,
    pub feedback: HashMap<ContextCategory, BetaDistribution>,
}

impl ContextPolicy {
    /// Apply outcome feedback to category allocations (Loop 1)
    pub fn record_outcome(&mut self, category: &ContextCategory, was_useful: bool) {
        let dist = self.feedback.entry(*category).or_insert(BetaDistribution::uniform());
        if was_useful { dist.alpha += 1.0; } else { dist.beta += 1.0; }
    }

    /// Evolve allocations based on accumulated feedback (Loop 2, every 50 ticks)
    pub fn evolve(&mut self, max_delta: f64) {
        for (cat, dist) in &self.feedback {
            let value = dist.mean(); // alpha / (alpha + beta)
            if let Some(alloc) = self.allocations.get_mut(cat) {
                let delta = (value - 0.5) * max_delta * 2.0;
                *alloc = (*alloc + delta).clamp(0.01, 0.5);
            }
        }
        self.normalize();
        self.revision += 1;
    }
}
```

## Event Fabric

```rust
pub struct EventFabric {
    tx: broadcast::Sender<RuntimeEvent>,
    ring: Arc<RwLock<VecDeque<RuntimeEvent>>>,
    ring_capacity: usize,  // default: 10,000
    seq: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct RuntimeEvent {
    pub seq: u64,
    pub timestamp: Instant,
    pub source: EventSource,
    pub payload: EventPayload,
}

#[derive(Clone, Debug)]
pub enum EventSource {
    Chain { chain_id: u64 },
    FileSystem { path: PathBuf },
    Agent { agent_id: AgentId },
    Gate { rung: String },
    Timer { name: String },
    External { source: String },
}

#[derive(Clone, Debug)]
pub enum EventPayload {
    // Chain events
    NewBlock { number: u64, timestamp: u64, tx_count: u32 },
    Transaction { hash: H256, from: Address, to: Address, value: U256, data: Bytes },
    PriceFeed { pair: String, price: f64, source: String },

    // File events
    FileChanged { path: PathBuf, kind: FileChangeKind },
    TestResult { suite: String, passed: u32, failed: u32 },

    // Agent events
    AgentStarted { agent_id: AgentId, domain: String },
    AgentCompleted { agent_id: AgentId, outcome: TaskOutcome },
    PheromoneSignal { source: AgentId, signal_type: String, intensity: f64 },

    // Gate events
    GateVerdict { rung: String, passed: bool, output: String },

    // Timer events
    HeartbeatTick { frequency: Frequency, tick: u64 },

    // Generic
    Custom { kind: String, data: serde_json::Value },
}

pub enum EventFilter {
    NewBlock,
    MempoolTx,
    PriceFeed,
    FileChange,
    TestResult,
    GateVerdict,
    Pheromone,
    NewPublication,
    DataUpdate,
    KnowledgeChange,
    Custom(String),
}

impl EventFabric {
    pub fn new(capacity: usize) -> Self;
    pub fn emit(&self, source: EventSource, payload: EventPayload);
    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent>;
    pub fn subscribe_filtered(&self, filters: &[EventFilter]) -> FilteredReceiver;
    pub fn replay_from(&self, seq: u64) -> Vec<RuntimeEvent>;
}
```

## CorticalState (Lock-Free Shared Perception)

```rust
/// Lock-free atomic state surface for concurrent subsystem reads/writes.
/// Each field is a single atomic — no locks, no contention.
pub struct CorticalState {
    // Affect (written by Daimon extension)
    pub pleasure: AtomicI32,    // fixed-point: value * 1000
    pub arousal: AtomicI32,
    pub dominance: AtomicI32,
    pub behavioral_phase: AtomicU8,

    // Vitality (written by Mortality extension)
    pub economic_vitality: AtomicU16,
    pub epistemic_confidence: AtomicU16,
    pub composite_vitality: AtomicU16,

    // Perception (written by Observer/Chain extensions)
    pub tick_count: AtomicU64,
    pub last_observation_ms: AtomicU64,
    pub prediction_error: AtomicU32, // fixed-point: value * 10000
    pub cognitive_tier: AtomicU8,

    // Communication (written by Social extensions)
    pub pheromone_signal: AtomicU64,
    pub attention_top_hash: AtomicU64,
}
```

## Concrete Extensions (Initial Set)

### Required for all agents:
1. **HeartbeatExt** (L0) — tick scheduling, frequency management
2. **ContextExt** (L0) — CognitiveWorkspace assembly + ContextPolicy evolution
3. **DaimonExt** (L3) — affect modulation, somatic markers
4. **LearningExt** (L6) — episode recording, outcome feedback
5. **DreamsExt** (L6) — sleep pressure, consolidation cycle

### Coding domain additions:
6. **GitExt** (L4) — worktree management, changed files, commit
7. **GateExt** (L4) — compile/test/clippy verification
8. **ConductorExt** (L3) — 10 watchers, circuit breaker, stuck detection

### Blockchain domain additions:
9. **ChainSubscriberExt** (L1) — block/tx subscription, triage pipeline
10. **RiskExt** (L3) — 5-layer risk assessment
11. **MortalityExt** (L2) — 3 death clocks, behavioral phases
12. **PheromoneExt** (L5) — inter-agent field communication

### Research domain additions:
13. **KnowledgeGraphExt** (L2) — graph queries, citation tracking
14. **SourceWatcherExt** (L1) — new paper/data detection
15. **SynthesisExt** (L6) — hypothesis generation, cross-source reasoning

## Agent Spawning (Backwards Compatible)

```rust
/// Create an agent from a domain profile (new path)
pub async fn spawn_agent(
    profile: DomainProfile,
    config: AgentConfig,
    event_fabric: Arc<EventFabric>,
) -> Result<Agent<Active>> {
    let mut builder = ExtensionChain::builder();

    // Load extensions based on profile
    for ext_name in &profile.extensions {
        let ext = extension_registry::create(ext_name, &config)?;
        builder = builder.add(ext);
    }

    let chain = builder.build()?;
    let agent = Agent::<Provisioning>::new(config, event_fabric);
    let agent = agent.activate(chain);
    Ok(agent)
}

/// Inject a task into a running agent (backwards compat with plan execution)
pub async fn dispatch_task(
    agent: &mut Agent<Active>,
    task: TaskEnvelope,
) -> Result<TaskOutcome> {
    // Task becomes a forced T2 stimulus
    agent.on_task(task).await
}

/// One-shot spawn for plan-based execution (transitional API)
pub async fn spawn_and_run_task(
    profile: DomainProfile,
    config: AgentConfig,
    task: TaskEnvelope,
    event_fabric: Arc<EventFabric>,
) -> Result<TaskOutcome> {
    let mut agent = spawn_agent(profile, config, event_fabric).await?;
    let outcome = agent.on_task(task).await?;
    let _genome = agent.begin_death().extract_genome().await;
    Ok(outcome)
}
```

## Crate Layout

```
crates/
  roko-runtime/        # REWRITE: AgentRuntime trait, lifecycle, heartbeat, event fabric
    src/
      lib.rs
      runtime.rs       # AgentRuntime trait + Agent<Phase> type-state
      heartbeat.rs     # HeartbeatPipeline (9-step tick)
      lifecycle.rs     # State transitions, shutdown protocol
      event_fabric.rs  # EventFabric, RuntimeEvent, EventFilter
      cortical.rs      # CorticalState (lock-free atomics)
      arena.rs         # TickArena (bumpalo per-tick allocation)
      cognitive.rs     # CognitiveWorkspace, ContextPolicy, learnable control
      extension.rs     # Extension trait, ExtensionChain, ExtensionLayer
      profile.rs       # DomainProfile definitions

  roko-ext-core/       # NEW: Core extensions (required for all agents)
    src/
      heartbeat.rs     # HeartbeatExt
      context.rs       # ContextExt (workspace assembly)
      daimon.rs        # DaimonExt (affect, somatic)
      learning.rs      # LearningExt (episodes, outcomes)
      dreams.rs        # DreamsExt (sleep pressure, consolidation)

  roko-ext-code/       # NEW: Coding-domain extensions
    src/
      git.rs           # GitExt (worktree, commit, changed files)
      gate.rs          # GateExt (compile/test/clippy)
      conductor.rs     # ConductorExt (watchers, circuit breaker)

  roko-ext-chain/      # NEW: Blockchain-domain extensions
    src/
      subscriber.rs    # ChainSubscriberExt (block/tx ingestion)
      risk.rs          # RiskExt (5-layer assessment)
      mortality.rs     # MortalityExt (3 death clocks)
      pheromone.rs     # PheromoneExt (field communication)

  roko-ext-research/   # NEW: Research-domain extensions
    src/
      knowledge_graph.rs # KnowledgeGraphExt
      source_watcher.rs  # SourceWatcherExt
      synthesis.rs       # SynthesisExt

  roko-agent/          # KEEP: LLM backends (Claude, Codex, etc.)
  roko-core/           # KEEP: Signal + 6 traits, types
  roko-compose/        # KEEP: Prompt templates (used by ContextExt)
  roko-gate/           # KEEP: Gate implementations (used by GateExt)
  roko-learn/          # KEEP: Learning runtime (used by LearningExt)
  roko-neuro/          # KEEP: Knowledge store (used by ContextExt, DreamsExt)
  roko-daimon/         # KEEP: Affect engine (used by DaimonExt)
  roko-dreams/         # KEEP: Dream cycle (used by DreamsExt)
  roko-conductor/      # KEEP: Watchers (used by ConductorExt)
  roko-chain/          # KEEP: Chain client/wallet (used by ChainSubscriberExt)
```

## Migration Path from orchestrate.rs

### Phase 1: Extract Runtime Crate
- Move `AgentRuntime` trait + `Agent<Phase>` to `roko-runtime`
- Move `EventFabric` (currently `RuntimeEventBus`) to `roko-runtime`
- Move `CorticalState` (new) to `roko-runtime`
- Move `HeartbeatPipeline` (currently metadata-only) to `roko-runtime`
- Move `DomainProfile` to `roko-runtime`

### Phase 2: Extract Core Extensions
- Extract DaimonState calls from orchestrate.rs → `DaimonExt`
- Extract learning/episode calls → `LearningExt`
- Extract conductor calls → `ConductorExt`
- Extract dream triggers → `DreamsExt`
- Extract context assembly → `ContextExt`

### Phase 3: Rewrite PlanRunner
- `PlanRunner` becomes a thin coordinator that:
  - Discovers plans
  - Spawns `Agent<Active>` per domain profile
  - Injects tasks as `TaskEnvelope` stimuli
  - Collects outcomes
- Each agent runs its own heartbeat and extensions
- The 137-field monolith dissolves into extension state

### Phase 4: Wire Event Fabric
- Chain subscriber extension emits `NewBlock` events
- File watcher emits `FileChanged` events
- Gate results emit `GateVerdict` events
- Agents subscribe based on their domain profile

### Phase 5: Persistent Chat + Deployment
- Agent serves a WebSocket for operator chat
- `on_message()` hook in extensions processes operator input
- Deployment: single binary (`roko agent start --profile blockchain`)
- Remote: same binary in container, WebSocket exposed
