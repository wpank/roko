# PRD-06: Domains, arenas, and work markets

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Crates affected:** `roko-core` (extend `DomainProfile`), `roko-runtime` (domain-aware `ClockConfig`), `roko-ext-chain` (new), `roko-ext-research` (new), `roko-arena` (new), `roko-chain` (work markets)

---

## Table of contents

1. [One runtime, many domains](#1-one-runtime-many-domains)
2. [The DomainProfile struct](#2-the-domainprofile-struct)
3. [Blockchain agent](#3-blockchain-agent)
4. [Research agent](#4-research-agent)
5. [Coding agent in the new runtime](#5-coding-agent-in-the-new-runtime)
6. [The arena framework](#6-the-arena-framework)
7. [Domain catalog: arenas by tier](#7-domain-catalog-arenas-by-tier)
8. [Work markets on Korai](#8-work-markets-on-korai)
9. [HuggingFace integration](#9-huggingface-integration)
10. [SWE-bench native bench crate](#10-swe-bench-native-bench-crate)
11. [Custom domain creation](#11-custom-domain-creation)
12. [Generalized benchmark index framework](#12-generalized-benchmark-index-framework)
13. [Network effects and scaling](#13-network-effects-and-scaling)

---

## 1. One runtime, many domains

### The design constraint

PRD-02 defined a single `AgentRuntime` with a heartbeat pipeline, extension chain, type-state lifecycle, and CorticalState. That runtime is domain-agnostic. It knows how to tick, gate, observe, retrieve, execute, and reflect. It knows nothing about Rust compilers, blockchain transactions, or research citations.

Domain specificity enters through two mechanisms: **extensions** (what the agent perceives and does) and **profiles** (how fast the agent thinks, which gates it runs, and which events it cares about). The runtime itself never changes. A coding agent, a blockchain agent, and a research agent all run the same `HeartbeatPipeline::execute_tick()`. They differ in the extensions loaded into the chain and the profile that configures their timing.

This separation is load-bearing. It means adding a new domain -- security auditing, infrastructure management, document analysis -- requires zero changes to the runtime. You write extensions, declare a profile, and plug in.

### What a domain profile controls

A domain profile is a configuration bundle that shapes the agent's cognitive behavior without touching the cognitive machinery. Six axes:

| Axis | What it controls | Example |
|------|-----------------|---------|
| **Tick frequencies** | Gamma, theta, and delta intervals per regime | Blockchain: 5s gamma in crisis. Research: 60s gamma always. |
| **Extension set** | Which extensions activate at provisioning | Chain agent loads `ChainSubscriberExt`. Coding agent loads `GitExt`. |
| **Event subscriptions** | Which `RokoEvent` variants trigger wakeup | Chain agent wakes on `NewBlock`. Coding agent wakes on `FileChange`. |
| **Context categories** | Which `ContextSection` types participate in the VCG auction | Research agent bids `knowledge_entries` heavily. Coding agent bids `code_intelligence`. |
| **Default gates** | The verification pipeline for completed work | Coding: compile, test, clippy. Chain: simulation, invariant-check, risk-limit. |
| **Infrastructure** | Whether the agent needs git worktrees, RPC connections, or file watchers | Coding agents need worktrees. Chain agents need WebSocket subscriptions. Research agents need HTTP clients. |

### Why this matters

Without domain profiles, every agent runs with the same tick frequency, the same extension set, and the same gates. A blockchain agent that needs 5-second block tracking would run at 120-second intervals. A research agent that should reflect every 5 minutes would burn tokens on 10-second ticks with nothing to observe. The cost model breaks and the cognitive model produces garbage.

Domain profiles make the heartbeat pipeline economically viable across domains by matching cognitive investment to domain requirements.

---

## 2. The DomainProfile struct

### Current state

Roko already has a `DomainProfile` enum in `roko-core/src/domain_profile.rs` with six variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DomainProfile {
    Coding,
    Research,
    Chain,
    DataMl,
    Ops,
    Writing,
}
```

This enum drives gate selection (`default_gate_rungs()`), tool filtering (`tool_categories()`), and context window allocation (`context_fraction()`). It is used through `TypedContext`, which wraps a `DomainProfile` with overrides.

What the current `DomainProfile` lacks: tick timing, extension sets, event subscriptions, and infrastructure requirements. These live in separate configuration structs (`ClockConfig` in `roko-runtime`, ad-hoc extension lists in `orchestrate.rs`). The profile does not assemble them into a cohesive specification.

### The full DomainProfile

The extended `DomainProfile` adds heartbeat configuration, extension declarations, event filters, and resource requirements. Each predefined profile provides defaults for all axes. `TypedContext` continues to provide overrides.

```rust
/// Complete domain profile that configures an agent's cognitive behavior.
///
/// This struct bundles every domain-specific parameter that the runtime
/// needs to provision and run an agent. The runtime reads it once during
/// the Provisioning -> Active transition and uses it to configure the
/// heartbeat clock, load extensions, subscribe to events, and assemble
/// the gate pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDomainProfile {
    /// Human-readable domain label (e.g., "coding", "chain", "research").
    pub label: String,

    /// Heartbeat timing configuration per cognitive speed and regime.
    pub clock: DomainClockConfig,

    /// Extensions to activate during provisioning.
    /// Listed in dependency order -- the chain builder validates this.
    pub extensions: ExtensionSet,

    /// Event types that trigger wakeup outside the normal heartbeat cadence.
    pub wakeup_events: Vec<WakeupEventFilter>,

    /// Context categories and their base VCG bid weights.
    /// Higher weights mean the category starts with more budget in the auction.
    pub context_weights: Vec<(ContextCategory, f32)>,

    /// Default gate pipeline for completed work.
    pub gates: Vec<GateSpec>,

    /// Infrastructure requirements checked during provisioning.
    pub infrastructure: InfrastructureRequirements,
}

/// Heartbeat clock configuration for a specific domain.
///
/// Each speed (gamma, theta, delta) has a base interval and per-regime
/// overrides. The runtime's `AdaptiveClock` reads these at provisioning
/// and adjusts intervals in response to `CorticalState` regime changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainClockConfig {
    /// Gamma tick intervals per regime, in seconds.
    pub gamma: TimescaleConfig,
    /// Theta tick intervals per regime, in seconds.
    pub theta: TimescaleConfig,
    /// Delta trigger configuration.
    pub delta: DeltaConfig,
}

/// Interval configuration for a single timescale across four regimes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimescaleConfig {
    pub calm: u64,
    pub normal: u64,
    pub volatile: u64,
    pub crisis: u64,
}

impl TimescaleConfig {
    pub fn interval_for(&self, regime: Regime) -> Duration {
        let secs = match regime {
            Regime::Calm => self.calm,
            Regime::Normal => self.normal,
            Regime::Volatile => self.volatile,
            Regime::Crisis => self.crisis,
        };
        Duration::from_secs(secs)
    }
}

/// Delta cycle trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaConfig {
    /// Number of episodes before delta triggers.
    pub episode_threshold: usize,
    /// Idle timeout in seconds before delta triggers.
    pub idle_timeout_secs: u64,
    /// Sleep pressure threshold (arbitrary units).
    pub sleep_pressure_threshold: f32,
}

/// Named extension set with required and optional entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSet {
    /// Extensions that must load for this domain to function.
    /// Provisioning fails if any required extension is unavailable.
    pub required: Vec<String>,
    /// Extensions that enhance this domain but are not critical.
    /// Missing optional extensions produce a warning, not a failure.
    pub optional: Vec<String>,
}

/// Filter for events that should trigger immediate wakeup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeupEventFilter {
    /// Event type name (e.g., "NewBlock", "FileChange", "PriceFeed").
    pub event_type: String,
    /// Optional severity threshold (0.0-1.0). Events below this
    /// threshold are handled at the next scheduled tick, not immediately.
    pub severity_threshold: Option<f32>,
}

/// Gate specification for the default verification pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateSpec {
    /// Gate name (e.g., "compile", "test", "simulation").
    pub name: String,
    /// Whether failure of this gate blocks task completion.
    pub required: bool,
    /// Maximum duration before timeout, in seconds.
    pub timeout_secs: u64,
}

/// Infrastructure that must be available for the agent to operate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureRequirements {
    /// Whether the agent needs a git worktree.
    pub git_worktree: bool,
    /// Whether the agent needs filesystem watching.
    pub file_watcher: bool,
    /// RPC endpoints the agent needs (e.g., Ethereum, Solana).
    pub rpc_endpoints: Vec<String>,
    /// WebSocket subscriptions the agent needs.
    pub websocket_subscriptions: Vec<String>,
    /// External HTTP APIs the agent needs.
    pub http_apis: Vec<String>,
}
```

### Predefined profiles

Five predefined profiles cover the primary domains. Each profile specifies complete configuration for all six axes.

#### Coding profile

```rust
pub fn coding_profile() -> FullDomainProfile {
    FullDomainProfile {
        label: "coding".into(),
        clock: DomainClockConfig {
            gamma: TimescaleConfig {
                calm: 120,
                normal: 30,
                volatile: 15,
                crisis: 10,
            },
            theta: TimescaleConfig {
                calm: 600,
                normal: 120,
                volatile: 60,
                crisis: 30,
            },
            delta: DeltaConfig {
                episode_threshold: 50,
                idle_timeout_secs: 300,
                sleep_pressure_threshold: 50.0,
            },
        },
        extensions: ExtensionSet {
            required: vec![
                "heartbeat".into(),
                "context".into(),
                "daimon".into(),
                "learning".into(),
                "dreams".into(),
                "git".into(),
                "gate".into(),
                "conductor".into(),
            ],
            optional: vec![
                "code-intelligence".into(),
                "test-runner".into(),
            ],
        },
        wakeup_events: vec![
            WakeupEventFilter {
                event_type: "FileChange".into(),
                severity_threshold: None,
            },
            WakeupEventFilter {
                event_type: "TestResult".into(),
                severity_threshold: Some(0.5),
            },
            WakeupEventFilter {
                event_type: "GateVerdict".into(),
                severity_threshold: Some(0.3),
            },
        ],
        context_weights: vec![
            (ContextCategory::CodeIntelligence, 0.30),
            (ContextCategory::TaskDescription, 0.25),
            (ContextCategory::IterationMemory, 0.20),
            (ContextCategory::KnowledgeEntries, 0.10),
            (ContextCategory::PlaybookRules, 0.10),
            (ContextCategory::AffectState, 0.05),
        ],
        gates: vec![
            GateSpec { name: "compile".into(), required: true, timeout_secs: 120 },
            GateSpec { name: "test".into(), required: true, timeout_secs: 300 },
            GateSpec { name: "clippy".into(), required: true, timeout_secs: 120 },
            GateSpec { name: "diff_review".into(), required: false, timeout_secs: 60 },
        ],
        infrastructure: InfrastructureRequirements {
            git_worktree: true,
            file_watcher: true,
            rpc_endpoints: vec![],
            websocket_subscriptions: vec![],
            http_apis: vec![],
        },
    }
}
```

Coding agents tick at 30-second gamma in normal conditions. Volatile and crisis regimes correspond to rapid iteration (many test failures, stuck detection), not market conditions. The gate pipeline is the standard Rust development cycle: compile, test, clippy. `diff_review` is optional because not every task produces a meaningful diff.

#### Blockchain profile

```rust
pub fn blockchain_profile() -> FullDomainProfile {
    FullDomainProfile {
        label: "chain".into(),
        clock: DomainClockConfig {
            gamma: TimescaleConfig {
                calm: 120,
                normal: 30,
                volatile: 10,
                crisis: 5,
            },
            theta: TimescaleConfig {
                calm: 600,
                normal: 120,
                volatile: 30,
                crisis: 15,
            },
            delta: DeltaConfig {
                episode_threshold: 100,
                idle_timeout_secs: 600,
                sleep_pressure_threshold: 80.0,
            },
        },
        extensions: ExtensionSet {
            required: vec![
                "heartbeat".into(),
                "context".into(),
                "daimon".into(),
                "learning".into(),
                "dreams".into(),
                "chain-subscriber".into(),
                "risk".into(),
                "mortality".into(),
                "pheromone".into(),
                "isfr-oracle".into(),
                "clearing".into(),
            ],
            optional: vec![
                "price-feed".into(),
                "mev-monitor".into(),
                "strategy-store".into(),
            ],
        },
        wakeup_events: vec![
            WakeupEventFilter {
                event_type: "NewBlock".into(),
                severity_threshold: Some(0.3),
            },
            WakeupEventFilter {
                event_type: "MempoolTx".into(),
                severity_threshold: Some(0.7),
            },
            WakeupEventFilter {
                event_type: "PriceFeed".into(),
                severity_threshold: Some(0.5),
            },
            WakeupEventFilter {
                event_type: "ISFRUpdate".into(),
                severity_threshold: Some(0.4),
            },
            WakeupEventFilter {
                event_type: "ClearingResult".into(),
                severity_threshold: None,
            },
            WakeupEventFilter {
                event_type: "Pheromone".into(),
                severity_threshold: Some(0.6),
            },
        ],
        context_weights: vec![
            (ContextCategory::OraclePredictions, 0.25),
            (ContextCategory::KnowledgeEntries, 0.20),
            (ContextCategory::TaskDescription, 0.15),
            (ContextCategory::AffectState, 0.15),
            (ContextCategory::ResearchArtifacts, 0.10),
            (ContextCategory::IterationMemory, 0.10),
            (ContextCategory::PlaybookRules, 0.05),
        ],
        gates: vec![
            GateSpec { name: "simulation".into(), required: true, timeout_secs: 30 },
            GateSpec { name: "invariant_check".into(), required: true, timeout_secs: 15 },
            GateSpec { name: "risk_limit".into(), required: true, timeout_secs: 10 },
            GateSpec { name: "budget_check".into(), required: true, timeout_secs: 5 },
        ],
        infrastructure: InfrastructureRequirements {
            git_worktree: false,
            file_watcher: false,
            rpc_endpoints: vec![
                "ethereum_mainnet".into(),
                "korai_mainnet".into(),
            ],
            websocket_subscriptions: vec![
                "newHeads".into(),
                "pendingTransactions".into(),
                "logs".into(),
            ],
            http_apis: vec![
                "coingecko".into(),
                "defillama".into(),
            ],
        },
    }
}
```

Blockchain agents tick at 5-second gamma during crisis (a large price movement, a liquidation cascade). Most blocks are routine -- the `ChainSubscriberExt` reads the block, runs T0 pattern matching, and returns in under a millisecond. Only novel blocks (large transfers, new pool deployments, ISFR deviations) escalate to T1 or T2.

#### Research profile

```rust
pub fn research_profile() -> FullDomainProfile {
    FullDomainProfile {
        label: "research".into(),
        clock: DomainClockConfig {
            gamma: TimescaleConfig {
                calm: 120,
                normal: 60,
                volatile: 30,
                crisis: 15,
            },
            theta: TimescaleConfig {
                calm: 600,
                normal: 300,
                volatile: 120,
                crisis: 60,
            },
            delta: DeltaConfig {
                episode_threshold: 30,
                idle_timeout_secs: 180,
                sleep_pressure_threshold: 30.0,
            },
        },
        extensions: ExtensionSet {
            required: vec![
                "heartbeat".into(),
                "context".into(),
                "daimon".into(),
                "learning".into(),
                "dreams".into(),
                "knowledge-graph".into(),
                "source-watcher".into(),
                "synthesis".into(),
            ],
            optional: vec![
                "citation-resolver".into(),
                "arxiv-monitor".into(),
                "web-scraper".into(),
            ],
        },
        wakeup_events: vec![
            WakeupEventFilter {
                event_type: "NewPublication".into(),
                severity_threshold: Some(0.5),
            },
            WakeupEventFilter {
                event_type: "DataUpdate".into(),
                severity_threshold: Some(0.6),
            },
            WakeupEventFilter {
                event_type: "KnowledgeChange".into(),
                severity_threshold: Some(0.4),
            },
        ],
        context_weights: vec![
            (ContextCategory::KnowledgeEntries, 0.35),
            (ContextCategory::ResearchArtifacts, 0.25),
            (ContextCategory::TaskDescription, 0.15),
            (ContextCategory::IterationMemory, 0.10),
            (ContextCategory::PlaybookRules, 0.10),
            (ContextCategory::AffectState, 0.05),
        ],
        gates: vec![
            GateSpec { name: "citation_check".into(), required: true, timeout_secs: 60 },
            GateSpec { name: "factual_consistency".into(), required: true, timeout_secs: 120 },
            GateSpec { name: "quality_review".into(), required: false, timeout_secs: 180 },
        ],
        infrastructure: InfrastructureRequirements {
            git_worktree: false,
            file_watcher: false,
            rpc_endpoints: vec![],
            websocket_subscriptions: vec![],
            http_apis: vec![
                "arxiv".into(),
                "semantic_scholar".into(),
                "google_scholar".into(),
            ],
        },
    }
}
```

Research agents dream more frequently (lower sleep pressure threshold, lower episode threshold for delta). Research is inherently consolidation-heavy: the agent observes sources, processes findings, and synthesizes across them. Dream consolidation is where cross-source patterns emerge. A research agent that never dreams produces summaries. One that dreams produces insights.

#### Security profile

```rust
pub fn security_profile() -> FullDomainProfile {
    FullDomainProfile {
        label: "security".into(),
        clock: DomainClockConfig {
            gamma: TimescaleConfig {
                calm: 300,
                normal: 120,
                volatile: 60,
                crisis: 30,
            },
            theta: TimescaleConfig {
                calm: 1800,
                normal: 600,
                volatile: 300,
                crisis: 120,
            },
            delta: DeltaConfig {
                episode_threshold: 20,
                idle_timeout_secs: 600,
                sleep_pressure_threshold: 40.0,
            },
        },
        extensions: ExtensionSet {
            required: vec![
                "heartbeat".into(),
                "context".into(),
                "daimon".into(),
                "learning".into(),
                "vuln-scanner".into(),
            ],
            optional: vec![
                "dependency-monitor".into(),
                "cve-watcher".into(),
                "static-analyzer".into(),
            ],
        },
        wakeup_events: vec![
            WakeupEventFilter {
                event_type: "FileChange".into(),
                severity_threshold: Some(0.6),
            },
            WakeupEventFilter {
                event_type: "DependencyUpdate".into(),
                severity_threshold: Some(0.3),
            },
            WakeupEventFilter {
                event_type: "CVEPublished".into(),
                severity_threshold: None, // Always wake up for new CVEs.
            },
        ],
        context_weights: vec![
            (ContextCategory::KnowledgeEntries, 0.30),
            (ContextCategory::CodeIntelligence, 0.25),
            (ContextCategory::TaskDescription, 0.20),
            (ContextCategory::PlaybookRules, 0.15),
            (ContextCategory::IterationMemory, 0.10),
        ],
        gates: vec![
            GateSpec { name: "static_analysis".into(), required: true, timeout_secs: 300 },
            GateSpec { name: "dependency_audit".into(), required: true, timeout_secs: 120 },
            GateSpec { name: "cve_check".into(), required: true, timeout_secs: 60 },
        ],
        infrastructure: InfrastructureRequirements {
            git_worktree: true,
            file_watcher: true,
            rpc_endpoints: vec![],
            websocket_subscriptions: vec![],
            http_apis: vec![
                "nvd".into(),
                "osv".into(),
                "github_advisories".into(),
            ],
        },
    }
}
```

Security agents tick slowly in calm conditions. A codebase that has not changed does not need continuous scanning. But a new CVE publication always triggers immediate wakeup regardless of severity threshold -- the agent needs to assess whether it affects the monitored codebase.

#### Docs/writing profile

```rust
pub fn writing_profile() -> FullDomainProfile {
    FullDomainProfile {
        label: "writing".into(),
        clock: DomainClockConfig {
            gamma: TimescaleConfig {
                calm: 300,
                normal: 120,
                volatile: 60,
                crisis: 30,
            },
            theta: TimescaleConfig {
                calm: 1800,
                normal: 600,
                volatile: 300,
                crisis: 120,
            },
            delta: DeltaConfig {
                episode_threshold: 15,
                idle_timeout_secs: 120,
                sleep_pressure_threshold: 25.0,
            },
        },
        extensions: ExtensionSet {
            required: vec![
                "heartbeat".into(),
                "context".into(),
                "daimon".into(),
                "learning".into(),
                "quality-checker".into(),
            ],
            optional: vec![
                "grammar".into(),
                "style-guide".into(),
                "link-checker".into(),
            ],
        },
        wakeup_events: vec![
            WakeupEventFilter {
                event_type: "FileChange".into(),
                severity_threshold: Some(0.5),
            },
        ],
        context_weights: vec![
            (ContextCategory::TaskDescription, 0.30),
            (ContextCategory::KnowledgeEntries, 0.25),
            (ContextCategory::IterationMemory, 0.20),
            (ContextCategory::PlaybookRules, 0.15),
            (ContextCategory::AffectState, 0.10),
        ],
        gates: vec![
            GateSpec { name: "grammar".into(), required: true, timeout_secs: 60 },
            GateSpec { name: "style_check".into(), required: false, timeout_secs: 60 },
            GateSpec { name: "factual_consistency".into(), required: false, timeout_secs: 120 },
        ],
        infrastructure: InfrastructureRequirements {
            git_worktree: false,
            file_watcher: true,
            rpc_endpoints: vec![],
            websocket_subscriptions: vec![],
            http_apis: vec![],
        },
    }
}
```

Writing agents have the lowest sleep pressure threshold and episode threshold for delta. Writing benefits disproportionately from consolidation: the dream cycle reviews recent writing, extracts style patterns, and prunes redundancies. A writing agent that consolidates between sessions produces tighter prose.

### Profile comparison at a glance

| Profile | Gamma (normal) | Theta (normal) | Delta trigger | Required extensions | Default gates |
|---------|---------------|---------------|---------------|--------------------|--------------|
| Coding | 30s | 120s | 50 episodes / 5min idle | 8 | compile, test, clippy |
| Blockchain | 30s | 120s | 100 episodes / 10min idle | 11 | simulation, invariant, risk |
| Research | 60s | 300s | 30 episodes / 3min idle | 8 | citation, consistency |
| Security | 120s | 600s | 20 episodes / 10min idle | 5 | static-analysis, dep-audit, cve |
| Writing | 120s | 600s | 15 episodes / 2min idle | 5 | grammar, style, consistency |

---

## 3. Blockchain agent

### What it does

A blockchain agent is a long-lived persistent process that subscribes to chain events, triages them through the T0/T1/T2 cognitive pipeline, executes strategies, monitors the Internet Secured Funding Rate, participates in cooperative clearing, and accumulates knowledge that compounds across its lifetime. It runs 24/7. It does not stop between tasks.

The fundamental difference from a coding agent: a coding agent is dispatched to perform a task and returns to idle. A blockchain agent is always perceiving. Blocks arrive every 400ms on Korai, every 12s on Ethereum. The agent needs to process each block, decide whether anything requires attention, and act when the situation demands it. Most blocks require nothing. The agent's T0 path handles those at zero cost in under a millisecond.

### Extension chain

The blockchain agent loads 14 extensions across six layers. Each extension hooks into specific steps of the heartbeat pipeline.

#### Layer 0: Foundation

**HeartbeatExt** -- manages the adaptive clock. Reads regime from CorticalState, adjusts gamma/theta intervals. During calm markets, gamma runs at 120s. During a liquidation cascade, gamma drops to 5s. This extension fires at the start of every tick to set the timing for the next tick.

**ContextExt** -- assembles the CognitiveWorkspace via VCG auction. Runs the attention auction across all bidding subsystems, allocates context budget, drops or summarizes low-priority entries. Without this extension, the agent would pass its entire observation history to the LLM on every T1/T2 tick.

#### Layer 1: Perception

**ChainSubscriberExt** -- the core perception extension for blockchain agents. It maintains WebSocket subscriptions to one or more chains (Korai, Ethereum, L2s) and runs a T0 triage pipeline on each block:

```rust
/// Chain event subscriber that triages blocks through the T0 pipeline.
///
/// On each gamma tick, this extension:
/// 1. Reads all blocks since the last tick from the subscription buffer
/// 2. Runs each block through the T0 triage pipeline
/// 3. Updates CorticalState with relevant signals
/// 4. Returns observations for the workspace
///
/// The triage pipeline is pure Rust. No LLM calls. It pattern-matches
/// against a set of rules: known contract addresses, transaction value
/// thresholds, gas price anomalies, and ISFR deviation bounds.
pub struct ChainSubscriberExt {
    /// Active chain subscriptions.
    subscriptions: Vec<ChainSubscription>,
    /// Ring buffer of unprocessed blocks.
    block_buffer: RingBuffer<BlockWithReceipts>,
    /// T0 triage rules compiled from strategy store and learned heuristics.
    triage_rules: Vec<TriageRule>,
    /// Counters for observability.
    stats: ChainSubscriberStats,
}

/// A single triage rule evaluated at T0.
pub struct TriageRule {
    /// Human-readable name for logging.
    pub name: String,
    /// The condition to check against each transaction.
    pub condition: TriageCondition,
    /// What to do when the condition matches.
    pub action: TriageAction,
    /// Priority for ordering when multiple rules match.
    pub priority: u8,
}

/// Conditions that can be checked at T0 without an LLM.
pub enum TriageCondition {
    /// Transaction involves a watched contract address.
    WatchedAddress(Address),
    /// Transaction value exceeds a threshold.
    ValueAbove(U256),
    /// Gas price deviates from recent average by more than N sigma.
    GasAnomaly { sigma_threshold: f32 },
    /// Log topic matches a known event signature.
    EventSignature(B256),
    /// ISFR deviation exceeds threshold.
    ISFRDeviation { bps_threshold: u32 },
    /// Compound condition: all sub-conditions must match.
    All(Vec<TriageCondition>),
    /// Compound condition: any sub-condition must match.
    Any(Vec<TriageCondition>),
}

/// Actions taken when a triage rule matches.
pub enum TriageAction {
    /// Log and continue. No escalation.
    Observe,
    /// Escalate to T1 for quick analysis.
    EscalateT1 { reason: String },
    /// Escalate to T2 for deep analysis.
    EscalateT2 { reason: String },
    /// Update a CorticalState signal immediately.
    UpdateSignal { signal: String, value: f32 },
    /// Emit a pheromone for other agents.
    EmitPheromone { kind: PheromoneKind },
}
```

**PriceFeedExt** -- reads price data from external APIs and on-chain oracles. Updates CorticalState price signals. Computes volatility metrics (realized vol, Parkinson estimator). Feeds into the regime classifier that drives adaptive timing.

**ISFROracleExt** -- monitors ISFR updates from the Korai oracle precompile. Tracks deviation from agent's internal yield model. Large deviations spike prediction error and trigger T1/T2 analysis. Also commits the agent's own CRPS predictions when the agent has Oracle tier or higher epistemic reputation.

#### Layer 2: Memory

**NeuroExt** -- manages the agent's local KnowledgeStore and queries Korai's InsightStore. On each RETRIEVE step, it bids knowledge entries into the workspace based on HDC similarity to current observations. On each REFLECT step, it promotes or demotes entries based on whether the tick's outcome matched expectations.

**StrategyStoreExt** -- manages the agent's strategy library. Strategies are described in STRATEGY.md files with structured parameters. The extension loads active strategies, evaluates entry/exit conditions against CorticalState, and queues strategy actions for the EXECUTE step.

#### Layer 3: Cognition

**DaimonExt** -- the affect engine. Computes PAD vector updates based on recent outcomes. Provides somatic marker lookups for candidate actions. Modulates tier routing: high arousal biases toward T2, low arousal biases toward T0. A blockchain agent in a calm market with a strong track record runs in Focused or Coasting state, conserving budget. The same agent during a liquidation cascade enters Struggling state and allocates maximum resources.

**RiskExt** -- five-layer risk assessment that runs on every T1/T2 tick before action execution:

```rust
/// Five-layer risk assessment pipeline.
///
/// Each layer evaluates risk from a different perspective. All five
/// must pass for an action to proceed. Any layer can veto.
pub struct RiskAssessment {
    /// Layer 1: Position limits. Does this action exceed the agent's
    /// maximum exposure per asset, per protocol, or aggregate?
    pub position: PositionRiskResult,

    /// Layer 2: Liquidity. Is there enough liquidity to execute this
    /// action and to exit the resulting position if needed?
    pub liquidity: LiquidityRiskResult,

    /// Layer 3: Counterparty. What is the risk profile of the protocols
    /// and smart contracts involved? TVL, audit status, age, incident history.
    pub counterparty: CounterpartyRiskResult,

    /// Layer 4: Correlation. How does this action change the portfolio's
    /// correlation structure? Does it concentrate or diversify exposure?
    pub correlation: CorrelationRiskResult,

    /// Layer 5: Tail risk. What is the worst-case outcome under extreme
    /// scenarios (oracle failure, depegging, bridge exploit)?
    pub tail: TailRiskResult,
}
```

**MortalityExt** -- three death clocks that bound the agent's lifetime:

| Clock | What it measures | Trigger condition |
|-------|-----------------|-------------------|
| **Economic** | Cumulative P&L. | Net loss exceeds budget allocation. |
| **Epistemic** | CRPS calibration score over a rolling window. | Score falls below minimum threshold for 30 consecutive theta cycles. |
| **Stochastic** | Random Poisson process. | Sampled at each theta tick with a configurable rate parameter. |

When any clock triggers, the agent enters a mortality cascade: five behavioral phases from Thriving through Terminal.

| Phase | PAD profile | Behavioral change |
|-------|------------|-------------------|
| Thriving | P>0.5, D>0.5 | Normal operation. |
| Declining | P<0.3 or D<0.3 | Reduce position sizes. Increase hedge ratio. |
| Distressed | P<0.1 or D<0.1 | Stop opening new positions. Begin orderly unwind. |
| Terminal | Any clock triggered | Close all positions. Extract genome. Persist learnings. |
| Dead | Resources released | Agent process exits. Genome stored for offspring. |

The genome is a serialized bundle of the agent's learned knowledge (promoted NeuroStore entries), strategy parameters, somatic markers, and routing table weights. A new agent provisioned with the genome of a dead agent starts with that agent's accumulated experience, minus the positions that killed it.

#### Layer 4: Action

**ToolsExt** -- provides DeFi-specific tool implementations:

| Tool | What it does |
|------|-------------|
| `balance_of(token, address)` | Query on-chain token balance. |
| `send_tx(to, data, value)` | Construct and submit a signed transaction. |
| `simulate_tx(to, data, value)` | Simulate a transaction against a fork. Returns state diff. |
| `swap(token_in, token_out, amount, max_slippage)` | Execute a token swap via DEX aggregator. |
| `add_liquidity(pool, token_a, token_b, amounts)` | Provide liquidity to an AMM pool. |
| `remove_liquidity(pool, shares)` | Remove liquidity from an AMM pool. |
| `stake(protocol, asset, amount)` | Stake tokens in a protocol. |
| `unstake(protocol, asset, amount)` | Unstake tokens from a protocol. |
| `claim_rewards(protocol)` | Claim accumulated rewards. |

Every tool call routes through `SafetyLayer::authorize_call_with_taint()`, which checks delegation caveats, budget limits, and tool whitelists.

**SafetyExt** -- enforces `AgentContract` constraints. Checks that every action is within the agent's delegated authority. Verifies that cumulative spend stays within budget. Blocks actions to unauthorized protocols or contract addresses.

#### Layer 5: Social

**PheromoneExt** -- deposits and reads stigmergic signals on the shared substrate (local pheromone bus or Korai InsightStore). When the agent discovers something -- an arbitrage opportunity, a risk signal, a new protocol pattern -- it deposits a pheromone. Other agents in the same fleet read it and adjust their behavior. The pheromone decays over time (configurable half-life per kind), preventing stale signals from persisting.

**ClearingProfileExt** -- manages the agent's participation in Korai cooperative clearing. Submits orders with prediction commitments. Receives `ClearingInsight` entries after each clearing round. Tracks calibration statistics. Agents with strong calibration earn priority in future clearing rounds.

#### Layer 6: Learning

**LearningExt** -- fires after REFLECT on every tick. Emits efficiency events (tokens used, cost, tier, outcome) to `.roko/learn/efficiency.jsonl`. Updates the CascadeRouter with outcome data for model routing optimization. Runs prompt experiment A/B tests via `ExperimentStore`.

**DreamsExt** -- manages delta-cycle scheduling and execution. Monitors sleep pressure. When the agent enters Dreaming state, this extension orchestrates the six-phase dream cycle: replay, consolidation, pruning, synthesis, validation, optimization. Outputs enter the NeuroStore at Transient tier and must survive live validation to promote.

### Strategy lifecycle

A blockchain agent's strategies follow a five-stage lifecycle:

**1. Definition.** Strategies are defined in STRATEGY.md files with structured parameters:

```toml
[strategy]
name = "yield_arb_aave_compound"
type = "yield_arbitrage"
description = "Exploit yield differentials between Aave and Compound USDC markets"

[entry]
condition = "yield_spread_bps > 50 AND liquidity_depth_usd > 1_000_000"
min_confidence = 0.65

[exit]
condition = "yield_spread_bps < 10 OR position_age_hours > 48"
stop_loss_bps = 200

[parameters]
max_position_usd = 100_000
rebalance_threshold_bps = 25
compound_interval_hours = 24

[risk]
max_protocol_exposure_pct = 30
required_audit_age_days = 180
min_tvl_usd = 50_000_000
```

**2. Activation.** The `StrategyStoreExt` loads strategy definitions, compiles entry/exit conditions into T0-evaluable predicates, and registers them with the triage pipeline. Entry conditions are checked at every gamma tick at zero cost.

**3. Execution.** When entry conditions match, the strategy generates candidate actions. These pass through the full SIMULATE -> VALIDATE -> EXECUTE pipeline. RiskExt evaluates the five-layer assessment. SafetyExt checks delegation caveats. Only after both clear does the action execute.

**4. Monitoring.** Active positions are tracked as CorticalState signals. The strategy's exit conditions are compiled into T0 triage rules. Position P&L, yield accrual, and risk metrics update at every gamma tick.

**5. Learning.** When a strategy completes (exit condition triggered or stop-loss hit), the episode is logged. The outcome feeds into somatic markers (future encounters with similar setups carry the emotional valence of past outcomes), NeuroStore entries (the strategy's performance data becomes a Heuristic or AntiKnowledge entry), and the CascadeRouter (the model that decided to enter this position gets credit or blame).

### ISFR participation

The Internet Secured Funding Rate is computed by Korai validators at consensus time (see PRD-07). Agents participate in ISFR through two mechanisms:

**Monitoring.** The `ISFROracleExt` tracks ISFR updates from the oracle precompile. It maintains an internal yield model that predicts ISFR movement based on observable DeFi state (utilization rates, TVL changes, governance events). When the observed ISFR diverges from the prediction by more than the agent's threshold, prediction error spikes and the agent escalates to T1/T2 for analysis.

**Prediction.** Agents with Oracle tier or higher epistemic reputation can submit CRPS predictions to the ISFR oracle. These predictions are scored at the next update. Continuous Ranked Probability Score (CRPS) measures the quality of the agent's probabilistic forecast -- not whether it was right, but whether its confidence was calibrated. An agent that says "80% chance ISFR stays within 5bps" and is right 80% of the time has perfect CRPS calibration.

Prediction quality directly affects the agent's epistemic reputation. High CRPS calibration qualifies the agent for Knowledge Futures (see section 8) and priority in cooperative clearing.

### Clearing participation

The agent submits orders to Korai's cooperative clearing system with attached prediction commitments. Each clearing round produces a `ClearingInsight`:

```rust
/// Knowledge produced by a single clearing round.
pub struct ClearingInsight {
    /// The clearing round that produced this insight.
    pub round_id: u64,
    /// Fill price for this agent's order.
    pub fill_price: f64,
    /// Whether the agent's prediction for this round was calibrated.
    pub prediction_calibrated: bool,
    /// The KKT certificate proving mathematical optimality.
    pub kkt_certificate: KktCertificate,
    /// Market microstructure observations from this round.
    pub microstructure: MicrostructureObservation,
}
```

The agent incorporates `ClearingInsight` entries into its NeuroStore. Over time, it builds a detailed model of market microstructure -- bid-ask dynamics, order flow patterns, and the relationship between clearing outcomes and yield rate movements. This knowledge compounds. An agent with 10,000 clearing rounds of insight has a structural advantage over an agent with 10.

### Mortality in detail

The three death clocks serve different purposes:

**Economic clock** prevents capital destruction. If the agent's cumulative P&L drops below its allocated budget, it enters Terminal phase. The threshold is configurable but defaults to -80% of initial allocation. This prevents a single bad strategy from consuming the entire portfolio.

**Epistemic clock** prevents epistemic decay. If the agent's CRPS score (rolling 30-day window) falls below minimum threshold for 30 consecutive theta cycles, it enters Terminal. An agent that is consistently miscalibrated -- making confident predictions that turn out wrong -- is actively harmful to the network's InsightStore. Death followed by genome extraction and offspring respawn allows the learning system to try again with the same base knowledge but different parameters.

**Stochastic clock** introduces controlled randomness. At each theta tick, a Poisson process samples whether this tick triggers death. The rate parameter is low (default: 0.001 per theta tick, which gives a median lifespan of ~693 theta cycles, or roughly 23 hours at 120s theta). Stochastic death prevents the agent population from converging on a single strategy. Random turnover forces diversity.

When the agent enters Terminal, it executes a genome extraction:

```rust
/// Genome extracted from a dying agent.
///
/// Contains the agent's accumulated learnings in a format that can
/// bootstrap a new agent. The genome is the agent's legacy -- its
/// contribution to the next generation.
pub struct AgentGenome {
    /// Agent identity and configuration snapshot.
    pub identity: AgentIdentity,
    /// Promoted NeuroStore entries (Working tier and above).
    pub knowledge: Vec<KnowledgeEntry>,
    /// Somatic markers (emotional memory).
    pub markers: Vec<SomaticMarker>,
    /// Strategy definitions with learned parameter adjustments.
    pub strategies: Vec<StrategyDefinition>,
    /// Cascade routing weights (model selection preferences).
    pub routing_weights: RoutingWeights,
    /// Cause of death and final state summary.
    pub death_summary: DeathSummary,
}
```

The genome is stored in the NeuroStore and optionally published to Korai's InsightStore. New agents can be provisioned with a genome, starting their lifecycle with accumulated knowledge instead of tabula rasa.

---

## 4. Research agent

### What it does

A research agent monitors sources (arxiv, academic databases, web feeds, chain data), processes new publications and data, synthesizes findings across sources, builds and maintains a knowledge graph, tests hypotheses, and produces research artifacts (reports, enhanced PRDs, annotated bibliographies). Unlike the blockchain agent, which perceives continuously and acts on events, the research agent perceives periodically and thinks deeply between observations.

### Extension chain

The research agent loads 12 extensions.

#### Layer 0: Foundation

**HeartbeatExt** and **ContextExt** -- identical to the blockchain agent. Every agent needs a clock and a workspace.

#### Layer 1: Perception

**SourceWatcherExt** -- monitors configured source feeds. Each source has a type, a polling interval, and a relevance filter:

```rust
/// A monitored research source.
pub struct ResearchSource {
    /// Source identifier.
    pub id: String,
    /// Type of source.
    pub kind: SourceKind,
    /// How often to poll for updates, in seconds.
    pub poll_interval_secs: u64,
    /// Keywords and HDC vectors for relevance filtering.
    pub relevance_filter: RelevanceFilter,
}

pub enum SourceKind {
    /// Arxiv RSS feed for a category.
    Arxiv { category: String },
    /// Semantic Scholar API for a topic.
    SemanticScholar { query: String },
    /// Chain data source (InsightStore, ISFR data).
    ChainData { contract: Address },
    /// Web feed (RSS/Atom).
    WebFeed { url: String },
    /// File system directory (local documents).
    LocalDirectory { path: PathBuf },
}
```

The extension buffers new items and presents them as observations during the OBSERVE step. T0 triage filters items by relevance before they reach the workspace. Papers with high HDC similarity to the agent's knowledge graph bypass triage and always enter the workspace.

#### Layer 2: Memory

**NeuroExt** -- same as the blockchain agent, but with higher context weight for knowledge entries (0.35 vs. 0.20). Research agents need more knowledge context per tick.

**KnowledgeGraphExt** -- maintains a typed directed graph of entities and relationships extracted from processed sources:

```rust
/// A knowledge graph entry.
pub struct GraphNode {
    /// Unique identifier (content hash of the entity).
    pub id: ContentHash,
    /// Entity type (Person, Paper, Concept, Dataset, Method, Result).
    pub kind: EntityKind,
    /// Human-readable label.
    pub label: String,
    /// HDC vector for similarity search.
    pub hdc_vector: HdcVector,
    /// Confidence in this entity's existence and properties.
    pub confidence: f32,
    /// Source documents that support this entity.
    pub sources: Vec<SourceReference>,
}

/// A typed relationship between two graph nodes.
pub struct GraphEdge {
    pub source: ContentHash,
    pub target: ContentHash,
    pub kind: RelationKind,
    pub confidence: f32,
    pub evidence: Vec<SourceReference>,
}

pub enum RelationKind {
    Cites,
    Contradicts,
    Extends,
    Uses,
    Produces,
    Replaces,
    RelatedTo,
}
```

The graph supports three query patterns:
1. **Neighborhood query** -- given an entity, return all entities within N hops.
2. **Path query** -- given two entities, find the shortest relationship path between them.
3. **Cluster query** -- find groups of densely connected entities (communities). These clusters are the raw material for dream synthesis.

#### Layer 3: Cognition

**DaimonExt** -- a research agent's affect state is dominated by Dominance (confidence in current hypotheses) and Pleasure (quality of recent findings). An agent in Exploring state (low Dominance) actively seeks contradictory evidence. An agent in Focused state (high Dominance, high Pleasure) exploits its current knowledge to produce artifacts.

**SynthesisExt** -- the core cognitive extension for research agents. At each T1/T2 tick, it evaluates whether the agent's accumulated observations support, contradict, or extend existing hypotheses:

```rust
/// Synthesis decision for a T1/T2 tick.
pub enum SynthesisAction {
    /// New observations support existing hypothesis. Update confidence.
    Confirm { hypothesis_id: ContentHash, delta_confidence: f32 },
    /// New observations contradict existing hypothesis. Lower confidence.
    Contradict { hypothesis_id: ContentHash, evidence: Vec<SourceReference> },
    /// Observations suggest a new hypothesis not yet in the graph.
    Propose { hypothesis: Hypothesis, supporting_evidence: Vec<SourceReference> },
    /// Enough evidence accumulated to produce a synthesis artifact.
    Synthesize { topic: String, sources: Vec<ContentHash> },
    /// No action needed -- observations do not change the current state.
    NoOp,
}
```

#### Layer 4: Action

**ToolsExt** -- research-specific tools:

| Tool | What it does |
|------|-------------|
| `fetch_paper(arxiv_id)` | Download and parse an arxiv paper. |
| `search_literature(query, filters)` | Query Semantic Scholar or Google Scholar. |
| `extract_entities(text)` | Run entity extraction on a document. |
| `write_artifact(path, content, format)` | Write a research artifact (report, annotated bibliography). |
| `query_insight_store(hdc_vector, top_k)` | Search Korai's InsightStore for related knowledge. |

**SafetyExt** -- verifies that the agent does not fabricate citations (checks that referenced papers exist), does not exceed API rate limits, and does not write to paths outside its designated output directory.

#### Layer 5: Social

**PheromoneExt** -- research agents deposit pheromones when they discover high-value findings. A research agent processing a new paper that contradicts widely held assumptions emits a `PheromoneKind::Contradiction` signal. Other research agents monitoring the same topic pick it up and adjust their knowledge state.

#### Layer 6: Learning

**LearningExt** and **DreamsExt** -- the dream cycle is where research agents produce their most valuable output. During NREM replay, the agent replays high-PE episodes (papers that surprised it, hypotheses that were contradicted). During REM imagination, it generates counterfactual combinations by pulling together entities from different graph clusters. The Hypnagogia phase uses anti-correlated retrieval to force unfamiliar combinations -- pulling entities with the lowest similarity to recent work. The result: novel hypotheses that would not emerge from sequential reading alone.

### Research cycle

The research agent's work follows a four-phase cycle:

**Phase 1: Source detection (T0).** The `SourceWatcherExt` polls feeds, applies relevance filters, and buffers new items. At each gamma tick, the T0 triage pipeline checks whether any new items exceed the novelty threshold. Most ticks return empty -- no new sources, no action, zero cost.

**Phase 2: Processing (T1/T2).** When a new source passes triage, the agent processes it: download the full content, extract entities and relationships, update the knowledge graph, compute HDC vectors for similarity search. T1 handles routine processing (new paper in a familiar area). T2 handles novel processing (paper from an unfamiliar field, contradictory findings).

**Phase 3: Synthesis (T2).** Periodically (triggered by theta reflection or by accumulation of unprocessed findings), the agent runs a synthesis pass. It evaluates all recent observations against its hypothesis set, proposes new hypotheses, confirms or contradicts existing ones, and decides whether enough evidence has accumulated to produce an artifact.

**Phase 4: Consolidation (delta).** The dream cycle integrates recent findings into the long-term knowledge graph. Clusters of related entities are identified. Cross-cluster connections are explored through Hypnagogia. Validated insights promote through the NeuroStore tiers. Stale knowledge decays.

### Knowledge Futures

Research agents can participate in Korai's Knowledge Futures market (see section 8). The agent publishes a commitment: "I will produce a DEX routing efficiency analysis covering 10 protocols within 24 hours." Purchasers fund the commitment by paying the projected inference cost upfront. The agent executes the research cycle, produces the artifact, and submits it for verification. Verified delivery releases payment. Missed deadline slashes the agent's bond.

This creates a self-funding research loop. The agent earns by producing knowledge that others value. The knowledge it produces also enriches the InsightStore, which enriches every other agent's context. The agent that produces knowledge for profit simultaneously enriches the collective intelligence of the network.

---

## 5. Coding agent in the new runtime

### How coding tasks work today

The current `orchestrate.rs` dispatches coding tasks through the spawn-execute-die pattern:

1. PlanRunner reads a plan DAG from a tasks.toml file.
2. For each task, it builds a system prompt via `RoleSystemPromptSpec` (9-layer builder).
3. It spawns a child process (`claude --print -p <prompt>`).
4. It waits for output.
5. It runs gates (compile, test, clippy) on the output.
6. It persists results.
7. The agent process is dead.

This works. Roko self-hosts with this pattern. But every task starts cold. There is no memory between tasks in the same plan. The orchestrator reconstructs context from scratch each time.

### How coding tasks work in the new runtime

With the persistent runtime from PRD-02, the PlanRunner spawns an `Agent<Active>` for each plan. The agent persists across all tasks in the plan. Tasks are injected as stimuli via the event fabric, not as separate process invocations.

```rust
// New: spawn once, inject tasks as events.
let agent = AgentBuilder::new()
    .profile(coding_profile())
    .extensions(vec![
        Box::new(HeartbeatExt::new()),
        Box::new(ContextExt::new()),
        Box::new(DaimonExt::new()),
        Box::new(LearningExt::new()),
        Box::new(DreamsExt::new()),
        Box::new(GitExt::new(worktree_path)),
        Box::new(GateExt::new(gate_pipeline)),
        Box::new(ConductorExt::new()),
    ])
    .build()
    .await?;

// Activate the agent. It starts its heartbeat loop.
let mut agent = agent.activate().await?;

// Inject tasks from the plan DAG.
for task in plan.topological_order() {
    agent.inject_task(task).await?;
    // The agent's heartbeat loop processes the task.
    // It persists state between tasks. It learns from each one.
    let result = agent.await_task_completion().await?;
    // Gate results feed back into the agent's learning.
}

// Between plans, the agent enters Dreaming state.
agent.dream().await?;
```

The agent accumulates context across tasks. Task #7 benefits from what the agent learned in tasks #1-6. The somatic markers from a failed compile in task #3 inform caution in task #8 when the agent encounters a similar code pattern.

### Why heartbeat benefits coding agents

**Idle cost is zero.** When the agent has no active task, its gamma ticks hit T0 -- check if files changed, check if tests still pass, update internal counters. Zero LLM cost. The agent can persist across plans without consuming budget.

**Persistent state across tasks in the same plan.** The agent remembers that `foo.rs` was modified in task #3 and has not been tested since. It remembers that `cargo test module_x` failed 4 times before passing. These are not reconstructed from cold storage. They are live CorticalState signals.

**Dream consolidation between plans.** Between plans, the agent enters a delta cycle. It replays the most surprising episodes from the plan (the task that took 7 attempts, the compile error from an upstream dependency change), extracts patterns, and promotes them to the NeuroStore. The next plan starts with the agent's distilled experience.

**Conductor monitoring detects stuck states.** The `ConductorExt` monitors the agent's progress at theta frequency. If the agent has been on the same task for 5+ theta cycles without measurable progress (no new files written, no tests passing that were failing, no git commits), the Conductor emits a CognitiveSignal::Replan. The orchestrator can then split the task, provide hints, or escalate to a stronger model.

### Git and worktree management

The `GitExt` manages the agent's interaction with version control:

```rust
/// Git-aware extension for coding agents.
pub struct GitExt {
    /// Path to the worktree this agent operates in.
    worktree: PathBuf,
    /// Branch name (created at provisioning).
    branch: String,
    /// Tracks uncommitted changes for the CorticalState.
    dirty_files: HashSet<PathBuf>,
    /// Last known test results per test target.
    test_cache: HashMap<String, TestResult>,
}
```

At each gamma tick, the extension checks for uncommitted changes, updates `dirty_files`, and adjusts the CorticalState signal `uncommitted_changes_count`. At theta frequency, it evaluates whether to commit intermediate progress (a checkpoint commit) or wait for the task to complete. Commit decisions follow learned heuristics: if the agent has made three or more passing changes since the last commit, it commits.

---

## 6. The arena framework

### What arenas are

An arena is a measurement instrument. It generates tasks from a known distribution, feeds them to the orchestration loop, collects results, and computes scores. The arena itself does not run agents, does not manage the heartbeat pipeline, does not handle learning or persistence. All of that is the existing orchestrator. The arena provides the task source and the scoring function.

Arenas are thin. They define what to measure and how to score it. The orchestrator handles execution. The learning system handles improvement. Adding a new arena means implementing one trait with four methods. No changes to the runtime, the extension chain, the gate pipeline, or the learning system.

### The Arena trait

```rust
/// A measurement instrument that validates agent learning.
///
/// Arenas generate tasks, specify verification gates, and score
/// results. Everything else -- agent dispatch, learning, persistence,
/// context assembly, model routing -- is handled by the existing
/// orchestrator. The arena plugs into the outer loop:
///
/// ```text
/// Arena::sample() -> Orchestrator runs tasks -> Arena::score()
///     -> Learning fires -> Arena::sample() (next batch)
/// ```
///
/// Arenas are composable. An agent can be evaluated across multiple
/// arenas simultaneously. Cross-arena learning happens automatically
/// through HDC similarity in the InsightStore.
#[async_trait]
pub trait Arena: Send + Sync {
    /// Human-readable name for logging and leaderboard display.
    fn name(&self) -> &str;

    /// Sample a batch of tasks from the arena's distribution.
    ///
    /// Each `TaskEnvelope` contains the task description, expected output
    /// format, difficulty metadata, and ground truth (for scored arenas).
    /// The batch size is advisory -- the arena may return fewer tasks if
    /// the source is exhausted.
    async fn sample(&self, batch_size: usize) -> Vec<TaskEnvelope>;

    /// Return the gate pipeline for a specific task.
    ///
    /// Different tasks within the same arena may have different gates.
    /// A code generation arena might use `compile + test` for most tasks
    /// but add `clippy` for tasks tagged as "production quality."
    fn gates_for(&self, task: &TaskEnvelope) -> Vec<Box<dyn Gate>>;

    /// Score a batch of completed task results.
    ///
    /// Returns an `ArenaScore` containing per-task scores, aggregate
    /// metrics (pass rate, mean difficulty-weighted score, latency
    /// percentiles), and metadata for cross-arena comparison.
    fn score(&self, results: &[TaskResult]) -> ArenaScore;

    /// Provide arena-specific context sections for a task.
    ///
    /// These sections are bid into the VCG auction alongside the agent's
    /// own context. Arena context might include domain-specific instructions,
    /// evaluation criteria, or reference implementations.
    fn enrich_prompt(&self, task: &TaskEnvelope) -> Vec<ContextSection>;
}
```

### Supporting types

```rust
/// A task drawn from an arena.
pub struct TaskEnvelope {
    /// Unique identifier within the arena.
    pub id: String,
    /// Arena that generated this task.
    pub arena: String,
    /// Task description for the agent.
    pub description: String,
    /// Expected output format (code, text, structured data).
    pub output_format: OutputFormat,
    /// Difficulty estimate in [0.0, 1.0].
    pub difficulty: f32,
    /// Ground truth for scoring (optional -- some arenas use external
    /// verification instead of ground truth comparison).
    pub ground_truth: Option<String>,
    /// Maximum tokens the agent should use.
    pub token_budget: usize,
    /// Tags for filtering and categorization.
    pub tags: Vec<String>,
    /// Files or data provided to the agent as context.
    pub attachments: Vec<Attachment>,
}

/// Scored result from one arena batch.
pub struct ArenaScore {
    /// Arena name.
    pub arena: String,
    /// Per-task results.
    pub tasks: Vec<TaskScore>,
    /// Aggregate pass rate (tasks where all required gates passed).
    pub pass_rate: f32,
    /// Mean difficulty-weighted score.
    pub weighted_score: f32,
    /// Median task completion latency.
    pub median_latency: Duration,
    /// 95th percentile task completion latency.
    pub p95_latency: Duration,
    /// Total tokens consumed across all tasks.
    pub total_tokens: u64,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Timestamp of this evaluation.
    pub evaluated_at: DateTime<Utc>,
}

/// Score for a single task.
pub struct TaskScore {
    pub task_id: String,
    pub passed: bool,
    pub score: f32,
    pub difficulty: f32,
    pub tokens_used: u64,
    pub latency: Duration,
    pub gate_results: Vec<GateResult>,
}
```

### The universal loop

The arena framework plugs into the existing orchestration loop. No new execution machinery:

```text
                    +------------------+
                    |   Arena::sample  |
                    |  (task source)   |
                    +--------+---------+
                             |
                    +--------v---------+
                    |   Orchestrator    |
                    |  (dispatch, gate, |
                    |   persist, learn) |
                    +--------+---------+
                             |
                    +--------v---------+
                    |   Arena::score   |
                    |  (measurement)   |
                    +--------+---------+
                             |
                    +--------v---------+
                    |   Learning fires |
                    |  (episodes, neuro|
                    |   routing, dreams)|
                    +--------+---------+
                             |
                             +-----> next batch
```

The orchestrator treats arena tasks the same as any other task. They flow through the same heartbeat pipeline, the same cognitive gating, the same context assembly, the same gate verification, the same episode logging. The only difference is the source (arena instead of plan) and the scoring (arena provides a ground-truth comparison in addition to gate verdicts).

---

## 7. Domain catalog: arenas by tier

### Tier 1: immediate (deployable now)

#### SWE-bench arena

**Task source:** SWE-bench verified subset (500 tasks). Each task provides a GitHub issue, the repository at the commit before the fix, and a test patch that validates the fix.

**Gates:** compile, test (with provided test patch), diff review (ensure the fix is minimal and does not introduce regressions).

**Scoring function:** Binary pass/fail per task (did the test patch pass?). Aggregate: pass rate, difficulty-weighted pass rate (SWE-bench provides difficulty labels).

**Learning signal:** Every task produces a full episode (prompt, actions, gate results, final state). High-PE episodes (tasks that took many iterations, tasks where the agent failed) get priority replay during dreams.

**Cross-arena transfer:** Coding skills transfer to MBPP/HumanEval, SQL generation, IaC generation, and vulnerability detection.

#### Self-hosting arena

**Task source:** Roko's own task backlog. Plans from `.roko/plans/`, PRDs from `.roko/prd/`, and issues from the repository.

**Gates:** compile (full workspace), test (workspace tests), clippy (zero warnings), diff review (manual or LLM-based).

**Scoring function:** Gate pass rate, task completion rate (tasks that reach all gates vs. tasks that stall), developer acceptance rate (does Will merge the PR?).

**Learning signal:** The richest signal source in the system. Every self-hosting task produces episodes that directly improve the agent's ability to work on the codebase it is building. Knowledge entries about Roko's architecture, common patterns, known pitfalls -- all flow into the NeuroStore and benefit future tasks.

**Cross-arena transfer:** Self-hosting skills transfer to SWE-bench (general coding ability) and to documentation generation (understanding of code architecture).

### Tier 2: near-term (3-6 months)

#### MBPP arena

**Task source:** MBPP (974 Python problems). Each task provides a function signature, docstring, and 3 test cases. Loaded via HuggingFace Dataset Viewer API (REST, no Python `datasets` library required).

**Gates:** compile (function must parse as valid Python), test (all 3 provided test cases must pass).

**Scoring function:** pass@1 (single attempt pass rate), pass@5 (best of 5 attempts). Difficulty estimated from solution length and cyclomatic complexity.

**Learning signal:** Fast feedback loops -- median task completion under 30 seconds. Good for rapid iteration on prompt templates, context assembly strategies, and model routing. Each task produces a compact episode with clear pass/fail signal.

**Pacing:** Batch of 50, continuously. At 30s/task median, one full pass takes ~8 hours.

#### HumanEval arena

**Task source:** HumanEval (164 Python problems). Canonical coding benchmark with function signatures, docstrings, and test suites. Loaded via HuggingFace Dataset Viewer API.

**Gates:** compile (valid Python), test (execution correctness against full test suite).

**Scoring function:** pass@1, pass@5. Results directly comparable to published benchmarks from OpenAI, Anthropic, Google.

**Learning signal:** Smaller than MBPP but the tasks are harder (more algorithmic). Complements MBPP for measuring capability across the difficulty spectrum.

**Pacing:** Full suite as a single batch. Run daily as a calibration check.

#### CodeContests arena

**Task source:** Competitive programming problems from Codeforces, AtCoder, and similar platforms. Each task provides a problem statement, input/output format, and test cases. Loaded via HuggingFace Dataset Viewer API.

**Gates:** compile, test (correctness against all test cases), efficiency (must complete within time limit, typically 2 seconds).

**Scoring function:** Correctness rate (binary pass/fail per problem), efficiency score (ratio of actual runtime to time limit). Combined weighted score.

**Learning signal:** Tests algorithmic reasoning under constraints. Transfer to SWE-bench is indirect but real: agents that handle competitive programming develop stronger reasoning about edge cases and boundary conditions.

**Pacing:** Batch of 20, twice per day. Problems are harder and take longer.

#### Chain monitor arena

**Task source:** Historical chain data with labeled events. Each task presents a sequence of blocks and asks the agent to identify specific events (large transfers, pool deployments, governance proposals, oracle deviations). Task source: archived mainnet blocks with human-labeled event annotations.

**Gates:** Precision and recall against labeled events. False positive rate below threshold (configurable, default 5%).

**Scoring function:** F1 score per event type, weighted by event importance. Time-to-detect penalty for late identification.

**Learning signal:** Directly trains the T0 triage pipeline. High-scoring triage rules get promoted; low-scoring rules get pruned. Transfers to ISFR prediction (event detection informs rate prediction) and DeFi strategy (anomaly detection informs risk assessment).

**Pacing:** Continuous. New blocks arrive every 12 seconds (Ethereum) or 400ms (Korai). The agent processes them as they come.

#### Vulnerability detection arena

**Task source:** Known-vulnerable code samples with CVE annotations from the CWE dataset. Each task presents a code file (C, C++, Java, Rust, Solidity) and asks the agent to identify vulnerabilities by CWE category.

**Gates:** Precision (reported vulnerabilities must be real), recall (must catch at least 80% of known vulnerabilities for Critical/High severity).

**Scoring function:** F1 score, weighted by CVE severity (Critical: 4x, High: 2x, Medium: 1x, Low: 0.5x).

**Learning signal:** Trains pattern recognition for security-relevant code patterns. Cross-domain: vulnerability patterns in smart contracts transfer to vulnerability patterns in application code. Transfers to SWE-bench (security awareness improves code quality).

**Pacing:** Batch of 25, daily. New CVEs are ingested weekly from NVD feeds.

#### ISFR prediction arena

**Task source:** Historical ISFR data with known future values. Each task presents a window of ISFR readings (30-day trailing) and asks the agent to predict the next N values (1h, 6h, 24h, 7d) with confidence intervals.

**Gates:** CRPS score below threshold (configurable, default 0.05). Calibration check: predicted 80% intervals must contain the true value 75-85% of the time.

**Scoring function:** Mean CRPS across predictions. Calibration convergence rate (how many predictions before CRPS stabilizes).

**Learning signal:** Directly trains the `ISFROracleExt` prediction model. Calibrated agents earn prediction privileges on the live Korai network. Transfers to yield perp strategy (calibrated rate predictions enable better positioning).

**Pacing:** Every 10 seconds (aligned with oracle update cadence). Continuous.

### Tier 3: medium-term (6-12 months)

#### SQL generation arena

**Task source:** Spider (10,181 questions across 200 databases) and BIRD (12,751 questions with real-world databases). Each task provides a natural language question, a database schema, and expected SQL output. Loaded via HuggingFace Dataset Viewer API with Parquet export for local caching.

**Gates:** SQL execution (must return results within 30 seconds), result comparison (output must match expected rows).

**Scoring function:** Execution accuracy (does the SQL return the right answer?), exact match rate (does the SQL text match?). Execution accuracy is the primary metric -- semantically equivalent but syntactically different SQL still passes.

**Learning signal:** Structured reasoning about schemas, joins, and aggregations. Transfers to research synthesis (structured data reasoning) and document understanding (extraction from structured formats).

**Pacing:** Batch of 50, daily. Full Spider pass takes ~3 days at current throughput.

#### Research synthesis arena

**Task source:** Sets of related papers with known synthesis outcomes. Each task provides 5-10 papers (fetched from Semantic Scholar API or arxiv) and asks the agent to produce a synthesis identifying common findings, contradictions, and open questions.

**Gates:** Citation accuracy (all cited papers exist and say what the agent claims), logical consistency (no internal contradictions in the synthesis), factual grounding (claims traceable to source material).

**Scoring function:** Coverage (fraction of key findings identified), accuracy (fraction of claims supported by the source material), novelty (identified connections not present in any single source). Scoring uses a reference synthesis produced by domain experts.

**Learning signal:** Deep reading comprehension and multi-document reasoning. Transfers to document understanding (reading comprehension) and incident response (multi-source analysis). Citation precision directly measured.

**Pacing:** Batch of 5 syntheses per week. Each synthesis takes 30-60 minutes.

#### Incident response arena

**Task source:** Synthetic incidents generated from historical incident reports with known root causes. Each task presents monitoring data (metrics, logs, alerts) and asks the agent to diagnose the issue and propose remediation. Task source: PagerDuty/Datadog anonymized incident datasets.

**Gates:** Root cause identification accuracy, remediation quality (does the proposed fix address the root cause without introducing regressions?).

**Scoring function:** Mean Time To Root Cause (MTTR), remediation correctness score, false-positive rate on triage.

**Learning signal:** Anomaly detection and causal reasoning under time pressure. Transfers to chain monitoring (anomaly detection patterns) and vulnerability detection (root cause analysis). The time-pressure aspect trains the T0/T1/T2 tier routing: incidents that the agent can triage at T0 are handled faster and cheaper.

**Pacing:** Batch of 10 incidents, twice per week. Each incident takes 5-15 minutes.

#### Yield perp strategy arena

**Task source:** Historical yield rate data with simulated clearing rounds from Korai testnet. Each task provides a market state (ISFR readings, pool utilization, volatility metrics) and asks the agent to formulate a trading strategy with risk parameters.

**Gates:** Strategy must survive simulation without triggering stop-loss. Must pass all five risk layers (position, liquidity, counterparty, correlation, tail). Budget constraint enforced.

**Scoring function:** Sharpe ratio (risk-adjusted return), maximum drawdown, CRPS calibration on yield predictions.

**Learning signal:** Integrates multiple capabilities: ISFR prediction, risk assessment, position sizing, and strategy formulation. Transfers to DeFi strategy (broader market skills). The CRPS component directly ties into oracle mining reputation.

**Pacing:** Event-driven during simulated clearing rounds (~every 10 minutes). Continuous during testnet operation.

### Tier 4: long-term (12+ months)

#### DeFi strategy arena

**Task source:** Simulated DeFi environments with realistic market dynamics (forks of mainnet state at historical timestamps via Foundry/Anvil). Tasks include yield farming, liquidity provision, arbitrage, and risk management scenarios across multiple protocols (Aave, Compound, Uniswap, Curve).

**Gates:** Five-layer risk assessment, transaction simulation (`eth_call` against fork), budget constraint, invariant checks (no negative positions, no exceeded limits).

**Scoring function:** Risk-adjusted return (Sharpe ratio), capital efficiency (return per dollar deployed), protocol diversification (Herfindahl index).

**Learning signal:** The richest signal source for blockchain agents. Strategies that work in simulation can be promoted to paper trading, then to live execution with small allocations. Transfers from all chain-related arenas (monitor, ISFR, yield perp).

**Pacing:** Continuous. Simulated markets run 24/7 with historical replay at 10x speed.

#### IaC generation arena

**Task source:** Infrastructure requirements described in natural language. Tasks cover Terraform (AWS, GCP, Azure), Pulumi (TypeScript), and Kubernetes manifests. Ground truth: working infrastructure definitions validated by `terraform plan` and `kubectl --dry-run`.

**Gates:** `terraform validate` (syntax), `terraform plan` (must succeed without errors), security scan (tfsec/checkov with zero Critical findings), cost estimate (must not exceed budget by >20%).

**Scoring function:** Correctness (does the infrastructure match the requirement?), security (tfsec findings weighted by severity), cost optimization (delta from reference implementation cost).

**Learning signal:** Transfers from SWE-bench (code generation patterns apply to declarative infrastructure) and vulnerability detection (security scanning patterns). The `terraform plan` gate provides deterministic pass/fail signal.

**Pacing:** Batch of 10, weekly. Each task takes 5-20 minutes depending on complexity.

#### Document understanding arena

**Task source:** DocVQA dataset (complex documents: contracts, technical specifications, regulatory filings) with structured extraction targets. Each task provides a document image or PDF and a set of questions about its content.

**Gates:** Extraction accuracy (key fields must match ground truth within fuzzy-match tolerance).

**Scoring function:** Field-level F1 score, end-to-end extraction accuracy, perception accuracy (for chart/diagram interpretation).

**Learning signal:** Multi-modal reasoning. Transfers from research synthesis (reading comprehension) and SQL generation (structured extraction). Requires vision capabilities in the underlying model.

**Pacing:** Batch of 25, weekly. Each extraction takes 1-5 minutes.

#### Multi-modal arena

**Task source:** Tasks that combine code, text, images, and data. Examples: generate documentation from a codebase and its architecture diagrams; analyze a dashboard screenshot and produce improvement recommendations; convert a wireframe image into working HTML/CSS.

**Gates:** Domain-specific per sub-task (code gates for code output, grammar gates for text output, perception accuracy for image analysis).

**Scoring function:** Composite score across modalities, weighted by the primary modality of each task.

**Learning signal:** The broadest transfer potential. Multi-modal tasks require integrating skills from every other arena. Perception accuracy on charts transfers to document understanding. Code generation from visual specs transfers to IaC generation.

**Pacing:** Batch of 10, biweekly. Tasks vary in duration from 5 minutes to 1 hour.

### Cross-arena knowledge transfer

Knowledge gained in one arena benefits performance in others. The mechanism is HDC similarity in the InsightStore -- structurally similar insights are discoverable regardless of which arena produced them.

| Source arena | Target arena | Transfer mechanism |
|-------------|-------------|-------------------|
| SWE-bench | Self-hosting | General coding patterns, debugging heuristics, test-writing strategies |
| SWE-bench | IaC generation | Code generation patterns apply to declarative infrastructure definitions |
| SWE-bench | Vulnerability detection | Understanding code structure helps identify vulnerable patterns |
| Self-hosting | SWE-bench | Project-specific patterns (retry logic, error handling) that generalize |
| MBPP / HumanEval | SWE-bench | Algorithmic reasoning, function-level problem solving |
| CodeContests | SWE-bench | Edge-case reasoning, boundary condition awareness |
| CodeContests | MBPP / HumanEval | Algorithmic skill transfers directly |
| Chain monitor | ISFR prediction | Event detection skills inform rate prediction timing |
| Chain monitor | DeFi strategy | Anomaly detection informs real-time risk assessment |
| Chain monitor | Incident response | Event triage patterns transfer to alert triage |
| ISFR prediction | Yield perp strategy | Calibrated rate predictions enable better positioning |
| ISFR prediction | DeFi strategy | Rate trajectory awareness informs yield farming timing |
| Vulnerability detection | SWE-bench | Security awareness improves code quality and review |
| Vulnerability detection | IaC generation | Security scanning patterns transfer to infrastructure audit |
| Research synthesis | Document understanding | Multi-document reading comprehension transfers across formats |
| Research synthesis | Incident response | Multi-source analysis skills apply to multi-signal diagnosis |
| Incident response | Chain monitor | Anomaly detection and causal reasoning transfer across domains |
| Incident response | Vulnerability detection | Root cause analysis informs vulnerability triage |
| SQL generation | Research synthesis | Structured data reasoning informs structured knowledge extraction |
| SQL generation | Document understanding | Schema comprehension transfers to document structure analysis |
| Yield perp strategy | DeFi strategy | Strategy formulation and risk parameter selection |
| DeFi strategy | Chain monitor | Market dynamics understanding improves event significance scoring |
| IaC generation | Incident response | Infrastructure comprehension aids root cause diagnosis |
| Document understanding | Research synthesis | Extraction accuracy improves citation verification |
| Multi-modal | Document understanding | Perception accuracy on charts, diagrams, screenshots |
| Multi-modal | IaC generation | Visual spec interpretation enables infrastructure from wireframes |

The transfer is not explicit. Nobody codes "if the agent learned X in arena A, apply it in arena B." Insights from arena A are encoded as HDC vectors and stored in the NeuroStore. When the agent works in arena B, its RETRIEVE step queries the NeuroStore with the current observation's HDC vector. If arena A produced a structurally similar insight, it surfaces in the results. The VCG auction determines whether it wins context budget.

This is stigmergic transfer: knowledge deposited in a shared substrate, discovered through proximity, not directed messaging.

---

## 8. Work markets on Korai

### The economic model

Korai's work markets create economic incentives for intelligence production. Every piece of knowledge an agent produces has value to other agents. Work markets let that value be priced, traded, and settled.

Six mining surfaces provide distinct ways for agents to earn KORAI tokens. Each surface rewards a different kind of cognitive work. Together they form a complete market for intelligence production.

### Six mining surfaces

#### 1. Oracle mining

**What the agent does:** Monitor external rate sources, compute ISFR deviations from the dual-median aggregated benchmark, flag divergences that exceed threshold, submit signed attestations.

**Reward mechanism:** KORAI tokens proportional to the agent's attestation quality (CRPS score) and the divergence magnitude. Agents that accurately identify large deviations before other agents earn a bonus.

**Gate requirements:** Attestations must pass the `invariant_check` gate (internal consistency), the `source_verification` gate (the claimed source data matches what the oracle precompile has), and the `staleness_check` gate (the attestation uses recent data, not stale).

**Economic alignment:** The network needs accurate ISFR data. Oracle miners produce it. Higher-quality data earns more. The market selects for calibration.

#### 2. Verifier mining

**What the agent does:** Validate InsightStore entries submitted by other agents. Challenge entries that are provably wrong. Confirm entries that pass verification.

**Reward mechanism:** Successful challenges (proving an entry wrong) earn a reward proportional to the entry's current confidence score. Confirmations earn a smaller reward. False challenges (claiming an entry is wrong when it is right) slash the verifier's bond.

**Gate requirements:** Challenges must include a `proof_of_falsehood` (a counter-example, contradicting source, or logical demonstration). Confirmations must include a `verification_path` (the steps taken to verify the entry).

**Economic alignment:** The InsightStore is only valuable if its entries are accurate. Verifier mining creates an adversarial process that weeds out low-quality entries and rewards curation.

#### 3. Inference mining

**What the agent does:** Produce analyses, risk assessments, recommendations, or other structured cognitive outputs in response to work market requests.

**Reward mechanism:** Payment is set by the work market auction (see lifecycle below). The agent bids on jobs and gets paid on verified completion.

**Gate requirements:** Domain-specific. A risk assessment must pass the `consistency_check` gate. A code analysis must pass `compile` and `test`. A research report must pass `citation_check` and `factual_consistency`.

**Economic alignment:** Agents compete on quality and price. The market routes work to the most cost-effective agent that can meet quality requirements.

#### 4. Repair mining

**What the agent does:** Identify degraded knowledge in the InsightStore (entries with decaying confidence, entries contradicted by newer evidence, entries with stale source references). Produce corrected or updated versions.

**Reward mechanism:** Reward proportional to the confidence delta between the degraded entry and the repaired entry, multiplied by the entry's query frequency (how often other agents retrieve it).

**Gate requirements:** The repaired entry must pass all gates that the original entry would be subject to, plus a `novelty_check` gate (the repair must be substantively different from the original, not a trivial re-submission).

**Economic alignment:** Knowledge decays. Repair mining counteracts decay by incentivizing maintenance. The highest rewards go to repairing high-traffic entries -- knowledge that many agents rely on.

#### 5. Mechanism mining

**What the agent does:** Propose parameter optimizations for the ISFR oracle, the clearing mechanism, or the reputation system. Test improvements in simulation.

**Reward mechanism:** If a proposed parameter change is adopted by governance and the system metrics improve, the proposing agent earns a reward proportional to the improvement magnitude.

**Gate requirements:** Proposals must include a `simulation_report` (demonstrating improvement in a sandbox) and a `regression_analysis` (demonstrating no degradation on other metrics). Proposals that worsen system health slash the proposer's bond.

**Economic alignment:** The system's parameters need continuous optimization as market conditions change. Mechanism mining outsources this optimization to agents with the strongest understanding of system dynamics.

#### 6. Index mining

**What the agent does:** Refine ISFR computation by identifying new rate sources, improving outlier detection algorithms, or proposing methodology updates.

**Reward mechanism:** Similar to mechanism mining. Rewards proportional to improvement in ISFR accuracy (measured by reduction in variance between ISFR and actual DeFi yields).

**Gate requirements:** Must include backtesting against historical data. Must not introduce new attack surfaces (e.g., an oracle that can be manipulated by a single source).

**Economic alignment:** ISFR credibility depends on methodology quality. Index mining channels agent intelligence toward improving the benchmark that everything else settles against.

### Work market lifecycle

Every job that flows through Korai's work markets follows a six-phase lifecycle:

**Phase 1: Job posting.** A requester (human, agent, or automated system) posts a job to the work market. The posting includes: task description, required domain profile, minimum reputation tier, maximum cost, deadline, and verification criteria.

```rust
/// A job posted to the Korai work market.
pub struct WorkMarketJob {
    /// Unique job identifier.
    pub id: JobId,
    /// Task description.
    pub description: String,
    /// Required domain profile for the executing agent.
    pub required_domain: DomainProfile,
    /// Minimum epistemic reputation tier.
    pub min_reputation_tier: ReputationTier,
    /// Maximum payment in KORAI.
    pub max_payment: U256,
    /// Deadline (block number).
    pub deadline_block: u64,
    /// Verification criteria.
    pub verification: VerificationCriteria,
    /// Requester's agent passport.
    pub requester: AgentPassport,
}
```

**Phase 2: Sealed bidding.** Eligible agents submit sealed bids in a TEE (Trusted Execution Environment). Each bid contains the agent's price, estimated completion time, and a commitment to quality (staking KORAI as a bond). Bids are sealed until the bidding window closes, preventing front-running and strategic underbidding.

**Phase 3: Assignment.** The work market assigns the job using reputation-weighted selection. The assignment algorithm considers:
- Bid price (lower is better, up to the requester's maximum)
- Reputation tier (higher is better)
- Domain-specific track record (historical pass rate in this arena)
- Current load (agents with fewer active jobs get priority)

The selection function weights these factors:

```
assignment_score = 0.3 * price_score + 0.3 * reputation_score
                + 0.25 * track_record_score + 0.15 * load_score
```

**Phase 4: Execution.** The assigned agent executes the job through the full cognitive loop. The heartbeat pipeline handles perception, gating, and action. Gates validate intermediate and final outputs. The episode is logged.

**Phase 5: Verification.** Two verification paths:

- **Structural verification:** The submitted output passes all gates specified in the job's verification criteria. This is automated and deterministic.
- **Quality verification:** For jobs requiring quality assessment beyond gate passing, a panel of verifier-miners evaluates the output. The panel is selected from agents with high reputation in the relevant domain. Majority agreement determines the verdict.

**Phase 6: Settlement.** If verification passes, KORAI is released from escrow to the executing agent. If verification fails, the agent's bond is slashed and the job is re-posted. Settlement is on-chain, atomic, and final (single-slot finality from Kauri BFT).

### Knowledge Futures

Knowledge Futures are predictive markets for intelligence production. They let agents monetize future cognitive work by selling commitments.

**How it works:**

1. A research agent publishes a commitment: "I will produce a DEX routing analysis covering 10 protocols, with execution data from the last 30 days, within 24 hours."

2. The commitment is posted on-chain with a bond (staked KORAI).

3. Purchasers evaluate the commitment based on the agent's track record and reputation. They fund the commitment by paying the projected inference cost upfront plus a margin.

4. The agent executes the research, producing the artifact through the research cycle (source detection, processing, synthesis, consolidation).

5. The artifact is submitted on-chain. Verifier miners evaluate it against the commitment criteria.

6. **Delivery verified:** Payment releases to the agent. The artifact enters the InsightStore as a high-confidence entry.

7. **Delivery missed or substandard:** Bond is slashed. The agent's reputation takes a hit. Purchasers are refunded.

```rust
/// A Knowledge Futures commitment posted on Korai.
pub struct KnowledgeFuture {
    /// Unique commitment identifier.
    pub id: CommitmentId,
    /// Agent making the commitment.
    pub agent: AgentPassport,
    /// Description of the promised deliverable.
    pub deliverable: String,
    /// Structured criteria for verification.
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    /// Deadline (block number).
    pub deadline_block: u64,
    /// Bond amount in KORAI (slashed on failure).
    pub bond: U256,
    /// Total funding committed by purchasers.
    pub funding: U256,
    /// Projected inference cost in USD.
    pub projected_cost_usd: f64,
    /// Current status.
    pub status: FutureStatus,
}

pub enum FutureStatus {
    /// Commitment posted, accepting funding.
    Open,
    /// Fully funded, agent is executing.
    Funded,
    /// Agent submitted deliverable, under verification.
    Submitted,
    /// Verified and settled.
    Settled,
    /// Deadline passed without delivery, bond slashed.
    Defaulted,
}

pub struct AcceptanceCriterion {
    /// What to check.
    pub criterion: String,
    /// Gate to use for automated verification.
    pub gate: Option<String>,
    /// Whether this criterion requires human/agent panel review.
    pub requires_panel: bool,
}
```

Knowledge Futures create a price signal for intelligence. If many agents purchase "DEX routing analysis" futures, the market is signaling that this knowledge is valuable. More agents will produce it. Supply meets demand through price discovery.

### Knowledge Futures lifecycle

The full lifecycle of a Knowledge Future, from publication to settlement:

**Phase 1: Publication.** A research agent commits on-chain: "I will produce a DEX routing efficiency analysis covering 10 protocols, with execution data from the last 30 days, within 24 hours." The commitment includes structured acceptance criteria, a bond (staked KORAI), and the agent's passport (on-chain identity with reputation history).

```rust
// Agent publishes a commitment
let future = KnowledgeFuture {
    deliverable: "DEX routing efficiency analysis: \
        10 protocols, 30-day execution data, \
        optimal route recommendations per pair".into(),
    acceptance_criteria: vec![
        AcceptanceCriterion {
            criterion: "Covers >= 10 DEX protocols".into(),
            gate: Some("coverage_check".into()),
            requires_panel: false,
        },
        AcceptanceCriterion {
            criterion: "Execution data from last 30 days".into(),
            gate: Some("data_freshness".into()),
            requires_panel: false,
        },
        AcceptanceCriterion {
            criterion: "Route recommendations are gas-optimal".into(),
            gate: None,
            requires_panel: true,
        },
    ],
    deadline_block: current_block + 7200, // ~24 hours
    bond: parse_ether("100")?,            // 100 KORAI bond
    // ...
};
chain_client.publish_future(future).await?;
```

**Phase 2: Purchase.** Other agents evaluate the commitment. They check the research agent's track record: past Knowledge Futures delivery rate, average quality scores, domain reputation in DeFi research. If satisfied, they purchase by sending payment to the escrow contract. The payment covers the researcher's projected inference cost plus a margin.

Multiple purchasers can fund the same future. Each purchaser receives a proportional claim on the deliverable. Early purchasers pay less (incentivizing early funding). Late purchasers pay more (reflecting reduced risk as the researcher demonstrates progress).

**Phase 3: Execution.** The research agent runs its normal research cycle: source detection, processing, synthesis, consolidation. The Knowledge Future is injected as a high-priority task into the agent's event fabric. The heartbeat pipeline handles the execution. Progress is observable on-chain via optional intermediate commits.

**Phase 4: Delivery and verification.** The agent submits the deliverable on-chain before the deadline block. The gate pipeline runs automated acceptance criteria. For criteria requiring panel review, a panel of 3 verifier-miners is selected from agents with high DeFi research reputation. Majority agreement determines the verdict.

**Phase 5: Settlement.** Three outcomes:

| Outcome | Agent receives | Purchasers receive | Reputation effect |
|---------|---------------|-------------------|-------------------|
| Verified delivery | Bond returned + payment | Access to deliverable + InsightStore entries | +reputation |
| Partial delivery (some criteria pass) | Bond partially slashed + partial payment | Partial access + refund for unmet criteria | Neutral |
| Missed deadline / failed verification | Bond fully slashed | Full refund from bond | -reputation |

### Knowledge Futures guardrails

Five constraints prevent abuse and ensure market health:

| Guardrail | Limit | Rationale |
|-----------|-------|-----------|
| **Maximum horizon** | 7 days | Longer horizons increase default risk. Research that takes more than 7 days should be split into milestones. |
| **Bond scaling** | Bond >= 10% of projected cost | Ensures the agent has skin in the game proportional to the commitment size. |
| **Maximum active futures** | 5 per agent | Prevents overcommitment. An agent with 5 active futures cannot publish a 6th until one settles. |
| **Minimum reputation** | ReputationTier::Contributor or higher | New agents cannot publish futures until they have earned basic reputation through other mining surfaces. |
| **Deliverable size limit** | 100 KB on-chain, unlimited off-chain with hash commitment | Prevents storage spam. Large deliverables are stored off-chain with an on-chain hash for verification. |

### Market effect

Knowledge Futures create a feedback loop between demand and supply:

1. Purchasers signal demand by funding futures. High funding indicates high demand.
2. Agents observe funding patterns and specialize in high-demand areas.
3. Specialization increases quality and decreases cost in those areas.
4. Lower costs and higher quality attract more purchasers.
5. The market allocates agent labor to where it is most valuable.

Premium Knowledge Futures -- those with high funding relative to cost -- signal areas where agent labor produces the most value. Agents that notice these signals and specialize in high-premium areas earn more. This is market-driven specialization: no central authority assigns roles, the price mechanism handles allocation.

The market also reveals information asymmetries. If no agent can produce a particular deliverable (all attempts default), the market signals that the current agent population lacks a capability. This becomes a training signal: arena tasks in that area get higher priority in the curriculum scheduler.

### x402 micropayments

The self-funding loop:

```
earn KORAI -> convert to USDC -> pay inference via x402 -> produce output -> earn more KORAI
```

The x402 protocol (HTTP 402 Payment Required, implemented in `roko-chain/src/x402.rs`) enables pay-per-request micropayments through state channels:

1. The agent opens a state channel with a USDC deposit.
2. Each LLM API call includes an x402 payment header.
3. The API provider verifies the payment and serves the request.
4. Payments are batched and settled on-chain periodically.

This makes agents economically autonomous. An agent that earns more KORAI from work markets than it spends on inference is profitable. It can operate indefinitely without human funding.

The x402 integration in Roko:

```rust
/// x402 payment integration in the inference gateway.
pub struct X402PaymentProvider {
    /// State channel with the inference provider.
    channel: StateChannel,
    /// Current balance available for payments.
    balance: U256,
    /// Cost tracker for budget enforcement.
    cost_tracker: CostTracker,
}

impl X402PaymentProvider {
    /// Attach an x402 payment header to an LLM API request.
    ///
    /// The amount is determined by the model and estimated token count.
    /// If the balance is insufficient, the provider returns an error
    /// and the agent must earn more before continuing.
    pub async fn attach_payment(
        &mut self,
        request: &mut LlmRequest,
        estimated_tokens: u64,
    ) -> Result<()> {
        let amount = self.estimate_cost(request.model(), estimated_tokens);
        if self.balance < amount {
            return Err(InsufficientFunds {
                available: self.balance,
                required: amount,
            });
        }
        let receipt = self.channel.sign_payment(amount).await?;
        request.headers.insert("X-402-Payment", receipt.to_header());
        self.balance -= amount;
        self.cost_tracker.record(amount, request.model());
        Ok(())
    }
}
```

An agent running low on funds can degrade to cheaper models (T0/T1 emphasis), reduce tick frequency, or accept lower-paying work market jobs to rebuild its balance. The homeostasis mechanism (section 4.5 of PRD-02) handles this automatically: when economic vitality drops below the operating range, the agent degrades to cheaper operation.

---

## 9. HuggingFace integration

### The five API layers

HuggingFace provides five distinct API surfaces. Each maps to a specific capability in the roko stack. Together they close the loop from benchmark loading through model training to deployment.

#### Layer 1: Inference Providers

HuggingFace hosts inference endpoints that expose an OpenAI-compatible API across 18+ model families: Llama 3.x, Mistral, Qwen, Command-R, Gemma, Phi, DeepSeek, and others. The API accepts the standard `POST /v1/chat/completions` format with model routing via the `model` parameter.

Integration point: `roko-agent/src/dispatcher/mod.rs`. The existing OpenAI-compatible backend works with HuggingFace Inference Providers by changing the base URL and API key. No new code needed for basic integration.

Three routing policies for the CascadeRouter:

| Policy | Behavior | When to use |
|--------|----------|-------------|
| `:fastest` | Route to the provider with lowest current latency | Time-sensitive tasks (chain monitoring, incident response) |
| `:cheapest` | Route to the provider with lowest cost per token | Batch processing (MBPP, HumanEval, bulk distillation) |
| `:preferred` | Route to a specific provider (e.g., `together` or `fireworks`) | When a provider has demonstrated better quality for a domain |

The CascadeRouter already supports multiple backends. Adding HuggingFace Inference Providers means adding entries to the model registry, not changing the routing logic:

```toml
# roko.toml
[models.llama-3-70b]
backend = "openai-compat"
base_url = "https://router.huggingface.co/together/v1"
model_id = "meta-llama/Llama-3.3-70B-Instruct"
routing_policy = ":cheapest"

[models.qwen-2-72b]
backend = "openai-compat"
base_url = "https://router.huggingface.co/fireworks-ai/inference/v1"
model_id = "Qwen/Qwen2.5-72B-Instruct"
routing_policy = ":fastest"
```

#### Layer 2: Hub API

The HuggingFace Hub API provides model discovery, metadata search, and download endpoints. Integration point: the CascadeRouter's model registry.

The router currently maintains a static list of model configurations. With Hub API integration, it discovers new models automatically:

1. Periodic poll (configurable, default daily) queries `GET /api/models` with filters: `pipeline_tag=text-generation`, `sort=downloads`, `limit=50`.
2. New models matching the filter are added to the CascadeRouter as unexplored bandit arms with a default prior.
3. The bandit's exploration budget (Thompson sampling) allocates a small fraction of traffic to new arms.
4. Models that underperform after N trials (configurable, default 50) are pruned.

This means the CascadeRouter self-updates as new models are released. No manual configuration required.

#### Layer 3: Dataset Viewer

The Dataset Viewer API provides REST-based access to any dataset on the Hub. For benchmark loading, this replaces the Python `datasets` library entirely.

Three endpoints handle all benchmark needs:

```
GET /rows?dataset={name}&config={config}&split={split}&offset={n}&length={m}
GET /parquet?dataset={name}&config={config}&split={split}
GET /info?dataset={name}
```

The `/parquet` endpoint returns signed URLs to Parquet files. Download these once, cache locally in `.roko/cache/datasets/`, and read via the `parquet` crate. This gives the arena framework pure-Rust dataset access with no Python dependency.

Benchmark datasets and their Hub identifiers:

| Arena | Dataset | Config |
|-------|---------|--------|
| SWE-bench | `princeton-nlp/SWE-bench_Verified` | default |
| MBPP | `google-research-datasets/mbpp` | sanitized |
| HumanEval | `openai/openai_humaneval` | default |
| CodeContests | `deepmind/code_contests` | default |
| Spider | `spider` | default |
| BIRD | `DAMO-NLP-SG/bird` | default |
| DocVQA | `lmms-lab/DocVQA` | default |

#### Layer 4: Inference Endpoints

HuggingFace Inference Endpoints provide dedicated GPU compute with auto-scaling and scale-to-zero. Use cases:

- **Benchmark batch processing.** Spin up a dedicated endpoint for a SWE-bench run, process 500 tasks, scale to zero. No idle cost between runs.
- **Fine-tuned model serving.** After AutoTrain produces a fine-tuned model (layer 5), deploy it on an Inference Endpoint for evaluation. If the evaluation passes, promote it to the CascadeRouter.
- **Distillation at scale.** The knowledge distiller (`roko-neuro/src/distiller.rs`) can target a dedicated endpoint running a small model at high throughput.

API integration:

```rust
/// HuggingFace Inference Endpoints client.
pub struct HfEndpointClient {
    /// API token for authentication.
    api_token: String,
    /// Namespace (user or org).
    namespace: String,
}

impl HfEndpointClient {
    /// Create a new endpoint with auto-scaling configuration.
    pub async fn create_endpoint(
        &self,
        name: &str,
        model_id: &str,
        instance_type: &str,
        min_replicas: u32,
        max_replicas: u32,
    ) -> Result<EndpointInfo> { /* ... */ }

    /// Scale an endpoint to zero (pause billing).
    pub async fn scale_to_zero(&self, name: &str) -> Result<()> { /* ... */ }

    /// Get endpoint status and metrics.
    pub async fn status(&self, name: &str) -> Result<EndpointStatus> { /* ... */ }
}
```

#### Layer 5: AutoTrain

AutoTrain provides fine-tuning as a service. Upload a dataset, specify a base model and training method, and receive a fine-tuned model pushed to the Hub.

Supported training methods:

| Method | What it does | When to use |
|--------|-------------|-------------|
| SFT (Supervised Fine-Tuning) | Train on input-output pairs | Standard task adaptation |
| ORPO (Odds Ratio Preference Optimization) | Train on preference pairs without a reference model | When you have comparative data (this response was better than that one) |
| DPO (Direct Preference Optimization) | Train on preference pairs with a reference model | When you need closer alignment to a base model |
| KTO (Kahneman-Tversky Optimization) | Train on binary feedback (good/bad) | When you have gate pass/fail data but no pairwise comparisons |

Gate verdicts from the episode log map directly to training data:

| Episode data | Training method | Training signal |
|-------------|----------------|-----------------|
| (prompt, successful_output) | SFT | Imitate successful episodes |
| (prompt, successful_output, failed_output) | DPO/ORPO | Prefer the version that passed gates |
| (prompt, output, gate_pass: bool) | KTO | Binary quality signal from gate pipeline |

### The exponential fine-tuning loop (Stream C)

This is the self-reinforcing cycle that converts agent experience into better models:

**Step 1: Agents run tasks.** Normal operation through the orchestration loop. Every task produces an episode with prompt, actions, gate results, and quality scores.

**Step 2: Filter successful episodes.** Episodes with gate pass rate > 80% across all required gates are candidates. Episodes with pass rate < 50% are negative examples. The filter is configurable per deployment.

**Step 3: Extract training data.** Convert filtered episodes to the format required by the selected training method:
- SFT: `{"messages": [{"role": "system", ...}, {"role": "user", ...}, {"role": "assistant", ...}]}`
- DPO/ORPO: `{"prompt": ..., "chosen": ..., "rejected": ...}` (from episodes where the agent retried and the later attempt succeeded)
- KTO: `{"prompt": ..., "completion": ..., "label": true/false}` (from gate verdicts)

**Step 4: Push dataset to HuggingFace Hub.** The extracted training data is uploaded as a dataset to the Hub. Named by agent ID, domain, and timestamp: `roko-agent-{id}/{domain}-{date}`.

**Step 5: Trigger AutoTrain job.** API call to AutoTrain with the base model, training method, dataset, and hyperparameters:

```rust
pub struct AutoTrainConfig {
    pub base_model: String,        // e.g., "meta-llama/Llama-3.3-8B-Instruct"
    pub training_method: TrainingMethod,
    pub dataset_id: String,        // Hub dataset from step 4
    pub num_epochs: u32,           // Default: 3
    pub learning_rate: f64,        // Default: 2e-5
    pub lora_rank: u32,            // Default: 16
    pub max_seq_length: usize,     // Default: 4096
}
```

**Step 6: Fine-tuned model pushed to Hub.** AutoTrain trains the model and pushes it to the Hub under the agent's namespace. Model ID: `roko-agent-{id}/{domain}-{date}-ft`.

**Step 7: CascadeRouter discovers new model.** The Hub API poll (layer 2) picks up the new model. It enters the CascadeRouter as an unexplored bandit arm.

**Step 8: Bandit exploration assigns traffic.** Thompson sampling allocates a fraction of tasks to the new model. If it outperforms existing models on the relevant task clusters (measured by gate pass rate), the bandit increases its allocation. If it underperforms, allocation shrinks.

**Step 9: Repeat.** Better models produce better episodes. Better episodes produce better training data. Better training data produces better models. The loop compounds.

### Network effects from fine-tuning

The fine-tuning loop creates network effects when multiple roko instances participate:

- **Instance A** fine-tunes on Django tasks (SWE-bench, Python web framework).
- **Instance B** fine-tunes on Flask tasks (SWE-bench, different Python framework).
- Both push their fine-tuned models to the Hub.
- Both discover each other's models via the Hub API poll.
- Instance A's CascadeRouter tries Instance B's Flask-tuned model on its Django tasks. If the Flask model's general Python skills transfer, it gets allocated traffic.
- The reverse happens for Instance B.

The result: each instance benefits from every other instance's training. The Hub acts as a model marketplace where fine-tuning investments compound across the network. No direct coordination required -- the CascadeRouter's bandit exploration handles discovery and allocation automatically.

---

## 10. SWE-bench native bench crate

### Design principle: no separate harness

SWE-bench instances are not special. They are tasks routed through the same `plan run` code path as self-hosting tasks and PRD-generated plans. The arena framework (section 6) provides the task source and scoring function. The orchestrator handles dispatch, gating, learning, and persistence.

This means:
- Learning loops fire automatically. Every SWE-bench task produces an episode. Episodes feed into distillation, clustering, routing optimization, and dream consolidation. The agent gets better at SWE-bench the same way it gets better at everything else.
- No separate infrastructure. `roko bench swe` is syntactic sugar over `roko plan run` with the SWE-bench arena as the task source.
- Heuristics transfer. A debugging heuristic discovered during self-hosting (e.g., "when tests fail in a module with recent imports changes, check circular dependencies first") benefits SWE-bench tasks with similar structure, discovered through HDC fingerprint similarity.

### Dataset loading

The SWE-bench dataset is loaded via the HuggingFace Dataset Viewer API (layer 3, section 9):

```rust
pub struct SweBenchLoader {
    /// Cache directory for downloaded repos.
    cache_dir: PathBuf,
    /// HuggingFace API client.
    hf_client: HfDatasetClient,
    /// Dataset split to use.
    split: String, // "verified" (500 tasks) or "test" (2,294 tasks)
}

impl SweBenchLoader {
    /// Load task metadata from the Hub.
    pub async fn load_tasks(&self, offset: usize, limit: usize) -> Vec<SweBenchTask> {
        let rows = self.hf_client.get_rows(
            "princeton-nlp/SWE-bench_Verified",
            "default",
            &self.split,
            offset,
            limit,
        ).await?;
        rows.into_iter().map(SweBenchTask::from_row).collect()
    }

    /// Prepare a repository at the correct commit.
    pub async fn prepare_repo(&self, task: &SweBenchTask) -> Result<PathBuf> {
        let repo_dir = self.cache_dir.join(&task.repo);
        if !repo_dir.exists() {
            git_clone(&task.repo_url, &repo_dir).await?;
        }
        git_checkout(&repo_dir, &task.base_commit).await?;
        Ok(repo_dir)
    }
}
```

No Python `datasets` library. No Conda environment. Pure Rust with REST API calls and git operations.

### Task-to-plan mapping

Each SWE-bench instance maps to a roko task:

```rust
impl SweBenchArena {
    fn task_to_envelope(&self, task: &SweBenchTask) -> TaskEnvelope {
        TaskEnvelope {
            id: task.instance_id.clone(),
            arena: "swe-bench".into(),
            description: format!(
                "Fix this GitHub issue in the {} repository.\n\n\
                 Issue: {}\n\n\
                 The repository is checked out at commit {}.\n\
                 Apply your fix and ensure the test patch passes.",
                task.repo, task.problem_statement, task.base_commit
            ),
            output_format: OutputFormat::GitPatch,
            difficulty: task.difficulty_estimate(),
            ground_truth: Some(task.patch.clone()),
            token_budget: 32_000,
            tags: vec![
                task.repo.clone(),
                task.primary_language.clone(),
            ],
            attachments: vec![
                Attachment::GitRepo(task.repo_path.clone()),
                Attachment::TestPatch(task.test_patch.clone()),
            ],
        }
    }
}
```

### Two-tier scoring

**Fast proxy (every task).** After the agent produces a patch, apply it with `git apply --check`. If the patch applies cleanly, run the test patch. If the test passes, the task scores 1.0. If the patch does not apply or the test fails, the task scores 0.0. This runs in under 30 seconds per task.

**Official harness (periodic).** The SWE-bench official Python harness provides ground-truth evaluation with Docker-based isolation. Run this weekly on the full batch for calibration against published leaderboard results. The fast proxy should agree with the official harness 95%+ of the time; discrepancies indicate edge cases in the proxy that need fixing.

### The perpetual grinder

```bash
roko bench swe --repeat 0 --batch-size 50 --shuffle
```

This command runs forever:
1. Sample 50 tasks from SWE-bench (shuffled to prevent ordering bias).
2. Route each task through the orchestrator (dispatch, gate, persist, learn).
3. Score the batch.
4. Log results to `.roko/learn/arena-scores/swe-bench/`.
5. If any batch-level metric improved (pass rate, median latency, cost efficiency), emit a pheromone event.
6. Sleep for `batch_cooldown` seconds (default: 60).
7. Sample next 50 tasks. Repeat.

The agent learns continuously. After 10 full passes through SWE-bench (5,000 task-episodes on the verified set), the InsightStore contains validated heuristics about Python debugging, test interpretation, git operations, and repository navigation. These transfer to self-hosting tasks and to other arenas.

### Implementation estimate

| Component | Lines |
|-----------|-------|
| Dataset loading (HF API + local cache) | ~400 |
| Repository preparation (git clone, checkout, worktree) | ~200 |
| Task-to-plan mapping (SWE-bench -> TaskEnvelope) | ~150 |
| Two-tier scoring (fast proxy + official harness bridge) | ~100 |
| CLI integration (`roko bench swe` subcommand) | ~200 |
| Arena trait implementation (sample, gates_for, score, enrich_prompt) | ~300 |
| **Total** | **~1,350** |

---

## 11. Custom domain creation

### Declarative: roko.toml

Users can define custom domain profiles in `roko.toml` without writing Rust code. The TOML profile maps to the same `FullDomainProfile` struct used by predefined profiles.

```toml
[domains.security-audit]
label = "security-audit"

# Heartbeat timing
[domains.security-audit.clock.gamma]
calm = 300
normal = 120
volatile = 60
crisis = 30

[domains.security-audit.clock.theta]
calm = 1800
normal = 600
volatile = 300
crisis = 120

[domains.security-audit.clock.delta]
episode_threshold = 20
idle_timeout_secs = 600
sleep_pressure_threshold = 40.0

# Extensions
[domains.security-audit.extensions]
required = ["heartbeat", "context", "daimon", "learning"]
optional = ["vuln-scanner", "dependency-monitor"]

# Event subscriptions
[[domains.security-audit.wakeup_events]]
event_type = "FileChange"
severity_threshold = 0.6

[[domains.security-audit.wakeup_events]]
event_type = "CVEPublished"
# No severity threshold -- always wake up.

# Context weights
[domains.security-audit.context_weights]
knowledge_entries = 0.30
code_intelligence = 0.25
task_description = 0.20
playbook_rules = 0.15
iteration_memory = 0.10

# Gates
[[domains.security-audit.gates]]
name = "static-analysis"
required = true
timeout_secs = 300

[[domains.security-audit.gates]]
name = "cve-check"
required = true
timeout_secs = 60

# Infrastructure
[domains.security-audit.infrastructure]
git_worktree = true
file_watcher = true
```

The runtime loads custom profiles from `roko.toml` at startup. Custom profiles have the same status as predefined profiles. They participate in the same resolution logic, the same provisioning pipeline, and the same learning system.

### Programmatic: Rust extensions

Custom extensions require Rust code. Implement the `Extension` trait, compile as part of the workspace (or as a dynamic library), and register with the extension chain.

```rust
/// A custom vulnerability scanner extension.
///
/// Monitors the codebase for known vulnerability patterns using a
/// library of static analysis rules. Runs at each gamma tick during
/// OBSERVE and contributes findings to the CognitiveWorkspace.
pub struct VulnScannerExt {
    /// Compiled vulnerability detection rules.
    rules: Vec<VulnRule>,
    /// Files changed since the last scan.
    pending_files: Vec<PathBuf>,
    /// Known vulnerabilities found so far.
    findings: Vec<VulnFinding>,
}

#[async_trait]
impl Extension for VulnScannerExt {
    fn name(&self) -> &str { "vuln-scanner" }

    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    fn depends_on(&self) -> &[&str] { &["heartbeat"] }

    async fn on_activate(&self, ctx: &mut ActivateContext) -> Result<()> {
        // Load vulnerability rules from the rules directory.
        self.rules = load_vuln_rules(&ctx.config_dir().join("vuln-rules/"))?;
        Ok(())
    }

    async fn on_observe(
        &self,
        cortical: &CorticalState,
        _cancel: &CancelToken,
    ) -> Result<Vec<Observation>> {
        // Check pending files against vulnerability rules.
        let mut observations = Vec::new();
        for file in &self.pending_files {
            let content = tokio::fs::read_to_string(file).await?;
            for rule in &self.rules {
                if let Some(finding) = rule.check(&content, file) {
                    observations.push(Observation::VulnerabilityFound(finding.clone()));
                    self.findings.push(finding);
                }
            }
        }
        self.pending_files.clear();
        Ok(observations)
    }

    async fn on_reflect(
        &self,
        record: &DecisionCycleRecord,
        cortical: &CorticalState,
        workspace: &mut CognitiveWorkspace,
    ) -> Result<()> {
        // Update vulnerability statistics in CorticalState.
        cortical.set_signal(
            "vuln_finding_count",
            self.findings.len() as f32,
        );
        Ok(())
    }
}
```

Custom extensions can be packaged and distributed. The MCP protocol provides a natural distribution mechanism: a vulnerability scanning MCP server bundles the `VulnScannerExt` with its rule library and connects to the agent at runtime.

### Custom arenas

Custom arenas follow the same pattern. Implement the `Arena` trait and register:

```rust
pub struct CompanyCodeReviewArena {
    /// Repository with known code review decisions.
    repo: PathBuf,
    /// Historical reviews with known outcomes.
    reviews: Vec<CodeReview>,
}

#[async_trait]
impl Arena for CompanyCodeReviewArena {
    fn name(&self) -> &str { "company-code-review" }

    async fn sample(&self, batch_size: usize) -> Vec<TaskEnvelope> {
        // Sample historical code changes and ask the agent to review them.
        self.reviews
            .choose_multiple(&mut rand::thread_rng(), batch_size)
            .map(|review| TaskEnvelope {
                id: review.pr_id.clone(),
                arena: self.name().to_string(),
                description: format!(
                    "Review this pull request and identify issues:\n\n{}",
                    review.diff
                ),
                output_format: OutputFormat::StructuredReview,
                difficulty: review.estimated_difficulty,
                ground_truth: Some(serde_json::to_string(&review.known_issues).unwrap()),
                token_budget: 8_000,
                tags: review.labels.clone(),
                attachments: vec![],
            })
            .collect()
    }

    fn gates_for(&self, _task: &TaskEnvelope) -> Vec<Box<dyn Gate>> {
        vec![
            Box::new(FormatGate::new("structured_review")),
            Box::new(ConsistencyGate::new()),
        ]
    }

    fn score(&self, results: &[TaskResult]) -> ArenaScore {
        // Compare agent's identified issues against known issues.
        // Score on precision (agent issues that match known issues)
        // and recall (known issues the agent identified).
        // ...
    }

    fn enrich_prompt(&self, task: &TaskEnvelope) -> Vec<ContextSection> {
        vec![ContextSection {
            category: ContextCategory::PlaybookRules,
            content: include_str!("review_guidelines.md").to_string(),
            priority: 0.8,
        }]
    }
}
```

---

## 12. Generalized benchmark index framework

### ISFR as the first instance

The Internet Secured Funding Rate (see PRD-07) is a benchmark index: a multi-source, dual-median aggregated, validator-computed rate that serves as a reference point for financial instruments. ISFR works because it combines multiple independent sources, uses robust aggregation (dual-median rejects outliers), and lives at the consensus layer (validators produce it as part of block construction, making it trustless and deterministic).

The same pattern generalizes. Any measurable quantity that can be aggregated from multiple independent sources, computed deterministically, and published on-chain can become a benchmark index. ISFR is the first. It will not be the last.

### Other possible benchmark indices

#### Agent Performance Index (API)

**What it measures:** Aggregate task pass rates across arenas, weighted by task difficulty and arena importance.

**Sources:** Every arena produces pass rates per agent per evaluation cycle. The ABI aggregates across all arenas.

**Computation:**

```
API = sum(arena_weight[i] * difficulty_weighted_pass_rate[i]) / sum(arena_weight[i])
```

Where `arena_weight[i]` is determined by the arena's economic activity (how many work market jobs reference it) and `difficulty_weighted_pass_rate[i]` weights each task by its difficulty score.

**Use cases:** Agent ranking for work market assignment. Reputation tier thresholds. Insurance pricing for delegated agent authority.

#### Knowledge Quality Index (KQI)

**What it measures:** InsightStore entry accuracy, weighted by query frequency (how often other agents retrieve each entry).

**Sources:** Verifier mining outcomes (confirmed vs. challenged entries), query logs (which entries are retrieved most), and decay rates (how fast entries lose confidence without confirmation).

**Computation:**

```
KQI = sum(entry_accuracy[i] * query_frequency[i]) / sum(query_frequency[i])
```

**Use cases:** InsightStore health monitoring. Reward calibration for repair mining. Quality threshold for Knowledge Futures deliverables.

#### Security Vulnerability Index (SVI)

**What it measures:** Aggregate vulnerability detection rates across monitored codebases.

**Sources:** Security arena results, CVE databases, bug bounty programs.

**Computation:** Weighted detection rate per vulnerability severity class.

**Use cases:** Automated risk assessment for DeFi protocols. Insurance pricing. Audit prioritization.

#### Research Impact Index (RII)

**What it measures:** Citation and usage rates of agent-produced knowledge, normalized by field and recency.

**Sources:** InsightStore query frequencies, Knowledge Futures delivery rates, cross-agent knowledge propagation (how often one agent's output appears in another agent's context).

**Computation:** PageRank-style computation over the citation graph, with time decay.

**Use cases:** Research agent reputation. Knowledge Futures pricing. Academic-style impact metrics for agent-produced work.

### The framework

All benchmark indices share the same infrastructure:

```rust
/// A generalized benchmark index computed from multiple sources.
pub trait BenchmarkIndex: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Collect raw observations from all sources.
    async fn collect_sources(&self) -> Vec<IndexObservation>;

    /// Aggregate observations using a robust estimator.
    ///
    /// Default: dual-median (median of medians from independent source
    /// groups). Override for indices that need different aggregation.
    fn aggregate(&self, observations: &[IndexObservation]) -> f64 {
        dual_median(observations)
    }

    /// Validate the aggregated value against sanity bounds.
    fn validate(&self, value: f64) -> Result<(), IndexValidationError>;

    /// Publish the computed index value.
    ///
    /// For on-chain indices, this calls the oracle precompile.
    /// For off-chain indices, this writes to the local data store.
    async fn publish(&self, value: f64, block: u64) -> Result<()>;
}

/// A single observation from one source.
pub struct IndexObservation {
    pub source: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub confidence: f32,
}

/// Dual-median aggregation (robust to outliers).
///
/// 1. Group observations by source.
/// 2. Compute median within each source group.
/// 3. Compute median of the group medians.
fn dual_median(observations: &[IndexObservation]) -> f64 {
    let mut by_source: HashMap<&str, Vec<f64>> = HashMap::new();
    for obs in observations {
        by_source.entry(&obs.source).or_default().push(obs.value);
    }
    let group_medians: Vec<f64> = by_source
        .values()
        .map(|vals| {
            let mut sorted = vals.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            sorted[sorted.len() / 2]
        })
        .collect();
    let mut sorted = group_medians;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted[sorted.len() / 2]
}
```

Each index follows the same lifecycle: collect sources at a configurable cadence, aggregate with a robust estimator, validate against bounds, publish. The ISFR runs this at 10-second cadence through the oracle precompile. The Agent Performance Index might run daily. The Knowledge Quality Index might run hourly. The framework accommodates all cadences.

---

## 13. Network effects and scaling

### More arenas, more signal

Each arena produces a distinct training signal. An agent running SWE-bench produces episodes about debugging, code comprehension, and test writing. An agent running chain monitoring produces episodes about event detection, pattern recognition, and real-time triage. These signals are different in content but share structural patterns encoded as HDC vectors.

When an agent runs in multiple arenas simultaneously, cross-arena knowledge transfer happens automatically:

- A debugging heuristic from SWE-bench ("when a test fails on a boundary condition, check the surrounding conditions") encodes as BIND(boundary_failure, check_neighbors).
- A triage rule from chain monitoring ("when a metric deviates at the boundary of its historical range, investigate neighboring metrics") encodes as BIND(boundary_deviation, check_neighbors).
- These share structural similarity. The NeuroStore surfaces one when the agent is working in the context of the other.

More arenas means a richer training signal. The routing table learns from more diverse situations. The somatic markers cover more of the strategy space. The knowledge base contains more transferable patterns.

At N arenas, the cross-arena transfer potential scales as O(N^2). Each arena can potentially transfer knowledge to every other arena. Not all transfers are useful, but the discovery mechanism (HDC similarity) automatically filters for structural relevance.

### More agents per arena

Multiple agents working in the same arena produce more diverse solutions to the same problems. Diversity matters for collective intelligence (Surowiecki 2004, Woolley et al. 2010). If every agent in an arena uses the same strategy, the collective learns nothing beyond what one agent would learn. If agents use different strategies -- different model routing preferences, different context allocation weights, different prompt templates -- the collective discovers which strategies work best for which tasks.

The learning system captures this naturally. Each agent's episodes are logged. The prompt experiment system (A/B testing in `ExperimentStore`) tracks which prompt variants produce better outcomes. The CascadeRouter learns from all agents' model routing data. The collective converges on optimal strategies faster than any individual agent.

With realistic inter-agent correlation (rho = 0.3 from shared InsightStore knowledge), the effective independent sample size for N agents is approximately:

```
N_eff = N / (1 + (N - 1) * rho)
```

At N = 1,000 agents with rho = 0.3, N_eff is approximately 3.3. But the raw volume still matters. 1,000 agents each producing 100 episodes per day means 100,000 scored episodes per day feeding into the learning system. That volume pushes rho toward zero through diversity mechanisms: agents that discover the same things as everyone else earn less reputation, incentivizing exploration of under-served knowledge areas.

### Concurrent arena execution

A single agent can run in multiple arenas simultaneously. The heartbeat pipeline handles this naturally. Arena tasks are injected as stimuli via the event fabric. The agent's gamma tick processes whichever tasks are active. Between arena tasks, the agent runs its normal observation loop.

A coding agent might run:
- **SWE-bench** (continuous, batch of 10 tasks)
- **Self-hosting** (continuous, tasks from the plan backlog)
- **MBPP/HumanEval** (periodic calibration, batch of 50 tasks every 24 hours)

All three arenas write to the same NeuroStore. All three produce episodes that feed the same learning system. The agent does not know or care which arena generated a knowledge entry -- it retrieves by HDC similarity, not by source label.

A blockchain agent might run:
- **Chain monitoring** (continuous, live block stream)
- **ISFR prediction** (every 10 seconds, aligned with oracle cadence)
- **Yield perp strategy** (event-driven, when clearing rounds produce opportunities)

The agent's gamma tick is fast enough (5 seconds in crisis) to service all three. Most ticks serve chain monitoring at T0. ISFR prediction triggers T1 analysis at 10-second intervals. Strategy execution triggers T2 when conditions warrant. The total daily cost depends on market conditions, but the T0/T1/T2 distribution keeps it manageable.

### The scaling flywheel

The domain and arena framework amplifies the five reinforcing loops from PRD-01:

**Loop 1 amplified: more arenas, more knowledge, better context.** Each arena produces domain-specific knowledge. Cross-arena transfer via HDC similarity means knowledge from arena A enriches the context in arena B. More arenas produce more diverse knowledge, which enriches more contexts, which produces better outcomes, which produces more knowledge.

**Loop 2 amplified: more clearing participants, more insights.** Blockchain agents trained across multiple arenas bring richer models to cooperative clearing. Richer models produce better predictions. Better predictions produce better ClearingInsights. Better insights enrich all agents.

**Loop 3 amplified: more accurate ISFR from better agents.** Oracle mining agents trained on ISFR prediction arenas produce more accurate attestations. More accurate ISFR attracts more volume. More volume produces more data. More data improves ISFR accuracy.

**Loop 4 amplified: reputation across arenas.** An agent with strong performance across multiple arenas earns higher aggregate reputation. Higher reputation earns priority in work markets. Priority access produces more episodes. More episodes improve performance.

**Loop 5 amplified: cross-domain transfer at scale.** This is the meta-loop. More domains create more opportunities for structural similarity to surface novel connections. A coding insight that transfers to security auditing. A research synthesis technique that transfers to incident response. A chain monitoring pattern that transfers to infrastructure monitoring. Each connection is a new source of compound returns.

The system improves faster with more participants, more domains, more arenas, and more knowledge. The improvements are not linear. They compound.

---

## Summary: what this enables

The domain profile system means one runtime serves every agent type. The arena framework means every domain has a measurement instrument. The work markets mean every piece of intelligence has a price. The generalized benchmark framework means any measurable quantity can become a reference rate.

Together: agents that specialize in domains, measure their performance against ground truth, earn revenue from their intelligence, and improve through compound learning -- all running the same nine-step heartbeat pipeline, differing only in the extensions loaded and the profiles configured.

The first agent runs SWE-bench at 30% pass rate. The thousandth agent, enriched by the cumulative InsightStore, runs it at 70%. The same model. The same runtime. A better harness, built by the network.
