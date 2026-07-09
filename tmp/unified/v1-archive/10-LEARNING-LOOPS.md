# 10 — Learning Loops

> Four cybernetic loops at increasing timescales and autonomy. The system improves itself using the same primitives it uses for everything else. Learning is structural: predict-publish-correct is the mechanism, not a separate subsystem.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse, demurrage), [02-BLOCK](02-BLOCK.md) (9 protocols, predict-publish-correct, Verify redesign, EFE routing), [03-GRAPH](03-GRAPH.md), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Loop definition), [07-AGENT-RUNTIME](07-AGENT-RUNTIME.md) (EFE gating, vitality, somatic markers), [09-TELEMETRY](09-TELEMETRY.md) (Lens data feeds Loops, c-factor)

---

## 1. Overview

Roko learns through four feedback loops, each operating at a different timescale with a different level of autonomy. All four are implemented as Loop specializations — Graphs that feed output back to input — using the same fundamentals as every other part of the system.

Learning is not a separate subsystem. It emerges from the **predict-publish-correct** pattern ([doc-02 §3.10](02-BLOCK.md)): every Block publishes its prediction as a Pulse, reality publishes the outcome, a CalibrationPolicy joins them and computes error, and the Block subscribes to its own error topic. This pattern is structural — it uses the same Bus that carries heartbeats and gate verdicts.

| Loop | Name | Timescale | Autonomy | What it adjusts |
|---|---|---|---|---|
| L1 | Parameter Tuning | Gamma (per-tick) | Fully automatic | Continuous params within declared bounds |
| L2 | Strategy Routing | Theta (per-task) | Fully automatic | Selection among pre-approved alternatives via EFE |
| L3 | Knowledge Consolidation | Delta (per-session) | Automatic + auditable | Compression of episodes into durable knowledge + hindsight relabeling |
| L4 | Structural Adaptation | Manual (per-approval) | Requires human approval | Changes to system structure, clade evolution, spec amendments |

Each loop is bounded by explicit safety constraints. Lower loops (L1, L2) operate within tight, pre-declared ranges. Higher loops (L3, L4) have broader scope but stricter oversight.

```
                            Increasing scope ──►
                            Increasing timescale ──►
                            Increasing oversight ──►

    ┌────────────┐  ┌─────────────────┐  ┌──────────────────────┐  ┌────────────────────────┐
    │ L1: Param  │  │ L2: Strategy    │  │ L3: Knowledge        │  │ L4: Structural         │
    │ Tuning     │  │ Routing         │  │ Consolidation        │  │ Adaptation             │
    │            │  │                 │  │                      │  │                        │
    │ gamma      │  │ theta           │  │ delta                │  │ manual                 │
    │ per-tick   │  │ per-task        │  │ per-session          │  │ per-approval           │
    │ automatic  │  │ automatic       │  │ auto + audit         │  │ human approval         │
    │            │  │                 │  │                      │  │                        │
    │ temperature│  │ EFE model       │  │ dream consolidation  │  │ clade evolution        │
    │ thresholds │  │ selection       │  │ hindsight relabeling │  │ spec amendments        │
    │ weights    │  │ failure strategy│  │ heuristic promotion  │  │ CycleQD exploration    │
    └────────────┘  └─────────────────┘  └──────────────────────┘  └────────────────────────┘
```

---

## 2. Predict-Publish-Correct: The Structural Mechanism

Every operator in Roko is a learner. This is not a metaphor — it is the literal mechanism by which all four loops update.

### The pattern

```
Block publishes prediction    →    Pulse on "prediction.{operator}"
Reality publishes outcome     →    Pulse on "outcome.{operator}"
CalibrationPolicy joins them  →    Pulse on "calibration.{operator}.updated"
Block subscribes to error     →    Updates its internal model
```

### How it works per loop

| Loop | What predicts | What is the outcome | What updates |
|---|---|---|---|
| **L1** | Gate threshold predicts pass/fail boundary | Actual gate verdict | EMA moves threshold toward observed optimal |
| **L2** | Router predicts best model for context | Task quality + cost + latency | EFE posterior updates belief about model quality per context |
| **L3** | Insight predicts task success when applied | Downstream gate pass/fail when Insight was in context | Demurrage balance refreshed on success, drained on failure |
| **L4** | Structural proposal predicts quality improvement | Observation window quality metrics | Proposal history informs future proposal generation |

### Why structural, not bolted-on

The predict-publish-correct pattern uses the same Bus that carries agent heartbeats, the same Pulse type that carries streaming output, and the same topic taxonomy that routes lifecycle events. There is no learning-specific infrastructure — the Bus IS the learning fabric.

This means:

1. **Every new Block automatically participates in learning** — it predicts, publishes, and can subscribe to corrections without any learning-specific code.
2. **Learning is observable** — Lenses attach to `prediction.*` and `calibration.*` topics to track learning dynamics.
3. **Learning is auditable** — graduated Signals from predictions and outcomes carry full lineage.

---

## 3. Loop 2: Strategy Routing (Theta) — EFE

**Timescale**: Per-task (750ms to 16s)
**Autonomy**: Fully automatic
**Safety**: Finite set of pre-approved alternatives only

### What it adjusts

L2 selects between existing, pre-approved alternatives using **Expected Free Energy (EFE)**, replacing the previous LinUCB bandit. Each alternative is evaluated by how much it reduces the agent's uncertainty (epistemic value) while advancing its goals (pragmatic value), conditioned on the current regime.

```rust
pub struct StrategySet {
    pub alternatives: Vec<StrategyAlternative>,
    pub efe_model: EFEModel,
    pub regime_context: Regime,
    pub feedback_signal: FeedbackSignalKind,
}

pub struct EFEModel {
    pub beliefs: BTreeMap<AlternativeId, BayesianPosterior>,  // per-alternative quality beliefs
    pub regime_factors: BTreeMap<Regime, Vec<f64>>,           // regime-conditioned priors
    pub exploration_budget: f64,                               // max % of selections for exploration
}
```

### EFE vs LinUCB

| Property | EFE | LinUCB (previous) |
|---|---|---|
| **Model type** | Bayesian generative | Linear contextual bandit |
| **Exploration** | Information gain (epistemic value) — principled | UCB confidence bound — heuristic |
| **Cost awareness** | Native — cost enters the free energy | External constraint only |
| **Regime conditioning** | First-class — different priors per regime | Context feature (indirect) |
| **Timescale separation** | T0/T1/T2 naturally emerge from EFE bound evaluation | Separate gating logic required |

Each timescale corresponds to a different free-energy lower bound:

| Timescale | EFE bound | What it means |
|---|---|---|
| **T0 (gamma)** | Expected free energy under reflex policy | "Can I handle this without thinking?" |
| **T1 (theta)** | Expected free energy under lightweight model | "Can a quick analysis resolve this?" |
| **T2 (delta)** | Expected free energy under full deliberation | "Do I need deep reasoning here?" |

The gate decision (see [doc-07 §8](07-AGENT-RUNTIME.md)) IS the L2 routing decision — there is no separate gating system. The same EFE computation selects the tier and, within T1/T2, selects the specific model.

### Regime conditioning on Route

The EFE model receives the current `regime: Regime` as a context signal (see [doc-07 §8](07-AGENT-RUNTIME.md)). Different regimes shift the prior beliefs:

- **Crisis regime**: epistemic value is weighted higher (need information to resolve crisis)
- **Calm regime**: pragmatic value is weighted higher (environment is stable, optimize for goals)
- **Volatile regime**: cost is weighted higher (uncertain environment, avoid expensive mistakes)

This is not a separate routing mode — it is the same EFE computation with regime-conditioned priors.

### Examples

| Strategy Domain | Alternatives | Learning Signal | Status |
|---|---|---|---|
| Model selection per domain | [Claude Opus, Sonnet, Haiku, Gemini, Ollama local] | Task quality + cost + latency | CascadeRouter → EFE migration |
| Failure strategy selection | [Retry, Replan, Escalate, Skip] | Recovery success rate | Partial — strategies exist |
| Extension chain ordering | Permutations of loaded Extensions | Agent efficiency score | Not yet wired |
| Compose method selection | [GreedyComposer, VcgComposer, HybridComposer] | Output quality + budget adherence | Partial — VCG built |
| Gate rung selection | [fast-3-rung, standard-5-rung, thorough-7-rung] | Quality vs cost tradeoff | Not yet wired |

### The `feedback()` method

The Route protocol includes a `feedback()` method that is the learning signal for L2:

```rust
pub trait Route: Block {
    async fn route(&self, candidates: &[Signal], ctx: &RouteContext) -> Result<RouteResult>;
    async fn feedback(&self, choice: &SignalRef, outcome: &Signal) -> Result<()>;
}
```

After each task completes, the orchestrator calls `feedback()` with the selected alternative and the outcome. The EFE model updates its Bayesian posteriors.

### Safety: pre-approved alternatives only

L2 can never introduce a new alternative. It can only select among the set declared in configuration. Adding a new model or a new failure strategy requires human action (L4).

If all alternatives degrade below a quality floor, L2:
1. Falls back to the highest-prior alternative (the default).
2. Emits an `Alert(Critical)` Signal.
3. Freezes routing until human review.

---

## 4. Loop 3: Knowledge Consolidation (Delta)

**Timescale**: Per-session (60s to 10m)
**Autonomy**: Automatic execution, auditable results
**Safety**: New knowledge starts Transient; promotion requires Verify passage; AntiKnowledge prevents known-bad rediscovery

### What it does

L3 compresses raw episodes (agent turns, gate results, cost reports) into durable knowledge (Insights, Heuristics, CausalLinks, StrategyFragments). This is the dream cycle — offline processing that runs between or alongside active work. L3 also performs **hindsight relabeling**, recovering value from failed trajectories.

### Four phases

The dream cycle runs four phases in order:

```
Phase 1: NREM Replay
    │
    ▼
Phase 2: Hindsight Relabeling
    │
    ▼
Phase 3: REM Imagination
    │
    ▼
Phase 4: Integration
```

#### Phase 1: NREM Replay

Cluster high prediction-error episodes and extract patterns.

The system replays recent episodes, focusing on those with high prediction error (PE > 0.30) — situations where the agent was surprised. Clustering by HDC similarity groups related surprises. For each cluster, a pattern extractor (LLM-driven Block) produces an Insight Signal describing the common structure.

**Output**: `Signal { kind: Insight, tier: Transient }` with pattern description, cluster size, avg PE, supporting episode refs, domain tag, and HDC fingerprint.

#### Phase 2: Hindsight Relabeling

Failed trajectories are relabeled for sub-goals that WERE achieved.

```
Failed trajectory (original goal: "implement auth + tests")
    │
    ▼
Sub-goal extraction: "auth implemented" (achieved), "tests written" (failed)
    │
    ▼
Relabel: trajectory is SUCCESSFUL for "implement auth"
    │
    ▼
Episode relabeled with achieved sub-goal → enters NREM replay as positive data
```

**Why this matters**: Without hindsight relabeling, any trajectory that fails its overall gate is discarded. But most failed trajectories contain useful partial work — code that compiled but didn't pass tests, research that found relevant sources but didn't synthesize them, refactors that improved one metric while regressing another. Hindsight relabeling recovers this value.

**Recovery rate**: Recovers useful learning signal from at least 45% of otherwise-discarded episodes. The key insight is that "failure" is always relative to a specific goal — the same trajectory may be a success for a different, simpler goal.

**Sub-goal extraction**: The system uses the task's dependency graph and intermediate gate results to identify which sub-goals were achieved. A task with 5 sub-goals that achieved 3 of them generates 3 positive episode relabelings.

#### Phase 3: REM Imagination

Generate counterfactual scenarios from high-value Insights.

For each new Insight, the system generates counterfactual scenarios: "If the agent had used model X instead of model Y, what would have happened?" An LLM-driven Block simulates the alternative and evaluates the projected outcome.

**Output**: `Signal { kind: StrategyFragment, tier: Transient }` with condition/action clauses, projected improvement, confidence, and lineage.

**Threat rehearsal** runs as a sub-phase: the system enumerates plausible threat scenarios from recent episodes and generates Warning Signals (ephemeral, published on Bus with short TTL).

#### Phase 4: Integration

Promote validated Insights and StrategyFragments through knowledge tiers. Tier promotion uses the demurrage model ([doc-11 §3](11-MEMORY-AND-KNOWLEDGE.md)): Signals with high balance (actively used, recently reinforced) get promoted; Signals with low balance get demoted.

```
Transient Insights/Strategies
    │
    ▼
Verify protocol (gate check)
    │
    ├── Passed (3+ confirmations) → Promote to Working
    ├── Passed (5+ across contexts) → Promote to Consolidated
    └── Failed → Demote one tier, possibly create AntiKnowledge
```

### AntiKnowledge

When a validated Insight is later disproven (gate failures using knowledge derived from it), the system creates an AntiKnowledge Signal that actively repels future knowledge in the same HDC region. See [doc-11 §6](11-MEMORY-AND-KNOWLEDGE.md).

### Implementation as Graph

```toml
[graph]
name = "dream-consolidation-loop"
loop = true

[[nodes]]
id = "collect"
type = "block"
block = "roko:episode-collector@^1.0"

[[nodes]]
id = "nrem"
type = "block"
block = "roko:nrem-replay@^1.0"

[[nodes]]
id = "hindsight"
type = "block"
block = "roko:hindsight-relabeler@^1.0"

[[nodes]]
id = "rem"
type = "block"
block = "roko:rem-imagination@^1.0"

[[nodes]]
id = "threat"
type = "block"
block = "roko:threat-rehearsal@^1.0"

[[nodes]]
id = "integrate"
type = "block"
block = "roko:knowledge-integrator@^1.0"

[[nodes]]
id = "persist"
type = "block"
block = "roko:knowledge-store@^1.0"

[[edges]]
from = "collect"
to = "nrem"

[[edges]]
from = "nrem"
to = "hindsight"

[[edges]]
from = "hindsight"
to = "rem"

[[edges]]
from = "nrem"
to = "threat"

[[edges]]
from = "rem"
to = "integrate"

[[edges]]
from = "threat"
to = "integrate"

[[edges]]
from = "integrate"
to = "persist"

[[edges]]
from = "persist"
to = "collect"
condition = "session_active"
```

---

## 5. Loop 4: Structural Adaptation

**Timescale**: Per-approval (unbounded — waits for human)
**Autonomy**: Requires human approval for every change
**Safety**: `RecursiveSafetyMonitor`, pre-change snapshot, auto-rollback on quality regression, c-factor gate

L4 is the only loop that modifies the system's own structure. It encompasses three mechanisms that share the same approval workflow: **proposal generation** (bottom-up from L1/L2/L3 observations), **clade evolution** (population-based search), and **spec-as-artifact** (the specification itself is mutable).

### 5.1 Proposal generation

L4 observes aggregated L1/L2/L3 output and identifies structural bottlenecks:

| Proposal Type | Example | Approval Required |
|---|---|---|
| New reflex rule | "Always use Haiku for formatting tasks" → T0 reflex | Human review |
| Modified gate pipeline | "Add security-audit gate at rung 5" | Human review |
| Graph revision | "Replace sequential steps 3-5 with parallel execution" | Human review |
| Agent config change | "Increase code-agent's budget from $5 to $10" | Human review |
| New model alternative | "Add Claude 4.6 to EFE alternatives" | Human review |
| Spec amendment | "Add new Signal kind for audit events" | Human review + CMP score |

### 5.2 HGM Clade-Metaproductivity

**Hierarchical Generative Model (HGM)** scores configuration variants by their descendant performance, not just direct performance. A configuration that produces high-quality children (through L3 knowledge consolidation) is scored higher than one that performs well in isolation.

```
Configuration A        Configuration B
    │                      │
    ├── Child A1 (0.8)     ├── Child B1 (0.9)
    ├── Child A2 (0.7)     ├── Child B2 (0.3)
    └── Child A3 (0.9)     └── Child B3 (0.4)

    Clade score: 0.80       Clade score: 0.53
    A wins despite no single child beating B1
```

Clade-Metaproductivity measures a configuration's ability to **produce good descendants**, not just to be good itself. This selects for generative capacity — configurations that seed productive lines of improvement.

### 5.3 CycleQD with HDC Behavioral Characterizations

**Quality-Diversity (QD)** search explores the space of system configurations, maintaining a diverse archive of high-quality variants. **CycleQD** adds a cyclical schedule that alternates between exploration (add new variants) and exploitation (refine best variants).

Behavioral characterizations use HDC fingerprints: each configuration variant is fingerprinted by its behavioral signature (which tasks it handles well, which gate rungs it passes, which models it selects). HDC similarity between fingerprints determines archive placement — similar behaviors occupy the same archive cell, diverse behaviors occupy different cells.

### 5.4 Verify-as-reward

The Verify protocol ([doc-02 §3.3](02-BLOCK.md)) serves as the reward function for L4 self-play. When a proposed configuration change is evaluated:

1. A candidate agent runs with the proposed configuration.
2. A verifier agent (distinct from the candidate) evaluates the output using the Verify protocol.
3. The continuous reward (`Verdict.reward`) becomes the fitness signal for the evolutionary archive.

**Variance Inequality**: The verifier must be spectrally cleaner than the generator. In practice: the verifier uses a different model than the agent being evaluated, and verification Blocks sit outside the modifiable surface (L4 cannot change the Verify Blocks it is evaluated by). This prevents self-congratulatory optimization.

### 5.5 c-factor gate

L4 only evolves configurations that increase genuine collective intelligence. The c-factor ([doc-09 §3.11](09-TELEMETRY.md)) — computed from turn-taking entropy, peer prediction accuracy, citation reciprocity, and HDC diversity — gates every structural change:

```
Proposed configuration change
    │
    ▼
Apply to sandbox environment
    │
    ▼
Run evaluation period
    │
    ▼
Measure c-factor (before and after)
    │
    ├── c-factor increased → Proposal eligible for human review
    │
    └── c-factor decreased → Auto-rejected (logged for analysis)
```

A configuration that improves individual agent metrics but decreases collective intelligence is rejected. This prevents optimization pressure from collapsing agent diversity.

### 5.6 Spec-as-artifact

The specification itself (these documents) is a mutable artifact in the L4 evolutionary archive. This means:

1. **The spec is readable by agents at startup** — injected into system prompt context.
2. **The spec is queryable as MCP tools** — agents can `query_spec("what is demurrage?")`.
3. **The spec is evolvable through L4** — structural proposals can include spec amendments.
4. **Spec changes require additional approval** — CMP (Comprehension-Maintainability-Precision) scoring plus human review.

Spec amendments follow the same proposal workflow as other L4 changes but with an additional constraint: the amended spec must score at least as well as the current spec on CMP metrics. This prevents specification drift toward complexity.

### 5.7 RecursiveSafetyMonitor

All L4 proposals pass through the `RecursiveSafetyMonitor` before reaching the human:

```rust
pub struct RecursiveSafetyMonitor {
    pub max_depth: u32,                // max nesting of structural changes (default: 3)
    pub max_rate: Rate,                // max proposals per time window (default: 5/hour)
    pub quality_floor: f64,            // reject proposals if system quality below this
    pub caveat_threshold: usize,       // max caveats before auto-reject (default: 3)
    pub c_factor_floor: f64,           // reject proposals that decrease c-factor below this
    pub snapshot_required: bool,       // always snapshot before applying (default: true)
}
```

### 5.8 Approval workflow

```
L4 generates proposal
    │
    ▼
RecursiveSafetyMonitor checks bounds
    │
    ├── Rejected (bounds violated) → Log, notify operator
    │
    ▼
c-factor gate (sandbox evaluation)
    │
    ├── c-factor decreased → Auto-rejected
    │
    ▼
Proposal Signal emitted (kind: StructuralProposal)
    │
    ▼
Human reviews via dashboard / CLI / API
    │
    ├── Rejected → Log rejection reason, feed back to L3 as lesson
    │
    ├── Approved → Execute change
    │       │
    │       ▼
    │   Snapshot current state
    │       │
    │       ▼
    │   Apply structural change
    │       │
    │       ▼
    │   Monitor quality for observation_window
    │       │
    │       ├── Quality maintained → Confirm change
    │       │
    │       └── Quality regressed → Auto-rollback to snapshot
    │
    └── Deferred → Park for later review
```

---

## 6. Loop 1: Parameter Tuning (Gamma)

**Timescale**: Per-tick (100ms to 2s)
**Autonomy**: Fully automatic
**Safety**: Parameters adjust only within declared `ParamRange` bounds

### What it adjusts

L1 tunes continuous-valued parameters that have declared safe ranges:

```rust
pub struct ParamRange {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
    pub default: f64,
    pub learning_rate: f64,
}
```

### Examples

| Parameter | ParamRange | Learning Signal |
|---|---|---|
| Gate thresholds per rung | `[0.3, 0.95]` step 0.01 | Pass rate EMA over window |
| Prompt experiment weights | `[0.0, 1.0]` | Experiment outcome tracking |
| Model temperature per task type | `[0.0, 1.5]` step 0.05 | Gate pass/fail on model output |
| Adaptive clock regime thresholds | `[0.1, 0.8]` | Prediction error distribution |
| T0/T1/T2 EFE cost weights | `[0.01, 10.0]` | Agent efficiency (T0 hit rate vs quality) |
| Compose budget allocation weights | `[0.0, 1.0]` | Downstream quality of composed output |
| Sampling rates for high-frequency Lenses | `[0.01, 1.0]` | Lens overhead vs data completeness |

### Algorithm

L1 uses EMA with directional adjustment:

```
new_value = current_value + learning_rate * (observed_optimal - current_value)
clamped to [param_range.min, param_range.max]
```

This is the predict-publish-correct pattern applied to continuous parameters: the current value is the prediction, the gate outcome is the observation, and the EMA update is the correction.

### Safety: auto-rollback

L1 maintains a rolling quality window. If quality drops below a configured threshold after parameter adjustment:

1. Revert to the previous parameter value.
2. Halve the learning rate for this parameter.
3. Emit an `Alert(Warning)` Signal via the BudgetLens.
4. Log the rollback in `.roko/learn/param-rollbacks.jsonl`.

After `max_rollbacks` consecutive rollbacks, the parameter freezes at its current value and an `Alert(Critical)` Signal is emitted recommending human review.

---

## 7. Autonomy Levels

Roko defines six autonomy levels. Each learning loop operates at specific levels, and the operator can configure the maximum allowed level per Space.

| Level | Name | What the system can do | Loop |
|---|---|---|---|
| 0 | **Observe** | Read-only observation, emit Signals | All loops read |
| 1 | **Suggest** | Propose changes, wait for approval | L4 default |
| 2 | **Tune** | Adjust parameters within ParamRange | L1 |
| 3 | **Select** | Choose among pre-approved alternatives via EFE | L2 |
| 4 | **Consolidate** | Create, promote, demote knowledge entries | L3 |
| 5 | **Execute** | Apply structural changes (after approval) | L4 (with approval) |

### Configuration

```toml
[space.autonomy]
max_level = 4
l1_enabled = true
l2_enabled = true
l3_enabled = true
l4_enabled = false

[space.autonomy.l2]
exploration_budget = 0.10
fallback_on_degradation = true

[space.autonomy.l3]
auto_promote = true
max_tier = "consolidated"
dream_schedule = "0 */6 * * *"

[space.autonomy.l4]
require_approval = true
max_proposals_per_hour = 3
auto_rollback = true
observation_window = "1h"
c_factor_floor = 0.4
```

### Level escalation

When a loop needs to operate above its configured level, it escalates:

1. Emit a `Signal { kind: Alert, level: Info }` describing the desired action.
2. Wait for human approval (via dashboard / CLI / API).
3. On approval, execute the action with a scoped capability grant.
4. On rejection, log the rejection and adapt.

---

## 8. Safety Bounds

Each loop has explicit safety bounds enforced by the runtime.

### L1: ParamRange bounds

```rust
pub struct L1SafetyBounds {
    pub param_ranges: BTreeMap<String, ParamRange>,
    pub max_adjustment_per_tick: f64,
    pub quality_window: usize,
    pub quality_floor: f64,
    pub max_consecutive_rollbacks: usize,
}
```

**Invariant**: After every L1 adjustment, `param_range.min <= value <= param_range.max`. Violation is a logic error that halts the loop.

### L2: Pre-approved alternatives with EFE bounds

```rust
pub struct L2SafetyBounds {
    pub alternatives: Vec<String>,
    pub exploration_budget: f64,
    pub min_observations: usize,
    pub degradation_threshold: f64,
    pub forbidden_contexts: Vec<Expr>,
    pub max_efe_cost_ratio: f64,          // cap cost term in EFE to prevent stinginess
}
```

**Invariant**: L2 never selects an alternative not in the `alternatives` set. Adding a new alternative requires L4 approval.

### L3: Knowledge gatekeeping

```rust
pub struct L3SafetyBounds {
    pub max_entries_per_cycle: usize,
    pub verification_required: bool,
    pub min_confirmations_d1_d2: usize,   // 3+ to promote Transient → Working
    pub min_confirmations_d2_d3: usize,   // 5+ to promote Working → Consolidated
    pub anti_knowledge_threshold: f64,    // HDC similarity for rejection (0.9)
    pub max_tier_auto: Tier,              // highest tier reachable without human (Consolidated)
    pub hindsight_min_subgoals: usize,    // min sub-goals achieved to relabel (default: 1)
}
```

**Invariant**: No knowledge entry reaches Persistent tier without human or consortium approval. AntiKnowledge cannot be overridden by automatic processes.

### L4: Structural safety

```rust
pub struct L4SafetyBounds {
    pub require_human_approval: bool,
    pub recursive_safety_monitor: RecursiveSafetyMonitor,
    pub snapshot_before_change: bool,
    pub auto_rollback_on_regression: bool,
    pub observation_window: Duration,
    pub rollback_threshold: f64,
    pub c_factor_gate: bool,              // require c-factor non-decrease
    pub variance_inequality: bool,        // enforce verifier != generator
    pub max_structural_changes_per_day: usize,
}
```

**Invariant**: No structural change is applied without a snapshot. No structural change persists if quality regresses beyond threshold or c-factor decreases.

### Cross-loop safety

1. **No circular self-amplification**: L4 cannot approve its own proposals. L3 knowledge about "L1 should be more aggressive" is treated as a suggestion, not a command.
2. **Monotonic quality**: The system tracks a global quality metric. If global quality drops below a configured floor, all loops above L0 pause until quality recovers.
3. **Audit trail**: Every loop action is logged as an Episode Signal with lineage back to the triggering observation.
4. **Variance Inequality**: The verifier is always spectrally cleaner than the generator. L4 proposals evaluated by Blocks outside the modifiable surface.

---

## 9. Seven Compounding Feedback Loops

Beyond the four formal learning loops, seven compounding mechanisms connect learning to runtime behavior. Each creates a virtuous cycle where improvement in one area feeds improvement in others.

### 9.1 Demurrage-Retrieval Loop

Knowledge that gets retrieved and cited has its demurrage balance refreshed ([doc-11 §3](11-MEMORY-AND-KNOWLEDGE.md)). Useful knowledge stays warm. Unused knowledge fades. This creates natural selection pressure on the knowledge store: the store self-trims toward genuinely useful content without explicit garbage collection.

### 9.2 Heuristic Calibration Loop

Heuristics ([doc-11 §4](11-MEMORY-AND-KNOWLEDGE.md)) carry mandatory falsifiers and calibration track records. When a heuristic's predictions diverge from reality, its calibration score drops, reducing its bid in the CognitiveWorkspace VCG auction ([doc-07 §10](07-AGENT-RUNTIME.md)). Poorly calibrated heuristics lose prompt space to better-calibrated ones.

### 9.3 HDC Cleanup Loop

As the knowledge store grows, Resonator Networks ([doc-11 §7](11-MEMORY-AND-KNOWLEDGE.md)) periodically factorize bundled HDC vectors to identify constituent patterns that have been separately learned. When a bundle's constituents all exist independently at higher tiers, the bundle is pruned as redundant. This keeps the HDC space clean and retrieval fast.

### 9.4 c-factor Feedback Loop

The c-factor ([doc-09 §3.11](09-TELEMETRY.md)) gates L4 structural changes. Configurations that increase collective intelligence are favored. This means L4 evolution naturally selects for agent diversity, balanced turn-taking, and genuine knowledge exchange — not just individual performance.

### 9.5 Playbook Meta-Learning Loop

Playbooks (top heuristics distilled to markdown) are consumed by the CognitiveWorkspace as bidder context. Agents that follow playbook recommendations and pass gates reinforce the playbook entries (demurrage refresh). Playbooks that lead to failures get demoted. The playbook evolves to reflect what actually works, not what was initially hypothesized.

### 9.6 Cross-Deployment Commons Loop

Knowledge Signals published on-chain ([doc-11 §10](11-MEMORY-AND-KNOWLEDGE.md)) create a shared commons across deployments. A heuristic learned by deployment A and validated by deployment B gets promoted faster than one validated only locally. The on-chain knowledge store acts as a global L3, accelerating convergence across the network.

### 9.7 Plugin Ecosystem Loop

Marketplace Blocks that perform well (high UsageLens metrics) get recommended to more users. Users who adopt them generate more training data for L1/L2. Better training data improves Block quality. Improved quality increases adoption. The ecosystem compounds through usage data, not just code sharing.

---

## 10. Implementation as Graphs

Each loop is a Graph (specifically, a Loop specialization). This is not a metaphor — the loops are literally defined as TOML Graphs with feedback edges, using the same engine that runs every other Graph.

### Why Graphs?

1. **Composability**: Loops use the same Blocks, scoring, routing, and verification as task Graphs.
2. **Observability**: Lenses attach to Loop Graphs. CostLens tracks how much learning costs. EfficiencyLens tracks improvement produced.
3. **Resumability**: Loop Graphs checkpoint like any other Flow. A crashed dream cycle resumes from its last checkpoint.
4. **Testability**: Loop Graphs are testable with the same test infrastructure. Inject synthetic episodes, verify the Loop produces correct knowledge.

### Loop Graph conventions

| Convention | Requirement |
|---|---|
| `loop = true` | Declares the Graph as a Loop specialization |
| Feedback edge | At least one edge from a downstream node back to an upstream node |
| Convergence condition | Feedback edge has a `condition` (e.g., `NOT converged`, `session_active`) |
| Max iterations | `max_iterations` field prevents infinite loops |
| Observation period | `min_interval` between iterations prevents busy-looping |

---

## 11. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| LL-1 | L1 adjusts gate threshold within ParamRange after gate outcome | Integration test |
| LL-2 | L1 auto-rollback triggers when quality drops below floor | Unit test |
| LL-3 | L1 freezes parameter after max_rollbacks | Unit test |
| LL-4 | L2 EFE model selects model given context and regime | Unit test: feed context + regime, verify selection |
| LL-5 | L2 Router.feedback() updates Bayesian posteriors | Unit test |
| LL-6 | L2 falls back to default when all alternatives degrade | Unit test |
| LL-7 | L2 never selects an alternative outside the configured set | Unit test |
| LL-8 | L2 regime conditioning shifts selection — Crisis biases T2 | Unit test |
| LL-9 | L3 NREM clusters high-PE episodes and produces Insights | Integration test |
| LL-10 | L3 hindsight relabeling recovers sub-goals from failed trajectory | Integration test: fail task with 3/5 sub-goals, verify 3 relabelings |
| LL-11 | L3 hindsight recovery rate >= 45% of discarded episodes | Statistical test over 100 episodes |
| LL-12 | L3 REM generates StrategyFragments from Insights | Integration test |
| LL-13 | L3 promotes Insight from D1 to D2 after 3 confirmations | Integration test |
| LL-14 | L3 creates AntiKnowledge when Insight is disproven | Integration test |
| LL-15 | L3 dream cycle fires automatically on cron/trigger | Integration test |
| LL-16 | L3 four-phase order: NREM → Hindsight → REM → Integration | Integration test |
| LL-17 | L4 proposal passes through RecursiveSafetyMonitor | Unit test |
| LL-18 | L4 c-factor gate rejects proposals that decrease c-factor | Integration test |
| LL-19 | L4 Variance Inequality enforced: verifier != generator model | Unit test |
| LL-20 | L4 clade-metaproductivity scores by descendant performance | Unit test |
| LL-21 | L4 CycleQD maintains diverse archive via HDC fingerprints | Integration test |
| LL-22 | L4 Verify-as-reward uses continuous Verdict.reward | Integration test |
| LL-23 | L4 spec-as-artifact: spec queryable as MCP tool | Integration test |
| LL-24 | L4 spec amendment requires CMP scoring + human approval | Workflow test |
| LL-25 | L4 snapshots state before applying structural change | Integration test |
| LL-26 | L4 auto-rollback triggers on quality regression | Integration test |
| LL-27 | Autonomy levels enforced: L3 cannot perform L4 actions | Unit test |
| LL-28 | All loop actions logged as Episode Signals with lineage | Integration test |
| LL-29 | Global quality floor pauses all loops when breached | Integration test |
| LL-30 | Predict-publish-correct pattern observable on Bus topics | Integration test |
| LL-31 | Each loop is representable as a Loop Graph (TOML) | Graph load test |
| LL-32 | Lens telemetry attaches to Loop Graphs (cost of learning) | Integration test |

---

## 12. Cross-References

| Topic | Document | Section |
|---|---|---|
| Signal/Pulse duality, demurrage | [doc-01](01-SIGNAL.md) | §1-5 |
| Predict-publish-correct | [doc-02](02-BLOCK.md) | §3.10 |
| Verify redesign (continuous reward, evidence) | [doc-02](02-BLOCK.md) | §3.3 |
| EFE Route protocol | [doc-02](02-BLOCK.md) | §3.4 |
| EFE gating in agent pipeline | [doc-07](07-AGENT-RUNTIME.md) | §8 |
| Vitality phases | [doc-07](07-AGENT-RUNTIME.md) | §3 |
| Somatic markers | [doc-07](07-AGENT-RUNTIME.md) | §9 |
| CognitiveWorkspace (VCG, section effects) | [doc-07](07-AGENT-RUNTIME.md) | §10 |
| StateHub projections, c-factor | [doc-09](09-TELEMETRY.md) | §3.11, §6 |
| Demurrage model | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §3 |
| Heuristic lifecycle | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §4 |
| Resonator Networks | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §7 |
| AntiKnowledge | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §6 |
| CaMeL IFC, 5-head corrigibility | [doc-17](17-SECURITY-MODEL.md) | — |
| On-chain knowledge commons | [doc-11](11-MEMORY-AND-KNOWLEDGE.md) | §10 |
