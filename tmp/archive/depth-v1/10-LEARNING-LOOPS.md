# 10 — Learning Loops

> Four cybernetic loops at increasing timescales and autonomy. The system improves itself using the same primitives it uses for everything else.

**Depends on**: [01-SIGNAL](01-SIGNAL.md), [02-BLOCK](02-BLOCK.md), [03-GRAPH](03-GRAPH.md), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Loop definition), [09-TELEMETRY](09-TELEMETRY.md) (Lens data feeds Loops)

---

## 1. Overview

Roko learns through four feedback loops, each operating at a different timescale with a different level of autonomy. All four are implemented as Loop specializations — Graphs that feed output back to input — using the same fundamentals as every other part of the system.

| Loop | Name | Timescale | Autonomy | What it adjusts |
|---|---|---|---|---|
| L1 | Parameter Tuning | Gamma (per-tick) | Fully automatic | Continuous params within declared bounds |
| L2 | Strategy Routing | Theta (per-task) | Fully automatic | Selection among pre-approved alternatives |
| L3 | Knowledge Consolidation | Delta (per-session) | Automatic + auditable | Compression of episodes into durable knowledge |
| L4 | Structural Adaptation | Manual (per-approval) | Requires human approval | Changes to system structure |

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
    │ temperature│  │ model selection │  │ dream consolidation  │  │ gate pipeline changes  │
    │ thresholds │  │ failure strategy│  │ insight promotion    │  │ Graph revisions        │
    │ weights    │  │ chain ordering  │  │ anti-knowledge       │  │ new reflex rules       │
    └────────────┘  └─────────────────┘  └──────────────────────┘  └────────────────────────┘
```

---

## 2. Loop 1: Parameter Tuning (Gamma)

**Timescale**: Per-tick (100ms to 2s)
**Autonomy**: Fully automatic
**Safety**: Parameters adjust only within declared `ParamRange` bounds

### What it adjusts

L1 tunes continuous-valued parameters that have declared safe ranges. Every tunable parameter carries a `ParamRange` that bounds adjustment:

```rust
pub struct ParamRange {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,           // minimum adjustment granularity
    pub default: f64,
    pub learning_rate: f64,          // how fast to adjust (0.0 to 1.0)
}
```

### Examples

| Parameter | ParamRange | Learning Signal | Current Code |
|---|---|---|---|
| Model temperature per task type | `[0.0, 1.5]` step 0.05 | Gate pass/fail on model output | Not yet wired |
| Gate thresholds per rung | `[0.3, 0.95]` step 0.01 | Pass rate EMA over window | **Built**: `gate-thresholds.json` adaptive EMA |
| Prompt experiment weights | `[0.0, 1.0]` | Experiment outcome tracking | **Built**: `ExperimentStore` with arm rewards |
| Adaptive clock regime thresholds | `[0.1, 0.8]` | Prediction error distribution | **Partial**: thresholds hardcoded, PE tracked |
| Compose budget allocation weights | `[0.0, 1.0]` | Downstream quality of composed output | Not yet wired |
| T0/T1/T2 prediction error boundaries | `[0.05, 0.60]` | Agent efficiency (T0 hit rate vs quality) | **Partial**: boundaries static in code |
| Sampling rates for high-frequency Lenses | `[0.01, 1.0]` | Lens overhead vs data completeness | Not yet wired |

### Algorithm

L1 uses exponential moving average (EMA) with directional adjustment:

```
new_value = current_value + learning_rate × (observed_optimal - current_value)
```

Where `observed_optimal` is derived from the learning signal. For gate thresholds:

```
// After each gate evaluation:
if gate_passed:
    observed_optimal = current_threshold + step    // tighten slightly
else:
    observed_optimal = current_threshold - step    // loosen slightly

// EMA update:
threshold = threshold + alpha × (observed_optimal - threshold)

// Clamp to ParamRange:
threshold = clamp(threshold, range.min, range.max)
```

### Safety: auto-rollback

L1 maintains a rolling quality window. If quality drops below a configured threshold after parameter adjustment:

1. Revert to the previous parameter value.
2. Halve the learning rate for this parameter.
3. Emit an `Alert(Warning)` Signal via the BudgetLens.
4. Log the rollback in `.roko/learn/param-rollbacks.jsonl`.

```rust
pub struct ParamRollbackPolicy {
    pub quality_window: usize,         // number of observations to track
    pub quality_threshold: f64,        // min acceptable quality (0.0 to 1.0)
    pub learning_rate_decay: f64,      // multiply learning rate by this on rollback
    pub max_rollbacks: usize,          // after N rollbacks, freeze parameter
}
```

After `max_rollbacks` consecutive rollbacks, the parameter freezes at its current value and an `Alert(Critical)` Signal is emitted recommending human review.

### Implementation as Graph

```toml
[graph]
name = "param-tuning-loop"
loop = true

[[nodes]]
id = "observe"
type = "block"
block = "roko:param-observation-collector@^1.0"
# Collects: gate results, model outcomes, experiment results

[[nodes]]
id = "compute"
type = "block"
block = "roko:ema-updater@^1.0"
# Computes new parameter values via EMA

[[nodes]]
id = "validate"
type = "block"
block = "roko:param-range-validator@^1.0"
# Clamps to ParamRange, checks rollback conditions

[[nodes]]
id = "apply"
type = "block"
block = "roko:param-applier@^1.0"
# Writes new values to config / runtime state

[[edges]]
from = "observe"
to = "compute"

[[edges]]
from = "compute"
to = "validate"

[[edges]]
from = "validate"
to = "apply"

# Feedback edge
[[edges]]
from = "apply"
to = "observe"
condition = "NOT frozen"
```

---

## 3. Loop 2: Strategy Routing (Theta)

**Timescale**: Per-task (750ms to 16s)
**Autonomy**: Fully automatic
**Safety**: Finite set of pre-approved alternatives only

### What it adjusts

L2 selects between existing, pre-approved alternatives using multi-armed bandit algorithms. Unlike L1 (which adjusts continuous values within a range), L2 chooses discrete options from a fixed set.

```rust
pub struct StrategySet {
    pub alternatives: Vec<StrategyAlternative>,
    pub selection_algorithm: SelectionAlgorithm,
    pub feedback_signal: FeedbackSignalKind,
}

pub struct StrategyAlternative {
    pub id: String,
    pub description: String,
    pub default_weight: f64,           // initial selection probability
    pub constraints: Vec<Constraint>,  // when this alternative is valid
}

pub enum SelectionAlgorithm {
    LinUCB { alpha: f64 },             // contextual bandit (current CascadeRouter)
    EpsilonGreedy { epsilon: f64 },    // simple exploration
    ThompsonSampling,                  // Bayesian selection
    Softmax { tau: f64 },              // temperature-based
}
```

### Examples

| Strategy Domain | Alternatives | Learning Signal | Current Code |
|---|---|---|---|
| Model selection per domain | [Claude Opus, Sonnet, Haiku, Gemini, Ollama local] | Task quality + cost + latency | **Built**: `CascadeRouter` with LinUCB |
| Failure strategy selection | [Retry, Replan, Escalate, Skip] | Recovery success rate | **Partial**: strategies exist, no bandit |
| Extension chain ordering | Permutations of loaded Extensions | Agent efficiency score | Not yet wired |
| Memory retrieval weighting | [HDC-heavy, keyword-heavy, recency-heavy, balanced] | Retrieval hit rate + downstream quality | Not yet wired |
| Compose method | [GreedyComposer, VcgComposer, HybridComposer] | Output quality + budget adherence | **Partial**: VCG built but greedy dominates |
| Gate rung selection | [fast-3-rung, standard-5-rung, thorough-7-rung] | Quality vs cost tradeoff | Not yet wired |

### The `feedback()` method

The Route protocol includes a `feedback()` method that is the learning signal for L2:

```rust
pub trait Route: Block {
    async fn route(&self, candidates: &[Signal], ctx: &RouteContext) -> Result<RouteResult>;
    async fn feedback(&self, choice: &SignalRef, outcome: &Signal) -> Result<()>;
}
```

After each task completes, the orchestrator calls `feedback()` with:
- `choice`: which alternative was selected
- `outcome`: a Signal containing quality score, cost, latency, and pass/fail

The Router uses this to update its internal model (e.g., LinUCB updates its weight matrix).

### CascadeRouter (built)

The existing `CascadeRouter` in `roko-learn` is the primary L2 implementation:

```rust
pub struct CascadeRouter {
    pub arms: Vec<ModelArm>,           // one per model
    pub context_dim: usize,            // LinUCB context dimension
    pub alpha: f64,                    // exploration factor
    pub weights: DMatrix<f64>,         // LinUCB weight matrix
    pub covariance: DMatrix<f64>,      // LinUCB covariance
    pub persist_path: PathBuf,         // .roko/learn/cascade-router.json
}
```

**Context features** for LinUCB:
- Task domain (coding, research, review, ops)
- Estimated complexity (token count estimate)
- Time pressure (deadline proximity)
- Budget remaining (% of total)
- Historical success rate for this domain

### Safety: pre-approved alternatives only

L2 can never introduce a new alternative. It can only select among the set declared in configuration. Adding a new model or a new failure strategy requires human action (L4).

If all alternatives degrade below a quality floor, L2:
1. Falls back to the highest-weight alternative (the default).
2. Emits an `Alert(Critical)` Signal.
3. Freezes routing until human review.

---

## 4. Loop 3: Knowledge Consolidation (Delta)

**Timescale**: Per-session (60s to 10m)
**Autonomy**: Automatic execution, auditable results
**Safety**: New knowledge starts Transient; promotion requires Verify passage; AntiKnowledge prevents known-bad rediscovery

### What it does

L3 compresses raw episodes (agent turns, gate results, cost reports) into durable knowledge (Insights, Heuristics, CausalLinks, StrategyFragments). This is the dream cycle — offline processing that runs between or alongside active work.

### Three phases

#### Phase 1: NREM Replay

Cluster high prediction-error episodes and extract patterns.

```
Raw episodes (high PE)
    │
    ▼
HDC clustering (similarity > 0.6)
    │
    ▼
Pattern extraction (common structure across cluster)
    │
    ▼
Insight Signals at Transient tier
```

**What happens**: The system replays recent episodes, focusing on those with high prediction error (PE > 0.30) — situations where the agent was surprised. Clustering by HDC similarity groups related surprises. For each cluster, a pattern extractor (LLM-driven Block) produces an Insight Signal describing the common structure.

**Output**: `Signal { kind: Insight, tier: Transient }` with payload:

```rust
pub struct InsightPayload {
    pub pattern: String,               // human-readable description
    pub cluster_size: usize,           // episodes in source cluster
    pub avg_prediction_error: f64,     // mean PE of cluster
    pub supporting_episodes: Vec<SignalRef>,  // lineage back to raw data
    pub domain: String,                // inferred domain tag
    pub hdc_fingerprint: HdcVector,    // for similarity matching
}
```

#### Phase 2: REM Imagination

Generate counterfactual scenarios from high-value Insights.

```
Insights from Phase 1
    │
    ▼
Counterfactual generation ("what if we had done X instead?")
    │
    ▼
Simulated outcome evaluation
    │
    ▼
StrategyFragment Signals at Transient tier
```

**What happens**: For each new Insight, the system generates counterfactual scenarios: "If the agent had used model X instead of model Y, what would have happened?" An LLM-driven Block simulates the alternative and evaluates the projected outcome.

**Output**: `Signal { kind: StrategyFragment, tier: Transient }` with payload:

```rust
pub struct StrategyFragmentPayload {
    pub condition: String,             // "when" clause
    pub action: String,                // "then" clause
    pub projected_improvement: f64,    // estimated quality delta
    pub source_insight: SignalRef,     // lineage
    pub counterfactual_basis: String,  // what was imagined
    pub confidence: f64,               // how confident the simulation is
}
```

#### Phase 3: Integration

Promote validated Insights and StrategyFragments through knowledge tiers.

```
Transient Insights/Strategies
    │
    ▼
Verify protocol (gate check)
    │
    ├── Passed (3+ confirmations) → Promote to Working (D1→D2)
    ├── Passed (5+ across contexts) → Promote to Consolidated (D2→D3)
    └── Failed → Demote one tier, possibly create AntiKnowledge
```

**Tier mapping** (Memory tiers to dream naming):

| Tier | Dream Name | Meaning |
|---|---|---|
| Transient | D1 | Newly generated, not yet validated |
| Working | D2 | Confirmed by multiple uses |
| Consolidated | D3 | Confirmed across distinct contexts |
| Persistent | D4 | Frozen bedrock knowledge |

Promotion requires explicit Verify protocol passage:
- **D1 to D2**: The Insight must be confirmed 3+ times by independent gate passes where the Insight's pattern was relevant.
- **D2 to D3**: The Insight must be confirmed 5+ times across distinct task contexts (different domains, different agents).
- **D3 to D4**: Requires human review or consortium approval (3+ validators).

### Threat rehearsal

During REM imagination, the system also generates **threat scenarios** — situations where known strategies would fail:

```
Known StrategyFragments
    │
    ▼
Adversarial scenario generation
    │
    ▼
Simulate strategy failure
    │
    ▼
Warning Signals (ephemeral, 1-hour half-life)
```

Warning Signals are consumed by React-protocol Blocks (SafetyReactor) and flag situations where the system should be extra cautious. They decay rapidly (1-hour half-life) because threat landscapes change.

### AntiKnowledge

When a validated Insight is later disproven (gate failures using knowledge derived from it), the system creates an AntiKnowledge Signal:

```rust
pub struct AntiKnowledgePayload {
    pub original: SignalRef,           // the disproven Insight
    pub reason: String,                // why it was disproven
    pub hdc_fingerprint: HdcVector,    // repels similar future Signals
    pub created_from: Vec<SignalRef>,  // evidence of disproval
}
```

AntiKnowledge Signals actively prevent the system from rediscovering known-bad information. New Signals with HDC similarity > 0.7 to an AntiKnowledge entry have their confidence halved; > 0.9 similarity results in outright rejection (see doc-01, section 11).

### Implementation as Graph

```toml
[graph]
name = "dream-consolidation-loop"
loop = true

[[nodes]]
id = "collect"
type = "block"
block = "roko:episode-collector@^1.0"
# Collects high-PE episodes since last cycle

[[nodes]]
id = "nrem"
type = "block"
block = "roko:nrem-replay@^1.0"
# HDC clustering + pattern extraction → Insight Signals

[[nodes]]
id = "rem"
type = "block"
block = "roko:rem-imagination@^1.0"
# Counterfactual generation → StrategyFragment Signals

[[nodes]]
id = "threat"
type = "block"
block = "roko:threat-rehearsal@^1.0"
# Adversarial scenarios → Warning Signals

[[nodes]]
id = "integrate"
type = "block"
block = "roko:knowledge-integrator@^1.0"
# Tier promotion/demotion, AntiKnowledge creation

[[nodes]]
id = "persist"
type = "block"
block = "roko:knowledge-store@^1.0"
# Write to neuro store

[[edges]]
from = "collect"
to = "nrem"

[[edges]]
from = "nrem"
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

# Feedback edge: new knowledge informs next cycle's collection
[[edges]]
from = "persist"
to = "collect"
condition = "session_active"
```

### Critical wiring gap

The dream consolidation code exists in `roko-dreams` but is only invoked manually via `roko knowledge dream run`. There is no daemon or cron trigger to run it automatically. The critical fix is wiring a `CronTrigger` or `BusTrigger` (on session end) to fire this Loop Graph.

---

## 5. Loop 4: Structural Adaptation

**Timescale**: Per-approval (unbounded — waits for human)
**Autonomy**: Requires human approval for every change
**Safety**: `RecursiveSafetyMonitor`, pre-change snapshot, auto-rollback on quality regression

### What it proposes

L4 is the only loop that modifies the system's own structure. It generates proposals that, if approved, change how the system operates.

| Proposal Type | Example | Approval Required |
|---|---|---|
| New reflex rule | "Always use Haiku for formatting tasks" → T0 reflex | Human review |
| Modified gate pipeline | "Add security-audit gate at rung 5" | Human review |
| Graph revision | "Replace sequential steps 3-5 with parallel execution" | Human review |
| Agent config change | "Increase code-agent's budget from $5 to $10" | Human review |
| Extension addition | "Install rate-limiting Extension for API connector" | Human review |
| New model alternative | "Add Claude 4.6 to CascadeRouter alternatives" | Human review |

### Proposal generation

L4 observes aggregated L1/L2/L3 output and identifies structural bottlenecks:

```
L1 output: "Gate threshold for rung 3 has been at range.min for 2 weeks"
L4 proposal: "Remove rung 3 from the gate pipeline — it never rejects"

L2 output: "CascadeRouter always picks Sonnet for research tasks"
L4 proposal: "Add a dedicated research-model arm to CascadeRouter"

L3 output: "5 Insights all describe the same retry pattern"
L4 proposal: "Create a T0 reflex rule for this pattern"
```

### RecursiveSafetyMonitor

All L4 proposals pass through the `RecursiveSafetyMonitor` before reaching the human:

```rust
pub struct RecursiveSafetyMonitor {
    pub max_depth: u32,                // max nesting of structural changes (default: 3)
    pub max_rate: Rate,                // max proposals per time window (default: 5/hour)
    pub quality_floor: f64,            // reject proposals if system quality below this
    pub caveat_threshold: usize,       // max caveats before auto-reject (default: 3)
    pub snapshot_required: bool,       // always snapshot before applying (default: true)
}
```

**Bounds enforced**:

| Bound | Enforcement | Rationale |
|---|---|---|
| **Depth** | Structural changes cannot nest more than `max_depth` levels deep | Prevents recursive self-modification spirals |
| **Rate** | No more than `max_rate` proposals per window | Prevents change fatigue and cascading modifications |
| **Quality** | Proposals rejected if system quality is below `quality_floor` | Fix what's broken before changing structure |
| **Caveats** | Proposals with > `caveat_threshold` caveats are auto-rejected | Too many unknowns = not ready |

### Approval workflow

```
L4 generates proposal
    │
    ▼
RecursiveSafetyMonitor checks bounds
    │
    ├── Rejected (bounds violated) → Log, notify operator
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

### Pre-change snapshot

Before every structural change:

1. Snapshot current configuration (roko.toml, agent configs, gate configs, Graph definitions).
2. Snapshot current quality metrics (pass rates, efficiency, cost).
3. Store snapshot at `.roko/state/structural-snapshots/<timestamp>/`.

### Post-change monitoring

After applying a structural change, the system enters an observation window (default: 1 hour). During this window:

- All L1/L2 loops continue but flag their output as "post-structural-change".
- QualityLens, EfficiencyLens, and ErrorLens output is compared against pre-change baselines.
- If any monitored metric degrades by more than `rollback_threshold` (default: 15%), auto-rollback is triggered.

```rust
pub struct StructuralChangePolicy {
    pub observation_window: Duration,  // how long to monitor after change
    pub rollback_threshold: f64,       // quality degradation % to trigger rollback
    pub baseline_window: Duration,     // how far back to compute baseline
    pub auto_rollback: bool,           // whether to rollback automatically or just alert
}
```

---

## 6. Autonomy Levels

Roko defines six autonomy levels. Each learning loop operates at specific levels, and the operator can configure the maximum allowed level per Space.

| Level | Name | What the system can do | Loop |
|---|---|---|---|
| 0 | **Observe** | Read-only observation, emit Signals | All loops read |
| 1 | **Suggest** | Propose changes, wait for approval | L4 default |
| 2 | **Tune** | Adjust parameters within ParamRange | L1 |
| 3 | **Select** | Choose among pre-approved alternatives | L2 |
| 4 | **Consolidate** | Create, promote, demote knowledge entries | L3 |
| 5 | **Execute** | Apply structural changes (after approval) | L4 (with approval) |

### Configuration

```toml
[space.autonomy]
max_level = 4                          # allow up to Consolidate
l1_enabled = true                      # parameter tuning
l2_enabled = true                      # strategy routing
l3_enabled = true                      # knowledge consolidation
l4_enabled = false                     # structural adaptation (disabled)

[space.autonomy.l1]
max_learning_rate = 0.3                # cap L1 adjustment speed
rollback_threshold = 0.85             # quality floor for auto-rollback

[space.autonomy.l2]
exploration_budget = 0.10              # max % of tasks used for exploration
fallback_on_degradation = true         # revert to default on quality drop

[space.autonomy.l3]
auto_promote = true                    # allow automatic tier promotion
max_tier = "consolidated"              # don't auto-promote beyond D3
dream_schedule = "0 */6 * * *"         # run dream cycle every 6 hours

[space.autonomy.l4]
require_approval = true                # always require human approval
max_proposals_per_hour = 3
auto_rollback = true
observation_window = "1h"
```

### Level escalation

When a loop needs to operate above its configured level, it escalates:

1. Emit a `Signal { kind: Alert, level: Info }` describing the desired action.
2. Wait for human approval (via dashboard / CLI / API).
3. On approval, execute the action with a scoped capability grant.
4. On rejection, log the rejection and adapt.

Escalation is the mechanism that prevents loops from exceeding their autonomy. An L1 loop that wants to adjust a parameter beyond its ParamRange must escalate to L4 (structural change) and wait for approval.

---

## 7. Safety Bounds

Each loop has explicit safety bounds. These are not optional — they are enforced by the runtime.

### L1: ParamRange bounds

```rust
pub struct L1SafetyBounds {
    pub param_ranges: BTreeMap<String, ParamRange>,  // declared per parameter
    pub max_adjustment_per_tick: f64,    // max change per tick (prevents jumps)
    pub quality_window: usize,           // rolling window for rollback detection
    pub quality_floor: f64,              // auto-rollback below this
    pub max_consecutive_rollbacks: usize, // freeze after N rollbacks
}
```

**Invariant**: After every L1 adjustment, `param_range.min <= value <= param_range.max`. Violation is a logic error that halts the loop.

### L2: Pre-approved alternatives

```rust
pub struct L2SafetyBounds {
    pub alternatives: Vec<String>,       // fixed set (e.g., model IDs)
    pub exploration_budget: f64,         // max % of selections that can explore
    pub min_observations: usize,         // per alternative before exploitation
    pub degradation_threshold: f64,      // fall back to default below this
    pub forbidden_contexts: Vec<Expr>,   // contexts where routing is locked
}
```

**Invariant**: L2 never selects an alternative not in the `alternatives` set. Adding a new alternative requires L4 approval.

### L3: Knowledge gatekeeping

```rust
pub struct L3SafetyBounds {
    pub max_entries_per_cycle: usize,    // cap on new knowledge per dream cycle
    pub verification_required: bool,     // require Verify passage for promotion
    pub min_confirmations_d1_d2: usize,  // 3+ to promote Transient → Working
    pub min_confirmations_d2_d3: usize,  // 5+ to promote Working → Consolidated
    pub anti_knowledge_threshold: f64,   // HDC similarity for rejection (0.9)
    pub max_tier_auto: Tier,             // highest tier reachable without human (Consolidated)
}
```

**Invariant**: No knowledge entry reaches Persistent tier without human or consortium approval. AntiKnowledge cannot be overridden by automatic processes.

### L4: Structural safety

```rust
pub struct L4SafetyBounds {
    pub require_human_approval: bool,    // always true in production
    pub recursive_safety_monitor: RecursiveSafetyMonitor,
    pub snapshot_before_change: bool,    // always true
    pub auto_rollback_on_regression: bool,
    pub observation_window: Duration,
    pub rollback_threshold: f64,
    pub max_structural_changes_per_day: usize,
}
```

**Invariant**: No structural change is applied without a snapshot. No structural change persists if quality regresses beyond threshold.

### Cross-loop safety

Loops interact. L1 output informs L2 decisions. L2 outcomes feed L3 consolidation. L3 knowledge informs L4 proposals. Safety must be maintained across these interactions:

1. **No circular self-amplification**: L4 cannot approve its own proposals. L3 knowledge about "L1 should be more aggressive" is treated as a suggestion, not a command.
2. **Monotonic quality**: The system tracks a global quality metric (composite of pass rates, efficiency, error rates). If global quality drops below a configured floor, all loops above L0 pause until quality recovers.
3. **Audit trail**: Every loop action is logged as an Episode Signal with lineage back to the triggering observation. The full chain is auditable.

---

## 8. Implementation as Graphs

Each loop is a Graph (specifically, a Loop specialization). This is not a metaphor — the loops are literally defined as TOML Graphs with feedback edges, using the same engine that runs every other Graph.

### Why Graphs?

1. **Composability**: Loops use the same Blocks, scoring, routing, and verification as task Graphs. No special machinery.
2. **Observability**: Lenses attach to Loop Graphs. CostLens tracks how much the learning loops themselves cost. EfficiencyLens tracks how much improvement they produce.
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

```toml
[graph]
name = "strategy-routing-loop"
loop = true
max_iterations = 1000                  # safety cap
min_interval = "750ms"                 # minimum time between iterations

[[nodes]]
id = "collect_outcomes"
# ...

[[nodes]]
id = "update_weights"
# ...

[[nodes]]
id = "select_strategy"
# ...

[[edges]]
from = "select_strategy"
to = "collect_outcomes"
condition = "tasks_remaining"
```

---

## 9. Wiring Gaps

The learning loop primitives exist in code. What follows is an honest assessment of what's built, what's wired, and what's missing.

### L1: Parameter Tuning

| Component | Status | Location |
|---|---|---|
| Adaptive gate thresholds (EMA) | **Built + Wired** | `orchestrate.rs` → `.roko/learn/gate-thresholds.json` |
| Prompt experiment weights | **Built + Wired** | `ExperimentStore` in `.roko/learn/experiments.json` |
| ParamRange type | **Not built** | Needs struct definition in roko-core |
| Auto-rollback policy | **Not built** | Needs rollback detection + revert logic |
| Temperature per task type | **Not wired** | CascadeRouter does model selection but not temperature |
| Adaptive clock thresholds | **Partial** | PE tracked, thresholds hardcoded |
| L1 as Loop Graph | **Not built** | Currently inline in orchestrate.rs |

### L2: Strategy Routing

| Component | Status | Location |
|---|---|---|
| CascadeRouter (LinUCB) | **Built + Wired** | `roko-learn/src/cascade_router.rs` → `.roko/learn/cascade-router.json` |
| Router.feedback() | **Built + Wired** | Called from orchestrate.rs after task completion |
| Failure strategy selection | **Partial** | Strategies exist in execution engine, no bandit selection |
| Extension chain ordering | **Not built** | Fixed config order |
| Compose method selection | **Partial** | VCG built, greedy dominates, no routing between them |
| force_backend override learning | **Not wired** | UX34 gap: manual overrides don't feed back to router |
| L2 as Loop Graph | **Not built** | CascadeRouter is a standalone struct |

### L3: Knowledge Consolidation

| Component | Status | Location |
|---|---|---|
| Dream cycle (NREM + REM + Integration) | **Built** | `roko-dreams` crate |
| Episode collection | **Built + Wired** | `.roko/episodes.jsonl` via orchestrate.rs |
| HDC clustering | **Built** | `roko-primitives` HDC operations |
| Insight/StrategyFragment types | **Built** | Signal kinds in roko-core |
| AntiKnowledge | **Built** | `roko-neuro` with thresholds |
| Tier promotion logic | **Built** | `roko-neuro` tier progression |
| Threat rehearsal | **Not built** | Described in roko-dreams but not implemented |
| Runtime trigger (cron/daemon) | **NOT WIRED** | `roko knowledge dream run` is manual only |
| L3 as Loop Graph | **Not built** | Dream cycle is imperative code in roko-dreams |

### L4: Structural Adaptation

| Component | Status | Location |
|---|---|---|
| RecursiveSafetyMonitor | **Not built** | Struct described but not implemented |
| Proposal generation | **Not built** | No L4 observation of L1/L2/L3 output |
| Approval workflow | **Partial** | Gate failure replan exists (orchestrate.rs) but is task-scoped |
| Pre-change snapshots | **Partial** | Executor snapshots exist, not structural snapshots |
| Post-change quality monitoring | **Not built** | No observation window + auto-rollback |
| Reflex rule creation | **Partial** | T0 reflexes exist, no automatic creation from L3 |
| L4 as Loop Graph | **Not built** | No Graph definition |

### Summary

```
L1: ████████░░ 80%  (gate thresholds + experiments wired; ParamRange + rollback missing)
L2: ██████░░░░ 60%  (CascadeRouter wired; failure/compose routing missing)
L3: █████░░░░░ 50%  (dream code built; runtime trigger critical gap)
L4: ██░░░░░░░░ 20%  (gate failure replan only; full L4 not started)
```

### Priority wiring order

1. **L3 runtime trigger** — Wire CronTrigger to fire dream consolidation. The code exists; it just needs a trigger.
2. **L2 failure strategy routing** — Add bandit selection over failure strategies in execution engine.
3. **L1 ParamRange** — Define ParamRange struct, attach to tunable parameters, add auto-rollback.
4. **L4 RecursiveSafetyMonitor** — Build the struct, wire proposal generation from L3 insights.

---

## 10. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| LL-1 | L1 adjusts gate threshold within ParamRange after gate outcome | Integration test: run gate, verify threshold moves toward outcome |
| LL-2 | L1 auto-rollback triggers when quality drops below floor | Unit test: simulate quality drop, verify parameter reverts |
| LL-3 | L1 freezes parameter after max_rollbacks consecutive rollbacks | Unit test: trigger N rollbacks, verify freeze + Alert |
| LL-4 | L2 CascadeRouter selects model via LinUCB given context | Unit test: feed context + outcomes, verify arm selection shifts |
| LL-5 | L2 Router.feedback() updates internal weights | Unit test: call feedback() with positive outcome, verify weight increase |
| LL-6 | L2 falls back to default when all alternatives degrade | Unit test: degrade all arms, verify default selection |
| LL-7 | L2 never selects an alternative outside the configured set | Unit test: verify exhaustive selection within alternatives[] |
| LL-8 | L3 NREM clusters high-PE episodes and produces Insights | Integration test: inject episodes with PE > 0.3, verify Insight Signals |
| LL-9 | L3 REM generates StrategyFragments from Insights | Integration test: inject Insights, verify StrategyFragment Signals |
| LL-10 | L3 promotes Insight from D1 to D2 after 3 confirmations | Integration test: confirm Insight 3 times, verify tier promotion |
| LL-11 | L3 creates AntiKnowledge when Insight is disproven | Integration test: fail gate using Insight-derived knowledge, verify AntiKnowledge |
| LL-12 | L3 AntiKnowledge rejects new Signal with HDC similarity > 0.9 | Unit test: create AntiKnowledge, submit similar Signal, verify rejection |
| LL-13 | L3 dream cycle fires automatically on cron/trigger | Integration test: configure trigger, verify dream cycle runs without manual invocation |
| LL-14 | L4 proposal passes through RecursiveSafetyMonitor bounds | Unit test: submit proposals exceeding depth/rate/quality bounds, verify rejection |
| LL-15 | L4 requires human approval before structural change | Integration test: generate proposal, verify it parks in approval queue |
| LL-16 | L4 snapshots state before applying structural change | Integration test: approve proposal, verify snapshot at expected path |
| LL-17 | L4 auto-rollback triggers on quality regression during observation window | Integration test: apply change, simulate quality drop, verify rollback |
| LL-18 | Autonomy levels enforced: L3 cannot perform L4 actions | Unit test: attempt structural change from L3 context, verify denial |
| LL-19 | All loop actions logged as Episode Signals with lineage | Integration test: run each loop, verify Episode Signals in log |
| LL-20 | Global quality floor pauses all loops when breached | Integration test: degrade global quality, verify loops pause |
| LL-21 | Each loop is representable as a Loop Graph (TOML) | Verify: load loop TOML, resolve Graph, check feedback edge |
| LL-22 | Lens telemetry attaches to Loop Graphs (cost of learning) | Integration test: attach CostLens to L1 Loop, verify CostReport Signals |
