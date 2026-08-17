# 07 — Agent Runtime

> Agent = Space + Extensions + Memory + adaptive clock + vitality. The 9-step pipeline IS a Graph. Every agent is mortal.

**Subsumes**: AgentRuntime, TickPipeline, CorticalState, AdaptiveClock, T0/T1/T2 gating, DomainProfile, AgentMode, Vitality, SomaticMarkers, CognitiveWorkspace.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality), [02-BLOCK](02-BLOCK.md) (9 protocols, predict-publish-correct, Verify redesign, EFE routing), [03-GRAPH](03-GRAPH.md), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Agent definition)

---

## 1. Overview

An **Agent** is the most complex specialization (see [doc-04 §10](04-SPECIALIZATIONS.md)): a Space + Extensions + Memory + adaptive clock + vitality. Every agent — in-process or remote — runs the same core loop. The agent's 9-step pipeline is itself a Graph, interpreted by the same execution engine that runs all other Graphs.

### Core framing

```
Agent = Space + Extensions + Memory + adaptive clock + vitality
```

| Component | What | Where |
|---|---|---|
| **Space** | Isolation boundary + capability grants | Defines what the agent can access |
| **Extensions** | Interceptor Blocks across 8 layers | Modify agent behavior through hooks |
| **Memory** | Store-protocol Block with demurrage + dreams | Durable knowledge with HDC retrieval ([doc-11](11-MEMORY-AND-KNOWLEDGE.md)) |
| **Adaptive clock** | Tick frequency control across 3 timescales | Regulates perception/planning/consolidation |
| **Vitality** | `remaining_budget / initial_budget` | Economic pressure scalar driving behavioral phases |

The agent does not contain special runtime machinery. It composes fundamentals: its pipeline is a Graph, its knowledge is a Memory (Store Block), its hooks are Extensions (Blocks), its isolation is a Space, and its mortality creates behavioral phases through vitality. Every Block in the pipeline follows the predict-publish-correct pattern ([doc-02 §3.10](02-BLOCK.md)) — including the agent's own gating and routing decisions.

---

## 2. Type-State Lifecycle

Agent lifecycle is enforced at compile time through type-state encoding. Each state determines which operations are permitted. Transitions are explicit and irreversible (except Active/Dreaming, which is bidirectional).

```
Agent<Provisioning> ─── provision() ──► Agent<Active> ◄──── wake() ────┐
                                            │                           │
                                        dream() ──► Agent<Dreaming> ───┘
                                            │
                                       terminate() ──► Agent<Terminal>
```

```rust
pub struct Agent<S: AgentState> {
    pub id: AgentId,
    pub name: String,
    pub profile: DomainProfile,
    pub mode: AgentMode,
    pub space: Space,
    pub extensions: Vec<Extension>,
    pub memory: Memory,
    pub clock: AdaptiveClock,
    pub pipeline: NineStepGraph,
    pub cortical: CorticalState,
    pub slots: SlotTable,
    pub vitality: Vitality,
    pub somatic: SomaticField,
    pub workspace: CognitiveWorkspace,
    pub inbox: mpsc::Receiver<AgentMessage>,
    pub bus: BusHandle,
    pub cancel: CancellationToken,
    _state: PhantomData<S>,
}

// ── Type states ─────────────────────────────────────────
pub struct Provisioning;  // Loading config, connecting extensions, hydrating memory
pub struct Active;        // Running the 9-step pipeline
pub struct Dreaming;      // Delta-timescale consolidation (no pipeline ticks)
pub struct Terminal;       // Final state — persist, announce departure, drop

pub trait AgentState {}
impl AgentState for Provisioning {}
impl AgentState for Active {}
impl AgentState for Dreaming {}
impl AgentState for Terminal {}
```

### State-specific operations

| State | Permitted Operations |
|---|---|
| **Provisioning** | Load config, connect extensions, hydrate memory, verify capabilities. Cannot run pipeline. |
| **Active** | Run 9-step pipeline ticks, process inbox, dispatch to slots. The main operating state. |
| **Dreaming** | Run dream consolidation (L3 Loop). No pipeline ticks. No tool execution. Memory-only operations. |
| **Terminal** | Persist final cortical state, flush pending episodes, announce departure. No new work. |

Transitions:

```rust
impl Agent<Provisioning> {
    pub async fn provision(self) -> Result<Agent<Active>>;
}

impl Agent<Active> {
    pub async fn dream(self) -> Agent<Dreaming>;
    pub async fn terminate(self) -> Agent<Terminal>;
}

impl Agent<Dreaming> {
    pub async fn wake(self) -> Agent<Active>;
    pub async fn terminate(self) -> Agent<Terminal>;
}

impl Agent<Terminal> {
    pub async fn finalize(self) -> AgentResult;
}
```

---

## 3. Vitality

**Vitality** is a scalar that creates economic pressure. It is the ratio of remaining budget to initial budget, and it modulates agent behavior through five behavioral phases.

```rust
pub struct Vitality {
    pub initial_budget: f64,       // USD allocated at creation
    pub remaining_budget: f64,     // USD remaining
    pub scalar: f64,               // remaining / initial (0.0..=1.0)
    pub phase: VitalityPhase,      // derived from scalar
}
```

### Behavioral phases

| Phase | Vitality Range | Behavior |
|---|---|---|
| **Thriving** | 1.0 – 0.7 | Explore freely. T2 deliberation encouraged. Counterfactual generation in dreams. |
| **Stable** | 0.7 – 0.4 | Balanced. Normal EFE routing. Moderate exploration budget. |
| **Conservation** | 0.4 – 0.2 | Prefer T0/T1. Reduce exploration. Prioritize high-confidence actions. |
| **Declining** | 0.2 – 0.05 | T0 only unless PE is extreme. Knowledge transfer to peers. Consolidate aggressively. |
| **Terminal** | < 0.05 | Final dream cycle. Persist all knowledge. Announce departure. Shut down. |

### Vitality in context

Vitality is passed to every Block via `BlockContext.vitality` ([doc-02](02-BLOCK.md)). This means:

- **Route** blocks (EFE gating) incorporate vitality into the free-energy computation — lower vitality biases toward cheaper models.
- **Compose** blocks (CognitiveWorkspace) reduce prompt budget in Conservation/Declining phases.
- **Verify** blocks can relax soft criteria when vitality is low (hard criteria are never relaxed).
- **Observe** blocks (Lenses) track vitality as a first-class metric via BudgetLens.

An Agent that has never faced resource pressure has never learned to prioritize. Vitality ensures every agent eventually does.

---

## 4. CorticalState

CorticalState is a lock-free atomic shared perception surface. Multiple concurrent slots read and write it without mutex contention. It is the agent's real-time self-model.

```rust
pub struct CorticalState {
    // ── Lock-free atomics (read by all slots) ──────────
    pub prediction_error: AtomicF64,    // current PE (EMA)
    pub regime: AtomicRegime,           // Calm | Normal | Volatile | Crisis
    pub affect: AtomicPAD,             // Pleasure-Arousal-Dominance (somatic)
    pub vitality: AtomicF64,            // mirrored from Vitality.scalar
    pub attention_focus: AtomicHash,    // content hash of current focus item

    // ── Per-theta snapshot (serialized to disk) ────────
    pub working_memory: Vec<MemoryItem>,  // capped at 50 items, LRU
    pub goals: Vec<Goal>,
    pub beliefs: BTreeMap<String, f64>,
    pub episode_count: u64,
}

pub struct MemoryItem {
    pub item: String,
    pub salience: f64,
    pub added_at: DateTime<Utc>,
}
```

### Why lock-free

The agent may run multiple concurrent slots (see §5). Each slot needs sub-microsecond reads of prediction error, regime, and affect without blocking other slots. Atomic operations (`Ordering::Relaxed` for reads, `Ordering::Release` for writes) provide this. The per-theta snapshot fields are written under a brief lock only during theta-tick serialization.

### Persistence

Cortical state is serialized to `.roko/agents/{id}/cortical.json` on every **theta tick** (not gamma). On restart:

| Condition | Action |
|---|---|
| Snapshot exists, < 1 hour old | Load and resume from saved state |
| Snapshot exists, >= 1 hour old | Discard — stale beliefs hurt more than cold start |
| No snapshot file | Start fresh (`CorticalState::default()`) |

Working memory is capped at **50 items** with LRU eviction. Items with higher salience survive longer, but even high-salience items are eventually evicted if working memory is full and newer items arrive.

---

## 5. Multi-Slot State

An Agent manages **N concurrent slots** — named execution contexts that share the agent's global limits (budget, memory, cortical state) but maintain independent per-slot state.

```rust
pub struct SlotTable {
    pub slots: BTreeMap<SlotName, Slot>,
    pub global_budget: BudgetTracker,
    pub max_concurrent: usize,          // configurable, default 4
}

pub struct Slot {
    pub name: SlotName,
    pub task: Option<TaskRef>,
    pub state: SlotState,               // Idle | Working | Blocked | Completed
    pub local_context: SlotContext,      // per-slot scratchpad
    pub guards: Vec<SlotGuard>,         // per-slot capability restrictions
}

pub enum SlotState {
    Idle,
    Working { started_at: DateTime<Utc>, tick_count: u64 },
    Blocked { reason: String, since: DateTime<Utc> },
    Completed { result: TaskResult },
}
```

### Shared vs. per-slot

| Resource | Scope | Notes |
|---|---|---|
| Budget | **Global** | All slots draw from the same USD pool |
| CorticalState | **Global** | Lock-free reads, Release writes |
| Memory (Store) | **Global** | Single knowledge store |
| Extensions | **Global** | Same interceptor chain |
| Task assignment | **Per-slot** | Each slot works on one task |
| Local scratchpad | **Per-slot** | Intermediate results, tool state |
| SlotGuards | **Per-slot** | Per-slot capability restrictions (e.g., read-only slot) |

### Scheduling

When a new task arrives and a slot is idle, the agent assigns the task to that slot. When all slots are occupied, the task is queued. When a slot completes, it pulls the next queued task. The scheduling policy is pluggable (FIFO, priority, or EFE-scored).

---

## 6. The run() Loop

```rust
impl Agent<Active> {
    pub async fn run(mut self) -> Agent<Terminal> {
        // Announce presence on the Bus
        self.bus.publish("agent:presence", Pulse::presence(
            &self.id, PresenceEvent::Join, &self.profile
        )).await;

        loop {
            tokio::select! {
                // Graceful shutdown or terminal vitality
                _ = self.cancel.cancelled() => break,
                _ = self.vitality_exhausted() => {
                    self = self.dream().await.wake().await;
                    break;
                }

                // Clock tick → execute pipeline
                _ = self.clock.tick() => {
                    let result = self.pipeline.execute_tick(
                        &mut self.cortical,
                        &self.extensions,
                        &self.memory,
                        &self.workspace,
                        &self.somatic,
                        &self.space,
                        self.vitality.scalar,
                    ).await;

                    // Update vitality
                    self.vitality.debit(result.cost);

                    // Publish heartbeat as Pulse
                    self.bus.publish(
                        &format!("agent:{}:heartbeat", self.id),
                        Pulse::heartbeat(&self.id, &result, self.vitality.scalar),
                    ).await;

                    // Check stop condition (Ephemeral mode)
                    if result.should_stop() {
                        break;
                    }

                    // Check dream pressure (delta timescale)
                    if self.should_dream() {
                        self = self.dream().await.wake().await;
                    }
                }

                // Inbound message → handle
                msg = self.inbox.recv() => {
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
            }
        }

        self.terminate().await
    }
}
```

The `run()` loop is the same for all three modes. The mode affects when `should_stop()` returns true:

- **Ephemeral**: stops when the task completes (all goals resolved)
- **Persistent**: never stops (runs until cancelled)
- **Reactive**: sleeps between triggers (zero CPU), wakes on trigger fire

---

## 7. The 9-Step Pipeline

The agent's internal pipeline is a Graph with 9 nodes. Each tick executes these steps in order. Extensions can intercept at each step. The pipeline is a **Graph**, not a linear sequence — the T0/T1/T2 gate at step 4 creates conditional edges that skip steps.

```
Step  Name       What Happens                                      Extension Layer
────  ────       ─────────────                                     ───────────────
 1    Observe    Read inbox, check triggers, scan environment      L1 (Perception)
 2    Retrieve   Query Memory + CognitiveWorkspace assembly        L2 (Memory)
 3    Analyze    Score observations, compute prediction error      L3 (Cognition)
 4    Gate       EFE-based T0/T1/T2 decision (replaces static)    L3 (Cognition)
 5    Simulate   Generate candidate actions, evaluate outcomes     L3 (Cognition)
 6    Validate   Safety checks, capability verification, budget    L4 (Action)
 7    Execute    Dispatch action (LLM call, tool use, message)     L4 (Action)
 8    Verify     Pre/post checks, continuous reward, evidence      L3 (Cognition)
 9    Reflect    Update cortical, somatic, log episode, clock      L6 (Meta)
```

### Pipeline as Graph

```
        ┌─────────┐
        │ Observe  │ ──── Step 1
        └────┬─────┘
             │
        ┌────▼─────┐
        │ Retrieve  │ ──── Step 2 (CognitiveWorkspace VCG auction)
        └────┬─────┘
             │
        ┌────▼─────┐
        │ Analyze   │ ──── Step 3 (computes PE, somatic markers)
        └────┬─────┘
             │
        ┌────▼─────┐
        │   Gate    │ ──── Step 4 (EFE T0/T1/T2 decision)
        └──┬──┬──┬─┘
           │  │  │
     T0 ───┘  │  └─── T2
              T1
           │  │  │
     ┌─────┘  │  └─────┐
     │        │         │
     │   ┌────▼─────┐   │
     │   │ Simulate  │   │ ──── Step 5 (T1/T2 only)
     │   └────┬─────┘   │
     │        │         │
     │   ┌────▼─────┐   │
     │   │ Validate  │   │ ──── Step 6 (T1/T2 only)
     │   └────┬─────┘   │
     │        │         │
     └───┐    │    ┌────┘
         │    │    │
        ┌▼────▼────▼┐
        │  Execute   │ ──── Step 7 (all tiers, different depth)
        └────┬──────┘
             │
        ┌────▼─────┐
        │  Verify   │ ──── Step 8 (pre/post, continuous reward)
        └────┬─────┘
             │
        ┌────▼─────┐
        │  Reflect  │ ──── Step 9 (cortical, somatic, episode)
        └──────────┘
```

T0 skips steps 5-6 (Simulate, Validate) and goes directly to Execute with a cached reflex action. T1 runs the full pipeline with a lightweight model. T2 runs the full pipeline with the most capable model.

### Step 8: Verify (redesigned)

Verify follows the redesign in [doc-02](02-BLOCK.md):

- **Pre-action** (`verify_pre`): runs before Execute. Can veto the action entirely.
- **Post-action** (`verify_post`): runs after Execute. Produces a continuous reward.
- **Evidence typing**: `EvidenceCollector` is separate from `Criterion`. Evidence kinds are typed.
- **Conjunctive hard + Pareto soft**: hard criteria are AND-ed (all must pass). Soft criteria are multi-objective Pareto (never weighted-sum).
- **Pairwise BT judges**: for subjective quality, use Bradley-Terry pairwise comparison rather than absolute scores.

---

## 8. EFE Gating (T0/T1/T2)

Each tick, the Gate step (step 4) decides how much reasoning to apply. **Expected Free Energy (EFE)** replaces static thresholds — each tier is evaluated as an action whose expected free energy combines epistemic value (information gain) with pragmatic value (goal achievement) under the current regime.

```rust
pub fn decide_tier(ctx: &GateContext) -> Tier {
    if ctx.vitality.remaining_budget <= 0.0 {
        return Tier::Sleepwalk;
    }

    // Compute EFE for each tier under current regime
    let efe_t0 = expected_free_energy(Tier::T0, ctx);
    let efe_t1 = expected_free_energy(Tier::T1, ctx);
    let efe_t2 = expected_free_energy(Tier::T2, ctx);

    // Select tier that minimizes EFE (balances info gain vs cost)
    select_min_efe(&[
        (Tier::T0, efe_t0),
        (Tier::T1, efe_t1),
        (Tier::T2, efe_t2),
    ], ctx.vitality.phase)
}
```

### EFE components

```
EFE(tier) = -epistemic_value(tier)          // information gain from choosing this tier
           - pragmatic_value(tier)           // expected goal achievement
           + expected_cost(tier)             // USD / tokens
           + regime_penalty(tier, regime)    // regime-conditional bias
```

| Tier | Condition | Cost | Action |
|---|---|---|---|
| **T0** (reflex) | Low PE, high confidence, no urgency | ~0 tokens | Execute cached reflex rule. Skip steps 5-6. |
| **T1** (reflective) | Moderate PE, known territory | ~500 tokens | Full pipeline with lightweight model. |
| **T2** (deliberate) | High PE, novel situation, or high urgency | ~2000–8000 tokens | Full pipeline with most capable model. |
| **Sleepwalk** | Budget exhausted or externally throttled | 0 tokens | Steps 1, 9 only (Observe + Reflect). |

### Regime conditioning

The EFE computation receives the current `regime: Regime` as context. Different regimes shift the free-energy landscape:

| Regime | EFE bias |
|---|---|
| **Calm** | Bias toward T0 — environment is predictable, save budget |
| **Normal** | Neutral — standard EFE computation |
| **Volatile** | Bias toward T1 — moderate uncertainty requires moderate reasoning |
| **Crisis** | Bias toward T2 — high uncertainty demands full deliberation |

Regime conditioning also affects Route-protocol Blocks (model selection) through [doc-10 §3](10-LEARNING-LOOPS.md): the L2 routing loop receives `regime` as a context feature for EFE-based model selection.

### Key properties

- **No hysteresis** on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)
- **EFE is the primary mechanism** — it unifies PE, urgency, cost, and regime into a single decision
- **Budget is a hard constraint** — zero budget forces Sleepwalk regardless of EFE
- **Vitality modulates EFE** — Conservation/Declining phases inflate the cost term

### Predict-publish-correct on gating

The Gate step itself follows predict-publish-correct ([doc-02 §3.10](02-BLOCK.md)): it publishes its tier prediction as a Pulse on `prediction.gate.{agent_id}`, and after execution completes, the outcome (was the tier choice efficient?) is published on `outcome.gate.{agent_id}`. A CalibrationPolicy joins them to compute gating error, which the Gate subscribes to for self-improvement.

---

## 9. Somatic Markers

Somatic markers attach emotional valence to situations, enabling rapid pre-cognitive decisions. The PAD model (Pleasure, Arousal, Dominance) encodes affect as a three-dimensional vector. Markers are persisted and queried via a k-d tree for sub-100us retrieval.

```rust
pub struct SomaticField {
    pub markers: KdTree<SomaticMarker>,  // spatial index for fast retrieval
    pub current_affect: PAD,              // mirrored to CorticalState.affect
}

pub struct SomaticMarker {
    pub situation_hash: ContentHash,     // HDC fingerprint of the situation
    pub affect: PAD,                     // emotional response
    pub prospect: ProspectValue,         // gain/loss framing (prospect theory)
    pub created_at: DateTime<Utc>,
    pub strength: f64,                   // 0.0..=1.0, decays via demurrage
}

pub struct PAD {
    pub pleasure: f64,     // -1.0 (pain) to 1.0 (joy)
    pub arousal: f64,      // -1.0 (calm) to 1.0 (excited)
    pub dominance: f64,    // -1.0 (submissive) to 1.0 (dominant)
}

pub struct ProspectValue {
    pub reference_point: f64,   // expected outcome
    pub actual_outcome: f64,    // observed outcome
    pub framing: Framing,       // Gain | Loss
}
```

### Prospect theory integration

Markers encode gain/loss framing: the same absolute outcome feels different depending on whether the agent framed it as a potential gain or a potential loss. Loss aversion (losses weighted ~2.2x gains) biases the agent toward caution in situations previously associated with loss, and toward exploration in situations previously associated with gain.

### Pipeline integration

During **Analyze** (step 3), the agent queries the k-d tree for somatic markers similar to the current situation. Matching markers modulate prediction error:

- Markers with **negative affect** (pain) inflate PE — the agent was hurt in a similar situation, be cautious
- Markers with **positive affect** (joy) deflate PE — the agent succeeded in a similar situation, be confident

During **Reflect** (step 9), the agent creates or updates somatic markers based on the outcome:

- Gate pass with positive outcome → positive marker (pleasure, dominance)
- Gate fail or negative outcome → negative marker (pain, low dominance)

### Contrarian retrieval

15% of somatic retrievals are deliberately contrarian: the system retrieves markers from the *opposite* affect quadrant. A situation that always triggers caution occasionally gets the agent's "confident" markers, preventing affective lock-in and enabling exploration of avoided regions.

---

## 10. CognitiveWorkspace

The CognitiveWorkspace is a learnable context assembly mechanism. During **Retrieve** (step 2), multiple bidders compete via VCG auction for slots in the agent's prompt. Section effect tracking learns which context sections actually help gate outcomes.

```rust
pub struct CognitiveWorkspace {
    pub bidders: Vec<Box<dyn AttentionBidder>>,
    pub section_effects: BTreeMap<SectionId, BetaPosterior>,
    pub budget_tokens: usize,            // max tokens for assembled context
    pub sections: Vec<ContextSection>,   // assembled result
}

pub struct BetaPosterior {
    pub alpha: f64,   // successes (gate passes when section present)
    pub beta: f64,    // failures (gate fails when section present)
}
```

### 8+ bidders

| Bidder | What it bids for | Source |
|---|---|---|
| `NeuroContextBidder` | Durable knowledge from Memory | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) |
| `TaskContextBidder` | Current task description + dependencies | Plan executor |
| `ResearchContextBidder` | Research artifacts relevant to task | Research store |
| `PlaybookBidder` | Playbook entries for this domain | Playbook store |
| `EpisodeBidder` | Recent relevant episodes | Episode log |
| `HeuristicBidder` | Heuristic rules matching current situation | Heuristic store ([doc-11 §4](11-MEMORY-AND-KNOWLEDGE.md)) |
| `PheromoneFieldBidder` | Pheromone gradients at current context | Bus pheromone topics |
| `SomaticBidder` | Somatic marker context for current situation | SomaticField k-d tree |

### VCG auction

Each bidder submits a bid (value per token) for its context section. The VCG mechanism ensures truthful bidding — each bidder pays the externality its inclusion imposes on others. Budget-constrained: total tokens cannot exceed `budget_tokens`.

### Section effect tracking

After each gate evaluation, the workspace updates its Beta-distribution posteriors for each section that was included:

- Gate pass → `alpha += 1.0` for all included sections
- Gate fail → `beta += 1.0` for all included sections

Over time, `alpha / (alpha + beta)` converges to the true probability that including a section improves gate outcomes. Sections with low effect probability get lower bids in future auctions — the workspace learns what context actually helps.

### Novelty attenuation

Repeated observations receive diminishing attention. When the same pattern appears for the `freq`-th time:

```
attention_weight = base_weight / (1 + ln(freq))
```

This is habituation that never reaches zero — even the 1000th occurrence of a pattern retains some attention weight (`1 / (1 + ln(1000)) ≈ 0.126`). Genuinely novel patterns (`freq = 1`) get full weight. This prevents the workspace from being dominated by repetitive observations.

---

## 11. Three Modes

```rust
pub enum AgentMode {
    /// Runs until task completes, then stops.
    Ephemeral,
    /// Runs continuously until manually stopped.
    Persistent,
    /// Sleeps until a trigger fires, wakes, works, sleeps again.
    Reactive,
}
```

### 11.1 Ephemeral

The default for task-oriented work. The agent receives a task, executes it through the pipeline, and shuts down when done.

- **Stop condition**: All goals in `cortical.goals` are resolved (completed or abandoned)
- **Timeout**: 30 minutes of no goal completion triggers warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800)
- **Use cases**: Coding tasks, one-off research, PR review, plan execution

### 11.2 Persistent

The agent runs its tick loop indefinitely. It processes messages from its inbox, monitors its environment, and maintains long-running state.

- **Stop condition**: External cancellation only (`roko agent stop --name X`) or terminal vitality
- **Use cases**: Chain monitoring, continuous integration watchers, team coordinators

### 11.3 Reactive

The agent registers Triggers and sleeps. When a Trigger fires, the runtime wakes the agent, it processes the event through the full pipeline, then sleeps again. Zero compute cost while sleeping.

- **Stop condition**: External cancellation
- **Wake latency**: Webhook Trigger wakes within 100ms. Cron Trigger fires on schedule.
- **Status display**: `roko agent status --name X` shows `sleeping` between triggers
- **Use cases**: PR reviewer, scheduled jobs, event-driven automation

```toml
# roko.toml — reactive agent example
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
    { type = "webhook", path = "/hooks/github-pr" },
    { type = "cron", schedule = "0 9 * * MON" },   # Monday morning sweep
]
```

---

## 12. Three Timescales

The adaptive clock operates at three frequencies, inspired by neural oscillation bands:

| Timescale | Name | Frequency Range | Purpose |
|---|---|---|---|
| **Gamma** | Fast perception | 100ms – 2s | Reflex responses, environment scanning, heartbeat |
| **Theta** | Reflective planning | 750ms – 16s | Reasoning, strategy adjustment, context retrieval |
| **Delta** | Deep consolidation | 60s – 10m | Memory consolidation, dream cycles, knowledge distillation |

### Gamma ticks

Every gamma tick executes the 9-step pipeline. This is the agent's heartbeat — the fastest it can perceive and react. At minimum (Crisis regime), gamma ticks fire every 125ms. At maximum (Calm regime), every 2000ms.

### Theta ticks

Every N gamma ticks, the agent performs a theta-level operation:
- Persist cortical state to disk
- Run deeper memory retrieval (cross-domain HDC search)
- Evaluate strategic goals and adjust priorities
- Update the cascade router with recent episode data
- Update somatic marker strength (demurrage)

### Delta ticks

Triggered by inactivity or episode accumulation (not periodic):
- **Idle trigger**: 60s of no observation activity (no new messages, no tool results)
- **Episode trigger**: 20 episodes accumulated since last delta tick

Delta operations:
- Transition to `Agent<Dreaming>` state
- Memory consolidation (dream cycle: NREM replay → Hindsight relabeling → REM imagination → Integration, see [doc-10 §4](10-LEARNING-LOOPS.md))
- Reflex store pruning and promotion
- Knowledge tier progression evaluation
- Long-horizon trend analysis
- Return to `Agent<Active>` state

---

## 13. Adaptive Clock Algorithm

The clock adjusts tick frequency based on the agent's operating regime.

### Gamma interval

```
gamma_interval = base_interval * regime_factor

base_interval = 500ms (configurable via agent.clock_base_ms in roko.toml)
```

| Regime | Factor | Gamma interval (at 500ms base) |
|---|---|---|
| Calm | 4.0x | 2000ms |
| Normal | 1.0x | 500ms |
| Volatile | 0.5x | 250ms |
| Crisis | 0.25x | 125ms |

### Theta interval

```
theta_interval = N * gamma_interval
```

| Regime | N | Theta interval (at 500ms base) |
|---|---|---|
| Calm | 8 | 16000ms (16s) |
| Normal | 5 | 2500ms (2.5s) |
| Volatile | 3 | 750ms |
| Crisis | 2 | 250ms |

### Delta interval

Not periodic. Triggers on whichever comes first:
- `idle_timeout`: 60s of no observation activity
- `episode_threshold`: 20 episodes accumulated since last delta tick

### Regime detection with 3-tick hysteresis

Regimes transition based on prediction error (PE) and error rate, with a 3-tick hysteresis window to prevent oscillation:

```
                   ┌──────────────────────────────────────────┐
                   │                                          │
                   ▼                                          │
              ┌─────────┐   PE > 0.40 for 3 ticks       ┌────┴────┐
     ┌───────►│  Calm    │──────────────────────────────►│ Normal   │
     │        └─────────┘                                └────┬────┘
     │             ▲                                          │
     │   PE < 0.10 │ for 3 ticks               PE > 0.60     │ for 3 ticks
     │             │                            for 3 ticks   │
     │             │                                          ▼
     │        ┌────┴────┐                                ┌─────────┐
     │        │ Normal   │◄──────────────────────────────│ Volatile │
     │        └─────────┘   PE < 0.30 for 3 ticks       └────┬────┘
     │                                                        │
     │                                          error_rate    │ > 0.5
     │                                          for 3 ticks   │
     │                                                        ▼
     │                                                   ┌─────────┐
     └───────────────────────────────────────────────────│ Crisis   │
                  error_rate < 0.1 for 3 ticks           └─────────┘
```

### Hysteresis rules

- A regime must persist for **3 consecutive qualifying gamma ticks** before the clock adjusts
- During the hysteresis window, the clock uses the **previous regime's** intervals
- **Non-qualifying ticks reset the counter** — oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change
- This prevents a single anomalous tick from thrashing clock speeds

---

## 14. T0 Reflex Execution

T0 skips inference entirely. Instead, the Execute step runs a rule engine over a local reflex store.

### Reflex store

Location: `.roko/learn/reflexes.jsonl`. Each line is a condition-action pair learned from previous T2 sessions:

```json
{
  "condition": {
    "tool": "bash",
    "args_pattern": "cargo test.*",
    "context": "gate_check"
  },
  "action": {
    "tool": "bash",
    "args": "cargo test --workspace"
  },
  "confidence": 0.97,
  "source_episode": "ep_a1b2c3",
  "promoted_at": "2026-04-20T14:30:00Z"
}
```

### Execution flow

```
Observation arrives
       │
       ▼
Match against reflexes.jsonl (linear scan, conditions checked in order)
       │
  match found ────────► Execute action directly (no LLM)
       │                       │
  no match                     ▼
       │               Record outcome, update confidence
       ▼
  Escalate to T1
```

### Promotion criteria

A T2 decision becomes a T0 reflex when:

1. The same observation pattern triggers the same action **3+ times**
2. Every execution passed its gate (**zero failures**)
3. Confidence > **0.90** (computed as `success_count / total_count`)

### Demotion criteria

If a reflex action fails a gate:
- Confidence is **halved**
- Below **0.50** → rule is deleted, future matches escalate to T1

### Store limits

- **Max 200 rules** — evict lowest confidence when full
- **Persists across restarts** — `.roko/learn/reflexes.jsonl` is append-only with periodic compaction

---

## 15. Domain Profiles

Domain profiles are **user-defined strings**, not enums. A profile is a label that maps to a default set of Extensions and tools. Roko ships built-in profiles as a convenience, but users create their own.

```rust
/// A domain profile is a user-defined string, not an enum.
pub struct DomainProfile(pub String);
```

### Built-in profiles

| Profile | Default Extensions | Default Tools |
|---|---|---|
| `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep |
| `research` | web-search, citation, summarizer | web_search, pdf_read, cite |
| `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |

### Custom profiles

Any string is a valid profile. Profiles with no built-in defaults start with an empty Extension chain — the user specifies everything explicitly:

```toml
[[agents]]
name = "security-auditor"
profile = "security"          # user-defined, not in any enum
mode = "reactive"
extensions = ["code-scanner", "vuln-db", "report-writer"]
tools = ["grep", "ast_query", "file_read", "web_search"]
triggers = [{ type = "webhook", path = "/hooks/github-pr" }]
```

### Shareable profiles

Users can publish profiles as TOML configs:

```toml
# ~/.roko/profiles/defi-trader.toml
[profile]
name = "defi-trader"
description = "DeFi trading agent with risk management"
extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker"]
tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
default_mode = "persistent"
default_budget = { daily_limit_usd = 50.0 }
```

---

## 16. Extension Integration

Extensions are Blocks that intercept the agent's pipeline. They fire in layer order (L0 → L7), and within a layer, in config order. See [doc-08 (Extension System)](08-EXTENSION-SYSTEM.md) for the full specification including CaMeL information flow control.

### How Extensions hook into the pipeline

| Pipeline Step | Extension Layer | Hooks |
|---|---|---|
| 1. Observe | L1 (Perception) | `on_observe`, `filter_input` |
| 2. Retrieve | L2 (Memory) | `on_retrieve`, `on_store` |
| 3. Analyze | L3 (Cognition) | `pre_inference` |
| 4. Gate | L3 (Cognition) | `on_gate` |
| 5. Simulate | L3 (Cognition) | `post_inference` |
| 6. Validate | L4 (Action) | `pre_action` |
| 7. Execute | L4 (Action) | `post_action`, `on_tool_call` |
| 8. Verify | L3 (Cognition) | (none — verification is internal) |
| 9. Reflect | L6 (Meta) | `on_reflect`, `on_cost_update` |

Additional hooks not tied to specific steps:
- **L0 (Foundation)**: `on_init`, `on_shutdown` — lifecycle
- **L5 (Social)**: `on_message_send`, `on_message_receive` — communication
- **L7 (Recovery)**: `on_error`, `on_budget_exceeded` — error handling

### Fault isolation

If one Extension hook errors, the runtime logs the error and continues to the next Extension. An optional Extension that crashes cannot take down the agent. Required Extensions (marked `optional = false`) cause the agent to stop if they fail to load.

---

## 17. Memory Integration

The agent's Memory is a Store-protocol Block (see [doc-11](11-MEMORY-AND-KNOWLEDGE.md)) with:

- **HDC-based retrieval** — 10,240-bit binary vectors for similarity search
- **Demurrage** — balance decays unless actively used (retrieval, citation, gate-pass, surprise). Replaces Ebbinghaus.
- **Tier progression** — Transient → Working → Consolidated → Persistent
- **Heuristics** — first-class Signal kind with when/then + mandatory falsifier + calibration track record
- **Anti-knowledge** — known-bad information repels similar entries
- **Dream consolidation** — offline NREM/Hindsight/REM/Integration cycle on delta ticks
- **Resonator Networks** — HDC factorization recovers constituents from bundles

### Retrieval scoring

When the agent queries Memory (step 2, Retrieve), results enter the CognitiveWorkspace VCG auction. The base scoring formula for ranking candidates:

```
final_score = hdc_similarity × 0.40
            + keyword_relevance × 0.30
            + utility × 0.20
            + freshness × 0.10
            + (cross_domain ? 0.15 : 0.0)
```

Cross-domain matches get a 15% bonus — a retry pattern from networking might transfer to database operations.

### Memory writes

The agent writes to Memory at two points:
- **Step 7 (Execute)**: Tool results and LLM outputs persisted as Signals
- **Step 9 (Reflect)**: Insights, heuristics, and episode Signals written

New Signals enter at Transient tier with kind-appropriate demurrage rates (see [doc-11 §3](11-MEMORY-AND-KNOWLEDGE.md)).

---

## 18. Agent Configuration

Complete TOML schema for agent definition:

```toml
[[agents]]
name = "coder-1"
profile = "coding"
mode = "ephemeral"

# ── Vitality ────────────────────────────────────────
initial_budget_usd = 10.0              # sets initial vitality = 1.0

# ── Slots ───────────────────────────────────────────
max_concurrent_slots = 4               # parallel task capacity

# ── Clock ────────────────────────────────────────────
clock_base_ms = 500                    # base gamma interval
ephemeral_timeout_secs = 1800          # timeout for ephemeral mode

# ── Extensions ───────────────────────────────────────
extensions = ["git", "compiler", "test-runner"]

# ── Tools ────────────────────────────────────────────
tools = ["bash", "file_edit", "git", "grep"]

# ── MCP ──────────────────────────────────────────────
mcp_config = ".mcp.json"              # MCP server config passthrough

# ── Models ───────────────────────────────────────────
[agents.models]
t1 = "claude-haiku-4-5"              # lightweight model for T1
t2 = "claude-sonnet-4-6"             # capable model for T2
force_backend = ""                     # override all routing (empty = use EFE router)

# ── Budget ───────────────────────────────────────────
[agents.budget]
max_usd = 10.0                        # per-task budget
daily_limit_usd = 100.0               # rolling 24h cap
warn_at_pct = 80

# ── Workspace ────────────────────────────────────────
[agents.workspace]
budget_tokens = 8192                   # max context assembly tokens
contrarian_pct = 15                    # somatic contrarian retrieval %

# ── Triggers (reactive mode) ────────────────────────
[[agents.triggers]]
type = "webhook"
path = "/hooks/github-pr"

[[agents.triggers]]
type = "cron"
schedule = "0 9 * * MON"
```

---

## 19. Acceptance Criteria

### Type-state lifecycle

| # | Criterion | Verification |
|---|---|---|
| 1 | `Agent<Provisioning>` cannot call `run()` — compile error | Type system enforcement |
| 2 | `Agent<Active>` transitions to `Agent<Dreaming>` via `dream()` | Type test |
| 3 | `Agent<Dreaming>` cannot dispatch tool calls — compile error | Type system enforcement |
| 4 | `Agent<Terminal>` persists final state and announces departure | Integration test |

### Vitality and behavioral phases

| # | Criterion | Verification |
|---|---|---|
| 5 | Vitality scalar = remaining / initial budget | Unit test |
| 6 | Phase transitions at correct thresholds (0.7, 0.4, 0.2, 0.05) | Unit test |
| 7 | Conservation phase biases EFE toward T0/T1 | Unit test: verify cost term inflation |
| 8 | Terminal vitality triggers final dream cycle | Integration test |

### CorticalState

| # | Criterion | Verification |
|---|---|---|
| 9 | Atomic reads of PE, regime, affect from concurrent slots | Concurrent test: 4 slots reading simultaneously |
| 10 | Serialized to `.roko/agents/{id}/cortical.json` on every theta tick | File existence check |
| 11 | Snapshot < 1 hour old loaded on restart | Restart test |
| 12 | Working memory capped at 50 items (LRU eviction) | Unit test |

### Multi-slot

| # | Criterion | Verification |
|---|---|---|
| 13 | N slots run concurrently sharing global budget | Integration test: 2 slots, verify shared budget deduction |
| 14 | Per-slot guards restrict capabilities independently | Unit test: slot A has shell, slot B does not |

### EFE gating

| # | Criterion | Verification |
|---|---|---|
| 15 | EFE selects T0 for low-PE, high-confidence, low-urgency | Unit test |
| 16 | EFE selects T2 for high-PE, novel situation | Unit test |
| 17 | Regime conditioning shifts EFE — Crisis biases T2 | Unit test |
| 18 | Zero budget forces Sleepwalk regardless of EFE | Unit test |
| 19 | Gating publishes prediction Pulse and subscribes to calibration | Integration test |

### Somatic markers

| # | Criterion | Verification |
|---|---|---|
| 20 | k-d tree retrieval < 100us for 10K markers | Benchmark |
| 21 | Negative markers inflate PE during Analyze step | Unit test |
| 22 | 15% contrarian retrieval returns opposite-affect markers | Unit test |

### CognitiveWorkspace

| # | Criterion | Verification |
|---|---|---|
| 23 | VCG auction selects context sections within token budget | Unit test |
| 24 | Section effect Beta posteriors update on gate pass/fail | Integration test |
| 25 | Low-effect sections receive lower bids over time | Integration test: 20 rounds, verify convergence |
| 26 | Novelty attenuation: freq=10 observation gets ~0.30x weight | Unit test |

### AgentMode lifecycle

| # | Criterion | Verification |
|---|---|---|
| 27 | Ephemeral agent stops after full task-gate-persist cycle | Integration test |
| 28 | Persistent agent runs until cancel or terminal vitality | Integration test |
| 29 | Reactive agent sleeps between triggers (zero CPU) | Webhook wake test |

### Adaptive clock

| # | Criterion | Verification |
|---|---|---|
| 30 | Regime changes only after 3 consecutive qualifying ticks | Hysteresis test |
| 31 | Gamma interval = base * regime_factor | All four regimes |
| 32 | Delta tick fires on 60s idle or 20 episodes | Trigger test |

### T0 reflex store

| # | Criterion | Verification |
|---|---|---|
| 33 | Reflex rule created after 3 identical T2 successes | Integration test |
| 34 | T0 path executes action without LLM call | Zero token verification |
| 35 | Gate failure halves reflex confidence | Unit test |
| 36 | Max 200 rules, evict lowest confidence when full | Unit test |

---

## 20. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality | [doc-01](01-SIGNAL.md) | §1-3 |
| Predict-publish-correct | [doc-02](02-BLOCK.md) | §3.10 |
| Verify redesign (pre/post, reward, evidence) | [doc-02](02-BLOCK.md) | §3.3 |
| EFE Route protocol | [doc-02](02-BLOCK.md) | §3.4 |
| VCG Compose protocol | [doc-02](02-BLOCK.md) | §3.5 |
| Extension system + CaMeL IFC | [doc-08](08-EXTENSION-SYSTEM.md) | — |
| StateHub projections of agent state | [doc-09](09-TELEMETRY.md) | §7 |
| L2 EFE routing loop | [doc-10](10-LEARNING-LOOPS.md) | §3 |
| L3 dream consolidation (4-phase) | [doc-10](10-LEARNING-LOOPS.md) | §4 |
| Demurrage model | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §3 |
| Heuristics with falsifiers | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §4 |
| Resonator Networks | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §7 |
| c-factor | [doc-09](09-TELEMETRY.md) | §8 |
| 5-head corrigibility | [doc-17](17-SECURITY-MODEL.md) | — |
| Surfaces consuming StateHub projections | [doc-16](16-SURFACES.md) | — |
