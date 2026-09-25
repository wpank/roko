# 08-learning/22 -- Autocatalytic Compounding

> The theoretical framework for why interconnected feedback loops can
> achieve super-linear improvement over time. An autocatalytic set
> (Kauffman 1993) of learning subsystems becomes self-sustaining once it
> reaches a critical diversity threshold. Ten flywheel mechanisms and
> seven autocatalytic metrics track whether the compound improvement
> thesis holds empirically.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 10

**Source:** `crates/roko-learn/src/aggregate.rs`
(`AutocatalyticMetrics`, `compute_compounding_metrics`),
`crates/roko-learn/src/cfactor.rs` (C-Factor trend tracking)

**Academic basis:** Kauffman, S.A. (1993). *The Origins of Order:
Self-Organization and Selection in Evolution*. Oxford University Press.
(Autocatalytic sets, critical diversity threshold.)

---

## 1. Purpose

Individual learning subsystems produce linear improvement: each subsystem
independently optimizes one dimension of performance (routing quality,
prompt effectiveness, failure prevention). The autocatalytic thesis predicts
that when these subsystems are connected by feedback loops, the compound
effect can be *super-linear* -- each subsystem's improvement amplifies the
improvements of every other subsystem.

This document specifies the theoretical framework, the empirical metrics
that test it, and the conditions under which the thesis is falsified.

---

## 2. Autocatalytic Sets

### 2.1 Definition (Kauffman 1993)

An autocatalytic set is a collection of entities where each entity's
production is catalyzed by other entities in the set. Formally:

- Let S = {s_1, s_2, ..., s_n} be a set of species (entities).
- Let R = {r_1, r_2, ..., r_m} be a set of reactions (transformations).
- S is autocatalytic if every reaction r_i in R has at least one catalyst
  from S, and every species s_j in S is produced by at least one reaction
  in R.

Once the set reaches a **critical diversity threshold**, it becomes
self-sustaining: the creation of new entities accelerates the creation of
further entities, producing exponential growth.

### 2.2 Application to Roko

Roko's learning subsystems form an autocatalytic set:

```
Skills catalyze -----> better prompts
Better prompts catalyze -----> higher pass rates
Higher pass rates catalyze -----> more successful episodes
More episodes catalyze -----> better pattern extraction
Better patterns catalyze -----> better playbook rules
Better rules catalyze -----> fewer failures
Fewer failures catalyze -----> lower costs
Lower costs catalyze -----> more experiments
More experiments catalyze -----> better skills
    ^                                        |
    +----------------------------------------+
              (autocatalytic cycle)
```

Each element enables the next. The cycle is autocatalytic because it is
self-reinforcing: once started, it accelerates without external input.

### 2.3 Critical Diversity Threshold

Kauffman's theory predicts a phase transition: below a certain number of
interacting components, the cycle cannot sustain itself. Above the
threshold, it becomes self-sustaining and accelerates.

For Roko, the critical components are:

| Component | Role in Cycle | Status |
|-----------|--------------|--------|
| Episode logger | Data substrate | Wired |
| Pattern miner | Knowledge extraction | Wired |
| Playbook rules | Knowledge application | Wired |
| Skill library | Capability accumulation | Wired |
| Cascade router | Resource optimization | Wired |
| Provider health | Reliability | Wired |
| Cost normalization | Budget management | Wired |
| Regression detection | Quality assurance | Wired |
| Prompt experiments | Prompt optimization | Wired |
| C-Factor | System measurement | Wired |

All 10 components are wired, and all 8 inter-component feedback loops are
connected. The thesis predicts that this connectivity enables the
autocatalytic cycle to operate.

---

## 3. Compound Improvement Mathematics

### 3.1 Multiplicative Model

The compound success probability is the product of independent component
success probabilities:

```
compound_success = P(routing) * P(prompts) * P(skills) * P(rules)
```

If each component has a 90% independent success rate:

```
compound = 0.9 * 0.9 * 0.9 * 0.9 = 0.656
```

### 3.2 Improvement Amplification

Small uniform improvements produce disproportionate compound gains:

| Scenario | Component Rate | Compound | Absolute Gain |
|----------|---------------|----------|---------------|
| Baseline | 90% each | 0.656 | -- |
| Routing 90% -> 95% | 95/90/90/90 | 0.692 | +3.6% |
| All 90% -> 92% | 92% each | 0.716 | +6.0% |
| All 90% -> 95% | 95% each | 0.815 | +15.9% |
| All 90% -> 98% | 98% each | 0.922 | +26.6% |

The multiplicative structure means a 5-point uniform improvement (90% to
95%) produces a 16-point compound improvement (65.6% to 81.5%). This
amplification is the mechanism by which small improvements in individual
subsystems produce large improvements in overall system performance.

### 3.3 Caveats on the Multiplicative Model

1. **Independence assumption.** Components are not independent. Better
   routing reduces the marginal value of better prompts (the model is
   already well-chosen). Positive correlation between components reduces
   the compound effect.

2. **Diminishing returns.** Each component has a ceiling at 100%. As
   components approach their ceilings, further improvement becomes harder
   and the compound effect plateaus.

3. **Stability constraint.** Compound improvement only occurs when the
   system is stable (see
   [stability-mechanisms.md](stability-mechanisms.md)). Oscillation
   between components can produce compound *degradation* rather than
   compound improvement.

4. **Minimum viable diversity.** The autocatalytic cycle requires *all*
   components to function. A missing component breaks the cycle. This is
   why closing the eight feedback loops was prioritized: incomplete
   connectivity prevents the autocatalytic cycle from operating.

---

## 4. Network Effects

The autocatalytic thesis invokes two network scaling laws:

### 4.1 Metcalfe's Law

The value of a network is proportional to N^2 (the number of possible
pairwise connections between N nodes). With 8 feedback loops connecting
10+ subsystems, the potential interaction space is O(N^2) ~ 100
interactions, each potentially creating an improvement pathway.

### 4.2 Reed's Law

The value of a network is proportional to 2^N (the number of possible
subsets). This applies when groups of subsystems form emergent coalitions:
the cascade router + provider health + cost normalization form a "routing
coalition" that is more than the sum of its parts.

### 4.3 Polya Urn Model (Loreto & Tria 2014)

The Polya urn model for innovation predicts that the rate of discovery
accelerates as the knowledge base grows: each new discovery opens adjacent
possibilities that increase the probability of further discoveries.

Applied to Roko: each new skill, pattern, or routing rule opens new
optimization pathways that were not previously visible. A skill for
"modifying config schemas" enables more efficient config modifications,
which produces more successful episodes, which enables extraction of finer-
grained patterns about config schema evolution.

---

## 5. Ten Flywheel Mechanisms

Each mechanism independently produces linear improvement. When connected
through feedback loops, the compound effect can be super-linear:

| # | Mechanism | Source | How It Compounds |
|---|-----------|--------|-----------------|
| 1 | Skill accumulation | Voyager (Wang et al. 2023) | More skills -> cheaper future tasks |
| 2 | Pattern extraction | Trigram mining | More patterns -> fewer repeated mistakes |
| 3 | Playbook rules | Reflexion/ExpeL | More rules -> higher first-attempt pass rate |
| 4 | Model routing | RouteLLM/FrugalGPT | Better routing -> lower cost per task |
| 5 | Cache optimization | KV cache affinity | More reuse -> lower marginal cost |
| 6 | Prompt optimization | DSPy/experiments | Better prompts -> fewer iterations |
| 7 | Calibration | Peer prediction | Better predictions -> better decisions |
| 8 | Crate familiarity | LinUCB context | More experience -> better per-crate routing |
| 9 | Cross-project transfer | HDC fingerprints | Skills from project A accelerate project B |
| 10 | Meta-optimization | ADAS (Hu et al. 2025) | Better architecture -> better everything |

Mechanisms 1-8 are wired and producing data. Mechanism 9 (cross-project
transfer) requires multi-workspace knowledge sharing. Mechanism 10 (meta-
optimization via ADAS) is planned but not implemented -- R04 delivers
bounded meta-agent lifecycle but explicitly does not implement ADAS.

---

## 6. Seven Autocatalytic Metrics

The `compute_compounding_metrics` function in `aggregate.rs` computes seven
metrics that indicate whether the autocatalytic cycle is active:

```rust
pub struct AutocatalyticMetrics {
    /// Tasks that reused durable knowledge or a playbook.
    pub knowledge_reuse_rate: f64,
    /// Tasks explicitly matched to a playbook.
    pub playbook_hit_rate: f64,
    /// Provider inference-cache hit rate when recorded.
    pub cache_hit_rate: f64,
    /// Initial routes that matched the eventual successful model.
    pub routing_accuracy: f64,
    /// Tasks passing gates without a replan.
    pub gate_pass_rate: f64,
    /// Failed gate signatures previously observed in the window.
    pub error_dedup_rate: f64,
    /// Dollar cost divided by successful episodes.
    pub cost_per_success: f64,
    /// Computation timestamp.
    pub computed_at: DateTime<Utc>,
    /// Number of episodes included.
    pub episode_window: usize,
}
```

### 6.1 Metric Interpretation

| Metric | Increasing Means | Autocatalytic Signal |
|--------|-----------------|---------------------|
| knowledge_reuse_rate | More tasks leverage stored knowledge | Knowledge substrate is feeding execution |
| playbook_hit_rate | More tasks match existing rules | Rules are generalizing across tasks |
| cache_hit_rate | More inference results are cached | Repeated patterns enable caching |
| routing_accuracy | Initial model choices are more often correct | Router is learning from outcomes |
| gate_pass_rate | More tasks pass gates first try | Overall quality is improving |
| error_dedup_rate | More errors were previously seen | Error patterns are being captured |
| cost_per_success | Cost per success is decreasing | Efficiency is improving |

### 6.2 Computation

Each metric is computed from a bounded episode window (typically the most
recent 200 episodes):

```rust
pub fn compute_compounding_metrics(episodes: &[Episode]) -> AutocatalyticMetrics {
    let total = episodes.len();

    let knowledge_reuse = episodes.iter()
        .filter(|e| e.extra.contains_key("knowledge_used")
            || e.extra.get("playbook_hits")
                .and_then(Value::as_u64)
                .is_some_and(|h| h > 0))
        .count();

    let routing_accuracy = episodes.iter()
        .filter_map(|e| {
            let initial = e.extra.get("initial_model")?.as_str()?;
            let successful = e.extra.get("successful_model")
                .and_then(Value::as_str)
                .unwrap_or(&e.model);
            Some(initial == successful && e.success)
        })
        .filter(|matched| *matched)
        .count();

    // ... similar computations for remaining metrics
}
```

### 6.3 Trend Analysis

The autocatalytic thesis predicts that these metrics should show a specific
temporal pattern:

1. **Initial plateau** (first 50 episodes): Learning subsystems are
   bootstrapping. Metrics are near their floor values. The system has not
   yet accumulated enough data to generalize.

2. **Acceleration** (50-200 episodes): Feedback loops engage. Metrics
   begin rising as skills, rules, and routing decisions improve from
   accumulated experience.

3. **Super-linear growth** (200-500 episodes): The autocatalytic cycle
   activates. Metrics rise faster than linear because each improvement
   amplifies other improvements.

4. **Saturation** (500+ episodes): Components approach their ceilings.
   Growth rate decreases as diminishing returns dominate.

---

## 7. Empirical Validation

### 7.1 C-Factor as Compound Proxy

The C-Factor (see [collective-calibration.md](collective-calibration.md))
is the primary proxy for compound improvement. If the autocatalytic thesis
holds, the C-Factor time series should show:

- Linear or sub-linear growth during the bootstrap phase
- Acceleration (second derivative > 0) during the compound phase
- Deceleration (second derivative < 0) during saturation

### 7.2 Monotonicity Tracking

Compound improvement should be monotonic (with small perturbations). The
C-Factor trend is tracked:

```
C-Factor time series:
    0.48, 0.51, 0.53, 0.55, 0.54, 0.57, 0.61, 0.63, 0.65, 0.68

Monotonicity score = fraction of steps where C(t) > C(t-1)
    = 8/9 = 0.89 (high monotonicity)

If monotonicity < 0.60 over 20+ episodes:
    -> Learning system is not converging
    -> Investigate: oscillation? regression? environmental shift?
```

### 7.3 Falsification Criteria

The autocatalytic thesis is a scientific hypothesis, not a marketing claim.
It is falsified if:

1. **No upward trend.** C-Factor shows no upward trend after 500 episodes
   with all 8 loops wired.

2. **Additive, not multiplicative.** Individual component improvements do
   not compound -- each improvement is additive (shifting the total by a
   fixed amount) rather than multiplicative (amplifying through the cycle).

3. **No acceleration from loop closure.** Closing additional feedback loops
   does not produce measurable C-Factor acceleration. The marginal loop
   adds no compound value.

These criteria provide concrete conditions under which the thesis should
be abandoned in favor of simpler linear improvement models.

---

## 8. ADAS: Automated Design of Agentic Systems

### 8.1 Background (Hu et al., ICLR 2025)

ADAS introduces a meta-agent that searches the space of possible agent
architectures by generating, evaluating, and iterating on agent designs in
code. Key results:

- +14% accuracy on ARC (Abstraction and Reasoning Corpus)
- +13.6 F1 improvement on reading comprehension tasks
- Discovered novel architectures that outperformed expert-designed baselines

### 8.2 ADAS as Flywheel Mechanism 10

ADAS represents the highest level of the autocatalytic hierarchy: a system
that optimizes its own optimization mechanisms. Where mechanisms 1-9
optimize within a fixed architecture, ADAS optimizes the architecture
itself.

**Status:** R04 delivers bounded meta-agent lifecycle (proposal, activation,
rollback, deactivation with lineage tracking), but explicitly does not
implement ADAS, Loop 4 (autonomous structural evolution), or autonomous
agent generation. ADAS is a target design aspiration.

### 8.3 Roko's ADAS Pathway

Roko's architecture supports ADAS-style meta-optimization:

| ADAS Requirement | Roko Component |
|-----------------|----------------|
| Architecture in code | `roko.toml` + `SystemPromptBuilder` templates |
| Evaluation harness | 19-gate pipeline |
| Performance metrics | C-Factor, four key metrics |
| Experiment framework | `ExperimentStore` for A/B testing |
| Search strategy | Cascade router bandits (extensible) |

The key insight is that Roko already has all the components needed for
ADAS -- it needs a meta-level agent that operates on configurations
rather than on code. Where a normal agent modifies `src/*.rs`, the ADAS
meta-agent would modify `roko.toml`, prompt templates, and routing rules.

---

## 9. EvoSkills: Evolutionary Skill Optimization

Chen et al. (2023) introduced EvoSkills: an evolutionary approach where
skills are treated as a population undergoing selection, crossover, and
mutation:

1. **Selection** -- skills with high success rates are selected for
   reproduction.
2. **Crossover** -- combine steps from two successful skills.
3. **Mutation** -- vary skill parameters to explore alternatives.
4. **Fitness** -- gate pass rate serves as the fitness function.

**Status:** Not implemented. The current skill library only accumulates
and tracks. Evolutionary optimization of existing skills is a target
design for future work.

---

## 10. Relationship to Kauffman (1993)

### 10.1 The Origins of Order

Kauffman's central thesis is that biological complexity arises not from
natural selection alone, but from the self-organizing properties of complex
systems. Autocatalytic sets are one such self-organizing structure: a
collection of molecules that catalyze each other's production, forming a
self-sustaining metabolism without external direction.

### 10.2 Phase Transition Prediction

Kauffman predicts a phase transition in autocatalytic sets as a function of
diversity (number of species) and connectivity (number of catalytic
relationships):

```
Below threshold:  isolated reactions, no self-sustaining cycle
At threshold:     percolation -- a connected cycle emerges
Above threshold:  self-sustaining, accelerating growth
```

For Roko, the "species" are learning subsystems (10 components) and the
"catalytic relationships" are the 8 feedback loops. The thesis predicts
that the system crossed the percolation threshold when the 8 loops were
wired, enabling the autocatalytic cycle to operate.

### 10.3 Empirical Test

The prediction is testable: if the thesis is correct, the C-Factor should
show a qualitative change in behavior (acceleration, not just improvement)
after the final feedback loop was closed. If no acceleration is observed,
the system has not crossed the percolation threshold despite having the
requisite connectivity.

---

## References

- Kauffman, S.A. (1993). *The Origins of Order: Self-Organization and
  Selection in Evolution*. Oxford University Press.
- Hu, S. et al. (2025). Automated Design of Agentic Systems. *ICLR 2025*.
- Chen, T. et al. (2023). EvoSkills: Emergent Skill Evolution for
  Open-World Robot Learning. arXiv:2306.09536.
- Loreto, V. & Tria, F. (2014). The Dynamics of Innovation: Polya Urn
  Models Revisited. arXiv:1401.4420.
- Wang, G. et al. (2023). Voyager: An Open-Ended Embodied Agent with
  Large Language Models. *NeurIPS 2023 (Oral)*.
