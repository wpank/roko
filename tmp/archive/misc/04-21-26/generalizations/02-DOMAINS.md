# Domain specialization, native harness, and blue ocean strategy

## 1. Domain profiles -- the parameterization layer

Roko's central design bet is that one runtime serves every domain. A blockchain agent and a coding agent share the same heartbeat pipeline, extension chain, and cognitive gating logic. What differs is configuration: tick frequency, which extensions load, what events the agent subscribes to, and how aggressively the gate suppresses LLM calls.

This configuration surface is the **DomainProfile**.

### Why profiles exist

Without profiles, you need one agent framework per domain. LangChain for coding, a separate event-driven system for blockchain, something else for research. Each reimplements the same LLM loop, the same tool dispatch, the same error handling. Profiles collapse that into a single runtime parameterized by a struct.

The runtime does not know or care what domain it operates in. It ticks its heartbeat, fires extensions in layer order, gates cognition, and records episodes. Domain-specific behavior lives in extensions. The profile just declares which extensions load and how fast the clock runs.

### The DomainProfile struct

```rust
/// Controls how an agent behaves within a specific domain.
/// This is pure configuration -- no domain logic lives here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainProfile {
    /// Human-readable name (e.g., "blockchain", "research", "coding")
    pub name: String,

    /// Gamma interval: perception tick. How often the agent observes its environment.
    /// Fast for reactive domains (blockchain: 5s), slow for deliberate ones (research: 60s).
    pub gamma_interval: Duration,

    /// Theta interval: decision tick. How often the agent runs a full decision cycle
    /// (observe → gate → assemble → infer → act → reflect).
    pub theta_interval: Duration,

    /// Delta interval: consolidation tick. How often the agent enters dream state
    /// to replay episodes, discover patterns, and evolve strategies.
    pub delta_interval: Duration,

    /// Base gate threshold for prediction error. Below this, ticks are T0 (no LLM, $0).
    /// Lower values = more reactive (more ticks escalate). Higher = more conservative.
    pub base_gate_threshold: f64,

    /// Which extensions to load. Order does not matter -- the chain sorts by layer.
    pub extensions: Vec<String>,

    /// Which event types this agent subscribes to from the EventFabric.
    pub event_subscriptions: Vec<EventFilter>,

    /// Which categories of context to assemble for LLM calls.
    pub context_categories: Vec<ContextCategory>,

    /// Default verification gates for this domain.
    pub default_gates: Vec<String>,

    /// Whether the agent manages git repositories.
    pub uses_git: bool,

    /// Whether the agent creates worktrees for parallel branches.
    pub uses_worktrees: bool,

    /// Maximum concurrent tool calls (domain-dependent parallelism).
    pub max_concurrent_tools: usize,

    /// Budget ceiling in USD before the agent enters Conservation phase.
    pub budget_ceiling_usd: Option<f64>,

    /// Custom metadata for domain-specific configuration.
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Predefined profiles

The system ships with six profiles. Each one reflects the operational characteristics of its domain.

#### Coding

```rust
impl DomainProfile {
    pub fn coding() -> Self {
        Self {
            name: "coding".into(),
            gamma_interval: Duration::from_secs(30),
            theta_interval: Duration::from_secs(120),
            delta_interval: Duration::from_secs(6000), // ~100 min
            base_gate_threshold: 0.4,
            extensions: vec![
                "heartbeat".into(), "context".into(), "neuro".into(),
                "daimon".into(), "conductor".into(), "tools".into(),
                "git".into(), "gate".into(), "safety".into(),
                "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![
                EventFilter::FileChange,
                EventFilter::TestResult,
                EventFilter::GateVerdict,
            ],
            context_categories: vec![
                ContextCategory::Task,
                ContextCategory::CodeContext,
                ContextCategory::Knowledge,
                ContextCategory::Playbook,
            ],
            default_gates: vec!["compile".into(), "test".into(), "clippy".into()],
            uses_git: true,
            uses_worktrees: true,
            max_concurrent_tools: 4,
            budget_ceiling_usd: None, // task-scoped, managed by orchestrator
            metadata: HashMap::new(),
        }
    }
}
```

**Rationale**: Code does not change fast between ticks -- the agent itself is the one modifying files. A 30s gamma is enough to detect file-change events from external editors or CI. The 120s theta gives the LLM enough time to complete a tool-use loop before the next decision point. The 0.4 gate threshold is relatively high because most coding ticks during a task are "continue working" -- only novel compiler errors or test failures should escalate.

#### Blockchain

```rust
    pub fn blockchain() -> Self {
        Self {
            name: "blockchain".into(),
            gamma_interval: Duration::from_secs(5),
            theta_interval: Duration::from_secs(30),
            delta_interval: Duration::from_secs(1500), // 25 min
            base_gate_threshold: 0.2,
            extensions: vec![
                "heartbeat".into(), "context".into(), "neuro".into(),
                "daimon".into(), "tools".into(), "chain-subscriber".into(),
                "price-feed".into(), "risk".into(), "mortality".into(),
                "safety".into(), "pheromones".into(), "strategy-store".into(),
                "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![
                EventFilter::NewBlock,
                EventFilter::MempoolTx,
                EventFilter::PriceFeed,
                EventFilter::Pheromone,
            ],
            context_categories: vec![
                ContextCategory::Strategy,
                ContextCategory::Positions,
                ContextCategory::MarketState,
                ContextCategory::Knowledge,
                ContextCategory::Risk,
                ContextCategory::Mortality,
            ],
            default_gates: vec![
                "simulation".into(),
                "invariant-check".into(),
                "risk-limit".into(),
            ],
            uses_git: false,
            uses_worktrees: false,
            max_concurrent_tools: 2, // sequential tx safety
            budget_ceiling_usd: Some(1000.0),
            metadata: HashMap::from_iter([
                ("chains".into(), json!(["ethereum", "base"])),
                ("max_gas_gwei".into(), json!(100)),
                ("kelly_fraction".into(), json!(0.25)),
            ]),
        }
    }
```

**Rationale**: Blockchain agents must react within block times (12s on Ethereum, 2s on L2s). The 5s gamma ensures every block is observed. The 0.2 threshold is low because chain state changes are often meaningful -- a new block with a relevant transaction should escalate quickly. Mortality is mandatory: a blockchain agent that runs forever without economic pressure has no incentive to optimize capital allocation.

#### Research

```rust
    pub fn research() -> Self {
        Self {
            name: "research".into(),
            gamma_interval: Duration::from_secs(60),
            theta_interval: Duration::from_secs(300), // 5 min
            delta_interval: Duration::from_secs(14400), // 4 hr
            base_gate_threshold: 0.35,
            extensions: vec![
                "heartbeat".into(), "context".into(), "neuro".into(),
                "daimon".into(), "tools".into(), "knowledge-graph".into(),
                "source-watcher".into(), "hypothesis".into(),
                "citation-store".into(), "synthesis".into(),
                "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![
                EventFilter::NewPublication,
                EventFilter::DataUpdate,
                EventFilter::KnowledgeChange,
            ],
            context_categories: vec![
                ContextCategory::Topic,
                ContextCategory::Sources,
                ContextCategory::Knowledge,
                ContextCategory::Hypotheses,
            ],
            default_gates: vec![
                "citation-check".into(),
                "factual-consistency".into(),
                "quality".into(),
            ],
            uses_git: false,
            uses_worktrees: false,
            max_concurrent_tools: 6, // parallel web fetches
            budget_ceiling_usd: None,
            metadata: HashMap::new(),
        }
    }
```

**Rationale**: Research is deliberate. New papers appear on the scale of hours, not seconds. A 60s gamma catches RSS/arxiv updates without waste. The 4-hour delta gives the dream cycle enough accumulated episodes to find real patterns -- consolidating too often produces noise. The 0.35 threshold balances reactivity (new contradictory evidence should escalate) against economy (most known-topic updates are T0 suppressible).

#### Documentation

```rust
    pub fn docs() -> Self {
        Self {
            name: "docs".into(),
            gamma_interval: Duration::from_secs(60),
            theta_interval: Duration::from_secs(180),
            delta_interval: Duration::from_secs(7200), // 2 hr
            base_gate_threshold: 0.45,
            extensions: vec![
                "heartbeat".into(), "context".into(), "neuro".into(),
                "daimon".into(), "tools".into(), "git".into(),
                "safety".into(), "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![
                EventFilter::FileChange,
                EventFilter::Custom("doc-request".into()),
            ],
            context_categories: vec![
                ContextCategory::Task,
                ContextCategory::CodeContext,
                ContextCategory::Knowledge,
                ContextCategory::StyleGuide,
            ],
            default_gates: vec![
                "spell-check".into(),
                "link-check".into(),
                "style-consistency".into(),
            ],
            uses_git: true,
            uses_worktrees: false,
            max_concurrent_tools: 4,
            budget_ceiling_usd: None,
            metadata: HashMap::new(),
        }
    }
```

**Rationale**: Documentation agents write prose, not code. Their gates verify language quality rather than compilation. High gate threshold (0.45) because docs tasks tend to be straightforward once the context is assembled -- the interesting work is context gathering, not decision-making.

#### Security

```rust
    pub fn security() -> Self {
        Self {
            name: "security".into(),
            gamma_interval: Duration::from_secs(120),
            theta_interval: Duration::from_secs(600),
            delta_interval: Duration::from_secs(36000), // 10 hr
            base_gate_threshold: 0.3,
            extensions: vec![
                "heartbeat".into(), "context".into(), "neuro".into(),
                "daimon".into(), "tools".into(), "vuln-scanner".into(),
                "dependency-audit".into(), "safety".into(),
                "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![
                EventFilter::FileChange,
                EventFilter::Custom("cve-published".into()),
                EventFilter::Custom("dependency-update".into()),
            ],
            context_categories: vec![
                ContextCategory::Vulnerabilities,
                ContextCategory::CodeContext,
                ContextCategory::Knowledge,
                ContextCategory::RemediationPlaybook,
            ],
            default_gates: vec![
                "static-analysis".into(),
                "dependency-audit".into(),
                "cve-check".into(),
            ],
            uses_git: true,
            uses_worktrees: false,
            max_concurrent_tools: 4,
            budget_ceiling_usd: None,
            metadata: HashMap::new(),
        }
    }
```

**Rationale**: Security scans are thorough and slow. The 120s gamma is sufficient -- CVEs don't drop every minute. The 0.3 threshold is moderately low because novel vulnerabilities should always escalate to full reasoning. The 10-hour dream cycle accumulates enough audit data to find patterns across codebases.

#### Custom (user-defined)

```rust
    pub fn custom(name: &str) -> Self {
        Self {
            name: name.into(),
            gamma_interval: Duration::from_secs(30),
            theta_interval: Duration::from_secs(120),
            delta_interval: Duration::from_secs(6000),
            base_gate_threshold: 0.35,
            extensions: vec![
                "heartbeat".into(), "context".into(), "daimon".into(),
                "learning".into(), "dreams".into(),
            ],
            event_subscriptions: vec![],
            context_categories: vec![ContextCategory::Task, ContextCategory::Knowledge],
            default_gates: vec![],
            uses_git: false,
            uses_worktrees: false,
            max_concurrent_tools: 4,
            budget_ceiling_usd: None,
            metadata: HashMap::new(),
        }
    }
```

Custom profiles start with minimum viable extensions (heartbeat, context, affect, learning, dreams) and let the user add domain-specific behavior through configuration or code.

---

## 2. Blockchain agent (subscriber-first design)

### What it does

A blockchain agent is a long-lived process that subscribes to chain events, triages them through a fast deterministic pipeline, and only invokes LLM reasoning when something novel or high-value appears. It manages positions, executes strategies, learns from outcomes, and communicates with peer agents through a pheromone field.

This is fundamentally different from a coding agent. A coding agent receives a task, works on it, and dies. A blockchain agent runs indefinitely, perceiving and reacting to its environment like an organism in an ecosystem.

The lifecycle:

1. **Subscribe** to block headers, pending transactions, price feeds
2. **Triage** every event through fast T0 filters (bloom filters, thresholds, ABI pattern matching)
3. **Escalate** novel or high-value events to T1/T2 reasoning
4. **Execute** strategies (swaps, LP management, vault rebalancing)
5. **Learn** from outcomes (episodes recorded, playbooks evolved)
6. **Dream** periodically (consolidate episodes into strategy refinements)
7. **Die** when economic viability exhausts (budget clock reaches zero)

### Tick schedule

| Frequency | Interval | What happens | Typical cost |
|-----------|----------|--------------|-------------|
| Gamma | 5s | Read latest block, triage transactions, update CorticalState | $0 (T0, pure Rust) |
| Theta | 30s | Full decision cycle: observe, gate, assemble context, infer, execute, reflect | $0-0.05 (T1/T2 conditional) |
| Delta | 25min | Dream: replay episodes, evolve strategies, consolidate knowledge, prune stale data | $0.01 (batch embedding) |

Over 24 hours: 17,280 gamma ticks (all free), 2,880 theta ticks (~80% suppressed = 576 paid), 58 delta ticks. Expected daily cost: $6-30 depending on market volatility.

### Extension chain (14 extensions)

```rust
ExtensionChain::builder()
    // L0: Foundation -- timing, state surface, lifecycle management
    .add(HeartbeatExt::new(Duration::from_secs(5)))
    .add(ContextExt::new(ContextPolicy::blockchain()))

    // L1: Perception -- data ingestion from external world
    .add(ChainSubscriberExt::new(ChainConfig {
        rpc_url: "wss://eth-mainnet.example.com",
        chains: vec![Chain::Ethereum, Chain::Base],
        interest_filter: BinaryFuse8::from_addresses(&watched_addresses),
        mempool_monitoring: true,
    }))
    .add(PriceFeedExt::new(vec![
        PricePair::new("ETH", "USD"),
        PricePair::new("BTC", "USD"),
        PricePair::new("ARB", "USD"),
    ]))

    // L2: Memory -- persistent knowledge and strategy state
    .add(NeuroExt::new(knowledge_store.clone()))
    .add(StrategyStoreExt::new(strategy_path))
    .add(MortalityExt::new(MortalityConfig {
        budget_usdc: 1000.0,
        epistemic_decay_rate: 0.001,
        stochastic_halflife_ticks: 10_000,
    }))

    // L3: Cognition -- affect, risk assessment, attention
    .add(DaimonExt::new(DaimonConfig::blockchain()))
    .add(RiskExt::new(RiskConfig {
        max_position_pct: 0.05,
        max_gas_gwei: 100,
        kelly_fraction: 0.25,
        max_drawdown_pct: 0.15,
        correlation_limit: 0.7,
    }))

    // L4: Action -- tool dispatch and safety enforcement
    .add(ToolsExt::new(vec![
        "balance_of", "send_tx", "simulate_tx", "approve",
        "swap", "add_liquidity", "remove_liquidity",
        "get_position", "get_pool_state",
    ]))
    .add(SafetyExt::new(SafetyConfig::blockchain()))

    // L5: Social -- inter-agent communication
    .add(PheromoneExt::new(PheromoneConfig {
        field_radius: 5,
        decay_rate: 0.1,
        signal_types: vec!["opportunity", "threat", "congestion"],
    }))

    // L6: Meta -- learning and consolidation
    .add(LearningExt::new(LearningConfig {
        episode_format: EpisodeFormat::Structured,
        min_episodes_for_playbook: 10,
    }))
    .add(DreamsExt::new(DreamsConfig {
        min_episodes_for_dream: 50,
        sleep_pressure_threshold: 0.8,
        dream_phases: vec![
            DreamPhase::Replay,
            DreamPhase::Counterfactual,
            DreamPhase::ThreatRehearsal,
            DreamPhase::StrategyEvolution,
        ],
    }))
    .build()?
```

### ChainSubscriberExt (block subscription and triage)

```rust
/// Subscribes to chain events and classifies them through a fast triage pipeline.
/// All triage is T0 -- no LLM, no latency, no cost.
pub struct ChainSubscriberExt {
    config: ChainConfig,
    client: Option<Box<dyn ChainClient>>,
    triage_pipeline: TriagePipeline,
    latest_block: u64,
    pending_events: VecDeque<ChainEvent>,
    block_processing_time: DDSketch, // latency tracking
}

impl Extension for ChainSubscriberExt {
    fn name(&self) -> &str { "chain-subscriber" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> {
        // Establish WebSocket connection to chain RPC
        self.client = Some(create_chain_client(&self.config).await?);

        // Subscribe to newHeads (new blocks)
        let block_rx = self.client.as_ref().unwrap()
            .subscribe_blocks().await?;

        // Subscribe to pendingTransactions (mempool)
        let mempool_rx = if self.config.mempool_monitoring {
            Some(self.client.as_ref().unwrap().subscribe_pending_txs().await?)
        } else {
            None
        };

        // Spawn background task that drains subscriptions into pending_events
        ctx.spawn_background(Self::drain_subscriptions(block_rx, mempool_rx));
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        let start = Instant::now();

        // Drain all buffered chain events, triage each one
        while let Some(event) = self.pending_events.pop_front() {
            let classification = self.triage_pipeline.classify(&event);

            match classification {
                Classification::Suppress => {
                    // T0 handled: do nothing, costs $0
                    ctx.increment_suppressed();
                }
                Classification::Escalate(tier) => {
                    ctx.add_observation(Observation::Chain {
                        event,
                        classification,
                        tier,
                    });
                }
            }
        }

        self.block_processing_time.add(start.elapsed().as_micros() as f64);
        Ok(())
    }

    async fn on_event(&mut self, event: &RuntimeEvent, _ctx: &mut EventContext) -> Result<()> {
        if let EventPayload::NewBlock { number, .. } = &event.payload {
            self.latest_block = *number;
        }
        self.pending_events.push_back(ChainEvent::from(event));
        Ok(())
    }

    async fn save_state(&self) -> Result<serde_json::Value> {
        Ok(json!({
            "latest_block": self.latest_block,
            "p50_latency_us": self.block_processing_time.quantile(0.5),
            "p99_latency_us": self.block_processing_time.quantile(0.99),
        }))
    }
}
```

### Triage pipeline (4-stage, all T0/Rust, no LLM)

The triage pipeline is the core economic mechanism. It processes every chain event in microseconds and decides whether the event deserves expensive LLM attention. Four stages, each progressively more expensive but still sub-millisecond:

```rust
pub struct TriagePipeline {
    /// Stage 1: Rule-based filters (bloom lookup, threshold check, pattern match)
    /// Latency: 1-5 microseconds per event
    rules: Vec<Box<dyn TriageRule>>,

    /// Stage 2: Statistical anomaly detection (MIDAS-R streaming, DDSketch quantiles)
    /// Latency: 10-50 microseconds per event
    anomaly: AnomalyDetector,

    /// Stage 3: Contextual enrichment (ABI resolution from cache, decoded function selectors)
    /// Latency: 50-200 microseconds per event (cache hit) or 1-5ms (cache miss)
    enricher: TxEnricher,

    /// Stage 4: Scoring (Thompson sampling with discounted hedge weighting)
    /// Latency: 5-20 microseconds per event
    scorer: DiscountedHedge,
}

impl TriagePipeline {
    pub fn classify(&self, event: &ChainEvent) -> Classification {
        // Stage 1: Fast reject/accept via rules
        // Most events (>90%) are eliminated here.
        for rule in &self.rules {
            if let Some(verdict) = rule.check(event) {
                return verdict;
            }
        }

        // Stage 2: Statistical anomaly detection
        // Catches events that rules don't cover -- unusual gas, unusual value,
        // unusual frequency from a known address.
        if self.anomaly.is_anomalous(event) {
            return Classification::Escalate(CognitiveTier::T1);
        }

        // Stage 3: Contextual enrichment
        // Decode the transaction (function selector → method name), resolve
        // contract ABIs from cache, identify DEX swaps vs. transfers vs. governance.
        let enriched = self.enricher.enrich(event);

        // Stage 4: Score via Thompson sampling
        // Each event type has a Beta distribution tracking historical escalation value.
        // Thompson sampling explores uncertain event types while exploiting known-valuable ones.
        let score = self.scorer.score(&enriched);

        if score > 0.7 {
            Classification::Escalate(CognitiveTier::T2) // full deliberation
        } else if score > 0.3 {
            Classification::Escalate(CognitiveTier::T1) // cheap model, minimal context
        } else {
            Classification::Suppress // T0, no cost
        }
    }
}

// Example rules:
pub struct WatchedAddressRule {
    filter: BinaryFuse8, // xor filter, O(1) lookup, <1 byte per address
}

pub struct ValueThresholdRule {
    min_value_eth: f64,
}

pub struct FunctionSelectorRule {
    /// 4-byte selectors for functions we care about (swap, addLiquidity, etc.)
    selectors: HashSet<[u8; 4]>,
}

pub struct GasAnomalyRule {
    /// Running p99 gas price; trigger if current > 2x p99
    gas_sketch: DDSketch,
    multiplier: f64,
}
```

### Strategy lifecycle

Strategies are the blockchain agent's equivalent of tasks. They define what the agent is trying to accomplish and how it evaluates success. The lifecycle:

```
STRATEGY.md (operator-authored, human-readable intent)
  ↓ parsed at boot
StrategyParams (typed configuration: entry/exit conditions, position sizing)
  ↓ evaluated every theta tick
CorticalState signals (real-time: price, volatility, position health)
  ↓ compared against expectations
Prediction Error (how much does reality differ from what the strategy expects?)
  ↓ gates cognition
Gating Decision:
  T0: ignore (within normal parameters, no action needed)
  T1: adjust (minor deviation, cheap model tweaks parameters)
  T2: deliberate (major deviation or novel situation, full reasoning)
  ↓ after execution
Episode Recording (what conditions existed, what action was taken, what outcome)
  ↓ every 50 episodes
Playbook Evolution (dream cycle extracts patterns from episodes)
  ↓ feeds back into
Strategy Adaptation (StrategyParams evolve based on learned patterns)
```

```rust
/// Operator-authored strategy definition
#[derive(Debug, Deserialize)]
pub struct StrategyParams {
    pub name: String,
    pub description: String,

    // Entry conditions
    pub entry_signals: Vec<SignalCondition>,
    pub min_confidence: f64,

    // Position sizing
    pub position_method: PositionMethod, // Kelly, Fixed, Volatility-scaled
    pub max_position_pct: f64,

    // Exit conditions
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub max_hold_duration: Duration,

    // Risk limits
    pub max_correlation: f64,
    pub max_drawdown_pct: f64,

    // Learning parameters
    pub adaptation_rate: f64, // how quickly params evolve from episode feedback
}
```

### Mortality (3 death clocks, 5 behavioral phases)

Mortality is not a safety feature. It is an architectural mechanism that creates economic pressure to optimize. An immortal agent has no incentive to be efficient. A mortal agent that knows its budget is finite must choose wisely.

```rust
pub struct MortalityExt {
    /// Clock 1: Economic -- USDC budget remaining.
    /// Decreases with every paid inference and every gas-spending transaction.
    /// When this reaches zero, the agent enters Terminal phase.
    economic_clock: f64,

    /// Clock 2: Epistemic -- knowledge confidence.
    /// Decays over time (Ebbinghaus curve) without validation.
    /// An agent whose knowledge is stale becomes unreliable.
    /// High decay without replenishment forces dream cycles or shutdown.
    epistemic_clock: f64,

    /// Clock 3: Stochastic -- random survival.
    /// Each tick has a tiny probability of triggering shutdown.
    /// Half-life configurable (default: 10,000 ticks ~ 14 hours at 5s gamma).
    /// Prevents immortal agents. Forces genome extraction and successor spawning.
    stochastic_clock: f64,

    /// Composite vitality (weighted average of all three clocks).
    /// Drives behavioral phase transitions.
    composite_vitality: f64,

    /// Current behavioral phase.
    phase: BehavioralPhase,

    /// Phase-specific configuration (risk limits, model selection, etc.)
    phase_config: PhaseConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BehavioralPhase {
    /// Vitality > 0.8: aggressive strategies, full model budget, explore freely
    Thriving,

    /// Vitality 0.5-0.8: balanced operation, standard risk limits
    Stable,

    /// Vitality 0.3-0.5: reduce position sizes, prefer cheaper models,
    /// increase gate threshold (fewer paid ticks), start genome preparation
    Conservation,

    /// Vitality 0.1-0.3: minimal activity, close positions, prepare genome,
    /// broadcast pheromone signals for successors to inherit
    Declining,

    /// Vitality < 0.1: shutdown initiated, extract genome, broadcast death signal
    Terminal,
}

impl MortalityExt {
    fn update_phase(&mut self) -> Option<PhaseTransition> {
        let new_phase = match self.composite_vitality {
            v if v > 0.8 => BehavioralPhase::Thriving,
            v if v > 0.5 => BehavioralPhase::Stable,
            v if v > 0.3 => BehavioralPhase::Conservation,
            v if v > 0.1 => BehavioralPhase::Declining,
            _ => BehavioralPhase::Terminal,
        };

        if new_phase != self.phase {
            let transition = PhaseTransition {
                from: self.phase,
                to: new_phase,
                vitality: self.composite_vitality,
            };
            self.phase = new_phase;
            Some(transition)
        } else {
            None
        }
    }
}
```

Mortality pressure modifies the agent's behavior at every level:

| Phase | Gate threshold | Model budget | Position sizing | Risk tolerance |
|-------|--------------|--------------|-----------------|----------------|
| Thriving | Base (0.2) | Unconstrained | Full Kelly | Standard |
| Stable | Base | Standard | 80% Kelly | Standard |
| Conservation | Base + 0.1 | Prefer cheap | 50% Kelly | Reduced |
| Declining | Base + 0.2 | Cheapest only | Close positions | Minimal |
| Terminal | Suppress all | None | Emergency close | Zero |

### Risk extension (5-layer assessment)

Every action the blockchain agent takes passes through a 5-layer risk assessment. Each layer can veto the action independently.

```rust
pub struct RiskExt {
    config: RiskConfig,
    position_tracker: PositionTracker,
    correlation_matrix: CorrelationMatrix,
    drawdown_tracker: DrawdownTracker,
    gas_oracle: GasOracle,
    somatic_history: SomaticHistory,
}

impl RiskExt {
    /// Five-layer assessment. Any layer can veto.
    pub fn assess(&self, action: &ProposedAction) -> RiskVerdict {
        // Layer 1: Position limits
        // Does this action exceed max_position_pct of portfolio?
        let position_ok = self.check_position_limits(action);

        // Layer 2: Correlation limits
        // Does this action create correlated exposure beyond threshold?
        let correlation_ok = self.check_correlation(action);

        // Layer 3: Drawdown limits
        // Is portfolio already in drawdown? Reduce new risk.
        let drawdown_ok = self.check_drawdown(action);

        // Layer 4: Gas economics
        // Does gas cost make this action uneconomical?
        let gas_ok = self.check_gas_economics(action);

        // Layer 5: Somatic markers
        // Has a similar action historically led to losses?
        let somatic_ok = self.check_somatic_history(action);

        if !position_ok { return RiskVerdict::Reject("position limit exceeded"); }
        if !correlation_ok { return RiskVerdict::Reject("correlation limit exceeded"); }
        if !drawdown_ok { return RiskVerdict::Reject("drawdown limit active"); }
        if !gas_ok { return RiskVerdict::Reject("gas cost uneconomical"); }
        if !somatic_ok { return RiskVerdict::Hesitate(self.somatic_confidence(action)); }

        RiskVerdict::Approve
    }
}
```

### Pheromone communication between peer agents

Blockchain agents do not communicate directly (no RPC, no message passing). They communicate through a shared field -- a spatial data structure that decays over time. This creates emergent coordination without explicit orchestration.

```rust
pub struct PheromoneExt {
    config: PheromoneConfig,
    field: PheromoneField,
    own_signals: VecDeque<PheromoneDeposit>,
}

/// Signal types in the pheromone field
pub enum PheromoneType {
    /// "I found something valuable here" -- attracts peers
    Opportunity { pool: Address, expected_apy: f64 },

    /// "Danger in this area" -- repels peers
    Threat { pool: Address, reason: String },

    /// "This area is crowded" -- disperses peers to avoid competition
    Congestion { pool: Address, agent_count: u32 },

    /// "I am dying, inherit my knowledge" -- successor attraction
    DeathSignal { genome_location: PathBuf },
}

impl PheromoneField {
    /// Deposit a signal. Decays exponentially over time.
    pub fn deposit(&mut self, signal: PheromoneDeposit) {
        self.grid.insert(signal.position, signal);
    }

    /// Read signals within radius of a position. Returns intensity-weighted list.
    pub fn sense(&self, position: &FieldPosition, radius: u32) -> Vec<(PheromoneType, f64)> {
        self.grid
            .within_radius(position, radius)
            .map(|deposit| {
                let age = deposit.timestamp.elapsed();
                let intensity = deposit.initial_intensity
                    * (-self.decay_rate * age.as_secs_f64()).exp();
                (deposit.signal_type.clone(), intensity)
            })
            .filter(|(_, intensity)| *intensity > 0.01) // below threshold = evaporated
            .collect()
    }
}
```

---

## 3. Research agent (knowledge-accumulating design)

### What it does

A research agent monitors information sources, synthesizes findings across sources, maintains hypotheses, and builds a persistent knowledge graph that improves retrieval over time. Unlike a coding agent (task-driven, dies after completion) or a blockchain agent (reactive, event-driven), the research agent is deliberate and accumulative.

Its core loop:

1. **Monitor** sources for new publications (arxiv, GitHub, RSS feeds, data APIs)
2. **Process** new sources through entity/relationship extraction
3. **Synthesize** across multiple sources to build composite understanding
4. **Test** hypotheses against accumulated evidence
5. **Consolidate** during dream cycles (find connections between disconnected clusters)
6. **Produce** artifacts (summaries, analyses, recommendations) on request

### Tick schedule

| Frequency | Interval | What happens | Typical cost |
|-----------|----------|--------------|-------------|
| Gamma | 60s | Check source feeds for new items, bloom-filter dedup | $0 (T0) |
| Theta | 5min | Full research cycle: read, extract, compare, synthesize | $0.01-0.10 (T1/T2) |
| Delta | 4hr | Dream: cluster discovery, cross-domain synthesis, knowledge pruning | $0.05 (batch) |

Over 24 hours: 1,440 gamma ticks (free), 288 theta ticks (~60% suppressed = 115 paid), 6 delta ticks. Expected daily cost: $3-15.

### Extension chain (12 extensions)

```rust
ExtensionChain::builder()
    // L0: Foundation
    .add(HeartbeatExt::new(Duration::from_secs(60)))
    .add(ContextExt::new(ContextPolicy::research()))

    // L1: Perception
    .add(SourceWatcherExt::new(SourceConfig {
        feeds: vec![
            SourceFeed::ArxivCategory("cs.AI"),
            SourceFeed::ArxivCategory("cs.CL"),
            SourceFeed::GithubRepo("anthropics/claude-code"),
            SourceFeed::GithubRepo("openai/codex"),
            SourceFeed::RssUrl("https://blog.research.example.com/feed.xml"),
            SourceFeed::HackerNews { min_score: 100, keywords: vec!["LLM", "agents"] },
        ],
        poll_interval: Duration::from_secs(300),
        dedup_filter: BloomFilter::new(100_000, 0.001),
    }))

    // L2: Memory
    .add(NeuroExt::new(knowledge_store.clone()))
    .add(KnowledgeGraphExt::new(KnowledgeGraphConfig {
        max_nodes: 50_000,
        max_edges: 200_000,
        embedding_model: "text-embedding-3-small",
        similarity_threshold: 0.85,
        decay_halflife: Duration::from_secs(86400 * 30), // 30 days
    }))
    .add(CitationStoreExt::new(citation_path))

    // L3: Cognition
    .add(DaimonExt::new(DaimonConfig::research()))
    .add(HypothesisExt::new(HypothesisConfig {
        max_active: 20,
        confidence_threshold_for_promotion: 0.8,
        evidence_required_for_rejection: 3,
    }))

    // L4: Action
    .add(ToolsExt::new(vec![
        "web_search", "web_fetch", "read_file", "write_file",
        "grep", "glob", "summarize", "extract_citations",
        "embed_text", "query_similar",
    ]))

    // L6: Meta
    .add(LearningExt::new(LearningConfig {
        episode_format: EpisodeFormat::Structured,
        min_episodes_for_playbook: 10,
    }))
    .add(DreamsExt::new(DreamsConfig {
        min_episodes_for_dream: 20,
        sleep_pressure_threshold: 0.6, // dreams more often than other domains
        dream_phases: vec![
            DreamPhase::Replay,
            DreamPhase::ClusterDiscovery,
            DreamPhase::CrossDomainSynthesis,
            DreamPhase::KnowledgePruning,
        ],
    }))
    .add(SynthesisExt::new(SynthesisConfig {
        min_sources_for_synthesis: 3,
        contradiction_threshold: 0.4,
    }))
    .build()?
```

### KnowledgeGraphExt (entity/relationship extraction, neighborhood queries, cluster detection)

```rust
pub struct KnowledgeGraphExt {
    graph: KnowledgeGraph,
    pending_entries: Vec<KnowledgeEntry>,
    synthesis_queue: Vec<SynthesisRequest>,
    embedding_cache: LruCache<String, Vec<f32>>,
}

pub struct KnowledgeGraph {
    nodes: HashMap<NodeId, KnowledgeNode>,
    edges: HashMap<EdgeId, KnowledgeEdge>,
    embeddings: HnswIndex, // approximate nearest neighbor for semantic search
    clusters: Vec<Cluster>,
    last_cluster_update: Instant,
}

#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    pub id: NodeId,
    pub entity: String,           // "Transformer architecture", "GPT-4", "RLHF"
    pub entity_type: EntityType,  // Concept, Paper, Person, Method, Dataset, Tool
    pub embedding: Vec<f32>,
    pub confidence: f64,          // decays over time without reinforcement
    pub first_seen: DateTime<Utc>,
    pub last_referenced: DateTime<Utc>,
    pub source_count: u32,        // how many independent sources mention this
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct KnowledgeEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub relation: Relation,       // Extends, Contradicts, Supports, UsedBy, PartOf, etc.
    pub confidence: f64,
    pub evidence: Vec<EvidenceRef>,
}

impl Extension for KnowledgeGraphExt {
    fn name(&self) -> &str { "knowledge-graph" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Memory }

    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> {
        // After each research action, extract entities and relationships
        let text = ctx.action_output();
        let entities = self.extract_entities(text).await?;
        let relationships = self.extract_relationships(text, &entities).await?;

        for entity in entities {
            let node = self.graph.upsert_node(entity);
            self.pending_entries.push(KnowledgeEntry::Node(node));
        }

        for rel in relationships {
            let edge = self.graph.add_edge(rel);
            self.pending_entries.push(KnowledgeEntry::Edge(edge));
        }

        Ok(())
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        // Inject relevant graph neighborhood into the LLM's context
        let topic = ws.current_focus();
        let neighborhood = self.graph.query_neighborhood(
            topic,
            QueryParams {
                max_depth: 2,
                max_nodes: 20,
                min_confidence: 0.3,
                sort_by: SortBy::Relevance,
            },
        );

        ws.add_section(ContextSection {
            category: ContextCategory::Knowledge,
            priority: 3,
            content: neighborhood.render_markdown(),
            tokens: neighborhood.estimated_tokens(),
            source: "knowledge-graph".into(),
            ..Default::default()
        });

        // Also inject active hypotheses that relate to this topic
        let relevant_hypotheses = self.graph.hypotheses_for_topic(topic);
        if !relevant_hypotheses.is_empty() {
            ws.add_section(ContextSection {
                category: ContextCategory::Hypotheses,
                priority: 2,
                content: render_hypotheses(&relevant_hypotheses),
                tokens: estimate_tokens(&relevant_hypotheses),
                source: "knowledge-graph-hypotheses".into(),
                ..Default::default()
            });
        }

        Ok(())
    }

    async fn on_dream_start(&mut self, ctx: &mut DreamContext) -> Result<()> {
        // During dream cycles, find disconnected clusters and queue synthesis
        self.graph.recompute_clusters();
        let clusters = self.graph.find_disconnected_clusters();

        for (cluster_a, cluster_b) in clusters.candidate_pairs() {
            // Only attempt synthesis if clusters are semantically close
            // but structurally disconnected (no edge path exists)
            let semantic_sim = cluster_a.centroid_similarity(&cluster_b);
            if semantic_sim > 0.4 {
                self.synthesis_queue.push(SynthesisRequest {
                    source_a: cluster_a.representative_topic(),
                    source_b: cluster_b.representative_topic(),
                    similarity: semantic_sim,
                    hypothesis: format!(
                        "What connects '{}' and '{}'? Similarity: {:.2}",
                        cluster_a.representative_topic(),
                        cluster_b.representative_topic(),
                        semantic_sim,
                    ),
                });
            }
        }

        ctx.set_synthesis_queue(self.synthesis_queue.drain(..).collect());
        Ok(())
    }
}
```

### SourceWatcherExt (arxiv, GitHub, RSS feeds)

```rust
pub struct SourceWatcherExt {
    config: SourceConfig,
    feeds: Vec<Box<dyn SourceFeed>>,
    seen: BloomFilter,
    pending_sources: VecDeque<SourceItem>,
    last_poll: HashMap<String, Instant>,
}

pub trait SourceFeed: Send + Sync {
    fn name(&self) -> &str;
    fn poll_interval(&self) -> Duration;
    async fn fetch_new(&mut self) -> Result<Vec<SourceItem>>;
}

pub struct ArxivFeed {
    category: String,
    last_seen_id: Option<String>,
}

impl SourceFeed for ArxivFeed {
    fn name(&self) -> &str { &self.category }
    fn poll_interval(&self) -> Duration { Duration::from_secs(300) }

    async fn fetch_new(&mut self) -> Result<Vec<SourceItem>> {
        let url = format!(
            "http://export.arxiv.org/api/query?search_query=cat:{}&sortBy=submittedDate&max_results=20",
            self.category
        );
        let response = reqwest::get(&url).await?;
        let entries = parse_arxiv_atom(&response.text().await?)?;

        let new_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                self.last_seen_id.as_ref()
                    .map(|last| e.id > *last)
                    .unwrap_or(true)
            })
            .collect();

        if let Some(newest) = new_entries.first() {
            self.last_seen_id = Some(newest.id.clone());
        }

        Ok(new_entries.into_iter().map(SourceItem::from).collect())
    }
}

impl Extension for SourceWatcherExt {
    fn name(&self) -> &str { "source-watcher" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        for feed in &mut self.feeds {
            let feed_name = feed.name().to_string();
            let last = self.last_poll.get(&feed_name)
                .copied()
                .unwrap_or(Instant::now() - feed.poll_interval());

            if last.elapsed() >= feed.poll_interval() {
                match feed.fetch_new().await {
                    Ok(items) => {
                        for item in items {
                            // Bloom filter deduplication
                            if !self.seen.check_and_set(&item.fingerprint()) {
                                ctx.add_observation(Observation::Custom {
                                    kind: "new_source".into(),
                                    data: serde_json::to_value(&item)?,
                                    prediction_error_contribution: item.novelty_estimate(),
                                });
                                self.pending_sources.push_back(item);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(feed = %feed_name, error = %e, "source poll failed");
                    }
                }
                self.last_poll.insert(feed_name, Instant::now());
            }
        }
        Ok(())
    }
}
```

### Research cycle (source detection, processing, synthesis, consolidation)

```
Source Detection (gamma tick, T0)
  New item arrives from SourceWatcherExt
  → Bloom filter: seen before?
    → Yes → suppress, no cost
    → No → compute novelty estimate (TF-IDF against knowledge graph)
      → Low novelty (familiar topic, incremental) → T0 suppress, just index metadata
      → Medium novelty (known topic, new angle) → T1 queue for theta processing
      → High novelty (new topic or contradicts existing) → T2 priority queue

Source Processing (theta tick, T1 or T2)
  → Fetch full content (web_fetch for papers, API for GitHub)
  → Extract entities (people, methods, datasets, results)
  → Extract relationships (X extends Y, A contradicts B, P uses Q)
  → Update knowledge graph with new nodes/edges
  → Compute prediction error against existing knowledge:
    - Confirms existing → low PE → quick metadata update
    - Extends existing → medium PE → update with new details
    - Contradicts existing → high PE → flag for synthesis

Synthesis (theta tick, T2 only -- triggered by high PE or accumulation)
  → Multiple sources on same topic → cross-reference
  → Generate hypothesis from combined evidence
  → Test hypothesis against known facts in the graph
  → If validated (confidence > 0.8) → promote to persistent knowledge
  → If refuted → record as anti-knowledge (things proven false)
  → If indeterminate → keep as active hypothesis with evidence list

Consolidation (delta tick, dream cycle)
  → Recompute graph clusters (community detection algorithm)
  → Find semantically-similar but structurally-disconnected clusters
  → Attempt cross-domain synthesis (T2 call with both clusters as context)
  → Prune stale knowledge (confidence decay below threshold → archive)
  → Generate research agenda (what questions remain unanswered?)
  → Evolve source-watching strategy (which feeds produced the most insights?)
```

### Dream synthesis (finding connections between disconnected clusters)

The dream cycle for research agents has a unique phase: cross-domain synthesis. The agent identifies clusters of knowledge that are semantically related (their embeddings are similar) but structurally disconnected (no edge path connects them in the graph). These are potential sites of novel insight -- the agent hypothesizes what the connection might be and validates it.

```rust
pub struct ResearchDreamPhase;

impl ResearchDreamPhase {
    pub async fn cross_domain_synthesis(
        graph: &KnowledgeGraph,
        synthesis_queue: &[SynthesisRequest],
        inference: &InferenceGateway,
    ) -> Result<Vec<SynthesisResult>> {
        let mut results = Vec::new();

        for request in synthesis_queue.iter().take(5) { // limit per dream
            // Build context from both clusters
            let context_a = graph.cluster_summary(&request.source_a);
            let context_b = graph.cluster_summary(&request.source_b);

            let prompt = format!(
                "Two knowledge domains appear related but have no known connection:\n\n\
                 Domain A: {}\n{}\n\n\
                 Domain B: {}\n{}\n\n\
                 Hypothesize: what connects these domains? \
                 Be specific. Cite which elements from each domain relate.",
                request.source_a, context_a,
                request.source_b, context_b,
            );

            let response = inference.infer(InferenceRequest {
                prompt,
                tier: CognitiveTier::T2,
                intent: Intent {
                    quality: Quality::High,
                    cost_sensitivity: 0.8, // dreams are cheap by design
                    ..Default::default()
                },
            }).await?;

            let synthesis = SynthesisResult {
                source_a: request.source_a.clone(),
                source_b: request.source_b.clone(),
                hypothesis: response.content,
                confidence: 0.3, // low initial confidence, needs validation
                status: SynthesisStatus::NeedsValidation,
            };

            results.push(synthesis);
        }

        Ok(results)
    }
}
```

---

## 4. Coding agent (existing orchestration mapped to the runtime)

### Profile

```rust
pub fn coding() -> DomainProfile {
    DomainProfile {
        name: "coding".into(),
        gamma_interval: Duration::from_secs(30),
        theta_interval: Duration::from_secs(120),
        delta_interval: Duration::from_secs(6000), // ~100 min
        base_gate_threshold: 0.4,
        extensions: vec![
            "heartbeat", "context", "neuro", "daimon", "conductor",
            "tools", "git", "gate", "safety", "learning", "dreams",
        ].into_iter().map(String::from).collect(),
        // ...
    }
}
```

### How existing plan-based orchestration maps to the runtime

The current orchestrator (`orchestrate.rs`) is a 19K-line monolith that manages agent dispatch, gating, learning, and persistence. In the new runtime, this dissolves:

| orchestrate.rs responsibility | New home | Mechanism |
|-------------------------------|----------|-----------|
| Plan discovery + DAG ordering | PlanRunner (thin coordinator) | Reads `tasks.toml`, builds DAG, injects tasks |
| Agent spawn + dispatch | `spawn_agent()` with coding profile | One agent per task (or persistent for session) |
| Gate execution (compile/test/clippy) | `GateExt` (L4) | Fires in `on_outcome` hook |
| Git worktree management | `GitExt` (L4) | Creates/cleans on `on_boot`/`on_shutdown` |
| Episode recording | `LearningExt` (L6) | Fires in `on_reflect` hook |
| Conductor watchers | `ConductorExt` (L3) | Fires in `on_tick_end`, detects stuck agents |
| Daimon affect | `DaimonExt` (L3) | Fires every tick, writes PAD to CorticalState |
| System prompt assembly | `ContextExt` (L0) | Fires in `assemble_context` hook |
| MCP config | Agent config | Passed through to tool discovery |
| Cost tracking | HeartbeatPipeline | Per-tick cost accumulated |
| Replan on failure | `ConductorExt` (L3) | Detects repeated failures, emits replan event |

### GitExt specific behaviors

```rust
pub struct GitExt {
    repo_path: PathBuf,
    worktree_path: Option<PathBuf>,
    branch: String,
    uses_worktrees: bool,
}

impl Extension for GitExt {
    fn name(&self) -> &str { "git" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Action }

    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> {
        if self.uses_worktrees {
            // Create isolated worktree for this agent's work
            let worktree = create_worktree(&self.repo_path, &self.branch)?;
            self.worktree_path = Some(worktree.path().to_owned());
            ctx.set_workspace_root(worktree.path());
        }
        Ok(())
    }

    async fn on_shutdown(&mut self, _ctx: &ShutdownContext) -> Result<ShutdownVote> {
        // Clean worktree on successful completion
        // (but NOT on failure -- user needs to inspect)
        Ok(ShutdownVote::Approve)
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        // Inject changed files, recent commits, branch context
        let diff = git_diff_staged(&self.workspace_path())?;
        let recent_commits = git_log(&self.workspace_path(), 5)?;

        ws.add_section(ContextSection {
            category: ContextCategory::CodeContext,
            priority: 4, // high priority -- agent needs to know what changed
            content: format!("## Changed files\n```diff\n{}\n```\n\n## Recent commits\n{}", diff, recent_commits),
            tokens: estimate_tokens(&diff) + estimate_tokens(&recent_commits),
            source: "git".into(),
            ..Default::default()
        });

        Ok(())
    }
}
```

### Why coding agents benefit from heartbeat (idle cost = $0)

A common objection: "coding agents do one task and die. Why give them a heartbeat?"

Three reasons:

1. **Stuck detection.** The conductor extension monitors progress every tick. If an agent has been running for 10 theta cycles without meaningful file changes or successful gate passes, it intervenes -- suggesting a different approach, escalating to a more capable model, or flagging for human review.

2. **Context freshness.** During a long coding session (30+ minutes), external state can change. Another agent might commit to the same branch. CI might report results. The heartbeat lets the coding agent incorporate new information mid-session without explicit polling.

3. **Cost accounting.** Every tick records cost. If an agent is spinning (retrying the same approach repeatedly), the heartbeat's reflection phase catches the pattern and adjusts. Idle ticks cost $0 because the gate suppresses them -- no downside.

---

## 5. Creating custom domains

### Path 1: Declarative (roko.toml)

For domains that can be expressed as combinations of existing extensions with different parameters:

```toml
# roko.toml

[domains.security-audit]
gamma_interval_secs = 120
theta_interval_secs = 600
delta_interval_secs = 36000
base_gate_threshold = 0.3
uses_git = true
uses_worktrees = false
max_concurrent_tools = 4

[domains.security-audit.extensions]
required = ["heartbeat", "context", "daimon", "neuro", "learning", "dreams"]
optional = ["conductor"]
custom = ["vuln-scanner"]

[domains.security-audit.gates]
default = ["static-analysis", "dependency-audit", "cve-check"]

[domains.security-audit.context_categories]
categories = ["codebase", "vulnerabilities", "cve-database", "remediation-playbook"]

[domains.security-audit.event_subscriptions]
filters = ["file-change", "dependency-update", "cve-published"]
```

Launch:

```bash
roko agent start --profile security-audit --workspace ./target-repo
```

The runtime reads the profile, instantiates extensions by name from the registry, and starts the heartbeat. No Rust code required for basic profiles.

### Path 2: Programmatic (Rust trait implementation)

For domains that need custom extensions with novel behavior:

```rust
/// Custom extension: vulnerability scanner for security audit agent
pub struct VulnScannerExt {
    scanner: VulnScanner,
    known_vulns: HashSet<String>,
    severity_threshold: Severity,
    last_full_scan: Instant,
    scan_interval: Duration,
}

impl Extension for VulnScannerExt {
    fn name(&self) -> &str { "vuln-scanner" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }
    fn depends_on(&self) -> &[&str] { &["heartbeat"] }

    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> {
        // Run initial full scan on boot
        let vulns = self.scanner.full_scan(ctx.workspace_root()).await?;
        for vuln in vulns {
            self.known_vulns.insert(vuln.id.clone());
        }
        self.last_full_scan = Instant::now();
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        // Periodic re-scan for new vulnerabilities
        if self.last_full_scan.elapsed() > self.scan_interval {
            let current_vulns = self.scanner.full_scan(ctx.workspace_root()).await?;

            for vuln in current_vulns {
                if self.known_vulns.insert(vuln.id.clone()) {
                    // New vulnerability found
                    let pe_contribution = match vuln.severity {
                        Severity::Critical => 0.9, // always escalate to T2
                        Severity::High => 0.6,
                        Severity::Medium => 0.3,
                        Severity::Low => 0.1,
                    };

                    ctx.add_observation(Observation::Custom {
                        kind: "vulnerability".into(),
                        data: serde_json::to_value(&vuln)?,
                        prediction_error_contribution: pe_contribution,
                    });
                }
            }

            self.last_full_scan = Instant::now();
        }
        Ok(())
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        // Inject known vulnerabilities into context for the LLM
        let critical = self.known_vulns_by_severity(Severity::Critical);
        if !critical.is_empty() {
            ws.add_section(ContextSection {
                category: ContextCategory::Vulnerabilities,
                priority: 5, // maximum -- critical vulns always in context
                content: render_vuln_report(&critical),
                tokens: estimate_tokens_for_vulns(&critical),
                source: "vuln-scanner".into(),
                ..Default::default()
            });
        }
        Ok(())
    }
}

// Register the custom extension and spawn the agent
fn main() -> Result<()> {
    let mut profile = DomainProfile::security();

    // Add custom extension to the registry
    extension_registry::register("vuln-scanner", |config| {
        Box::new(VulnScannerExt::new(config))
    });

    let agent = spawn_agent(profile, config, fabric).await?;
    agent.run_until_shutdown().await?;
    Ok(())
}
```

### Example: Writing/documentation agent

```toml
[domains.writing]
gamma_interval_secs = 60
theta_interval_secs = 180
delta_interval_secs = 7200
base_gate_threshold = 0.45
uses_git = true
uses_worktrees = false

[domains.writing.extensions]
required = ["heartbeat", "context", "daimon", "neuro", "tools", "git", "learning", "dreams"]
custom = ["style-checker"]

[domains.writing.gates]
default = ["spell-check", "link-check", "style-consistency", "readability-score"]

[domains.writing.context_categories]
categories = ["task", "style-guide", "existing-docs", "audience-profile"]
```

The writing agent's custom extension:

```rust
pub struct StyleCheckerExt {
    style_guide: StyleGuide,
    violations: Vec<StyleViolation>,
}

impl Extension for StyleCheckerExt {
    fn name(&self) -> &str { "style-checker" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Action }

    async fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()> {
        // After any write_file call, check style
        if call.name == "write_file" {
            let content = result.output_text();
            let violations = self.style_guide.check(content);
            self.violations.extend(violations);
        }
        Ok(())
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        if !self.violations.is_empty() {
            ws.add_section(ContextSection {
                category: ContextCategory::StyleGuide,
                priority: 4,
                content: format!(
                    "## Style violations to fix\n{}",
                    self.violations.iter()
                        .map(|v| format!("- {}: {}", v.location, v.message))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                tokens: self.violations.len() as u32 * 20, // ~20 tokens per violation
                source: "style-checker".into(),
                ..Default::default()
            });
        }
        Ok(())
    }
}
```

---

## 6. The native harness question

### Current state

Roko has two dispatch paths today:

**Path A: Claude CLI (external harness)**
- The orchestrator spawns `claude` as a subprocess
- Claude CLI drives its own internal tool loop
- Roko is a passive observer -- it reads final output but cannot intercept individual tool calls
- This is the primary path for plan execution via `orchestrate.rs`

**Path B: Native ToolLoop (API backends)**
- `ToolLoop` in `roko-agent/src/tool_loop/` drives the cycle directly
- Works for: Anthropic API, OpenAI/Codex, Gemini, Perplexity, Ollama
- Roko sends prompt, gets tool calls, dispatches via `ToolDispatcher`, sends results back, loops
- This path exists but is secondary -- the orchestrator defaults to Claude CLI

**The problem with both paths:**

Path A delegates everything. You cannot teach an opaque subprocess to hesitate before risky actions, learn from its tool call patterns, or gate its cognition based on prediction error. It is a black box.

Path B is a standard tool loop identical to every other framework. `prompt → LLM → parse_tool_calls → dispatch → results → loop`. It works, but it offers no architectural differentiation. LangChain does this. CrewAI does this. Cursor does this.

Neither path integrates the cognitive subsystems that make roko different: daimon (affect), neuro (knowledge), dreams (consolidation), gating (cost control), somatic markers (embodied hesitation).

### What makes roko's native harness different from "yet another tool loop"

The native harness wraps the standard tool loop in four stages that no other framework implements:

```
OBSERVE
  Extensions read their data sources (chain blocks, file changes, source feeds)
  Compute prediction error: how surprising is the current state vs. expectations?
  Result: a numeric PE value that determines what happens next

GATE
  Is this tick novel enough to justify an LLM call?
  T0 (PE < threshold): handle with Rust pattern matching. Cost: $0.
  T1 (PE < 2x threshold): cheap model, minimal context. Cost: $0.001-0.01.
  T2 (PE >= 2x threshold): full reasoning, complete workspace. Cost: $0.01-0.10.

  The threshold adapts over time. After 20 consecutive passes on a gate rung,
  the threshold rises (that rung is predictable). After a failure, it drops
  (something unexpected happened). This is EMA + CUSUM change detection.

ASSEMBLE (learnable)
  Build the CognitiveWorkspace: typed, budgeted, audited context package.
  VCG auction among context bidders (neuro, task, research, playbook).
  Affect-modulated allocation (high arousal → more risk warnings in context).
  Cache-aligned prefix (90% of system prompt is static → prefix cache hit).
  Section-effect tracking (which sections correlated with success last time?).

[STANDARD TOOL LOOP]
  prompt → LLM → parse tool calls → dispatch → results → loop
  BUT: somatic check on every tool call (extension hook: before_tool_call)
  AND: per-turn cost tracking with budget pressure from MortalityExt
  AND: tool results pass through after_tool_call for learning

REFLECT
  Record DecisionCycleRecord (complete tick history, not just "pass/fail")
  Attribute: which context sections were referenced by the model?
  Feedback: update section allocations for next time
  Episode: full record stored for dream consolidation
  Somatic update: if outcome was bad, mark similar actions with hesitation
```

### Comparison: Claude CLI vs. native roko harness

| Capability | Claude CLI (external) | Native roko harness |
|------------|----------------------|---------------------|
| Tool interception | No (black box subprocess) | Yes (every call passes through `before_tool_call` / `after_tool_call`) |
| Somatic checks | No | Yes -- `DaimonExt` computes affect similarity to past failures, gates risky calls |
| Cost gating | No (Claude CLI manages its own budget) | Yes -- `HeartbeatPipeline` gates entire ticks, `MortalityExt` tracks cumulative spend |
| Learning from tools | No (only final output visible) | Yes -- per-tool-call outcomes feed `LearningExt` episode records |
| Caching | Claude's internal cache (opaque) | Three layers: L3 deterministic (SHA-256), L2 semantic (embedding sim), L1 prefix (provider KV) |
| Model routing | Fixed at dispatch time | Intent-based per-tick (`CascadeRouter` with LinUCB bandit, adapts from outcomes) |
| Context assembly | Static system prompt (user builds it) | Learnable workspace (sections compete via VCG auction, allocations evolve from feedback) |
| Cognitive tiers | Always T2 (full call every time) | T0/T1/T2 gated by prediction error (80% of ticks are $0) |
| Cross-session learning | None (starts fresh) | Playbooks, neuro store, section-effect weights, gate thresholds -- all persist |
| Embodied hesitation | None | Somatic markers create graded risk response (allow → extra verification → escalate → block) |
| Dream consolidation | None | Offline replay, counterfactual generation, pattern extraction between sessions |
| Multi-timescale | Single timescale (one call, one response) | Three timescales (gamma/theta/delta) with different behaviors at each |

### When to use which path

- **Claude CLI path**: Use for one-shot tasks where you want maximum tool coverage and Claude Code's built-in behaviors (subagents, worktrees, git management). The external harness is battle-tested and handles edge cases well. Cost control is less critical because the task is bounded.

- **Native harness path**: Use for long-lived agents, cost-sensitive workloads, domains where learning matters (repeated similar tasks), and situations requiring tool interception (somatic checks, safety enforcement, audit logging).

The two paths coexist. Claude CLI becomes one backend option in the native harness -- you can still delegate to it when appropriate, but you lose tool-level visibility.

---

## 7. Blue ocean analysis

### The red ocean (what every agent harness does)

Every agent framework in 2025-2026 converges on one architecture:

```
System prompt + context → LLM API → Parse tool calls → Execute tools → Loop until done
```

| Feature | Claude Code | Codex CLI | Cursor | Aider | Cline | LangGraph | CrewAI |
|---------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| LLM API wrapper | X | X | X | X | X | X | X |
| File read/write | X | X | X | X | X | X | X |
| Shell execution | X | X | X | X | X | X | X |
| Git integration | X | X | - | X | X | - | - |
| MCP protocol | X | X | X | - | X | - | - |
| Context management | X | X | X | X | X | X | X |
| Permission gates | X | X | X | - | X | - | - |
| Multi-file editing | X | X | X | X | X | X | X |
| Streaming output | X | X | X | X | X | X | X |

Differentiation is purely UX (terminal vs. IDE vs. web) and model defaults. The underlying architecture is identical. Every one of these tools is a thin wrapper around `prompt → call → tools → loop`.

### Fundamental limitations nobody has solved

These are not feature gaps. They are architectural limitations that cannot be fixed by adding another tool or adjusting a prompt.

**1. Goldfish memory**

Every session starts at zero. Agent session #1000 assembles the same context, makes the same mistakes, and learns nothing from the 999 sessions that preceded it. "Memory" in current tools means a static markdown file or a vector store that retrieves raw text -- there is no mechanism for the system to improve its own prompts based on what worked.

Why nobody has solved this: Learning requires feedback attribution ("which context section helped?") and a feedback loop that modifies future assembly. This is a control system, not a feature. You cannot bolt it onto a stateless loop.

**2. Compounding errors**

A mistake at turn 3 bakes into the codebase by turn 30. The agent has no mechanism for mid-session self-correction because it has no concept of prediction error -- it does not track what it expected vs. what happened. It confidently proceeds regardless of accumulated deviation from the plan.

Why nobody has solved this: Self-correction requires a comparison signal (expected vs. observed), which requires a model of expectation, which requires state that persists across turns within a session. Stateless loops have no such state.

**3. Cost runaway**

Agents cost $3-12 per task with no proactive cost gating. Every turn is a full-context LLM call regardless of whether the situation is novel or routine. An agent fixing a typo uses the same computational resources as an agent designing a new architecture.

Why nobody has solved this: Cost gating requires classifying the difficulty of a situation before calling the LLM to handle it. This is a catch-22 in stateless architectures -- you need the LLM to classify, which costs the same as just doing the work. Roko solves this with T0 prediction error (computed in Rust, $0) that decides whether the LLM is needed at all.

**4. No learning curve**

Agent #1000 performs identically to agent #1 on the same task category. There is no mechanism for accumulated experience to improve performance. The model improves (when providers ship updates), but the harness does not.

Why nobody has solved this: Learning requires episode recording (structured, not just logs), feedback attribution (what worked), knowledge extraction (patterns from episodes), and knowledge injection (modified prompts for future sessions). This is a complete system, not a single feature.

**5. Brittle context**

Context assembly is static. Someone writes a system prompt, maybe adds some retrieved documents, and hopes for the best. There is no feedback mechanism telling the system "this section was useless last time" or "this type of context correlates with success for this task type."

Why nobody has solved this: Adaptive context requires per-section outcome tracking (was this referenced?), allocation adjustment (increase budget for useful sections), and policy evolution (change the assembly strategy). This is a multi-loop control system that must be designed in, not added.

**6. No embodied hesitation**

Agents confidently perform destructive actions (deleting files, force pushing, dropping tables) with no "gut feeling" that something is wrong. Binary permission systems (allow/deny) do not capture the gradient of risk. A tool call that matches the pattern of three past incidents where the agent corrupted the repository should trigger hesitation -- not a hard block, but increased verification.

Why nobody has solved this: Hesitation requires affect state (emotional valence from past outcomes), similarity matching (does this action resemble past failures?), and graded response (not binary allow/deny, but a spectrum from "proceed" to "pause and verify" to "refuse"). This requires an affect engine -- a subsystem that no stateless framework has.

### The 5 killer features that create blue ocean

#### Feature 1: Cognitive gating (35x cost reduction)

**Mechanism**: The heartbeat pipeline gates expensive LLM calls behind prediction error. Most ticks are T0 (deterministic Rust, no LLM, $0 cost). Only novel situations escalate to T1 (cheap model) or T2 (full reasoning).

**Economics**:
- Without gating: continuous operation at Opus pricing = ~$576/day
- With gating: 80% T0 (free) + 15% T1 ($0.003 each) + 5% T2 ($0.05 each) = ~$6-58/day
- Cost reduction: 10-96x depending on domain volatility

**What is already built in roko**:
- `roko-gate`: 7-rung adaptive pipeline with 11 gate implementations
- `adaptive_threshold.rs`: EMA + CUSUM change detection for dynamic thresholds
- `cascade_router.rs`: static tier → confidence-based → LinUCB bandit model selection

**What remains**: Integrating the heartbeat pipeline so gating happens per-tick rather than per-task.

#### Feature 2: Learnable context assembly (prompt N+1 > prompt N)

**Mechanism**: Context assembly is a control system with feedback. The system tracks which sections correlated with successful outcomes and adjusts allocations over time. Sections compete for token budget via a VCG auction mechanism.

**Components**:
- VCG auction among context bidders (Neuro, Task, Research, Playbook contexts bid for inclusion)
- Section-effect tracking (did the model reference this section? Was the outcome good?)
- Beta-distribution feedback per category (Bayesian update on usefulness)
- Policy evolution (every 50 ticks, reallocate based on accumulated evidence)
- HDC fingerprinting for O(1) task similarity lookup ("tasks like this one succeeded with X context")

**What is already built in roko**:
- `cfactor.rs`: C-factor metrics for prompt quality assessment
- `section_effect.rs`: section-outcome correlation tracking
- `vcg_allocate`: auction mechanism for competitive context bidding
- `playbook.rs`: validated patterns injected at dispatch time
- HDC fingerprinting per episode in the learning system

#### Feature 3: Dream consolidation (offline pattern discovery at $0)

**Mechanism**: Between sessions, the system replays episodes, discovers patterns, generates counterfactuals, rehearses threats, and consolidates validated insights into playbooks. This happens during delta ticks (dream state) at near-zero cost (batch embeddings only).

**Dream phases**:
1. **Replay** -- revisit high-surprise and high-failure episodes with affect weighting
2. **Counterfactual** -- "what if we had used a different tool / approach / model?"
3. **Threat rehearsal** -- "what dangerous patterns might we encounter next?"
4. **Staging** -- candidate insights sit in a buffer requiring validation from multiple episodes
5. **Promotion** -- validated insights graduate to persistent knowledge (neuro store) or playbooks

**What is already built in roko**:
- `roko-dreams`: complete dream cycle implementation (replay, imagination, rehearsal, staging, promotion)
- Sleep pressure accumulation in the learning system
- Episode recording with HDC fingerprints for similarity matching

#### Feature 4: Somatic markers (continuous risk gradient)

**Mechanism**: An affect system assigns emotional valence to actions based on historical outcomes. When an agent is about to take an action that matches past failure patterns, it generates a hesitation signal. The strength of hesitation determines the response:

| Affect level | Response |
|-------------|----------|
| Low (< 0.2) | Proceed at full speed |
| Medium (0.2 - 0.5) | Add extra verification rungs (additional gates) |
| High (0.5 - 0.8) | Escalate to more capable model for review |
| Critical (> 0.8) | Block and request human approval |

This is fundamentally different from binary allow/deny permission systems. It creates a gradient of caution based on accumulated experience.

**What is already built in roko**:
- `roko-daimon`: ALMA temporal affect model (Pleasure-Arousal-Dominance)
- Somatic marker computation from episode history
- `before_tool_call` hook in the extension trait (injection point)

#### Feature 5: Native Rust + type-state lifecycle (zero overhead + compile-time safety)

**Mechanism**: The entire agent loop -- inference dispatch, tool execution, gating, learning -- runs as compiled Rust. No subprocess spawning, no JSON-RPC serialization overhead, no Python interpreter, no cold starts.

Type-state lifecycle makes invalid states compile errors:

```rust
// This compiles:
let agent = Agent::<Provisioning>::new(config);
let agent = agent.activate(chain);  // Provisioning → Active
let outcome = agent.tick().await;   // Active has tick()

// This does NOT compile:
let agent = Agent::<Terminal>::new(config);
agent.tick().await;  // ERROR: Agent<Terminal> does not implement tick()
```

**Performance comparison**:

| Metric | Python (LangGraph) | Node.js (Cline) | Rust (roko) |
|--------|-------------------|-----------------|-------------|
| Cold start | 400-800ms | 100-300ms | 2-5ms |
| Tool call overhead | 5-15ms (serialize + deserialize) | 3-8ms | 0.01-0.05ms (direct fn call) |
| Memory per agent | 200-500MB | 100-200MB | 10-30MB |
| Concurrent agents per 4GB | 8-20 | 20-40 | 100-400 |

Roko's 19 built-in tools execute as direct function calls within the same process. No serialization. No subprocess spawn. No IPC.

### Why building from scratch is necessary

"Why not add learning to LangChain? Why not add gating to Claude Code?"

Four reasons this cannot be retrofitted:

**1. You cannot retrofit learning into a stateless loop.** Learning requires a universal data type (Engram) that flows through a substrate (durable store) in a feedback loop (`score → route → compose → act → verify → write → react`). Every component must speak the same type. You cannot add this to a system designed around string-in/string-out tool calls -- the type foundation must exist from day zero.

**2. You cannot retrofit gating into subprocess-based tools.** If tools execute as subprocesses (shell out to `git`, spawn a file editor, call MCP over stdio), the harness cannot gate them in the same stack frame. Gates must inspect tool call intent before execution, in microseconds. Subprocess IPC adds 50-100ms minimum. Over thousands of calls, this is hours of overhead. Native function-call tools enable sub-millisecond gating.

**3. You cannot retrofit type-state safety after the fact.** Lifecycle guarantees must be in the type system from the start. Adding them later means wrapping everything in `Result<T, InvalidStateError>` -- runtime checks instead of compile-time prevention. Roko's `Agent<Phase>` pattern makes it impossible to call `.tick()` on a dead agent. The compiler rejects the code.

**4. You cannot retrofit evolutionary dynamics into append-only logs.** Signal metabolism in roko treats data as a living population. Signals have fitness, reproduce (get promoted), and die (get archived). This is a fundamentally different data model from "append to vector store, retrieve by similarity." You cannot add replicator dynamics to a system designed for append-only storage.

---

## 8. Deployment and UX

### Local (one command)

```bash
# Start a blockchain agent with default configuration
roko agent start --profile blockchain

# Start with custom config
roko agent start --profile blockchain --config chain.toml

# Start a research agent watching specific topics
roko agent start --profile research --topic "LLM agents" --topic "Rust async"

# Start a coding agent as a persistent session (not one-shot task execution)
roko agent start --profile coding --workspace ./my-project

# Start a custom-domain agent
roko agent start --profile security-audit --workspace ./target-repo

# List running agents
roko agent list

# Stop an agent gracefully (allows genome extraction)
roko agent stop blockchain-1

# Force stop (no genome, immediate termination)
roko agent kill blockchain-1
```

Output on start:

```
[blockchain-1] Starting with profile: blockchain
[blockchain-1] Extensions loaded: 14
[blockchain-1] Subscribing to: Ethereum (mainnet), Base
[blockchain-1] Gamma: 5s | Theta: 30s | Delta: 25min
[blockchain-1] Budget: $1000.00 USDC
[blockchain-1] Phase: Thriving (vitality: 1.00)
[blockchain-1] Listening on ws://localhost:6678 for operator chat
[blockchain-1] Ready.
```

### Remote (container deployment)

```dockerfile
FROM rust:1.91-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p roko-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/roko /usr/local/bin/
COPY config/ /etc/roko/

# Environment variables for secrets (never bake into image)
ENV ANTHROPIC_API_KEY=""
ENV RPC_URL=""

# Agent starts and exposes WebSocket on :8080 for operator chat
# Health check endpoint on :8081
EXPOSE 8080 8081

ENTRYPOINT ["roko", "agent", "start", \
    "--profile", "blockchain", \
    "--config", "/etc/roko/blockchain.toml", \
    "--serve", ":8080", \
    "--health", ":8081"]
```

Docker compose for multi-agent deployment:

```yaml
version: "3.8"

services:
  blockchain-eth:
    image: roko-agent:latest
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RPC_URL=wss://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_KEY}
    ports:
      - "8080:8080"
    volumes:
      - eth-data:/var/lib/roko  # persistent state (episodes, knowledge, genome)
    deploy:
      resources:
        limits:
          memory: 256M
          cpus: "0.5"

  blockchain-base:
    image: roko-agent:latest
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RPC_URL=wss://base-mainnet.g.alchemy.com/v2/${ALCHEMY_KEY}
    ports:
      - "8081:8080"
    volumes:
      - base-data:/var/lib/roko

  research-agent:
    image: roko-agent:latest
    command: ["roko", "agent", "start", "--profile", "research", "--serve", ":8080"]
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
    ports:
      - "8082:8080"
    volumes:
      - research-data:/var/lib/roko

volumes:
  eth-data:
  base-data:
  research-data:
```

### Persistent chat interface

Agents are long-lived processes. Operators communicate with them through a persistent chat interface:

```bash
$ roko chat --agent blockchain-1

[blockchain-1] Phase: Stable | Vitality: 0.72 | Tick: 4,523 | Cost: $14.20
[blockchain-1] Watching: ETH mainnet, Base
[blockchain-1] Positions: 2 active (ETH/USDC LP on Uniswap, AAVE wstETH vault)
[blockchain-1] Last action: 12min ago (rebalanced LP range)

you> How are positions performing?

[blockchain-1] Checking current state...
  ETH/USDC LP (Uniswap v3, 1800-2200 range):
    Fees earned (7d): $47.20
    Impermanent loss (7d): -$18.40
    Net P&L: +$28.80

  AAVE wstETH vault:
    Current APY: 3.2%
    Health factor: 2.1 (safe, liquidation at 1.0)
    Accrued: $12.40

  Total 7d P&L: +$41.20
  Budget remaining: $985.80

you> I'm worried about a potential ETH dump. Reduce risk.

[blockchain-1] Understood. Adjusting risk posture:
  1. Narrowing LP range to 1900-2100 (less IL exposure)
  2. Raising AAVE health factor target from 1.5 to 2.5 (reducing leverage)
  3. Setting stop-loss at -5% portfolio value (was -10%)
  4. Entering Conservation phase override for 48h

  Estimated impact: -40% yield potential, +60% downside protection.
  Execute? [y/n]

you> y

[blockchain-1] Executing adjustments...
  [tx 0xabc...] Removed liquidity from old range
  [tx 0xdef...] Added liquidity in 1900-2100 range
  [tx 0x123...] Repaid partial AAVE debt (HF now 2.4)
  Strategy parameters updated. Conservation mode active for 48h.
```

### Multi-agent management

The TUI dashboard shows all running agents:

```
roko dashboard
```

```
┌─ Agents ─────────────────────────────────────────────────────────────────────┐
│ Name           │ Domain     │ Phase    │ Vitality │ Tick   │ Cost/24h │ Last │
│────────────────│────────────│──────────│──────────│────────│──────────│──────│
│ blockchain-1   │ blockchain │ Stable   │ 0.72     │ 4,523  │ $14.20   │ 12m  │
│ blockchain-2   │ blockchain │ Thriving │ 0.91     │ 2,108  │ $8.40    │ 3m   │
│ research-ai    │ research   │ Stable   │ 0.65     │ 891    │ $4.10    │ 22m  │
│ coding-roko    │ coding     │ Active   │ -        │ 45     │ $1.20    │ now  │
│ security-scan  │ security   │ Stable   │ 0.80     │ 312    │ $2.30    │ 45m  │
└──────────────────────────────────────────────────────────────────────────────┘

┌─ Events (last 5min) ──────────────────────────────────────────────────────────┐
│ 14:23:01 [blockchain-1] T2 escalation: unusual swap volume on WETH/USDC pool │
│ 14:22:45 [blockchain-2] T0 suppressed: 12 routine transfers                   │
│ 14:22:30 [research-ai]  New source: arxiv/2404.12345 "Attention Sinks in..."  │
│ 14:22:12 [coding-roko]  Gate passed: all tests (42 pass, 0 fail)              │
│ 14:21:58 [blockchain-1] Position update: ETH/USDC LP rebalanced              │
└───────────────────────────────────────────────────────────────────────────────┘

[F1] Agents  [F2] Events  [F3] Episodes  [F4] Knowledge  [F5] Costs  [F6] Chat
```

### One-click onboarding vision

The target UX for new users:

```bash
# Install
cargo install roko-cli

# Initialize a project (creates .roko/ directory and roko.toml)
cd my-project
roko init

# Answer 3 questions:
#   1. What domain? (coding / blockchain / research / custom)
#   2. What model provider? (anthropic / openai / ollama / custom)
#   3. Any MCP servers to connect? (y/n)

# Start working
roko run "Refactor the auth module to use async middleware"
```

For blockchain agents:

```bash
# Initialize with blockchain profile
roko init --profile blockchain

# Configure (interactive):
#   1. Which chains? (ethereum, base, arbitrum, ...)
#   2. RPC endpoint?
#   3. Strategy file? (create example STRATEGY.md)
#   4. Budget limit? ($100, $1000, unlimited)

# Start the agent
roko agent start --profile blockchain
```

The goal: from `cargo install` to a running, learning agent in under 5 minutes. No PhD in prompt engineering. No YAML configuration hell. Sensible defaults that work out of the box, with full customization available through `roko.toml` and custom extensions.

### TUI integration

The existing ratatui TUI (`roko dashboard`) already supports 7 tabs (F1-F7). For multi-agent management, it expands:

- **F1 Agents**: running agents, phase, vitality, cost, last action
- **F2 Events**: real-time event stream from all agents (filterable by agent/type)
- **F3 Episodes**: episode log with HDC fingerprint similarity grouping
- **F4 Knowledge**: neuro store contents, confidence levels, tier progression
- **F5 Costs**: per-agent cost breakdown (T0/T1/T2 distribution, tool costs, inference costs)
- **F6 Chat**: operator chat with selected agent (inline, not a separate terminal)
- **F7 Dreams**: dream cycle status, sleep pressure, recent consolidation results

The TUI connects to running agents via the existing HTTP control plane (`roko-serve` on :6677) and per-agent sidecar WebSockets. No additional infrastructure needed -- the pieces already exist.
