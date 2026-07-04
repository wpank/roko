# Domain Specialization: Blockchain + Research Agents

## Blockchain Agent (Subscriber-First Design)

### What It Does

A blockchain agent is a **long-lived process** that:
1. Subscribes to chain events (blocks, mempool, price feeds)
2. Triages every event through fast filters (T0, no LLM)
3. Escalates novel/high-value situations to LLM reasoning (T1/T2)
4. Executes strategies (swaps, LP rebalances, vault management)
5. Learns from outcomes (episodes → playbook → evolved strategies)
6. Communicates with peers via pheromone field

### Tick Schedule

| Frequency | Interval | What Happens | Cost |
|---|---|---|---|
| Gamma | 5s | Check latest block, triage txs, update CorticalState | $0 (T0) |
| Theta | 30s | Full decision cycle: observe→gate→decide→execute | $0-0.05 |
| Delta | 25min | Dream: consolidate episodes, evolve strategies | $0.01 batch |

### Extensions

```rust
// Blockchain agent extension chain
ExtensionChain::builder()
    // L0: Foundation
    .add(HeartbeatExt::new(Duration::from_secs(5)))
    .add(ContextExt::new(ContextPolicy::blockchain()))
    // L1: Perception
    .add(ChainSubscriberExt::new(ChainConfig {
        rpc_url: "wss://eth.example.com",
        chains: vec![Chain::Ethereum, Chain::Base],
        interest_filter: BinaryFuse8::from_addresses(&watched_addresses),
    }))
    .add(PriceFeedExt::new(vec!["ETH/USD", "BTC/USD"]))
    // L2: Memory
    .add(NeuroExt::new(knowledge_store.clone()))
    .add(StrategyStoreExt::new(strategy_path))
    // L3: Cognition
    .add(DaimonExt::new(DaimonConfig::blockchain()))
    .add(RiskExt::new(RiskConfig {
        max_position_pct: 0.05,
        max_gas_gwei: 100,
        kelly_fraction: 0.25,
    }))
    .add(MortalityExt::new(MortalityConfig {
        budget_usdc: 1000.0,
        epistemic_decay_rate: 0.001,
        stochastic_halflife_ticks: 10_000,
    }))
    // L4: Action
    .add(ToolsExt::new(vec![
        "balance_of", "send_tx", "simulate_tx", "approve",
        "swap", "add_liquidity", "remove_liquidity",
    ]))
    .add(SafetyExt::new(SafetyConfig::blockchain()))
    // L5: Social
    .add(PheromoneExt::new(pheromone_config))
    // L6: Meta
    .add(LearningExt::new(learning_config))
    .add(DreamsExt::new(DreamsConfig {
        min_episodes_for_dream: 50,
        sleep_pressure_threshold: 0.8,
    }))
    .build()?
```

### Chain Subscriber Extension

```rust
pub struct ChainSubscriberExt {
    config: ChainConfig,
    client: Option<Box<dyn ChainClient>>,
    triage_pipeline: TriagePipeline,
    latest_block: u64,
    pending_events: VecDeque<ChainEvent>,
}

impl Extension for ChainSubscriberExt {
    fn name(&self) -> &str { "chain-subscriber" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> {
        // Connect to chain RPC
        self.client = Some(create_chain_client(&self.config).await?);
        // Subscribe to newHeads + pendingTransactions
        self.subscribe_events(ctx.event_fabric()).await?;
        Ok(())
    }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        // Drain pending chain events, triage each one
        while let Some(event) = self.pending_events.pop_front() {
            let classification = self.triage_pipeline.classify(&event);
            ctx.add_observation(Observation::Chain {
                event,
                classification,
            });
        }
        Ok(())
    }

    async fn on_event(&mut self, event: &RuntimeEvent, _ctx: &mut EventContext) -> Result<()> {
        // Buffer incoming chain events for next observe cycle
        if let EventPayload::NewBlock { number, .. } = &event.payload {
            self.latest_block = *number;
        }
        self.pending_events.push_back(ChainEvent::from(event));
        Ok(())
    }
}
```

### Triage Pipeline (T0, Pure Rust, No LLM)

```rust
pub struct TriagePipeline {
    /// Stage 1: Rule-based fast filters (bloom, threshold, pattern)
    rules: Vec<Box<dyn TriageRule>>,
    /// Stage 2: Statistical anomaly detection (MIDAS-R, DDSketch)
    anomaly: AnomalyDetector,
    /// Stage 3: Contextual enrichment (ABI resolution)
    enricher: TxEnricher,
    /// Stage 4: Scoring (Thompson sampling with discounted hedge)
    scorer: DiscountedHedge,
}

impl TriagePipeline {
    pub fn classify(&self, event: &ChainEvent) -> Classification {
        // Stage 1: Fast filter (microseconds)
        for rule in &self.rules {
            if let Some(verdict) = rule.check(event) {
                return verdict;
            }
        }

        // Stage 2: Anomaly detection (sub-millisecond)
        if self.anomaly.is_anomalous(event) {
            return Classification::Escalate(CognitiveTier::T1);
        }

        // Stage 3: Enrich (may need ABI lookup, cached)
        let enriched = self.enricher.enrich(event);

        // Stage 4: Score via Thompson sampling
        let score = self.scorer.score(&enriched);
        if score > 0.7 { Classification::Escalate(CognitiveTier::T2) }
        else if score > 0.3 { Classification::Escalate(CognitiveTier::T1) }
        else { Classification::Suppress }
    }
}
```

### Strategy Lifecycle

```
STRATEGY.md (operator-authored, human-readable)
  → StrategyParams (parsed by CompilerExt at boot)
    → CorticalState signals (updated every gamma tick)
      → Prediction Error (how much does reality differ from strategy expectation?)
        → Gating (T0: ignore | T1: adjust | T2: full deliberation)
          → Episode recording (what happened, what was the outcome)
            → PLAYBOOK.md evolution (every 50 episodes, dream cycle)
              → Strategy adaptation (learned adjustments to StrategyParams)
```

### Mortality (Economic Viability)

```rust
pub struct MortalityExt {
    /// USDC budget remaining
    economic_clock: f64,
    /// Knowledge confidence (decays without validation)
    epistemic_clock: f64,
    /// Random survival (prevents immortal agents)
    stochastic_clock: f64,
    /// Current behavioral phase
    phase: BehavioralPhase,
}

pub enum BehavioralPhase {
    Thriving,     // vitality > 0.8: aggressive strategies
    Stable,       // vitality 0.5-0.8: balanced
    Conservation, // vitality 0.3-0.5: reduce risk, preserve capital
    Declining,    // vitality 0.1-0.3: minimal activity, genome prep
    Terminal,     // vitality < 0.1: shutdown initiated
}
```

---

## Research Agent

### What It Does

A research agent is a **deliberate, knowledge-accumulating process** that:
1. Monitors sources for new information (papers, data feeds, code repos)
2. Synthesizes across sources to build understanding
3. Tests hypotheses against evidence
4. Accumulates a persistent knowledge graph
5. Produces artifacts (summaries, analyses, recommendations)
6. Dreams to consolidate and find novel connections

### Tick Schedule

| Frequency | Interval | What Happens | Cost |
|---|---|---|---|
| Gamma | 60s | Check source feeds, detect new publications | $0 (T0) |
| Theta | 5min | Full research cycle: read → analyze → synthesize | $0.01-0.10 |
| Delta | 4hr | Dream: consolidate, hypothesis generation, prune | $0.05 batch |

### Extensions

```rust
ExtensionChain::builder()
    // L0: Foundation
    .add(HeartbeatExt::new(Duration::from_secs(60)))
    .add(ContextExt::new(ContextPolicy::research()))
    // L1: Perception
    .add(SourceWatcherExt::new(SourceConfig {
        feeds: vec![
            SourceFeed::ArxivCategory("cs.AI"),
            SourceFeed::GithubRepo("anthropics/claude-code"),
            SourceFeed::RssUrl("https://example.com/feed.xml"),
        ],
        poll_interval: Duration::from_secs(300),
    }))
    // L2: Memory
    .add(NeuroExt::new(knowledge_store.clone()))
    .add(KnowledgeGraphExt::new(graph_config))
    .add(CitationStoreExt::new(citation_path))
    // L3: Cognition
    .add(DaimonExt::new(DaimonConfig::research()))
    .add(HypothesisExt::new())  // tracks active hypotheses
    // L4: Action
    .add(ToolsExt::new(vec![
        "web_search", "web_fetch", "read_file", "write_file",
        "grep", "glob", "summarize", "extract_citations",
    ]))
    // L6: Meta
    .add(LearningExt::new(learning_config))
    .add(DreamsExt::new(DreamsConfig {
        min_episodes_for_dream: 20,
        sleep_pressure_threshold: 0.6, // dreams more often (research benefits from consolidation)
    }))
    .add(SynthesisExt::new()) // cross-source reasoning
    .build()?
```

### Knowledge Graph Extension

```rust
pub struct KnowledgeGraphExt {
    graph: KnowledgeGraph,
    pending_entries: Vec<KnowledgeEntry>,
    synthesis_queue: Vec<SynthesisRequest>,
}

impl Extension for KnowledgeGraphExt {
    fn name(&self) -> &str { "knowledge-graph" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Memory }

    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> {
        // After each research action, extract entities and relationships
        let new_knowledge = ctx.extract_knowledge();
        for entry in new_knowledge {
            self.graph.add(entry.clone());
            self.pending_entries.push(entry);
        }
        Ok(())
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        // Inject relevant graph neighborhood into context
        let topic = ws.current_topic();
        let neighborhood = self.graph.query_neighborhood(topic, depth: 2, max_nodes: 20);
        ws.add_section(ContextSection {
            category: ContextCategory::Knowledge,
            priority: 3,
            content: neighborhood.render_markdown(),
            tokens: neighborhood.estimated_tokens(),
            ..Default::default()
        });
        Ok(())
    }

    async fn on_dream_start(&mut self, ctx: &mut DreamContext) -> Result<()> {
        // During dreams, find disconnected clusters and queue synthesis
        let clusters = self.graph.find_disconnected_clusters();
        for (cluster_a, cluster_b) in clusters.pairs() {
            self.synthesis_queue.push(SynthesisRequest {
                source_a: cluster_a.topic(),
                source_b: cluster_b.topic(),
                hypothesis: format!(
                    "What connects {} and {}?",
                    cluster_a.topic(), cluster_b.topic()
                ),
            });
        }
        Ok(())
    }
}
```

### Research Cycle

```
Source Detection (gamma, T0)
  → Is this new? (bloom filter check)
    → No → suppress
    → Yes → queue for theta processing

Source Processing (theta, T1/T2)
  → Read source (web_fetch / read_file)
  → Extract entities + relationships → knowledge graph
  → Compare against existing knowledge → prediction error
  → If contradicts existing → high PE → T2 deliberation
  → If confirms existing → low PE → T0/T1 quick update

Synthesis (theta, T2 only)
  → Multiple sources on same topic → cross-reference
  → Generate hypothesis from combined evidence
  → Test hypothesis against known facts
  → If validated → promote to persistent knowledge
  → If refuted → record as anti-knowledge

Consolidation (delta, dream cycle)
  → Find disconnected knowledge clusters
  → Attempt cross-domain synthesis
  → Prune stale knowledge (confidence decay)
  → Generate research agenda (what to investigate next)
  → Evolve search strategies based on what produced insights
```

---

## How Users Create New Domains

### Declarative (roko.toml)

```toml
[domains.security-audit]
gamma_interval_secs = 120
theta_interval_secs = 600
delta_interval_secs = 36000
base_gate_threshold = 0.3
uses_git = true
uses_worktrees = false

[domains.security-audit.extensions]
required = ["heartbeat", "context", "daimon", "learning"]
optional = ["conductor", "dreams"]
custom = ["vuln-scanner"]

[domains.security-audit.gates]
default = ["static-analysis", "dependency-audit", "cve-check"]

[domains.security-audit.context_categories]
categories = ["codebase", "vulnerabilities", "cve-database", "remediation-playbook"]

[domains.security-audit.event_subscriptions]
filters = ["file-change", "dependency-update", "cve-published"]
```

### Programmatic (Rust)

```rust
// Custom extension for a security agent
pub struct VulnScannerExt {
    scanner: VulnScanner,
    known_vulns: HashSet<String>,
}

impl Extension for VulnScannerExt {
    fn name(&self) -> &str { "vuln-scanner" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        let new_vulns = self.scanner.scan(ctx.workspace_root()).await?;
        for vuln in new_vulns {
            if self.known_vulns.insert(vuln.id.clone()) {
                ctx.add_observation(Observation::Custom {
                    kind: "vulnerability".into(),
                    data: serde_json::to_value(&vuln)?,
                    prediction_error_contribution: 0.5, // high PE → escalate to T2
                });
            }
        }
        Ok(())
    }
}

// Register and spawn
let profile = DomainProfile::from_config("security-audit", &config)?;
let mut agent = spawn_agent(profile, config, fabric).await?;
agent.run_until_shutdown().await?;
```

## Deployment

### Local (One Command)

```bash
# Start a blockchain agent
roko agent start --profile blockchain --config chain.toml

# Start a research agent
roko agent start --profile research --topic "LLM context engineering"

# Start a coding agent (persistent, not one-shot)
roko agent start --profile coding --workspace ./my-project

# Chat with a running agent
roko chat --agent blockchain-1
```

### Remote (Container)

```dockerfile
FROM rust:1.91 AS builder
COPY . .
RUN cargo build --release -p roko-cli

FROM debian:bookworm-slim
COPY --from=builder /target/release/roko /usr/local/bin/
COPY config/ /etc/roko/

# Agent starts and exposes WebSocket for operator chat
ENTRYPOINT ["roko", "agent", "start", "--profile", "blockchain", "--serve", ":8080"]
```

### Interaction (Persistent Chat)

```
$ roko chat --agent blockchain-1

[blockchain-1] Phase: Stable | Vitality: 0.72 | Tick: 4,523
[blockchain-1] Watching: ETH mainnet, Base
[blockchain-1] Positions: 2 active (ETH/USDC LP, AAVE vault)

you> How are positions performing?

[blockchain-1] Analyzing positions...
[blockchain-1] ETH/USDC LP: +2.3% (7d), impermanent loss -0.4%, net +1.9%
[blockchain-1] AAVE vault: +0.8% APY, health factor 2.1 (safe)
[blockchain-1] Combined P&L: +$142.50 (7d)

you> Lower risk — I want to be more conservative this week

[blockchain-1] Acknowledged. Adjusting strategy:
  - Reducing position sizes by 30%
  - Raising health factor target from 1.5 to 2.5
  - Disabling MEV opportunities below 0.1 ETH
  - Phase override: Conservation mode for 7d
```
