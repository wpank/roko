# The Golem Vision (Recovered from Bardo PRDs)

## Core Thesis

> "An autonomous agent that cannot die has no reason to be efficient, no pressure to
> learn, and no incentive to share what it knows — mortality is the architectural
> constraint that makes intelligence economically viable." [JONAS-1966]

The Golem is not a "coding assistant that runs tasks." It's an **autonomous entity** with:
- A continuous perception-action loop
- Economic mortality (finite budget = finite life)
- Adaptive cognition (most ticks are cheap; expensive reasoning is gated)
- Embodied knowledge (somatic markers, not just text)
- Social communication (pheromones, A2A protocol)
- Generational learning (genome inheritance across agent lifetimes)

## The 9-Step Heartbeat

Every tick (gamma=5-15s, theta=30-120s) runs this pipeline:

```
1. OBSERVE   — Read environment (chain blocks, file changes, events)
2. RETRIEVE  — Query grimoire/neuro for relevant knowledge
3. ANALYZE   — Compute prediction error (expected vs observed)
4. GATE      — Decide cognitive tier:
                T0 (80%): Deterministic pattern match, no LLM, ~$0
                T1 (15%): Cheap model (Haiku), ~$0.001
                T2 (5%):  Full reasoning (Opus), ~$0.05
5. SIMULATE  — (T1/T2 only) Run candidate actions in sandbox
6. VALIDATE  — (T1/T2 only) Check safety constraints
7. EXECUTE   — (T1/T2 only) Take action (tool calls, transactions)
8. VERIFY    — (T1/T2 only) Confirm outcome matches expectation
9. REFLECT   — Build DecisionCycleRecord, update knowledge
```

**This is the key innovation.** Gating means 80% of agent computation is pure Rust —
no LLM call, no cost. The agent watches, recognizes patterns it's seen before, and only
escalates to expensive reasoning when something novel appears.

## Three Timescales

| Scale | Period | Purpose | Cognitive Level |
|---|---|---|---|
| Gamma | 5-15s | Perception, triage, streaming TA | T0 (deterministic) |
| Theta | 30-120s | Full decision cycle, tool execution | T0/T1/T2 (gated) |
| Delta | ~50 theta ticks | Consolidation, dreams, evolution | Offline (batch) |

## Six Concurrent Cognitive Mechanisms

Running alongside the heartbeat (not steps, but background processes):

1. **Attention Salience** — Binary heap ordered by score:
   `salience = novelty × 0.4 + relevance × 0.35 + urgency × 0.25`
   Exponential decay so stale stimuli lose priority.

2. **Habituation Mask** — Tracks pattern frequency via Blake3 hashes.
   Attenuates novelty for repeated patterns (stops reacting to noise).

3. **Sleep Pressure** — Accumulates with each tick without consolidation.
   Forces dream entry when pressure exceeds threshold.

4. **Event-Driven Wakeup** — Chain events / price spikes / pheromones can
   interrupt dreams and re-enter active mode immediately.

5. **Homeostasis** — Maintains stable operating ranges:
   economic vitality, epistemic confidence, arousal levels.

6. **Compensation/Rollback** — If action produces unexpected outcome,
   trigger compensation before reflecting.

## Extension System (28 Extensions, 7 Layers)

The trait with 20 lifecycle hooks:

```rust
trait Extension: Send + Sync {
    // Session lifecycle
    fn on_boot(&mut self, state: &mut GolemState) -> Result<()>;
    fn on_resume(&mut self, state: &mut GolemState) -> Result<()>;
    fn on_compact(&mut self, state: &mut GolemState) -> Result<()>;
    fn on_branch(&mut self, state: &mut GolemState) -> Result<()>;

    // Input processing
    fn classify_input(&self, input: &Input) -> InputKind; // steer vs message

    // Agent lifecycle
    fn before_agent_start(&mut self, ctx: &mut AgentContext) -> Result<()>;
    fn after_agent_start(&mut self, ctx: &mut AgentContext) -> Result<()>;

    // Turn lifecycle
    fn on_turn_start(&mut self, turn: &mut Turn) -> Result<()>;
    fn assemble_context(&mut self, workspace: &mut CognitiveWorkspace) -> Result<()>;
    fn before_request(&mut self, request: &mut LlmRequest) -> Result<()>;
    fn on_response(&mut self, response: &LlmResponse) -> Result<()>;
    fn after_turn(&mut self, turn: &Turn) -> Result<()>;

    // Tool dispatch
    fn before_tool_call(&mut self, call: &mut ToolCall) -> Result<ToolCallDecision>;
    fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()>;

    // Dream lifecycle
    fn on_dream_start(&mut self, state: &mut GolemState) -> Result<()>;
    fn on_dream_phase(&mut self, phase: DreamPhase) -> Result<()>;
    fn on_dream_outcome(&mut self, outcome: &DreamOutcome) -> Result<()>;

    // Shutdown
    fn on_shutdown(&mut self, state: &GolemState) -> Result<ShutdownVote>;
}
```

**Layer organization:**
```
L0: Foundation   — Heartbeat, Clock, CorticalState, Mortality
L1: Perception   — EventFabric, Probes, ChainSubscription
L2: Memory       — Grimoire, Episodic, Semantic, WorkingMemory
L3: Cognition    — Daimon (affect), Attention, Gating, Habituation
L4: Action       — ToolDispatch, Safety, Execution, Budget
L5: Social       — Pheromones, A2A, OperatorChat, Delegation
L6: Meta         — Dreams, Consolidation, Evolution, Playbooks
L7: Recovery     — Compensation, Rollback, Death, GenomeExtraction
```

## Type-State Lifecycle

```rust
struct Golem<S: State> {
    state: PhantomData<S>,
    inner: GolemInner,
}

// Valid transitions (compile-time enforced):
impl Golem<Provisioning> {
    fn activate(self) -> Golem<Active>;
}

impl Golem<Active> {
    fn tick(&mut self);                    // heartbeat
    fn begin_dream(self) -> Golem<Dreaming>;
    fn begin_death(self) -> Golem<Terminal>;
}

impl Golem<Dreaming> {
    fn dream_cycle(&mut self);
    fn wake(self) -> Golem<Active>;        // normal wake
    fn emergency_wake(self) -> Golem<Active>; // event-driven
    fn begin_death(self) -> Golem<Terminal>;
}

impl Golem<Terminal> {
    fn thanatopsis(self) -> Golem<Dead>;   // extract genome, final state
}

// These are COMPILER ERRORS, not runtime checks:
// dead_golem.tick()    — no method `tick` on Golem<Dead>
// active_golem.dream() — no method `dream_cycle` on Golem<Active>
```

## CorticalState (Lock-Free Shared Perception)

~32 atomic signals where subsystems write pattern signals and read state concurrently:

```rust
struct CorticalState {
    // Perception (updated by probes)
    last_block_number: AtomicU64,
    gas_price_gwei: AtomicU32,
    eth_price_usd: AtomicU32,

    // Affect (updated by daimon)
    pleasure: AtomicI16,       // PAD model
    arousal: AtomicI16,
    dominance: AtomicI16,
    behavioral_phase: AtomicU8,

    // Mortality (updated by death clocks)
    economic_vitality: AtomicU16,
    epistemic_confidence: AtomicU16,
    stochastic_survival: AtomicU16,

    // Attention (updated by salience engine)
    top_stimulus_hash: AtomicU64,
    novelty_score: AtomicU16,

    // Communication
    pheromone_signal: AtomicU64,
}
```

## Event Fabric

```rust
// 50+ event types across 16 subsystems
enum GolemEvent {
    // Heartbeat
    TickStarted { frequency: Frequency, tick_number: u64 },
    TickCompleted { tier: CognitiveTier, duration_ms: u32 },

    // LLM
    InferenceStarted { model: String, input_tokens: u32 },
    InferenceCompleted { output_tokens: u32, cost_usd: f32 },

    // Tool execution
    ToolCallStarted { name: String },
    ToolCallCompleted { name: String, success: bool },

    // Dreams
    DreamCycleStarted,
    DreamPhaseTransition { phase: DreamPhase },
    InsightPromoted { entry_id: String },

    // Mortality
    VitalityChanged { old: f32, new: f32 },
    DeathClockTick { remaining_budget_usd: f32 },

    // Chain
    BlockReceived { number: u64 },
    TransactionTriaged { hash: H256, classification: TxClass },

    // Social
    PheromoneReceived { source: AgentId, signal: PheromoneSignal },
    // ... 30+ more
}
```

Published via `tokio::broadcast` with 10,000-event ring buffer.
WebSocket clients, TUI, extensions all subscribe.

## Three Agent Archetypes

1. **Golem Instance** — The autonomous agent. Reads PLAYBOOK.md + STRATEGY.md.
   Delegates to specialists. Evolves reasoning based on outcomes.

2. **Memory Consolidator** — Manages grimoire lifecycle. Uses ACE pattern
   (Generator-Reflector-Curator) for incremental PLAYBOOK.md updates.

3. **Sleepwalker-Observer** — Half-dream state. Observes 5 market domains.
   Publishes typed artifacts. Revenue from x402 micropayments.

## Genome Inheritance

When a Golem dies, its grimoire is compressed through a "genomic bottleneck"
(≤2048 entries). Successor Golems inherit this compressed knowledge with
confidence discounting. Knowledge transfers across generations.

## Context Governor (Learnable Control)

Context assembly is a **learnable control problem**, not static budgeting:

```
CognitiveWorkspace
  ├── Role section (system prompt, instructions)    — priority 5, always
  ├── Workspace section (structure, cross-refs)     — priority 4, cacheable
  ├── Plan section (PRD, brief, task)               — priority 3
  ├── Knowledge section (neuro, playbooks)          — priority 2
  └── Volatile section (per-turn, unique)           — priority 1, drop first

CognitiveWorkspaceDelta
  ├── What changed since last assembly?
  ├── Which categories correlated with good outcomes?
  └── Feedback loop: shrink bad categories, grow good ones
```

Regime-aware (bull/bear), phase-aware (thriving/declining), task-aware.
