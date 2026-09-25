# Active Inference for Compute Allocation

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/10-active-inference-compute-allocation.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS12.
> Cite: Friston (2006, 2010), Friston et al. (2015).

---

## 1. Abstract

Active inference (Friston 2010, "The free-energy principle: a unified brain theory?",
Nature Reviews Neuroscience 11(2); Friston et al. 2015, "Active inference and
epistemic value", Cognitive Neuroscience 6(4)) provides the theoretical foundation
for the tier decision: the agent should invest compute that **minimizes expected
free energy (EFE)** -- balancing pragmatic value with epistemic value, minus cost.

For Roko, active inference provides a principled, zero-hyperparameter answer to:
"How much should I think about this?" The tier decision, context budget, and model
selection all emerge from EFE minimization rather than hand-tuned thresholds.

---

## 2. The EFE Formula

```
G(pi, tau) = -E_Q[ln P(o_tau | C)]  +  E_Q[H[P(o_tau | s_tau)]]
              -----------------------     -----------------------
              pragmatic value              epistemic value
              (expected utility             (expected information
               of preferred outcomes)        gain from observations)
```

Where Q is the approximate posterior, o_tau is expected observation, s_tau is
expected hidden state, C is preferred outcomes, H is entropy.

The first term (pragmatic value) captures how much the agent expects to achieve its
goals under this policy. The second term (epistemic value) captures how much
uncertainty the agent expects to resolve. This is what drives exploration -- the
agent seeks out information that reduces uncertainty.

---

## 3. Applied to Tier Selection

```
EFE(tier) = pragmatic_value(tier) + epistemic_value(tier) - cost(tier)

  pragmatic_value(T0) = value of applying playbook rules (low if no match)
  pragmatic_value(T1) = value of quick assessment (medium)
  pragmatic_value(T2) = value of deep analysis (high)

  epistemic_value(T0) = 0 (no new information)
  epistemic_value(T1) = moderate (some uncertainty reduction)
  epistemic_value(T2) = high (maximal uncertainty reduction)

  cost(T0) = 0
  cost(T1) = $0.001-0.003 + 200-500ms
  cost(T2) = $0.01-0.25 + 1-5s
```

**Zero hyperparameters.** Unlike epsilon-greedy, UCB, or Thompson sampling, EFE
naturally balances exploration (epistemic) and exploitation (pragmatic) as two
aspects of the same objective. This is Friston's key insight: they are not opposing
objectives requiring a tradeoff parameter.

---

## 4. The Generative Model Q

A factorized categorical distribution over CorticalState signals. Each dimension
modeled with a Dirichlet-categorical pair, updated online via Bayesian updating.

```rust
pub struct GenerativeModel {
    dimensions: HashMap<SignalId, DirichletCategorical>,
    observation_count: u64,
}

pub struct DirichletCategorical {
    pub alphas: Vec<f64>,
    pub num_categories: usize,
}

impl DirichletCategorical {
    pub fn new_uniform(num_categories: usize) -> Self {
        Self { alphas: vec![1.0; num_categories], num_categories }
    }

    pub fn observe(&mut self, k: usize) {
        if k < self.num_categories { self.alphas[k] += 1.0; }
    }

    pub fn expected_prob(&self, k: usize) -> f64 {
        let total: f64 = self.alphas.iter().sum();
        self.alphas[k] / total
    }

    pub fn entropy(&self) -> f64 {
        let total: f64 = self.alphas.iter().sum();
        -(0..self.num_categories)
            .map(|k| {
                let p = self.alphas[k] / total;
                if p > 0.0 { p * p.ln() } else { 0.0 }
            })
            .sum::<f64>()
    }
}
```

### 4.1 Bootstrapping (Cold Start)

1. **Ticks 0-49 (prior phase):** Flat Dirichlet prior (alpha=1.0). Defaults to T2.
   ~8 minutes of T2-heavy operation.
2. **Ticks 50-199 (transition):** Heuristic threshold with EFE tiebreaker.
3. **Ticks 200+ (steady state):** Full ActiveInferenceRouter.

---

## 5. Applied to Context Selection (PredictiveScorer)

```rust
pub struct PredictiveScorer {
    pragmatic_weight: f32,  // default 1.0
}

impl Scorer for PredictiveScorer {
    fn score(&self, signal: &Signal, ctx: &Context) -> Score {
        let pragmatic = self.compute_pragmatic_value(signal, ctx);
        let epistemic = self.compute_epistemic_value(signal, ctx);
        let cost_penalty = signal.estimated_tokens() as f32 / 1000.0 * 0.01;
        let effective = pragmatic * self.pragmatic_weight + epistemic - cost_penalty;
        Score { salience: effective.max(0.0), novelty: epistemic, utility: pragmatic, .. }
    }
}
```

### 5.1 Pragmatic Weight Tuning

| Agent role | pragmatic_weight | Rationale |
|---|---|---|
| Compilation/test runner | 2.0 | Clear success criteria |
| Code generator | 1.5 | Defined output, moderate uncertainty |
| General purpose | 1.0 | Balanced |
| Research/exploration | 0.7 | High uncertainty |
| Brainstorming/creative | 0.5 | Epistemic value dominates |

### 5.2 Token Cost Derivation

`tokens / 1000.0 * 0.01`: each 1,000 tokens costs 0.01 salience points. For a
typical 500-token Signal, penalty = 0.005 -- negligible compared to pragmatic/
epistemic terms (typically 0.1-0.5 each).

---

## 6. Preferred Outcomes C

Derived from two sources:
1. **Task specification.** Success criteria -> preferred CorticalState values.
2. **PAD vector baseline.** Homeostatic drive to return to personality baseline.

**Temporal discounting:** `gamma^t` where gamma = 0.95. An outcome 20 ticks away
is worth 0.358 of face value. 100 ticks out is worth 0.006.

---

## 7. Epistemic Value Estimation

```rust
pub struct EpistemicEstimator {
    history: HashMap<InferenceTier, VecDeque<(f32, f32)>>,
    window: usize,  // default: 50
}

impl EpistemicEstimator {
    pub fn expected_info_gain(&self, tier: InferenceTier) -> f32 {
        let entries = match self.history.get(&tier) {
            Some(h) if !h.is_empty() => h,
            _ => return self.default_info_gain(tier),
        };
        let total_improvement: f32 = entries.iter()
            .map(|(before, after)| (after - before).max(0.0))
            .sum();
        total_improvement / entries.len() as f32
    }

    fn default_info_gain(&self, tier: InferenceTier) -> f32 {
        match tier {
            InferenceTier::T0 => 0.0,
            InferenceTier::T1 => 0.05,
            InferenceTier::T2 => 0.15,
        }
    }
}
```

---

## 8. Connection to CascadeRouter

The CascadeRouter's three-stage cascade (Static / Confidence / UCB1):
- **Static** (< 50 obs): Fixed routing table (prior phase).
- **Confidence** (50-200 obs): Route by confidence score.
- **UCB1** (> 200 obs): Contextual bandit with LinUCB. UCB exploration bonus
  approximates epistemic value.

Target: `ActiveInferenceRouter` replaces UCB1 with full EFE computation.

---

## 9. Implementation Stages

1. **Heuristic Threshold (current):** Prediction error vs. adaptive threshold.
2. **PredictiveScorer (implemented):** EFE-style context ranking in roko-core.
3. **ActiveInferenceRouter (target):** Full EFE computation replacing UCB1.

---

## 10. References

- **Friston 2010** -- "The free-energy principle: a unified brain theory?" (Nature
  Reviews Neuroscience 11(2)).
- **Friston et al. 2015** -- "Active inference and epistemic value" (Cognitive
  Neuroscience 6(4)).
- **Parr & Friston 2017** -- "Working memory, attention, and salience in active
  inference" (Scientific Reports 7).
- **Sims 2003** -- "Implications of rational inattention" (Journal of Monetary
  Economics 50(3)).
- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176).
- **Koudahl et al. 2024** -- Factorized discrete POMDP (arXiv:2412.10425).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Heuristic tier gating
- `docs/v3/depth/29-heartbeat/16-t0-probes.md` -- Probe signals for prediction error
- `docs/v3/depth/29-heartbeat/active-inference-state-space.md` -- Factorized POMDP
- `docs/v3/depth/29-heartbeat/attention-auction-and-gating.md` -- VCG context budget
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
