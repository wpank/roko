# 08-learning/20 -- C-Factor Governance

> Sliding-window governance over collective performance snapshots. The
> governance system detects pathological collective patterns (domination,
> groupthink, prediction collusion), emits non-binding recommendations,
> and computes five process variables adapted from Woolley et al. (2010).

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 7

**Source:** `crates/roko-learn/src/cfactor.rs` (`CFactorGovernance`,
`CFactorRecommendation`, `CollectivePathology`, `detect_pathologies`,
`is_declining_streak`, `agent_trend`)

**Academic basis:** Woolley, A.W. et al. (2010). Evidence for a collective
intelligence factor in the performance of human groups. *Science*
330(6004), 686-688.

---

## 1. Purpose

The C-Factor composite metric (see
[collective-calibration.md](collective-calibration.md)) measures *what* the
collective's performance is. C-Factor governance measures *how* the
collective is performing and *why* -- detecting structural pathologies,
computing process variables, and emitting diagnostic recommendations.

The distinction matters: a declining C-Factor says "things are getting
worse" but not whether the cause is model lock-in, groupthink, or a
provider outage. Governance answers the causal question by tracking the
five Woolley process variables and detecting three collective pathologies.

---

## 2. The Five Woolley Variables

Woolley et al. (2010) identified process variables that predict group
performance independently of individual member ability. Their finding --
that collective intelligence depends more on group dynamics than on the
smartest individual -- applies directly to agent collectives where "member
ability" is determined by model selection and prompt configuration.

### 2.1 Variable Definitions

| # | Variable | Formula | What It Measures |
|---|----------|---------|------------------|
| 1 | Turn-taking entropy | `H = -sum(p_i * ln(p_i)) / ln(N)` | Conversational equality |
| 2 | Peer prediction accuracy | `1.0 - MSE(predictions, outcomes)` | Social perceptiveness |
| 3 | Citation reciprocity | `survived_citations / total_citations` | Trust calibration |
| 4 | Delivery rate | `confirmed / (confirmed + dropped)` | Channel openness |
| 5 | HDC diversity | `1.0 - mean_pairwise_similarity` | Cognitive diversity |

### 2.2 Variable 1: Turn-Taking Entropy

Turn-taking entropy measures whether agent participation is evenly
distributed across episodes. The formula normalizes Shannon entropy by
`ln(N)` to produce a value in [0, 1] regardless of the number of agents:

```
H = -sum(p_i * ln(p_i)) / ln(N)

where p_i = episodes_by_agent_i / total_episodes
      N   = number of distinct agents
```

- H = 1.0 means perfectly equal participation (every agent handles the
  same number of episodes).
- H = 0.0 means one agent handles all episodes.

**Why it matters:** A collective where one agent handles 90% of episodes is
effectively a single agent -- it has lost the diversity benefit that
justifies the overhead of collective operation. The CLT-inspired 31.6x
heuristic assumes N independent agents; if one agent dominates, the
effective N approaches 1.

### 2.3 Variable 2: Peer Prediction Accuracy

Peer prediction measures whether agents can predict each other's outcomes.
If agent A can predict that agent B will fail a particular task, the
collective has useful predictive structure. If predictions are random, the
agents are not learning from each other's patterns.

**Roko computation:** Each agent's historical pass rate on task categories
similar to the current task (measured via HDC fingerprint similarity) serves
as the prediction. The MSE between these predictions and actual outcomes
gives the accuracy score.

### 2.4 Variable 3: Citation Reciprocity

Citation reciprocity tracks whether knowledge shared between agents (via
playbook rules, skills, or episode references) actually survives validation.
A "citation" is a playbook rule sourced from one agent's episodes and
applied to another agent's task. "Survival" means the rule was validated
(not contradicted) when applied.

```
reciprocity = survived_citations / total_citations
```

Low reciprocity (< 0.3) indicates that knowledge transfer between agents is
unreliable -- rules that work for one agent do not generalize to others.

### 2.5 Variable 4: Delivery Rate

Delivery rate measures the reliability of the execution pipeline. A
"confirmed delivery" is an episode that reaches gate evaluation (regardless
of outcome). A "dropped delivery" is an episode that fails before reaching
gates (agent crash, timeout, infrastructure failure).

```
delivery_rate = confirmed / (confirmed + dropped)
```

Low delivery rate indicates infrastructure problems that prevent agents from
even attempting tasks, independent of their intellectual capability.

### 2.6 Variable 5: HDC Diversity

HDC diversity measures the cognitive diversity of the collective by
computing mean pairwise HDC fingerprint similarity across recent episodes:

```
diversity = 1.0 - mean_pairwise_similarity(hdc_fingerprints)
```

High diversity (> 0.7) means agents are producing structurally different
outputs. Low diversity (< 0.3) means agents are converging on the same
approach -- a sign of groupthink.

---

## 3. Learned Weights

The five variables are combined into a governance score using learned
weights rather than fixed coefficients:

```
governance_score = w_1 * turn_taking_entropy
                 + w_2 * peer_prediction_accuracy
                 + w_3 * citation_reciprocity
                 + w_4 * delivery_rate
                 + w_5 * hdc_diversity
                 + bias
```

Weights are learned online via gradient descent from cohort outcomes:

```
On cohort completion with observed outcome_quality:
    predicted = weights.dot(metrics)
    error = outcome_quality - predicted

    for i in 0..5:
        w_i += learning_rate * error * metric_i
```

This approach discovers which process variables actually predict quality in
Roko's specific context rather than relying on Woolley et al.'s fixed
weights (which were derived from human groups performing cognitive tasks,
not agent collectives performing software engineering).

---

## 4. Collective Pathologies

The governance system detects three pathological patterns that degrade
collective performance:

### 4.1 Pathology Definitions

```rust
pub enum CollectivePathology {
    /// One agent dominates: turn_taking_equality < 0.3
    Domination { dominant_agent: String, share: f64 },
    /// All agents converge on same approach: hdc_diversity < 0.2
    Groupthink { diversity_score: f64 },
    /// Peer predictions are worse than random: prediction_accuracy < 0.5
    PredictionCollusion { accuracy: f64 },
}
```

### 4.2 Domination

Domination occurs when one agent handles a disproportionate share of
episodes. The detection threshold is a turn-taking equality score below
0.3 (one agent handles roughly 70%+ of episodes).

**Cause:** Cascade router lock-in. If one model consistently outperforms
others, the bandit exploits it so heavily that other agents never receive
enough episodes to improve their statistics.

**Remedy:** The governance recommendation is `InjectDiversity` -- force
a minimum fraction of episodes to be routed to under-represented agents,
even at the cost of short-term quality.

### 4.3 Groupthink

Groupthink occurs when agents produce similar outputs despite receiving
different prompts or using different models. The detection threshold is
HDC diversity below 0.2 (mean pairwise similarity > 0.8).

**Cause:** Prompt template homogeneity. If all roles use similar system
prompts, agents converge on the same approach regardless of model
differences.

**Remedy:** The governance recommendation is `InjectDiversity` -- vary
prompt templates, tool sets, or model selections to increase output
diversity.

### 4.4 Prediction Collusion

Prediction collusion occurs when peer prediction accuracy drops below
random (0.5). This indicates that agents' models of each other are
actively misleading -- worse than having no model at all.

**Cause:** Non-stationarity. If model performance changes rapidly (provider
updates, codebase evolution), historical prediction models become stale
and produce systematic errors.

**Remedy:** The governance recommendation is `AdjustModel` -- reset the
cascade router's historical statistics to force re-evaluation of model
performance under current conditions.

---

## 5. Governance Recommendations

### 5.1 Recommendation Structure

```rust
pub struct CFactorRecommendation {
    pub target: CFactorTarget,
    pub action: CFactorAction,
    pub dispatch_bias: Option<AgentDispatchBias>,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

pub enum CFactorAction {
    AdjustModel,
    AdjustGate,
    InjectDiversity,
}
```

### 5.2 Non-Binding by Design

Governance recommendations are **diagnostic, not prescriptive**. They are
logged and surfaced to operators (via `roko show learning`, dashboard, and
telemetry events) but never automatically applied. This is deliberate:

1. **Goodhart's Law.** If the C-Factor became a reward signal, the system
   would game it -- optimizing turn-taking equality by artificially
   distributing tasks, rather than by genuinely improving collective
   dynamics.

2. **Observation, not control.** The C-Factor is a covariate that
   *correlates* with quality, not an objective to *maximize*. Treating
   it as an optimization target would create a positive feedback loop that
   the stability mechanisms (hysteresis, frequency separation) are designed
   to prevent.

3. **Human oversight.** Self-modification of collective dynamics is a
   high-severity change (it affects all future routing and task allocation).
   The improvement velocity limits (section 6.3 of
   [self-improvement-frameworks.md](self-improvement-frameworks.md))
   require human review for such changes.

### 5.3 Declining Streak Detection

```rust
pub fn is_declining_streak(&self, n: usize) -> bool {
    if self.snapshots.len() < n {
        return false;
    }
    let recent: Vec<f64> = self.snapshots
        .iter()
        .rev()
        .take(n)
        .map(|s| s.overall)
        .collect();
    recent.windows(2).all(|w| w[0] <= w[1])
    // Note: reversed iteration, so w[0] is more recent
}
```

A declining streak of 3+ snapshots triggers an `InvestigateRegression`
recommendation. Because each snapshot covers 50 episodes, a 3-snapshot
declining streak represents 150 episodes of sustained degradation -- strong
evidence that a systemic issue exists, not merely statistical noise.

### 5.4 Agent Trend Analysis

```rust
pub fn agent_trend(&self, agent_id: &str) -> Option<f64> {
    let values = self.snapshots.iter()
        .filter_map(|s| s.agent_contribution(agent_id))
        .map(|c| c.contribution_score)
        .collect::<Vec<_>>();
    (values.len() >= 2).then(|| values[values.len() - 1] - values[0])
}
```

The agent trend is a linear endpoint estimate: the difference between the
most recent and earliest contribution scores. A negative trend for an agent
means its contributions are declining relative to the collective -- the
agent is becoming less useful over time.

---

## 6. Sliding Window Implementation

```rust
pub struct CFactorGovernance {
    snapshots: VecDeque<CFactor>,
    capacity: usize,  // default: 20
}
```

The governance system maintains the most recent 20 C-Factor snapshots in a
ring buffer. At 50 episodes per snapshot, this covers 1000 episodes -- a
window large enough to detect structural trends while remaining bounded in
memory.

### 6.1 Lifecycle

```
Every 50 episodes:
    |
    +-- 1. CFactor::compute(episodes_window_200)
    |
    +-- 2. CFactorGovernance::push_snapshot(cfactor)
    |       (evicts oldest if at capacity)
    |
    +-- 3. detect_pathologies(cfactor)
    |       -> Vec<CollectivePathology>
    |
    +-- 4. is_declining_streak(3)
    |       -> bool
    |
    +-- 5. agent_trend(agent_id) for each active agent
    |       -> Option<f64> per agent
    |
    +-- 6. Emit CFactorRecommendation if pathology or decline detected
    |
    +-- 7. Persist to .roko/learn/cfactor.json
```

---

## 7. Relationship to Woolley et al. (2010)

### 7.1 Key Finding

Woolley et al. conducted experiments with 699 participants in 192 groups.
Their central finding: a general "collective intelligence" factor (c) exists
that predicts group performance across diverse tasks, analogous to the
general intelligence factor (g) for individuals. Crucially, c was *not*
correlated with the maximum individual intelligence in the group -- smart
individuals do not guarantee smart groups.

### 7.2 Predictive Variables

The strongest predictors of c were:

1. **Equal turn-taking** -- groups where members contributed equally
   outperformed groups dominated by one member.
2. **Social perceptiveness** -- measured via the "Reading the Mind in the
   Eyes" test. Groups with higher average social perceptiveness performed
   better.
3. **Proportion of women** -- which the authors attributed to women's
   higher average social perceptiveness scores, not gender per se.

### 7.3 Adaptation to Agent Collectives

For agent collectives, "social perceptiveness" has no direct analogue (agents
do not read facial expressions). Roko substitutes **peer prediction accuracy**
as a functional equivalent: the ability to predict another agent's outcome is
a form of social awareness translated into the software engineering domain.

The "proportion of women" finding is reinterpreted as **model diversity**: a
collective using the same model for all agents is analogous to a group of
identical individuals. Diversity of models, prompts, and tool configurations
increases the effective independence of agent contributions.

---

## 8. Variance Inequality Check

The Variance Inequality (VI) check is a governance tool applied to
experiments rather than to the collective as a whole:

```rust
pub struct VICheck {
    pub can_conclude: bool,
    pub reason: String,
    pub leading_variant: Option<String>,
    pub gap: f64,
    pub required_gap: f64,
}
```

When the variance between the leading and second-best variant is too small
relative to the sample size, the experiment cannot conclude -- adding more
data would not change the outcome. The VI check terminates such experiments
early, saving execution budget that would otherwise be wasted on
inconclusive A/B tests.

---

## References

- Woolley, A.W. et al. (2010). Evidence for a collective intelligence
  factor in the performance of human groups. *Science* 330(6004), 686-688.
- Engel, D. et al. (2014). Reading the Mind in the Eyes or Reading between
  the Lines? Theory of Mind Predicts Collective Intelligence Equally Well
  Online and Face-To-Face. *PLOS ONE* 9(12), e115212.
- Surowiecki, J. (2004). *The Wisdom of Crowds*. Doubleday.
