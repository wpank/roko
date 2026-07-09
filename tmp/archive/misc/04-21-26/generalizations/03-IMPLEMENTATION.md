# Implementation roadmap: current state, gaps, and migration path

This document is an honest assessment of where Roko stands today, what the architecture specification (01-ARCHITECTURE.md) requires, and how to get from one to the other without breaking what already works.

---

## 1. Current state audit

Roko has 18 crates totaling ~177K lines of Rust. The system runs, self-hosts, and executes complex multi-step plans end to end. But it has structural problems that the new architecture intends to solve.

### Working and useful

These subsystems are production-grade, battle-tested through thousands of self-hosting runs.

**Core trait system (roko-core)**

The 1-noun-6-verb architecture is clean and domain-agnostic:
- `Engram` (the universal signal type): content-addressed, typed, timestamped
- `Substrate`: read/write persistence
- `Scorer`: relevance ranking
- `Gate`: pass/fail verification
- `Router`: model/strategy selection
- `Composer`: prompt/context assembly
- `Policy`: constraint enforcement

These traits survive intact into the new architecture. They define the vocabulary that all higher-level code speaks.

**Agent dispatch (roko-agent)**

Six production LLM backends with a native ToolLoop for API backends:
- Claude CLI (subprocess-based, full MCP support)
- Claude API (HTTP, streaming, tool_use native)
- OpenAI-compatible (HTTP, function_calling)
- Gemini (HTTP, context caching, function declarations)
- Ollama (local inference, streaming)
- Perplexity (search-augmented generation)

The `ToolLoop` handles multi-turn tool calling natively for API backends -- the agent dispatches, gets tool_use blocks, executes tools, and feeds results back in a loop until the model produces a final response. This is the single most critical piece of infrastructure.

Also: `MultiAgentPool` for warm-agent reuse, `SafetyLayer` for pre/post-call authorization, `TaskRunner` for anomaly detection and budget guardrails during execution.

**Gate pipeline (roko-gate)**

11 gate types across a 7-rung pipeline:
- Compile gate, test gate, clippy gate (standard Rust verification)
- Generated test gate (agent-written tests, gated by the pipeline itself)
- LLM judge gate (quality/correctness scoring via cheap model)
- Symbol gate (exported API surface verification)
- Shell gate (arbitrary command execution)
- Search oracle, diff gate, security gate, format gate

Adaptive thresholds via EMA + CUSUM change detection. Gate ratchets prevent rung regression. Artifact store for content-addressed gate outputs.

**Learning runtime (roko-learn)**

- Episodes: full turn-by-turn records with tool calls, costs, outcomes
- Playbooks: extracted reusable recipes from successful episodes
- Cascade router: 3-level model selection (static rules -> confidence intervals -> LinUCB bandit)
- Prompt experiments: A/B testing of system prompt variants with statistical significance
- Section effectiveness: per-category attribution tracking that feeds the VCG auction
- Skill library: successful task patterns extracted and reusable as context
- Efficiency events: per-turn cost/quality/latency telemetry

**HDC vectors (roko-primitives)**

10,240-bit hyperdimensional computing vectors for similarity search and fingerprinting. Used for episode deduplication, pattern matching, and content-addressed storage. Binary operations (XOR, popcount) run at memory bandwidth.

**Daimon affect model (roko-daimon)**

ALMA (Arousal, Liking, Morality, Agency) model with:
- Somatic markers: past negative outcomes create visceral aversion to similar situations
- Strategy coordinates: Thompson sampling over learned approaches
- Goals: priority-weighted objectives that modulate dispatch

Queried at dispatch time in orchestrate.rs. Modulates model routing, prompt tone, and risk tolerance. The DaimonState participates in every agent dispatch decision.

**Knowledge store (roko-neuro)**

6 entry kinds: Heuristic, AntiKnowledge, Insight, Reflection, Strategy, Fact. Tier progression (Candidate -> Established -> Core) based on repeated validation. Queried per-task for strategy fragments that get injected into system prompts.

**Dream consolidation (roko-dreams)**

Full cycle: replay (re-examine recent episodes) -> imagination (hypothetical scenarios) -> rehearsal (simulated execution) -> staging (candidate promotions) -> promotion (knowledge tier advancement). Wired to fire at plan boundaries.

**Conductor (roko-conductor)**

10 anomaly watchers:
- Loop detection, cost spike, quality drift, latency anomaly
- Error rate, retry storm, context bloat, stuck detection
- Pattern compound detection, health degradation

Circuit breaker with half-open state for progressive recovery. Stuck detector with meta-cognition hooks. Fully wired into the plan execution loop.

**VCG auction for context bidding**

Three bidder types (Neuro/Task/Research) compete for context token budget via Vickrey-Clarke-Groves mechanism. Section effect tracking measures post-hoc whether included sections correlated with success. Budget allocation evolves over time.

**Plugin system**

Event source plugins, feedback collector plugins, manifest-based discovery. Agents can consume external event streams and push feedback to external systems.

### Built but underutilized

These subsystems exist and function but do not reach their potential because they lack the runtime architecture to support them.

**Event bus: broadcasts without consumers**

`RuntimeEventBus<RokoEvent>` works. It broadcasts gate verdicts, plan revisions, lifecycle transitions. But no agent subscribes to these events reactively. The bus fires into the void. The TUI and HTTP API consume events -- agents do not.

What the architecture requires: agents subscribing via `EventFabric`, receiving events in their tick loop, contributing to prediction error calculation.

**Frequency tracking: metadata without behavior**

Gamma/theta/delta frequencies are tracked as metadata in `HeartbeatClock`. The heartbeat fires at configured intervals. But the frequencies do not drive differentiated behavior -- a gamma tick and a theta tick run the same code path. There is no "observe-only" fast path or "consolidate" slow path.

What the architecture requires: three distinct tick types with different extension hook profiles. Gamma fires `on_observe` only. Theta fires the full 9-step pipeline. Delta triggers dream consolidation.

**Dreams: triggered manually, not by sleep pressure**

`DreamRunner` works. It replays episodes, runs imagination, promotes knowledge. But it fires only at plan end (or manual trigger), not when sleep pressure accumulates. There is no metabolic model driving the decision "you should dream now."

What the architecture requires: sleep pressure accumulates from ticks (especially novel/effortful ones). When pressure exceeds threshold, the agent transitions to dreaming state. Extensions influence this via `on_dream_start` hooks.

**Neuro: queried for hints, not routing**

The knowledge store is queried per-task for strategy fragments. These fragments go into the system prompt. But they do not influence model routing (which model to use), tool selection (which tools to enable), or gate configuration (which rungs to run).

What the architecture requires: neuro entries inform the CascadeRouter's context features, modulating model selection based on accumulated domain knowledge.

**Daimon: orchestrator-side only**

The affect model runs in PlanRunner. Agents themselves have no affective state. They cannot feel frustration from repeated failures, satisfaction from task completion, or anxiety from budget pressure. Affect modulates the orchestrator's decisions about agents, not the agents' own decisions.

What the architecture requires: each agent carries its own DaimonState. Affect accumulates from the agent's own experiences and modulates its own gating threshold, tool preferences, and communication style.

**Chain crate: traits without runtime**

`ChainClient` and `ChainWallet` traits are defined. MEV gate exists. But no runtime instantiates a chain subscriber, connects to an RPC endpoint, or processes real blocks. The entire blockchain domain exists as type signatures and test mocks.

What the architecture requires: `ChainSubscriberExt` wrapping an alloy provider, subscribing to block headers, filtering transactions through bloom/fuse filters, and emitting events to the EventFabric.

### The monolith problem

`orchestrate.rs` is 20,678 lines with 137 fields on `PlanRunner`. It is the single largest file in the codebase and the source of most maintenance burden. Here is what lives there, by responsibility:

| Responsibility | Approximate LOC | Key methods |
|---|---|---|
| Agent dispatch | ~7,500 | `dispatch_agent_with`, `spawn_agent_with_layer`, prompt routing, enrichment |
| Gate execution | ~4,000 | `run_gates_for_task`, rung dispatch, artifact collection, ratchet |
| Learning/episodes | ~3,500 | Episode recording, efficiency events, playbook extraction, skill library |
| System prompt assembly | ~2,200 | `build_role_system_prompt`, VCG auction, section selection |
| Conductor/stuck detection | ~2,000 | Anomaly watcher integration, stuck patterns, circuit breaker |
| Worktree/git management | ~2,000 | Branch creation, worktree lifecycle, merge, conflict resolution |
| Replan/recovery | ~3,000 | Gate failure replan, retry conductor, error classification |
| Event bus/persistence | ~1,500 | Runtime event emission, snapshot save/restore |
| Metrics/observability | ~1,200 | Counter updates, histogram recording, trace emission |
| Miscellaneous helpers | ~3,500 | Domain routing, config synthesis, plan discovery, misc utilities |

Every one of these responsibilities corresponds to one or more extensions in the target architecture. The path forward is extraction, not rewrite.

---

## 2. Gap analysis

Eight gaps separate the current implementation from the architecture specification. Each gap is scoped: what was envisioned, what exists, what must be built.

### Gap 1: Agent Runtime (critical)

**Envisioned**: Agents are persistent processes with a typed lifecycle (`Unvalidated -> Validated -> ResourcesAllocated -> ToolsLoaded -> NeuroInitialized -> RoutingConfigured -> Ready`). Each agent runs its own heartbeat pipeline. The type system prevents calling `.tick()` on a dead agent.

**What exists**: `roko-runtime/src/lifecycle.rs` already defines `Agent<Phase>` with type-state transitions. The lifecycle states are declared. `HeartbeatClock` exists. `ProcessSupervisor` tracks processes.

But: agents are still spawn-and-die. PlanRunner creates a subprocess (or API call), waits for completion, collects the result, and discards the agent. There is no persistent agent process running a tick loop. The type-state machine is defined but never driven by a heartbeat.

**What must be built**:

```rust
/// The AgentRuntime trait: the contract that a running agent fulfills.
/// This replaces the implicit contract in PlanRunner's dispatch code.
#[async_trait]
pub trait AgentRuntime: Send + Sync + 'static {
    /// Unique identifier for this agent instance.
    fn id(&self) -> &AgentId;

    /// The domain profile governing this agent's behavior.
    fn profile(&self) -> &DomainProfile;

    /// Current lifecycle phase (compile-time enforced via phantom type,
    /// runtime-queryable via this method for introspection).
    fn phase(&self) -> AgentLifecycleState;

    /// Boot the agent: validate config, allocate resources, load tools,
    /// initialize knowledge store, configure routing, transition to Ready.
    async fn boot(&mut self) -> Result<()>;

    /// Run one heartbeat tick. Returns the outcome (suppressed, acted, dream, error).
    /// This is the core loop -- everything else is configuration.
    async fn tick(&mut self) -> Result<TickOutcome>;

    /// Inject an external task for the agent to work on.
    /// The task enters the agent's working memory and influences the next tick.
    async fn inject_task(&mut self, task: Task) -> Result<()>;

    /// Inject an external event (operator message, system event).
    async fn inject_event(&mut self, event: RuntimeEvent) -> Result<()>;

    /// Request shutdown. Extensions get to vote.
    async fn shutdown(&mut self) -> Result<ShutdownOutcome>;

    /// Persist current state for later resume.
    async fn snapshot(&self) -> Result<AgentSnapshot>;

    /// Resume from a persisted snapshot.
    async fn resume(snapshot: AgentSnapshot) -> Result<Self>
    where
        Self: Sized;
}
```

The concrete implementation (`LiveAgent<Ready>`) drives the `HeartbeatPipeline` and `ExtensionChain` on each tick. PlanRunner's role shrinks from "do everything" to "discover plans, spawn agents, inject tasks, collect outcomes."

### Gap 2: Extension system (critical)

**Envisioned**: 22 hooks across 8 layers, topologically sorted, composable. Adding behavior to an agent means implementing a trait, not modifying the monolith.

**What exists**: The Extension trait is fully specified in 01-ARCHITECTURE.md but has no implementation in the codebase. All extension-like behavior lives inline in `orchestrate.rs` as methods on `PlanRunner`.

**What must be built**:

The `Extension` trait itself, `ExtensionChain` builder with topological sort, and `ExtensionLayer` enum. Then: extract each responsibility from PlanRunner into an extension implementation.

```rust
/// Layer 3 (Cognition): Affect model integration
pub struct DaimonExt {
    state: DaimonState,
    somatic_buffer: Vec<SomaticSignal>,
}

#[async_trait]
impl Extension for DaimonExt {
    fn name(&self) -> &str { "daimon" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Cognition }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        // Update arousal from observations
        let novelty = ctx.observations().environmental_delta();
        self.state.update_arousal(novelty);
        Ok(())
    }

    async fn on_gate(&mut self, ctx: &mut GateContext) -> Result<Option<CognitiveTier>> {
        // Somatic markers can force escalation
        for signal in &self.somatic_buffer {
            if signal.intensity > 0.8 {
                return Ok(Some(CognitiveTier::T2));
            }
        }
        Ok(None)
    }

    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        // Inject affect state and somatic warnings into context
        let arousal = self.state.arousal();
        if arousal > 0.7 {
            ws.boost_category(ContextCategory::Warnings, 1.0 + arousal * 0.5);
        }
        if !self.somatic_buffer.is_empty() {
            ws.add_section(ContextSection {
                category: ContextCategory::Warnings,
                priority: 4,
                content: self.render_somatic_warnings(),
                tokens: estimate_tokens(&self.somatic_buffer),
                ..Default::default()
            });
        }
        Ok(())
    }

    async fn on_reflect(&mut self, record: &DecisionCycleRecord) -> Result<()> {
        // Update affect based on outcome
        let event = AffectEvent::from_outcome(&record.outcome);
        self.state.process_event(event);
        Ok(())
    }

    async fn save_state(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(&self.state)?)
    }

    async fn load_state(&mut self, state: serde_json::Value) -> Result<()> {
        self.state = serde_json::from_value(state)?;
        Ok(())
    }
}
```

Each extracted extension follows this pattern: wrap an existing crate's state, implement the relevant hooks, delegate to the crate's logic. The logic does not change -- only its invocation surface.

### Gap 3: Cognitive gating (high)

**Envisioned**: Every tick runs through a prediction-error-based gate that classifies it as T0 ($0), T1 ($0.001), or T2 ($0.05). 80% of ticks are T0 -- pure Rust pattern matching, no LLM call. This makes continuous operation economically viable.

**What exists**: The cascade router does model selection (cheap vs. expensive) based on task complexity. Adaptive gate thresholds (EMA + CUSUM) exist for the rung pipeline. But there is no per-tick gate that decides "don't call the LLM at all." Every dispatched task goes to an LLM.

The fundamental issue: the current architecture is task-driven (receive task -> dispatch -> collect result), not tick-driven (tick -> observe -> gate -> maybe act). Cognitive gating requires the tick model.

**What must be built**:

```rust
pub struct CognitiveGate {
    /// Base threshold from domain profile
    base_threshold: f64,
    /// Adaptive component (EWMA of recent prediction errors)
    ewma_pe: f64,
    /// CUSUM detector for regime changes
    cusum: CusumDetector,
    /// Habituation mask: patterns seen N+ times get suppressed
    habituation: HashMap<PatternHash, u32>,
    /// Per-source weights for prediction error computation
    source_weights: PredictionErrorWeights,
}

impl CognitiveGate {
    pub fn classify(
        &mut self,
        observations: &Observations,
        cortical: &CorticalState,
    ) -> CognitiveTier {
        let pe = self.compute_prediction_error(observations);
        let effective_threshold = self.effective_threshold(cortical);

        // Update habituation
        for pattern in observations.matched_patterns() {
            *self.habituation.entry(pattern).or_insert(0) += 1;
        }

        // Habituated patterns reduce effective PE
        let habituated_pe = pe * self.habituation_factor(observations);

        if habituated_pe < effective_threshold {
            CognitiveTier::T0
        } else if habituated_pe < effective_threshold * 2.0 {
            CognitiveTier::T1
        } else {
            CognitiveTier::T2
        }
    }

    fn effective_threshold(&self, cortical: &CorticalState) -> f64 {
        let confidence = cortical.epistemic_confidence();
        let vitality = cortical.economic_vitality();
        let arousal = cortical.arousal();

        let confidence_factor = 0.5 + (confidence * 0.5);
        let mortality_factor = if vitality < 0.3 { 1.5 }
            else if vitality < 0.5 { 1.2 }
            else { 1.0 };
        let arousal_factor = 1.0 - (arousal * 0.3);

        (self.base_threshold * confidence_factor * mortality_factor * arousal_factor)
            .clamp(0.05, 0.80)
    }
}
```

This is the centerpiece of the economic model. Without it, agents are unboundedly expensive. With it, 80%+ of time costs nothing.

### Gap 4: Event fabric -> agent subscriptions (high)

**Envisioned**: Agents subscribe to typed event streams. A blockchain agent subscribes to `NewBlock` and `PriceFeed`. Events interrupt dreams (emergency wakeup), contribute to prediction error, and drive reactive behavior.

**What exists**: `RuntimeEventBus<RokoEvent>` provides broadcast channels with envelope wrapping. The bus supports typed events, sequence numbers, and filtered subscription. TUI and HTTP API consume events. Agents do not.

**What must be built**:

```rust
/// Agent-side event receiver with filtered subscription.
pub struct AgentEventReceiver {
    /// Filtered broadcast receiver
    rx: broadcast::Receiver<RuntimeEventEnvelope<RokoEvent>>,
    /// Subscription filters (which event types to deliver)
    filters: Vec<EventFilter>,
    /// Buffered events not yet consumed by the tick loop
    buffer: VecDeque<RuntimeEvent>,
    /// Emergency events that interrupt dreams
    emergency_buffer: VecDeque<RuntimeEvent>,
}

impl AgentEventReceiver {
    /// Drain all pending events matching our filters.
    /// Called at the start of each tick (step 1: OBSERVE).
    pub fn drain(&mut self) -> Vec<RuntimeEvent> {
        // Non-blocking drain of broadcast channel
        while let Ok(envelope) = self.rx.try_recv() {
            if self.matches_filter(&envelope.payload) {
                if self.is_emergency(&envelope.payload) {
                    self.emergency_buffer.push_back(envelope.payload);
                } else {
                    self.buffer.push_back(envelope.payload);
                }
            }
        }
        self.buffer.drain(..).collect()
    }

    /// Check for emergency events that should interrupt dreaming.
    pub fn has_emergency(&mut self) -> bool {
        self.drain(); // side-effect: fills emergency buffer
        !self.emergency_buffer.is_empty()
    }
}
```

The integration point: each agent's `on_observe` hook drains its `AgentEventReceiver` and contributes received events to the tick's observation set. Events feed prediction error calculation.

### Gap 5: CognitiveWorkspace (high)

**Envisioned**: Context assembly is a learnable feedback loop. Sections that correlate with success grow. The `ContextPolicy` evolves autonomously. Affect modulates allocation. A VCG auction determines which sections compete for limited token budget.

**What exists**: `PromptComposer` in roko-compose does context assembly with VCG-style bidding and section attribution. `SectionEffectivenessRegistry` tracks per-section outcomes. `AttentionBidder` types (Neuro/Task/Research) participate in allocation.

The gap: this is orchestrator-side, called once per task dispatch. It is not per-tick. It does not live inside the agent. And there is no `ContextPolicy` that evolves its allocation weights based on accumulated feedback.

**What must be built**:

```rust
pub struct CognitiveWorkspace {
    pub tier: CognitiveTier,
    pub sections: Vec<ContextSection>,
    pub total_budget_tokens: u32,
    pub used_tokens: u32,
    pub assembly_log: Vec<AssemblyDecision>,
    pub policy: ContextPolicy,
    cortical: Arc<CorticalState>,
}

impl CognitiveWorkspace {
    pub fn new(tier: CognitiveTier, policy: ContextPolicy, cortical: Arc<CorticalState>) -> Self {
        let budget = match tier {
            CognitiveTier::T0 => 0,      // no LLM call
            CognitiveTier::T1 => 4_096,  // cheap model, small context
            CognitiveTier::T2 => 32_768, // full model, large context
        };
        Self {
            tier,
            sections: Vec::new(),
            total_budget_tokens: budget,
            used_tokens: 0,
            assembly_log: Vec::new(),
            policy,
            cortical,
        }
    }

    /// Add a section. Budget-checked and priority-sorted.
    pub fn add_section(&mut self, section: ContextSection) {
        if self.used_tokens + section.tokens <= self.total_budget_tokens {
            self.used_tokens += section.tokens;
            self.assembly_log.push(AssemblyDecision::Included {
                category: section.category,
                tokens: section.tokens,
                reason: "within budget".into(),
            });
            self.sections.push(section);
        } else {
            self.assembly_log.push(AssemblyDecision::Excluded {
                category: section.category,
                tokens: section.tokens,
                reason: "budget exhausted".into(),
            });
        }
    }

    /// Boost a category's effective priority (affect modulation).
    pub fn boost_category(&mut self, category: ContextCategory, factor: f64) {
        for section in &mut self.sections {
            if section.category == category {
                section.allocation *= factor;
            }
        }
    }

    /// Render to final prompt string, sorted by priority then insertion order.
    pub fn render(&self) -> String {
        let mut sorted = self.sections.clone();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        sorted.iter().map(|s| &s.content).collect::<Vec<_>>().join("\n\n")
    }
}
```

The workspace wraps the existing `PromptComposer` logic but operates per-tick inside the agent, and carries a `ContextPolicy` that evolves.

### Gap 6: Agent persistence and state (medium)

**Envisioned**: Agents maintain working memory across ticks, access the knowledge store, and persist state between sessions. An agent that restarts picks up where it left off.

**What exists**: `AgentSnapshot` serialization in `roko-runtime/lifecycle.rs`. `ExecutorSnapshot` for plan state. Episode logger for turn history. Knowledge store for long-term memory.

The gap: no working memory that survives across ticks within a session. Each dispatch is a fresh context window. The agent cannot remember "I tried approach X on tick 12 and it failed" without it being explicitly assembled into the prompt.

**What must be built**:

```rust
/// Per-agent working memory: survives across ticks, bounded in size.
pub struct WorkingMemory {
    /// Ring buffer of recent observations (last N ticks)
    observations: VecDeque<ObservationRecord>,
    /// Active hypotheses the agent is tracking
    hypotheses: Vec<Hypothesis>,
    /// Pending decisions awaiting confirmation
    pending: Vec<PendingDecision>,
    /// Scratchpad: agent can write arbitrary structured notes
    scratchpad: BTreeMap<String, serde_json::Value>,
    /// Maximum entries (oldest evicted when exceeded)
    capacity: usize,
}

impl WorkingMemory {
    /// Query working memory for relevant entries given current context.
    pub fn query_relevant(&self, task: &Task, limit: usize) -> Vec<&ObservationRecord> {
        // HDC similarity between task embedding and observation embeddings
        self.observations.iter()
            .map(|obs| (obs, obs.embedding.similarity(&task.embedding())))
            .filter(|(_, sim)| *sim > 0.3)
            .sorted_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Ordering::Equal))
            .take(limit)
            .map(|(obs, _)| obs)
            .collect()
    }

    /// Record the outcome of the current tick.
    pub fn record(&mut self, record: ObservationRecord) {
        if self.observations.len() >= self.capacity {
            self.observations.pop_front();
        }
        self.observations.push_back(record);
    }
}
```

Working memory is distinct from the knowledge store. Knowledge store = long-term, distilled, tier-promoted facts. Working memory = short-term, raw, task-scoped context.

### Gap 7: Process model (medium)

**Envisioned**: Supervision trees with one-for-one restart, escalating kill, orphan reaping. Agents are processes within a supervision hierarchy.

**What exists**: `ProcessSupervisor` in roko-runtime tracks PIDs, sends kill signals, reaps zombies. `CancelToken` for cooperative shutdown. Circuit breaker for progressive degradation.

The gap: flat process model. All agents are direct children of PlanRunner. There is no hierarchy, no restart policy differentiation, no isolation between unrelated agents.

**What must be built**:

```rust
pub struct SupervisionTree {
    /// Root supervisor
    root: SupervisorNode,
}

pub struct SupervisorNode {
    /// This supervisor's identity
    id: SupervisorId,
    /// Restart strategy for children
    strategy: RestartStrategy,
    /// Child agents or nested supervisors
    children: Vec<SupervisedChild>,
    /// Maximum restarts within window before escalating
    max_restarts: u32,
    /// Window for counting restarts
    restart_window: Duration,
    /// Restart history
    restart_log: VecDeque<Instant>,
}

pub enum RestartStrategy {
    /// Restart only the failed child
    OneForOne,
    /// Restart all children when one fails (shared state dependency)
    OneForAll,
    /// Restart the failed child and all children started after it
    RestForOne,
}

pub enum SupervisedChild {
    Agent(AgentHandle),
    Supervisor(SupervisorNode),
}
```

This is lower priority than gaps 1-5 because the current flat model works for plan execution. Supervision trees become critical when multiple agents cooperate on shared state (blockchain fleet scenario).

### Gap 8: A2A communication (lower)

**Envisioned**: Agents communicate via pheromones (ambient signals) and JSON-RPC 2.0 A2A protocol (direct messages). Delegation DAG tracks who delegated what to whom.

**What exists**: `Pheromone` struct in roko-orchestrator with `PheromoneKind` and `PheromoneScope`. Gate verdicts deposit pheromones. But emission is orchestrator-side -- agents cannot read or emit pheromones themselves.

**What must be built**:

```rust
/// Extension for pheromone-based ambient communication
pub struct PheromoneExt {
    /// Local pheromone field (what this agent can sense)
    field: Vec<Pheromone>,
    /// Pheromones emitted by this agent (pending broadcast)
    outbox: Vec<Pheromone>,
    /// Evaporation rate (pheromones decay over time)
    evaporation_rate: f64,
    /// Fabric connection for broadcast/receive
    fabric: Arc<EventFabric>,
}

#[async_trait]
impl Extension for PheromoneExt {
    fn name(&self) -> &str { "pheromones" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        // Receive pheromones from fabric
        let events = self.fabric.drain_filtered(EventFilter::Pheromone);
        for event in events {
            if let RokoEvent::Pheromone(p) = event {
                self.field.push(p);
            }
        }
        // Evaporate old pheromones
        self.field.retain(|p| p.intensity > 0.01);
        for p in &mut self.field {
            p.intensity *= 1.0 - self.evaporation_rate;
        }
        // Add pheromone observations to tick context
        ctx.add_observation(Observation::Pheromones(self.field.clone()));
        Ok(())
    }

    async fn on_reflect(&mut self, record: &DecisionCycleRecord) -> Result<()> {
        // Emit pheromones based on outcomes
        if let Some(verdict) = &record.gate_verdict {
            let kind = if verdict.passed {
                PheromoneKind::Success
            } else {
                PheromoneKind::Warning
            };
            self.outbox.push(Pheromone {
                kind,
                scope: PheromoneScope::Task(record.task_id.clone()),
                intensity: 1.0,
                emitter: self.agent_id.clone(),
                timestamp: Utc::now(),
            });
        }
        // Broadcast outbox
        for p in self.outbox.drain(..) {
            self.fabric.emit(RokoEvent::Pheromone(p));
        }
        Ok(())
    }
}
```

Direct A2A messaging (JSON-RPC 2.0) is Phase 2+. The pheromone model provides ambient coordination without requiring explicit addressing.

---

## 3. What can be preserved vs. rewritten

The migration philosophy: **extract, don't rewrite**. Existing logic moves into extension wrappers. The logic itself is proven -- it ran thousands of self-hosting cycles. What changes is how it gets invoked.

| Crate | Fate | Rationale |
|---|---|---|
| `roko-core` | **KEEP** (foundation) | Traits are domain-agnostic. They define the vocabulary. Nothing about them conflicts with the new architecture. |
| `roko-agent` | **KEEP** (backends + ToolLoop) | The 6 backends and ToolLoop are infrastructure. They get called from inside the heartbeat pipeline instead of from PlanRunner, but their implementation is unchanged. |
| `roko-runtime` | **EXTEND** | Already has lifecycle type-state, event bus, process supervisor, heartbeat clock. Add: `AgentRuntime` trait, `ExtensionChain`, `CognitiveGate`, `CognitiveWorkspace`, `EventFabric` (wrap existing bus). |
| `roko-gate` | **KEEP** (used by GateExt) | Gate pipeline stays. `GateExt` wraps it as an extension hook called during the VERIFY step. |
| `roko-compose` | **KEEP -> evolves** | `PromptComposer` and VCG auction logic move into `CognitiveWorkspace`. The crate becomes the implementation backing ContextExt. |
| `roko-learn` | **KEEP** (used by LearningExt) | Episodes, playbooks, routing, experiments all stay. `LearningExt` wraps them in `on_reflect` and `on_outcome` hooks. |
| `roko-daimon` | **KEEP** (used by DaimonExt) | ALMA model, somatic markers, strategy coordinates all stay. `DaimonExt` wraps them in cognition-layer hooks. |
| `roko-neuro` | **KEEP** (used by NeuroExt) | Knowledge store, tier progression stay. `NeuroExt` wraps them in `assemble_context` to inject knowledge sections. |
| `roko-dreams` | **KEEP** (used by DreamsExt) | Consolidation cycle stays. `DreamsExt` wraps it, triggers on sleep pressure threshold. |
| `roko-conductor` | **KEEP** (used by ConductorExt) | 10 watchers, circuit breaker stay. `ConductorExt` wraps them in perception-layer hooks. |
| `roko-orchestrator` | **KEEP** (plan DAG execution) | Plan discovery, parallel executor, merge queue stay for orchestrated plan runs. |
| `roko-chain` | **KEEP** (used by ChainSubscriberExt) | Trait definitions stay. Extensions implement them against real providers. |
| `roko-primitives` | **KEEP** | HDC vectors and tier routing are infrastructure. Used by multiple extensions. |
| `roko-cli/orchestrate.rs` | **DECOMPOSE** | Each responsibility extracts into its corresponding extension. PlanRunner becomes a thin coordinator. |

Estimated preservation: ~85% of existing code survives. The remaining ~15% is glue code in orchestrate.rs that gets replaced by extension hook invocations.

---

## 4. Target crate layout

### New crates

**`roko-runtime` (extended, not rewritten)**

Existing: `cancel`, `event_bus`, `process`, `lifecycle`, `heartbeat`, `metrics`, `energy`.

Added:
- `agent_runtime.rs`: `AgentRuntime` trait, `LiveAgent<Phase>` implementation
- `extension.rs`: `Extension` trait, `ExtensionLayer`, `ExtensionChain`, `ExtensionChainBuilder`
- `cognitive_gate.rs`: `CognitiveGate`, `CognitiveTier`, prediction error computation
- `cognitive_workspace.rs`: `CognitiveWorkspace`, `ContextSection`, `ContextPolicy`
- `cortical_state.rs`: `CorticalState` (lock-free shared state for inter-extension communication)
- `domain_profile.rs`: `DomainProfile` struct, predefined profiles
- `working_memory.rs`: `WorkingMemory`, bounded ring buffer

**`roko-ext-core`** (new crate)

Core extensions that every agent loads regardless of domain:
- `HeartbeatExt`: drives tick scheduling, frequency transitions
- `ContextExt`: wraps CognitiveWorkspace assembly, affect modulation
- `DaimonExt`: wraps roko-daimon, manages per-agent affect
- `LearningExt`: wraps episode recording, playbook extraction, routing feedback
- `DreamsExt`: wraps roko-dreams, triggers on sleep pressure
- `ConductorExt`: wraps roko-conductor watchers, drives circuit breaker
- `NeuroExt`: wraps roko-neuro queries, injects knowledge into context
- `SafetyExt`: wraps roko-agent safety layer, enforces tool policies
- `ToolsExt`: tool dispatch, format selection, MCP integration

**`roko-ext-code`** (new crate)

Extensions specific to coding agents:
- `GitExt`: worktree management, branch creation, merge conflict detection
- `GateExt`: compile/test/clippy gate pipeline, rung dispatch
- `CodeIntelExt`: file watching, symbol resolution, context gathering
- `CiExt`: CI pipeline monitoring, PR status tracking

**`roko-ext-chain`** (new crate)

Extensions specific to blockchain agents:
- `ChainSubscriberExt`: alloy provider, block subscription, tx filtering
- `PriceFeedExt`: oracle subscription, price deviation detection
- `RiskExt`: position health, liquidation distance, portfolio exposure
- `MortalityExt`: economic vitality clock, budget depletion, graceful death
- `StrategyStoreExt`: persistent strategy parameters, Kelly criterion
- `PheromoneExt`: ambient inter-agent coordination via signal field

**`roko-ext-research`** (new crate)

Extensions specific to research agents:
- `SourceWatcherExt`: RSS/arXiv/API monitoring, new publication detection
- `KnowledgeGraphExt`: citation tracking, contradiction detection
- `HypothesisExt`: hypothesis lifecycle, evidence accumulation
- `SynthesisExt`: cross-source synthesis, literature review assembly

### Dependency diagram

```
roko-core (foundation)
    |
    +-- roko-primitives (HDC, tier routing)
    |
    +-- roko-runtime (agent runtime, extensions, event fabric, lifecycle)
    |       |
    |       +-- roko-agent (LLM backends, ToolLoop, safety)
    |       |
    |       +-- roko-gate (verification pipeline)
    |       |
    |       +-- roko-compose (prompt assembly)
    |       |
    |       +-- roko-learn (episodes, routing, experiments)
    |       |
    |       +-- roko-daimon (affect model)
    |       |
    |       +-- roko-neuro (knowledge store)
    |       |
    |       +-- roko-dreams (consolidation)
    |       |
    |       +-- roko-conductor (anomaly detection)
    |
    +-- roko-ext-core (core extensions -- depends on runtime + all subsystem crates)
    |
    +-- roko-ext-code (coding extensions -- depends on ext-core + gate + compose)
    |
    +-- roko-ext-chain (blockchain extensions -- depends on ext-core + chain)
    |
    +-- roko-ext-research (research extensions -- depends on ext-core + neuro)
    |
    +-- roko-orchestrator (plan DAG, merge queue -- uses runtime for agent spawning)
    |
    +-- roko-cli (binary: subcommands, TUI, config -- depends on everything above)
```

The key insight: `roko-ext-core` depends on the subsystem crates (daimon, neuro, etc.) but wraps them as extensions. The subsystem crates remain independent -- you can use `roko-daimon` standalone without the extension system.

---

## 5. Migration path (5 phases)

### Phase 1: Extract runtime core (2 weeks)

**Goal**: The `AgentRuntime` trait, `Extension` trait, `ExtensionChain`, `CorticalState`, and `CognitiveGate` exist in `roko-runtime`. Nothing else changes. Existing code continues working.

**Steps**:

1. Add `extension.rs` to roko-runtime:
   - `Extension` trait with 22 hooks (all default-no-op)
   - `ExtensionLayer` enum (8 layers)
   - `ExtensionChainBuilder` with topological sort (Kahn's algorithm)
   - `ExtensionChain` with pre-computed firing orders

2. Add `cortical_state.rs`:
   - Lock-free shared state via `AtomicF64` (or `AtomicU64` with f64 bit-cast)
   - Read/write methods for arousal, pleasure, dominance, confidence, vitality
   - Agent-local (not shared between agents)

3. Add `cognitive_gate.rs`:
   - `CognitiveTier` enum (T0/T1/T2)
   - `CognitiveGate` struct with prediction error computation
   - Domain-specific observation sources (coding, blockchain, research)
   - Adaptive threshold with confidence/mortality/arousal modulation
   - Habituation mask for suppressing repeated patterns

4. Add `cognitive_workspace.rs`:
   - `CognitiveWorkspace` struct with budgeted section management
   - `ContextSection`, `ContextCategory`, `ContextPolicy`
   - `AssemblyDecision` audit log
   - `BetaDistribution` for Thompson sampling on categories

5. Add `domain_profile.rs`:
   - `DomainProfile` struct
   - Predefined profiles: coding, blockchain, research, docs, security, custom
   - Profile loading from `roko.toml` configuration

6. Add `agent_runtime.rs`:
   - `AgentRuntime` trait
   - `LiveAgent<Phase>` implementation (placeholder -- drives HeartbeatPipeline + ExtensionChain)
   - Integration with existing `lifecycle.rs` type-state

**Validation**: `cargo test --workspace` passes. No existing behavior changes. The new code is purely additive.

### Phase 2: Extract core extensions (3 weeks)

**Goal**: Each responsibility currently in orchestrate.rs has a corresponding extension implementation. Extensions are tested in isolation with mock CorticalState and mock EventFabric.

**Steps**:

1. Create `roko-ext-core` crate with Cargo.toml depending on roko-runtime + subsystem crates.

2. Extract `DaimonExt`:
   - Move `DaimonState` usage from PlanRunner into extension
   - Implement `on_observe` (arousal update), `on_gate` (somatic override), `assemble_context` (inject affect), `on_reflect` (update affect from outcome)
   - Test: mock observations -> verify affect state transitions

3. Extract `LearningExt`:
   - Move episode recording, efficiency events, playbook extraction
   - Implement `on_reflect` (record episode), `on_outcome` (extract skills), `on_tick_end` (flush efficiency events)
   - Test: mock outcomes -> verify episode JSONL output

4. Extract `ConductorExt`:
   - Move 10 anomaly watchers, circuit breaker integration
   - Implement `on_observe` (run watchers), `on_gate` (circuit breaker veto), `on_tick_end` (update health)
   - Test: inject anomalous observations -> verify circuit breaker trips

5. Extract `DreamsExt`:
   - Move dream trigger logic, sleep pressure accumulation
   - Implement `on_tick_end` (accumulate pressure), `on_dream_start`/`on_dream_phase`/`on_dream_end` (drive cycle)
   - Test: accumulate ticks -> verify dream triggers at threshold

6. Extract `ContextExt`:
   - Move prompt assembly, VCG auction, section attribution
   - Implement `assemble_context` (orchestrate workspace filling), `on_outcome` (update ContextPolicy)
   - Test: mock extensions contribute sections -> verify budget enforcement

7. Extract `NeuroExt`:
   - Move knowledge store queries from PlanRunner
   - Implement `assemble_context` (inject relevant knowledge), `on_reflect` (promote/demote entries)
   - Test: pre-loaded store -> verify relevant entries appear in workspace

8. Extract `SafetyExt`:
   - Move SafetyLayer authorization from PlanRunner
   - Implement `before_tool_call` (authorize), `after_tool_call` (audit)
   - Test: policy violation -> verify tool call blocked

9. Extract `ToolsExt`:
   - Move tool dispatch, format bandit, MCP integration
   - Implement tool execution during EXECUTE step
   - Test: tool call -> verify dispatch to correct backend

**Validation**: Each extension has unit tests with >90% coverage of its hook implementations. Integration test spawns a `LiveAgent` with all core extensions, injects a task, verifies the full tick pipeline.

### Phase 3: Rewrite PlanRunner (2 weeks)

**Goal**: PlanRunner becomes a thin coordinator. Its 137 fields shrink to ~20. Extensions own their state.

**Steps**:

1. Create `AgentOrchestrator` -- the new plan coordination layer:
   ```rust
   pub struct AgentOrchestrator {
       /// Working directory
       workdir: PathBuf,
       /// Configuration
       config: Config,
       /// Plan executor state machine
       executor: ParallelExecutor,
       /// Spawn agents when tasks are ready
       agent_pool: AgentPool,
       /// Collect outcomes when agents complete ticks
       outcome_rx: mpsc::Receiver<TaskOutcome>,
       /// Event fabric for cross-agent communication
       fabric: Arc<EventFabric>,
       /// Supervision tree
       supervision: SupervisionTree,
       /// Metrics
       metrics: Arc<MetricRegistry>,
   }
   ```

2. Task injection: when executor yields `ExecutorAction::RunTask`, orchestrator finds (or spawns) an agent with the right DomainProfile, calls `agent.inject_task(task)`.

3. Outcome collection: agents emit `TaskOutcome` via channel when a task completes (gate verified or failed). Orchestrator feeds these back to executor as `ExecutorEvent`.

4. Agent lifecycle: orchestrator boots agents at plan start, keeps them running across tasks (no spawn-die), shuts them down at plan end.

5. Progressive migration: keep old PlanRunner working behind a feature flag. New orchestrator runs in parallel. Compare outcomes.

**Validation**: Run a real plan through the new orchestrator. All tasks complete. Gate verdicts match. Episodes record correctly.

### Phase 4: Wire event fabric (1 week)

**Goal**: Events flow between components. Agents subscribe to relevant events and incorporate them into perception.

**Steps**:

1. Wire `RuntimeEventBus` as the backing store for `EventFabric`.

2. Create `AgentEventReceiver` for each spawned agent, filtered by DomainProfile's `event_subscriptions`.

3. Gate verdicts emit events:
   ```rust
   fabric.emit(RokoEvent::GateVerdict {
       task_id, rung, passed, output
   });
   ```

4. File watcher emits events:
   ```rust
   fabric.emit(RokoEvent::FileChanged {
       path, kind, timestamp
   });
   ```

5. Agent `on_observe` hook drains the receiver:
   ```rust
   impl Extension for PerceptionExt {
       async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
           let events = self.receiver.drain();
           for event in events {
               ctx.add_observation(Observation::Event(event));
           }
           Ok(())
       }
   }
   ```

6. Emergency wakeup: if `has_emergency()` is true during a dream state, interrupt the dream and escalate to T2.

**Validation**: Emit a test event, verify an agent's tick incorporates it into observations and prediction error.

### Phase 5: Persistent agent + chat deployment (2 weeks)

**Goal**: Agents run as persistent processes. Operators chat with them via WebSocket. Single binary deployment.

**Steps**:

1. Agent serves WebSocket for operator messages:
   ```rust
   // In roko-agent-server (already exists, extend it)
   async fn ws_handler(ws: WebSocket, agent: Arc<Mutex<LiveAgent<Ready>>>) {
       while let Some(msg) = ws.recv().await {
           agent.lock().await.inject_event(
               RuntimeEvent::OperatorMessage(msg.into())
           ).await;
       }
   }
   ```

2. `on_message` hook processes operator input:
   ```rust
   impl Extension for ContextExt {
       async fn on_message(&mut self, msg: &OperatorMessage, ctx: &mut MessageContext) -> Result<()> {
           // Force next tick to T2 (operator message = infinite prediction error)
           ctx.force_tier(CognitiveTier::T2);
           // Add message to working memory
           ctx.working_memory().record(ObservationRecord::from_message(msg));
           Ok(())
       }
   }
   ```

3. Single-binary deployment:
   ```bash
   # Start a persistent coding agent
   roko agent start --profile coding --ws-port 8080

   # Start a persistent blockchain agent
   roko agent start --profile blockchain --ws-port 8081 --chain ethereum

   # Start orchestrated plan execution (spawns agents as needed)
   roko plan run plans/ --persistent
   ```

4. Agent resume: on restart, load snapshot, call `resume()`, extensions restore their state.

**Validation**: Start agent, send operator message via WebSocket, verify T2 tick fires, action taken, response streamed back.

---

## 6. Testing strategy

### Unit tests (per extension)

Every extension gets isolated tests with mock dependencies:

```rust
#[tokio::test]
async fn daimon_ext_somatic_forces_t2() {
    let mut ext = DaimonExt::new(DaimonState::default());

    // Inject a high-intensity somatic signal
    ext.somatic_buffer.push(SomaticSignal {
        pattern: "repeated_failure".into(),
        intensity: 0.9,
        source: "gate_failure".into(),
    });

    let mut ctx = GateContext::mock();
    let result = ext.on_gate(&mut ctx).await.unwrap();

    assert_eq!(result, Some(CognitiveTier::T2));
}

#[tokio::test]
async fn conductor_ext_trips_circuit_breaker() {
    let mut ext = ConductorExt::new(Conductor::default());

    // Simulate 5 consecutive failures (threshold = 3)
    for _ in 0..5 {
        let mut ctx = ObserveContext::mock_with_failure();
        ext.on_observe(&mut ctx).await.unwrap();
    }

    let mut gate_ctx = GateContext::mock();
    let result = ext.on_gate(&mut gate_ctx).await.unwrap();

    // Circuit breaker forces T0 (suppress all actions while open)
    assert_eq!(result, Some(CognitiveTier::T0));
}
```

### Integration tests (full pipeline)

Spawn a real `LiveAgent<Ready>` with all core extensions, inject a task, verify the lifecycle:

```rust
#[tokio::test]
async fn full_tick_pipeline_coding_agent() {
    let profile = DomainProfile::coding();
    let mut agent = LiveAgent::boot(profile, test_config()).await.unwrap();

    // Inject a coding task
    agent.inject_task(Task::new("Fix the null pointer in auth.rs")).await.unwrap();

    // First tick should escalate to T2 (novel task = high prediction error)
    let outcome = agent.tick().await.unwrap();
    assert!(matches!(outcome, TickOutcome::Acted { tier: CognitiveTier::T2, .. }));

    // Subsequent tick with no changes should suppress
    let outcome2 = agent.tick().await.unwrap();
    assert!(matches!(outcome2, TickOutcome::Suppressed { .. }));
}
```

### Property tests (type-state exhaustiveness)

The type system enforces valid transitions, but property tests verify runtime behavior matches:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn lifecycle_transitions_are_monotonic(
        transitions in prop::collection::vec(
            prop_oneof![
                Just(LifecycleTransition::Validate),
                Just(LifecycleTransition::AllocateResources),
                Just(LifecycleTransition::LoadTools),
                Just(LifecycleTransition::InitNeuro),
                Just(LifecycleTransition::ConfigureRouting),
            ],
            1..10
        )
    ) {
        let mut agent = Agent::<Unvalidated>::new(test_config());
        let mut phase_ordinal = 0;

        for t in transitions {
            match agent.try_transition(t) {
                Ok(next) => {
                    let next_ordinal = next.phase().ordinal();
                    prop_assert!(next_ordinal > phase_ordinal);
                    phase_ordinal = next_ordinal;
                    agent = next;
                }
                Err(_) => {
                    // Invalid transition -- must be out-of-order
                    // This is expected; verify it doesn't panic
                }
            }
        }
    }
}
```

### Performance benchmarks

Critical path latency must stay low:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_t0_tick(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut agent = rt.block_on(async {
        LiveAgent::boot(DomainProfile::coding(), test_config()).await.unwrap()
    });

    c.bench_function("t0_tick_no_novelty", |b| {
        b.to_async(&rt).iter(|| async {
            agent.tick().await.unwrap()
        });
    });
}

fn bench_extension_chain_fire(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let chain = rt.block_on(async {
        ExtensionChainBuilder::new()
            .add(HeartbeatExt::new())
            .add(ContextExt::new())
            .add(DaimonExt::new(DaimonState::default()))
            .add(LearningExt::new())
            .add(ConductorExt::new(Conductor::default()))
            .build()
            .unwrap()
    });

    c.bench_function("fire_observe_5_extensions", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = ObserveContext::mock();
            chain.fire_observe(&mut ctx).await.unwrap();
        });
    });
}

criterion_group!(benches, bench_t0_tick, bench_extension_chain_fire);
criterion_main!(benches);
```

**Target latencies**:
- T0 tick (no LLM): <1ms
- Extension chain fire (5 extensions): <100us
- Cognitive gate evaluation: <10us
- CognitiveWorkspace assembly (10 sections): <500us
- Full T2 tick (including LLM): dominated by LLM latency (1-10s)

---

## 7. Risk assessment

### Risk 1: Breaking the plan-execution workflow

**Severity**: High. The plan workflow is what users depend on today.

**Scenario**: Refactoring PlanRunner breaks existing `roko plan run` behavior. Tests pass in isolation but real multi-step plans fail due to state management differences.

**Mitigation**:
- Phases 1-2 are purely additive. Old code stays working.
- Phase 3 runs old and new in parallel, comparing outcomes.
- Feature flag: `--use-new-runtime` for opt-in during migration.
- Integration test suite runs identical plans through both paths.

### Risk 2: Extension composition complexity

**Severity**: Medium. Emergent behavior from extension interactions.

**Scenario**: Two extensions both implement `on_gate` and produce conflicting tier decisions. Or a circular dependency in `assemble_context` where extension A needs data from extension B which needs data from extension A.

**Mitigation**:
- Gate hook returns `Option<CognitiveTier>` -- first non-None wins, with deterministic layer-order priority.
- `ExtensionChainBuilder::build()` runs cycle detection (Kahn's algorithm). Circular deps are a compile-time error.
- Start with 5 core extensions. Validate composition before adding domain-specific ones.
- Extensive integration tests for extension interaction patterns.

### Risk 3: Performance regression from indirection

**Severity**: Low-Medium. Extension dispatch adds virtual calls.

**Scenario**: 22 hooks x 10 extensions = 220 virtual method calls per tick. For a blockchain agent ticking every 5s, that is 44 vcalls/second (trivial). But if extension state grows or hooks become expensive, ticks slow down.

**Mitigation**:
- Per-tick arena allocator (`bumpalo::Bump`) prevents heap fragmentation.
- Benchmark T0 tick latency continuously. Regression = investigation.
- Extensions that don't implement a hook cost zero (default no-op is inlined by the compiler in release mode).
- Pre-computed firing orders avoid per-tick topological sort.

### Risk 4: State migration during snapshot resume

**Severity**: Medium. Changing data structures while agents hold persisted state.

**Scenario**: Agent snapshots from Phase 1 format cannot be loaded in Phase 3. Agents lose their working memory, knowledge, and affect state.

**Mitigation**:
- Versioned snapshot format from day one. `schema_version` field in every snapshot.
- Forward-compatible migration functions: v1 -> v2 -> v3 (never skip versions).
- Extension `save_state`/`load_state` hooks use `serde_json::Value` (schema-flexible).
- Test: save snapshot in version N, load in version N+1, verify state preserved.

### Risk 5: Cognitive gating too aggressive

**Severity**: Medium. Agent suppresses ticks that should have escalated.

**Scenario**: Habituation mask suppresses a pattern that looks familiar but is actually novel in a critical way. Agent misses an important event because prediction error was just below threshold.

**Mitigation**:
- Floor threshold at 0.05: even maximally habituated patterns can escalate truly extreme events.
- Extensions can force-override the gate (somatic markers, operator messages).
- Adaptive threshold CUSUM detects regime changes and temporarily lowers the threshold.
- Monitoring: track suppressed-tick-followed-by-escalation as a metric. High rate = threshold too aggressive.
- Operator kill switch: send message -> forces T2 regardless of gate.

### Risk 6: Divergent evolution between plan-mode and agent-mode

**Severity**: Medium. Two runtime paths that drift apart.

**Scenario**: Plan-mode (orchestrated, multi-agent, task-driven) and agent-mode (persistent, single-agent, event-driven) develop different code paths for the same operations. Bug fixes apply to one but not the other.

**Mitigation**:
- Both modes use the same AgentRuntime trait and Extension system.
- Plan-mode's `AgentOrchestrator` spawns `LiveAgent` instances -- same agent, different lifecycle.
- The only difference: plan-mode injects tasks from executor; agent-mode discovers tasks from events.
- Shared test suite that runs identical scenarios through both modes.

---

## 8. Success criteria

The migration is complete when:

1. **`roko plan run` works identically** to today, but internally uses AgentRuntime + Extensions instead of the monolith.

2. **`roko agent start --profile coding`** boots a persistent agent that ticks, gates, and responds to operator messages.

3. **T0 tick latency < 1ms**. A blockchain agent ticking every 5s spends <0.02% of wall time in T0 ticks.

4. **Extension isolation verified**: disabling any single non-core extension does not crash the agent.

5. **85%+ code reuse** from the existing codebase. The migration is extraction, not rewrite.

6. **Cost model validated**: a blockchain agent in calm conditions costs <$10/day (vs. $864/day without gating).

7. **Dream cycle triggers automatically** from sleep pressure, not manual invocation.

8. **Event fabric delivers** events from one agent's gate verdict to another agent's observation step within 1 tick.

---

## 9. Timeline

| Phase | Duration | Milestone |
|---|---|---|
| 1: Extract runtime core | 2 weeks | `AgentRuntime` + `Extension` + `CognitiveGate` exist, all tests pass |
| 2: Extract core extensions | 3 weeks | 9 extensions implemented, unit + integration tested |
| 3: Rewrite PlanRunner | 2 weeks | `AgentOrchestrator` runs real plans, old code behind feature flag |
| 4: Wire event fabric | 1 week | Agents subscribe to events, prediction error influenced |
| 5: Persistent agent + chat | 2 weeks | `roko agent start` runs indefinitely, WebSocket chat works |
| **Total** | **10 weeks** | Full migration from monolith to composable runtime |

Buffer: 2 weeks for unforeseen issues, integration testing, documentation.

Aggressive timeline: 8 weeks if phases 4 and 5 overlap (they share no code dependencies).

---

## 10. Decision log

| Decision | Rationale | Alternative considered |
|---|---|---|
| Extend roko-runtime rather than new crate | Avoids crate explosion; runtime already has lifecycle + event bus | New `roko-agent-runtime` crate (rejected: too many crates already) |
| 8 extension layers | Matches natural data flow (perceive -> remember -> decide -> act -> communicate -> learn -> recover) | 3 layers (too coarse), 16 layers (too fine-grained) |
| `Option<CognitiveTier>` for gate hook | First-non-None semantics are simple and deterministic | Vote-based (complex), priority-based (ordering headaches) |
| Arena allocator per tick | Prevents heap fragmentation in high-frequency agents | Global allocator (fragmentation), custom slab (over-engineering) |
| Parallel migration with feature flag | Zero-downtime migration, rollback capability | Big-bang rewrite (too risky), gradual inline refactor (too slow) |
| `serde_json::Value` for extension state | Schema-flexible, forward-compatible, serialization-agnostic | Typed state (breaks on schema change), protobuf (over-engineering) |
| VCG auction from existing PromptComposer | Already proven, handles incentive-compatible allocation | Simple priority sort (no incentive compatibility), random (unfair) |
| Pheromones before JSON-RPC A2A | Ambient coordination is simpler and emergent | Direct messaging first (requires addressing, routing, protocol) |
