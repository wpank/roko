# VCG Attention Auction

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/auction.rs` -- VCG allocation and `LearningBidder`
> v1 source: `docs/v1/03-composition/10-vcg-attention-auction.md`

---

## Overview

The VCG (Vickrey-Clarke-Groves) attention auction applies mechanism design to the
problem of allocating the scarce context window among competing cognitive subsystems.
Each subsystem (Neuro, Daimon, TaskContext, CodeIntelligence, PlaybookRules,
Research, IterationMemory, Oracles, GroupContext) bids for attention bandwidth based
on its expected contribution to task success. The VCG mechanism ensures truthful
bidding (no subsystem benefits from misrepresenting its value), efficient allocation
(the combination maximizing total value wins), and individual rationality (no
subsystem is worse off for participating). Winners pay the externality they impose
on others -- a second-price rule -- not their own bid.

Herbert Simon (1971): "A wealth of information creates a poverty of attention."

---

## 1. The Attention Allocation Problem

The context window is scarce. A 128K-token model with 28K reserved for output has
approximately 100K tokens of input budget. Nine cognitive subsystems compete for
this budget:

| # | Subsystem | What It Wants to Inject |
|---|-----------|------------------------|
| 1 | **Neuro** | Durable knowledge, insights, heuristics from the knowledge store |
| 2 | **Daimon** | Affect-modulated guidance, motivational state |
| 3 | **IterationMemory** | Recent turns, retries, prior outputs from this task |
| 4 | **CodeIntelligence** | Symbols, file content, structural context |
| 5 | **PlaybookRules** | Skills, playbooks, distilled rules from learning |
| 6 | **Research** | Research memos, external domain context |
| 7 | **TaskContext** | Task brief, plan, PRD slices, acceptance criteria |
| 8 | **Oracles** | Predictions, warnings, forecast outputs |
| 9 | **GroupContext** | Mesh knowledge, cross-agent context |

Each subsystem believes its content is the most important. Without a coordination
mechanism, the subsystem that produces the most content dominates the prompt -- not
because its content is the most valuable, but because it is the loudest.

The naive approach -- static priority ordering -- is fragile. A priority table
tuned for implementation tasks misallocates for research tasks. A priority table
tuned for the first attempt misallocates for retry attempts where gate errors
should dominate. The VCG mechanism provides a principled, adaptive, self-correcting
alternative.

---

## 2. The VCG Mechanism

### 2.1 Origins

The VCG mechanism combines three foundational results in mechanism design:

**Vickrey (1961), "Counterspeculation, Auctions, and Competitive Sealed
Tenders."** Journal of Finance, 16(1), 8-37. In a second-price auction, the
winner pays the second-highest bid. This incentivizes truthful bidding -- a
bidder's optimal strategy is to bid its true value, regardless of what others
bid. If a bidder bids below true value, it risks losing an auction it should
have won. If it bids above true value, it risks winning at a price exceeding
its value. Truthful bidding is a dominant strategy.

**Clarke (1971), "Multipart Pricing of Public Goods."** Public Choice, 11(1),
17-33. Extended second-price auctions to multiple items. Each winner pays the
externality their allocation imposes on others -- the reduction in total welfare
caused by their presence in the auction.

**Groves (1973), "Incentives in Teams."** Econometrica, 41(4), 617-631. Proved
that the VCG payment rule is the unique mechanism that simultaneously achieves
truthful bidding and efficient allocation for quasi-linear utility functions.

### 2.2 Formal Properties

| Property | Formal Definition | Meaning for Context Allocation |
|----------|------------------|-------------------------------|
| **Truthful** | `bid_i* = v_i` is a dominant strategy for all i | Each subsystem's optimal strategy is to bid its true expected value. No gaming. |
| **Efficient** | Allocation maximizes `Sum v_i * x_i` | The allocation maximizes total expected value across all subsystems. |
| **Individually rational** | `utility_i >= 0` for all winners | No subsystem is made worse off by participating. |
| **Weakly budget balanced** | `Sum payments <= Sum values` | Total payments do not exceed total welfare generated. |

### 2.3 Why VCG Over Static Priorities

Static priorities are a special case of VCG where all bids are fixed constants.
VCG generalizes this by allowing bids to be:
- **Learned** from historical task outcomes (Thompson sampling)
- **Modulated** by affect state (PAD vector)
- **Adaptive** to task complexity and domain uncertainty
- **Self-correcting** through outcome feedback

The static priority system remains as a cold-start fallback. The PromptComposer
auto-selects VCG mode when bidders have accumulated sufficient observations to
produce calibrated bids.

---

## 3. The Bid Formula

From the canonical specification:

```
bid(section) = expected_value * urgency * affect_weight

Where:
  expected_value = track_record(section) * relevance(section)
  urgency        = 1.0 + max(0, (deadline - now) / total_time_budget)^(-1)
  affect_weight  = daimon_modulation(section.type, current_pad_state)
```

### 3.1 Expected Value

```
expected_value = E[task_success | section_included] * relevance_to_current_task
```

This is the same `track_record` used in the active inference scorer (see
`active-inference-context-selection.md`), multiplied by a task-specific relevance
score. The expected value measures: "if I include this section, how much does it
improve the probability of task success?"

The track record component is computed from historical episode data:

```rust
fn track_record(section_type: &str, task_category: &str) -> f64 {
    let pass_when_included = episodes
        .filter(|e| e.included_sections.contains(section_type))
        .filter(|e| e.task_category == task_category)
        .mean(|e| e.gate_passed as f64);

    let pass_when_excluded = episodes
        .filter(|e| !e.included_sections.contains(section_type))
        .filter(|e| e.task_category == task_category)
        .mean(|e| e.gate_passed as f64);

    // Positive = this section helps. Negative = this section hurts.
    pass_when_included - pass_when_excluded
}
```

### 3.2 Urgency

```
urgency = 1.0 + max(0, (deadline - now) / total_time_budget)^(-1)
```

When the agent is under time pressure (approaching a deadline or budget limit),
urgency increases. High urgency amplifies bids for action-oriented content (task
description, file context, gate errors) and dampens bids for exploratory content
(research memos, cross-plan context). In the limit, as deadline approaches, only
the most direct task-relevant content survives.

### 3.3 Affect Weight

The Daimon's PAD (Pleasure-Arousal-Dominance) state (Mehrabian 1996) modulates bids:

| PAD State | Modulation | Effect |
|-----------|-----------|--------|
| High arousal (>= 0.35) | x1.3 for action-oriented, x0.7 for exploratory | Time pressure: narrow to proven context |
| Low pleasure (<= -0.35) | x1.5 for anti-patterns/warnings, x0.8 for standard | Caution after failures: boost warnings |
| Low dominance (<= -0.35) | x1.2 for explanatory, x0.9 for directive | Seeking understanding before acting |
| Neutral | x1.0 (no modulation) | Default behavior |

The affect weight ensures that the auction responds to the agent's motivational
state. An anxious agent (low pleasure, high arousal) automatically receives more
cautionary context without any manual priority adjustment.

---

## 4. The Nine Bidding Subsystems

Each subsystem implements a bid computation that combines the base formula with
subsystem-specific modulation. The `AttentionBidder` enum in
`crates/roko-compose/src/prompt.rs` tags sections so the PromptComposer can
compute per-subsystem metrics:

```rust
pub enum AttentionBidder {
    Neuro,             // Durable knowledge from neuro store
    Daimon,            // Affect/somatic guidance
    IterationMemory,   // Recent turns, retries, prior outputs
    CodeIntelligence,  // Symbols, files, structural context
    PlaybookRules,     // Skills, playbooks, distilled rules
    Research,          // Research memos, external domain context
    TaskContext,       // Task brief, plan, PRD slices (default)
    Oracles,           // Predictions, warnings, forecasts
    GroupContext,       // Membership-scoped group knowledge
}
```

### 4.1 Subsystem Bid Profiles

| # | Subsystem | Typical Bid Range | When Bids High | When Bids Low |
|---|-----------|-------------------|----------------|---------------|
| 1 | **Neuro** | 0.4-0.9 | Domain with rich knowledge, low pleasure (warnings) | Novel domain, no prior knowledge |
| 2 | **Daimon** | 0.1-0.4 | Extreme PAD state (anxious, excited) | Neutral affect |
| 3 | **IterationMemory** | 0.3-0.8 | Retry attempts (gate errors critical) | First attempt |
| 4 | **CodeIntelligence** | 0.5-0.9 | Implementation tasks needing signatures | Planning tasks |
| 5 | **PlaybookRules** | 0.3-0.7 | Tasks matching learned rules | Novel task types |
| 6 | **Research** | 0.3-0.7 | Novel domains, complex integration | Well-understood domains |
| 7 | **TaskContext** | 0.7-1.0 | Always high (Critical priority) | Never low |
| 8 | **Oracles** | 0.2-0.6 | Active predictions/warnings exist | No predictions |
| 9 | **GroupContext** | 0.2-0.6 | Agent has group membership, peer knowledge | Solo agent |

### 4.2 Per-Subsystem Bid Computation

Each subsystem computes its bid using the base formula plus affect modulation
via the `subsystem_bias` formula:

```rust
match section.bidder {
    AttentionBidder::Neuro => {
        1.0 + urgency * 0.12 * warningish
            + low_dominance * 0.18 * exploratory
            + low_pleasure * 0.28 * warningish
    }
    AttentionBidder::Daimon => {
        // Affect-modulated: higher when emotional state is extreme
        // Maximum bid when PAD vector magnitude exceeds threshold
        1.0 + pad_magnitude * 0.35
    }
    AttentionBidder::IterationMemory => {
        1.0 + urgency * 0.14 * proven
            + low_dominance * 0.20 * exploratory
            + low_pleasure * 0.22 * conservative.max(proven)
    }
    AttentionBidder::CodeIntelligence => {
        1.0 + urgency * 0.16 * action_oriented
            + low_dominance * 0.12 * structural
    }
    AttentionBidder::PlaybookRules => {
        1.0 + urgency * 0.10 * proven
            + low_pleasure * 0.18 * warningish
    }
    AttentionBidder::Research => {
        1.0 + low_dominance * 0.30 * exploratory.max(1.0)
    }
    AttentionBidder::TaskContext => {
        1.0 + urgency * 0.18 * deadline.max(1.0)
    }
    AttentionBidder::Oracles => {
        1.0 + urgency * 0.22 * prediction.max(1.0)
    }
    AttentionBidder::GroupContext => {
        1.0 + urgency * 0.18 * warningish.max(deadline)
            + low_dominance * 0.18 * exploratory
    }
}
```

The modulation variables are derived from the PAD state:
- `warningish`: 1.0 when low pleasure signals caution
- `exploratory`: 1.0 when low arousal signals exploration mode
- `proven`: 1.0 when sections have high track record
- `conservative`: 1.0 when both low pleasure and low dominance
- `action_oriented`: 1.0 when high arousal signals urgency
- `structural`: 1.0 when task involves cross-crate work
- `deadline`: approaches infinity as time pressure increases

---

## 5. The Auction Algorithm

### 5.1 Combinatorial Allocation

The attention auction is a **combinatorial auction**: the auctioneer (Composer)
must allocate multiple items (context window slots) to multiple bidders
(subsystems). Two knowledge entries from the same domain may be worth more
together than separately (complementary). Two overlapping entries may be worth
less than either alone (substitutes).

The allocation problem is formally:

```
maximize   Sum_i value_i * x_i
subject to Sum_i tokens_i * x_i <= budget
           x_i in {0, 1}  for all i
```

This is the 0/1 knapsack problem. While NP-hard in general, for prompt assembly
with N < 50 candidates, the greedy approximation is both tractable and near-optimal.

### 5.2 VCG Allocation Rule

```
Algorithm: VCG allocation for context assembly

1. COLLECT bids from all 9 subsystems
   bids = {(section_i, value_i, tokens_i)} for i in 1..N

2. FIND the allocation maximizing total value within budget
   Sort candidates by value_i / tokens_i (value density) descending
   Greedily include candidates until budget is exhausted

3. ENFORCE Critical sections
   Sections with SectionPriority::Critical are always included
   (TaskContext sections are never dropped, only truncated)

4. RECORD the optimal allocation: x* = greedy solution
   optimal_welfare = Sum_{i: x_i* = 1} value_i
```

### 5.3 VCG Payment Rule (Second-Price)

Each winning section pays the externality it imposes on others:

```
payment(section_i) = Sum_{j != i} value_j(optimal_allocation_without_i)
                   - Sum_{j != i} value_j(optimal_allocation_with_i)
```

In words: section i pays the difference between the total value others would
achieve without i in the auction and the total value others achieve with i
present. This is the "damage" that i's inclusion causes to others by consuming
budget that they could have used.

**Worked example:**

Suppose we have a 1000-token budget and three candidate sections:

| Section | Tokens | Value | Density |
|---------|--------|-------|---------|
| A (TaskContext) | 400 | 0.9 | 2.25 |
| B (Neuro) | 500 | 0.7 | 1.40 |
| C (Research) | 300 | 0.5 | 1.67 |

**Step 1: Greedy allocation.** Sort by density: A (2.25), C (1.67), B (1.40).
Include A (400 tokens, 600 remaining). Include C (300 tokens, 300 remaining).
B does not fit (500 > 300). Winners: {A, C}. Total welfare: 0.9 + 0.5 = 1.4.

**Step 2: Compute payments.**

Payment for A: Without A, the optimal allocation is {B, C}
(B fits: 500 tokens, C fits: 300 tokens, total 800 <= 1000).
Value others get without A: 0.7 + 0.5 = 1.2.
Value others get with A: 0.5 (only C wins alongside A).
Payment(A) = 1.2 - 0.5 = 0.7.

Payment for C: Without C, the optimal allocation is {A, B}
(A: 400 + B: 500 = 900 <= 1000).
Value others get without C: 0.9 + 0.7 = 1.6.
Value others get with C: 0.9 (only A wins alongside C).
Payment(C) = 1.6 - 0.9 = 0.7.

**Interpretation:** Both A and C impose significant externalities (0.7 each).
A's inclusion displaces B (0.7 value). C's inclusion also displaces B.
This signals that B is the "marginal loser" -- improvements to B's value or
reductions in its token count would change the allocation.

### 5.4 Why Payments Matter

In a single-agent system, payments are accounting constructs -- no actual money
changes hands. Their purpose is **diagnostic**. A section with a high payment is
consuming disproportionate budget relative to its value. This signals:

1. **The section should be compressed** -- same value in fewer tokens reduces
   its externality.
2. **The section should be split** -- breaking it into independently biddable
   parts allows finer allocation.
3. **The budget should be increased** for this tier -- if high-value sections
   are consistently displaced, the overall budget is too small.
4. **A low-value section is occupying critical space** -- if a section wins
   with low value but high token count, it may be a candidate for removal.

Payments provide a principled measure of "budget pressure" that manual priority
tuning cannot replicate. The diagnostic data feeds into the `AuctionDiagnostics`
struct that tracks per-assembly metrics.

---

## 6. Strategic Bidding via Thompson Sampling

### 6.1 The Learning-Truthfulness Tension

VCG guarantees truthful bidding is a dominant strategy in a **single-shot**
auction. In **repeated** auctions (context assembly runs for every task),
strategic behavior can theoretically emerge. However, research on learning in
repeated auctions (MIT CEEPR Working Paper 2023-18) shows that no-regret
learning algorithms tend to converge to welfare-maximizing equilibria. VCG's
second-price rule further dampens strategic incentives.

### 6.2 The LearningBidder

Each subsystem maintains a posterior distribution over its bid value, updated
by task outcomes using Thompson sampling:

```rust
/// A subsystem that learns its bid value from historical outcomes.
/// Source: crates/roko-compose/src/auction.rs
pub struct LearningBidder {
    pub subsystem_id: SubsystemId,
    /// Per-section Beta distributions for Thompson sampling.
    /// Beta(alpha, beta) where alpha = successes when included,
    /// beta = failures when included.
    pub section_betas: HashMap<String, (f64, f64)>,
    /// Prior bid value (before learning).
    pub prior_bid: f64,
}

impl LearningBidder {
    /// Compute bid for a section using Thompson sampling.
    pub fn bid(&self, section_name: &str, relevance: f64) -> f64 {
        let (alpha, beta) = self.section_betas
            .get(section_name)
            .copied()
            .unwrap_or((1.0, 1.0));  // Uniform prior

        // Sample from Beta(alpha, beta) for exploration
        let sampled_track_record = beta_sample(alpha, beta);

        // Bid = sampled track record * relevance to current task
        sampled_track_record * relevance
    }

    /// Update after observing a task outcome.
    pub fn update(
        &mut self,
        section_name: &str,
        was_included: bool,
        gate_passed: bool,
    ) {
        if was_included {
            let entry = self.section_betas
                .entry(section_name.to_string())
                .or_insert((1.0, 1.0));
            if gate_passed {
                entry.0 += 1.0;  // alpha: success count
            } else {
                entry.1 += 1.0;  // beta: failure count
            }
        }
    }
}
```

### 6.3 Thompson Sampling Properties

Thompson sampling provides natural exploration-exploitation balance:

1. **Exploration**: Sections with uncertain value (wide Beta distribution) are
   occasionally sampled high, ensuring the system discovers their true value.
   A section with Beta(2, 2) has high variance -- it may be sampled at 0.8
   one task and 0.3 the next.

2. **Exploitation**: As observations accumulate, the Beta distribution narrows
   and bids converge to the true expected value. A section with Beta(50, 10)
   reliably bids near 0.83.

3. **Cold start**: With the uniform prior Beta(1, 1), all sections start with
   equal expected value (0.5). The system explores broadly until it discovers
   which sections are valuable.

### 6.4 Convergence Properties

From the MARL auction literature (arXiv:2402.19420, 2024):

| Property | Expected Behavior |
|----------|------------------|
| Convergence time | ~50-100 tasks per subsystem to stabilize bids |
| Equilibrium type | Welfare-maximizing (under VCG payment rule) |
| Exploration rate | Decreasing: Beta distributions narrow over time |
| Sensitivity to distribution shift | Moderate: sudden shifts require re-exploration |

The PromptComposer auto-selects VCG mode when the minimum subsystem observation
count exceeds a configurable threshold (default: 20). Below this threshold,
the static priority system is used as a cold-start fallback.

### 6.5 Persistence

LearningBidder state persists to `.roko/learn/auction-bidders.json`:

```json
{
  "version": 1,
  "updated_at": "2026-09-12T10:00:00Z",
  "bidders": {
    "neuro": {
      "section_betas": {
        "knowledge_insight": [42.0, 8.0],
        "knowledge_warning": [28.0, 3.0],
        "knowledge_heuristic": [15.0, 12.0]
      },
      "prior_bid": 0.6
    },
    "iteration_memory": {
      "section_betas": {
        "gate_errors": [55.0, 5.0],
        "prior_output": [22.0, 18.0]
      },
      "prior_bid": 0.5
    }
  }
}
```

---

## 7. Auction Efficiency Metrics

### 7.1 Welfare Loss

The **welfare loss** (deadweight loss) measures how much total value is lost
compared to the optimal allocation:

```
welfare_loss = optimal_total_value - actual_total_value

Where:
  optimal_total_value = value of the allocation maximizing Sum v_i * x_i
                        subject to Sum tokens_i * x_i <= budget
  actual_total_value  = value of the allocation produced by the auction
```

For the greedy knapsack used in Roko, the welfare loss is bounded:

```
greedy_welfare >= 0.5 * optimal_welfare  (Dantzig 1957)
```

In practice, with section values correlated to their token size, the greedy
approximation is much tighter -- typically > 90% of optimal for the candidate
set sizes (N < 50) seen in prompt assembly.

### 7.2 Pareto Optimality

An allocation is **Pareto optimal** if no section can be added without removing
another section of equal or greater value. The VCG mechanism produces
Pareto-optimal allocations when the welfare maximization is exact.

```rust
/// Check if a VCG allocation is Pareto optimal.
/// Source: crates/roko-compose/src/auction.rs
pub fn is_pareto_optimal(
    included: &[SectionAllocation],
    excluded: &[SectionAllocation],
    budget_remaining: usize,
) -> bool {
    // Can we add any excluded section without removing anything?
    for exc in excluded {
        if exc.tokens <= budget_remaining {
            return false;  // Free improvement available
        }
    }
    // Can we swap any excluded section for an included one to improve welfare?
    for exc in excluded {
        for inc in included {
            if inc.value < exc.value && inc.tokens >= exc.tokens {
                return false;  // Swap improves welfare
            }
        }
    }
    true
}
```

### 7.3 Price of Anarchy

The **Price of Anarchy** (PoA) measures welfare loss from strategic behavior:

```
PoA = welfare(socially optimal) / welfare(worst Nash equilibrium)
```

Under VCG with truthful bidding, PoA = 1 (no loss from strategic behavior).
The concern is the greedy approximation: when welfare maximization is approximate
(greedy knapsack), VCG payments no longer guarantee exact truthfulness (Nisan &
Ronen, "Computationally Feasible VCG Mechanisms").

The practical PoA for Roko's context allocation is estimated at < 1.1 (less
than 10% welfare loss) based on the small candidate set size (N < 50).

Research on strong and Pareto equilibria (Chien & Sinclair, UC Berkeley) shows
that the PoA for Pareto-optimal Nash equilibria is significantly smaller than
for arbitrary Nash equilibria in congestion games -- a related allocation setting.

### 7.4 Diagnostic Dashboard

```rust
/// Auction diagnostics computed after each context assembly.
/// Source: crates/roko-compose/src/auction.rs
pub struct AuctionDiagnostics {
    /// Total bid value of winning sections.
    pub total_welfare: f64,
    /// Total VCG payments across all winners.
    pub total_payments: f64,
    /// Welfare loss vs. optimal (estimated by exhaustive search for N < 20).
    pub welfare_loss: f64,
    /// Is the allocation Pareto optimal?
    pub pareto_optimal: bool,
    /// Sections with highest payment (most budget pressure).
    pub highest_payment_sections: Vec<(String, f64)>,
    /// Sections that were displaced (excluded due to budget).
    pub displaced_sections: Vec<(String, f64)>,
    /// Budget utilization: tokens_used / tokens_available.
    pub budget_utilization: f64,
}
```

These diagnostics are emitted after each composition. They power:
- **Cost prediction**: estimated tokens directly predict inference cost.
- **Budget tuning**: consistently displaced high-value sections indicate the
  budget needs expansion.
- **Section compression**: high-payment sections are candidates for compression
  to reduce their externality.
- **Subsystem health**: a subsystem whose sections are consistently displaced
  may need its bidding strategy recalibrated.

---

## 8. Alternative Fairness Criteria

VCG maximizes aggregate welfare, but there are scenarios where other fairness
criteria are more appropriate.

### 8.1 Proportional Fairness

Each subsystem receives allocation proportional to its bid:

```
allocation_i = (bid_i / Sum bid_j) * total_budget
```

**Advantage:** Every subsystem gets some representation. No subsystem is
completely starved.

**Disadvantage:** Low-value subsystems consume budget that higher-value
subsystems need. Can produce worse outcomes than aggressive priority-based
dropping.

**When to use:** When the system has no confidence in bid accuracy (early
cold-start phase, or when all subsystems are poorly calibrated). Proportional
fairness is the safe default.

Research: Regularized Proportional Fairness (RPF) (Zhu et al., ICLR 2025,
arXiv:2501.01111) adds neural-network-learned regularization to standard PF,
increasing robustness to misreported bids.

### 8.2 Max-Min Fairness

Maximize the minimum allocation across all subsystems:

```
max min_i allocation_i
subject to Sum allocation_i <= total_budget
```

**Advantage:** The worst-served subsystem is as well-served as possible.
Prevents catastrophic context gaps.

**Disadvantage:** Very inefficient -- gives equal weight to low-value and
high-value subsystems. The Daimon's 50-token affect guidance gets the same
allocation as the CodeIntelligence module's 8000-token file context.

**When to use:** Only for safety-critical subsystems. A max-min guarantee on
the Safety subsystem ensures that safety constraints always get minimum viable
representation, regardless of how other subsystems bid.

### 8.3 Alpha-Fairness Spectrum

The three criteria are special cases of the alpha-fairness family
(Bertsimas et al.):

```
maximize Sum_i (allocation_i^(1-alpha)) / (1-alpha)

alpha = 0:    Utilitarian (VCG) -- maximize total welfare
alpha = 1:    Proportional fairness -- maximize geometric mean
alpha -> inf: Max-min fairness -- maximize the minimum
```

Roko implements a configurable alpha parameter:

```rust
/// Configurable fairness parameter for the attention auction.
/// Source: crates/roko-compose/src/auction.rs
pub struct FairnessConfig {
    /// Alpha parameter for the alpha-fairness family.
    /// 0.0 = pure efficiency (VCG-like)
    /// 1.0 = proportional fairness
    /// 10.0 = approximately max-min
    pub alpha: f64,  // default: 0.0 (pure efficiency)
    /// Minimum guaranteed allocation for safety subsystem (max-min floor).
    pub safety_floor_tokens: usize,  // default: 200
}
```

### 8.4 Hybrid Policy: VCG + Safety Floor

The recommended and implemented policy combines VCG efficiency with a max-min
floor for safety:

```
1. Reserve safety_floor_tokens for the Safety subsystem (guaranteed minimum)
2. Run VCG auction on the remaining budget across all subsystems
3. Safety subsystem can bid for ADDITIONAL tokens beyond its floor
4. All other subsystems compete in the standard VCG auction
```

This ensures safety constraints always appear (max-min floor) while maximizing
total value for the remaining budget (VCG efficiency). The safety floor is
never subject to auction pressure -- it is a constitutional guarantee, not an
economic one.

---

## 9. Collusion Detection

Although subsystems under the same operator's control have no incentive to
collude, structural coupling can create emergent collusion-like behavior. For
example, two subsystems that share a relevance signal (both key off the same
task description tokens) may always bid high together, effectively forming a
bidding ring that captures more budget than warranted.

### 9.1 Correlation-Based Detection

```rust
/// Detect bid correlation that might indicate structural coupling.
/// Source: crates/roko-compose/src/auction.rs
pub fn detect_bid_correlation(
    bid_history: &[(SubsystemId, SubsystemId, Vec<(f64, f64)>)],
    threshold: f64,  // default: 0.85
) -> Vec<(SubsystemId, SubsystemId, f64)> {
    bid_history.iter()
        .filter_map(|(s1, s2, pairs)| {
            let correlation = pearson_correlation(pairs);
            if correlation > threshold {
                Some((*s1, *s2, correlation))
            } else {
                None
            }
        })
        .collect()
}
```

### 9.2 Mitigation

When high correlation is detected between subsystems:

1. **Investigate shared inputs**: check if both subsystems derive relevance
   from the same signal. If so, deduplicate the input.
2. **Apply bid dampening**: reduce one subsystem's bid by a decorrelation
   factor to prevent joint over-bidding.
3. **Merge subsystems**: if two subsystems consistently bid identically, they
   may represent the same cognitive function and should be consolidated.

---

## 10. Relationship to Active Inference

The VCG auction and active inference (see `active-inference-context-selection.md`)
solve the same allocation problem through different mechanisms:

| Aspect | Active Inference | VCG Auction |
|--------|-----------------|-------------|
| **Setting** | Single agent, centralized | Multi-subsystem, decentralized |
| **Scoring** | EFE: pragmatic + epistemic | Bid: expected_value * urgency * affect |
| **Selection** | Softmax over scores | Combinatorial optimization |
| **Exploration** | Emerges from epistemic value | Emerges from Thompson sampling |
| **Optimality** | Maximizes expected free energy | Maximizes total welfare |
| **Truthfulness** | N/A (single scorer) | Guaranteed (VCG property) |
| **Complementarity** | WHAT to include | HOW MUCH budget to allocate |

Both converge on the same allocation under certain conditions:
- When all subsystems bid truthfully (VCG guarantees this), the VCG allocation
  maximizes total value.
- When the EFE scorer has accurate track_record estimates, the softmax selection
  approximates the value-maximizing allocation.

The practical integration: active inference scores individual sections within a
subsystem (which knowledge entries are most relevant?), while VCG allocates
budget across subsystems (how much total budget does the knowledge store get
vs. the file context module?). They operate at different granularities and
compose naturally.

---

## 11. Mechanism Design for LLMs

A landmark paper directly connecting mechanism design to LLM systems:

**Duetting, Mirrokni, Paes Leme, Xu, Zuo (2024), "Mechanism Design for Large
Language Models."** WWW 2024 Best Paper, arXiv:2310.10826. Proposes a **token
auction** model where competing LLM agents bid for influence over the output,
operating token-by-token. Key results:

1. Desirable incentive properties (truthful bidding) are equivalent to a
   **monotonicity condition** on output aggregation.
2. When valuations are KL-divergence-based, the welfare-maximizing rule is a
   **weighted log-space convex combination** of target distributions.
3. This is the first clean extension of VCG to LLM content generation.

The connection to Roko's VCG attention auction: Duetting et al.'s token auction
operates at the generation level (which tokens to produce), while Roko's
operates at the context level (which tokens to include in the prompt). Both use
the same incentive-compatibility framework. The token auction validates that
mechanism design is applicable to LLM systems in practice, not just in theory.

---

## 12. Game-Theoretic Properties

### 12.1 Incentive Compatibility

The VCG mechanism is **dominant-strategy incentive compatible**: each subsystem's
optimal strategy is to bid its true expected value, regardless of what other
subsystems bid.

Proof sketch: In VCG, each subsystem's utility is:

```
utility_i = value_i * x_i - payment_i
```

where `payment_i` depends only on OTHER subsystems' bids. Since `payment_i` is
independent of `bid_i`, the subsystem maximizes its utility by maximizing the
probability that it wins when its value exceeds its payment. Bidding truthfully
(bid_i = value_i) achieves this.

### 12.2 No Useful Deviation

No subsystem can improve its allocation by deviating from truthful bidding:

- **Overbidding**: wins more often, but wins may have negative utility
  (value < payment). The second-price rule means overbidding does not reduce
  payments -- it only causes wins in cases where the subsystem should not win.

- **Underbidding**: loses auctions it should have won. The subsystem misses
  allocations where its value exceeds its payment.

### 12.3 Practical Limitations

VCG has known limitations in the context allocation setting:

1. **Computational complexity**: Combinatorial knapsack is NP-hard. For prompt
   assembly with N < 50 candidates, the greedy approximation is sufficient
   and runs in O(N log N) time.

2. **Revenue non-monotonicity**: Adding more candidates can decrease total
   payments. Not problematic for context allocation since payments are
   diagnostic, not monetary.

3. **Collusion vulnerability**: Multiple subsystems could theoretically collude
   to lower their payments. Not a practical concern when subsystems are software
   modules under the same operator's control, but emergent structural coupling
   is monitored via correlation detection.

4. **Approximate welfare maximization**: When the greedy knapsack is used
   instead of exact optimization, VCG payments no longer guarantee exact
   truthfulness (Nisan & Ronen). The practical impact is small for N < 50.

---

## 13. Academic Foundations

**Vickrey, W. (1961), "Counterspeculation, Auctions, and Competitive Sealed
Tenders."** Journal of Finance, 16(1), 8-37. The foundational paper on
second-price auctions and truthful bidding as a dominant strategy.

**Clarke, E. H. (1971), "Multipart Pricing of Public Goods."** Public Choice,
11(1), 17-33. Extended second-price auctions to combinatorial settings with
externality-based payments.

**Groves, T. (1973), "Incentives in Teams."** Econometrica, 41(4), 617-631.
Proved uniqueness of the VCG payment rule for achieving truthful bidding and
efficient allocation simultaneously.

**Simon, H. A. (1971), "Designing Organizations for an Information-Rich World."**
The information-attention tradeoff that motivates the entire auction framework.

**Friston, K. (2022), The Free Energy Principle.** Active inference as the
complementary scoring mechanism operating within subsystems.

**Duetting, Mirrokni, Paes Leme, Xu, Zuo (2024), "Mechanism Design for Large
Language Models."** WWW 2024 Best Paper, arXiv:2310.10826. Token auction model
for aggregating competing LLM agents. First clean extension of VCG to LLM
systems. Monotonicity condition for incentive compatibility.

**Zhu et al. (2025), "Regularized Proportional Fairness Mechanism for Resource
Allocation Without Money."** ICLR 2025, arXiv:2501.01111. RPF-Net adds neural
regularization to proportional fairness for robustness against misreports.

**MIT CEEPR (2023), "Learning in Repeated Multi-Unit Auctions."** Working
Paper 2023-18. No-regret learning converges to welfare-maximizing equilibria
in repeated auctions under VCG.

**arXiv:2402.19420 (2024), "Understanding Iterative Combinatorial Auction
Designs via Multi-Agent Reinforcement Learning."** Deep MARL computes equilibria
in combinatorial auctions, providing convergence bounds for Thompson sampling
bidders.

**arXiv:2402.07363 (2024), "Strategically-Robust Learning Algorithms for
Bidding in First-Price Auctions."** Robustness guarantees against adversarial
strategic behavior in learning-based bidding.

**Chien & Sinclair (UC Berkeley), "Strong and Pareto Price of Anarchy in
Congestion Games."** PoA for Pareto-optimal Nash equilibria is significantly
smaller than for arbitrary Nash equilibria.

**Nisan & Ronen, "Computationally Feasible VCG Mechanisms."** When welfare
maximization is approximate, VCG-based mechanisms lose exact truthfulness
guarantees.

**Mehrabian, A. (1996), "Pleasure-Arousal-Dominance: A General Framework."**
The PAD model underlying the affect_weight component of the bid formula.

**Bertsimas, D., Farias, V. F., & Trichakis, N.** "The Price of Fairness."
Operations Research. The alpha-fairness family unifying utilitarian, proportional,
and max-min fairness criteria.

---

## 14. Current Status and Gaps

| Aspect | Status |
|--------|--------|
| VCG allocation (greedy knapsack) | **Implemented** (`vcg_allocate` in auction.rs) |
| VCG payment computation | **Implemented** (diagnostic payments) |
| LearningBidder (Thompson sampling) | **Implemented** (per-subsystem Beta distributions) |
| FairnessConfig (alpha-fairness) | **Implemented** (configurable alpha + safety floor) |
| AuctionDiagnostics | **Implemented** (welfare, payments, Pareto, utilization) |
| Collusion detection | **Implemented** (Pearson correlation threshold) |
| 9 AttentionBidder variants | **Implemented** (per-subsystem modulation) |
| Auto-selection (VCG vs. static fallback) | **Implemented** (observation count threshold) |
| Full VCG payment diagnostics dashboard | **Designed** -- route/TUI integration pending |
| Active inference + VCG integration proof | **Not yet** -- equivalence demonstration |

---

## Cross-References

| File | Relationship |
|------|-------------|
| `active-inference-context-selection.md` | Complementary scoring mechanism within subsystems |
| `token-budget-management.md` | Budget constraints fed into the auction |
| `predictive-foraging-mvt.md` | MVT as complementary stopping rule |
| `affect-modulated-retrieval.md` | PAD state modulates bid affect_weight |
| `distributed-context-engineering.md` | VCG as Level 3 allocation mechanism |
| [06-COMPOSITION.md](../../06-COMPOSITION.md) ss 9 | Parent chapter VCG section |
