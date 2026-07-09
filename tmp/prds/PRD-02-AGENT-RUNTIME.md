# PRD-02: Agent runtime -- from LLM wrappers to persistent processes

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Crates affected:** `roko-runtime` (rewrite), `roko-ext-core` (new), `roko-ext-code` (new), `roko-ext-chain` (new), `roko-ext-research` (new)

---

## Table of contents

1. [Introduction and rationale](#1-introduction-and-rationale)
2. [The heartbeat pipeline](#2-the-heartbeat-pipeline)
3. [Three timescales](#3-three-timescales)
4. [Six concurrent cognitive mechanisms](#4-six-concurrent-cognitive-mechanisms)
5. [The extension system](#5-the-extension-system)
6. [Extension chain composition and firing](#6-extension-chain-composition-and-firing)
7. [Type-state lifecycle](#7-type-state-lifecycle)
8. [CorticalState -- lock-free shared perception](#8-corticalstate----lock-free-shared-perception)
9. [Event fabric](#9-event-fabric)
10. [Process model and supervision](#10-process-model-and-supervision)
11. [Backwards compatibility](#11-backwards-compatibility)
12. [Crate layout](#12-crate-layout)
13. [Unified narrative: one agent tick, end to end](#13-unified-narrative-one-agent-tick-end-to-end)
14. [Performance targets](#14-performance-targets)
15. [Inference Gateway](#15-inference-gateway)
16. [Updated Extension trait: EventPayload additions](#16-updated-extension-trait-eventpayload-additions)

---

## 1. Introduction and rationale

### What Roko is

Roko is an 18-crate Rust toolkit (~177K LOC) for building agents that build themselves. It sits inside a larger system called Nunchi that includes Korai, an intelligence blockchain where agents publish knowledge, earn reputation, and trade cognitive services. The core loop today: read a PRD, generate an implementation plan, dispatch tasks to Claude-backed agents, validate results through a multi-rung gate pipeline, persist outcomes as signals, and learn from the feedback. That loop works end-to-end.

### What agents are today

Agents are ephemeral LLM wrappers. The orchestrator (`orchestrate.rs`, 19K lines, 217 methods, 137+ fields) does everything. It:

- Spawns a child process (`spawn_process()`)
- Feeds it a system prompt and task description
- Waits for output
- Kills the process

The agent process has no memory between tasks. No heartbeat. No ability to observe its environment between prompts. No state that survives the end of a tool call. Every bit of intelligence -- tier routing, affect modeling, prediction tracking, knowledge retrieval, pheromone signaling, dream scheduling, stuck detection, budget enforcement -- lives in the orchestrator.

This means the orchestrator is responsible for modeling the cognitive state of every agent it manages. For a coding agent, that works tolerably well. For a blockchain agent that needs to subscribe to block events continuously, it falls apart. For a research agent that should accumulate knowledge across sessions, the orchestrator has to reconstruct context from cold storage on every dispatch.

The agent spawn pattern today looks like this:

```rust
// Current: spawn-execute-die
let spec = SpawnAgentSpec {
    program: "claude".into(),
    args: vec!["--print", "--model", model, "-p", &prompt],
    working_dir: Some(worktree_path),
    env: agent_env,
    ..Default::default()
};
let handle = spawn_agent_with_layer(&spec, &safety)?;
let output = handle.wait_with_timeout(timeout).await?;
// Agent is dead. All state was in the orchestrator.
```

### What agents need to become

Agents need to be persistent processes with their own:

- **Heartbeat loop** -- a recurring tick that observes the environment, decides whether to think, and acts if needed
- **Cognitive subsystem access** -- direct connections to knowledge stores, affect engines, prediction trackers
- **Event subscriptions** -- the ability to react to chain blocks, file changes, price feeds, and inter-agent signals without the orchestrator mediating
- **Durable state** -- lifecycle phases enforced at compile time, with state that persists across ticks and survives restarts
- **Extension hooks** -- a plugin system so that blockchain agents, coding agents, and research agents share the same heartbeat but run different perception and action logic

### Why this matters

Three concrete cases make the argument.

**Blockchain agents need continuous perception.** Korai agents subscribe to new blocks, resolve InsightStore queries, manage staking positions, and react to price movements. Under the current model, the orchestrator polls for chain state, constructs a prompt, spawns an agent, and reads the output. Latency from block arrival to agent reaction: seconds to minutes. With a persistent heartbeat at 5-second gamma ticks, a `ChainSubscriberExt` extension reads the block directly. Most ticks hit T0 -- pattern-match the block, update internal state, no LLM call, $0 cost, sub-millisecond. When prediction error spikes (a large price move, an unexpected transaction), the agent escalates to T1 or T2 automatically.

**Research agents need accumulated knowledge.** A research agent that enhances PRDs needs to remember what it learned in previous sessions. Today, the orchestrator reconstructs context by querying the NeuroStore and injecting results into the system prompt. The agent itself retains nothing. With a persistent process, the agent's `NeuroExt` extension maintains a hot cache of relevant knowledge entries, builds associations across sessions, and promotes validated insights through tier progression without orchestrator involvement.

**Coding agents can be cheap with tiered gating.** The current model pays for an LLM call on every task dispatch, even when the agent's next action is deterministic (run `cargo test`, check if a file exists, apply a known patch). With T0 gating, roughly 80% of a coding agent's ticks cost $0. The agent runs deterministic probes -- did the last compile succeed? are there unstaged changes? -- and only escalates to an LLM when the situation is genuinely novel.

### Academic grounding

This design draws on established work:

- **Actor model** (Hewitt et al. 1973, Agha 1986): agents as concurrent, autonomous entities with private state and asynchronous message passing. Each agent has a mailbox. Processing a message can create new agents, send messages, and modify local state.

- **Process calculi** (Milner, "Communicating and Mobile Systems: the Pi-Calculus", 1999): formal reasoning about concurrent processes that communicate over named channels. The extension chain's event fabric is a typed channel system.

- **Erlang/OTP supervision** (Armstrong 2003): hierarchical process supervision with restart strategies (one-for-one, rest-for-one, one-for-all). The process model section adapts these to Roko's needs.

- **CoALA** (Sumers et al. 2023, "Cognitive Architectures for Language Agents"): a framework for language agents organized around working memory, long-term memory, and decision-making. Roko's CognitiveWorkspace maps to working memory; the NeuroStore maps to long-term memory; the heartbeat gate maps to the decision procedure.

---

## 2. The heartbeat pipeline

Every agent runs a heartbeat loop. Each tick executes a nine-step pipeline. Extensions hook into each step to provide domain-specific behavior.

### The nine steps

```
OBSERVE -> RETRIEVE -> ANALYZE -> GATE -> SIMULATE -> VALIDATE -> EXECUTE -> VERIFY -> REFLECT
```

**Step 1: OBSERVE.** Extensions read their environment. A `ChainSubscriberExt` reads the latest block. A `FileWatcherExt` reads changed files. A `GateExt` reads recent test results. Observations land in the `CognitiveWorkspace` -- a per-tick scratchpad assembled via VCG attention auction.

**Step 2: RETRIEVE.** Extensions query knowledge stores. A `NeuroExt` queries the local NeuroStore for entries relevant to current observations. A `ChainKnowledgeExt` queries Korai's InsightStore via the HDC precompile for cross-agent knowledge. A `PlaybookExt` retrieves matching learned rules. Results are bid into the workspace.

**Step 3: ANALYZE.** Compute prediction error (PE). PE quantifies how much observed state differs from what the agent predicted. For a coding agent, PE might measure test failures against expected passes. For a chain agent, PE might measure price deviation from a forecast. Each extension contributes its domain-specific PE component. The pipeline aggregates them into a scalar.

**Step 4: GATE.** Decide the cognitive tier for this tick based on PE:

| Tier | Frequency | What happens | Cost | Latency |
|------|-----------|--------------|------|---------|
| T0 | ~80% of ticks | Deterministic pattern match. No LLM. Update internal counters, run probes, check thresholds. | $0 | <1ms |
| T1 | ~15% of ticks | Cheap model (Haiku-class). Minimal context window. Quick reasoning about moderate novelty. | ~$0.001 | ~200ms |
| T2 | ~5% of ticks | Full reasoning model (Opus-class). Complete CognitiveWorkspace. Deep analysis of genuinely novel situations. | $0.01-$0.10 | 2-30s |

The gate uses exponentially weighted moving average (EWMA) thresholds that adapt over time. Thresholds persist to `.roko/learn/gate-thresholds.json`.

**Step 5: SIMULATE.** (T1/T2 only.) Sandbox candidate actions before committing. For a coding agent: dry-run a patch in a worktree, check if it compiles. For a chain agent: simulate a transaction against a local fork. Extensions provide domain-specific simulation backends.

**Step 6: VALIDATE.** (T1/T2 only.) Check safety constraints. Query somatic markers for situation-specific warnings. Verify delegation caveats (budget limits, role permissions, tool restrictions). The `SafetyLayer` from `roko-agent` runs here.

**Step 7: EXECUTE.** (T1/T2 only.) Take action. Tool calls, file writes, git commits, blockchain transactions. Routed through the `ToolDispatcher` with the full safety hook chain.

**Step 8: VERIFY.** (T1/T2 only.) Confirm the outcome matches expectations. Did the compile succeed? Did the transaction land? Did the test pass? Verification feeds back into PE for the next tick.

**Step 9: REFLECT.** Build a `DecisionCycleRecord`. Update knowledge stores (promote observations to entries, update somatic markers). Fire learning hooks (efficiency events, episode logging, skill extraction). Update epistemic reputation if connected to Korai.

### Implementation sketch

```rust
/// The heartbeat pipeline executes one tick of the agent's cognitive loop.
///
/// Each step is a hook point where extensions contribute domain-specific
/// behavior. The pipeline itself is domain-agnostic -- it orchestrates
/// the flow and manages the CognitiveWorkspace lifecycle.
pub struct HeartbeatPipeline {
    /// Registered extensions, sorted by layer and dependency order.
    chain: ExtensionChain,
    /// Lock-free shared perception surface.
    cortical: Arc<CorticalState>,
    /// Per-tick scratchpad assembled via VCG attention auction.
    workspace: CognitiveWorkspace,
    /// Tier gate with adaptive EWMA thresholds.
    gate: TierGate,
    /// Monotonically increasing tick counter.
    tick_count: u64,
    /// Safety layer for action validation.
    safety: Arc<SafetyLayer>,
    /// Event fabric for broadcasting outcomes.
    event_tx: BusSender<RokoEvent>,
}

impl HeartbeatPipeline {
    /// Execute one tick of the cognitive loop.
    ///
    /// Returns a `DecisionCycleRecord` summarizing what happened this tick.
    /// The record is used for learning, logging, and inter-agent communication.
    pub async fn execute_tick(
        &mut self,
        clock: &HeartbeatClock,
        cancel: &CancelToken,
    ) -> Result<DecisionCycleRecord> {
        self.tick_count += 1;
        let tick_start = Instant::now();

        // ── Step 1: OBSERVE ──────────────────────────────────────────
        // Each extension reads its environment and contributes observations
        // to the workspace. ChainSubscriberExt reads latest block.
        // FileWatcherExt reads changed files. GateExt reads test results.
        let observations = self.chain
            .run_observe(&self.cortical, cancel)
            .await?;
        self.workspace.ingest_observations(observations);

        // ── Step 2: RETRIEVE ─────────────────────────────────────────
        // Extensions query knowledge stores. Results are bid into the
        // workspace via the VCG attention auction. Higher-salience entries
        // win context budget; losers are dropped or summarized.
        let retrievals = self.chain
            .run_retrieve(&self.workspace, &self.cortical, cancel)
            .await?;
        self.workspace.run_attention_auction(retrievals);

        // ── Step 3: ANALYZE ──────────────────────────────────────────
        // Compute aggregate prediction error from per-extension components.
        // PE drives tier selection in the next step.
        let pe_components = self.chain
            .run_analyze(&self.workspace, &self.cortical)
            .await?;
        let prediction_error = pe_components.aggregate();
        self.cortical.set_prediction_error(prediction_error);

        // ── Step 4: GATE ─────────────────────────────────────────────
        // Select cognitive tier based on PE and adaptive thresholds.
        // T0 means no LLM call. T1/T2 enable the remaining steps.
        let tier = self.gate.select_tier(
            prediction_error,
            self.cortical.behavioral_state(),
            self.cortical.regime(),
        );
        self.cortical.set_cognitive_tier(tier);

        // T0: update counters, run probes, return early.
        if tier == InferenceTier::T0 {
            let record = DecisionCycleRecord::t0(
                self.tick_count,
                tick_start.elapsed(),
                prediction_error,
            );
            self.chain.run_reflect_t0(&record, &self.cortical).await?;
            self.event_tx.emit(RokoEvent::HeartbeatTick(
                HeartbeatTick::from_record(&record),
            ));
            return Ok(record);
        }

        // ── Step 5: SIMULATE ─────────────────────────────────────────
        // (T1/T2 only) Sandbox candidate actions before committing.
        let simulations = self.chain
            .run_simulate(&self.workspace, tier, cancel)
            .await?;

        // ── Step 6: VALIDATE ─────────────────────────────────────────
        // (T1/T2 only) Safety checks, somatic markers, delegation caveats.
        let validated_actions = self.chain
            .run_validate(
                &simulations,
                &self.safety,
                &self.cortical,
            )
            .await?;

        // ── Step 7: EXECUTE ──────────────────────────────────────────
        // (T1/T2 only) Perform validated actions through the tool dispatcher.
        let outcomes = self.chain
            .run_execute(&validated_actions, tier, cancel)
            .await?;

        // ── Step 8: VERIFY ───────────────────────────────────────────
        // (T1/T2 only) Confirm outcomes match expectations. Feed
        // verification results back into PE for the next tick.
        let verifications = self.chain
            .run_verify(&outcomes, &self.workspace, cancel)
            .await?;

        // ── Step 9: REFLECT ──────────────────────────────────────────
        // Build the decision cycle record. Update knowledge stores.
        // Fire learning hooks. Log the episode.
        let record = DecisionCycleRecord::new(
            self.tick_count,
            tick_start.elapsed(),
            prediction_error,
            tier,
            &outcomes,
            &verifications,
        );
        self.chain
            .run_reflect(&record, &self.cortical, &mut self.workspace)
            .await?;

        self.event_tx.emit(RokoEvent::HeartbeatTick(
            HeartbeatTick::from_record(&record),
        ));

        Ok(record)
    }
}
```

---

## 3. Three timescales

The heartbeat runs at three concurrent timescales inspired by neural oscillation bands.

| Scale | Default period | Range | Purpose | Typical tier |
|-------|---------------|-------|---------|-------------|
| **Gamma** | 5-120s (domain-dependent) | 1s-300s | Perception and triage. Read environment, compute PE, decide tier, execute if T1/T2. | T0 (80%), T1/T2 (20%) |
| **Theta** | 30-300s | 30s-600s | Full decision cycle. Summarize recent gamma work, update affect, check calibration drift, re-evaluate plan progress, trigger interventions if stuck. | T1/T2 |
| **Delta** | ~50 theta ticks | configurable | Consolidation. Replay high-value episodes (Mattar-Daw priority), generate counterfactuals, promote validated knowledge, prune stale entries. | Offline batch |

### Why multiple timescales

**Biological precedent.** Mammalian brains run nested oscillations: gamma (30-100Hz) for sensory binding, theta (4-8Hz) for working memory and navigation, delta (0.5-4Hz) for deep sleep consolidation. The functional separation is not arbitrary -- each band serves a distinct computational purpose. Fast perception, medium-speed planning, slow integration. Roko adapts this hierarchy to agent cognition, compressed from milliseconds to seconds.

**Economic efficiency.** Without tiered timing, an agent either polls too fast (wasting LLM calls on unchanged state) or too slow (missing events). Gamma ticks are cheap because most hit T0. Theta ticks are expensive but infrequent. Delta ticks are batch operations that run during idle periods. A blockchain agent running 5-second gamma ticks processes ~17,000 ticks per day. At 80% T0, that's ~13,600 ticks at $0, ~2,600 at T1 ($2.60), and ~800 at T2 ($8-80). Total daily cost: $10-83, depending on market volatility. Without tiered gating, 17,000 LLM calls per day at T1 would cost $17.

**Cognitive load management.** Gamma handles "what is happening." Theta handles "am I making progress." Delta handles "what have I learned." Mixing these concerns in a single loop creates the problem that `orchestrate.rs` has today: a 19K-line file doing perception, planning, execution, and reflection in interleaved steps with no clear boundary between them.

### Timescale configuration

Each timescale adapts to environmental regime:

```rust
/// Adaptive clock configuration for a single timescale.
pub struct TimescaleConfig {
    /// Base interval in the calm regime.
    pub calm_interval: Duration,
    /// Interval during normal operation.
    pub normal_interval: Duration,
    /// Interval during volatile conditions.
    pub volatile_interval: Duration,
    /// Interval during crisis.
    pub crisis_interval: Duration,
}

impl TimescaleConfig {
    /// Resolve the interval for the current regime.
    pub fn interval_for(&self, regime: Regime) -> Duration {
        match regime {
            Regime::Calm => self.calm_interval,
            Regime::Normal => self.normal_interval,
            Regime::Volatile => self.volatile_interval,
            Regime::Crisis => self.crisis_interval,
        }
    }
}
```

Example: a chain agent's gamma clock runs at 120s in calm markets, 30s in normal markets, 10s in volatile markets, and 5s during a crisis. The regime is determined by the `CorticalState` and updated by extensions that monitor volatility indicators.

### Theta reflective loop

Theta runs a five-phase reflection cycle (implemented in `roko-runtime/src/theta_consumer.rs`):

1. **Summarize** recent gamma work -- aggregate decision cycle records since the last theta tick
2. **Update affect** -- appraise outcomes through the ALMA (Adaptive Layered Model of Affect) model, shift the PAD vector
3. **Check calibration** -- detect prediction accuracy drift; trigger recalibration if accuracy drops below threshold
4. **Re-evaluate plan** -- compare progress against the DAG schedule; detect stuck or thrashing states
5. **Intervene** -- emit `CognitiveSignal` events (pause, escalate, replan) if meta-cognition detects problems

### Delta consolidation loop

Delta runs a three-phase dream cycle (implemented in `roko-runtime/src/delta_consumer.rs`, dream logic in `roko-dreams`):

1. **NREM replay** -- Mattar-Daw priority replay of high-utility episodes. Episodes that produced large prediction errors or changed knowledge state get replayed first.
2. **REM imagination** -- counterfactual generation. "What if I had used a different approach?" The imagination engine generates alternative action sequences and evaluates them against the recorded outcome.
3. **Integration** -- promote validated insights through the NeuroStore tier progression (ephemeral -> working -> consolidated -> core). Prune stale entries. Update somatic markers.

---

## 4. Six concurrent cognitive mechanisms

These mechanisms run alongside the heartbeat as background state machines. They read and write `CorticalState` atomics, influencing the heartbeat's tier selection, context allocation, and action policies.

### 4.1 Attention salience

A binary min-heap ranked by salience score:

```
salience = novelty * 0.4 + relevance * 0.35 + urgency * 0.25
```

Every observation and retrieval result gets a salience score. The top-K entries win context budget in the VCG attention auction. Salience decays exponentially between ticks:

```rust
/// Attention entry with exponential decay.
pub struct AttentionEntry {
    /// Unique identifier for deduplication.
    pub id: ContentHash,
    /// Current salience score in [0.0, 1.0].
    pub salience: f32,
    /// Tick when this entry was last refreshed.
    pub last_seen_tick: u64,
    /// Source subsystem that produced this entry.
    pub source: SubsystemId,
}

impl AttentionEntry {
    /// Decay salience based on ticks elapsed since last refresh.
    /// Half-life is ~10 gamma ticks.
    pub fn decayed_salience(&self, current_tick: u64) -> f32 {
        let elapsed = (current_tick - self.last_seen_tick) as f32;
        self.salience * (-0.069 * elapsed).exp() // ln(2)/10 ~ 0.069
    }
}
```

The VCG auction (already implemented in `roko-runtime/src/heartbeat_attention.rs`) allocates context tokens to the highest-bidding entries. Each subsystem has a carryover budget -- tokens it did not use in the previous tick are partially carried forward, preventing starvation of lower-priority subsystems.

### 4.2 Habituation mask

Repeated identical observations lose novelty. The habituation mask tracks observation frequency using Blake3 hashes:

```rust
/// Frequency tracker for habituation (novelty attenuation).
pub struct HabituationMask {
    /// Blake3 hash -> (count, last_tick).
    seen: HashMap<[u8; 32], (u32, u64)>,
    /// Maximum entries before LRU eviction.
    capacity: usize,
}

impl HabituationMask {
    /// Record an observation and return the attenuation factor in [0.0, 1.0].
    /// First occurrence: 1.0 (full novelty).
    /// After N occurrences: 1.0 / (1.0 + ln(N)).
    pub fn attenuate(&mut self, hash: [u8; 32], tick: u64) -> f32 {
        let entry = self.seen.entry(hash).or_insert((0, tick));
        entry.0 += 1;
        entry.1 = tick;
        1.0 / (1.0 + (entry.0 as f32).ln())
    }
}
```

This prevents a blockchain agent from spending T2 reasoning on every routine block. After seeing 100 blocks with normal gas and no relevant transactions, the novelty factor for "normal block" drops to ~0.18, keeping those ticks at T0.

### 4.3 Sleep pressure

Sleep pressure accumulates each gamma tick without consolidation. It tracks a simple counter that grows linearly and triggers delta entry at a configurable threshold:

```rust
/// Sleep pressure accumulator.
///
/// Pressure grows linearly with each gamma tick that does not trigger
/// consolidation. When pressure exceeds the threshold, the delta
/// consumer forces a dream cycle.
pub struct SleepPressure {
    /// Current pressure in arbitrary units.
    pub pressure: f32,
    /// Increment per gamma tick.
    pub rate: f32,
    /// Threshold that triggers a delta cycle.
    pub threshold: f32,
}

impl SleepPressure {
    /// Accumulate one tick of pressure.
    pub fn tick(&mut self) {
        self.pressure += self.rate;
    }

    /// Check whether pressure exceeds the threshold.
    pub fn should_dream(&self) -> bool {
        self.pressure >= self.threshold
    }

    /// Reset pressure after a completed delta cycle.
    pub fn reset(&mut self) {
        self.pressure = 0.0;
    }
}
```

Default threshold: 50.0. Default rate: 1.0 per gamma tick. At 5-second gamma intervals, the agent enters a dream cycle roughly every 250 seconds without other triggers. During volatile regimes, the rate decreases (the agent needs to stay alert) and the threshold increases.

### 4.4 Event-driven wakeup

Certain events bypass the normal heartbeat cadence and trigger an immediate gamma tick:

- **Chain events:** large price movements (>2 sigma), liquidation events, governance proposals
- **File system:** changes to watched files during an active coding task
- **Pheromone signals:** another agent broadcasts an urgent discovery or warning
- **Gate verdicts:** a gate failure on a task this agent owns

The wakeup system uses the event fabric's filtered subscription. When a high-priority event arrives, it cancels the current sleep interval and forces an immediate tick:

```rust
/// Wakeup condition that interrupted the normal heartbeat cadence.
pub enum WakeupCondition {
    /// A chain event exceeded the volatility threshold.
    ChainVolatility { asset: String, sigma: f32 },
    /// A watched file changed.
    FileChanged { path: PathBuf },
    /// Another agent broadcast an urgent pheromone.
    Pheromone { source: AgentId, kind: PheromoneKind },
    /// A gate failed on this agent's task.
    GateFailed { task_id: String, gate: String },
    /// Explicit operator trigger.
    Operator { reason: String },
}
```

### 4.5 Homeostasis

The agent maintains three vital signals within operating ranges:

| Signal | Range | Below-range response | Above-range response |
|--------|-------|---------------------|---------------------|
| Economic vitality | [0.2, 0.9] | Degrade to cheaper models, reduce tick rate | No action (healthy surplus) |
| Epistemic confidence | [0.3, 0.8] | Increase exploration, query knowledge stores more aggressively | Reduce retrieval, trust cached state |
| Arousal | [-0.5, 0.5] | Increase gamma frequency, seek novel stimuli | Reduce frequency, favor consolidation |

Homeostasis reads from CorticalState and writes corrective signals back:

```rust
/// Homeostatic regulator that keeps vital signals in operating range.
pub struct Homeostasis {
    pub economic_range: (f32, f32),
    pub epistemic_range: (f32, f32),
    pub arousal_range: (f32, f32),
}

impl Homeostasis {
    /// Compute corrective actions based on current cortical state.
    pub fn regulate(&self, cortical: &CorticalState) -> Vec<HomeostaticAction> {
        let mut actions = Vec::new();
        let pad = cortical.pad();
        let arousal = pad.arousal as f32;

        if arousal < self.arousal_range.0 {
            actions.push(HomeostaticAction::IncreaseGammaFrequency);
        } else if arousal > self.arousal_range.1 {
            actions.push(HomeostaticAction::DecreaseGammaFrequency);
        }

        let resource_health = cortical.resource_health();
        if resource_health < self.economic_range.0 {
            actions.push(HomeostaticAction::DegradeToT0Emphasis);
        }

        let knowledge_health = cortical.knowledge_health();
        if knowledge_health < self.epistemic_range.0 {
            actions.push(HomeostaticAction::IncreaseRetrieval);
        } else if knowledge_health > self.epistemic_range.1 {
            actions.push(HomeostaticAction::ReduceRetrieval);
        }

        actions
    }
}
```

### 4.6 Compensation and rollback

When an action produces an unexpected outcome (verification fails in step 8), the agent attempts compensation before reflecting. Compensation is domain-specific:

- **Coding agent:** `git checkout -- <file>` to revert a bad patch, then reflect on why the approach failed
- **Chain agent:** submit a reversal transaction if possible, or flag the position for manual review
- **Research agent:** retract a published finding that was later contradicted

The compensation mechanism is an extension hook (`on_compensation`) that fires between VERIFY and REFLECT when verification fails.

---

## 5. The extension system

Extensions are the primary mechanism for specializing agents. A `ChainSubscriberExt` turns a generic agent into a blockchain agent. A `GitExt` turns it into a coding agent. A `SourceWatcherExt` turns it into a research agent.

### The Extension trait

```rust
/// Hook into the agent heartbeat pipeline.
///
/// Extensions provide domain-specific behavior at each step of the
/// nine-step heartbeat. They are composed into an `ExtensionChain`
/// that fires hooks in dependency-respecting topological order.
///
/// # Layers
///
/// Extensions declare a layer (0-7) that determines their position
/// in the chain. Foundation (0) extensions run first; Recovery (7)
/// extensions run last. Within a layer, dependency edges determine
/// the exact order.
///
/// # Lifecycle
///
/// An extension is constructed once, added to the chain during agent
/// provisioning, and dropped when the agent enters the Terminal state.
/// Extensions may hold interior-mutable state (via `Arc<Mutex<_>>` or
/// lock-free atomics) that persists across ticks.
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    /// Human-readable name for logging and dependency resolution.
    fn name(&self) -> &str;

    /// Which layer this extension occupies in the chain.
    fn layer(&self) -> ExtensionLayer;

    /// Names of extensions that must fire before this one.
    /// The chain builder validates that all dependencies exist and
    /// that the dependency graph is acyclic.
    fn depends_on(&self) -> &[&str] { &[] }

    // ── Session lifecycle (4 hooks) ──────────────────────────────

    /// Called once when the agent transitions from Provisioning to Active.
    /// Use this to open connections, start background tasks, and load
    /// initial state from disk.
    async fn on_activate(&self, ctx: &mut ActivateContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called once when the agent transitions to Suspended.
    /// Flush buffers, close non-essential connections, persist hot state.
    async fn on_suspend(&self, ctx: &mut SuspendContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called once when the agent transitions from Suspended to Active.
    /// Reopen connections, reload hot state, validate consistency.
    async fn on_resume(&self, ctx: &mut ResumeContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called once when the agent transitions to Terminal.
    /// Close all connections, flush all state, release all resources.
    /// This is the last hook that fires.
    async fn on_terminate(&self, ctx: &mut TerminateContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    // ── Heartbeat (3 hooks) ──────────────────────────────────────

    /// Called at the start of every gamma tick, before OBSERVE.
    /// Use this for pre-tick housekeeping: update timers, check
    /// cancellation, refresh cached state.
    async fn on_tick_start(&self, ctx: &TickContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called at the end of every gamma tick, after REFLECT.
    /// Use this for post-tick cleanup: flush metrics, update
    /// progress indicators, emit telemetry.
    async fn on_tick_end(&self, ctx: &TickContext, record: &DecisionCycleRecord) -> Result<()> {
        let _ = (ctx, record);
        Ok(())
    }

    /// Called when the heartbeat clock adjusts its cadence.
    /// Extensions that maintain their own timers should adjust
    /// accordingly.
    async fn on_cadence_change(
        &self,
        speed: HeartbeatSpeed,
        old_interval: Duration,
        new_interval: Duration,
    ) -> Result<()> {
        let _ = (speed, old_interval, new_interval);
        Ok(())
    }

    // ── Perception (2 hooks) ─────────────────────────────────────

    /// OBSERVE step. Read environment and return observations.
    /// Called once per gamma tick. Extensions that monitor external
    /// state (chain blocks, file system, network) implement this.
    async fn observe(
        &self,
        cortical: &CorticalState,
        cancel: &CancelToken,
    ) -> Result<Vec<Observation>> {
        let _ = (cortical, cancel);
        Ok(Vec::new())
    }

    /// RETRIEVE step. Query knowledge stores and return candidates
    /// for the attention auction.
    async fn retrieve(
        &self,
        workspace: &CognitiveWorkspace,
        cortical: &CorticalState,
        cancel: &CancelToken,
    ) -> Result<Vec<ContextCandidate>> {
        let _ = (workspace, cortical, cancel);
        Ok(Vec::new())
    }

    // ── Cognition (4 hooks) ──────────────────────────────────────

    /// ANALYZE step. Compute this extension's prediction error component.
    /// Return 0.0 if this extension has no PE contribution this tick.
    async fn analyze(
        &self,
        workspace: &CognitiveWorkspace,
        cortical: &CorticalState,
    ) -> Result<f32> {
        let _ = (workspace, cortical);
        Ok(0.0)
    }

    /// SIMULATE step. Sandbox candidate actions.
    /// Return None if this extension does not participate in simulation.
    async fn simulate(
        &self,
        workspace: &CognitiveWorkspace,
        tier: InferenceTier,
        cancel: &CancelToken,
    ) -> Result<Option<SimulationResult>> {
        let _ = (workspace, tier, cancel);
        Ok(None)
    }

    /// VALIDATE step. Check safety constraints on proposed actions.
    /// Return Err to veto an action.
    async fn validate(
        &self,
        actions: &[ProposedAction],
        cortical: &CorticalState,
    ) -> Result<Vec<ValidationResult>> {
        let _ = (actions, cortical);
        Ok(Vec::new())
    }

    /// EXECUTE step. Perform domain-specific actions.
    /// Return outcomes for verification.
    async fn execute(
        &self,
        actions: &[ValidatedAction],
        tier: InferenceTier,
        cancel: &CancelToken,
    ) -> Result<Vec<ActionOutcome>> {
        let _ = (actions, tier, cancel);
        Ok(Vec::new())
    }

    // ── Action (2 hooks) ─────────────────────────────────────────

    /// VERIFY step. Confirm action outcomes match expectations.
    async fn verify(
        &self,
        outcomes: &[ActionOutcome],
        workspace: &CognitiveWorkspace,
        cancel: &CancelToken,
    ) -> Result<Vec<VerificationResult>> {
        let _ = (outcomes, workspace, cancel);
        Ok(Vec::new())
    }

    /// Compensation hook. Called when verification fails, before REFLECT.
    /// Attempt to undo or mitigate the failed action.
    async fn on_compensation(
        &self,
        failed: &ActionOutcome,
        verification: &VerificationResult,
    ) -> Result<CompensationResult> {
        let _ = (failed, verification);
        Ok(CompensationResult::NoAction)
    }

    // ── Learning (2 hooks) ───────────────────────────────────────

    /// REFLECT step. Update internal state based on the completed cycle.
    /// Called after every tick (including T0 ticks, via run_reflect_t0).
    async fn reflect(
        &self,
        record: &DecisionCycleRecord,
        cortical: &CorticalState,
    ) -> Result<()> {
        let _ = (record, cortical);
        Ok(())
    }

    /// Called when the delta consolidation loop completes a dream cycle.
    /// Use this to process newly promoted knowledge, updated somatic
    /// markers, and pruning decisions.
    async fn on_dream_complete(&self, report: &DreamCycleReport) -> Result<()> {
        let _ = report;
        Ok(())
    }

    // ── Events (2 hooks) ─────────────────────────────────────────

    /// Called when a runtime event matches this extension's subscription
    /// filter. Extensions declare interest via `event_filter()`.
    async fn on_event(&self, event: &RuntimeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }

    /// Declare which event categories this extension subscribes to.
    /// Return an empty set to receive no events.
    fn event_filter(&self) -> HashSet<EventCategory> {
        HashSet::new()
    }

    // ── Dreams (3 hooks) ─────────────────────────────────────────

    /// Called when the agent enters the Dreaming state.
    /// Flush working state, prepare for consolidation.
    async fn on_dream_enter(&self) -> Result<()> {
        Ok(())
    }

    /// Called during NREM replay when an episode relevant to this
    /// extension is being replayed.
    async fn on_replay(&self, episode: &Episode) -> Result<()> {
        let _ = episode;
        Ok(())
    }

    /// Called when the agent exits the Dreaming state.
    /// Reload working state, resume normal perception.
    async fn on_dream_exit(&self) -> Result<()> {
        Ok(())
    }
}
```

### Extension layers

Extensions declare a layer that determines their coarse position in the firing order. Within a layer, dependency edges refine the exact order.

```rust
/// Layer in the extension chain. Lower layers fire first.
///
/// The layer system provides coarse ordering guarantees so that
/// extension authors do not need to enumerate fine-grained dependencies
/// against every other extension. Within a layer, topological sort
/// on `depends_on()` edges determines exact order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ExtensionLayer {
    /// Layer 0: Foundation.
    /// Core infrastructure: heartbeat clock, event fabric, CorticalState
    /// initialization. Must run before anything that reads shared state.
    Foundation = 0,

    /// Layer 1: Perception.
    /// Environment readers: chain subscriber, file watcher, network probes.
    /// Depends on Foundation for event fabric access.
    Perception = 1,

    /// Layer 2: Memory.
    /// Knowledge store interfaces: NeuroExt, PlaybookExt, ChainKnowledgeExt.
    /// Depends on Perception because retrievals reference observations.
    Memory = 2,

    /// Layer 3: Cognition.
    /// Analysis, prediction, tier selection assistance.
    /// Depends on Memory for retrieved context.
    Cognition = 3,

    /// Layer 4: Action.
    /// Tool execution, transaction submission, file modification.
    /// Depends on Cognition for action plans.
    Action = 4,

    /// Layer 5: Learning.
    /// Episode logging, skill extraction, efficiency tracking.
    /// Depends on Action for outcome data.
    Learning = 5,

    /// Layer 6: Affect.
    /// Emotional state, somatic markers, behavioral phase transitions.
    /// Runs after Learning because affect appraisal uses outcome quality.
    Affect = 6,

    /// Layer 7: Recovery.
    /// Error recovery, compensation, circuit breaking.
    /// Runs last so it can observe all other extension outputs.
    Recovery = 7,
}
```

---

## 6. Extension chain composition and firing

### Building the chain

The `ExtensionChainBuilder` validates dependencies, detects cycles, and produces a topologically sorted firing order for each hook:

```rust
/// Builder for the extension chain.
///
/// Validates that all dependency edges resolve, that no cycles exist,
/// and that layer ordering is consistent with declared dependencies
/// (an extension cannot depend on an extension in a higher layer).
pub struct ExtensionChainBuilder {
    extensions: Vec<Box<dyn Extension>>,
}

impl ExtensionChainBuilder {
    pub fn new() -> Self {
        Self { extensions: Vec::new() }
    }

    /// Add an extension to the chain.
    pub fn add(mut self, ext: Box<dyn Extension>) -> Self {
        self.extensions.push(ext);
        self
    }

    /// Validate and build the extension chain.
    ///
    /// Returns Err if:
    /// - A dependency references a name that does not exist in the chain
    /// - A dependency points to an extension in a higher layer
    /// - The dependency graph contains a cycle
    pub fn build(self) -> Result<ExtensionChain> {
        // Index by name
        let name_to_idx: HashMap<&str, usize> = self.extensions
            .iter()
            .enumerate()
            .map(|(i, ext)| (ext.name(), i))
            .collect();

        // Validate dependencies exist and respect layer ordering
        for ext in &self.extensions {
            for dep_name in ext.depends_on() {
                let dep_idx = name_to_idx.get(dep_name)
                    .ok_or_else(|| anyhow!(
                        "extension '{}' depends on '{}' which is not in the chain",
                        ext.name(), dep_name
                    ))?;
                let dep_layer = self.extensions[*dep_idx].layer();
                if dep_layer > ext.layer() {
                    return Err(anyhow!(
                        "extension '{}' (layer {:?}) depends on '{}' (layer {:?}) \
                         -- dependencies must not point to higher layers",
                        ext.name(), ext.layer(), dep_name, dep_layer
                    ));
                }
            }
        }

        // Topological sort within each layer
        let firing_order = topological_sort(&self.extensions, &name_to_idx)?;

        Ok(ExtensionChain {
            extensions: self.extensions,
            firing_order,
        })
    }
}
```

### How extensions communicate

Extensions do not call each other directly. They communicate through four channels, each suited to a different pattern.

**CorticalState (lock-free atomics).** For real-time inter-extension signals with no contention. CorticalState holds ~32 atomic fields (affect, vitality, perception, communication). Any extension can read or write any field at any time with acquire/release ordering. No locks, no allocation, no blocking.

Example: `DaimonExt` writes the PAD vector after affect appraisal. `ContextExt` reads the PAD vector to modulate attention auction weights (negative pleasure increases weight on iteration-memory context). `DreamsExt` reads arousal to compute sleep pressure rate adjustment.

**CognitiveWorkspace (per-tick).** For context contribution via VCG auction. Each extension bids context candidates into the workspace during the RETRIEVE step. The workspace runs the attention auction and assembles the final context window. This is the mechanism that determines what an LLM sees in its prompt.

The workspace is rebuilt every tick. It is not shared state -- it is a scratchpad that lives for exactly one heartbeat cycle.

**EventFabric (broadcast).** For async event notification across extensions and across agents. An extension emits a `RuntimeEvent` on the bus. All extensions subscribed to that event category receive it via their `on_event` hook. Events are fire-and-forget from the emitter's perspective.

Example: `GateExt` emits a `GateVerdict` event when tests fail. `ConductorExt` receives it and updates the circuit breaker state. `DaimonExt` receives it and appraises the failure emotionally.

**BootContext (initialization-time).** For shared state wiring during agent provisioning. When extensions are added to the chain, the `ActivateContext` provides references to shared resources (NeuroStore handle, ToolRegistry, EventFabric sender). Extensions store these references internally and use them throughout their lifetime.

### Concrete communication flow

Here is a full tick showing inter-extension communication:

1. `HeartbeatExt` (Foundation) increments the tick counter on CorticalState
2. `ChainSubscriberExt` (Perception) reads the latest block, writes `gas_gwei` to CorticalState, emits `NewBlock` event
3. `NeuroExt` (Memory) queries knowledge store using observations from step 2, bids results into workspace
4. `DaimonExt` (Affect) reads PAD from CorticalState, appraises recent outcomes, writes updated PAD back
5. `ContextExt` (Memory) reads PAD from CorticalState, adjusts auction weights, runs final auction
6. `GateExt` (Action) runs tier-appropriate gates based on workspace context
7. `LearningExt` (Learning) logs the decision cycle record, extracts skills, updates efficiency metrics
8. `DreamsExt` (Affect) reads arousal from CorticalState, accumulates sleep pressure
9. `ConductorExt` (Recovery) reads all CorticalState fields, checks circuit breaker, emits `CognitiveSignal` if intervention needed

No extension calls any other extension. They read and write shared surfaces. The firing order guarantees that when `ContextExt` reads PAD, `DaimonExt` has already written it.

---

## 7. Type-state lifecycle

The agent's lifecycle is encoded in the Rust type system. Invalid transitions are compiler errors, not runtime checks.

### Phase markers

```rust
/// Agent is being provisioned. Extensions are being loaded,
/// resources allocated, knowledge store initialized.
pub struct Provisioning;

/// Agent is running. The heartbeat loop is active.
/// This is the only state where `tick()` is callable.
pub struct Active;

/// Agent is in a dream cycle. The heartbeat is paused.
/// Consolidation is running. Only emergency wakeup is allowed.
pub struct Dreaming;

/// Agent is operator-paused. State is retained in memory.
/// The heartbeat is stopped. Resume restarts the loop.
pub struct Suspended;

/// Agent is shutting down. Extensions are flushing state.
/// No new ticks. Waiting for graceful completion.
pub struct Terminal;

/// Agent has shut down. All resources released.
/// The only valid operation is extracting the genome
/// (configuration + learned state for reincarnation).
pub struct Dead;
```

### The Agent struct

```rust
/// A persistent agent process with compile-time lifecycle enforcement.
///
/// `Agent<Phase>` uses Rust's type-state pattern: each lifecycle phase
/// is a zero-sized type used as a generic parameter. Methods are defined
/// in `impl` blocks specific to each phase. Calling `.tick()` on a
/// `Agent<Dead>` is a compile error, not a runtime check.
///
/// # Move semantics
///
/// Transitions consume `self` and return `Agent<NewPhase>`. The old
/// phase is gone -- you cannot use a reference to the provisioning
/// agent after activation. This prevents use-after-transition bugs
/// at compile time.
pub struct Agent<Phase> {
    /// Stable identifier across restarts.
    id: AgentId,
    /// The extension chain that defines this agent's behavior.
    chain: ExtensionChain,
    /// Lock-free shared perception surface.
    cortical: Arc<CorticalState>,
    /// Event fabric for broadcast communication.
    event_fabric: Arc<EventFabric>,
    /// The heartbeat pipeline (only usable in Active state).
    pipeline: HeartbeatPipeline,
    /// Domain profile (chain, code, research, etc.).
    domain: DomainProfile,
    /// Accumulated cognitive state.
    state: AgentRuntimeState,
    /// Zero-sized phase marker.
    _phase: PhantomData<Phase>,
}
```

### Valid transitions

Each `impl` block exists only for the phase where its methods are valid. The compiler enforces the transition graph.

```rust
impl Agent<Provisioning> {
    /// Create a new agent in the provisioning phase.
    pub fn new(
        id: AgentId,
        domain: DomainProfile,
        config: AgentConfig,
    ) -> Self {
        let cortical = Arc::new(CorticalState::new(config.personality));
        let event_fabric = Arc::new(EventFabric::new(10_000));
        Agent {
            id,
            chain: ExtensionChain::empty(),
            cortical,
            event_fabric,
            pipeline: HeartbeatPipeline::new(config.clone()),
            domain,
            state: AgentRuntimeState::default(),
            _phase: PhantomData,
        }
    }

    /// Add an extension to this agent's chain.
    /// Only callable during provisioning.
    pub fn with_extension(mut self, ext: Box<dyn Extension>) -> Self {
        self.chain.add(ext);
        self
    }

    /// Finalize the extension chain and activate the agent.
    ///
    /// This validates the chain (dependency resolution, cycle detection),
    /// calls `on_activate` on every extension in firing order, and
    /// transitions to the Active state.
    ///
    /// Consumes `self`. The Provisioning agent no longer exists.
    pub async fn activate(mut self) -> Result<Agent<Active>> {
        self.chain.validate_and_build()?;
        let ctx = ActivateContext {
            cortical: Arc::clone(&self.cortical),
            event_fabric: Arc::clone(&self.event_fabric),
            domain: &self.domain,
        };
        self.chain.run_activate(ctx).await?;
        Ok(self.transition())
    }
}

impl Agent<Active> {
    /// Execute one heartbeat tick.
    /// Only callable in the Active state.
    pub async fn tick(
        &mut self,
        clock: &HeartbeatClock,
        cancel: &CancelToken,
    ) -> Result<DecisionCycleRecord> {
        self.pipeline.execute_tick(clock, cancel).await
    }

    /// Enter the dreaming state for consolidation.
    /// Consumes the Active agent. Returns a Dreaming agent.
    pub async fn dream(mut self) -> Result<Agent<Dreaming>> {
        self.chain.run_dream_enter().await?;
        Ok(self.transition())
    }

    /// Operator-initiated pause.
    /// Consumes the Active agent. Returns a Suspended agent.
    pub async fn suspend(mut self) -> Result<Agent<Suspended>> {
        self.chain.run_suspend().await?;
        Ok(self.transition())
    }

    /// Begin graceful shutdown.
    /// Consumes the Active agent. Returns a Terminal agent.
    pub async fn terminate(self) -> Agent<Terminal> {
        self.transition()
    }
}

impl Agent<Dreaming> {
    /// Normal wakeup after dream cycle completion.
    /// Consumes the Dreaming agent. Returns an Active agent.
    pub async fn wake(mut self) -> Result<Agent<Active>> {
        self.chain.run_dream_exit().await?;
        Ok(self.transition())
    }

    /// Emergency wakeup triggered by a high-priority event.
    /// Aborts the current dream cycle and returns to Active.
    pub async fn emergency_wake(
        mut self,
        condition: WakeupCondition,
    ) -> Result<Agent<Active>> {
        self.chain.run_dream_exit().await?;
        self.event_fabric.emit(RuntimeEvent::emergency_wakeup(condition));
        Ok(self.transition())
    }

    /// Shut down from the dreaming state.
    pub async fn terminate(self) -> Agent<Terminal> {
        self.transition()
    }
}

impl Agent<Suspended> {
    /// Resume the agent. Reopens connections, reloads hot state.
    /// Consumes the Suspended agent. Returns an Active agent.
    pub async fn resume(mut self) -> Result<Agent<Active>> {
        self.chain.run_resume().await?;
        Ok(self.transition())
    }

    /// Shut down from the suspended state.
    pub async fn terminate(self) -> Agent<Terminal> {
        self.transition()
    }
}

impl Agent<Terminal> {
    /// Flush all extension state and release resources.
    /// Consumes the Terminal agent. Returns a Dead agent.
    pub async fn finalize(mut self) -> Agent<Dead> {
        // Best-effort: call on_terminate on each extension.
        // Errors are logged but do not prevent finalization.
        let _ = self.chain.run_terminate().await;
        self.transition()
    }
}

impl Agent<Dead> {
    /// Extract the agent's genome: configuration + learned state.
    ///
    /// The genome can be used to provision a new agent with the
    /// same personality, knowledge, and behavioral patterns.
    /// This is how agents survive across restarts and migrations.
    pub fn extract_genome(self) -> AgentGenome {
        AgentGenome {
            id: self.id,
            domain: self.domain,
            cortical_snapshot: self.cortical.snapshot(),
            state: self.state,
        }
    }
}

// Private transition helper. Works for any phase pair.
impl<Phase> Agent<Phase> {
    fn transition<NewPhase>(self) -> Agent<NewPhase> {
        Agent {
            id: self.id,
            chain: self.chain,
            cortical: self.cortical,
            event_fabric: self.event_fabric,
            pipeline: self.pipeline,
            domain: self.domain,
            state: self.state,
            _phase: PhantomData,
        }
    }
}
```

### Why type-state

Rust's ownership system makes type-state particularly effective. When you call `agent.activate()`, the `Provisioning` agent is consumed by move. You cannot accidentally call `.with_extension()` after activation -- the variable is gone. You cannot call `.tick()` on a suspended agent. You cannot call `.dream()` on a dead agent.

This replaces the `match agent.state { ... }` pattern that most languages use, where every method starts with a runtime check for valid state. Those checks are easy to forget, especially in concurrent code. With type-state, the compiler catches every violation.

The approach uses `PhantomData<Phase>` and zero-sized type markers. The `Agent` struct is the same size regardless of phase. Transitions are `mem::transmute`-equivalent -- they move data without copying. The only runtime cost is the extension chain hooks that fire on transition.

---

## 8. CorticalState -- lock-free shared perception

`CorticalState` is the shared perception surface that all extensions and the heartbeat pipeline read and write concurrently. It uses atomic operations exclusively -- no locks, no allocation on the hot path, no blocking.

### Structure

The struct is `#[repr(C, align(64))]` to prevent false sharing on cache lines:

```rust
/// Lock-free shared perception surface for heartbeat subsystems.
///
/// Every field is an atomic. Extensions write specific fields;
/// the heartbeat pipeline reads all fields for tier selection
/// and workspace assembly. No locks, no contention.
///
/// # Encoding
///
/// Floating-point values are stored as `AtomicU32` using `f32::to_bits()`
/// and `f32::from_bits()`. This gives sub-integer precision without the
/// overhead of `AtomicF32` (which does not exist in `std`). For values
/// in [-1.0, 1.0], f32 is lossless.
///
/// Enum variants are stored as `AtomicU8` with manual `from_u8` conversion.
#[repr(C, align(64))]
pub struct CorticalState {
    // ── Affect (4 fields) ────────────────────────────────────────
    /// Pleasure dimension of the PAD vector. Range: [-1.0, 1.0].
    pleasure: AtomicU32,
    /// Arousal dimension of the PAD vector. Range: [-1.0, 1.0].
    arousal: AtomicU32,
    /// Dominance dimension of the PAD vector. Range: [-1.0, 1.0].
    dominance: AtomicU32,
    /// Current behavioral phase (Engaged, Struggling, etc.).
    behavioral_state: AtomicU8,

    // ── Vitality (4 fields) ──────────────────────────────────────
    /// Economic health: remaining budget / total budget. [0.0, 1.0].
    economic_vitality: AtomicU32,
    /// Epistemic confidence: calibrated prediction accuracy. [0.0, 1.0].
    epistemic_confidence: AtomicU32,
    /// Composite vitality: weighted combination. [0.0, 1.0].
    composite_vitality: AtomicU32,
    /// Stochastic survival probability (Korai-derived). [0.0, 1.0].
    stochastic_survival: AtomicU32,

    // ── Perception (4 fields) ────────────────────────────────────
    /// Monotonically increasing tick counter.
    tick_count: AtomicU64,
    /// Timestamp of last observation (unix millis).
    last_observation_ms: AtomicU64,
    /// Most recent aggregate prediction error. [0.0, 1.0].
    prediction_error: AtomicU32,
    /// Current cognitive tier (T0=0, T1=1, T2=2).
    cognitive_tier: AtomicU8,

    // ── Prediction (6 fields) ────────────────────────────────────
    /// Aggregate prediction accuracy (EWMA). [0.0, 1.0].
    aggregate_accuracy: AtomicU32,
    /// Accuracy trend: -1 (declining), 0 (stable), +1 (improving).
    accuracy_trend: AtomicI8,
    /// Per-category accuracy (16 slots, indexed by category).
    category_accuracies: [AtomicU32; 16],
    /// Surprise rate: fraction of recent ticks that exceeded PE threshold.
    surprise_rate: AtomicU32,
    /// Number of active predictions being tracked.
    active_count: AtomicU16,
    /// Number of predictions awaiting resolution.
    pending_predictions: AtomicU32,

    // ── Creativity (3 fields) ────────────────────────────────────
    /// Whether creative mode (high-entropy generation) is active.
    creative_mode: AtomicU8,
    /// Number of novel fragments captured this session.
    fragments_captured: AtomicU32,
    /// Tick when the last novel prediction was registered.
    last_novel_prediction_tick: AtomicU64,

    // ── Environment (3 fields) ───────────────────────────────────
    /// Environmental regime (Calm=0, Normal=1, Volatile=2, Crisis=3).
    regime: AtomicU8,
    /// Gas price in gwei (chain agents).
    gas_gwei: AtomicU32,
    /// Resource health: memory/disk/network composite. [0.0, 1.0].
    resource_health: AtomicU32,

    // ── Knowledge (2 fields) ─────────────────────────────────────
    /// Knowledge health: coverage and freshness. [0.0, 1.0].
    knowledge_health: AtomicU32,
    /// Performance trend: recent outcome quality. [-1.0, 1.0].
    performance_trend: AtomicU32,

    // ── Communication (3 fields) ─────────────────────────────────
    /// Pheromone signal for inter-agent communication.
    pheromone_signal: AtomicU32,
    /// Blake3 hash of the current top-attention entry.
    attention_top_hash: AtomicU64,
    /// Compounding momentum: streak of successful outcomes. [0.0, 1.0].
    compounding_momentum: AtomicU32,

    // ── Emotion (1 field) ────────────────────────────────────────
    /// Primary Plutchik emotion label.
    primary_emotion: AtomicU8,
}
```

### Fixed-point encoding

Rust's standard library does not provide `AtomicF32` or `AtomicF64`. CorticalState uses a bit-pattern encoding:

```rust
/// Read an f32 from an AtomicU32 using bit-pattern reinterpretation.
fn load_f32(atom: &AtomicU32) -> f32 {
    f32::from_bits(atom.load(Ordering::Acquire))
}

/// Write an f32 to an AtomicU32 using bit-pattern reinterpretation.
fn store_f32(atom: &AtomicU32, value: f32) {
    atom.store(value.to_bits(), Ordering::Release);
}
```

This is lossless for all f32 values. For fields clamped to [-1.0, 1.0] (PAD dimensions, health signals), f32 provides ~7 decimal digits of precision, which far exceeds what affect models need.

All reads use `Ordering::Acquire` and all writes use `Ordering::Release`. This guarantees that if extension A writes a value and extension B later reads it, B sees A's write (assuming the firing order puts A before B). Within a single tick, the topological firing order provides this guarantee. Across ticks, eventual consistency is sufficient -- all fields converge within one gamma period.

### Snapshot

For serialization, logging, and cross-process communication, CorticalState provides an eventually consistent snapshot:

```rust
/// Consistent read of all CorticalState fields.
///
/// "Eventually consistent" because each field is read independently.
/// Within a single gamma tick, the snapshot reflects a coherent state
/// because extensions fire sequentially within the tick. Across ticks,
/// fields may be from different tick generations, but the delta is
/// bounded to one gamma period.
pub fn snapshot(&self) -> CorticalSnapshot {
    CorticalSnapshot {
        pad: self.pad(),
        behavioral_state: self.behavioral_state(),
        economic_vitality: load_f32(&self.economic_vitality),
        epistemic_confidence: load_f32(&self.epistemic_confidence),
        tick_count: self.tick_count.load(Ordering::Acquire),
        prediction_error: load_f32(&self.prediction_error),
        cognitive_tier: InferenceTier::from_u8(
            self.cognitive_tier.load(Ordering::Acquire)
        ),
        regime: self.regime(),
        primary_emotion: self.primary_emotion(),
        // ... remaining fields
    }
}
```

---

## 9. Event fabric

The event fabric provides typed, sequenced, filtered event distribution across extensions, agents, and external subscribers.

### Core structure

```rust
/// Broadcast event bus with ring buffer replay and filtered subscriptions.
///
/// Built on `tokio::sync::broadcast` for live fan-out and a bounded
/// `VecDeque` ring for replay support. New subscribers can catch up
/// by replaying events from a sequence number.
pub struct EventFabric {
    /// Broadcast sender for live events.
    tx: broadcast::Sender<Envelope<RuntimeEvent>>,
    /// Ring buffer for replay (bounded, default 10K events).
    ring: Mutex<VecDeque<Envelope<RuntimeEvent>>>,
    /// Monotonically increasing sequence counter.
    seq: AtomicU64,
    /// Ring buffer capacity.
    capacity: usize,
}

impl EventFabric {
    /// Create a new event fabric with the given ring buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            ring: Mutex::new(VecDeque::with_capacity(capacity)),
            seq: AtomicU64::new(0),
            capacity,
        }
    }

    /// Emit an event. Assigns a sequence number and timestamp.
    pub fn emit(&self, event: RuntimeEvent) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let ts_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let envelope = Envelope {
            seq,
            ts_millis,
            payload: event,
        };

        // Write to ring buffer for replay
        let mut ring = self.ring.lock();
        if ring.len() >= self.capacity {
            ring.pop_front();
        }
        ring.push_back(envelope.clone());
        drop(ring);

        // Broadcast to live subscribers (best-effort: lagging
        // subscribers miss events and must use replay)
        let _ = self.tx.send(envelope);
    }

    /// Subscribe with a filter. Only events matching the filter
    /// are delivered to this subscriber's channel.
    pub fn subscribe_filtered(
        &self,
        filter: HashSet<EventCategory>,
    ) -> FilteredSubscription {
        let rx = self.tx.subscribe();
        FilteredSubscription { rx, filter }
    }

    /// Replay all events from the given sequence number.
    pub fn replay_from(&self, from_seq: u64) -> Vec<Envelope<RuntimeEvent>> {
        let ring = self.ring.lock();
        ring.iter()
            .filter(|e| e.seq >= from_seq)
            .cloned()
            .collect()
    }
}
```

### Event types

```rust
/// Source of a runtime event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSource {
    /// Blockchain or Korai network.
    Chain,
    /// Local file system watcher.
    FileSystem,
    /// Another agent in the same runtime.
    Agent(AgentId),
    /// Gate pipeline.
    Gate,
    /// Heartbeat timer.
    Timer,
    /// External system (HTTP webhook, CLI command).
    External(String),
}

/// Payload of a runtime event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventPayload {
    /// A new block was produced on the watched chain.
    NewBlock {
        chain_id: u64,
        block_number: u64,
        block_hash: String,
        timestamp: u64,
    },
    /// A transaction relevant to this agent was observed.
    Transaction {
        chain_id: u64,
        tx_hash: String,
        from: String,
        to: String,
        value_wei: String,
    },
    /// A price feed updated.
    PriceFeed {
        asset: String,
        price_usd: f64,
        source: String,
    },
    /// A watched file changed.
    FileChanged {
        path: PathBuf,
        kind: FileChangeKind,
    },
    /// A gate produced a verdict.
    GateVerdict {
        task_id: String,
        gate_name: String,
        passed: bool,
        details: Option<String>,
    },
    /// A test run completed.
    TestResult {
        suite: String,
        passed: u32,
        failed: u32,
        skipped: u32,
        duration_ms: u64,
    },
    /// An agent started.
    AgentStarted {
        agent_id: AgentId,
        domain: String,
    },
    /// An agent completed its task or shut down.
    AgentCompleted {
        agent_id: AgentId,
        outcome: AgentOutcome,
    },
    /// Inter-agent pheromone signal.
    PheromoneSignal {
        source: AgentId,
        kind: PheromoneKind,
        scope: PheromoneScope,
        payload: Vec<u8>,
    },
    /// Heartbeat tick notification.
    HeartbeatTick {
        agent_id: AgentId,
        tick_count: u64,
        tier: InferenceTier,
        pe: f32,
    },
    /// ISFR (Intertemporal Survival Function Ratio) update from Korai.
    ISFRUpdate {
        agent_id: AgentId,
        isfr: f64,
        block_number: u64,
    },
    /// Clearing result from Korai marketplace.
    ClearingResult {
        auction_id: String,
        winning_bid: f64,
        agent_id: AgentId,
    },
    /// Custom event for extension-defined payloads.
    Custom {
        kind: String,
        data: serde_json::Value,
    },
}

/// Categorization for filtered subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    Chain,
    FileSystem,
    Gate,
    Agent,
    Timer,
    Pheromone,
    Economic,
    Custom,
}

/// A complete runtime event with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    /// Sequence number (assigned by EventFabric).
    pub seq: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Where this event originated.
    pub source: EventSource,
    /// The event payload.
    pub payload: EventPayload,
}

impl RuntimeEvent {
    /// Return the event category for subscription filtering.
    pub fn category(&self) -> EventCategory {
        match &self.payload {
            EventPayload::NewBlock { .. }
            | EventPayload::Transaction { .. }
            | EventPayload::PriceFeed { .. } => EventCategory::Chain,
            EventPayload::FileChanged { .. } => EventCategory::FileSystem,
            EventPayload::GateVerdict { .. }
            | EventPayload::TestResult { .. } => EventCategory::Gate,
            EventPayload::AgentStarted { .. }
            | EventPayload::AgentCompleted { .. } => EventCategory::Agent,
            EventPayload::HeartbeatTick { .. } => EventCategory::Timer,
            EventPayload::PheromoneSignal { .. } => EventCategory::Pheromone,
            EventPayload::ISFRUpdate { .. }
            | EventPayload::ClearingResult { .. } => EventCategory::Economic,
            EventPayload::Custom { .. } => EventCategory::Custom,
        }
    }
}
```

### Filtered subscription

Agents subscribe to the categories they care about. A coding agent subscribes to FileSystem, Gate, and Agent events. A chain agent subscribes to Chain, Economic, and Pheromone events. Irrelevant events are filtered before delivery, avoiding wasted processing.

```rust
/// A subscription that filters events by category.
pub struct FilteredSubscription {
    rx: broadcast::Receiver<Envelope<RuntimeEvent>>,
    filter: HashSet<EventCategory>,
}

impl FilteredSubscription {
    /// Receive the next event matching this subscription's filter.
    /// Blocks until an event is available or the fabric is dropped.
    pub async fn recv(&mut self) -> Result<RuntimeEvent> {
        loop {
            let envelope = self.rx.recv().await?;
            if self.filter.contains(&envelope.payload.category()) {
                return Ok(envelope.payload);
            }
            // Event does not match filter -- skip it
        }
    }
}
```

---

## 10. Process model and supervision

### Actor model with mailboxes

Each agent runs as a Tokio task with a mailbox (bounded `mpsc` channel). External systems and other agents send messages to the mailbox. The agent's main loop alternates between heartbeat ticks and mailbox reads:

```rust
/// Main loop for a persistent agent process.
///
/// Alternates between heartbeat ticks and mailbox reads.
/// The heartbeat timer fires at the gamma interval; mailbox
/// messages are processed between ticks.
pub async fn agent_main_loop(
    mut agent: Agent<Active>,
    mut mailbox: mpsc::Receiver<AgentMessage>,
    clock: HeartbeatClock,
    cancel: CancelToken,
) -> Result<Agent<Dead>> {
    let mut interval = tokio::time::interval(
        clock.gamma_interval(agent.cortical.regime())
    );
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let record = agent.tick(&clock, &cancel).await?;
                // Check for dream entry
                if agent.should_dream() {
                    let dreaming = agent.dream().await?;
                    // Run consolidation
                    let dreaming = run_dream_cycle(dreaming).await?;
                    agent = dreaming.wake().await?;
                }
            }
            Some(msg) = mailbox.recv() => {
                match msg {
                    AgentMessage::Task(envelope) => {
                        agent.inject_task(envelope).await?;
                    }
                    AgentMessage::Suspend => {
                        let suspended = agent.suspend().await?;
                        // Wait for resume or terminate
                        match wait_for_resume_or_terminate(&mut mailbox).await {
                            ResumeOrTerminate::Resume => {
                                agent = suspended.resume().await?;
                            }
                            ResumeOrTerminate::Terminate => {
                                let terminal = suspended.terminate().await;
                                return Ok(terminal.finalize().await);
                            }
                        }
                    }
                    AgentMessage::Terminate => {
                        let terminal = agent.terminate().await;
                        return Ok(terminal.finalize().await);
                    }
                    AgentMessage::WakeupEvent(condition) => {
                        // Force an immediate tick
                        let record = agent.tick(&clock, &cancel).await?;
                    }
                }
            }
            _ = cancel.cancelled() => {
                let terminal = agent.terminate().await;
                return Ok(terminal.finalize().await);
            }
        }
    }
}
```

### Supervision strategies

The `ProcessSupervisor` (already implemented in `roko-runtime/src/process.rs`) manages a pool of agent processes. It supports three Erlang/OTP-inspired strategies:

**One-for-one.** If a child crashes, restart only that child. Other children continue running. This is the default for independent agents that do not share state.

```rust
SupervisionStrategy::OneForOne {
    max_restarts: 5,
    within_ms: 60_000,   // 5 restarts in 60 seconds
    fallback_tier: "standard".into(),
}
```

**Rest-for-one.** If a child crashes, restart it and all children started after it. Use this when later agents depend on earlier agents' state (e.g., a chain agent depends on a price oracle agent).

```rust
SupervisionStrategy::RestForOne {
    max_restarts: 3,
}
```

**One-for-all.** If any child crashes, restart all children. Use this when agents share critical state that becomes inconsistent after a partial restart. Rarely needed in practice.

### Kill sequence

When an agent must be terminated:

1. **Close stdin.** Signals the agent that no more input is coming. Well-behaved agents begin graceful shutdown.
2. **Wait 800ms.** Allow the agent to flush state, close connections, and exit cleanly.
3. **Send SIGTERM.** (Unix only.) Request termination. The agent's `on_terminate` hooks fire.
4. **Wait grace period** (configurable, default 5s). Allow hooks to complete.
5. **Send SIGKILL.** Force-kill if the process has not exited.

```rust
/// Escalating kill sequence for managed processes.
pub async fn kill_escalating(handle: &mut ProcessHandle) -> ProcessOutcome {
    // Step 1: close stdin
    if let Some(stdin) = handle.child.stdin.take() {
        drop(stdin);
    }

    // Step 2: wait 800ms for voluntary exit
    if let Some(outcome) = handle
        .wait_for_exit(Duration::from_millis(800))
        .await
    {
        return outcome;
    }

    // Step 3: SIGTERM
    #[cfg(unix)]
    if let Some(pid) = handle.os_pid {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
    }

    // Step 4: wait grace period
    if let Some(outcome) = handle
        .wait_for_exit(handle.grace_period)
        .await
    {
        return outcome;
    }

    // Step 5: SIGKILL
    let _ = handle.child.kill().await;
    handle.outcome(None, true)
}
```

### PID registry

Active agent PIDs persist to `.roko/runtime/agent-pids.json`. On startup, the supervisor reads this file and reaps any orphaned processes from a previous crash:

```rust
/// Persistent PID registry for orphan detection.
#[derive(Debug, Serialize, Deserialize)]
pub struct PidRegistry {
    /// Map from agent_id to OS PID.
    pub agents: HashMap<String, u32>,
    /// Timestamp when the registry was last written.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PidRegistry {
    /// Load from disk. Returns empty registry if file does not exist.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self {
                    agents: HashMap::new(),
                    updated_at: chrono::Utc::now(),
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Reap orphaned processes that are still running from a previous session.
    pub fn reap_orphans(&self) {
        for (agent_id, pid) in &self.agents {
            if process_is_alive(*pid) {
                tracing::warn!(
                    agent_id = %agent_id,
                    pid = pid,
                    "reaping orphaned agent process from previous session"
                );
                #[cfg(unix)]
                let _ = kill(Pid::from_raw(*pid as i32), Signal::SIGTERM);
            }
        }
    }
}
```

---

## 11. Backwards compatibility

The existing plan-based orchestration loop in `orchestrate.rs` continues to work. The new agent runtime is an evolution, not a replacement. Migration is incremental.

### PlanRunner becomes a thin coordinator

Today, `PlanRunner` does everything: discover plans, schedule tasks, spawn agents, run gates, persist results. In the new model, PlanRunner becomes a coordinator that:

1. Discovers plans (unchanged)
2. Determines which domain each task belongs to (unchanged)
3. Spawns or connects to a persistent `Agent<Active>` for each domain
4. Injects tasks into the agent's mailbox as `TaskEnvelope` stimuli
5. Collects outcomes from the agent's event fabric
6. Runs cross-agent coordination (merge queue, pheromone routing)

Tasks become stimuli, not commands. The agent decides when and how to execute them based on its current cognitive state, prediction error, and available context.

### TaskEnvelope

```rust
/// A task injected into an agent's mailbox.
///
/// The agent processes task envelopes during its heartbeat loop.
/// Unlike the current model where the orchestrator waits for output,
/// the agent schedules the task based on its own priorities and
/// returns results asynchronously via the event fabric.
pub struct TaskEnvelope {
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Task description and requirements.
    pub task: Task,
    /// Gate configuration for verifying the task's output.
    pub gates: Vec<GateConfig>,
    /// Deadline (if any) -- the agent prioritizes accordingly.
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    /// Channel for sending the result back to PlanRunner.
    pub result_tx: oneshot::Sender<TaskResult>,
}
```

### Transitional API

For the migration period, a compatibility shim wraps the new persistent agent in the old spawn-execute-die interface:

```rust
/// One-shot task execution using the new agent runtime.
///
/// Creates a temporary Agent<Active>, injects the task, runs
/// heartbeat ticks until the task completes, and shuts down.
/// This preserves the existing `spawn_and_run_task()` call sites
/// while routing through the new pipeline.
pub async fn spawn_and_run_task(
    task: Task,
    config: AgentConfig,
    extensions: Vec<Box<dyn Extension>>,
    cancel: &CancelToken,
) -> Result<TaskResult> {
    let mut agent = Agent::new(
        AgentId::ephemeral(),
        config.domain.clone(),
        config,
    );
    for ext in extensions {
        agent = agent.with_extension(ext);
    }
    let mut agent = agent.activate().await?;

    let (result_tx, result_rx) = oneshot::channel();
    agent.inject_task(TaskEnvelope {
        plan_id: String::new(),
        task_id: task.id.clone(),
        task,
        gates: Vec::new(),
        deadline: None,
        result_tx,
    }).await?;

    // Tick until the task completes
    let clock = HeartbeatClock::default();
    loop {
        tokio::select! {
            result = &mut result_rx => {
                let terminal = agent.terminate().await;
                let _ = terminal.finalize().await;
                return result?;
            }
            _ = tokio::time::sleep(clock.gamma_interval(Regime::Normal)) => {
                agent.tick(&clock, cancel).await?;
            }
        }
    }
}
```

### What stays the same

All existing subsystems continue working through extensions:

| Existing subsystem | Extension equivalent |
|---|---|
| Gate pipeline (`roko-gate`) | `GateExt` runs gates in the VERIFY step |
| Episode logger (`roko-learn`) | `LearningExt` logs decision cycle records |
| Efficiency events | `LearningExt` emits per-tick efficiency data |
| Cascade router | `ContextExt` consults the router for tier-to-model mapping |
| Prompt experiments | `ContextExt` applies experiment overrides to the workspace |
| Somatic markers (`roko-daimon`) | `DaimonExt` stores and retrieves markers |
| Stuck detection (`roko-conductor`) | `ConductorExt` monitors gamma summaries |
| Adaptive thresholds | `TierGate` reads from `.roko/learn/gate-thresholds.json` |
| Dream cycles (`roko-dreams`) | `DreamsExt` manages sleep pressure and consolidation |
| Pheromone signaling | `CorticalState::pheromone_signal` + EventFabric |
| Knowledge store (`roko-neuro`) | `NeuroExt` queries and updates entries |
| Safety layer | `SafetyLayer` runs in the VALIDATE step |
| Tool dispatch | `ToolDispatcher` runs in the EXECUTE step |
| MCP passthrough | Agent config includes `mcp_config`, passed to tool dispatcher |

---

## 12. Crate layout

### `roko-runtime` (rewrite)

The existing `roko-runtime` crate provides process management, event bus, cancellation, and metrics. The rewrite preserves these and adds the agent runtime core:

```
crates/roko-runtime/src/
  lib.rs                    # Crate root, public API
  agent.rs                  # Agent<Phase> type-state struct
  heartbeat_pipeline.rs     # HeartbeatPipeline::execute_tick()
  extension.rs              # Extension trait, ExtensionLayer enum
  extension_chain.rs        # ExtensionChainBuilder, topological sort
  cortical_state.rs         # CorticalState (lock-free atomics)
  event_fabric.rs           # EventFabric (broadcast + ring buffer)
  cognitive_workspace.rs    # Per-tick scratchpad with VCG auction
  tier_gate.rs              # Adaptive tier selection (T0/T1/T2)
  domain_profile.rs         # DomainProfile configuration
  agent_main_loop.rs        # agent_main_loop() entry point
  genome.rs                 # AgentGenome for reincarnation
  cancel.rs                 # CancelToken (existing, unchanged)
  process.rs                # ProcessSupervisor (existing, extended)
  event_bus.rs              # EventBus (existing, unchanged)
  metrics.rs                # Metrics recording (existing, unchanged)
  heartbeat.rs              # Heartbeat clock (existing, extended)
  energy.rs                 # Cognitive energy model (existing, unchanged)
  lifecycle.rs              # Lifecycle types (existing, extended)
  theta_consumer.rs         # Theta reflective loop (existing, unchanged)
  delta_consumer.rs         # Delta consolidation loop (existing, unchanged)
```

### `roko-ext-core` (new)

Core extensions that every agent needs regardless of domain:

```
crates/roko-ext-core/src/
  lib.rs
  heartbeat_ext.rs     # Foundation layer. Manages tick counter, timing
                       # adjustments, regime detection on CorticalState.
  context_ext.rs       # Memory layer. Runs the VCG attention auction,
                       # assembles the LLM prompt from workspace candidates,
                       # modulates bid weights based on PAD state.
  daimon_ext.rs        # Affect layer. Reads outcomes, appraises through
                       # ALMA model, writes PAD vector to CorticalState,
                       # stores/retrieves somatic markers.
  learning_ext.rs      # Learning layer. Logs DecisionCycleRecords as
                       # episodes, extracts skills, emits efficiency events,
                       # updates calibration trackers.
  dreams_ext.rs        # Affect layer. Accumulates sleep pressure, manages
                       # dream entry/exit transitions, coordinates with the
                       # delta consumer for consolidation cycles.
```

### `roko-ext-code` (new)

Extensions for software engineering agents:

```
crates/roko-ext-code/src/
  lib.rs
  git_ext.rs           # Perception layer. Watches file system via
                       # notify::RecommendedWatcher, detects changed files,
                       # manages worktree state, stages and commits.
  gate_ext.rs          # Action layer. Runs the multi-rung gate pipeline
                       # (compile, test, clippy, symbol, generated-test,
                       # LLM-judge) during the VERIFY step.
  conductor_ext.rs     # Recovery layer. Monitors gamma summaries via
                       # roko-conductor's StuckDetector, manages the
                       # circuit breaker, emits CognitiveSignals for
                       # stuck/thrashing/ghost-turn detection.
```

### `roko-ext-chain` (new)

Extensions for Korai blockchain agents:

```
crates/roko-ext-chain/src/
  lib.rs
  chain_subscriber_ext.rs   # Perception layer. Subscribes to new blocks
                             # via alloy provider, reads transactions,
                             # writes gas_gwei to CorticalState, emits
                             # NewBlock events on the fabric.
  risk_ext.rs                # Cognition layer. Computes position risk
                             # metrics, tracks exposure, evaluates
                             # liquidation proximity, contributes PE.
  mortality_ext.rs           # Affect layer. Reads ISFR (Intertemporal
                             # Survival Function Ratio) from Korai,
                             # modulates behavioral state based on
                             # stochastic survival probability.
  isfr_ext.rs                # Perception layer. Queries the Korai
                             # mortality precompile for current ISFR,
                             # writes stochastic_survival to CorticalState.
  clearing_ext.rs            # Action layer. Participates in Korai
                             # marketplace auctions for knowledge and
                             # compute resources.
```

### `roko-ext-research` (new)

Extensions for knowledge research agents:

```
crates/roko-ext-research/src/
  lib.rs
  knowledge_graph_ext.rs   # Memory layer. Builds and queries a local
                           # knowledge graph. Tracks entity relationships,
                           # citation chains, and contradiction detection.
                           # Bids knowledge entries into the workspace.
  source_watcher_ext.rs    # Perception layer. Monitors external sources
                           # (arxiv feeds, API endpoints, watched URLs)
                           # for new content. Emits observations when
                           # sources update.
  synthesis_ext.rs         # Cognition layer. Synthesizes findings from
                           # multiple sources into coherent summaries.
                           # Detects gaps and contradictions. Generates
                           # research questions for the next cycle.
```

### Dependency graph

```
roko-ext-chain ──┐
roko-ext-code ───┼──► roko-ext-core ──► roko-runtime ──► roko-primitives
roko-ext-research┘         │
                           ├──► roko-daimon
                           ├──► roko-dreams
                           ├──► roko-learn
                           ├──► roko-neuro
                           ├──► roko-gate
                           └──► roko-compose
```

The `roko-ext-*` crates depend on `roko-ext-core` for the base extension set and on `roko-runtime` for the `Extension` trait, `CorticalState`, `EventFabric`, and agent types. Domain-specific crates (`roko-gate`, `roko-neuro`, etc.) are dependencies of the extension crates, not of the runtime itself. This keeps the runtime layer clean and domain-agnostic.

---

## Migration path

Phase 1: **Extension trait and chain builder** in `roko-runtime`. No behavioral change. Existing code continues using the orchestrator.

Phase 2: **Core extensions** in `roko-ext-core`. Port existing heartbeat, affect, learning, and dream logic from `orchestrate.rs` into extensions.

Phase 3: **Agent<Phase> type-state** and `agent_main_loop`. Wire the persistent agent loop alongside the existing PlanRunner. Both paths coexist.

Phase 4: **Domain extensions**. Port gate, conductor, chain, and research logic into extension crates.

Phase 5: **PlanRunner migration**. Switch PlanRunner from spawn-execute-die to task injection into persistent agents. The transitional `spawn_and_run_task` API handles the overlap.

Phase 6: **Orchestrator slimming**. Move remaining logic out of `orchestrate.rs`. The file shrinks from 19K lines to a thin PlanRunner coordinator.

Each phase ships independently. No phase breaks existing functionality.

---

## 13. Unified narrative: one agent tick, end to end

The sections above describe individual subsystems. This section traces one complete agent tick through all of them, from boot to dream, showing how every component connects. This is the definitive flow -- if a subsystem is not mentioned here, it does not participate in the runtime path.

### Phase 0: Agent boot

Boot happens once per agent lifetime (or once per resume from suspended state). The sequence:

1. **Load extensions by layer order (L0 through L7).** The `ExtensionChainBuilder` validates dependencies, detects cycles, and produces a topologically sorted firing order. Foundation extensions (heartbeat clock, event fabric, CorticalState initialization) load first. Recovery extensions (circuit breaker, compensation) load last.

2. **Connect chain actors.** For blockchain agents, each target chain gets a `ChainActor` spawned as a Tokio task. The ChainActor opens a `ChainConnector` (alloy for EVM, native RPC for Korai), subscribes to relevant event streams (new blocks, specific contract logs, price feeds), and begins translating raw chain events into `CanonicalEvent` format. Multi-chain agents run one ChainActor per chain. All actors feed into the same `CanonicalEventBus`.

3. **Initialize WorldGraph.** The WorldGraph starts empty (or loads from `.roko/state/{agent_id}/world-graph.json` if resuming). During the first few ticks, OBSERVE populates it with entities discovered from chain subscriptions, file system scans, and task descriptions.

4. **Load ForagingModel.** Load persisted Gittins indices from `.roko/state/{agent_id}/foraging.json`, or initialize with uniform indices (equal exploration probability across all information patches). The ForagingModel governs retrieval budget allocation during every RETRIEVE step.

5. **Initialize CorticalState atomics.** All 32+ atomic fields set to default values. PAD vector to neutral (0.0, 0.0, 0.0). Prediction error to 0.5 (moderate uncertainty -- the agent does not yet know its environment). Economic vitality to the configured budget fraction. Regime to Normal.

6. **Call `on_activate` on every extension.** Extensions open connections (NeuroStore handle, ToolRegistry, chain RPC), start background tasks (file watchers, event subscribers), and load initial state from disk.

7. **Enter `Agent<Active>` state.** The type-state transition consumes `Agent<Provisioning>` and returns `Agent<Active>`. The heartbeat loop begins.

### Phase 1: OBSERVE (gamma tick, every 5-120s)

The perception phase. Every extension with an `observe` hook contributes observations to the workspace.

1. **ChainSubscriberExt** (Perception layer): Drain the `CanonicalEventBus` for new blocks and transactions received since the last tick. For each block: extract gas price, write `gas_gwei` to CorticalState, emit `NewBlock` event on the EventFabric. For each relevant transaction: classify by type (transfer, swap, liquidation, oracle update), add to the observation batch.

2. **FileWatcherExt** (Perception layer): Check the `notify::RecommendedWatcher` queue for file change events. Group changes by directory. Emit `FileChanged` events. For coding agents, this is the primary perception source -- changed test files, modified source, new compilation errors.

3. **ForagingModel update**: Adjust attention budgets per entity based on Gittins indices. Entities that produced high-salience observations in recent ticks get increased budgets. Entities that produced nothing get decreased budgets. This is not a separate extension -- it runs inside the heartbeat pipeline before the RETRIEVE step.

4. **ContractRegistry** (Perception layer, chain agents only): Classify any new addresses discovered in transactions. Is this a known DEX router? A lending pool? An unknown contract? Classification feeds into WorldGraph entity typing.

5. **WorldGraph update**: Add new entities (addresses, files, contracts) and relationships (caller-callee, dependency, ownership) discovered during observation. The graph is append-only within a tick; pruning happens during delta consolidation.

6. **CorticalState perception signals**: Update `last_observation_ms`, increment `tick_count`, write perception signals (observation count, novelty estimates, event density). These signals are visible to all extensions in subsequent steps.

### Phase 2: RETRIEVE

The knowledge retrieval phase. Extensions query stores and bid results into the CognitiveWorkspace.

1. **NeuroExt** (Memory layer): Compute the HDC fingerprint of the current task + observations. Query the local NeuroStore by Hamming distance. Retrieve the top-K entries (Insights, Heuristics, Warnings, CausalLinks, StrategyFragments). Each entry becomes a `ContextCandidate` with a bid value proportional to its similarity score and validation tier.

2. **ChainKnowledgeExt** (Memory layer, chain agents only): Query Korai's InsightStore via the HTC precompile. The query vector is the same HDC fingerprint used for local retrieval. Results are cross-agent knowledge -- entries deposited by other agents on the network. Chain RPC latency: ~100ms, cached per task.

3. **PlaybookExt** (Memory layer): Query the playbook store for patterns matching the current task type, domain, and context features. Playbooks contain proven strategies: which context sections to prioritize, which model routing worked, which tool sequences succeeded. Results bid into the workspace with high priority (playbooks are validated patterns).

4. **WorldGraph contribution**: Relevant entity context from the WorldGraph is packaged as context candidates. If the current task involves a specific contract address, the WorldGraph provides that address's type, known interactions, associated warnings, and relationship graph.

5. **ForagingModel stopping rule**: The Marginal Value Theorem governs when retrieval stops. For each information patch (NeuroStore, InsightStore, PlaybookStore, WorldGraph), the ForagingModel tracks marginal return -- the salience gain from the last retrieval. When marginal return for a patch drops below the average return across all patches, retrieval from that patch stops. This prevents over-retrieval (wasting tokens) and under-retrieval (missing context).

6. **VCG attention auction**: All context candidates from all extensions compete for limited token budget. Eight bidders (Neuro, Daimon affect state, iteration memory, code intelligence, playbook rules, research artifacts, task context, oracle predictions) submit bids. Winners pay the second-highest price. The auction assembles the final context window. Section ordering optimizes for prefix cache reuse (stable sections first, volatile sections last).

### Phase 3: ANALYZE

Compute prediction error (PE) -- the scalar that drives tier selection.

1. **Per-extension PE components**: Each extension with an `analyze` hook returns a PE contribution in [0.0, 1.0].
   - **GateExt**: PE based on recent gate pass rate. Many recent failures = high PE (the agent is in unfamiliar territory). Consistent passes = low PE.
   - **ChainSubscriberExt**: PE based on price deviation from forecast, gas spike magnitude, transaction volume anomaly.
   - **FileWatcherExt**: PE based on number of changed files, presence of compilation errors, test failure rate.
   - **DaimonExt**: PE contribution from affect state. High arousal and low dominance increase PE (the agent is stressed and uncertain).

2. **Aggregation**: PE components are weighted and combined into a single scalar. Default weights are equal; the weights adapt based on which components predicted tier escalation that proved justified (measured by outcome quality at the selected tier).

3. **Somatic marker check**: Before proceeding to GATE, query the k-d tree over the 8-dimensional strategy space. If past outcomes in similar situations were negative (low pleasure, high arousal), add a caution signal that biases toward higher tiers. If past outcomes were positive, add a confidence signal that biases toward lower tiers. Lookup latency: <100 microseconds.

4. **CorticalState update**: Write the aggregate PE to `prediction_error`. Write per-component PE values to category slots. Update `surprise_rate` (fraction of recent ticks exceeding the PE threshold).

### Phase 4: GATE

Select the cognitive tier for this tick.

1. **CognitiveGate evaluation**: 16 deterministic probes run. Each probe checks a CorticalState signal (memory hit rate, tool call novelty, prediction residual, context divergence, task complexity, affect state, economic vitality, knowledge freshness, gate pass rate, episode similarity, and six domain-specific signals). The probes compute a composite score.

2. **Tier selection with adaptive thresholds**: The composite score maps to a tier via EWMA thresholds that persist to `.roko/learn/gate-thresholds.json`. Thresholds adapt based on outcome quality at each tier -- if T1 decisions frequently fail (requiring T2 escalation), the T0-T1 boundary moves lower, routing more ticks to T1.

3. **If T0**: Suppress the standard loop. Run deterministic pattern matching, update internal counters, execute any cached heuristics that apply. Skip to Phase 7 (REFLECT). Total tick time: <10ms. Cost: $0.

4. **If T1/T2**: Continue to Phase 5. T1 uses a cheap model (Haiku-class) with minimal context. T2 uses a frontier model (Opus-class) with the full CognitiveWorkspace.

### Phase 5: ASSEMBLE + INFER (T1/T2 only)

Build the prompt, call the model, execute tool calls.

1. **CognitiveWorkspace finalization**: The workspace from Phase 2 is finalized for the selected tier. T1 gets a compressed workspace (highest-priority sections only, ~4K tokens). T2 gets the full workspace (~32K tokens or more, depending on model).

2. **VCG auction replay (if T2)**: For T2 ticks, the auction runs a second pass with expanded budget. Sections that lost in the first pass get a second chance. This two-pass approach means T1 always gets the most important context, and T2 gets breadth.

3. **ContextPolicy enforcement**: Learnable allocation rules apply. If section X has a negative section effect (its presence correlates with worse outcomes), its bid weight is reduced. If section Y has a strong positive effect, its weight is increased. The policy updates after every theta cycle based on outcome data.

4. **Inference Gateway cache check**:
   - **L3 (deterministic)**: SHA-256 hash of the assembled prompt. If exact match exists in L3 cache, return the cached response. Hit rate: ~10%. Savings: 100% of LLM cost.
   - **L2 (semantic)**: If L3 misses, compute embedding of the prompt and check L2 cache for similarity >0.92. If match, return cached response with confidence adjustment. Hit rate: ~30% of L3 misses. Savings: 100% of LLM cost.
   - **L1 (prefix)**: If L2 misses, check whether the prompt shares a prefix with a recent request to the same provider. If so, the provider's KV cache can reuse the shared prefix. Savings: ~90% of input token cost for the shared portion.
   - **L0 (backend call)**: On full cache miss, route to the provider. Intent-based routing resolves which provider handles the call. Subsystems declare needs: model quality tier, latency bound, cost sensitivity. The resolver matches against available providers (Anthropic, OpenAI, Google, Ollama, HuggingFace) using first-match-wins.

5. **Translator pattern**: The response from any provider is normalized into a common format. `AnthropicBlocks` handles Claude's content block format. `OpenAiJson` handles OpenAI's tool-call format. `GeminiNative` handles Gemini's function-calling format. `ReActText` handles models without native function calling by parsing ReAct-style text output.

6. **Tool loop**: Parse tool calls from the model response. For each tool call:
   - Check somatic markers for the proposed action. If strong negative signal, log a warning and optionally escalate.
   - Route through `SafetyLayer::authorize_call_with_taint` for role authorization, pre-checks, and delegation caveat enforcement.
   - Execute through `ToolDispatcher`.
   - Feed results back to the model for the next turn.
   - Continue until the model produces a final response or the turn limit is reached.

7. **Cost tracking**: Record per-turn token usage, model cost, cache hit/miss, latency. Update `economic_vitality` on CorticalState.

8. **Mortality integration**: For chain agents with mortality clocks, dying agents (low `stochastic_survival`) increase `cost_sensitivity` in their inference intent, causing the gateway to prefer cheaper models for non-critical subsystems. The agent conserves resources as its survival probability decreases.

### Phase 6: VERIFY (T1/T2 only)

Confirm the outcome.

1. **Domain-specific gate pipeline**: The gate pipeline from `roko-gate` runs against the action outcomes. For coding tasks: CompileGate (does it build?), TestGate (do tests pass?), ClippyGate (any lint warnings?), SymbolGate (are all references resolved?), GeneratedTestGate (do generated tests cover the change?), LLMJudgeGate (does a reviewer model approve?). For chain tasks: TxSimGate (does the transaction succeed on a fork?), MEVGate (is the transaction vulnerable to sandwich attacks?).

2. **Gate result recording**: Each gate verdict (pass/fail, details, latency) is recorded in the `DecisionCycleRecord`. Gate failures update PE for the next tick (failed verification = high surprise = route to higher tier next time).

3. **Compensation on failure**: If verification fails, the `on_compensation` hook fires. Domain-specific: coding agents run `git checkout -- <file>` to revert bad patches; chain agents flag positions for review. Compensation runs between VERIFY and REFLECT.

### Phase 7: REFLECT (every tick, including T0)

The learning phase. Runs unconditionally.

1. **Build DecisionCycleRecord**: Capture everything that happened this tick: tier selected, PE components, context sections used, model called (if any), tool calls made, gate results, latency breakdown, cost.

2. **HDC fingerprint**: Compute a 10,240-bit HDC fingerprint for this episode using bind/bundle/permute over: task type, observation hashes, action types, outcome quality. The fingerprint enables similarity search across episodes.

3. **Episode clustering update**: If the episode batch has reached the clustering threshold (~50 episodes), run k-medoids clustering in the background. Cluster labels attach to the CascadeRouter's context features, enabling the router to recognize "I have seen situations like this before" and route to the model that performed best on the cluster.

4. **Resonance detection**: Check Lotka-Volterra dynamics across subsystems. If two subsystems show correlated prediction error patterns (resonance), strengthen the coupling between their context contributions. Cross-domain resonance -- a rate signal correlating with a code quality signal -- gets flagged for investigation.

5. **CascadeRouter update**: Feed the outcome (gate pass/fail, cost, latency) back to the CascadeRouter as a reward signal. Thompson sampling updates the posterior distribution for the model used. Over time, the router converges on the best model for each task-context cluster.

6. **Adaptive threshold update**: Update EWMA gate thresholds based on whether the selected tier was appropriate. If T1 was selected but the task failed and required T2 escalation, adjust the T0-T1 boundary. Thresholds persist to `.roko/learn/gate-thresholds.json`.

7. **Efficiency event emission**: Write a per-tick efficiency record to `.roko/learn/efficiency.jsonl`: tick number, tier, cost, latency, gate pass/fail, cache hit/miss.

8. **Episode logging**: Write the full `DecisionCycleRecord` to `.roko/episodes.jsonl`. Include the HDC fingerprint. This is the raw training data for the fine-tuning loop (Stream C).

9. **EventFabric broadcast**: Emit a `HeartbeatTick` event with the tick summary. The TUI dashboard, HTTP control plane, and any subscribed extensions receive it.

10. **Sleep pressure check**: Increment sleep pressure by the configured rate. If pressure exceeds the threshold AND no active tasks remain, transition to Phase 8 (DREAM). Otherwise, the tick is complete. Wait for the next gamma interval.

### Phase 8: DREAM (delta cycle, triggered by sleep pressure)

The consolidation phase. Runs when the agent is idle and sleep pressure has accumulated.

1. **State transition**: `Agent<Active>` transitions to `Agent<Dreaming>`. The heartbeat loop pauses. Only emergency wakeup events can interrupt the dream cycle.

2. **NREM replay (Mattar-Daw prioritized)**: Episodes are prioritized for replay based on: (a) prediction error magnitude (high surprise = high replay priority), (b) recency (newer episodes replay first), (c) outcome quality (both very good and very bad outcomes replay -- the extremes are most informative). For each replayed episode, the agent re-evaluates: Was the tier selection correct? Was the context allocation optimal? Did the model routing make sense given the outcome?

3. **REM imagination (counterfactual generation)**: For high-value episodes, generate counterfactuals. "What if I had used T2 instead of T1 on that task?" "What if I had included the research context that lost the auction?" Counterfactuals are evaluated against recorded outcomes. If a counterfactual strategy would have produced better results, it is flagged as a learning signal.

4. **Threat rehearsal**: Replay episodes where gate verification failed or somatic markers fired. Extract patterns from failures. Populate the k-d tree with new somatic markers: this combination of task features + context state + action type led to failure. Future ticks hitting similar coordinates get the caution signal. Latency of future lookups: <100 microseconds.

5. **Staging buffer and knowledge promotion**: Dream outputs (replayed insights, counterfactual lessons, extracted patterns) enter a staging buffer at low confidence (0.20-0.30). The staging buffer is separate from the main NeuroStore. Only insights that reach 0.70 confidence through live validation in subsequent active sessions are promoted to permanent memory. This prevents hallucinated dream insights from corrupting the knowledge base.

6. **WorldGraph consolidation**: Prune entities not referenced in recent episodes. Strengthen edges between entities that co-occur in successful outcomes. Generate hypotheses about unobserved connections (entities in similar clusters that have not been directly linked). Strategy evolution: if the WorldGraph shows that certain entity patterns precede success, encode that pattern as a StrategyFragment.

7. **Korai InsightStore publication**: For agents connected to Korai, promote validated knowledge entries to the on-chain InsightStore. Entries are PP-HDC encoded (privacy-preserving) before publication -- the structural pattern is shared without revealing raw proprietary data. Other agents on the network can retrieve these entries via Hamming distance search.

8. **Sleep pressure reset**: Set sleep pressure to 0.0. Transition back to `Agent<Active>`. The heartbeat loop resumes. The agent starts the next session with distilled, validated lessons from its own experience.

---

## 14. Performance targets

Every operation in the cognitive architecture has a latency budget. The architecture is designed so that the overhead of gating, context assembly, and learning is negligible compared to LLM call time.

### Per-operation latency targets

| Operation | Target latency | Where it runs | Notes |
|-----------|---------------|---------------|-------|
| HDC fingerprint (bind/bundle/permute) | ~5 microseconds | REFLECT step | 10,240-bit vectors, POPCNT-based |
| Somatic marker lookup (k-d tree) | <100 microseconds | ANALYZE step, before GATE | 8-dimensional strategy space, ~10K entries |
| CorticalState read (single atomic) | <50 nanoseconds | Every step | Lock-free, cache-line aligned |
| CorticalState snapshot (all fields) | <500 nanoseconds | REFLECT step | 32 atomic reads, acquire ordering |
| Habituation mask check | <1 microsecond | OBSERVE step | Blake3 hash lookup in HashMap |
| Attention salience decay | <10 microseconds | RETRIEVE step | Binary min-heap, ~100 entries |
| VCG auction (8 bidders) | <100 microseconds | RETRIEVE step | Sealed-bid computation, allocation |
| Prediction error aggregation (16 probes) | <50 microseconds | ANALYZE step | Weighted sum of cached values |
| Tier gate selection | <10 microseconds | GATE step | EWMA threshold comparison |
| ForagingModel budget update | <100 microseconds | OBSERVE/RETRIEVE | Gittins index recalculation |
| Local NeuroStore query (Hamming distance) | ~10 milliseconds | RETRIEVE step | HDC search over ~10K local entries |
| Chain RPC query (cached per task) | ~100 milliseconds | RETRIEVE step | Network round-trip to Korai HTC precompile |
| Inference Gateway L3 check (SHA-256) | <50 microseconds | ASSEMBLE step | Hash computation + cache lookup |
| Inference Gateway L2 check (embedding) | ~5 milliseconds | ASSEMBLE step | Embedding computation + similarity search |
| Episode clustering (k-medoids) | ~50 milliseconds | Background, every 50 episodes | Not on hot path. Runs async. |
| WorldGraph entity lookup | <1 millisecond | OBSERVE/RETRIEVE | In-memory graph traversal |
| Resonance detection (Lotka-Volterra) | <10 milliseconds | REFLECT step | Differential equation step over ~10 signal pairs |

### Per-tier total tick latency

| Tier | Latency target | Composition | Cost |
|------|---------------|-------------|------|
| T0 | <10 milliseconds | CorticalState reads + probe evaluation + pattern match + reflect | $0 |
| T1 | ~500 milliseconds | T0 overhead + VCG auction + L3/L2/L1 cache check + Haiku-class LLM call (~200ms) + reflect | ~$0.001 |
| T2 | ~3-5 seconds | T0 overhead + full VCG auction + cache check + Opus-class LLM call (2-30s) + tool loop + gate pipeline + reflect | $0.01-$0.10 |

### Overhead as percentage of LLM call time

The cognitive architecture's total overhead per T1/T2 tick: ~15-25 milliseconds (all operations before and after the LLM call, excluding the call itself). A T1 call takes ~200ms. A T2 call takes 2-30 seconds.

| Tier | Architecture overhead | LLM call time | Overhead as % of call |
|------|---------------------|---------------|----------------------|
| T0 | ~8ms | $0 (no call) | N/A -- the overhead IS the computation |
| T1 | ~15ms | ~200ms | ~7.5% |
| T2 | ~25ms | 2,000-30,000ms | <1.25% (often <0.1%) |

At T2, the cognitive architecture adds less than 0.1% overhead to the LLM call time. The gating, context assembly, and learning machinery is effectively free relative to the model cost it saves.

### Expected savings from Inference Gateway

The three cache layers compound savings:

| Layer | Hit rate | Savings per hit | Net savings |
|-------|---------|----------------|-------------|
| L3 (deterministic) | ~10% of T1/T2 calls | 100% (full response cached) | 10% |
| L2 (semantic) | ~30% of L3 misses (~27% of total) | 100% (similar response cached) | 27% |
| L1 (prefix) | ~90% of remaining calls | ~60% of input tokens (shared prefix) | ~38% of remaining cost |

Combined: L3 eliminates 10%. L2 eliminates 27% of the remainder. L1 saves ~60% of input tokens on the remaining 63%. Aggregate savings: 60-80% on top of cognitive gating's 3-5x cost reduction.

A naive harness making 1,000 Opus-class calls at $0.05 each spends $50. Roko with cognitive gating routes 800 to T0 ($0), 150 to T1 ($0.15), and 50 to T2. The Inference Gateway catches ~37% of those 200 calls in L3/L2 cache ($0), and saves ~60% of input tokens on the rest. Effective cost: ~$3-5, a 10-15x reduction from the naive baseline.

---

## 15. Inference Gateway

The Inference Gateway sits between the cognitive architecture's tier selection and the actual LLM provider. Its job: never make the same expensive call twice, and always route to the cheapest provider that meets the subsystem's quality requirements.

### Three cache layers

```
Request arrives from ASSEMBLE step
    |
    v
L3: Deterministic cache (SHA-256 exact match)
    |-- HIT (~10%): return cached response immediately
    |-- MISS: continue to L2
    |
    v
L2: Semantic cache (embedding similarity >0.92)
    |-- HIT (~30% of L3 misses): return similar response with confidence adjustment
    |-- MISS: continue to L1
    |
    v
L1: Prefix cache (provider KV reuse)
    |-- Compute shared prefix with recent requests to same provider
    |-- If shared prefix: flag for KV reuse (saves ~90% of shared input tokens)
    |-- Continue to L0
    |
    v
L0: Backend call (intent routing to optimal provider)
    |-- Resolve intent against available providers
    |-- Apply translator pattern for response normalization
    |-- Return response, populate L3 and L2 caches
```

**L3: Deterministic cache.** SHA-256 hash of the full assembled prompt (system prompt + context window + user message). If the exact same prompt has been sent before, the cached response is returned. This catches repeated patterns: the same lint warning across files, the same compilation error across iterations, the same chain query across ticks. Hit rate depends on task repetitiveness. Coding agents doing iterative fixes see higher L3 hit rates than research agents exploring novel topics.

Cache key: `SHA256(system_prompt || context_sections || user_message || tool_state)`. Cache value: the full model response (text + tool calls). TTL: 1 hour by default, configurable per domain. Invalidation: any change to the knowledge store that would affect the response triggers cache busting via a bloom filter of relevant HDC fingerprints.

**L2: Semantic cache.** When L3 misses, compute an embedding of the prompt and check L2 for similar prompts (cosine similarity >0.92). "Similar" means the structural pattern is the same even if surface details differ. "Fix the lint warning in `auth.rs` line 42" and "Fix the lint warning in `auth.rs` line 57" are semantically similar and likely have structurally similar responses.

The response from L2 comes with a confidence discount: the original confidence is multiplied by the similarity score. If the cached response had 0.95 confidence and the similarity is 0.93, the returned confidence is 0.88. Downstream consumers can decide whether to accept the response or request a fresh call.

Embedding model: a small, fast model (all-MiniLM-L6-v2 or similar) running locally. Embedding latency: ~5ms. The embedding model is not an LLM call -- it is a local inference with a 22M parameter model.

**L1: Prefix cache.** Most LLM providers cache the KV (key-value) states of recently processed prompts. If two consecutive requests share a long common prefix (same system prompt, same context sections), the provider can reuse the cached KV states for the shared portion and only process the new tokens. Roko exploits this by ordering context sections to maximize prefix stability: system prompt first (never changes), domain context second (changes rarely), task context third (changes per task), observation context last (changes per tick).

L1 is not a Roko-side cache -- it is provider-side KV reuse. Roko's contribution is ordering sections to maximize the shared prefix length. For an agent doing iterative coding (many turns on the same task), L1 can save 80-90% of input token cost.

### Intent-based routing

Subsystems do not choose a specific model. They declare an intent:

```rust
/// An inference intent declares what a subsystem needs from the model.
/// The gateway resolves intents to specific providers using
/// first-match-wins against available provider configurations.
pub struct InferenceIntent {
    /// Minimum quality tier: Draft, Standard, Frontier, Reasoning.
    pub quality: QualityTier,
    /// Maximum acceptable latency.
    pub latency_bound: Duration,
    /// Cost sensitivity: 0.0 (cost-insensitive) to 1.0 (minimize cost).
    pub cost_sensitivity: f32,
    /// Whether the response needs tool-calling support.
    pub needs_tools: bool,
    /// Whether the response needs structured output (JSON mode).
    pub needs_structured: bool,
    /// Optional: specific capabilities required (vision, code, math).
    pub capabilities: Vec<Capability>,
}
```

The resolver maintains a ranked list of providers with their capabilities. Resolution is first-match-wins: iterate the list, find the first provider that satisfies all intent constraints, route to it. If no provider matches, relax constraints in priority order: latency first, then cost, then quality.

### Translator pattern

Different providers return responses in different formats. The translator pattern normalizes them:

```rust
/// Translate a provider response into Roko's internal format.
pub trait ResponseTranslator: Send + Sync {
    /// Parse the raw provider response into tool calls and text.
    fn translate(&self, raw: &[u8]) -> Result<TranslatedResponse>;
}

/// Four built-in translators.
pub enum BuiltinTranslator {
    /// Claude's content block format (text blocks + tool_use blocks).
    AnthropicBlocks,
    /// OpenAI's tool_calls array in the assistant message.
    OpenAiJson,
    /// Gemini's functionCall parts.
    GeminiNative,
    /// ReAct-style text parsing for models without native tool calling.
    /// Parses "Thought: ... Action: ... Action Input: ..." patterns.
    ReActText,
}
```

The `ReActText` translator enables using any text-generation model (including fine-tuned open-source models from HuggingFace) as an agent backend. The model outputs ReAct-formatted text; the translator parses it into structured tool calls. This is how the fine-tuning loop (Stream C) connects: fine-tuned models that do not support native function calling still work through ReActText translation.

### Mortality integration

For chain agents with mortality clocks, the Inference Gateway adjusts routing based on survival probability:

- **Healthy agent** (stochastic_survival > 0.7): Normal routing. Intent constraints applied as declared.
- **Declining agent** (stochastic_survival 0.3-0.7): Cost sensitivity increases by `(1.0 - survival) * 0.5`. Non-critical subsystems (telemetry, exploratory research) downgrade to Draft quality.
- **Dying agent** (stochastic_survival < 0.3): Maximum cost sensitivity. All non-essential inference suppressed. Only critical safety checks and final state persistence get Frontier quality.

This ensures that dying agents do not waste resources on diminishing-return inference. Their remaining budget is spent on wrapping up cleanly and persisting what they learned.

---

## 16. Updated Extension trait: EventPayload additions

The `EventPayload` enum in section 9 requires two additional variants for multi-chain and economic integration. The updated enum includes `ISFRUpdate` and `ClearingResult` (already shown in section 9) plus the following additions to support WorldGraph and multi-chain actors:

```rust
/// Additional EventPayload variants for multi-chain and WorldGraph integration.
///
/// These extend the EventPayload enum defined in section 9.
/// They enable chain actors and the WorldGraph to participate
/// in the event fabric alongside existing event types.

/// A canonical event from any chain, translated by a ChainActor.
/// This is the chain-agnostic bridge between raw blockchain events
/// and the extension system.
CanonicalChainEvent {
    /// Source chain identifier.
    chain_id: u64,
    /// Chain-specific block number.
    block_number: u64,
    /// Event type classification.
    event_type: CanonicalEventType,
    /// Involved addresses (normalized to lowercase hex).
    addresses: Vec<String>,
    /// Event-specific data.
    data: serde_json::Value,
},

/// A WorldGraph entity was discovered or updated.
/// Emitted during OBSERVE when new entities enter the graph.
WorldGraphUpdate {
    /// The entity identifier (address, file path, etc.).
    entity_id: String,
    /// Entity type classification.
    entity_type: EntityType,
    /// New relationships added this tick.
    new_edges: Vec<(String, RelationshipType)>,
},

/// A ChainConnector status change (connected, disconnected, degraded).
/// Emitted by ChainActors when RPC connection state changes.
ChainConnectorStatus {
    /// Which chain this connector serves.
    chain_id: u64,
    /// New connection status.
    status: ConnectorStatus,
    /// Latency to the RPC endpoint (if connected).
    latency_ms: Option<u64>,
},
```

The `EventCategory` enum gains a corresponding `WorldGraph` variant:

```rust
/// Additional event category for WorldGraph events.
WorldGraph,
```

Extensions that need multi-chain awareness subscribe to `EventCategory::Chain` (which now includes `CanonicalChainEvent`) and `EventCategory::WorldGraph`. The `ChainConnector` reference is passed to chain extensions during `on_activate` via the `ActivateContext`:

```rust
/// Extended ActivateContext with ChainConnector access.
pub struct ActivateContext {
    pub cortical: Arc<CorticalState>,
    pub event_fabric: Arc<EventFabric>,
    pub domain: &DomainProfile,
    /// Chain connectors for multi-chain agents. Empty for non-chain domains.
    pub chain_connectors: Vec<Arc<dyn ChainConnector>>,
    /// WorldGraph reference for entity tracking.
    pub world_graph: Arc<RwLock<WorldGraph>>,
}
```

This ensures that chain extensions can access their connectors from activation through termination, and that the WorldGraph is available to any extension that needs entity context.
