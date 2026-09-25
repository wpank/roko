# Attention Auction, Context Governor, and CorticalState

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/12-attention-auction-and-gating.md (1,067 lines).
> Parent: `docs/v3/29-HEARTBEAT.md` SS14--16.

---

## 1. Abstract

An agent's context window is its most constrained resource. A T2 tick assembles
~32,000 tokens of context from competing subsystems: Neuro knowledge entries, Daimon
affect state, iteration memory, code intelligence, playbook rules, research artifacts,
task context, and oracle predictions. Which sections get included -- and how many tokens
each receives -- determines decision quality.

Naive approaches (fixed priority ordering, round-robin allocation, first-come-first-
served) waste this resource. A Vickrey-Clarke-Groves (VCG) auction (Vickrey 1961,
Clarke 1971, Groves 1973) provides the optimal solution: each subsystem bids for token
budget based on its expected contribution to the current tick's success, and the
mechanism guarantees truthful bidding -- no subsystem can gain by inflating its bid.

This document specifies three interconnected components:

1. **VCG Attention Auction** -- allocates context budget across competing subsystems
   using truthful second-price bidding.
2. **CorticalState** -- the shared atomic perception surface written by gamma ticks and
   read by all subsystems, providing zero-latency state access.
3. **Context Governor** -- the orchestrator that runs the auction, enforces budget
   constraints, and assembles the final context window.

---

## 2. The VCG Attention Auction

### 2.1 Why an Auction?

Context space is a scarce resource with competing demands. On a T2 tick with ~32,000
tokens of budget:

| Subsystem | Wants | Typical Ask | Why It Matters |
|-----------|-------|-------------|----------------|
| Task context | Current task description, success criteria | ~4,000 tokens | Without it, the agent does not know what to do |
| Neuro knowledge | Relevant insights, heuristics, warnings | ~6,000 tokens | Past experience improves decisions |
| Iteration memory | Previous attempts, feedback from gates | ~3,000 tokens | Avoids repeating mistakes |
| Code intelligence | File contents, AST summaries, imports | ~8,000 tokens | Grounds code generation in reality |
| Playbook rules | Matching learned heuristics | ~2,000 tokens | Enables faster, cheaper decisions |
| Daimon state | Affect vector, behavioral recommendations | ~1,000 tokens | Emotional regulation prevents thrashing |
| Research artifacts | Relevant research findings, citations | ~4,000 tokens | Grounds decisions in evidence |
| Oracle predictions | Forecasts and confidence intervals | ~2,000 tokens | Uncertainty-aware decision-making |

Total demand: ~30,000 tokens. Budget: ~32,000 tokens. The system usually fits, but
under pressure (complex tasks, large codebases, rich knowledge stores), demand exceeds
supply and something must be cut. The question is: **what gets cut, and how?**

**Fixed priority** (the naive approach) always cuts the same subsystems. But the
optimal allocation is context-dependent: for a coding task with many past failures,
iteration memory matters more than research; for a novel task, research matters more
than iteration memory. Fixed priority cannot adapt.

**VCG auction** discovers the optimal allocation dynamically. Each subsystem reports
its expected value truthfully (because second-price billing makes truthful reporting the
dominant strategy), and the governor allocates budget to maximize aggregate value.

### 2.2 Mechanism Design

The VCG mechanism has three properties that make it ideal for context allocation:

1. **Truthfulness (incentive compatibility).** Each bidder's dominant strategy is to bid
   its true value. No bidder can gain by over- or under-bidding. This eliminates the
   need for a designer to guess the "right" priorities.

2. **Efficiency (social welfare maximization).** The mechanism allocates the scarce
   resource (tokens) to maximize the total expected value across all bidders.

3. **Individual rationality.** No bidder is worse off participating in the auction than
   not participating. Every bidder receives non-negative utility.

### 2.3 The Auction Protocol

```
For each gamma tick at T1 or T2:

1. ANNOUNCE: The Context Governor announces the available token budget B.
   - T1: B = ~4,000 tokens (Focused context)
   - T2: B = ~32,000 tokens (Full Cognitive Workspace)

2. BID: Each subsystem i submits a bid:
   bid_i = (requested_tokens_i, value_per_token_i, content_i)

   where:
   - requested_tokens_i = how many tokens subsystem i wants
   - value_per_token_i = expected marginal value of each token to task success
   - content_i = the actual content to include (pre-rendered text)

3. ALLOCATE: The Governor solves the allocation problem:
   maximize  sum(allocated_i * value_per_token_i)
   subject to  sum(allocated_i) <= B
                0 <= allocated_i <= requested_tokens_i  for all i

4. PRICE: Each winning bidder pays the second-price (VCG payment):
   payment_i = value that other bidders lose due to i's allocation

5. ASSEMBLE: The Governor assembles the context window from allocated sections,
   ordered by the U-shape primacy/recency placement strategy.
```

### 2.4 The Bid Function: 8 Subsystem Formulas

Each subsystem computes its bid value using a subsystem-specific formula. The formulas
are designed to be fast (< 1ms each) and to produce values on a common scale [0.0, 1.0]
so bids are directly comparable.

#### Subsystem 1: Task Context Bidder

```rust
/// Task context is almost always high-value.
/// The bid decreases only when the task is extremely well-understood.
fn bid_task_context(state: &TickState) -> BidValue {
    let base = 0.95;  // task context is almost always critical
    let familiarity_discount = if state.task_retry_count > 3 {
        0.15  // agent has seen this task many times
    } else {
        0.0
    };
    let novelty_boost = if state.task_is_new {
        0.05  // new tasks need more context
    } else {
        0.0
    };
    BidValue {
        value_per_token: (base - familiarity_discount + novelty_boost).clamp(0.0, 1.0),
        requested_tokens: estimate_task_tokens(state),
    }
}
```

**Rationale.** Task context is the agent's "what am I doing?" section. Without it, the
agent cannot reason about the current objective. The base value of 0.95 reflects this
criticality. The familiarity discount (0.15 after 3+ retries) reflects that the agent
already has the task memorized. The novelty boost (0.05 for new tasks) reflects that
unfamiliar tasks benefit more from explicit context.

#### Subsystem 2: Neuro Knowledge Bidder

```rust
/// Neuro knowledge bids based on retrieval relevance and tier.
///
/// Higher-tier knowledge (Consolidated, Persistent) has been validated
/// through experience and is more valuable. Higher-relevance entries
/// are more likely to improve the current tick's outcome.
fn bid_neuro_knowledge(
    entries: &[NeuroEntry],
    state: &TickState,
) -> BidValue {
    if entries.is_empty() {
        return BidValue::zero();
    }

    let avg_relevance = entries.iter()
        .map(|e| e.relevance_to_task(&state.task))
        .sum::<f32>() / entries.len() as f32;

    let tier_weight = entries.iter()
        .map(|e| match e.tier {
            Tier::Transient => 0.3,
            Tier::Working => 0.5,
            Tier::Consolidated => 0.8,
            Tier::Persistent => 1.0,
        })
        .sum::<f32>() / entries.len() as f32;

    // Epistemic value: knowledge that reduces uncertainty is worth more
    let epistemic = state.uncertainty_level * 0.3;

    BidValue {
        value_per_token: (avg_relevance * 0.4 + tier_weight * 0.3 + epistemic)
            .clamp(0.0, 1.0),
        requested_tokens: entries.iter()
            .map(|e| e.estimated_tokens())
            .sum(),
    }
}
```

**Rationale.** Neuro entries with higher relevance to the current task provide more
value. Higher-tier entries (Consolidated, Persistent) have been validated through
multiple uses and are more reliable. Epistemic value scales with the agent's current
uncertainty -- when the agent is confused, knowledge is worth more.

#### Subsystem 3: Iteration Memory Bidder

```rust
/// Iteration memory bids higher when the agent has failed before.
///
/// Previous attempt feedback prevents repeating the same mistakes.
/// Value increases with failure count -- more failures mean the agent
/// needs more guidance from past experience.
fn bid_iteration_memory(
    iterations: &[IterationRecord],
    state: &TickState,
) -> BidValue {
    if iterations.is_empty() {
        return BidValue::zero();
    }

    let failure_count = iterations.iter()
        .filter(|i| !i.gate_passed)
        .count();

    let failure_weight = match failure_count {
        0 => 0.1,       // no failures: low value (nothing to learn from)
        1 => 0.5,       // one failure: moderate value
        2 => 0.75,      // two failures: high value
        _ => 0.9,       // 3+ failures: critical (agent is stuck)
    };

    // Recency: recent feedback is more valuable
    let recency = iterations.last()
        .map(|i| recency_decay(i.timestamp, 0.95))
        .unwrap_or(0.0);

    BidValue {
        value_per_token: (failure_weight * 0.7 + recency * 0.3).clamp(0.0, 1.0),
        requested_tokens: iterations.iter()
            .map(|i| i.estimated_tokens())
            .sum::<usize>()
            .min(4000),  // cap at 4K tokens
    }
}
```

**Rationale.** Iteration memory is the "don't repeat this mistake" signal. Its value
is directly proportional to failure count. An agent that has never failed on this task
gains little from iteration memory (0.1). An agent that has failed three or more times
urgently needs the previous feedback (0.9). Recency-weighting ensures that recent
feedback is prioritized over stale records.

#### Subsystem 4: Code Intelligence Bidder

```rust
/// Code intelligence bids based on task type and file relevance.
///
/// For coding tasks, relevant source files are critical. For non-coding
/// tasks, code intelligence has minimal value.
fn bid_code_intelligence(
    files: &[FileEntry],
    state: &TickState,
) -> BidValue {
    if !state.task_is_coding() || files.is_empty() {
        return BidValue::zero();
    }

    let avg_relevance = files.iter()
        .map(|f| f.relevance_score)
        .sum::<f32>() / files.len() as f32;

    // Complexity: complex files need more context
    let complexity_factor = files.iter()
        .map(|f| (f.cyclomatic_complexity as f32 / 20.0).min(1.0))
        .sum::<f32>() / files.len().max(1) as f32;

    // Import graph depth: deeply connected files need more context
    let connectivity = files.iter()
        .map(|f| (f.import_depth as f32 / 5.0).min(1.0))
        .sum::<f32>() / files.len().max(1) as f32;

    BidValue {
        value_per_token: (avg_relevance * 0.5 + complexity_factor * 0.3
            + connectivity * 0.2).clamp(0.0, 1.0),
        requested_tokens: files.iter()
            .map(|f| f.estimated_tokens())
            .sum(),
    }
}
```

**Rationale.** Code intelligence is the highest-volume bidder for coding tasks. File
relevance determines the base value. Complexity and connectivity boost the bid because
complex, highly-connected files are harder to reason about without seeing the actual
source. For non-coding tasks, this bidder returns zero.

#### Subsystem 5: Playbook Rules Bidder

```rust
/// Playbook rules bid based on match confidence and historical success.
///
/// Matching rules are cheap (< 500 tokens each) and high-value because
/// they encode validated heuristics from past experience.
fn bid_playbook_rules(
    matching_rules: &[PlaybookRule],
    state: &TickState,
) -> BidValue {
    if matching_rules.is_empty() {
        return BidValue::zero();
    }

    let avg_confidence = matching_rules.iter()
        .map(|r| r.confidence)
        .sum::<f32>() / matching_rules.len() as f32;

    let avg_success_rate = matching_rules.iter()
        .map(|r| r.success_rate)
        .sum::<f32>() / matching_rules.len() as f32;

    // High-confidence, high-success rules are very valuable
    BidValue {
        value_per_token: (avg_confidence * 0.5 + avg_success_rate * 0.5)
            .clamp(0.0, 1.0),
        requested_tokens: matching_rules.iter()
            .map(|r| r.estimated_tokens())
            .sum::<usize>()
            .min(2000),
    }
}
```

**Rationale.** Playbook rules are the most token-efficient content type. Each rule is
typically 200-500 tokens and encodes a validated pattern. High confidence + high success
rate means the rule reliably helps. Rules with low confidence are still included at
reduced priority because they may be useful.

#### Subsystem 6: Daimon State Bidder

```rust
/// Daimon state bids based on affect intensity and behavioral state.
///
/// The Daimon's bid is always modest (< 1K tokens) but increases
/// during high-affect periods when emotional regulation matters most.
fn bid_daimon_state(
    daimon: &DaimonState,
    state: &TickState,
) -> BidValue {
    let affect_intensity = daimon.pad_magnitude();

    // High affect intensity means emotional state is influencing decisions
    let intensity_factor = if affect_intensity > 0.5 {
        0.6  // strong emotions: regulation context matters
    } else if affect_intensity > 0.3 {
        0.3  // moderate emotions: helpful but not critical
    } else {
        0.1  // calm: minimal value from affect context
    };

    // Behavioral state matters: Struggling agents need affect awareness
    let behavioral_boost = match daimon.behavioral_state() {
        BehavioralState::Struggling => 0.3,
        BehavioralState::Exploring => 0.1,
        BehavioralState::Focused => 0.05,
        _ => 0.0,
    };

    BidValue {
        value_per_token: (intensity_factor + behavioral_boost).clamp(0.0, 1.0),
        requested_tokens: 800,  // Daimon state is compact
    }
}
```

**Rationale.** The Daimon's affect context is always small (~800 tokens: PAD vector,
behavioral state, somatic markers, recommended adjustments). Its value scales with
affect intensity -- during calm periods, the agent gains little from seeing its own
emotional state. During high-affect periods (struggling, anxious, excited), affect
awareness helps the agent regulate its behavior.

#### Subsystem 7: Research Artifacts Bidder

```rust
/// Research artifacts bid based on relevance and recency.
///
/// For research-heavy tasks, artifacts are critical. For routine
/// tasks, they provide background context at reduced priority.
fn bid_research_artifacts(
    artifacts: &[ResearchArtifact],
    state: &TickState,
) -> BidValue {
    if artifacts.is_empty() {
        return BidValue::zero();
    }

    let task_is_research = state.task_tags.contains("research")
        || state.task_tags.contains("analysis");

    let base = if task_is_research { 0.8 } else { 0.3 };

    let avg_relevance = artifacts.iter()
        .map(|a| a.relevance_to_task(&state.task))
        .sum::<f32>() / artifacts.len() as f32;

    let recency = artifacts.iter()
        .map(|a| recency_decay(a.timestamp, 0.98))
        .sum::<f32>() / artifacts.len() as f32;

    BidValue {
        value_per_token: (base * avg_relevance * 0.6 + recency * 0.4)
            .clamp(0.0, 1.0),
        requested_tokens: artifacts.iter()
            .map(|a| a.estimated_tokens())
            .sum::<usize>()
            .min(4000),
    }
}
```

**Rationale.** Research artifacts provide evidence-backed context. For research tasks,
they are critical (base 0.8). For routine tasks, they provide background (base 0.3).
Relevance and recency modulate the bid -- stale, irrelevant research wastes tokens.

#### Subsystem 8: Oracle Predictions Bidder

```rust
/// Oracle predictions bid based on prediction uncertainty.
///
/// When the agent is making predictions that it will later be evaluated
/// on, having calibration data and confidence intervals is valuable.
fn bid_oracle_predictions(
    predictions: &[PredictionState],
    state: &TickState,
) -> BidValue {
    if predictions.is_empty() || !state.task_involves_predictions() {
        return BidValue::zero();
    }

    let avg_uncertainty = predictions.iter()
        .map(|p| 1.0 - p.confidence)
        .sum::<f32>() / predictions.len() as f32;

    // High uncertainty means predictions are more valuable
    // (the agent needs calibration data to improve)
    let calibration_value = if avg_uncertainty > 0.5 {
        0.7  // very uncertain: calibration is critical
    } else if avg_uncertainty > 0.3 {
        0.4  // moderately uncertain
    } else {
        0.15  // well-calibrated: minimal additional value
    };

    BidValue {
        value_per_token: calibration_value.clamp(0.0, 1.0),
        requested_tokens: predictions.iter()
            .map(|p| p.estimated_tokens())
            .sum::<usize>()
            .min(2000),
    }
}
```

**Rationale.** Prediction context helps the agent calibrate its forecasts. When
uncertainty is high, calibration data is very valuable (0.7) because the agent needs
to understand its own prediction biases. When well-calibrated, the marginal value drops
to 0.15.

### 2.5 Second-Price Payment (VCG Payment Rule)

The VCG payment ensures truthful bidding. Each winning bidder i pays:

```
payment_i = (total value to others without i) - (total value to others with i)
```

This is the **externality** that bidder i imposes on the other bidders. In the context
allocation setting, it is the total value lost by other subsystems because i took some
of the token budget.

```rust
/// Compute VCG payments for the allocation.
///
/// The payment for bidder i is the value lost by other bidders
/// due to i's allocation. This makes truthful bidding the
/// dominant strategy: no bidder can gain by misreporting value.
fn compute_vcg_payments(
    bids: &[Bid],
    allocation: &[usize],
    budget: usize,
) -> Vec<f64> {
    let n = bids.len();
    let mut payments = vec![0.0; n];

    for i in 0..n {
        // Solve allocation WITHOUT bidder i
        let mut bids_without_i: Vec<Bid> = bids.iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, b)| b.clone())
            .collect();

        let allocation_without_i = solve_knapsack(
            &bids_without_i,
            budget,
        );

        // Value to others without i
        let value_without_i: f64 = bids_without_i.iter()
            .zip(allocation_without_i.iter())
            .map(|(b, a)| *a as f64 * b.value_per_token as f64)
            .sum();

        // Value to others with i
        let value_with_i: f64 = bids.iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, b)| allocation[j] as f64 * b.value_per_token as f64)
            .sum();

        // VCG payment: externality imposed on others
        payments[i] = value_without_i - value_with_i;
    }

    payments
}
```

**Why VCG payments matter.** Without second-price payments, subsystems could gain by
strategically inflating bids. A Neuro knowledge bidder that reports 0.9 value when the
true value is 0.5 would get more tokens at the expense of other subsystems, degrading
overall context quality. VCG payments eliminate this incentive: overbidding cannot
improve a bidder's allocation, and underbidding risks losing allocation to a competitor.

The payments are not "charged" in any monetary sense -- they are used for internal
accounting and bid calibration. The payment history is used to adjust future bid
parameters: if a subsystem consistently pays high externalities, it may be allocating
more tokens than it needs, and its bid function should be recalibrated.

### 2.6 Knapsack Solver

The allocation problem is a bounded knapsack problem: maximize total value subject to
a token budget constraint. With 8 bidders, the problem is small enough for exact
solution:

```rust
/// Solve the bounded knapsack for context allocation.
///
/// With 8 bidders and a budget of ~32,000 tokens, exact DP is
/// overkill. Greedy-by-value-density produces optimal or near-optimal
/// results in O(n log n).
fn solve_knapsack(bids: &[Bid], budget: usize) -> Vec<usize> {
    // Sort by value_per_token descending (greedy by density)
    let mut indexed: Vec<(usize, &Bid)> = bids.iter().enumerate().collect();
    indexed.sort_by(|a, b| {
        b.1.value_per_token.partial_cmp(&a.1.value_per_token)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut remaining = budget;
    let mut allocation = vec![0usize; bids.len()];

    for (idx, bid) in indexed {
        let alloc = bid.requested_tokens.min(remaining);
        allocation[idx] = alloc;
        remaining -= alloc;
        if remaining == 0 {
            break;
        }
    }

    allocation
}
```

The greedy approach is optimal for the fractional knapsack (which this effectively is,
since token allocations can be fractional by truncating content). For the rare case
where a subsystem's content cannot be meaningfully truncated, the integer variant uses
dynamic programming over the 8 items.

---

## 3. PAD Coefficients and Affect Modulation

The Daimon's PAD (Pleasure-Arousal-Dominance) vector modulates auction behavior
through three coefficient channels. Each coefficient adjusts one aspect of the
bidding process.

### 3.1 Pleasure Coefficient: Risk Tolerance

Pleasure modulates **risk tolerance** in the allocation. High pleasure (success,
positive outcomes) increases tolerance for exploratory allocations. Low pleasure
(failure, negative outcomes) increases preference for conservative, validated content.

```rust
/// Pleasure-modulated risk adjustment.
///
/// High pleasure -> more exploratory (include speculative content)
/// Low pleasure -> more conservative (prefer validated content)
fn pleasure_risk_adjustment(pleasure: f32) -> RiskAdjustment {
    if pleasure > 0.3 {
        RiskAdjustment {
            speculative_boost: 0.15,   // boost bids from speculative sources
            validated_boost: 0.0,
            tier_filter: None,         // include all tiers
        }
    } else if pleasure < -0.3 {
        RiskAdjustment {
            speculative_boost: -0.10,  // penalize speculative sources
            validated_boost: 0.10,     // boost validated knowledge
            tier_filter: Some(Tier::Working),  // exclude Transient tier
        }
    } else {
        RiskAdjustment::neutral()
    }
}
```

### 3.2 Arousal Coefficient: Attention Width

Arousal modulates **attention width** -- how many subsystems receive substantial
allocations. High arousal (surprise, urgency) narrows attention to the most critical
subsystems. Low arousal (calm, routine) broadens attention to include diverse sources.

```rust
/// Arousal-modulated attention width.
///
/// High arousal -> narrow focus (top 3-4 bidders get most budget)
/// Low arousal -> broad attention (all bidders get meaningful allocation)
fn arousal_attention_width(arousal: f32) -> AttentionWidth {
    if arousal > 0.5 {
        AttentionWidth {
            top_bidder_share: 0.70,   // top bidder gets 70% of budget
            min_allocation: 100,       // minimal allocation for others
            concentration: 0.9,        // highly concentrated
        }
    } else if arousal < -0.2 {
        AttentionWidth {
            top_bidder_share: 0.30,   // top bidder gets only 30%
            min_allocation: 500,       // meaningful allocation for all
            concentration: 0.4,        // broadly distributed
        }
    } else {
        AttentionWidth {
            top_bidder_share: 0.45,
            min_allocation: 300,
            concentration: 0.6,
        }
    }
}
```

### 3.3 Dominance Coefficient: Exploration vs. Exploitation

Dominance modulates the **exploration/exploitation balance**. High dominance (agent
feels in control) favors exploitation of known strategies. Low dominance (agent feels
uncertain) favors exploration via novel content.

```rust
/// Dominance-modulated exploration balance.
///
/// High dominance -> exploit (prefer content aligned with current strategy)
/// Low dominance -> explore (prefer novel, uncertainty-reducing content)
fn dominance_exploration_balance(dominance: f32) -> ExplorationBalance {
    if dominance > 0.3 {
        ExplorationBalance {
            exploit_weight: 0.8,       // strongly prefer known strategies
            explore_weight: 0.2,       // minimal exploration
            epistemic_boost: 0.0,      // no boost for uncertainty-reducing content
        }
    } else if dominance < -0.2 {
        ExplorationBalance {
            exploit_weight: 0.4,       // reduced preference for known strategies
            explore_weight: 0.6,       // strongly favor exploration
            epistemic_boost: 0.2,      // boost all uncertainty-reducing bids
        }
    } else {
        ExplorationBalance {
            exploit_weight: 0.6,
            explore_weight: 0.4,
            epistemic_boost: 0.1,
        }
    }
}
```

### 3.4 Combined PAD Bid Adjustment

The three PAD coefficients combine additively to adjust each subsystem's bid before
the auction:

```rust
/// Apply PAD-modulated adjustments to all bids.
///
/// Each subsystem's bid is adjusted based on the current PAD state.
/// Adjustments are additive, clamped to [0.0, 1.0].
fn adjust_bids_for_affect(
    bids: &mut [Bid],
    pad: &PadVector,
) {
    let risk = pleasure_risk_adjustment(pad.pleasure);
    let width = arousal_attention_width(pad.arousal);
    let explore = dominance_exploration_balance(pad.dominance);

    for bid in bids.iter_mut() {
        let mut adjustment = 0.0;

        // Pleasure: risk tolerance
        if bid.source.is_speculative() {
            adjustment += risk.speculative_boost;
        }
        if bid.source.is_validated() {
            adjustment += risk.validated_boost;
        }

        // Dominance: exploration/exploitation
        if bid.source.is_exploratory() {
            adjustment += explore.explore_weight * explore.epistemic_boost;
        }

        // Apply tier filter from pleasure
        if let Some(min_tier) = risk.tier_filter {
            if bid.source.tier() < min_tier {
                adjustment -= 0.20;  // significant penalty for excluded tiers
            }
        }

        bid.value_per_token = (bid.value_per_token + adjustment).clamp(0.0, 1.0);
    }
}
```

---

## 4. CorticalState: The Shared Perception Surface

The CorticalState is a fixed-layout struct of atomic fields written by the gamma loop
and read by all subsystems. It provides zero-latency, lock-free access to the agent's
current cognitive state.

### 4.1 Design Principles

1. **Atomic reads/writes.** All fields are `AtomicU32` or `AtomicI32`, read/written
   with `Ordering::Relaxed`. No locks, no contention, no blocking.

2. **Fixed layout.** The struct has a known size and layout at compile time. No heap
   allocations, no dynamic dispatch.

3. **Write-once-per-tick.** The gamma loop writes the CorticalState once per tick
   (at REACT). All reads during the tick see the previous tick's state, providing
   consistency without synchronization.

4. **Domain-agnostic.** The CorticalState contains only universal signals. Domain-
   specific state lives in the Substrate.

### 4.2 Fields

```rust
/// Shared atomic perception surface.
///
/// Written by the gamma loop at REACT, read by all subsystems at
/// any time. All reads are Relaxed-ordered -- subsystems may see
/// slightly stale values, which is acceptable for the cognitive
/// cross-cuts that read this state.
pub struct CorticalState {
    // ── Regime ──────────────────────────────────────────
    /// Current environmental regime (Calm=0, Normal=1, Volatile=2, Crisis=3)
    pub regime: AtomicU8,

    // ── PAD Affect Vector ──────────────────────────────
    /// Pleasure [-1.0, 1.0] stored as i16 * 1000
    pub pleasure: AtomicI32,
    /// Arousal [-1.0, 1.0] stored as i16 * 1000
    pub arousal: AtomicI32,
    /// Dominance [-1.0, 1.0] stored as i16 * 1000
    pub dominance: AtomicI32,

    // ── Behavioral State ────────────────────────────────
    /// Derived from PAD: Engaged=0, Struggling=1, Coasting=2,
    /// Exploring=3, Focused=4, Resting=5
    pub behavioral_state: AtomicU8,

    // ── Performance Signals ─────────────────────────────
    /// Rolling gate pass rate [0.0, 1.0] stored as u16 * 1000
    pub accuracy: AtomicU32,
    /// Daily budget usage [0.0, 1.0] stored as u16 * 1000
    pub budget_usage: AtomicU32,
    /// Resource health [0.0, 1.0] stored as u16 * 1000
    pub resource_health: AtomicU32,

    // ── Prediction Signals ──────────────────────────────
    /// Aggregate prediction error [0.0, 1.0] stored as u16 * 1000
    pub prediction_error: AtomicU32,
    /// World model drift [0.0, 1.0] stored as u16 * 1000
    pub drift: AtomicU32,
    /// Number of anomalous probes on the last tick
    pub anomaly_count: AtomicU32,

    // ── Activity Signals ────────────────────────────────
    /// Current tick number (monotonically increasing)
    pub tick: AtomicU64,
    /// Ticks since last T2 escalation
    pub ticks_since_t2: AtomicU32,
    /// Active task count
    pub active_tasks: AtomicU32,
    /// Episodes since last delta consolidation
    pub episodes_since_delta: AtomicU32,

    // ── Energy Fields (E23) ─────────────────────────────
    /// Cognitive energy [0.0, 1.0] stored as u16 * 1000
    pub cognitive_energy: AtomicU32,
    /// Motivational energy [0.0, 1.0] stored as u16 * 1000
    pub motivational_energy: AtomicU32,
    /// Behavioral vitality [0.0, 1.0] stored as u16 * 1000
    pub vitality: AtomicU32,
}
```

### 4.3 Read/Write Patterns

```rust
impl CorticalState {
    /// Read the PAD vector (3 relaxed atomic reads).
    pub fn pad(&self) -> PadVector {
        PadVector {
            pleasure: self.pleasure.load(Ordering::Relaxed) as f32 / 1000.0,
            arousal: self.arousal.load(Ordering::Relaxed) as f32 / 1000.0,
            dominance: self.dominance.load(Ordering::Relaxed) as f32 / 1000.0,
        }
    }

    /// Write the PAD vector (3 relaxed atomic writes).
    pub fn set_pad(&self, pad: &PadVector) {
        self.pleasure.store((pad.pleasure * 1000.0) as i32, Ordering::Relaxed);
        self.arousal.store((pad.arousal * 1000.0) as i32, Ordering::Relaxed);
        self.dominance.store((pad.dominance * 1000.0) as i32, Ordering::Relaxed);
    }

    /// Read the behavioral state derived from PAD.
    pub fn behavioral_state(&self) -> BehavioralState {
        BehavioralState::from_u8(self.behavioral_state.load(Ordering::Relaxed))
    }
}
```

---

## 5. Context Governor: The Orchestrator

The Context Governor runs the auction and assembles the final context window.

### 5.1 Governor Protocol

```rust
/// The Context Governor orchestrates context assembly.
///
/// For each tick that requires context (T1 or T2), the governor:
/// 1. Announces the token budget
/// 2. Collects bids from all registered subsystems
/// 3. Applies PAD affect modulation to bids
/// 4. Solves the allocation via the knapsack solver
/// 5. Computes VCG payments for accounting
/// 6. Assembles the context window with U-shape placement
pub struct ContextGovernor {
    bidders: Vec<Box<dyn AttentionBidder>>,
    cortical_state: Arc<CorticalState>,
}

impl ContextGovernor {
    pub fn assemble(
        &self,
        tier: InferenceTier,
        state: &TickState,
    ) -> Result<ContextWindow> {
        let budget = match tier {
            InferenceTier::T0 => return Ok(ContextWindow::empty()),
            InferenceTier::T1 => 4_000,
            InferenceTier::T2 => 32_000,
        };

        // 1. Collect bids
        let mut bids: Vec<Bid> = self.bidders.iter()
            .map(|b| b.bid(state))
            .collect();

        // 2. Apply PAD modulation
        let pad = self.cortical_state.pad();
        adjust_bids_for_affect(&mut bids, &pad);

        // 3. Solve allocation
        let allocation = solve_knapsack(&bids, budget);

        // 4. Compute VCG payments (for accounting/logging)
        let payments = compute_vcg_payments(&bids, &allocation, budget);

        // 5. Assemble with U-shape placement
        let window = assemble_u_shape(&bids, &allocation);

        // 6. Log auction result
        log_auction_result(&bids, &allocation, &payments, tier);

        Ok(window)
    }
}
```

### 5.2 U-Shape Placement Strategy

The assembled context follows the U-shape primacy/recency strategy: the most important
content goes at the beginning and end of the context window, with less critical content
in the middle. This exploits the serial position effect (Murdock 1962) -- LLMs, like
humans, attend more to the beginning and end of their input.

```rust
/// Assemble the context window with U-shape placement.
///
/// Position strategy:
/// - Beginning (primacy zone): invariants, task description, critical warnings
/// - Middle (filler zone): knowledge entries, code files, research
/// - End (recency zone): iteration feedback, playbook rules, Daimon state
fn assemble_u_shape(
    bids: &[Bid],
    allocation: &[usize],
) -> ContextWindow {
    let mut primacy = Vec::new();   // beginning
    let mut middle = Vec::new();     // middle
    let mut recency = Vec::new();    // end

    for (bid, alloc) in bids.iter().zip(allocation.iter()) {
        if *alloc == 0 { continue; }

        let content = bid.content.truncate_to(*alloc);

        match bid.placement_preference {
            Placement::Primacy => primacy.push(content),
            Placement::Middle => middle.push(content),
            Placement::Recency => recency.push(content),
        }
    }

    ContextWindow {
        sections: [primacy, middle, recency].concat(),
    }
}
```

### 5.3 Budget Tiers

| Tier | Budget | Sections Included | Placement |
|------|--------|-------------------|-----------|
| T0 | 0 | None (probes + playbook rules only) | N/A |
| T1 | ~4,000 | System prompt + task + top-5 knowledge + warnings | Fixed template |
| T2 | ~32,000 | Full VCG auction across all 8 subsystems | U-shape |

At T1, the context assembly skips the VCG auction and uses a fixed template with
hard-coded partitions. The auction overhead (~0.5ms for 8 bidders) is negligible at T2
but unnecessary at T1 where the budget is too small for meaningful competition.

---

## 6. Meta-Cognition Hooks

The Context Governor provides hooks for meta-cognitive assessment -- the agent's
ability to monitor and regulate its own cognitive processes.

### 6.1 Allocation Quality Monitoring

After each tick, the governor computes an **allocation quality score** by comparing
the auction allocation against the actual outcome:

```rust
/// Post-tick allocation quality assessment.
///
/// Measures whether the context assembly contributed to success.
/// Used to calibrate future bid functions.
fn assess_allocation_quality(
    allocation: &[usize],
    bids: &[Bid],
    outcome: &TickOutcome,
) -> f32 {
    // Correlation between allocation and positive outcome
    let value_allocated: f64 = bids.iter()
        .zip(allocation.iter())
        .map(|(b, a)| *a as f64 * b.value_per_token as f64)
        .sum();

    let success_signal = if outcome.gate_passed {
        1.0
    } else {
        0.0
    };

    // Rolling correlation: high value_allocated + success = good allocation
    (value_allocated as f32 * success_signal).min(1.0)
}
```

### 6.2 Bid Calibration

Over time, the governor calibrates each bidder's value estimates against actual
outcomes. If a bidder consistently bids high but contributes little to success (low
correlation between allocation and positive outcomes), its bids are discounted. If a
bidder consistently underbids and the agent succeeds more when it receives more tokens,
its bids are boosted.

```rust
/// Per-bidder calibration tracker.
///
/// Adjusts bid values based on historical outcome correlation.
pub struct BidCalibration {
    /// Per-bidder: rolling correlation between allocation and success.
    correlations: HashMap<BidderId, f32>,
    /// EMA smoothing factor.
    alpha: f32,  // default: 0.05
}

impl BidCalibration {
    /// Update calibration after a tick.
    pub fn update(
        &mut self,
        bidder_id: BidderId,
        allocated: usize,
        total_budget: usize,
        outcome_success: bool,
    ) {
        let share = allocated as f32 / total_budget.max(1) as f32;
        let signal = if outcome_success { 1.0 } else { -0.5 };
        let entry = self.correlations.entry(bidder_id).or_insert(0.5);
        *entry = *entry * (1.0 - self.alpha) + (share * signal) * self.alpha;
    }

    /// Calibration multiplier for a bidder.
    pub fn multiplier(&self, bidder_id: BidderId) -> f32 {
        let corr = self.correlations.get(&bidder_id).copied().unwrap_or(0.5);
        // Map [-1, 1] correlation to [0.5, 1.5] multiplier
        0.5 + corr
    }
}
```

---

## 7. Frequency Scheduler Integration

The attention auction interacts with the adaptive clock (see `adaptive-clock.md`)
through the CorticalState. The scheduler reads the following CorticalState fields to
adjust tick emission:

| CorticalState Field | Scheduler Effect |
|---------------------|------------------|
| `prediction_error` | High error -> faster gamma ticks |
| `anomaly_count` | More anomalies -> faster gamma ticks |
| `regime` | Volatile/Crisis -> faster theta ticks |
| `episodes_since_delta` | Threshold exceeded -> emit delta tick |
| `budget_usage` | High usage -> slower theta, restrict T2 |
| `behavioral_state` | Resting -> extend gamma interval |

The scheduler never reads bids or auction results directly. It uses the CorticalState
as a shared perception surface, maintaining the principle that the clock is a Bus
producer with no knowledge of domain-specific cognition.

---

## 8. Test Criteria

| Test | Assertion |
|---|---|
| Empty bids produce empty context | No allocation, no payments |
| Single bidder gets full request up to budget | Allocation = min(requested, budget) |
| Two bidders, budget < total request | Higher-value bidder gets priority |
| VCG payment = 0 when budget exceeds total demand | No externality when all fit |
| VCG payment > 0 when budget is constrained | Winner imposes externality |
| PAD pleasure < -0.3 filters Transient tier entries | Bid adjustment penalizes |
| PAD arousal > 0.5 narrows to top 3 bidders | Concentration increases |
| PAD dominance < -0.2 boosts exploratory bids | Epistemic boost applied |
| T1 uses fixed template, not auction | No bid collection at T1 |
| CorticalState atomic reads never block | Relaxed ordering, no contention |
| Allocation quality tracks gate pass correlation | Post-tick calibration runs |
| U-shape placement: task at start, feedback at end | Primacy/recency ordering |
| 8 bidders complete auction in < 1ms | Performance bound |
| Bid values are in [0.0, 1.0] after PAD adjustment | Clamp enforced |
| Knapsack solver respects budget exactly | sum(allocation) <= budget |

---

## 9. Configuration

| Parameter | Default | Range | Where |
|---|---|---|---|
| `t1_budget_tokens` | 4,000 | [2000, 8000] | `roko.toml [heartbeat.context]` |
| `t2_budget_tokens` | 32,000 | [16000, 128000] | `roko.toml [heartbeat.context]` |
| `pleasure_risk_threshold` | 0.3 | [0.1, 0.5] | `roko.toml [heartbeat.affect]` |
| `arousal_narrow_threshold` | 0.5 | [0.3, 0.8] | `roko.toml [heartbeat.affect]` |
| `dominance_explore_threshold` | -0.2 | [-0.5, 0.0] | `roko.toml [heartbeat.affect]` |
| `calibration_alpha` | 0.05 | [0.01, 0.20] | `roko.toml [heartbeat.context]` |
| `min_allocation_tokens` | 300 | [100, 1000] | `roko.toml [heartbeat.context]` |

---

## 10. Implementation Sources

| Surface | Authority | Status |
|---------|-----------|--------|
| AttentionBidder variants | `crates/roko-cli/src/runner/` | Neuro/Task/Research bidders wired |
| CorticalState energy fields | `crates/roko-agent/` | E23 10/10 energy/affect live |
| PredictiveScorer | `crates/roko-core/src/score.rs` | EFE-approximate scoring shipped |
| Cross-cut functors | `crates/roko-cli/src/runner/` | E44 VCG conflict resolution live |
| SystemPromptBuilder | `crates/roko-compose/src/system_prompt_builder.rs` | 9-layer prompt assembly |

---

## 11. References

### Auction theory

- **Vickrey 1961** -- "Counterspeculation, Auctions, and Competitive Sealed Tenders"
  (Journal of Finance 16(1)). Second-price sealed-bid auctions.
- **Clarke 1971** -- "Multipart Pricing of Public Goods" (Public Choice 11(1)).
  Generalization of Vickrey to multiple goods.
- **Groves 1973** -- "Incentives in Teams" (Econometrica 41(4)). Incentive-compatible
  mechanisms for team production.
- **Nisan & Ronen 2001** -- "Algorithmic Mechanism Design" (Games and Economic
  Behavior 35(1-2)). Computational aspects of truthful mechanisms.

### Cognitive science

- **Baddeley 2000** -- "The Episodic Buffer" (Trends in Cognitive Sciences 4(11)).
  Working memory model.
- **Murdock 1962** -- "The serial position effect of free recall" (Journal of
  Experimental Psychology 64(5)). Primacy-recency effect in memory.
- **Baars 1988** -- "A Cognitive Theory of Consciousness" (Cambridge University Press).
  Global Workspace Theory.
- **Barrett 2017** -- "How Emotions Are Made" (Houghton Mifflin). Constructed emotion
  from prediction residuals.
- **Damasio 1994** -- "Descartes' Error" (Putnam). Somatic marker hypothesis.
- **Kahneman 2011** -- "Thinking, Fast and Slow" (Farrar, Straus and Giroux).
  Dual-process theory.

### Active inference

- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Parr & Friston 2017** -- "Working memory, attention, and salience in active
  inference" (Scientific Reports 7). Attention as precision weighting.
- **Sims 2003** -- "Implications of rational inattention" (Journal of Monetary
  Economics 50(3)). Optimal attention allocation.

### Agent efficiency

- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176). Cascade cost optimization.
- **Gebhard 2005** -- "ALMA: A Layered Model of Affect" (AAMAS 2005). Three-layer
  affect model.

---

## Cross-References

- `docs/v3/depth/29-heartbeat/coala-9-step-pipeline.md` -- Pipeline context
- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Tier gating that drives
  budget selection
- `docs/v3/depth/29-heartbeat/adaptive-clock.md` -- Frequency scheduler
- `docs/v3/depth/29-heartbeat/active-inference-compute-allocation.md` -- EFE theory
  underlying bid formulas
- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Where the auction runs
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
