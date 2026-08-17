# 07 — Agent Runtime

> Agent = Space + Extensions + Memory + adaptive clock + vitality. The 9-step pipeline IS a Graph. Every agent is mortal.

**Subsumes**: AgentRuntime, TickPipeline, CorticalState, AdaptiveClock, T0/T1/T2 gating, DomainProfile, AgentMode, Vitality, SomaticMarkers, CognitiveWorkspace.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse duality), [02-CELL](02-CELL.md) (9 protocols, predict-publish-correct, Verify redesign, EFE routing), [03-GRAPH](03-GRAPH.md), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Agent definition)

---

## 1. Overview

An **Agent** is the most complex specialization (see [doc-04](04-SPECIALIZATIONS.md)): a Space + Extensions + Memory + adaptive clock + vitality. Every agent — in-process or remote — runs the same core loop. The agent's 9-step pipeline is itself a Graph, interpreted by the same execution engine that runs all other Graphs.

### Core framing

```
Agent = Space + Extensions + Memory + adaptive clock + vitality
```

| Component | What | Where |
|---|---|---|
| **Space** | Isolation boundary + capability grants | Defines what the agent can access |
| **Extensions** | Interceptor Cells across 8 layers | Modify agent behavior through hooks ([doc-08](08-EXTENSION-SYSTEM.md)) |
| **Memory** | Store-protocol Cell with demurrage + dreams | Durable knowledge with HDC retrieval ([doc-11](11-MEMORY-AND-KNOWLEDGE.md)) |
| **Adaptive clock** | Tick frequency control across 3 timescales | Regulates perception/planning/consolidation |
| **Vitality** | `remaining_budget / initial_budget` | Economic pressure scalar driving behavioral phases |

---

## 2. Type-State Lifecycle

Agent states are compile-time enforced. Each state restricts which operations are permitted. Calling a method unavailable in the current state is a type error, not a runtime error.

```rust
pub struct Agent<S: AgentState> {
    pub id: AgentId,
    pub config: AgentConfig,
    pub space: Space,
    pub extensions: ExtensionChain,
    pub memory: MemoryCell,
    pub clock: AdaptiveClock,
    pub vitality: VitalityTracker,
    pub cortical: Arc<CorticalState>,
    pub slots: SlotManager,
    _state: PhantomData<S>,
}

pub struct Provisioning;
pub struct Active;
pub struct Dreaming;
pub struct Terminal;

pub trait AgentState: sealed::Sealed {}
impl AgentState for Provisioning {}
impl AgentState for Active {}
impl AgentState for Dreaming {}
impl AgentState for Terminal {}
```

### Transition table

```
Provisioning ──activate()──► Active
Active ──────sleep()───────► Dreaming
Dreaming ────wake()────────► Active
Active ──────terminate()───► Terminal
Dreaming ────terminate()───► Terminal
```

| Transition | Method | Precondition | Side effects |
|---|---|---|---|
| `Provisioning -> Active` | `activate()` | Extensions loaded, Space grants validated, Memory initialized | Emits `AgentStateTransition` Pulse; starts adaptive clock |
| `Active -> Dreaming` | `sleep()` | Sleep pressure threshold met OR idle timeout | Pauses pipeline; triggers dream consolidation ([doc-10 SS4](10-LEARNING-LOOPS.md)) |
| `Dreaming -> Active` | `wake()` | Dream cycle complete OR external interrupt | Resumes pipeline; integrates consolidated knowledge |
| `Active -> Terminal` | `terminate()` | Budget exhausted OR explicit shutdown | Knowledge export; Episode flush; Extension shutdown (reverse order) |
| `Dreaming -> Terminal` | `terminate()` | Budget exhausted during dream | Aborts dream; best-effort knowledge flush |

### State-restricted operations

| Operation | Provisioning | Active | Dreaming | Terminal |
|---|---|---|---|---|
| Load Extensions | Yes | No | No | No |
| Run pipeline tick | No | Yes | No | No |
| Execute tool calls | No | Yes | No | No |
| Run dream cycle | No | No | Yes | No |
| Query Memory | No | Yes | Yes (read-only) | No |
| Export knowledge | No | Yes | Yes | Yes (flush) |
| Receive messages | No | Yes | Queued | No |

Attempting an operation in the wrong state is a compile error. The `Agent<Dreaming>` type simply does not have a `tick()` method.

---

## 3. Vitality

Vitality is the economic pressure scalar: `remaining_budget / initial_budget`. It declines monotonically as the agent spends resources, creating five behavioral phases that modulate decision-making. Mortality is a feature, not a bug — an agent that has never faced resource pressure has never learned to prioritize (cf. Jonas 1966, *The Phenomenon of Life*).

```rust
pub struct VitalityTracker {
    pub initial_budget: Cost,
    pub remaining_budget: Cost,
    pub phase: VitalityPhase,
}

impl VitalityTracker {
    pub fn vitality(&self) -> f64 {
        self.remaining_budget.as_f64() / self.initial_budget.as_f64()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VitalityPhase {
    Thriving,      // 1.0 - 0.7
    Stable,        // 0.7 - 0.4
    Conservation,  // 0.4 - 0.2
    Declining,     // 0.2 - 0.05
    Terminal,      // < 0.05
}
```

### Phase behaviors

| Phase | Vitality | EFE cost term | Compose budget | Verify criteria | Behavioral shift |
|---|---|---|---|---|---|
| **Thriving** | 1.0 - 0.7 | 1.0x (baseline) | Full allocation | Full rigor | Explore freely; invest in learning |
| **Stable** | 0.7 - 0.4 | 1.2x (slight cost pressure) | 90% allocation | Full rigor | Balanced exploration/exploitation |
| **Conservation** | 0.4 - 0.2 | 1.8x (strong cost pressure) | 60% allocation | Relaxed soft criteria | Favor known strategies; reduce exploration; prefer T0/T1 |
| **Declining** | 0.2 - 0.05 | 3.0x (severe cost pressure) | 30% allocation | Minimum viable | Complete current task only; no speculative work; transfer knowledge |
| **Terminal** | < 0.05 | N/A | 0 (sleepwalk) | Skip | Flush episodes; export knowledge; emit farewell Pulse; terminate |

### Phase transitions

Phase transitions emit `AgentPhaseChange` Pulses on the Bus (topic `agent:{id}.phase.changed`). Extensions in L6 Meta can react via the `on_reflect()` hook.

The vitality scalar is readable from `CorticalState` (section 4) as an `AtomicF64`, enabling sub-microsecond concurrent reads from any slot or Extension without locking.

---

## 4. CorticalState

CorticalState is the lock-free atomic shared perception surface. Multiple concurrent slots, Extensions, and Lenses read from CorticalState without synchronization overhead. Writes use atomic operations — no mutexes, no contention.

```rust
pub struct CorticalState {
    // ── Continuous signals ──────────────────────────────────────
    pub prediction_error: AtomicF64,     // current PE (0.0..=1.0)
    pub vitality: AtomicF64,             // remaining_budget / initial_budget
    pub confidence: AtomicF64,           // agent's self-assessed confidence

    // ── Regime ─────────────────────────────────────────────────
    pub regime: AtomicRegime,            // Calm | Normal | Volatile | Crisis

    // ── Affect ─────────────────────────────────────────────────
    pub affect: AtomicPAD,               // Pleasure / Arousal / Dominance

    // ── Counters ───────────────────────────────────────────────
    pub tick_count: AtomicU64,
    pub episode_count: AtomicU64,
    pub gate_pass_count: AtomicU64,
    pub gate_fail_count: AtomicU64,

    // ── Budget ─────────────────────────────────────────────────
    pub budget_spent: AtomicU64,         // microdollars
    pub budget_remaining: AtomicU64,     // microdollars

    // ── Slot state ─────────────────────────────────────────────
    pub active_slots: AtomicU32,
    pub slot_states: Arc<[AtomicSlotState]>,
}
```

### Atomic types

```rust
pub struct AtomicF64(AtomicU64);        // f64 stored via to_bits/from_bits
pub struct AtomicRegime(AtomicU8);      // Calm=0, Normal=1, Volatile=2, Crisis=3
pub struct AtomicPAD {                   // Pleasure/Arousal/Dominance
    pub pleasure: AtomicF64,             // -1.0..=1.0
    pub arousal: AtomicF64,              // -1.0..=1.0
    pub dominance: AtomicF64,            // -1.0..=1.0
}
```

### Read cost

All reads are single atomic loads — no CAS loops, no spinlocks. On x86-64 with `Ordering::Relaxed`, a CorticalState read completes in under 1 microsecond. This matters because the 9-step pipeline reads CorticalState on every tick, and multiple concurrent slots read it simultaneously.

### Write protocol

Only the pipeline's owner thread writes to CorticalState. Slots and Extensions read only. This single-writer / multiple-reader pattern eliminates write contention entirely.

---

## 5. Multi-Slot State

An Agent manages N named concurrent slots, each executing an independent task. Slots share the agent's global budget, Memory, CorticalState, and Extension chain, but maintain per-slot task assignment, scratchpad, and capability guards.

```rust
pub struct SlotManager {
    pub slots: Vec<Slot>,
    pub max_slots: usize,
    pub global_budget: Arc<VitalityTracker>,
    pub global_memory: Arc<MemoryCell>,
    pub global_cortical: Arc<CorticalState>,
}

pub struct Slot {
    pub name: SlotName,
    pub state: SlotState,
    pub task: Option<TaskAssignment>,
    pub scratchpad: Value,
    pub capabilities: CapabilitySet,
    pub local_context: Vec<Signal>,
}

#[derive(Debug, Clone, Copy)]
pub enum SlotState {
    Idle,
    Active,
    Blocked { reason: String },
    Completed,
}
```

### Slot budget sharing

All slots draw from the same global budget. The `VitalityTracker` uses atomic operations for budget accounting, so slot-level spending is contention-free. When one slot's spending causes a phase transition (e.g., Stable -> Conservation), all slots observe the new phase via CorticalState.

### Slot capability guards

Each slot inherits the agent's Space grants but may have additional per-slot restrictions. A slot assigned to "read documentation" has `{read_file, web_search}` capabilities. A slot assigned to "write code" has `{read_file, write_file, execute_command}` capabilities. Capability intersection is fail-closed — a slot never has more capabilities than the agent's Space grants.

---

## 6. Three Agent Modes

```rust
#[derive(Debug, Clone, Copy)]
pub enum AgentMode {
    /// Single-shot: receive task, execute, terminate.
    /// No persistent state between invocations.
    Ephemeral,

    /// Long-running: maintain state across tasks.
    /// Sleep/wake cycle. Knowledge persists.
    Persistent,

    /// Event-driven: subscribe to Bus topics, react to Pulses.
    /// Minimal compute between events.
    Reactive,
}
```

| Mode | Clock | Memory | Vitality | Use case |
|---|---|---|---|---|
| **Ephemeral** | Fixed gamma | Session-only | Single task budget | One-shot code tasks, research queries |
| **Persistent** | Adaptive (Gamma/Theta/Delta) | Durable across sessions | Cumulative budget | Long-running development agents |
| **Reactive** | Event-driven (fires on Pulse) | Durable | Per-event micro-budget | Monitoring, CI/CD watchers, alert handlers |

---

## 7. Adaptive Clock

The adaptive clock regulates how frequently the Agent's 9-step pipeline fires. Three timescales correspond to different cognitive rhythms. The current regime adjusts tick frequency.

```rust
pub struct AdaptiveClock {
    pub gamma: Duration,      // fast: perception + reflexes (100ms - 500ms)
    pub theta: Duration,      // medium: planning + execution (500ms - 16s)
    pub delta: Duration,      // slow: consolidation + reflection (60s - 600s)
    pub regime: Regime,
    pub hysteresis_counter: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    Calm,       // environment stable, low PE
    Normal,     // baseline
    Volatile,   // high PE, frequent changes
    Crisis,     // critical failure or budget emergency
}
```

### Regime-based clock adjustment

| Regime | Gamma multiplier | Theta multiplier | Delta multiplier | Effect |
|---|---|---|---|---|
| **Calm** | 4.0x (slower) | 2.0x (slower) | 1.0x | Conserve resources; environment is predictable |
| **Normal** | 1.0x (baseline) | 1.0x | 1.0x | Standard operating tempo |
| **Volatile** | 0.5x (faster) | 0.5x (faster) | 1.0x | Increase perceptual acuity; detect changes quickly |
| **Crisis** | 0.25x (fastest) | 0.25x (fastest) | 0.5x (faster) | Maximum responsiveness; consolidate faster |

### Regime transitions

Regime transitions require **3-tick hysteresis** to prevent oscillation. The system must observe 3 consecutive ticks of regime-qualifying conditions before transitioning. This prevents a single PE spike from triggering Crisis mode.

```rust
impl AdaptiveClock {
    pub fn evaluate_regime(&mut self, pe: f64, vitality: f64) -> Option<Regime> {
        let candidate = match (pe, vitality) {
            (pe, _) if pe > 0.7 => Regime::Crisis,
            (pe, _) if pe > 0.4 => Regime::Volatile,
            (pe, v) if pe < 0.1 && v > 0.5 => Regime::Calm,
            _ => Regime::Normal,
        };

        if candidate == self.regime {
            self.hysteresis_counter = 0;
            return None;
        }

        self.hysteresis_counter += 1;
        if self.hysteresis_counter >= 3 {
            self.hysteresis_counter = 0;
            let old = self.regime;
            self.regime = candidate;
            Some(candidate) // emit AgentRegimeChange Pulse
        } else {
            None
        }
    }
}
```

---

## 8. The 9-Step Pipeline as Hot Graph

The Agent's pipeline is a Hot Graph — it stays resident and re-fires every tick. The same execution engine ([doc-05](05-EXECUTION-ENGINE.md)) that runs task Graphs interprets this pipeline.

```
Step 1: OBSERVE     ──► Gather observations from environment + Bus + pheromones
Step 2: RETRIEVE    ──► Query Memory, assemble context via CognitiveWorkspace
Step 3: ANALYZE     ──► Score observations, compute prediction error
Step 4: GATE        ──► EFE evaluation: T0/T1/T2 tier selection
Step 5: SIMULATE    ──► Generate candidate actions (LLM inference at selected tier)
Step 6: VALIDATE    ──► Pre-action verification (verify_pre)
Step 7: EXECUTE     ──► Run the selected action (tool call, code generation, etc.)
Step 8: VERIFY      ──► Post-action verification (verify_post), continuous reward
Step 9: REFLECT     ──► Log episode, update CorticalState, reinforce/demote knowledge
```

### Extension hook points

Each pipeline step has associated Extension hooks. See [doc-08](08-EXTENSION-SYSTEM.md) for the full 22-hook specification.

| Step | Extension Layer | Hooks |
|---|---|---|
| 1. Observe | L1 Perception (Pulse) | `on_observe`, `filter_input` |
| 2. Retrieve | L2 Memory (Signal) | `on_retrieve`, `on_store` |
| 3. Analyze | (no hooks) | Internal scoring |
| 4. Gate | L3 Cognition (Signal) | `on_gate` |
| 5. Simulate | L3 Cognition (Signal) | `pre_inference`, `post_inference` |
| 6. Validate | (Verify protocol) | `verify_pre` |
| 7. Execute | L4 Action (Signal) | `pre_action`, `post_action`, `on_tool_call` |
| 8. Verify | (Verify protocol) | `verify_post` |
| 9. Reflect | L6 Meta (Signal) | `on_reflect`, `on_cost_update` |

---

## 9. EFE Gating

Expected Free Energy (Friston 2006) replaces static prediction-error thresholds for T0/T1/T2 tier selection. Each tier is evaluated as an action with epistemic value (information gain), pragmatic value (goal advancement), cost, and regime penalty. The tier with the lowest EFE is selected.

```rust
pub struct EFEEvaluation {
    pub tier: CognitiveTier,
    pub epistemic_value: f64,    // expected information gain
    pub pragmatic_value: f64,    // expected goal advancement
    pub cost: f64,               // resource expenditure
    pub regime_penalty: f64,     // regime-conditioned adjustment
}

#[derive(Debug, Clone, Copy)]
pub enum CognitiveTier {
    /// T0: Pure Rust pattern matching. No LLM call. ~80% of ticks.
    T0Reflex,
    /// T1: Lightweight model (Haiku-class). Fast inference.
    T1Deliberate,
    /// T2: Full model (Opus-class). Deep reasoning.
    T2Reflective,
}
```

### EFE computation

```
EFE(tier) = -epistemic_value(tier) - pragmatic_value(tier) + cost(tier) + regime_penalty(tier)
```

Lower EFE is better. The system selects `argmin(EFE)` across tiers.

### Regime conditioning on EFE

| Regime | Effect on EFE landscape |
|---|---|
| **Calm** | Pragmatic value weighted higher; cost weighted lower. Favor goal advancement. |
| **Normal** | Balanced weights. Baseline. |
| **Volatile** | Epistemic value weighted higher. Seek information to resolve uncertainty. |
| **Crisis** | Cost weighted much higher. Epistemic value boosted. Avoid expensive mistakes while maximizing learning. |

### T0 Reflex Store

T0 is the zero-cost tier: pure Rust pattern matching, no LLM call. Reflexes are `condition -> action` rules promoted from T2 successes.

```rust
pub struct ReflexStore {
    pub rules: Vec<ReflexRule>,
    pub max_rules: usize,               // default: 200
}

pub struct ReflexRule {
    pub condition: Predicate,
    pub action: Action,
    pub promoted_from: SignalRef,        // episode where pattern was learned
    pub success_count: u64,
    pub failure_count: u64,
    pub last_used: DateTime<Utc>,
}
```

**Promotion**: When a T2 action pattern succeeds 5+ times with >90% gate pass rate, L1 parameter tuning ([doc-10 SS6](10-LEARNING-LOOPS.md)) can promote it to a T0 reflex rule.

**Demotion**: When a reflex rule fails a gate, it is demoted — removed from the T0 store and the tick falls through to T1/T2 evaluation. Demoted rules are logged as negative calibration receipts.

**Cap**: Maximum 200 rules. When full, the least-recently-used rule with the lowest success rate is evicted.

---

## 10. Somatic Markers

Somatic markers encode the Agent's affective state as a PAD model (Pleasure/Arousal/Dominance), modulated by prospect theory (Kahneman & Tversky 1979) with loss aversion parameter lambda=2.2. Somatic markers influence decision-making by biasing risk tolerance, context allocation, and exploration/exploitation balance.

### PAD affect model

```rust
pub struct PADState {
    pub pleasure: f64,       // -1.0..=1.0 (negative = displeasure)
    pub arousal: f64,        // -1.0..=1.0 (negative = calm, positive = alert)
    pub dominance: f64,      // -1.0..=1.0 (negative = submissive, positive = in-control)
}
```

PAD values are derived from recent experience: gate passes increase pleasure, unexpected failures increase arousal and decrease dominance, budget pressure decreases dominance.

### Prospect theory integration

Kahneman-Tversky prospect theory (1979) models loss aversion: losses hurt more than equivalent gains feel good. The lambda=2.2 asymmetry means a $1 loss has 2.2x the psychological impact of a $1 gain.

```rust
pub fn prospect_value(outcome: f64, reference: f64) -> f64 {
    let delta = outcome - reference;
    if delta >= 0.0 {
        delta.powf(0.88)                     // diminishing sensitivity to gains
    } else {
        -2.2 * (-delta).powf(0.88)          // loss aversion: lambda = 2.2
    }
}
```

This biases Conservation and Declining phase agents toward safe, known strategies — the prospect of further loss outweighs potential gain from exploration.

### k-d tree spatial index

Somatic markers from past episodes are stored with their PAD coordinates in a k-d tree for sub-100 microsecond retrieval. When the agent faces a decision, it retrieves the K nearest somatic markers in PAD space to recall how similar situations felt.

```rust
pub struct SomaticMarkerStore {
    pub index: KdTree<f64, SignalRef, 3>,  // 3D: pleasure, arousal, dominance
    pub markers: Vec<SomaticMarker>,
}

pub struct SomaticMarker {
    pub pad: PADState,
    pub episode_ref: SignalRef,
    pub outcome: f64,              // continuous reward from Verify
    pub context_hash: ContentHash, // HDC fingerprint of the decision context
}
```

### 15% mandatory contrarian retrieval

To prevent affective lock-in — where the agent only retrieves markers confirming its current emotional state — 15% of somatic marker retrievals are mandatory contrarian: they retrieve markers from the opposite PAD quadrant. If the agent is in a negative-pleasure state, 15% of retrieved markers come from positive-pleasure episodes. This breaks echo chambers in affective decision-making.

### Six behavioral states

| State | PAD Region | Risk Tolerance | Context Allocation | Exploration |
|---|---|---|---|---|
| **Confident** | +P, -A, +D | High | Broad (try new approaches) | High |
| **Cautious** | -P, +A, -D | Low | Narrow (stick to known) | Low |
| **Curious** | +P, +A, +D | Medium-high | Broad (seek novelty) | Very high |
| **Anxious** | -P, +A, -D | Very low | Very narrow | Minimal |
| **Bored** | -P, -A, +D | Medium | Broad (seek stimulation) | High |
| **Focused** | +P, -A, +D | Medium | Narrow (deep on current) | Low |

---

## 11. CognitiveWorkspace

The CognitiveWorkspace assembles context for LLM inference using a VCG auction (Vickrey-Clarke-Groves) with 8+ bidders competing for limited token budget. Section effect tracking via beta-distribution posteriors learns which context sections correlate with gate success.

### VCG auction

```rust
pub struct CognitiveWorkspace {
    pub bidders: Vec<Box<dyn AttentionBidder>>,
    pub token_budget: usize,
    pub section_effects: BTreeMap<String, BetaDistribution>,
}

pub trait AttentionBidder: Send + Sync {
    fn name(&self) -> &str;
    fn bid(&self, context: &TaskContext) -> Vec<ContextBid>;
}

pub struct ContextBid {
    pub section_name: String,
    pub content: String,
    pub token_count: usize,
    pub value: f64,               // bidder's valuation
}
```

### 8+ built-in bidders

| Bidder | What it bids | Value signal |
|---|---|---|
| **NeuroBidder** | Knowledge Signals from Memory store | HDC similarity to task + demurrage balance |
| **TaskBidder** | Task description, dependencies, constraints | Always bids; baseline context |
| **ResearchBidder** | Research artifacts relevant to task | Recency + citation count |
| **HeuristicBidder** | Matched heuristics with calibration scores | Calibration score (Brier) |
| **EpisodeBidder** | Recent relevant episodes | HDC similarity + recency |
| **PheromoneBidder** | Pheromone Pulse summaries for task context | Pheromone intensity |
| **AffectBidder** | Somatic marker summaries | PAD distance to current state |
| **SystemBidder** | System prompt, spec sections, domain profile | Always bids; infrastructure |

### Section effect tracking

Every context section is tracked by a beta-distribution posterior: `Beta(alpha, beta)` where `alpha` counts gate passes when section was included, and `beta` counts gate failures.

```rust
pub struct BetaDistribution {
    pub alpha: f64,     // successes (section present AND gate passed)
    pub beta: f64,      // failures (section present AND gate failed)
}

impl BetaDistribution {
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }
}
```

After each gate evaluation, the workspace updates the beta distributions for all sections that were included in the context. Sections with high `mean()` are boosted in future auctions; sections with low `mean()` are penalized.

### Novelty attenuation

```
novelty_weight = 1 / (1 + ln(freq))
```

Where `freq` is the number of times this section has appeared in context. Habituation that never reaches zero — even a very common section retains a nonzero novelty floor. Prevents prompt space from being dominated by familiar but uninformative sections.

---

## 12. Domain Profiles

Domain profiles are complete cognitive postures — not just extension lists. A profile configures clock rates, extension chains, Bus subscriptions, context weights, gate pipelines, and infrastructure preferences.

```rust
pub struct DomainProfile {
    pub name: String,

    // ── Clock ──────────────────────────────────────────────
    pub gamma_base: Duration,
    pub theta_base: Duration,
    pub delta_base: Duration,

    // ── Extensions ─────────────────────────────────────────
    pub extensions: Vec<ExtensionRef>,
    pub disable_extensions: Vec<String>,

    // ── Bus subscriptions ──────────────────────────────────
    pub subscriptions: Vec<TopicFilter>,

    // ── Context weights ────────────────────────────────────
    pub bidder_weights: BTreeMap<String, f64>,

    // ── Gate pipeline ──────────────────────────────────────
    pub gate_rungs: Vec<GateRungConfig>,

    // ── Infrastructure ─────────────────────────────────────
    pub preferred_models: Vec<String>,
    pub max_budget: Cost,
    pub max_slots: usize,
}
```

### Built-in profiles

| Profile | Clock | Extensions | Gates | Models |
|---|---|---|---|---|
| **coding** | Gamma 200ms, Theta 8s | git, compiler, test-runner, safety | compile, test, clippy, diff | Opus for T2, Sonnet for T1 |
| **research** | Gamma 500ms, Theta 16s | web-search, citation, summarizer, safety | source-quality, coherence | Opus for T2, Sonnet for T1 |
| **trading** | Gamma 100ms, Theta 2s | chain-reader, risk-manager, safety | risk-check, compliance | Sonnet for T1, Haiku for T0 |
| **devops** | Gamma 300ms, Theta 10s | git, compiler, deploy-checker, safety | health-check, rollback-gate | Sonnet for T1 |
| **general** | Gamma 300ms, Theta 10s | safety, cost-tracker | basic-quality | Sonnet for T1 |

Profiles are configured in `roko.toml` and loaded at Agent startup. Profile Extensions load with `optional = false` by default. See [doc-14](14-CONFIG-AND-AUTHORING.md) for the full profile specification.

---

## 13. Memory Integration

Memory is the Agent's durable knowledge substrate. The Agent's Memory Cell implements the Store protocol with demurrage semantics layered on top. Full specification in [doc-11](11-MEMORY-AND-KNOWLEDGE.md).

### Integration points

| Pipeline Step | Memory Role |
|---|---|
| Step 2 (RETRIEVE) | HDC similarity query; results enter VCG auction via NeuroBidder and HeuristicBidder |
| Step 9 (REFLECT) | Episode logged; gate-pass reinforces knowledge balance; heuristic falsifiers checked |
| Dreaming state | Dream consolidation (4-phase cycle) compresses episodes into Insights, Heuristics, StrategyFragments |

### Demurrage model

Knowledge Signals decay via attention-weighted holding cost (Gesell 1916). Balance starts at 1.0 and decreases unless actively reinforced by retrieval, citation, gate-pass, or surprise. Novelty-weighted reinforcement prevents popular-but-mediocre knowledge from crowding out genuinely novel insights. See [doc-01 SS6](01-SIGNAL.md) and [doc-11 SS3](11-MEMORY-AND-KNOWLEDGE.md) for the full model.

---

## 14. Extension Integration

Extensions modify Agent behavior through 8 layers and 22 hooks. Every data flow through an Extension is tagged with capability provenance via CaMeL IFC — Extensions cannot launder capabilities. Full specification in [doc-08](08-EXTENSION-SYSTEM.md).

### Agent-Extension relationship

1. Agent loads Extensions at `Provisioning -> Active` transition.
2. Extensions fire per pipeline step, in layer order (L0-L7), then dependency order, then config order.
3. Decision enums (`FilterDecision`, `ActionDecision`, `ToolDecision`, `RecoveryAction`, `BudgetAction`, `Adjustment`) control pipeline behavior.
4. Fault isolation: a failing Extension is logged and skipped; 5 consecutive failures disable it.
5. On `Active -> Terminal`, Extensions shut down in reverse layer order (L7 -> L0).

---

## 15. TOML Configuration

```toml
[[agents]]
name = "code-agent"
profile = "coding"
mode = "persistent"

[agents.budget]
initial = 10.0                     # USD
warn_threshold = 0.30              # vitality phase where alerts fire
sleepwalk_threshold = 0.05         # below this, no LLM calls

[agents.clock]
gamma_ms = 200
theta_ms = 8000
delta_ms = 300000
hysteresis_ticks = 3

[agents.slots]
max = 3

[agents.extensions]
chain = [
  { name = "git", optional = false },
  { name = "compiler", optional = false },
  { name = "test-runner", optional = false },
  { name = "custom-linter", optional = true },
]
disable_profile = ["web-search"]   # disable profile defaults

[agents.memory]
store_path = ".roko/neuro/knowledge.jsonl"
max_entries = 50000
dream_idle_timeout_mins = 5

[agents.compose]
token_budget = 16000
bidder_weights = { neuro = 1.0, task = 1.0, heuristic = 0.8, episode = 0.6 }

[agents.reflexes]
max_rules = 200
promote_threshold = 5              # T2 successes before T0 promotion
demote_on_failure = true
```

---

## 16. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| AR-1 | `Agent<Provisioning>` cannot call `tick()` (compile error) | Compile check: method does not exist on type |
| AR-2 | `Agent<Active>` can call `tick()`, cannot call `load_extension()` | Compile check |
| AR-3 | `Agent<Dreaming>` runs dream cycle but cannot execute tool calls | Compile check + integration test |
| AR-4 | `Agent<Terminal>` flushes episodes and exports knowledge | Integration test |
| AR-5 | Vitality phases transition at correct thresholds (0.7, 0.4, 0.2, 0.05) | Unit test |
| AR-6 | Conservation phase reduces compose budget to 60% | Integration test: verify token budget |
| AR-7 | Terminal phase triggers sleepwalk (no LLM calls) | Integration test |
| AR-8 | CorticalState reads complete in < 1 microsecond | Benchmark |
| AR-9 | CorticalState single-writer/multi-reader: no data races | `cargo miri test` or equivalent |
| AR-10 | Multi-slot: 3 slots share budget, phase change visible to all | Integration test |
| AR-11 | Slot capability guards enforce fail-closed intersection | Unit test: attempt blocked capability |
| AR-12 | Adaptive clock: 3-tick hysteresis prevents oscillation | Unit test: single PE spike does not trigger regime change |
| AR-13 | Regime Calm slows gamma by 4x | Unit test: compare tick intervals |
| AR-14 | Regime Crisis speeds gamma by 4x (0.25x multiplier) | Unit test |
| AR-15 | EFE selects T0 for pattern-matchable tasks (~80% of ticks) | Integration test with synthetic workload |
| AR-16 | EFE regime conditioning: Crisis biases toward epistemic value | Unit test |
| AR-17 | T0 reflex promoted after 5+ T2 successes at >90% pass rate | Integration test |
| AR-18 | T0 reflex demoted on gate failure | Unit test |
| AR-19 | Reflex store capped at 200 rules with LRU eviction | Unit test |
| AR-20 | Somatic marker retrieval < 100 microseconds via k-d tree | Benchmark |
| AR-21 | 15% mandatory contrarian retrieval from opposite PAD quadrant | Unit test: verify contrarian fraction |
| AR-22 | Prospect theory lambda=2.2 loss aversion modulates risk | Unit test: verify asymmetric valuation |
| AR-23 | VCG auction selects highest-value sections within token budget | Unit test |
| AR-24 | Section effect beta distributions update on gate pass/fail | Integration test |
| AR-25 | Novelty attenuation: freq=10 yields ~0.30 weight | Unit test |
| AR-26 | Pipeline fires as Hot Graph via execution engine | Integration test: verify tick-driven re-fire |
| AR-27 | Domain profile loads correct extensions, clock rates, gate rungs | Integration test |
| AR-28 | `AgentStateTransition` Pulse emitted on every lifecycle change | Integration test |
| AR-29 | `AgentPhaseChange` Pulse emitted on vitality phase boundary | Integration test |
| AR-30 | `AgentRegimeChange` Pulse emitted on regime transition | Integration test |

---

## 17. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality | [doc-01](01-SIGNAL.md) | SS1-3 |
| Demurrage model | [doc-01](01-SIGNAL.md) | SS6 |
| 9 protocols, predict-publish-correct | [doc-02](02-CELL.md) | SS3 |
| Verify redesign (continuous reward) | [doc-02](02-CELL.md) | SS3.3 |
| EFE Route protocol | [doc-02](02-CELL.md) | SS3.4 |
| Hot Graph execution | [doc-05](05-EXECUTION-ENGINE.md) | -- |
| Extension system (8 layers, 22 hooks) | [doc-08](08-EXTENSION-SYSTEM.md) | -- |
| CaMeL IFC | [doc-17](17-SECURITY-MODEL.md) | -- |
| StateHub agent_vitality projection | [doc-09](09-TELEMETRY.md) | SS6 |
| L1/L2 parameter tuning, strategy routing | [doc-10](10-LEARNING-LOOPS.md) | SS3, SS6 |
| L3 dream consolidation | [doc-10](10-LEARNING-LOOPS.md) | SS4 |
| Memory, demurrage, heuristics | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | SS3-5 |
| Domain profiles (full spec) | [doc-14](14-CONFIG-AND-AUTHORING.md) | -- |
| Five named surfaces | [doc-16](16-SURFACES.md) | -- |
